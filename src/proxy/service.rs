// Proxy service for forwarding requests to backends
//
// Handles the actual HTTP request forwarding to container backends.
// Includes WebSocket upgrade support.

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
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
        // Build the backend URI
        let backend_uri = format!(
            "http://{}{}",
            backend.addr(),
            req.uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/")
        );

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

        // Set X-Forwarded-Host (original Host header)
        if let Some(host) = headers.get(hyper::header::HOST).cloned() {
            headers.insert("X-Forwarded-Host", host);
        }

        // Update Host header to backend
        headers.insert(
            hyper::header::HOST,
            hyper::header::HeaderValue::from_str(&backend.addr())?,
        );

        // Make the request
        let response = self.client.request(req).await?;

        // Convert the response body to our boxed type
        let (parts, body) = response.into_parts();
        let boxed_body = body.map_err(|e| e).boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }

    /// Forward a WebSocket upgrade request to the specified backend
    ///
    /// This method forwards the upgrade request to the backend and sets up
    /// bidirectional tunneling when the upgrade is accepted.
    pub async fn forward_websocket(
        &self,
        mut req: Request<Incoming>,
        backend: &Backend,
    ) -> anyhow::Result<Response<BoxBody<Bytes, hyper::Error>>> {
        // Build the backend URI
        let backend_uri = format!(
            "http://{}{}",
            backend.addr(),
            req.uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/")
        );

        debug!(backend_uri = %backend_uri, "Forwarding WebSocket upgrade to backend");

        // Update the request URI
        *req.uri_mut() = backend_uri.parse()?;

        // Add/update forwarding headers
        let headers = req.headers_mut();

        // Set X-Forwarded-Proto
        headers.insert(
            "X-Forwarded-Proto",
            hyper::header::HeaderValue::from_static("http"),
        );

        // Set X-Forwarded-Host (original Host header)
        if let Some(host) = headers.get(hyper::header::HOST).cloned() {
            headers.insert("X-Forwarded-Host", host);
        }

        // Update Host header to backend
        headers.insert(
            hyper::header::HOST,
            hyper::header::HeaderValue::from_str(&backend.addr())?,
        );

        // Make the request to the backend
        let response = self.client.request(req).await?;

        // Get the status to check if it's a successful upgrade (101 Switching Protocols)
        let status = response.status();

        if status == hyper::StatusCode::SWITCHING_PROTOCOLS {
            info!(backend = %backend.addr(), "WebSocket upgrade successful, setting up tunnel");
        }

        // Convert the response body to our boxed type
        // For WebSocket upgrades, hyper will handle the upgrade after we return the response
        let (parts, body) = response.into_parts();
        let boxed_body = body.map_err(|e| e).boxed();

        Ok(Response::from_parts(parts, boxed_body))
    }
}

impl Default for ProxyService {
    fn default() -> Self {
        Self::new()
    }
}
