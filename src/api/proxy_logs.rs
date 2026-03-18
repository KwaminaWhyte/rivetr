//! Proxy access log query endpoint.
//!
//! Exposes the `proxy_logs` table for inspection via the dashboard.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use super::error::ApiError;
use crate::AppState;

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ProxyLog {
    pub id: i64,
    pub ts: String,
    pub host: String,
    pub method: String,
    pub path: String,
    pub status: i64,
    pub response_ms: i64,
    pub bytes_out: i64,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListProxyLogsParams {
    /// Filter by exact host / domain
    pub domain: Option<String>,
    /// Filter by HTTP status code class: "2xx", "3xx", "4xx", "5xx"
    pub status: Option<String>,
    /// Max rows to return (default 100, max 1000)
    pub limit: Option<i64>,
    /// Row offset for pagination (default 0)
    pub offset: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// List proxy access logs.
///
/// GET /api/proxy/logs?domain=&status=2xx&limit=100&offset=0
pub async fn list_proxy_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListProxyLogsParams>,
) -> Result<Json<Vec<ProxyLog>>, ApiError> {
    let limit = params.limit.unwrap_or(100).min(1000).max(1);
    let offset = params.offset.unwrap_or(0).max(0);

    // Build a dynamic query.  SQLx doesn't support fully dynamic WHERE clauses,
    // so we build the SQL string and bind parameters manually.
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_domain: Option<String> = None;
    let mut bind_status_min: Option<i64> = None;
    let mut bind_status_max: Option<i64> = None;

    if let Some(ref domain) = params.domain {
        if !domain.is_empty() {
            conditions.push("host = ?".to_string());
            bind_domain = Some(domain.clone());
        }
    }

    if let Some(ref status_class) = params.status {
        let (min, max) = match status_class.as_str() {
            "2xx" => (200i64, 299i64),
            "3xx" => (300, 399),
            "4xx" => (400, 499),
            "5xx" => (500, 599),
            _ => (0, 999),
        };
        if max < 999 {
            conditions.push("status BETWEEN ? AND ?".to_string());
            bind_status_min = Some(min);
            bind_status_max = Some(max);
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, ts, host, method, path, status, response_ms, bytes_out, client_ip, user_agent \
         FROM proxy_logs {} ORDER BY id DESC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut query = sqlx::query_as::<_, ProxyLog>(&sql);

    if let Some(ref domain) = bind_domain {
        query = query.bind(domain);
    }
    if let Some(min) = bind_status_min {
        query = query.bind(min);
    }
    if let Some(max) = bind_status_max {
        query = query.bind(max);
    }
    query = query.bind(limit).bind(offset);

    let logs = query.fetch_all(&state.db).await?;

    Ok(Json(logs))
}
