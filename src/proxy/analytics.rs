//! Traffic analytics aggregator (E3).
//!
//! Hot path: the proxy calls [`record`] once per finished request. When no
//! server has analytics on, that is one relaxed atomic load. Otherwise the event
//! goes on a bounded channel with `try_send` and is dropped (and counted) when
//! the channel is full, so a slow database never slows the proxy down.
//!
//! The aggregator task resolves the host to an app and server, applies the
//! per-server toggle, rolls events into per-minute counters plus hourly top-N
//! breakdowns and daily visitor hashes in memory, and flushes every 60 seconds.
//! It also prunes rows older than the retention setting once an hour.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use arc_swap::ArcSwap;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Bounded queue between the proxy and the aggregator.
const QUEUE_CAPACITY: usize = 20_000;
/// How often in-memory rollups are written to SQLite.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(60);
/// Max distinct values kept per (host, dimension) between flushes; the rest
/// are counted under "(other)".
const MAX_DIM_VALUES: usize = 200;
/// Max visitor hashes held between flushes.
const MAX_VISITORS: usize = 200_000;
const MAX_PATH_LEN: usize = 200;

/// Upper bounds (ms) of the latency histogram bins; the last bin is overflow.
pub const LATENCY_BOUNDS_MS: [u64; 11] =
    [10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 30000];
pub const LATENCY_BINS: usize = LATENCY_BOUNDS_MS.len() + 1;

pub const LOCALHOST: &str = "localhost";
pub const SETTING_RETENTION: &str = "analytics_retention_days";
pub const SETTING_GEOIP: &str = "analytics_geoip_path";
pub const DEFAULT_RETENTION_DAYS: i64 = 30;

// ---------------------------------------------------------------------------
// Hot path
// ---------------------------------------------------------------------------

/// One finished proxied request.
#[derive(Debug, Clone)]
pub struct TrafficEvent {
    pub host: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub client_ip: String,
    pub user_agent: String,
    pub referrer: String,
}

struct Sender {
    tx: mpsc::Sender<TrafficEvent>,
    dropped: Arc<AtomicU64>,
}

static SENDER: OnceLock<Sender> = OnceLock::new();
/// True when at least one server has analytics on (checked before building an event).
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Should the proxy build an event at all? Cheap enough to call per request.
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed) && SENDER.get().is_some()
}

/// Queue an event without blocking. Dropped when the queue is full.
pub fn record(event: TrafficEvent) {
    if let Some(s) = SENDER.get() {
        if s.tx.try_send(event).is_err() {
            s.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit tested)
// ---------------------------------------------------------------------------

/// Histogram bin for a latency.
pub fn latency_bin(ms: u64) -> usize {
    LATENCY_BOUNDS_MS
        .iter()
        .position(|b| ms <= *b)
        .unwrap_or(LATENCY_BINS - 1)
}

/// Estimate a percentile (0..=100) from histogram bins, interpolating inside
/// the bin. Returns `None` for an empty histogram.
pub fn percentile(bins: &[u64; LATENCY_BINS], p: f64) -> Option<f64> {
    let total: u64 = bins.iter().sum();
    if total == 0 {
        return None;
    }
    let target = (p / 100.0 * total as f64).max(1.0);
    let mut seen = 0f64;
    for (i, &count) in bins.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let next = seen + count as f64;
        if next >= target {
            let lower = if i == 0 {
                0.0
            } else {
                LATENCY_BOUNDS_MS[i - 1] as f64
            };
            let upper = LATENCY_BOUNDS_MS
                .get(i)
                .map(|b| *b as f64)
                // Overflow bin: report its lower bound.
                .unwrap_or(lower);
            let frac = (target - seen) / count as f64;
            return Some(lower + (upper - lower) * frac);
        }
        seen = next;
    }
    Some(*LATENCY_BOUNDS_MS.last().unwrap() as f64)
}

/// Lowercased host without port.
pub fn normalize_host(raw: &str) -> String {
    let h = raw.trim();
    let h = if let Some(rest) = h.strip_prefix('[') {
        // [v6]:port
        rest.split(']').next().unwrap_or(rest)
    } else {
        h.rsplit_once(':')
            .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
            .map(|(host, _)| host)
            .unwrap_or(h)
    };
    h.to_ascii_lowercase()
}

/// Path without query string, capped in length.
pub fn normalize_path(raw: &str) -> String {
    let p = raw.split(['?', '#']).next().unwrap_or("/");
    let p = if p.is_empty() { "/" } else { p };
    if p.len() > MAX_PATH_LEN {
        let mut end = MAX_PATH_LEN;
        while !p.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &p[..end])
    } else {
        p.to_string()
    }
}

