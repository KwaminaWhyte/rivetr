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
use regex::Regex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tracing::{debug, error, info, warn};

use super::acme::AcmeChallenges;
use super::{Backend, ProxyService, RouteTable};

/// ACME HTTP-01 challenge path prefix
const ACME_CHALLENGE_PREFIX: &str = "/.well-known/acme-challenge/";

/// Owned fields needed to write a proxy access log row.
struct ProxyLogEntry {
    host: String,
    method: String,
    path: String,
    status: u16,
    response_ms: u64,
    client_ip: String,
    user_agent: String,
}

/// Handles incoming proxy connections
#[derive(Clone)]
pub struct ProxyHandler {
    routes: Arc<ArcSwap<RouteTable>>,
    proxy_service: ProxyService,
    acme_challenges: Option<AcmeChallenges>,
    /// If set, redirect HTTP to HTTPS on this port (only when flag is true)
    https_redirect_port: Option<u16>,
    /// Runtime flag — set to true after TLS cert is confirmed available
    https_redirect_enabled: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// Optional database pool for proxy access logging
    db: Option<sqlx::SqlitePool>,
}

impl ProxyHandler {
    pub fn new(routes: Arc<ArcSwap<RouteTable>>) -> Self {
        Self {
            routes,
            proxy_service: ProxyService::new(),
            acme_challenges: None,
            https_redirect_port: None,
            https_redirect_enabled: None,
            db: None,
        }
    }

    /// Enable proxy access logging by providing a database pool
    pub fn with_db(mut self, db: sqlx::SqlitePool) -> Self {
        self.db = Some(db);
        self
    }

    /// Create a new handler with ACME challenge support
    pub fn with_acme(mut self, challenges: AcmeChallenges) -> Self {
        self.acme_challenges = Some(challenges);
        self
    }

