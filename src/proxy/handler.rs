// Proxy connection handler
//
// Handles incoming HTTP connections, parses requests, and forwards them to backends.
// Supports WebSocket upgrade for real-time applications.

use arc_swap::ArcSwap;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{AUTHORIZATION, CONNECTION, UPGRADE, WWW_AUTHENTICATE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tracing::{debug, error, info, warn};

use super::acme::AcmeChallenges;
use super::{Backend, ProxyService, RouteTable};

/// ACME HTTP-01 challenge path prefix
const ACME_CHALLENGE_PREFIX: &str = "/.well-known/acme-challenge/";

/// Handles incoming proxy connections
#[derive(Clone)]
pub struct ProxyHandler {
    routes: Arc<ArcSwap<RouteTable>>,
    proxy_service: ProxyService,
    acme_challenges: Option<AcmeChallenges>,
}

impl ProxyHandler {
    pub fn new(routes: Arc<ArcSwap<RouteTable>>) -> Self {
        Self {
            routes,
            proxy_service: ProxyService::new(),
            acme_challenges: None,
        }
    }

    /// Create a new handler with ACME challenge support
    pub fn with_acme(mut self, challenges: AcmeChallenges) -> Self {
        self.acme_challenges = Some(challenges);
        self
    }

    /// Handle a single TCP connection
    pub async fn handle_connection(
        &self,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let io = TokioIo::new(stream);
        let handler = self.clone();

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

    /// Handle a TLS connection
    pub async fn handle_tls_connection(
        &self,
        stream: TlsStream<TcpStream>,
        remote_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let io = TokioIo::new(stream);
        let handler = self.clone();

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
        let path = uri.path();

        // Check for ACME HTTP-01 challenge
        if let Some(response) = self.handle_acme_challenge(path) {
            return Ok(response);
        }

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
                // Check HTTP Basic Auth if enabled (but bypass for health check path)
                if backend.basic_auth.enabled {
                    let is_healthcheck = backend
                        .healthcheck_path
                        .as_ref()
                        .map(|p| path == p)
                        .unwrap_or(false);

                    if !is_healthcheck {
                        if let Err(response) = self.check_basic_auth(&req, &backend) {
                            debug!(
                                host = ?host,
                                path = %path,
                                "Basic auth required but not provided or invalid"
                            );
                            return Ok(response);
                        }
                    }
                }

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

    /// Handle ACME HTTP-01 challenge requests
    fn handle_acme_challenge(&self, path: &str) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
        // Check if this is an ACME challenge request
        if !path.starts_with(ACME_CHALLENGE_PREFIX) {
            return None;
        }

        // Extract the token from the path
        let token = &path[ACME_CHALLENGE_PREFIX.len()..];

        if token.is_empty() {
            return Some(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from("Missing token")).map_err(|e| match e {}).boxed())
                    .unwrap(),
            );
        }

        // Look up the challenge
        let challenges = self.acme_challenges.as_ref()?;

        match challenges.get(token) {
            Some(key_auth) => {
                info!(token = %token, "Serving ACME challenge");
                Some(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("Content-Type", "text/plain")
                        .body(Full::new(Bytes::from(key_auth)).map_err(|e| match e {}).boxed())
                        .unwrap(),
                )
            }
            None => {
                debug!(token = %token, "ACME challenge not found");
                Some(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from("Challenge not found")).map_err(|e| match e {}).boxed())
                        .unwrap(),
                )
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

    /// Check HTTP Basic Auth credentials
    /// Returns Ok(()) if auth is valid, or Err(response) with 401 response
    fn check_basic_auth<T>(
        &self,
        req: &Request<T>,
        backend: &Backend,
    ) -> Result<(), Response<BoxBody<Bytes, hyper::Error>>> {
        // Get Authorization header
        let auth_header = req.headers().get(AUTHORIZATION).and_then(|h| h.to_str().ok());

        let credentials = match auth_header {
            Some(header) if header.starts_with("Basic ") => {
                // Decode base64 credentials
                let encoded = &header[6..];
                match BASE64.decode(encoded) {
                    Ok(decoded) => String::from_utf8(decoded).ok(),
                    Err(_) => None,
                }
            }
            _ => None,
        };

        // Parse username:password
        let (username, password) = match credentials {
            Some(creds) => {
                if let Some((user, pass)) = creds.split_once(':') {
                    (user.to_string(), pass.to_string())
                } else {
                    return Err(self.unauthorized_response("Protected Application"));
                }
            }
            None => {
                return Err(self.unauthorized_response("Protected Application"));
            }
        };

        // Verify credentials
        let expected_username = backend.basic_auth.username.as_deref().unwrap_or("");
        let password_hash = backend.basic_auth.password_hash.as_deref().unwrap_or("");

        // Check username
        if username != expected_username {
            return Err(self.unauthorized_response("Protected Application"));
        }

        // Verify password against hash
        let parsed_hash = match PasswordHash::new(password_hash) {
            Ok(h) => h,
            Err(_) => {
                error!("Invalid password hash in backend config");
                return Err(self.unauthorized_response("Protected Application"));
            }
        };

        if Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(self.unauthorized_response("Protected Application"));
        }

        Ok(())
    }

    /// Create a 401 Unauthorized response with WWW-Authenticate header
    fn unauthorized_response(&self, realm: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <title>401 Unauthorized - Rivetr</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: #f5f5f5;
        }
        .error {
            text-align: center;
            padding: 40px;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 { color: #e74c3c; margin-bottom: 10px; }
        p { color: #666; margin: 0; }
        .code { font-size: 48px; color: #333; margin-bottom: 20px; }
    </style>
</head>
<body>
    <div class="error">
        <div class="code">401</div>
        <h1>Unauthorized</h1>
        <p>This application requires authentication.</p>
        <p style="margin-top: 10px; font-size: 12px;">Powered by Rivetr</p>
    </div>
</body>
</html>"#;

        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(WWW_AUTHENTICATE, format!("Basic realm=\"{}\"", realm))
            .header("Content-Type", "text/html; charset=utf-8")
            .header("X-Powered-By", "Rivetr")
            .body(Full::new(Bytes::from(body)).map_err(|e| match e {}).boxed())
            .unwrap()
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