/// Referring site as a bare host, or `None` for direct traffic and same-site
/// navigation.
pub fn referrer_host(referrer: &str, own_host: &str) -> Option<String> {
    let r = referrer.trim();
    let rest = r
        .strip_prefix("https://")
        .or_else(|| r.strip_prefix("http://"))?;
    let host = normalize_host(rest.split(['/', '?', '#']).next().unwrap_or(""));
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let own = own_host.strip_prefix("www.").unwrap_or(own_host);
    if host.is_empty() || host == own {
        None
    } else {
        Some(host)
    }
}

/// Coarse user agent family: browser, bot or tool.
pub fn ua_family(ua: &str) -> &'static str {
    let l = ua.to_ascii_lowercase();
    if l.is_empty() {
        return "Unknown";
    }
    const BOTS: [&str; 6] = [
        "bot",
        "crawler",
        "spider",
        "slurp",
        "facebookexternalhit",
        "monitor",
    ];
    if BOTS.iter().any(|b| l.contains(b)) {
        return "Bot";
    }
    if l.starts_with("curl/") {
        return "curl";
    }
    if l.starts_with("wget/") {
        return "Wget";
    }
    if l.contains("python-requests") || l.contains("python-urllib") || l.contains("aiohttp") {
        return "Python";
    }
    if l.starts_with("go-http-client") {
        return "Go";
    }
    if l.contains("node-fetch") || l.starts_with("axios") || l.contains("undici") {
        return "Node.js";
    }
    if l.contains("postman") {
        return "Postman";
    }
    if l.contains("edg/") || l.contains("edge/") {
        return "Edge";
    }
    if l.contains("opr/") || l.contains("opera") {
        return "Opera";
    }
    if l.contains("samsungbrowser") {
        return "Samsung Internet";
    }
    if l.contains("firefox/") || l.contains("fxios") {
        return "Firefox";
    }
    if l.contains("chrome/") || l.contains("crios") || l.contains("chromium") {
        return "Chrome";
    }
    if l.contains("safari/") {
        return "Safari";
    }
    "Other"
}

/// Status class column for a status code.
fn status_class(status: u16) -> usize {
    match status {
        200..=299 => 0,
        300..=399 => 1,
        400..=499 => 2,
        _ => 3,
    }
}

/// Truncated SHA-256 visitor hash.
pub fn visitor_hash(salt: &str, ip: &str, ua: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update([0]);
    h.update(ip.as_bytes());
    h.update([0]);
    h.update(ua.as_bytes());
    hex::encode(&h.finalize()[..8])
}

/// UTC day string for a unix timestamp.
pub fn day_of(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .unwrap_or_default()
        .format("%Y-%m-%d")
        .to_string()
}

/// Is analytics on for `server_id` given its stored toggle (if any)?
pub fn server_enabled(server_id: &str, stored: Option<bool>) -> bool {
    stored.unwrap_or(server_id == LOCALHOST)
}

// ---------------------------------------------------------------------------
// In-memory rollup
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MinuteAgg {
    pub requests: u64,
    pub status: [u64; 4],
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub latency: [u64; LATENCY_BINS],
}

/// Where a host's traffic belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostTarget {
    pub app_id: Option<String>,
    pub server_id: String,
}

