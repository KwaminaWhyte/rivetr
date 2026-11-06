//! Traffic analytics API (E3). Reads the rollups written by
//! `proxy::analytics`.
//!
//! - `GET  /api/apps/{id}/analytics?range=`      one app
//! - `GET  /api/servers/{id}/analytics?range=`   one server (`localhost` = admin)
//! - `PUT  /api/servers/{id}/analytics`          enable / disable for a server
//! - `GET  /api/analytics?range=&team_id=`       everything the caller can see
//! - `GET/PUT /api/analytics/settings`           retention + GeoIP path (admin)
//!
//! `range` is `1h`, `24h` (default), `7d` or `30d`.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::db::User;
use crate::proxy::analytics::{self as an, is_server_enabled, percentile, LATENCY_BINS, LOCALHOST};
use crate::AppState;

use super::authz;
use super::error::ApiError;

// ---------------------------------------------------------------------------
// Range and scope
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    Hour,
    Day,
    Week,
    Month,
}

impl Range {
    pub fn parse(raw: Option<&str>) -> Result<Self, ApiError> {
        match raw.unwrap_or("24h") {
            "1h" => Ok(Self::Hour),
            "24h" | "1d" => Ok(Self::Day),
            "7d" => Ok(Self::Week),
            "30d" => Ok(Self::Month),
            other => Err(ApiError::validation_field(
                "range",
                format!("Unknown range '{other}'. Use 1h, 24h, 7d or 30d"),
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Hour => "1h",
            Self::Day => "24h",
            Self::Week => "7d",
            Self::Month => "30d",
        }
    }

    /// (length, chart step) in seconds.
    pub fn spans(self) -> (i64, i64) {
        match self {
            Self::Hour => (3_600, 60),
            Self::Day => (86_400, 900),
            Self::Week => (7 * 86_400, 3_600),
            Self::Month => (30 * 86_400, 6 * 3_600),
        }
    }
}

/// Which rollup rows a report covers.
#[derive(Debug, Clone)]
pub enum Scope {
    App(String),
    Server(String),
    Apps(Vec<String>),
    All,
}

impl Scope {
    /// Append ` AND <filter>` for the tables that carry app_id / server_id.
    fn push_filter(&self, qb: &mut QueryBuilder<Sqlite>) {
        match self {
            Scope::App(id) => {
                qb.push(" AND app_id = ").push_bind(id.clone());
            }
            Scope::Server(id) => {
                qb.push(" AND server_id = ").push_bind(id.clone());
            }
            Scope::Apps(ids) => {
                qb.push(" AND app_id IN (");
                let mut sep = qb.separated(", ");
                for id in ids {
                    sep.push_bind(id.clone());
                }
                qb.push(")");
            }
            Scope::All => {}
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, Scope::Apps(ids) if ids.is_empty())
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct Totals {
    pub requests: i64,
    pub status_2xx: i64,
    pub status_3xx: i64,
    pub status_4xx: i64,
    pub status_5xx: i64,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    /// Sum of daily unique visitors over the days the range touches.
    pub visitors: i64,
}

#[derive(Debug, Serialize)]
pub struct Point {
    /// Bucket start, unix seconds.
    pub ts: i64,
    pub requests: i64,
    pub status_2xx: i64,
    pub status_3xx: i64,
    pub status_4xx: i64,
    pub status_5xx: i64,
    pub bytes_out: i64,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct Ranked {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct HostRow {
    pub host: String,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub requests: i64,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub range: &'static str,
    pub step_secs: i64,
    /// Analytics is on for the server(s) behind this report.
    pub enabled: bool,
    /// A GeoIP database is configured and loaded (country breakdown available).
    pub geoip: bool,
    pub totals: Totals,
    pub series: Vec<Point>,
    pub top_paths: Vec<Ranked>,
    pub top_referrers: Vec<Ranked>,
    pub top_user_agents: Vec<Ranked>,
    /// `None` when no GeoIP database is configured.
    pub countries: Option<Vec<Ranked>>,
    pub top_hosts: Vec<HostRow>,
}

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    pub range: Option<String>,
    pub team_id: Option<String>,
}

fn sum_cols() -> String {
    let mut s = String::from(
        "SUM(requests), SUM(status_2xx), SUM(status_3xx), SUM(status_4xx), SUM(status_5xx), \
         SUM(bytes_in), SUM(bytes_out)",
    );
    for i in 0..LATENCY_BINS {
        s.push_str(&format!(", SUM(l{i})"));
    }
    s
}

fn hist_from(row: &sqlx::sqlite::SqliteRow, offset: usize) -> [u64; LATENCY_BINS] {
    let mut bins = [0u64; LATENCY_BINS];
    for (i, b) in bins.iter_mut().enumerate() {
        *b = row
            .try_get::<Option<i64>, _>(offset + i)
            .ok()
            .flatten()
            .unwrap_or(0)
            .max(0) as u64;
    }
    bins
}

fn col(row: &sqlx::sqlite::SqliteRow, i: usize) -> i64 {
    row.try_get::<Option<i64>, _>(i).ok().flatten().unwrap_or(0)
}

/// Round a percentile for display.
fn round1(v: Option<f64>) -> Option<f64> {
    v.map(|x| (x * 10.0).round() / 10.0)
}

async fn top(
    db: &SqlitePool,
    scope: &Scope,
    dim: &str,
    since_hour: i64,
    limit: i64,
) -> Result<Vec<Ranked>, sqlx::Error> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT value, SUM(count) AS c FROM traffic_dimensions WHERE hour >= ",
    );
    qb.push_bind(since_hour)
        .push(" AND dim = ")
        .push_bind(dim.to_string());
    scope.push_filter(&mut qb);
    qb.push(" GROUP BY value ORDER BY c DESC LIMIT ")
        .push_bind(limit);
    let rows = qb.build().fetch_all(db).await?;
    Ok(rows
        .iter()
        .map(|r| Ranked {
            value: r.try_get::<String, _>(0).unwrap_or_default(),
            count: col(r, 1),
        })
        .collect())
}

/// Build a report for `scope` over `range`, ending at `now` (unix seconds).
pub async fn build_report(
    db: &SqlitePool,
    scope: &Scope,
    range: Range,
    now: i64,
    enabled: bool,
) -> Result<Report, sqlx::Error> {
    let (len, step) = range.spans();
    let start = (now - len) / step * step + step;
    let geoip = an::load_settings(db).await.1.is_some() && an::geoip_error().is_none();

    let mut report = Report {
        range: range.label(),
        step_secs: step,
        enabled,
        geoip,
        totals: Totals::default(),
        series: Vec::new(),
        top_paths: Vec::new(),
        top_referrers: Vec::new(),
        top_user_agents: Vec::new(),
        countries: geoip.then(Vec::new),
        top_hosts: Vec::new(),
    };

    // Series (buckets of `step` seconds), gaps filled with zeros.
    let mut by_bucket: HashMap<i64, Point> = HashMap::new();
    let mut all_bins = [0u64; LATENCY_BINS];
    if !scope.is_empty() {
        let mut qb = QueryBuilder::<Sqlite>::new("SELECT (minute * 60 / ");
        qb.push_bind(step)
            .push(") * ")
            .push_bind(step)
            .push(" AS ts, ")
            .push(sum_cols())
            .push(" FROM traffic_minute WHERE minute >= ")
            .push_bind(start / 60);
        scope.push_filter(&mut qb);
        qb.push(" GROUP BY ts");
        for row in qb.build().fetch_all(db).await? {
            let ts = col(&row, 0);
            let bins = hist_from(&row, 8);
            for (a, b) in all_bins.iter_mut().zip(bins.iter()) {
                *a += b;
            }
            let t = &mut report.totals;
            t.requests += col(&row, 1);
            t.status_2xx += col(&row, 2);
            t.status_3xx += col(&row, 3);
            t.status_4xx += col(&row, 4);
            t.status_5xx += col(&row, 5);
            t.bytes_in += col(&row, 6);
            t.bytes_out += col(&row, 7);
            by_bucket.insert(
                ts,
                Point {
                    ts,
                    requests: col(&row, 1),
                    status_2xx: col(&row, 2),
                    status_3xx: col(&row, 3),
                    status_4xx: col(&row, 4),
                    status_5xx: col(&row, 5),
                    bytes_out: col(&row, 7),
                    p50_ms: round1(percentile(&bins, 50.0)),
                    p95_ms: round1(percentile(&bins, 95.0)),
                },
            );
        }
    }
    let mut ts = start;
    while ts <= now {
        report.series.push(by_bucket.remove(&ts).unwrap_or(Point {
            ts,
            requests: 0,
            status_2xx: 0,
            status_3xx: 0,
            status_4xx: 0,
            status_5xx: 0,
            bytes_out: 0,
            p50_ms: None,
            p95_ms: None,
        }));
        ts += step;
    }
    report.totals.p50_ms = round1(percentile(&all_bins, 50.0));
    report.totals.p95_ms = round1(percentile(&all_bins, 95.0));

    if scope.is_empty() {
        return Ok(report);
    }

    let since_hour = start / 3600;
    report.top_paths = top(db, scope, "path", since_hour, 10).await?;
    report.top_referrers = top(db, scope, "referrer", since_hour, 10).await?;
    report.top_user_agents = top(db, scope, "ua", since_hour, 10).await?;
    if geoip {
        report.countries = Some(top(db, scope, "country", since_hour, 15).await?);
    }

    let mut qb = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM traffic_visitors WHERE day >= ");
    qb.push_bind(an::day_of(start));
    scope.push_filter(&mut qb);
    report.totals.visitors = qb.build().fetch_one(db).await.map(|r| col(&r, 0))?;

    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT t.host, t.app_id, a.name, SUM(t.requests) AS r FROM traffic_minute t \
         LEFT JOIN apps a ON a.id = t.app_id WHERE t.minute >= ",
    );
    qb.push_bind(start / 60);
    // Same filter, qualified for the join.
    match scope {
        Scope::App(id) => {
            qb.push(" AND t.app_id = ").push_bind(id.clone());
        }
        Scope::Server(id) => {
            qb.push(" AND t.server_id = ").push_bind(id.clone());
        }
        Scope::Apps(ids) => {
            qb.push(" AND t.app_id IN (");
            let mut sep = qb.separated(", ");
            for id in ids {
                sep.push_bind(id.clone());
            }
            qb.push(")");
        }
        Scope::All => {}
    }
    qb.push(" GROUP BY t.host ORDER BY r DESC LIMIT 10");
    report.top_hosts = qb
        .build()
        .fetch_all(db)
        .await?
        .iter()
        .map(|r| HostRow {
            host: r.try_get(0).unwrap_or_default(),
            app_id: r.try_get(1).ok().flatten(),
            app_name: r.try_get(2).ok().flatten(),
            requests: col(r, 3),
        })
        .collect();

    Ok(report)
}

fn db_err(e: sqlx::Error) -> ApiError {
    ApiError::database(e.to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/apps/{id}/analytics
pub async fn app_analytics(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Report>, ApiError> {
    let range = Range::parse(q.range.as_deref())?;
    let app = authz::authorize_app(&state, &user, &id).await?;
    let server = app
        .server_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| LOCALHOST.to_string());
    let enabled = is_server_enabled(&state.db, &server).await;
    let report = build_report(&state.db, &Scope::App(id), range, now(), enabled)
        .await
        .map_err(db_err)?;
    Ok(Json(report))
}

async fn authorize_server_target(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
    write: bool,
) -> Result<(), ApiError> {
    if id == LOCALHOST {
        return authz::require_privileged(user);
    }
    let server = authz::authorize_server(state, user, id).await?;
    if write && !authz::can_write_team(state, user, server.team_id.as_deref()).await? {
        return Err(ApiError::forbidden(
            "Your team role is read-only; this action requires the developer role or higher",
        ));
    }
    Ok(())
}

/// GET /api/servers/{id}/analytics
pub async fn server_analytics(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Report>, ApiError> {
    let range = Range::parse(q.range.as_deref())?;
    authorize_server_target(&state, &user, &id, false).await?;
    let enabled = is_server_enabled(&state.db, &id).await;
    let report = build_report(&state.db, &Scope::Server(id), range, now(), enabled)
        .await
        .map_err(db_err)?;
    Ok(Json(report))
}

#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ToggleResponse {
    pub server_id: String,
    pub enabled: bool,
}

/// PUT /api/servers/{id}/analytics
pub async fn set_server_analytics(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(req): Json<ToggleRequest>,
) -> Result<Json<ToggleResponse>, ApiError> {
    authorize_server_target(&state, &user, &id, true).await?;
    an::set_server_enabled(&state.db, &id, req.enabled)
        .await
        .map_err(db_err)?;
    Ok(Json(ToggleResponse {
        server_id: id,
        enabled: req.enabled,
    }))
}

/// App ids whose traffic belongs in a team's report.
///
/// Unassigned apps (`team_id IS NULL`, created before teams existed) are shown
/// in every team's app list by `GET /api/apps`, so their traffic must count
/// here too; otherwise the app shows traffic on its own page and nothing on
/// the monitoring page.
async fn team_app_ids(db: &SqlitePool, team_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM apps WHERE team_id = ? OR team_id IS NULL")
        .bind(team_id)
        .fetch_all(db)
        .await
}

/// GET /api/analytics?range=&team_id=
pub async fn team_analytics(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(q): Query<RangeQuery>,
) -> Result<Json<Report>, ApiError> {
    let range = Range::parse(q.range.as_deref())?;
    let team = q.team_id.as_deref().filter(|t| !t.is_empty());
    let scope = match team {
        None if authz::is_privileged_user(&user) => Scope::All,
        Some(tid) => {
            authz::authorize_team_filter(&state, &user, Some(tid)).await?;
            Scope::Apps(team_app_ids(&state.db, tid).await?)
        }
        None => {
            let rows: Vec<(String, Option<String>)> =
                sqlx::query_as("SELECT id, team_id FROM apps")
                    .fetch_all(&state.db)
                    .await?;
            let visible = authz::retain_visible(&state, &user, rows, |(_, t)| t.as_deref()).await?;
            Scope::Apps(visible.into_iter().map(|(id, _)| id).collect())
        }
    };
    let toggles = an::server_toggles(&state.db).await.map_err(db_err)?;
    let enabled = an::server_enabled(LOCALHOST, toggles.get(LOCALHOST).copied())
        || toggles.values().any(|on| *on);
    let report = build_report(&state.db, &scope, range, now(), enabled)
        .await
        .map_err(db_err)?;
    Ok(Json(report))
}

#[derive(Debug, Serialize)]
pub struct ServerToggle {
    pub server_id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSettings {
    pub retention_days: i64,
    pub geoip_path: Option<String>,
    /// Set when the configured database could not be loaded.
    pub geoip_error: Option<String>,
    pub servers: Vec<ServerToggle>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAnalyticsSettings {
    pub retention_days: Option<i64>,
    /// Empty string clears it.
    pub geoip_path: Option<String>,
}

async fn settings_view(state: &AppState) -> Result<AnalyticsSettings, ApiError> {
    let (retention_days, geoip_path) = an::load_settings(&state.db).await;
    let toggles = an::server_toggles(&state.db).await.map_err(db_err)?;
    let mut servers = vec![ServerToggle {
        server_id: LOCALHOST.to_string(),
        name: "localhost".to_string(),
        enabled: an::server_enabled(LOCALHOST, toggles.get(LOCALHOST).copied()),
    }];
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, name FROM servers ORDER BY name")
        .fetch_all(&state.db)
        .await?;
    for (id, name) in rows {
        let enabled = an::server_enabled(&id, toggles.get(&id).copied());
        servers.push(ServerToggle {
            server_id: id,
            name,
            enabled,
        });
    }
    Ok(AnalyticsSettings {
        retention_days,
        geoip_error: geoip_path.as_ref().and_then(|_| an::geoip_error()),
        geoip_path,
        servers,
    })
}

/// GET /api/analytics/settings (admin)
pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<AnalyticsSettings>, ApiError> {
    authz::require_privileged(&user)?;
    Ok(Json(settings_view(&state).await?))
}

/// PUT /api/analytics/settings (admin)
pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<UpdateAnalyticsSettings>,
) -> Result<Json<AnalyticsSettings>, ApiError> {
    authz::require_privileged(&user)?;
    if let Some(days) = req.retention_days {
        if !(1..=365).contains(&days) {
            return Err(ApiError::validation_field(
                "retention_days",
                "Retention must be between 1 and 365 days",
            ));
        }
        crate::db::set_setting_value(&state.db, an::SETTING_RETENTION, Some(&days.to_string()))
            .await?;
    }
    if let Some(path) = req.geoip_path.as_deref().map(str::trim) {
        if path.is_empty() {
            crate::db::set_setting_value(&state.db, an::SETTING_GEOIP, None).await?;
        } else {
            let p = path.to_string();
            tokio::task::spawn_blocking(move || an::GeoIp::open(&p).map(|_| ()))
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
                .map_err(|e| ApiError::validation_field("geoip_path", e))?;
            crate::db::set_setting_value(&state.db, an::SETTING_GEOIP, Some(path)).await?;
        }
    }
    Ok(Json(settings_view(&state).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::analytics::{flush, HostTarget, Rollup, TrafficEvent};

    #[test]
    fn ranges_parse() {
        assert_eq!(Range::parse(None).unwrap(), Range::Day);
        assert_eq!(Range::parse(Some("7d")).unwrap(), Range::Week);
        assert!(Range::parse(Some("2y")).is_err());
        let (len, step) = Range::Month.spans();
        assert_eq!(len / step, 120);
    }

    #[tokio::test]
    async fn report_sums_scope_and_fills_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        let now = 1_700_000_000i64;
        let ev = |host: &str, status| TrafficEvent {
            host: host.into(),
            path: "/".into(),
            status,
            latency_ms: 20,
            bytes_in: 0,
            bytes_out: 50,
            client_ip: "198.51.100.1".into(),
            user_agent: "curl/8".into(),
            referrer: String::new(),
        };
        let mut r = Rollup::default();
        let a = HostTarget {
            app_id: Some("a1".into()),
            server_id: LOCALHOST.into(),
        };
        let b = HostTarget {
            app_id: Some("b1".into()),
            server_id: "srv".into(),
        };
        r.add(now - 120, &ev("a.io", 200), &a, None, Some("v".into()));
        r.add(now - 60, &ev("a.io", 404), &a, None, Some("w".into()));
        r.add(now - 60, &ev("b.io", 500), &b, None, None);
        flush(&db, r).await.unwrap();

        let rep = build_report(&db, &Scope::App("a1".into()), Range::Hour, now, true)
            .await
            .unwrap();
        assert_eq!(rep.totals.requests, 2);
        assert_eq!(rep.totals.status_4xx, 1);
        assert_eq!(rep.totals.bytes_out, 100);
        assert_eq!(rep.totals.visitors, 2);
        assert_eq!(rep.series.len(), 60);
        assert_eq!(rep.series.iter().map(|p| p.requests).sum::<i64>(), 2);
        assert!(rep.totals.p50_ms.is_some());
        assert!(rep.countries.is_none(), "no GeoIP configured");
        assert_eq!(rep.top_user_agents[0].value, "curl");

        let rep = build_report(&db, &Scope::Server("srv".into()), Range::Hour, now, true)
            .await
            .unwrap();
        assert_eq!(rep.totals.requests, 1);
        assert_eq!(rep.totals.status_5xx, 1);

        let rep = build_report(&db, &Scope::All, Range::Day, now, true)
            .await
            .unwrap();
        assert_eq!(rep.totals.requests, 3);
        assert_eq!(rep.top_hosts.len(), 2);

        let rep = build_report(&db, &Scope::Apps(vec![]), Range::Day, now, true)
            .await
            .unwrap();
        assert_eq!(rep.totals.requests, 0);
    }

    #[tokio::test]
    async fn team_scope_includes_unassigned_apps() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO teams (id, name, slug) VALUES \
             ('team-1', 'One', 'one'), ('team-2', 'Two', 'two')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, team_id) VALUES \
             ('a1', 'one', 'https://example.com/one.git', 'team-1'), \
             ('a2', 'two', 'https://example.com/two.git', 'team-2'), \
             ('a3', 'legacy', 'https://example.com/legacy.git', NULL)",
        )
        .execute(&db)
        .await
        .unwrap();

        let mut ids = team_app_ids(&db, "team-1").await.unwrap();
        ids.sort();
        assert_eq!(ids, vec!["a1".to_string(), "a3".to_string()]);
    }
}