    /// Redirect all HTTP traffic to HTTPS (only when the flag is set to true)
    pub fn with_https_redirect(
        mut self,
        https_port: u16,
        enabled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        self.https_redirect_port = Some(https_port);
        self.https_redirect_enabled = Some(enabled);
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

    /// Write a proxy access log entry to the database (fire-and-forget)
    fn log_request(&self, entry: ProxyLogEntry) {
        if let Some(ref db) = self.db {
            let db = db.clone();
            tokio::spawn(async move {
                let ProxyLogEntry {
                    host,
                    method,
                    path,
                    status,
                    response_ms,
                    client_ip,
                    user_agent,
                } = entry;
                let _ = sqlx::query(
                    "INSERT INTO proxy_logs (host, method, path, status, response_ms, bytes_out, client_ip, user_agent) \
                     VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
                )
                .bind(&host)
                .bind(&method)
                .bind(&path)
                .bind(status as i64)
                .bind(response_ms as i64)
                .bind(if client_ip.is_empty() { None } else { Some(client_ip) })
                .bind(if user_agent.is_empty() { None } else { Some(user_agent) })
                .execute(&db)
                .await;
            });
        }
    }

    /// Handle a single HTTP request
    async fn handle_request(
        &self,
        req: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let start = Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();
        let path = uri.path();

        // Capture logging metadata from request headers before they are consumed
        let log_method = method.to_string();
        let log_path = path.to_string();
        let log_client_ip = req
            .headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| remote_addr.ip().to_string());
        let log_user_agent = req
            .headers()
            .get(hyper::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Check for ACME HTTP-01 challenge (must happen before HTTPS redirect)
        if let Some(response) = self.handle_acme_challenge(path) {
            return Ok(response);
        }

        // Redirect HTTP → HTTPS only if configured AND TLS cert is actually available.
        // Never redirect webhook paths — webhook providers (GitHub, GitLab, etc.) do NOT
        // follow HTTP redirects, so the delivery would silently fail.
        let is_webhook_path = path.starts_with("/webhooks/");
        let redirect_active = !is_webhook_path
            && self
                .https_redirect_enabled
                .as_ref()
                .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
                .unwrap_or(false);
        if redirect_active {
            if let Some(https_port) = self.https_redirect_port {
                let host_header = req
                    .headers()
                    .get(hyper::header::HOST)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                // Strip existing port from host header, add https port if non-standard
                let host_without_port = host_header.split(':').next().unwrap_or(host_header);
                let redirect_host = if https_port == 443 {
                    host_without_port.to_string()
                } else {
                    format!("{}:{}", host_without_port, https_port)
                };
                let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
                let location = format!("https://{}{}{}", redirect_host, path, query);
                let response = Response::builder()
                    .status(hyper::StatusCode::MOVED_PERMANENTLY)
                    .header(hyper::header::LOCATION, location)
                    .body(Full::new(Bytes::new()).map_err(|e| match e {}).boxed())
                    .unwrap();
                return Ok(response);
            }
        } // end redirect_active

        let host = self.extract_host(&req);
        let is_websocket = self.is_websocket_upgrade(&req);
        let log_host = host.clone().unwrap_or_else(|| "unknown".to_string());

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

        let response = match backend {
            Some(backend) if backend.healthy => {
                // If this backend is a www-redirect proxy, issue a permanent redirect
                if let Some(ref target_host) = backend.www_redirect_target {
                    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
                    let location = format!("https://{}{}{}", target_host, path, query);
                    debug!(
                        from = ?host,
                        to = %target_host,
                        "www redirect"
                    );
                    Response::builder()
                        .status(hyper::StatusCode::MOVED_PERMANENTLY)
                        .header(hyper::header::LOCATION, location)
                        .header("X-Powered-By", "Rivetr")
                        .body(Full::new(Bytes::new()).map_err(|e| match e {}).boxed())
                        .unwrap()
                } else {
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
                                let ms = start.elapsed().as_millis() as u64;
                                self.log_request(ProxyLogEntry {
                                    host: log_host,
                                    method: log_method,
                                    path: log_path,
                                    status: response.status().as_u16(),
                                    response_ms: ms,
                                    client_ip: log_client_ip,
                                    user_agent: log_user_agent,
                                });
                                return Ok(response);
                            }
                        }
                    }

                    // Apply redirect rules (evaluated before forwarding)
                    if !backend.redirect_rules.is_empty() {
                        if let Some(redirect_response) =
                            self.apply_redirect_rules(path, &backend.redirect_rules)
                        {
                            let ms = start.elapsed().as_millis() as u64;
                            self.log_request(ProxyLogEntry {
                                host: log_host,
                                method: log_method,
                                path: log_path,
                                status: redirect_response.status().as_u16(),
                                response_ms: ms,
                                client_ip: log_client_ip,
                                user_agent: log_user_agent,
                            });
                            return Ok(redirect_response);
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

                    // Handle WebSocket upgrades specially (skip logging for WS)
                    if is_websocket {
                        return self.handle_websocket_upgrade(req, &backend).await;
                    }

                    match self.proxy_service.forward(req, &backend).await {
                        Ok(response) => response,
                        Err(e) => {
                            error!(error = %e, backend = %backend.addr(), "Backend request failed");
                            self.error_response(StatusCode::BAD_GATEWAY, "Backend unavailable")
                        }
                    }
                }
            }
            Some(_) => {
                warn!(host = ?host, "Backend is unhealthy");
                self.error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Service temporarily unavailable",
                )
            }
            None => {
                warn!(host = ?host, "No backend found for host");
                self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!(
                        "No application found for host: {}",
                        host.as_deref().unwrap_or("unknown")
                    ),
                )
            }
        };

        let ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        self.log_request(ProxyLogEntry {
            host: log_host,
            method: log_method,
            path: log_path,
            status,
            response_ms: ms,
            client_ip: log_client_ip,
            user_agent: log_user_agent,
        });
        Ok(response)
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
                Ok(self.error_response(StatusCode::BAD_GATEWAY, "WebSocket upgrade failed"))
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
                    .body(
                        Full::new(Bytes::from("Missing token"))
                            .map_err(|e| match e {})
                            .boxed(),
                    )
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
                        .body(
                            Full::new(Bytes::from(key_auth))
                                .map_err(|e| match e {})
                                .boxed(),
                        )
                        .unwrap(),
                )
            }
            None => {
                debug!(token = %token, "ACME challenge not found");
                Some(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(
                            Full::new(Bytes::from("Challenge not found"))
                                .map_err(|e| match e {})
                                .boxed(),
                        )
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
    #[allow(clippy::result_large_err)]
    fn check_basic_auth<T>(
        &self,
        req: &Request<T>,
        backend: &Backend,
    ) -> Result<(), Response<BoxBody<Bytes, hyper::Error>>> {
        // Get Authorization header
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

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

    /// Apply redirect rules to the given request path.
    /// Returns `Some(redirect response)` if a rule matches, or `None` to continue proxying.
    fn apply_redirect_rules(
        &self,
        path: &str,
        rules: &[crate::proxy::RedirectRule],
    ) -> Option<Response<BoxBody<Bytes, hyper::Error>>> {
        for rule in rules {
            let re = match Regex::new(&rule.source_pattern) {
                Ok(r) => r,
                Err(e) => {
                    warn!(pattern = %rule.source_pattern, error = %e, "Invalid redirect rule regex");
                    continue;
                }
            };

            if re.is_match(path) {
                let destination = re.replace(path, rule.destination.as_str()).to_string();

                let status = if rule.is_permanent {
                    StatusCode::MOVED_PERMANENTLY
                } else {
                    StatusCode::FOUND
                };

                debug!(
                    path = %path,
                    destination = %destination,
                    permanent = rule.is_permanent,
                    "Redirect rule matched"
                );

                let response = Response::builder()
                    .status(status)
                    .header(hyper::header::LOCATION, &destination)
                    .header("X-Powered-By", "Rivetr")
                    .body(Full::new(Bytes::new()).map_err(|e| match e {}).boxed())
                    .unwrap();

                return Some(response);
            }
        }
        None
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