#[derive(Debug, Default)]
pub struct Rollup {
    pub minutes: HashMap<(i64, String), MinuteAgg>,
    pub dims: HashMap<(i64, String, &'static str, String), u64>,
    dim_values: HashMap<(String, &'static str), usize>,
    pub visitors: HashSet<(String, String, String)>,
    pub targets: HashMap<String, HostTarget>,
}

impl Rollup {
    fn add_dim(&mut self, hour: i64, host: &str, dim: &'static str, value: String) {
        let key = (hour, host.to_string(), dim, value);
        if let Some(c) = self.dims.get_mut(&key) {
            *c += 1;
            return;
        }
        let n = self.dim_values.entry((host.to_string(), dim)).or_insert(0);
        let key = if *n >= MAX_DIM_VALUES {
            (key.0, key.1, key.2, "(other)".to_string())
        } else {
            *n += 1;
            key
        };
        *self.dims.entry(key).or_insert(0) += 1;
    }

    /// Fold one event in. `country` and `visitor` are precomputed by the caller.
    pub fn add(
        &mut self,
        ts: i64,
        ev: &TrafficEvent,
        target: &HostTarget,
        country: Option<String>,
        visitor: Option<String>,
    ) {
        let host = normalize_host(&ev.host);
        self.targets.insert(host.clone(), target.clone());

        let m = self.minutes.entry((ts / 60, host.clone())).or_default();
        m.requests += 1;
        m.status[status_class(ev.status)] += 1;
        m.bytes_in += ev.bytes_in;
        m.bytes_out += ev.bytes_out;
        m.latency[latency_bin(ev.latency_ms)] += 1;

        let hour = ts / 3600;
        self.add_dim(hour, &host, "path", normalize_path(&ev.path));
        if let Some(r) = referrer_host(&ev.referrer, &host) {
            self.add_dim(hour, &host, "referrer", r);
        }
        self.add_dim(hour, &host, "ua", ua_family(&ev.user_agent).to_string());
        if let Some(c) = country {
            self.add_dim(hour, &host, "country", c);
        }
        if let Some(v) = visitor {
            if self.visitors.len() < MAX_VISITORS {
                self.visitors.insert((day_of(ts), host, v));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.minutes.is_empty()
    }
}

/// Write a rollup with upserts in one transaction.
pub async fn flush(db: &SqlitePool, rollup: Rollup) -> Result<(), sqlx::Error> {
    if rollup.is_empty() {
        return Ok(());
    }
    let target_of = |host: &str| {
        rollup.targets.get(host).cloned().unwrap_or(HostTarget {
            app_id: None,
            server_id: LOCALHOST.to_string(),
        })
    };
    let mut tx = db.begin().await?;
    for ((minute, host), m) in &rollup.minutes {
        let t = target_of(host);
        let mut q = sqlx::query(
            "INSERT INTO traffic_minute (minute, host, app_id, server_id, requests, \
             status_2xx, status_3xx, status_4xx, status_5xx, bytes_in, bytes_out, \
             l0, l1, l2, l3, l4, l5, l6, l7, l8, l9, l10, l11) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (minute, host) DO UPDATE SET \
             app_id = excluded.app_id, server_id = excluded.server_id, \
             requests = requests + excluded.requests, \
             status_2xx = status_2xx + excluded.status_2xx, \
             status_3xx = status_3xx + excluded.status_3xx, \
             status_4xx = status_4xx + excluded.status_4xx, \
             status_5xx = status_5xx + excluded.status_5xx, \
             bytes_in = bytes_in + excluded.bytes_in, \
             bytes_out = bytes_out + excluded.bytes_out, \
             l0 = l0 + excluded.l0, l1 = l1 + excluded.l1, l2 = l2 + excluded.l2, \
             l3 = l3 + excluded.l3, l4 = l4 + excluded.l4, l5 = l5 + excluded.l5, \
             l6 = l6 + excluded.l6, l7 = l7 + excluded.l7, l8 = l8 + excluded.l8, \
             l9 = l9 + excluded.l9, l10 = l10 + excluded.l10, l11 = l11 + excluded.l11",
        )
        .bind(minute)
        .bind(host)
        .bind(&t.app_id)
        .bind(&t.server_id)
        .bind(m.requests as i64);
        for s in m.status {
            q = q.bind(s as i64);
        }
        q = q.bind(m.bytes_in as i64).bind(m.bytes_out as i64);
        for l in m.latency {
            q = q.bind(l as i64);
        }
        q.execute(&mut *tx).await?;
    }
    for ((hour, host, dim, value), count) in &rollup.dims {
        let t = target_of(host);
        sqlx::query(
            "INSERT INTO traffic_dimensions (hour, host, app_id, server_id, dim, value, count) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (hour, host, dim, value) DO UPDATE SET count = count + excluded.count",
        )
        .bind(hour)
        .bind(host)
        .bind(&t.app_id)
        .bind(&t.server_id)
        .bind(*dim)
        .bind(value)
        .bind(*count as i64)
        .execute(&mut *tx)
        .await?;
    }
    for (day, host, visitor) in &rollup.visitors {
        let t = target_of(host);
        sqlx::query(
            "INSERT OR IGNORE INTO traffic_visitors (day, host, app_id, server_id, visitor) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(day)
        .bind(host)
        .bind(&t.app_id)
        .bind(&t.server_id)
        .bind(visitor)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// Delete rollups older than `days` before `now` and salts older than yesterday.
pub async fn prune(db: &SqlitePool, days: i64, now: i64) -> Result<(), sqlx::Error> {
    let cutoff = now - days.max(1) * 86_400;
    sqlx::query("DELETE FROM traffic_minute WHERE minute < ?")
        .bind(cutoff / 60)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM traffic_dimensions WHERE hour < ?")
        .bind(cutoff / 3600)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM traffic_visitors WHERE day < ?")
        .bind(day_of(cutoff))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM traffic_salts WHERE day < ?")
        .bind(day_of(now - 86_400))
        .execute(db)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings and lookups
// ---------------------------------------------------------------------------

/// Stored per-server toggles.
pub async fn server_toggles(db: &SqlitePool) -> Result<HashMap<String, bool>, sqlx::Error> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT server_id, enabled FROM traffic_analytics_servers")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|(s, e)| (s, e != 0)).collect())
}

/// Save a server toggle and refresh the hot-path flag.
pub async fn set_server_enabled(
    db: &SqlitePool,
    server_id: &str,
    enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO traffic_analytics_servers (server_id, enabled, updated_at) VALUES (?, ?, datetime('now')) \
         ON CONFLICT (server_id) DO UPDATE SET enabled = excluded.enabled, updated_at = excluded.updated_at",
    )
    .bind(server_id)
    .bind(enabled as i64)
    .execute(db)
    .await?;
    refresh_active(db).await;
    Ok(())
}

/// Is analytics on for this server?
pub async fn is_server_enabled(db: &SqlitePool, server_id: &str) -> bool {
    let stored: Option<i64> =
        sqlx::query_scalar("SELECT enabled FROM traffic_analytics_servers WHERE server_id = ?")
            .bind(server_id)
            .fetch_optional(db)
            .await
            .unwrap_or(None);
    server_enabled(server_id, stored.map(|e| e != 0))
}

/// Recompute the hot-path flag: on when localhost or any server is enabled.
pub async fn refresh_active(db: &SqlitePool) {
    let toggles = server_toggles(db).await.unwrap_or_default();
    let any = server_enabled(LOCALHOST, toggles.get(LOCALHOST).copied())
        || toggles.iter().any(|(id, on)| id != LOCALHOST && *on);
    ACTIVE.store(any, Ordering::Relaxed);
}

/// Retention in days and the GeoIP database path from instance settings.
pub async fn load_settings(db: &SqlitePool) -> (i64, Option<String>) {
    let map = crate::db::load_setting_map(db).await.unwrap_or_default();
    let days = map
        .get(SETTING_RETENTION)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|d| (1..=365).contains(d))
        .unwrap_or(DEFAULT_RETENTION_DAYS);
    let geo = map
        .get(SETTING_GEOIP)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    (days, geo)
}

/// Map every app host to its app and server.
async fn load_targets(db: &SqlitePool) -> HashMap<String, HostTarget> {
    let apps = sqlx::query_as::<_, crate::db::App>("SELECT * FROM apps")
        .fetch_all(db)
        .await
        .unwrap_or_default();
    let mut out = HashMap::new();
    for app in apps {
        let server_id = app
            .server_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| LOCALHOST.to_string());
        for d in app.get_all_domain_names() {
            out.insert(
                normalize_host(&d),
                HostTarget {
                    app_id: Some(app.id.clone()),
                    server_id: server_id.clone(),
                },
            );
        }
    }
    out
}

/// Loaded GeoIP reader, kept with the path it came from.
pub struct GeoIp {
    path: PathBuf,
    reader: maxminddb::Reader<Vec<u8>>,
}

impl GeoIp {
    pub fn open(path: &str) -> Result<Self, String> {
        let reader = maxminddb::Reader::open_readfile(path)
            .map_err(|e| format!("Could not open GeoIP database: {e}"))?;
        Ok(Self {
            path: PathBuf::from(path),
            reader,
        })
    }

    /// ISO country code for an IP, if the database has one.
    pub fn country(&self, ip: IpAddr) -> Option<String> {
        use maxminddb::PathElement;
        self.reader
            .lookup(ip)
            .ok()?
            .decode_path::<String>(&[PathElement::Key("country"), PathElement::Key("iso_code")])
            .ok()
            .flatten()
    }
}

static GEO_STATUS: OnceLock<ArcSwap<Option<String>>> = OnceLock::new();

/// Last GeoIP load error, if any (shown on the settings page).
pub fn geoip_error() -> Option<String> {
    GEO_STATUS
        .get()
        .and_then(|s| s.load().as_ref().as_ref().cloned())
}

fn set_geoip_error(e: Option<String>) {
    GEO_STATUS
        .get_or_init(|| ArcSwap::from_pointee(None))
        .store(Arc::new(e));
}

// ---------------------------------------------------------------------------
// Aggregator task
// ---------------------------------------------------------------------------

/// Start the aggregator once (later calls are ignored).
pub fn spawn(db: SqlitePool) {
    let (tx, rx) = mpsc::channel(QUEUE_CAPACITY);
    let dropped = Arc::new(AtomicU64::new(0));
    if SENDER
        .set(Sender {
            tx,
            dropped: dropped.clone(),
        })
        .is_err()
    {
        return;
    }
    tokio::spawn(run(db, rx, dropped));
}

struct Aggregator {
    db: SqlitePool,
    rollup: Rollup,
    targets: HashMap<String, HostTarget>,
    toggles: HashMap<String, bool>,
    geo: Option<GeoIp>,
    geo_cache: HashMap<IpAddr, Option<String>>,
    salt: Option<(String, String)>,
}

impl Aggregator {
    async fn refresh(&mut self) {
        self.targets = load_targets(&self.db).await;
        self.toggles = server_toggles(&self.db).await.unwrap_or_default();
        refresh_active(&self.db).await;
        let (_, path) = load_settings(&self.db).await;
        match path {
            None => {
                self.geo = None;
                set_geoip_error(None);
            }
            Some(p)
                if self
                    .geo
                    .as_ref()
                    .is_some_and(|g| g.path.as_path() == std::path::Path::new(&p)) => {}
            Some(p) => match GeoIp::open(&p) {
                Ok(g) => {
                    self.geo = Some(g);
                    set_geoip_error(None);
                }
                Err(e) => {
                    warn!(path = %p, error = %e, "Traffic analytics GeoIP database not loaded");
                    self.geo = None;
                    set_geoip_error(Some(e));
                }
            },
        }
        self.geo_cache.clear();
    }

    async fn salt_for(&mut self, day: &str) -> Option<String> {
        if let Some((d, s)) = &self.salt {
            if d == day {
                return Some(s.clone());
            }
        }
        let existing: Option<String> =
            sqlx::query_scalar("SELECT salt FROM traffic_salts WHERE day = ?")
                .bind(day)
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();
        let salt = match existing {
            Some(s) => s,
            None => {
                let bytes: [u8; 16] = {
                    use rand::RngExt;
                    rand::rng().random()
                };
                let s = hex::encode(bytes);
                sqlx::query("INSERT OR IGNORE INTO traffic_salts (day, salt) VALUES (?, ?)")
                    .bind(day)
                    .bind(&s)
                    .execute(&self.db)
                    .await
                    .ok()?;
                // Another writer may have won the race; read back the stored one.
                sqlx::query_scalar("SELECT salt FROM traffic_salts WHERE day = ?")
                    .bind(day)
                    .fetch_optional(&self.db)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(s)
            }
        };
        self.salt = Some((day.to_string(), salt.clone()));
        Some(salt)
    }

    async fn ingest(&mut self, ev: TrafficEvent) {
        let host = normalize_host(&ev.host);
        let target = self.targets.get(&host).cloned().unwrap_or(HostTarget {
            app_id: None,
            server_id: LOCALHOST.to_string(),
        });
        if !server_enabled(
            &target.server_id,
            self.toggles.get(&target.server_id).copied(),
        ) {
            return;
        }
        let ts = chrono::Utc::now().timestamp();
        let ip: Option<IpAddr> = ev.client_ip.parse().ok();
        let country = match (&self.geo, ip) {
            (Some(geo), Some(ip)) => self
                .geo_cache
                .entry(ip)
                .or_insert_with(|| geo.country(ip))
                .clone(),
            _ => None,
        };
        let visitor = if ev.client_ip.is_empty() {
            None
        } else {
            let day = day_of(ts);
            self.salt_for(&day)
                .await
                .map(|salt| visitor_hash(&salt, &ev.client_ip, &ev.user_agent))
        };
        self.rollup.add(ts, &ev, &target, country, visitor);
    }

    async fn flush(&mut self) {
        let rollup = std::mem::take(&mut self.rollup);
        if let Err(e) = flush(&self.db, rollup).await {
            warn!(error = %e, "Failed to write traffic analytics rollup");
        }
        if self.geo_cache.len() > 50_000 {
            self.geo_cache.clear();
        }
    }
}

async fn run(db: SqlitePool, mut rx: mpsc::Receiver<TrafficEvent>, dropped: Arc<AtomicU64>) {
    let mut agg = Aggregator {
        db: db.clone(),
        rollup: Rollup::default(),
        targets: HashMap::new(),
        toggles: HashMap::new(),
        geo: None,
        geo_cache: HashMap::new(),
        salt: None,
    };
    agg.refresh().await;

    let mut tick = tokio::time::interval(FLUSH_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    tick.tick().await;
    let mut last_prune = 0i64;

    loop {
        tokio::select! {
            maybe = rx.recv() => match maybe {
                Some(ev) => agg.ingest(ev).await,
                None => {
                    agg.flush().await;
                    break;
                }
            },
            _ = tick.tick() => {
                agg.flush().await;
                let n = dropped.swap(0, Ordering::Relaxed);
                if n > 0 {
                    warn!(dropped = n, "Traffic analytics queue full; events dropped");
                }
                agg.refresh().await;
                let now = chrono::Utc::now().timestamp();
                if now - last_prune >= 3600 {
                    let (days, _) = load_settings(&db).await;
                    if let Err(e) = prune(&db, days, now).await {
                        debug!(error = %e, "Traffic analytics prune failed");
                    }
                    last_prune = now;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(host: &str, path: &str, status: u16, ms: u64) -> TrafficEvent {
        TrafficEvent {
            host: host.into(),
            path: path.into(),
            status,
            latency_ms: ms,
            bytes_in: 10,
            bytes_out: 100,
            client_ip: "203.0.113.9".into(),
            user_agent: "Mozilla/5.0 (Macintosh) AppleWebKit Chrome/120.0 Safari/537.36".into(),
            referrer: "https://news.ycombinator.com/item?id=1".into(),
        }
    }

    #[test]
    fn latency_bins_and_percentiles() {
        assert_eq!(latency_bin(0), 0);
        assert_eq!(latency_bin(10), 0);
        assert_eq!(latency_bin(11), 1);
        assert_eq!(latency_bin(30_000), 10);
        assert_eq!(latency_bin(99_999), LATENCY_BINS - 1);

        let mut bins = [0u64; LATENCY_BINS];
        assert_eq!(percentile(&bins, 50.0), None);
        bins[latency_bin(40)] = 90; // 25..=50 ms
        bins[latency_bin(800)] = 10; // 500..=1000 ms
        let p50 = percentile(&bins, 50.0).unwrap();
        assert!((25.0..=50.0).contains(&p50), "p50 = {p50}");
        let p95 = percentile(&bins, 95.0).unwrap();
        assert!((500.0..=1000.0).contains(&p95), "p95 = {p95}");
    }

    #[test]
    fn hosts_paths_and_referrers_are_normalized() {
        assert_eq!(normalize_host("Example.COM:8443"), "example.com");
        assert_eq!(normalize_host("[::1]:80"), "::1");
        assert_eq!(normalize_path("/a/b?x=1#y"), "/a/b");
        assert_eq!(normalize_path(""), "/");
        assert!(normalize_path(&"/x".repeat(300)).ends_with("..."));
        assert_eq!(
            referrer_host("https://www.google.com/search?q=1", "app.io"),
            Some("google.com".into())
        );
        assert_eq!(referrer_host("https://app.io/page", "app.io"), None);
        assert_eq!(referrer_host("", "app.io"), None);
        assert_eq!(referrer_host("android-app://x", "app.io"), None);
    }

    #[test]
    fn user_agent_families() {
        assert_eq!(
            ua_family("Mozilla/5.0 ... Chrome/120.0 Safari/537.36"),
            "Chrome"
        );
        assert_eq!(
            ua_family("Mozilla/5.0 ... Chrome/120.0 Safari/537.36 Edg/120.0"),
            "Edge"
        );
        assert_eq!(
            ua_family("Mozilla/5.0 ... Version/17.0 Safari/605.1.15"),
            "Safari"
        );
        assert_eq!(ua_family("Mozilla/5.0 ... Firefox/121.0"), "Firefox");
        assert_eq!(
            ua_family("Googlebot/2.1 (+http://www.google.com/bot.html)"),
            "Bot"
        );
        assert_eq!(ua_family("curl/8.4.0"), "curl");
        assert_eq!(ua_family(""), "Unknown");
    }

    #[test]
    fn server_toggle_defaults() {
        assert!(server_enabled(LOCALHOST, None));
        assert!(!server_enabled("srv-1", None));
        assert!(server_enabled("srv-1", Some(true)));
        assert!(!server_enabled(LOCALHOST, Some(false)));
    }

    #[test]
    fn visitor_hash_depends_on_salt() {
        let a = visitor_hash("s1", "1.2.3.4", "ua");
        assert_eq!(a.len(), 16);
        assert_eq!(a, visitor_hash("s1", "1.2.3.4", "ua"));
        assert_ne!(a, visitor_hash("s2", "1.2.3.4", "ua"));
        assert_ne!(a, visitor_hash("s1", "1.2.3.5", "ua"));
    }

    #[test]
    fn rollup_caps_dimension_values() {
        let mut r = Rollup::default();
        let t = HostTarget {
            app_id: None,
            server_id: LOCALHOST.into(),
        };
        for i in 0..(MAX_DIM_VALUES + 50) {
            r.add(0, &ev("a.io", &format!("/p{i}"), 200, 5), &t, None, None);
        }
        let paths = r.dims.keys().filter(|k| k.2 == "path").count();
        assert_eq!(paths, MAX_DIM_VALUES + 1);
        assert_eq!(
            r.dims
                .get(&(0, "a.io".to_string(), "path", "(other)".to_string()))
                .copied(),
            Some(50)
        );
    }

    #[tokio::test]
    async fn flush_upserts_and_prune_removes_old_rows() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        let t = HostTarget {
            app_id: Some("app-1".into()),
            server_id: LOCALHOST.into(),
        };
        let now = 1_700_000_000i64;
        for _ in 0..2 {
            let mut r = Rollup::default();
            r.add(
                now,
                &ev("A.io:443", "/x", 200, 5),
                &t,
                Some("GH".into()),
                Some("v1".into()),
            );
            r.add(
                now,
                &ev("a.io", "/x", 503, 800),
                &t,
                None,
                Some("v1".into()),
            );
            flush(&db, r).await.unwrap();
        }
        let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT requests, status_2xx, status_5xx, l0, l6 FROM traffic_minute WHERE host = 'a.io'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(row, (4, 2, 2, 2, 2));
        let paths: i64 = sqlx::query_scalar(
            "SELECT count FROM traffic_dimensions WHERE dim = 'path' AND value = '/x'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(paths, 4);
        let visitors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traffic_visitors")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(visitors, 1);
        let app: Option<String> =
            sqlx::query_scalar("SELECT app_id FROM traffic_minute WHERE host = 'a.io'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(app.as_deref(), Some("app-1"));

        prune(&db, 30, now + 31 * 86_400).await.unwrap();
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traffic_minute")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(left, 0);
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traffic_visitors")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }

    #[tokio::test]
    async fn toggles_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        assert!(is_server_enabled(&db, LOCALHOST).await);
        assert!(!is_server_enabled(&db, "srv").await);
        set_server_enabled(&db, "srv", true).await.unwrap();
        set_server_enabled(&db, LOCALHOST, false).await.unwrap();
        assert!(is_server_enabled(&db, "srv").await);
        assert!(!is_server_enabled(&db, LOCALHOST).await);
        let (days, geo) = load_settings(&db).await;
        assert_eq!(days, 30);
        assert!(geo.is_none());
    }
}
