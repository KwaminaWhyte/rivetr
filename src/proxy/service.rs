// Proxy service for forwarding requests to backends
//
// Handles the actual HTTP request forwarding to container backends.
// Includes WebSocket upgrade support.

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::time::Duration;
use tracing::{debug, info};

use super::Backend;

/// Service for forwarding HTTP requests to backends
#[derive(Clone)]
pub struct ProxyService {
    client: Client<hyper_util::client::legacy::connect::HttpConnector, Incoming>,
}

impl ProxyService {
    pub fn new() -> Self {
        let mut connector = hyper_util::client::legacy::connect::HttpConnector::new();
        connector.set_connect_timeout(Some(Duration::from_secs(10)));
        connector.set_nodelay(true);

        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build(connector);

        Self { client }
    }

    /// Forward a request to the specified backend
    pub async fn forward(
        &self,
        mut req: Request<Incoming>,
        backend: &Backend,
    ) -> anyhow::Result<Response<BoxBody<Bytes, hyper::Error>>> {
        // Compute path, stripping the prefix if configured
        let original_pq = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let forwarded_pq = if let Some(ref prefix) = backend.strip_prefix {
            let path = req.uri().path();
            if path.starts_with(prefix.as_str()) {
                let stripped = &path[prefix.len()..];
                // Ensure the stripped path starts with '/'
                let stripped = if stripped.is_empty() || !stripped.starts_with('/') {
                    format!("/{}", stripped)
                } else {
                    stripped.to_string()
                };
                // Re-attach query string if present
                if let Some(query) = req.uri().query() {
                    format!("{}?{}", stripped, query)
                } else {
                    stripped
                }
            } else {
                original_pq.to_string()
            }
        } else {
            original_pq.to_string()
        };

        // Build the backend URI
        let backend_uri = format!("http://{}{}", backend.addr(), forwarded_pq);

        debug!(backend_uri = %backend_uri, "Forwarding to backend");

        // Update the request URI
        *req.uri_mut() = backend_uri.parse()?;

        // Add/update forwarding headers
        let headers = req.headers_mut();

        // Set X-Forwarded-For (append if exists)
        // Note: In production, we'd extract the remote IP from the connection
        // For now, we just ensure the header structure is correct

        // Set X-Forwarded-Proto
        headers.insert(
            "X-Forwarded-Proto",
            hyper::header::HeaderValue::from_static("http"),
        );

        // Set X-Forwarded-Host (original Host header) and preserve it.
        // We do NOT overwrite the Host header with the backend address — upstream
        // apps (e.g. Laravel, Rails) use Host to generate redirect URLs and must
        // see the public domain, not the internal container address.
        if let Some(host) = headers.get(hyper::header::HOST).cloned() {
            headers.insert("X-Forwarded-Host", host);
        }

        // Make the request
        let response = self.client.request(req).await?;

        // Convert the response body to our boxed type
        let (parts, body) = response.into_parts();
        let boxed_body = body.map_err(|e| e).boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }

    /// Forward a WebSocket upgrade request to the specified backend.
    ///
    /// Opens a fresh TCP connection to the backend, performs the HTTP/WS handshake,
    /// then sets up a bidirectional byte tunnel between the client and backend.
    pub async fn forward_websocket(
        &self,
        mut req: Request<Incoming>,
        backend: &Backend,
    ) -> anyhow::Result<Response<BoxBody<Bytes, hyper::Error>>> {
        let backend_addr = backend.addr();
        let original_pq = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
            .to_string();
        let path_and_query = if let Some(ref prefix) = backend.strip_prefix {
            let path = req.uri().path();
            if path.starts_with(prefix.as_str()) {
                let stripped = &path[prefix.len()..];
                let stripped = if stripped.is_empty() || !stripped.starts_with('/') {
                    format!("/{}", stripped)
                } else {
                    stripped.to_string()
                };
                if let Some(query) = req.uri().query() {
                    format!("{}?{}", stripped, query)
                } else {
                    stripped
                }
            } else {
                original_pq.clone()
            }
        } else {
            original_pq.clone()
        };

        debug!(backend = %backend_addr, path = %path_and_query, "Forwarding WebSocket upgrade to backend");

        // Extract the client-side upgrade future BEFORE we consume the request.
        // This future resolves (after we return the 101) with the raw client IO stream.
        let client_upgrade = hyper::upgrade::on(&mut req);

        // Rewrite the request URI and headers for the backend
        *req.uri_mut() = format!("http://{}{}", backend_addr, path_and_query).parse()?;
        {
            let headers = req.headers_mut();
            headers.insert(
                "X-Forwarded-Proto",
                hyper::header::HeaderValue::from_static("http"),
            );
            if let Some(host) = headers.get(hyper::header::HOST).cloned() {
                headers.insert("X-Forwarded-Host", host);
            }
            // Preserve original Host header (do not replace with backend address)
        }

        // Open a raw TCP connection to the backend (bypass the HTTP connection pool,
        // which doesn't support WebSocket upgrades).
        let backend_tcp = tokio::net::TcpStream::connect(&backend_addr).await?;
        let _ = backend_tcp.set_nodelay(true);
        let backend_io = TokioIo::new(backend_tcp);

        // Perform the HTTP/1.1 handshake with the backend.
        let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
            .handshake::<_, Incoming>(backend_io)
            .await?;

        // Spawn the connection driver with upgrade support so it can hand off
        // the raw stream once the 101 is received.
        tokio::spawn(conn.with_upgrades());

        // Send the WebSocket upgrade request to the backend.
        let mut backend_response = sender.send_request(req).await?;
        let status = backend_response.status();

        if status == hyper::StatusCode::SWITCHING_PROTOCOLS {
            info!(backend = %backend_addr, "WebSocket upgrade successful, setting up tunnel");

            // Extract the backend-side upgrade future.
            let backend_upgrade = hyper::upgrade::on(&mut backend_response);

            // Spawn a task that waits for both sides to complete the upgrade,
            // then copies bytes bidirectionally for the lifetime of the WS session.
            tokio::spawn(async move {
                let client_io = match client_upgrade.await {
                    Ok(io) => TokioIo::new(io),
                    Err(e) => {
                        tracing::error!("Client WebSocket upgrade failed: {}", e);
                        return;
                    }
                };
                let backend_io = match backend_upgrade.await {
                    Ok(io) => TokioIo::new(io),
                    Err(e) => {
                        tracing::error!("Backend WebSocket upgrade failed: {}", e);
                        return;
                    }
                };
                let mut client_io = client_io;
                let mut backend_io = backend_io;
                if let Err(e) = tokio::io::copy_bidirectional(&mut client_io, &mut backend_io).await
                {
                    debug!("WebSocket tunnel closed: {}", e);
                }
            });
        }

        let (parts, body) = backend_response.into_parts();
        Ok(Response::from_parts(parts, body.map_err(|e| e).boxed()))
    }
}

impl Default for ProxyService {
    fn default() -> Self {
        Self::new()
    }
}
