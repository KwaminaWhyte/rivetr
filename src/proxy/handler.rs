// Proxy connection handler
//
// Handles incoming HTTP connections, parses requests, and forwards them to backends.
// Supports WebSocket upgrade for real-time applications.

use arc_swap::ArcSwap;
use argon2::{password_hash::phc::PasswordHash, Argon2, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{AUTHORIZATION, CONNECTION, UPGRADE, WWW_AUTHENTICATE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use regex::Regex;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;
use tracing::{debug, error, info, warn};

use super::acme::AcmeChallenges;
use super::{Backend, ProxyService, RouteTable};

/// ACME HTTP-01 challenge path prefix
const ACME_CHALLENGE_PREFIX: &str = "/.well-known/acme-challenge/";

/// Capacity of the access-log queue; entries are dropped (and counted) when full.
const LOG_QUEUE_CAPACITY: usize = 10_000;

/// Max rows per batched INSERT (7 binds per row stays far below SQLite's bind limit).
const LOG_BATCH_SIZE: usize = 200;

/// Max time a queued entry waits before being flushed.
const LOG_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

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

/// Request metadata captured up front; only built when access logging is enabled.
struct LogMeta {
    method: String,
    path: String,
    client_ip: String,
    user_agent: String,
}

/// Handle used by request tasks to enqueue access-log entries for the writer task.
#[derive(Clone)]
struct ProxyLogSender {
    tx: mpsc::Sender<ProxyLogEntry>,
    dropped: Arc<AtomicU64>,
}

impl ProxyLogSender {
    /// Spawn the single batching writer task and return a sender for it.
    fn spawn(db: sqlx::SqlitePool) -> Self {
        let (tx, rx) = mpsc::channel(LOG_QUEUE_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        tokio::spawn(run_log_writer(db, rx, dropped.clone()));
        Self { tx, dropped }
    }

    /// Enqueue without blocking; drops the entry when the queue is full.
    fn send(&self, entry: ProxyLogEntry) {
        if self.tx.try_send(entry).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Single writer: batches queued entries into multi-row INSERTs, flushing every
/// `LOG_FLUSH_INTERVAL` or `LOG_BATCH_SIZE` entries, whichever comes first.
async fn run_log_writer(
    db: sqlx::SqlitePool,
    mut rx: mpsc::Receiver<ProxyLogEntry>,
    dropped: Arc<AtomicU64>,
) {
    let mut buf: Vec<ProxyLogEntry> = Vec::with_capacity(LOG_BATCH_SIZE);
    let mut tick = tokio::time::interval(LOG_FLUSH_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(entry) => {
                    buf.push(entry);
                    if buf.len() >= LOG_BATCH_SIZE {
                        flush_log_batch(&db, &mut buf).await;
                    }
                }
                None => {
                    flush_log_batch(&db, &mut buf).await;
                    break;
                }
            },
            _ = tick.tick() => {
                flush_log_batch(&db, &mut buf).await;
                let n = dropped.swap(0, Ordering::Relaxed);
                if n > 0 {
                    warn!(dropped = n, "Proxy access log queue full; entries dropped");
                }
            }
        }
    }
}

/// Insert all buffered entries with one multi-row statement and clear the buffer.
async fn flush_log_batch(db: &sqlx::SqlitePool, buf: &mut Vec<ProxyLogEntry>) {
    if buf.is_empty() {
        return;
    }
    let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
        "INSERT INTO proxy_logs (host, method, path, status, response_ms, bytes_out, client_ip, user_agent) ",
    );
    qb.push_values(buf.drain(..), |mut row, e| {
        row.push_bind(e.host)
            .push_bind(e.method)
            .push_bind(e.path)
            .push_bind(e.status as i64)
            .push_bind(e.response_ms as i64)
            .push("0")
            .push_bind(if e.client_ip.is_empty() {
                None
            } else {
                Some(e.client_ip)
            })
            .push_bind(if e.user_agent.is_empty() {
                None
            } else {
                Some(e.user_agent)
            });
    });
    if let Err(e) = qb.build().execute(db).await {
        debug!(error = %e, "Failed to write proxy access log batch");
    }
}

/// Compiled redirect-rule regexes keyed by pattern; `None` marks an invalid pattern
/// so it is reported once and skipped on later requests.
static REDIRECT_REGEX_CACHE: OnceLock<DashMap<String, Option<Regex>>> = OnceLock::new();

/// Upper bound on cached patterns; the cache is cleared if exceeded (rules changed a lot).
const REDIRECT_REGEX_CACHE_MAX: usize = 1024;

fn redirect_regex(pattern: &str) -> Option<Regex> {
    let cache = REDIRECT_REGEX_CACHE.get_or_init(DashMap::new);
    if let Some(entry) = cache.get(pattern) {
        return entry.value().clone();
    }
    if cache.len() >= REDIRECT_REGEX_CACHE_MAX {
        cache.clear();
    }
    let compiled = match Regex::new(pattern) {
        Ok(r) => Some(r),
        Err(e) => {
            warn!(pattern = %pattern, error = %e, "Invalid redirect rule regex");
            None
        }
    };
    cache.insert(pattern.to_string(), compiled.clone());
    compiled
}

/// Whether a request should be access-logged: the writer must be running and the
/// matched backend must not have opted out (unmatched hosts keep logging).
fn should_log(writer_enabled: bool, backend_flag: Option<bool>) -> bool {
    writer_enabled && backend_flag.unwrap_or(true)
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
    /// Queue to the access-log writer task (None when logging is disabled)
    log_tx: Option<ProxyLogSender>,
    /// Scheme reported to backends via X-Forwarded-Proto ("http" or "https").
    /// The HTTPS listener sets this to "https" so upstream apps (Laravel, Rails,
    /// etc.) generate correct absolute URLs instead of insecure http:// ones.
    forwarded_proto: &'static str,
}

impl ProxyHandler {
    pub fn new(routes: Arc<ArcSwap<RouteTable>>) -> Self {
        Self {
            routes,
            proxy_service: ProxyService::new(),
            acme_challenges: None,
            https_redirect_port: None,
            https_redirect_enabled: None,
            log_tx: None,
            forwarded_proto: "http",
        }
    }

    /// Enable proxy access logging by providing a database pool
    pub fn with_db(mut self, db: sqlx::SqlitePool) -> Self {
        self.log_tx = Some(ProxyLogSender::spawn(db));
        self
    }

    /// Mark this handler as serving TLS, so forwarded requests report
    /// X-Forwarded-Proto: https to backends.
    pub fn with_tls(mut self) -> Self {
        self.forwarded_proto = "https";
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

    /// Queue a proxy access log entry for the batching writer (non-blocking)
    fn log_request(
        &self,
        meta: Option<LogMeta>,
        host: Option<&str>,
        status: u16,
        response_ms: u64,
    ) {
        if let (Some(tx), Some(meta)) = (&self.log_tx, meta) {
            tx.send(ProxyLogEntry {
                host: host.unwrap_or("unknown").to_string(),
                method: meta.method,
                path: meta.path,
                status,
                response_ms,
                client_ip: meta.client_ip,
                user_agent: meta.user_agent,
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
                    .unwrap_or_else(|_| empty_response(StatusCode::BAD_REQUEST));
                return Ok(response);
            }
        } // end redirect_active

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

        // Capture logging metadata from request headers before they are consumed
        // (skipped entirely when the writer is off or the backend opted out)
        let log_meta = should_log(
            self.log_tx.is_some(),
            backend.as_ref().map(|b| b.access_logging),
        )
        .then(|| LogMeta {
            method: method.to_string(),
            path: path.to_string(),
            client_ip: req
                .headers()
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| remote_addr.ip().to_string()),
            user_agent: req
                .headers()
                .get(hyper::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string(),
        });

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
                        .unwrap_or_else(|_| empty_response(StatusCode::BAD_REQUEST))
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
                                self.log_request(
                                    log_meta,
                                    host.as_deref(),
                                    response.status().as_u16(),
                                    ms,
                                );
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
                            self.log_request(
                                log_meta,
                                host.as_deref(),
                                redirect_response.status().as_u16(),
                                ms,
                            );
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

                    // Counted so a deploy can drain this backend before stopping it.
                    let _in_flight = super::inflight::track(&backend.addr());
                    match self
                        .proxy_service
                        .forward(req, &backend, self.forwarded_proto)
                        .await
                    {
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
        self.log_request(log_meta, host.as_deref(), status, ms);
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
        match self
            .proxy_service
            .forward_websocket(req, backend, self.forwarded_proto)
            .await
        {
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
                    .unwrap_or_else(|_| empty_response(StatusCode::BAD_REQUEST)),
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
                        .unwrap_or_else(|_| empty_response(StatusCode::OK)),
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
                        .unwrap_or_else(|_| empty_response(StatusCode::NOT_FOUND)),
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
            let Some(re) = redirect_regex(&rule.source_pattern) else {
                continue;
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
                    .unwrap_or_else(|_| empty_response(StatusCode::BAD_REQUEST));

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
            .unwrap_or_else(|_| empty_response(StatusCode::UNAUTHORIZED))
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
            .unwrap_or_else(|_| empty_response(status))
    }
}

/// Build a header-less response with just a status and empty body.
///
/// `Response::new` cannot fail (no header/status validation), so this is a
/// guaranteed-safe fallback for `Response::builder()` calls whose header values
/// come from untrusted input (Host, path, redirect target, auth realm) — a
/// crafted control character would otherwise make the builder error and the
/// old `.unwrap()` panic, dropping the connection.
fn empty_response(status: StatusCode) -> Response<BoxBody<Bytes, hyper::Error>> {
    let mut resp = Response::new(Full::new(Bytes::new()).map_err(|e| match e {}).boxed());
    *resp.status_mut() = status;
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn should_log_respects_writer_and_backend_flag() {
        assert!(should_log(true, Some(true)));
        assert!(should_log(true, None));
        assert!(!should_log(true, Some(false)));
        assert!(!should_log(false, Some(true)));
        assert!(!should_log(false, None));
    }

    #[test]
    fn backend_access_logging_defaults_on_and_toggles() {
        let b = Backend::new("c".into(), "127.0.0.1".into(), 3000);
        assert!(b.access_logging);
        assert!(!b.with_access_logging(false).access_logging);
    }

    fn entry(path: &str, ip: &str) -> ProxyLogEntry {
        ProxyLogEntry {
            host: "example.com".into(),
            method: "GET".into(),
            path: path.into(),
            status: 200,
            response_ms: 5,
            client_ip: ip.into(),
            user_agent: String::new(),
        }
    }

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../migrations/103_proxy_logs.sql"))
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn flush_batch_inserts_rows_and_maps_empty_to_null() {
        let pool = test_pool().await;
        let mut buf = vec![entry("/a", "1.2.3.4"), entry("/b", "")];
        flush_log_batch(&pool, &mut buf).await;
        assert!(buf.is_empty());

        let rows: Vec<(String, i64, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT path, bytes_out, client_ip, user_agent FROM proxy_logs ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("/a".into(), 0, Some("1.2.3.4".into()), None));
        assert_eq!(rows[1].2, None);
    }

    #[tokio::test]
    async fn writer_flushes_on_interval_and_on_close() {
        let pool = test_pool().await;
        let sender = ProxyLogSender::spawn(pool.clone());
        sender.send(entry("/x", "9.9.9.9"));
        // Flushed by the 1s tick.
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proxy_logs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn full_queue_drops_and_counts() {
        let (tx, _rx) = mpsc::channel(1);
        let sender = ProxyLogSender {
            tx,
            dropped: Arc::new(AtomicU64::new(0)),
        };
        sender.send(entry("/1", ""));
        sender.send(entry("/2", ""));
        sender.send(entry("/3", ""));
        assert_eq!(sender.dropped.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn redirect_regex_caches_and_skips_invalid() {
        assert!(redirect_regex("^/old/(.*)$").is_some());
        assert!(redirect_regex("^/old/(.*)$").is_some());
        assert!(redirect_regex("(unclosed").is_none());
        assert!(redirect_regex("(unclosed").is_none());
    }
}
