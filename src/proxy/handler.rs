// Proxy connection handler
//
// Handles incoming HTTP connections, parses requests, and forwards them to backends.
// Supports WebSocket upgrade for real-time applications.

use arc_swap::ArcSwap;
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{CONNECTION, UPGRADE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use super::{Backend, ProxyService, RouteTable};

/// Handles incoming proxy connections
#[derive(Clone)]
pub struct ProxyHandler {
    routes: Arc<ArcSwap<RouteTable>>,
    proxy_service: ProxyService,
}

impl ProxyHandler {
    pub fn new(routes: Arc<ArcSwap<RouteTable>>) -> Self {
        Self {
            routes,
            proxy_service: ProxyService::new(),
        }
    }

    /// Handle a single TCP connection
    pub async fn handle_connection(
        &self,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let io = TokioIo::new(stream);
        let handler = self.clone();

        // Enable upgrades for WebSocket support
        http1::Builder::new()
            .serve_connection(
                io,
                service_fn(move |req| {
                    let handler = handler.clone();
                    async move { handler.handle_request(req, remote_addr).await }
                }),
            )
            .with_upgrades()
            .await?;

        Ok(())
    }

    /// Handle a single HTTP request
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let host = self.extract_host(&req);
        let is_websocket = self.is_websocket_upgrade(&req);

        debug!(
            method = %method,
            uri = %uri,
            host = ?host,
            remote = %remote_addr,
            websocket = is_websocket,
            "Incoming proxy request"
        );

        // Get the route table
        let routes = self.routes.load();

        // Look up the backend
        let backend = match &host {
            Some(h) => routes.get_backend(h),
            None => None,
        };

        match backend {
            Some(backend) if backend.healthy => {
                info!(
                    method = %method,
                    uri = %uri,
                    host = ?host,
                    backend = %backend.addr(),
                    websocket = is_websocket,
                    "Forwarding request"
                );

                // Handle WebSocket upgrades specially
                if is_websocket {
                    return self.handle_websocket_upgrade(req, &backend).await;
                }

                match self.proxy_service.forward(req, &backend).await {
                    Ok(response) => Ok(response),
                    Err(e) => {
                        error!(error = %e, backend = %backend.addr(), "Backend request failed");
                        Ok(self.error_response(
                            StatusCode::BAD_GATEWAY,
                            "Backend unavailable",
                        ))
                    }
                }
            }
            Some(_) => {
                warn!(host = ?host, "Backend is unhealthy");
                Ok(self.error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Service temporarily unavailable",
                ))
            }
            None => {
                warn!(host = ?host, "No backend found for host");
                Ok(self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!(
                        "No application found for host: {}",
                        host.as_deref().unwrap_or("unknown")
                    ),
                ))
            }
        }
    }

    /// Check if the request is a WebSocket upgrade
    fn is_websocket_upgrade<T>(&self, req: &Request<T>) -> bool {
        let headers = req.headers();

        // Check for Upgrade: websocket header
        let has_upgrade = headers
            .get(UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

        // Check for Connection: upgrade header
        let has_connection_upgrade = headers
            .get(CONNECTION)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("upgrade"))
            .unwrap_or(false);

        has_upgrade && has_connection_upgrade
    }

    /// Handle WebSocket upgrade requests
    async fn handle_websocket_upgrade(
        &self,
        req: Request<Incoming>,
        backend: &Backend,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        info!(backend = %backend.addr(), "Handling WebSocket upgrade");

        // Forward the WebSocket upgrade request to the backend
        match self.proxy_service.forward_websocket(req, backend).await {
            Ok(response) => Ok(response),
            Err(e) => {
                error!(error = %e, "WebSocket upgrade failed");
                Ok(self.error_response(
                    StatusCode::BAD_GATEWAY,
                    "WebSocket upgrade failed",
                ))
            }
        }
    }

    /// Extract the host from the request (Host header or URI authority)
    fn extract_host<T>(&self, req: &Request<T>) -> Option<String> {
        // First try the Host header
        if let Some(host) = req.headers().get(hyper::header::HOST) {
            if let Ok(host_str) = host.to_str() {
                return Some(host_str.to_string());
            }
        }

        // Fall back to URI authority
        req.uri().host().map(|h| h.to_string())
    }

    /// Create an error response
    fn error_response(
        &self,
        status: StatusCode,
        message: &str,
    ) -> Response<BoxBody<Bytes, hyper::Error>> {
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{} - Rivetr</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: #f5f5f5;
        }}
        .error {{
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        h1 {{ color: #e74c3c; margin-bottom: 10px; }}
        p {{ color: #666; margin: 0; }}
        .code {{ font-size: 48px; color: #333; margin-bottom: 20px; }}
    </style>
</head>
<body>
    <div class="error">
        <div class="code">{}</div>
        <h1>{}</h1>
        <p>Powered by Rivetr</p>
    </div>
</body>
</html>"#,
            status.as_u16(),
            status.as_u16(),
            message
        );

        Response::builder()
            .status(status)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("X-Powered-By", "Rivetr")
            .body(Full::new(Bytes::from(body)).map_err(|e| match e {}).boxed())
            .unwrap()
    }
}
