//! Audit log API endpoints and helpers.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use std::{net::SocketAddr, sync::Arc};

use crate::db::{list_audit_logs, log_audit, AuditLogListResponse, AuditLogQuery};
use crate::AppState;

use super::error::ApiError;

/// Extract client IP address from request headers or connection info.
/// Checks X-Forwarded-For, X-Real-IP headers first (for reverse proxy scenarios),
/// then falls back to the connection info.
pub fn extract_client_ip(headers: &HeaderMap, conn_info: Option<&SocketAddr>) -> Option<String> {
    // Check X-Forwarded-For header first (comma-separated list, first is client)
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(first_ip) = forwarded.split(',').next() {
            let ip = first_ip.trim();
            if !ip.is_empty() {
                return Some(ip.to_string());
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let ip = real_ip.trim();
        if !ip.is_empty() {
            return Some(ip.to_string());
        }
    }

    // Fall back to connection info
    conn_info.map(|addr| addr.ip().to_string())
}

/// Helper function to log an audit event with common patterns.
/// This is a convenience wrapper around db::log_audit that handles errors gracefully.
pub async fn audit_log(
    state: &AppState,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    resource_name: Option<&str>,
    user_id: Option<&str>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) {
    if let Err(e) = log_audit(
        &state.db,
        action,
        resource_type,
        resource_id,
        resource_name,
        user_id,
        ip_address,
        details,
    )
    .await
    {
        // Log the error but don't fail the request
        tracing::warn!(
            action = action,
            resource_type = resource_type,
            error = %e,
            "Failed to create audit log entry"
        );
    }
}

/// List audit logs with filtering and pagination
///
/// Query parameters:
/// - action: Filter by action type (e.g., "app.create")
/// - resource_type: Filter by resource type (e.g., "app", "database")
/// - resource_id: Filter by specific resource ID
/// - user_id: Filter by user ID
/// - start_date: Start date for date range filter (ISO 8601)
/// - end_date: End date for date range filter (ISO 8601)
/// - page: Page number (1-indexed, defaults to 1)
/// - per_page: Items per page (defaults to 50, max 100)
pub async fn list_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<AuditLogListResponse>, ApiError> {
    let result = list_audit_logs(&state.db, &query).await?;
    Ok(Json(result))
}

/// Get distinct action types for filtering UI
pub async fn list_action_types(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    let actions: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT action FROM audit_logs ORDER BY action"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(actions.into_iter().map(|(a,)| a).collect()))
}

/// Get distinct resource types for filtering UI
pub async fn list_resource_types(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    let types: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT resource_type FROM audit_logs ORDER BY resource_type"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(types.into_iter().map(|(t,)| t).collect()))
}
