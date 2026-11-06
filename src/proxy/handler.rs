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
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
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

/// Max concurrent HTTP/2 streams per client connection.
const H2_MAX_CONCURRENT_STREAMS: u32 = 250;

/// Which HTTP versions a connection may speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HttpVersions {
    /// Sniff the connection preface: HTTP/1.1 or HTTP/2 prior knowledge.
    Auto,
    Http1,
    Http2,
}

impl HttpVersions {
    /// Map the ALPN protocol a TLS client negotiated to the versions to serve.
    pub(crate) fn from_alpn(alpn: Option<&[u8]>) -> Self {
        match alpn {
            Some(b"h2") => Self::Http2,
            _ => Self::Http1,
        }
    }
}

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

/// Request fields captured for traffic analytics (only when analytics is active).
struct AnalyticsMeta {
    path: String,
    client_ip: String,
    user_agent: String,
    referrer: String,
    bytes_in: u64,
}

fn content_length(headers: &hyper::HeaderMap) -> u64 {
    headers
        .get(hyper::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Hand a finished request to the analytics aggregator (non-blocking).
fn record_analytics(
    meta: Option<AnalyticsMeta>,
    host: Option<&str>,
    status: u16,
    ms: u64,
    bytes_out: u64,
) {
    if let (Some(m), Some(host)) = (meta, host) {
        super::analytics::record(super::analytics::TrafficEvent {
            host: host.to_string(),
            path: m.path,
            status,
            latency_ms: ms,
            bytes_in: m.bytes_in,
            bytes_out,
            client_ip: m.client_ip,
            user_agent: m.user_agent,
            referrer: m.referrer,
        });
    }
}

/// Analytics fields waiting for the response body to finish.
struct PendingAnalytics {
    meta: AnalyticsMeta,
    host: String,
    status: u16,
    ms: u64,
}

/// Response body that counts the bytes it streams and records the analytics event
/// exactly once: when the body ends, fails, or is dropped (client went away).
struct CountingBody {
    inner: BoxBody<Bytes, hyper::Error>,
    bytes: u64,
    pending: Option<PendingAnalytics>,
}

impl CountingBody {
    fn finish(&mut self) {
        if let Some(p) = self.pending.take() {
            record_analytics(Some(p.meta), Some(&p.host), p.status, p.ms, self.bytes);
        }
    }
}

impl hyper::body::Body for CountingBody {
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Bytes>, hyper::Error>>> {
        let this = &mut *self;
        let res = std::pin::Pin::new(&mut this.inner).poll_frame(cx);
        match &res {
            std::task::Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    this.bytes += data.len() as u64;
                }
            }
            std::task::Poll::Ready(Some(Err(_))) | std::task::Poll::Ready(None) => this.finish(),
            std::task::Poll::Pending => {}
        }
        res
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for CountingBody {
    fn drop(&mut self) {
        self.finish();
    }
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
/// Value sent in `X-Robots-Tag` for domains with search indexing turned off.
pub const NOINDEX_HEADER_VALUE: &str = "noindex, nofollow";

/// Add `X-Robots-Tag: noindex, nofollow` when the domain has search indexing off.
/// Replaces any value the app set, so the domain setting always wins.
pub fn apply_robots_header<B>(response: &mut Response<B>, noindex: bool) {
    if noindex {
        response.headers_mut().insert(
            "x-robots-tag",
            hyper::header::HeaderValue::from_static(NOINDEX_HEADER_VALUE),
        );
    }
}

/// Status for the www redirect: 301 when permanent, 302 otherwise.
fn www_redirect_status(permanent: bool) -> StatusCode {
    if permanent {
        StatusCode::MOVED_PERMANENTLY
    } else {
        StatusCode::FOUND
    }
}

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
    /// When set, HTTP is redirected to HTTPS only for hosts with a certificate.
    cert_store: Option<Arc<super::CertStore>>,
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
            cert_store: None,
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

    /// Limit the HTTPS redirect to hosts `store` has a real certificate for, so
    /// a domain whose certificate is still pending stays reachable over HTTP.
    pub fn with_cert_store(mut self, store: Arc<super::CertStore>) -> Self {
        self.cert_store = Some(store);
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

    /// Handle a plain TCP connection: HTTP/1.1 (with upgrades, so WebSockets
    /// work) plus HTTP/2 with prior knowledge (h2c), detected from the preface.
    pub async fn handle_connection(
        &self,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        self.serve(TokioIo::new(stream), remote_addr, HttpVersions::Auto)
            .await
    }

    /// Handle a TLS connection. The protocol follows ALPN: `h2` serves HTTP/2,
    /// anything else (including no ALPN) serves HTTP/1.1 with upgrades.
    pub async fn handle_tls_connection(
        &self,
        stream: TlsStream<TcpStream>,
        remote_addr: SocketAddr,
    ) -> anyhow::Result<()> {
        let versions = HttpVersions::from_alpn(stream.get_ref().1.alpn_protocol());
        self.serve(TokioIo::new(stream), remote_addr, versions)
            .await
    }

    async fn serve<I>(
        &self,
        io: I,
        remote_addr: SocketAddr,
        versions: HttpVersions,
    ) -> anyhow::Result<()>
    where
        I: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
    {
        let handler = self.clone();
        let service = service_fn(move |req| {
            let handler = handler.clone();
            async move { handler.handle_request(req, remote_addr).await }
        });

        let mut builder = auto::Builder::new(TokioExecutor::new());
        builder
            .http2()
            .max_concurrent_streams(H2_MAX_CONCURRENT_STREAMS);
        let builder = match versions {
            HttpVersions::Auto => builder,
            HttpVersions::Http1 => builder.http1_only(),
            HttpVersions::Http2 => builder.http2_only(),
        };
        builder
            .serve_connection_with_upgrades(io, service)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

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
        mut req: Request<Incoming>,
        remote_addr: SocketAddr,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let start = Instant::now();
        let method = req.method().clone();
        let uri = req.uri().clone();
        let path = uri.path();

        // Keep the peer address on the request so the forwarding code can put it
        // in X-Forwarded-For. Added even for the early returns below, harmless there.
        req.extensions_mut().insert(super::ClientAddr(remote_addr));

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
        // HTTP/2 (h2c) requests carry the host in the URI authority.
        let request_host = self.extract_host(&req).unwrap_or_default();
        let redirect_active = redirect_active
            && self
                .cert_store
                .as_ref()
                .is_none_or(|store| store.covers(&request_host));
        if redirect_active {
            if let Some(https_port) = self.https_redirect_port {
                let host_header = request_host.as_str();
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

        // Look up the backend (pooled domains apply their balancing policy and
        // the sticky session cookie here)
        let sticky_cookie = req
            .headers()
            .get(hyper::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|c| super::balancer::cookie_value(c, super::balancer::STICKY_COOKIE));
        let (backend, sticky_addr) = match host
            .as_deref()
            .and_then(|h| routes.select_backend(h, sticky_cookie))
        {
            Some(r) => (Some(r.backend), r.sticky_addr),
            None => (None, None),
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

        // Analytics fields, independent of the per-app access log switch.
        let analytics_meta = super::analytics::active().then(|| AnalyticsMeta {
            path: path.to_string(),
            client_ip: log_meta
                .as_ref()
                .map(|m| m.client_ip.clone())
                .unwrap_or_else(|| {
                    req.headers()
                        .get("X-Forwarded-For")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| remote_addr.ip().to_string())
                }),
            user_agent: req
                .headers()
                .get(hyper::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string(),
            referrer: req
                .headers()
                .get(hyper::header::REFERER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string(),
            bytes_in: content_length(req.headers()),
        });

        let response = match backend {
            Some(backend) if backend.healthy => {
                // If this backend is a www-redirect proxy, redirect (301 or 302 per domain)
                if let Some(ref target_host) = backend.www_redirect_target {
                    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
                    let location = format!("https://{}{}{}", target_host, path, query);
                    debug!(
                        from = ?host,
                        to = %target_host,
                        "www redirect"
                    );
                    Response::builder()
                        .status(www_redirect_status(backend.permanent_redirect))
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
                                record_analytics(
                                    analytics_meta,
                                    host.as_deref(),
                                    response.status().as_u16(),
                                    ms,
                                    content_length(response.headers()),
                                );
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
                            record_analytics(
                                analytics_meta,
                                host.as_deref(),
                                redirect_response.status().as_u16(),
                                ms,
                                0,
                            );
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
                        Ok(mut response) => {
                            apply_robots_header(&mut response, backend.noindex);
                            if let Some(addr) = sticky_addr.as_deref() {
                                super::balancer::append_sticky_cookie(
                                    &mut response,
                                    addr,
                                    self.forwarded_proto == "https",
                                );
                            }
                            response
                        }
                        Err(e) => {
                            error!(error = %e, backend = %backend.addr(), "Backend request failed");
                            // Passive health: a pooled upstream that fails is skipped for a while.
                            if let Some(h) = host.as_deref() {
                                routes.mark_upstream_failed(h, &backend.addr());
                            }
                            self.error_response(
                                StatusCode::BAD_GATEWAY,
                                "The application did not respond. It may be starting, crashed, or still deploying.",
                            )
                        }
                    }
                }
            }
            Some(_) => {
                warn!(host = ?host, "Backend is unhealthy");
                self.error_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "The application is temporarily unavailable. It may be restarting or failing its health check.",
                )
            }
            None => {
                warn!(host = ?host, "No backend found for host");
                self.error_response(
                    StatusCode::NOT_FOUND,
                    &format!(
                        "No application is configured for {} yet.",
                        host.as_deref().unwrap_or("this address")
                    ),
                )
            }
        };

        let ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        self.log_request(log_meta, host.as_deref(), status, ms);
        // Count the bytes actually streamed (most app responses are chunked, so the
        // Content-Length header alone undercounts); the event is recorded when the
        // body ends or the client goes away.
        let response = match (analytics_meta, host) {
            (Some(meta), Some(host)) => {
                let (parts, body) = response.into_parts();
                let counted = CountingBody {
                    inner: body,
                    bytes: 0,
                    pending: Some(PendingAnalytics {
                        meta,
                        host,
                        status,
                        ms,
                    }),
                };
                Response::from_parts(parts, counted.boxed())
            }
            _ => response,
        };
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
                Ok(self.error_response(
                    StatusCode::BAD_GATEWAY,
                    "The WebSocket connection to the application could not be established.",
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
        let body = error_page(
            StatusCode::UNAUTHORIZED,
            "This application requires a username and password.",
        );

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
        let body = error_page(status, message);

        Response::builder()
            .status(status)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("X-Powered-By", "Rivetr")
            .body(Full::new(Bytes::from(body)).map_err(|e| match e {}).boxed())
            .unwrap_or_else(|_| empty_response(status))
    }
}

/// Escape text that goes into HTML content or an attribute value.
///
/// The proxy error page prints the request's `Host`, which any client controls,
/// so the message cannot be interpolated raw.
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The page the proxy serves when it cannot reach an application.
///
/// Plain HTML on purpose: it is rendered by the proxy, which cannot depend on
/// the dashboard bundle. It follows the dashboard's light and dark appearance
/// through `prefers-color-scheme`.
fn error_page(status: StatusCode, message: &str) -> String {
    let code = status.as_u16();
    let reason = escape_html(status.canonical_reason().unwrap_or("Request failed"));
    let detail = escape_html(message);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{code} {reason} - Rivetr</title>
<style>
:root {{
  color-scheme: light dark;
  --bg: #fafafa; --card: #ffffff; --fg: #18181b; --muted: #71717a;
  --border: #e4e4e7; --chip: #f4f4f5; --dot: #ef4444;
}}
@media (prefers-color-scheme: dark) {{
  :root {{ --bg: #09090b; --card: #18181b; --fg: #fafafa; --muted: #a1a1aa;
           --border: #27272a; --chip: #27272a; --dot: #f87171; }}
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0; min-height: 100vh; display: flex; align-items: center; justify-content: center;
  padding: 24px; background: var(--bg); color: var(--fg);
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
}}
.card {{
  width: 100%; max-width: 440px; background: var(--card); border: 1px solid var(--border);
  border-radius: 12px; padding: 32px; text-align: center;
}}
.chip {{
  display: inline-flex; align-items: center; gap: 6px; margin-bottom: 20px;
  padding: 4px 10px; border: 1px solid var(--border); border-radius: 999px;
  background: var(--chip); color: var(--muted); font-size: 12px; font-weight: 500;
}}
.chip span {{ width: 6px; height: 6px; border-radius: 999px; background: var(--dot); display: inline-block; }}
h1 {{ margin: 0 0 8px; font-size: 20px; line-height: 1.3; letter-spacing: -0.01em; }}
p {{ margin: 0; color: var(--muted); font-size: 14px; line-height: 1.6; overflow-wrap: anywhere; }}
footer {{ margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--border); color: var(--muted); font-size: 12px; }}
</style>
</head>
<body>
  <main class="card">
    <div class="chip"><span></span>{code}</div>
    <h1>{reason}</h1>
    <p>{detail}</p>
    <footer>Rivetr could not complete this request.</footer>
  </main>
</body>
</html>"#
    )
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
    #[test]
    fn noindex_header_is_added_only_when_indexing_is_off() {
        let mut resp = Response::new(());
        super::apply_robots_header(&mut resp, false);
        assert!(resp.headers().get("x-robots-tag").is_none());

        let mut resp = Response::builder()
            .header("X-Robots-Tag", "all")
            .body(())
            .unwrap();
        super::apply_robots_header(&mut resp, true);
        let values: Vec<_> = resp.headers().get_all("x-robots-tag").iter().collect();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], super::NOINDEX_HEADER_VALUE);
    }

    #[test]
    fn error_page_escapes_the_host_in_the_message() {
        let page = super::error_page(
            StatusCode::NOT_FOUND,
            "No application is configured for <script>alert(1)</script> yet.",
        );
        assert!(!page.contains("<script>"));
        assert!(page.contains("&lt;script&gt;"));
        assert!(page.contains("<title>404 Not Found - Rivetr</title>"));
        assert!(page.contains("<h1>Not Found</h1>"));
    }

    #[test]
    fn escape_html_covers_every_dangerous_character() {
        assert_eq!(
            super::escape_html("a & b < c > d \" e ' f"),
            "a &amp; b &lt; c &gt; d &quot; e &#39; f"
        );
    }

    #[test]
    fn www_redirect_status_follows_redirect_type() {
        assert_eq!(
            super::www_redirect_status(true),
            StatusCode::MOVED_PERMANENTLY
        );
        assert_eq!(super::www_redirect_status(false), StatusCode::FOUND);
    }

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

#[cfg(test)]
mod counting_body_tests {
    use super::*;
    use futures::stream;
    use http_body_util::StreamBody;

    #[tokio::test]
    async fn counts_every_streamed_chunk() {
        let chunks: Vec<Result<hyper::body::Frame<Bytes>, hyper::Error>> = vec![
            Ok(hyper::body::Frame::data(Bytes::from(vec![0u8; 1000]))),
            Ok(hyper::body::Frame::data(Bytes::from(vec![0u8; 2500]))),
            Ok(hyper::body::Frame::data(Bytes::from_static(b"end"))),
        ];
        let inner = StreamBody::new(stream::iter(chunks)).boxed();
        let mut body = CountingBody {
            inner,
            bytes: 0,
            pending: None,
        };
        while let Some(frame) = body.frame().await {
            frame.unwrap();
        }
        assert_eq!(body.bytes, 3503);
    }
}
