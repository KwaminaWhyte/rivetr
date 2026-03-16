//! Webhook audit log — helper to persist received events and endpoint to query them.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

// ---------------------------------------------------------------------------
// DB model
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WebhookEvent {
    pub id: String,
    pub provider: String,
    pub event_type: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub payload_size: Option<i64>,
    pub apps_triggered: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub received_at: String,
}

// ---------------------------------------------------------------------------
// Helper — fire-and-forget insert
// ---------------------------------------------------------------------------

/// Persist an incoming webhook event to the audit table.
/// Errors are silently ignored so callers are never blocked.
#[allow(clippy::too_many_arguments)]
pub async fn log_webhook_event(
    db: &crate::DbPool,
    provider: &str,
    event_type: &str,
    repo: Option<&str>,
    branch: Option<&str>,
    sha: Option<&str>,
    payload_size: usize,
    apps_triggered: i64,
    status: &str,
    error: Option<&str>,
    delivery_id: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO webhook_events \
         (id, provider, event_type, repository, branch, commit_sha, payload_size, apps_triggered, status, error_message, delivery_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(provider)
    .bind(event_type)
    .bind(repo)
    .bind(branch)
    .bind(sha)
    .bind(payload_size as i64)
    .bind(apps_triggered)
    .bind(status)
    .bind(error)
    .bind(delivery_id)
    .execute(db)
    .await;
}

/// Atomically claim a delivery ID by inserting a placeholder audit row.
///
/// Returns `true` if this call was the first to claim the ID (the INSERT succeeded),
/// or `false` if the delivery ID already existed (duplicate / concurrent retry).
///
/// Must be called *before* any deployment work is queued. Because it uses
/// `INSERT OR IGNORE` against a UNIQUE index on `delivery_id`, only one
/// concurrent caller can get `true` — the UNIQUE constraint rejects all others.
pub async fn record_delivery_id(db: &crate::DbPool, provider: &str, delivery_id: &str) -> bool {
    sqlx::query(
        "INSERT OR IGNORE INTO webhook_events \
         (id, provider, event_type, apps_triggered, status, delivery_id) \
         VALUES (?, ?, 'push', 0, 'received', ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(provider)
    .bind(delivery_id)
    .execute(db)
    .await
    .map(|r| r.rows_affected() == 1)
    .unwrap_or(false)
}

/// Update the placeholder row created by `record_delivery_id` with full event details.
/// Falls back to a plain INSERT when no delivery_id is provided (non-GitHub providers).
pub async fn update_webhook_event(
    db: &crate::DbPool,
    provider: &str,
    event_type: &str,
    repo: Option<&str>,
    branch: Option<&str>,
    sha: Option<&str>,
    payload_size: usize,
    apps_triggered: i64,
    status: &str,
    error: Option<&str>,
    delivery_id: Option<&str>,
) {
    if let Some(did) = delivery_id {
        // Update the placeholder row that record_delivery_id already inserted.
        let _ = sqlx::query(
            "UPDATE webhook_events SET \
             event_type = ?, repository = ?, branch = ?, commit_sha = ?, \
             payload_size = ?, apps_triggered = ?, status = ?, error_message = ? \
             WHERE delivery_id = ?",
        )
        .bind(event_type)
        .bind(repo)
        .bind(branch)
        .bind(sha)
        .bind(payload_size as i64)
        .bind(apps_triggered)
        .bind(status)
        .bind(error)
        .bind(did)
        .execute(db)
        .await;
    } else {
        // No delivery ID — just insert a fresh row (other providers).
        let _ = sqlx::query(
            "INSERT INTO webhook_events \
             (id, provider, event_type, repository, branch, commit_sha, payload_size, apps_triggered, status, error_message, delivery_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(provider)
        .bind(event_type)
        .bind(repo)
        .bind(branch)
        .bind(sha)
        .bind(payload_size as i64)
        .bind(apps_triggered)
        .bind(status)
        .bind(error)
        .execute(db)
        .await;
    }
}

// ---------------------------------------------------------------------------
// GET /api/webhook-events
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct WebhookEventsQuery {
    pub provider: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_webhook_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebhookEventsQuery>,
) -> Result<Json<Vec<WebhookEvent>>, StatusCode> {
    let limit = query.limit.unwrap_or(100).min(500);

    let events = match (&query.provider, &query.status) {
        (Some(provider), Some(status)) => {
            sqlx::query_as::<_, WebhookEvent>(
                "SELECT * FROM webhook_events WHERE provider = ? AND status = ? \
                 ORDER BY received_at DESC LIMIT ?",
            )
            .bind(provider)
            .bind(status)
            .bind(limit)
            .fetch_all(&state.db)
            .await
        }
        (Some(provider), None) => {
            sqlx::query_as::<_, WebhookEvent>(
                "SELECT * FROM webhook_events WHERE provider = ? \
                 ORDER BY received_at DESC LIMIT ?",
            )
            .bind(provider)
            .bind(limit)
            .fetch_all(&state.db)
            .await
        }
        (None, Some(status)) => {
            sqlx::query_as::<_, WebhookEvent>(
                "SELECT * FROM webhook_events WHERE status = ? \
                 ORDER BY received_at DESC LIMIT ?",
            )
            .bind(status)
            .bind(limit)
            .fetch_all(&state.db)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, WebhookEvent>(
                "SELECT * FROM webhook_events ORDER BY received_at DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&state.db)
            .await
        }
    }
    .map_err(|e| {
        tracing::error!("Failed to fetch webhook events: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(events))
}
