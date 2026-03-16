//! Advanced monitoring API endpoints.
//!
//! Provides endpoints for log search, log retention policies,
//! uptime tracking, and scheduled container restarts.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{
    CreateScheduledRestartRequest, LogRetentionPolicy, LogRetentionPolicyResponse, LogSearchResult,
    ScheduledRestart, ScheduledRestartResponse, UpdateLogRetentionRequest,
    UpdateScheduledRestartRequest, UptimeCheck, UptimeCheckResponse, UptimeSummary,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// Log Search
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LogSearchQuery {
    pub q: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub level: Option<String>,
    pub limit: Option<i64>,
}

/// Search deployment logs for an app
///
/// GET /api/apps/:id/logs/search
pub async fn search_logs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<LogSearchQuery>,
) -> Result<Json<Vec<LogSearchResult>>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let limit = query.limit.unwrap_or(100).min(1000);

    // Build dynamic query
    let mut sql = String::from(
        r#"
        SELECT dl.id, dl.deployment_id, dl.timestamp, dl.level, dl.message
        FROM deployment_logs dl
        INNER JOIN deployments d ON d.id = dl.deployment_id
        WHERE d.app_id = ?
        "#,
    );
    let mut bind_values: Vec<String> = vec![app_id.clone()];

    if let Some(ref q) = query.q {
        sql.push_str(" AND dl.message LIKE ?");
        bind_values.push(format!("%{}%", q));
    }

    if let Some(ref from) = query.from {
        sql.push_str(" AND dl.timestamp >= ?");
        bind_values.push(from.clone());
    }

    if let Some(ref to) = query.to {
        sql.push_str(" AND dl.timestamp <= ?");
        bind_values.push(to.clone());
    }

    if let Some(ref level) = query.level {
        sql.push_str(" AND dl.level = ?");
        bind_values.push(level.clone());
    }

    sql.push_str(" ORDER BY dl.timestamp DESC LIMIT ?");
    bind_values.push(limit.to_string());

    // Execute with dynamic binds
    let mut query_builder = sqlx::query_as::<_, (i64, String, String, String, String)>(&sql);
    for val in &bind_values[..bind_values.len() - 1] {
        query_builder = query_builder.bind(val);
    }
    // Last bind is the limit (integer)
    query_builder = query_builder.bind(limit);

    let rows = query_builder.fetch_all(&state.db).await.map_err(|e| {
        tracing::error!("Failed to search logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let results: Vec<LogSearchResult> = rows
        .into_iter()
        .map(
            |(id, deployment_id, timestamp, level, message)| LogSearchResult {
                id,
                deployment_id,
                timestamp,
                level,
                message,
            },
        )
        .collect();

    Ok(Json(results))
}

// ---------------------------------------------------------------------------
// Log Retention
// ---------------------------------------------------------------------------

/// Get log retention policy for an app
///
/// GET /api/apps/:id/log-retention
pub async fn get_log_retention(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<LogRetentionPolicyResponse>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let policy: Option<LogRetentionPolicy> = sqlx::query_as(
        "SELECT id, app_id, retention_days, max_size_mb, created_at, updated_at FROM log_retention_policies WHERE app_id = ?",
    )
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get log retention policy: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match policy {
        Some(p) => Ok(Json(p.into())),
        None => {
            // Return a default policy
            Ok(Json(LogRetentionPolicyResponse {
                id: String::new(),
                app_id: app_id.clone(),
                retention_days: 30,
                max_size_mb: None,
                created_at: String::new(),
                updated_at: String::new(),
            }))
        }
    }
}

/// Update log retention policy for an app (upsert)
///
/// PUT /api/apps/:id/log-retention
pub async fn update_log_retention(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<UpdateLogRetentionRequest>,
) -> Result<Json<LogRetentionPolicyResponse>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let retention_days = req.retention_days.unwrap_or(30);
    if !(1..=365).contains(&retention_days) {
        tracing::warn!("Invalid retention_days: {}", retention_days);
        return Err(StatusCode::BAD_REQUEST);
    }

    let max_size_mb = req.max_size_mb.unwrap_or_default();

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        r#"
        INSERT INTO log_retention_policies (id, app_id, retention_days, max_size_mb, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(app_id) DO UPDATE SET
            retention_days = excluded.retention_days,
            max_size_mb = excluded.max_size_mb,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(retention_days)
    .bind(max_size_mb)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to upsert log retention policy: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Fetch the updated record
    let policy: LogRetentionPolicy = sqlx::query_as(
        "SELECT id, app_id, retention_days, max_size_mb, created_at, updated_at FROM log_retention_policies WHERE app_id = ?",
    )
    .bind(&app_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch updated policy: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        app_id = %app_id,
        retention_days = retention_days,
        "Updated log retention policy"
    );

    Ok(Json(policy.into()))
}

// ---------------------------------------------------------------------------
// Log Cleanup
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct LogCleanupResult {
    pub apps_processed: i64,
    pub logs_deleted: i64,
}

/// Trigger log cleanup based on retention policies
///
/// POST /api/system/log-cleanup
pub async fn trigger_log_cleanup(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LogCleanupResult>, StatusCode> {
    let policies: Vec<LogRetentionPolicy> = sqlx::query_as(
        "SELECT id, app_id, retention_days, max_size_mb, created_at, updated_at FROM log_retention_policies",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch retention policies: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut total_deleted: i64 = 0;
    let apps_processed = policies.len() as i64;

    for policy in &policies {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(policy.retention_days as i64);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            r#"
            DELETE FROM deployment_logs
            WHERE deployment_id IN (
                SELECT id FROM deployments WHERE app_id = ?
            ) AND timestamp < ?
            "#,
        )
        .bind(&policy.app_id)
        .bind(&cutoff_str)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to cleanup logs for app {}: {}", policy.app_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        total_deleted += result.rows_affected() as i64;
    }

    // Also clean up apps without a policy using default 30-day retention
    let default_cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let default_cutoff_str = default_cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let default_result = sqlx::query(
        r#"
        DELETE FROM deployment_logs
        WHERE deployment_id IN (
            SELECT d.id FROM deployments d
            LEFT JOIN log_retention_policies lrp ON lrp.app_id = d.app_id
            WHERE lrp.id IS NULL
        ) AND timestamp < ?
        "#,
    )
    .bind(&default_cutoff_str)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to cleanup default retention logs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    total_deleted += default_result.rows_affected() as i64;

    tracing::info!(
        apps_processed = apps_processed,
        logs_deleted = total_deleted,
        "Log cleanup completed"
    );

    Ok(Json(LogCleanupResult {
        apps_processed,
        logs_deleted: total_deleted,
    }))
}

// ---------------------------------------------------------------------------
// Uptime
// ---------------------------------------------------------------------------

/// Get uptime summary for an app
///
/// GET /api/apps/:id/uptime
pub async fn get_uptime(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<UptimeSummary>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get summary stats for last 30 days
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let total_checks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM uptime_checks WHERE app_id = ? AND checked_at >= ?",
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count uptime checks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let up_checks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM uptime_checks WHERE app_id = ? AND status = 'up' AND checked_at >= ?",
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count up checks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let down_checks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM uptime_checks WHERE app_id = ? AND status = 'down' AND checked_at >= ?",
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count down checks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let degraded_checks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM uptime_checks WHERE app_id = ? AND status = 'degraded' AND checked_at >= ?",
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count degraded checks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let avg_response_time: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(CAST(response_time_ms AS REAL)) FROM uptime_checks WHERE app_id = ? AND response_time_ms IS NOT NULL AND checked_at >= ?",
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get avg response time: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get recent checks (last 20)
    let recent: Vec<UptimeCheck> = sqlx::query_as(
        r#"
        SELECT id, app_id, status, response_time_ms, status_code, error_message, checked_at
        FROM uptime_checks
        WHERE app_id = ?
        ORDER BY checked_at DESC
        LIMIT 20
        "#,
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch recent uptime checks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let availability_percent = if total_checks > 0 {
        (up_checks as f64 / total_checks as f64) * 100.0
    } else {
        100.0
    };

    Ok(Json(UptimeSummary {
        app_id,
        availability_percent,
        total_checks,
        up_checks,
        down_checks,
        degraded_checks,
        avg_response_time_ms: avg_response_time,
        recent_checks: recent.into_iter().map(|c| c.into()).collect(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UptimeHistoryQuery {
    pub period: Option<String>,
}

/// Get uptime check history for an app
///
/// GET /api/apps/:id/uptime/history
pub async fn get_uptime_history(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<UptimeHistoryQuery>,
) -> Result<Json<Vec<UptimeCheckResponse>>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let hours = match query.period.as_deref() {
        Some("24h") | None => 24,
        Some("7d") => 168,
        Some("30d") => 720,
        _ => 24,
    };

    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let checks: Vec<UptimeCheck> = sqlx::query_as(
        r#"
        SELECT id, app_id, status, response_time_ms, status_code, error_message, checked_at
        FROM uptime_checks
        WHERE app_id = ? AND checked_at >= ?
        ORDER BY checked_at ASC
        "#,
    )
    .bind(&app_id)
    .bind(&cutoff_str)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch uptime history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<UptimeCheckResponse> = checks.into_iter().map(|c| c.into()).collect();

    Ok(Json(responses))
}

// ---------------------------------------------------------------------------
// Scheduled Restarts
// ---------------------------------------------------------------------------

/// Create a scheduled restart for an app
///
/// POST /api/apps/:id/scheduled-restarts
pub async fn create_scheduled_restart(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateScheduledRestartRequest>,
) -> Result<(StatusCode, Json<ScheduledRestartResponse>), StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Validate cron expression (normalize to 6-field before parsing)
    let normalized_cron = normalize_cron(&req.cron_expression);
    if cron::Schedule::from_str(&normalized_cron).is_err() {
        tracing::warn!("Invalid cron expression: {}", req.cron_expression);
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let enabled_int: i32 = if req.enabled { 1 } else { 0 };

    // Calculate next restart time
    let next_restart = next_run_from_cron(&req.cron_expression);

    sqlx::query(
        r#"
        INSERT INTO scheduled_restarts (id, app_id, cron_expression, enabled, next_restart, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.cron_expression)
    .bind(enabled_int)
    .bind(&next_restart)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create scheduled restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let restart: ScheduledRestart = sqlx::query_as(
        "SELECT id, app_id, cron_expression, enabled, last_restart, next_restart, created_at FROM scheduled_restarts WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch created restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        app_id = %app_id,
        cron = %req.cron_expression,
        "Created scheduled restart"
    );

    Ok((StatusCode::CREATED, Json(restart.into())))
}

/// List scheduled restarts for an app
///
/// GET /api/apps/:id/scheduled-restarts
pub async fn list_scheduled_restarts(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<ScheduledRestartResponse>>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let restarts: Vec<ScheduledRestart> = sqlx::query_as(
        "SELECT id, app_id, cron_expression, enabled, last_restart, next_restart, created_at FROM scheduled_restarts WHERE app_id = ? ORDER BY created_at DESC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list scheduled restarts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<ScheduledRestartResponse> = restarts.into_iter().map(|r| r.into()).collect();

    Ok(Json(responses))
}

/// Update a scheduled restart
///
/// PUT /api/apps/:id/scheduled-restarts/:restart_id
pub async fn update_scheduled_restart(
    State(state): State<Arc<AppState>>,
    Path((app_id, restart_id)): Path<(String, String)>,
    Json(req): Json<UpdateScheduledRestartRequest>,
) -> Result<Json<ScheduledRestartResponse>, StatusCode> {
    // Get existing
    let existing: Option<ScheduledRestart> = sqlx::query_as(
        "SELECT id, app_id, cron_expression, enabled, last_restart, next_restart, created_at FROM scheduled_restarts WHERE id = ?",
    )
    .bind(&restart_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get scheduled restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let existing = existing.ok_or(StatusCode::NOT_FOUND)?;

    if existing.app_id != app_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let new_cron = req.cron_expression.unwrap_or(existing.cron_expression);
    let new_enabled = req
        .enabled
        .map(|e| if e { 1 } else { 0 })
        .unwrap_or(existing.enabled);

    // Validate cron if changed (normalize to 6-field before parsing)
    let normalized_new_cron = normalize_cron(&new_cron);
    if cron::Schedule::from_str(&normalized_new_cron).is_err() {
        tracing::warn!("Invalid cron expression: {}", new_cron);
        return Err(StatusCode::BAD_REQUEST);
    }

    let next_restart = if new_enabled != 0 {
        next_run_from_cron(&new_cron)
    } else {
        None
    };

    sqlx::query(
        "UPDATE scheduled_restarts SET cron_expression = ?, enabled = ?, next_restart = ? WHERE id = ?",
    )
    .bind(&new_cron)
    .bind(new_enabled)
    .bind(&next_restart)
    .bind(&restart_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update scheduled restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let updated: ScheduledRestart = sqlx::query_as(
        "SELECT id, app_id, cron_expression, enabled, last_restart, next_restart, created_at FROM scheduled_restarts WHERE id = ?",
    )
    .bind(&restart_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch updated restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        app_id = %app_id,
        restart_id = %restart_id,
        "Updated scheduled restart"
    );

    Ok(Json(updated.into()))
}

/// Delete a scheduled restart
///
/// DELETE /api/apps/:id/scheduled-restarts/:restart_id
pub async fn delete_scheduled_restart(
    State(state): State<Arc<AppState>>,
    Path((app_id, restart_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    // Get existing to verify ownership
    let existing: Option<ScheduledRestart> = sqlx::query_as(
        "SELECT id, app_id, cron_expression, enabled, last_restart, next_restart, created_at FROM scheduled_restarts WHERE id = ?",
    )
    .bind(&restart_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get scheduled restart: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let existing = existing.ok_or(StatusCode::NOT_FOUND)?;

    if existing.app_id != app_id {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query("DELETE FROM scheduled_restarts WHERE id = ?")
        .bind(&restart_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete scheduled restart: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        app_id = %app_id,
        restart_id = %restart_id,
        "Deleted scheduled restart"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

use std::str::FromStr;

/// Normalise a cron expression so it always has exactly 6 fields (seconds
/// included).  The `cron` crate requires a seconds field as the first field
/// (`sec min hour day month weekday`), but users and the frontend supply the
/// conventional 5-field POSIX format (`min hour day month weekday`).
///
/// If the expression already has 6 or more whitespace-separated fields it is
/// returned unchanged; otherwise `"0"` is prepended.
fn normalize_cron(expr: &str) -> String {
    let field_count = expr.split_whitespace().count();
    if field_count >= 6 {
        expr.to_string()
    } else {
        format!("0 {}", expr)
    }
}

fn next_run_from_cron(cron_expression: &str) -> Option<String> {
    let normalized = normalize_cron(cron_expression);
    let schedule = cron::Schedule::from_str(&normalized).ok()?;
    let next = schedule.upcoming(chrono::Utc).next()?;
    Some(next.to_rfc3339())
}
