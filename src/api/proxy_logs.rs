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
    /// Page number (1-based, default 1)
    pub page: Option<i64>,
    /// Rows per page (default 100, max 1000)
    pub per_page: Option<i64>,
}

// ── Response ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ProxyLogListResponse {
    pub items: Vec<ProxyLog>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// List proxy access logs.
///
/// GET /api/proxy/logs?domain=&status=2xx&page=1&per_page=100
pub async fn list_proxy_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListProxyLogsParams>,
) -> Result<Json<ProxyLogListResponse>, ApiError> {
    let per_page = params.per_page.unwrap_or(100).clamp(1, 1000);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    // Build dynamic WHERE clause
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

    // COUNT query
    let count_sql = format!("SELECT COUNT(*) FROM proxy_logs {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(ref domain) = bind_domain {
        count_query = count_query.bind(domain);
    }
    if let Some(min) = bind_status_min {
        count_query = count_query.bind(min);
    }
    if let Some(max) = bind_status_max {
        count_query = count_query.bind(max);
    }
    let total: i64 = count_query.fetch_one(&state.db).await?;

    // Data query
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
    query = query.bind(per_page).bind(offset);

    let items = query.fetch_all(&state.db).await?;
    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(ProxyLogListResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
    }))
}
