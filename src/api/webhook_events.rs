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

/// Claim a delivery ID and return whether this request should be processed.
///
/// Returns `false` (skip) only when the delivery_id was already **fully processed**
/// (status = 'processed' or 'ignored').  A row with status = 'received' means a
/// prior attempt started but the server restarted before finishing — we allow the
/// retry through by replacing that placeholder with a fresh one.
///
/// This prevents double-processing of genuine GitHub retries of the same delivery
/// while still allowing retries after a server crash/restart.
pub async fn record_delivery_id(db: &crate::DbPool, provider: &str, delivery_id: &str) -> bool {
    // If a *completed* row exists for this delivery ID, it was already handled.
    let already_done: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM webhook_events \
         WHERE delivery_id = ? AND status IN ('processed', 'ignored')",
    )
    .bind(delivery_id)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    if already_done > 0 {
        return false;
    }

    // Upsert a placeholder row (replaces any stale 'received' row from a prior crash).
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO webhook_events \
         (id, provider, event_type, apps_triggered, status, delivery_id) \
         VALUES (?, ?, 'push', 0, 'received', ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(provider)
    .bind(delivery_id)
    .execute(db)
    .await;

    true
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
