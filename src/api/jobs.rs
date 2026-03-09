//! Scheduled jobs API endpoints.
//!
//! Provides endpoints for managing cron-based scheduled jobs
//! that run commands inside app containers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use cron::Schedule;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;

use crate::db::{
    CreateScheduledJobRequest, ScheduledJob, ScheduledJobResponse, ScheduledJobRun,
    ScheduledJobRunResponse, UpdateScheduledJobRequest,
};
use crate::AppState;

/// Calculate the next run time from a cron expression
fn next_run_from_cron(cron_expression: &str) -> Option<String> {
    let schedule = Schedule::from_str(cron_expression).ok()?;
    let next = schedule.upcoming(chrono::Utc).next()?;
    Some(next.to_rfc3339())
}

/// Query parameters for listing job runs
#[derive(Debug, Deserialize)]
pub struct JobRunsQuery {
    /// Maximum number of runs to return (default: 50)
    pub limit: Option<i64>,
    /// Offset for pagination (default: 0)
    pub offset: Option<i64>,
}

/// List all scheduled jobs for an app
///
/// GET /api/apps/:app_id/jobs
pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<ScheduledJobResponse>>, StatusCode> {
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

    let jobs = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs
        WHERE app_id = ?
        ORDER BY name ASC
        "#,
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list scheduled jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<ScheduledJobResponse> = jobs.into_iter().map(|j| j.into()).collect();
    Ok(Json(responses))
}

/// Create a new scheduled job
///
/// POST /api/apps/:app_id/jobs
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateScheduledJobRequest>,
) -> Result<(StatusCode, Json<ScheduledJobResponse>), StatusCode> {
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

    // Validate inputs
    if req.name.is_empty() {
        tracing::warn!("Job name is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    if req.command.is_empty() {
        tracing::warn!("Job command is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate cron expression
    if Schedule::from_str(&req.cron_expression).is_err() {
        tracing::warn!("Invalid cron expression: {}", req.cron_expression);
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let next_run = if req.enabled {
        next_run_from_cron(&req.cron_expression)
    } else {
        None
    };

    sqlx::query(
        r#"
        INSERT INTO scheduled_jobs (id, app_id, name, command, cron_expression, enabled, next_run_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.name)
    .bind(&req.command)
    .bind(&req.cron_expression)
    .bind(if req.enabled { 1 } else { 0 })
    .bind(&next_run)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create scheduled job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let job = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs WHERE id = ?
        "#,
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(ScheduledJobResponse::from(job))))
}

/// Get a single scheduled job
///
/// GET /api/apps/:app_id/jobs/:id
pub async fn get_job(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
) -> Result<Json<ScheduledJobResponse>, StatusCode> {
    let job = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs WHERE id = ? AND app_id = ?
        "#,
    )
    .bind(&job_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get scheduled job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ScheduledJobResponse::from(job)))
}

/// Update a scheduled job
///
/// PUT /api/apps/:app_id/jobs/:id
pub async fn update_job(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
    Json(req): Json<UpdateScheduledJobRequest>,
) -> Result<Json<ScheduledJobResponse>, StatusCode> {
    // Fetch existing job
    let existing = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs WHERE id = ? AND app_id = ?
        "#,
    )
    .bind(&job_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let new_name = req.name.unwrap_or(existing.name);
    let new_command = req.command.unwrap_or(existing.command);
    let new_cron = req
        .cron_expression
        .unwrap_or(existing.cron_expression.clone());
    let new_enabled = req
        .enabled
        .map(|b| if b { 1 } else { 0 })
        .unwrap_or(existing.enabled);

    // Validate cron expression if changed
    if Schedule::from_str(&new_cron).is_err() {
        tracing::warn!("Invalid cron expression: {}", new_cron);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Recalculate next_run_at if cron changed or re-enabled
    let cron_changed = new_cron != existing.cron_expression;
    let re_enabled = new_enabled == 1 && existing.enabled == 0;
    let next_run = if new_enabled == 1 && (cron_changed || re_enabled) {
        next_run_from_cron(&new_cron)
    } else if new_enabled == 0 {
        None
    } else {
        existing.next_run_at
    };

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE scheduled_jobs SET
            name = ?,
            command = ?,
            cron_expression = ?,
            enabled = ?,
            next_run_at = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&new_name)
    .bind(&new_command)
    .bind(&new_cron)
    .bind(new_enabled)
    .bind(&next_run)
    .bind(&now)
    .bind(&job_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update scheduled job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let job = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs WHERE id = ?
        "#,
    )
    .bind(&job_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ScheduledJobResponse::from(job)))
}

/// Delete a scheduled job
///
/// DELETE /api/apps/:app_id/jobs/:id
pub async fn delete_job(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM scheduled_jobs WHERE id = ? AND app_id = ?")
        .bind(&job_id)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete scheduled job: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Manually trigger a job run
///
/// POST /api/apps/:app_id/jobs/:id/run
pub async fn trigger_job_run(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
) -> Result<Json<ScheduledJobRunResponse>, StatusCode> {
    // Fetch the job
    let job = sqlx::query_as::<_, ScheduledJob>(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs WHERE id = ? AND app_id = ?
        "#,
    )
    .bind(&job_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get scheduled job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Find the running container
    let container_id: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT container_id FROM deployments
        WHERE app_id = ? AND status = 'running' AND container_id IS NOT NULL
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(&job.app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query container: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let container_id = match container_id {
        Some((id,)) if !id.is_empty() => id,
        _ => {
            tracing::warn!("No running container found for app {}", job.app_id);
            return Err(StatusCode::PRECONDITION_FAILED);
        }
    };

    // Verify container is running
    let info = state.runtime.inspect(&container_id).await.map_err(|e| {
        tracing::error!("Failed to inspect container: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !info.running {
        return Err(StatusCode::PRECONDITION_FAILED);
    }

    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = chrono::Utc::now().to_rfc3339();

    // Insert run record
    sqlx::query(
        r#"
        INSERT INTO scheduled_job_runs (id, job_id, status, started_at)
        VALUES (?, ?, 'running', ?)
        "#,
    )
    .bind(&run_id)
    .bind(&job.id)
    .bind(&started_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create job run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Execute the command in the background
    let db = state.db.clone();
    let runtime = state.runtime.clone();
    let run_id_clone = run_id.clone();
    let started_at_clone = started_at.clone();

    tokio::spawn(async move {
        let cmd = vec!["/bin/sh".to_string(), "-c".to_string(), job.command.clone()];

        tracing::info!(
            job_id = %job.id,
            job_name = %job.name,
            run_id = %run_id_clone,
            "Manually triggered scheduled job"
        );

        match runtime.run_command(&container_id, cmd).await {
            Ok(result) => {
                let output = if result.stderr.is_empty() {
                    result.stdout.clone()
                } else {
                    format!("{}\n--- stderr ---\n{}", result.stdout, result.stderr)
                };

                let (status, error) = if result.exit_code == 0 {
                    ("success", None)
                } else {
                    (
                        "failed",
                        Some(format!("Command exited with code {}", result.exit_code)),
                    )
                };

                let finished_at = chrono::Utc::now().to_rfc3339();
                let duration_ms = chrono::DateTime::parse_from_rfc3339(&finished_at)
                    .ok()
                    .and_then(|end| {
                        chrono::DateTime::parse_from_rfc3339(&started_at_clone)
                            .ok()
                            .map(|start| (end - start).num_milliseconds())
                    });

                let _ = sqlx::query(
                    r#"
                    UPDATE scheduled_job_runs
                    SET status = ?, output = ?, error_message = ?, finished_at = ?, duration_ms = ?
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(&output)
                .bind(error.as_deref())
                .bind(&finished_at)
                .bind(duration_ms)
                .bind(&run_id_clone)
                .execute(&db)
                .await;

                // Update job's last_run_at
                let _ = sqlx::query(
                    "UPDATE scheduled_jobs SET last_run_at = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&finished_at)
                .bind(&finished_at)
                .bind(&job.id)
                .execute(&db)
                .await;
            }
            Err(e) => {
                let finished_at = chrono::Utc::now().to_rfc3339();
                let duration_ms = chrono::DateTime::parse_from_rfc3339(&finished_at)
                    .ok()
                    .and_then(|end| {
                        chrono::DateTime::parse_from_rfc3339(&started_at_clone)
                            .ok()
                            .map(|start| (end - start).num_milliseconds())
                    });

                let _ = sqlx::query(
                    r#"
                    UPDATE scheduled_job_runs
                    SET status = 'failed', error_message = ?, finished_at = ?, duration_ms = ?
                    WHERE id = ?
                    "#,
                )
                .bind(format!("Exec failed: {}", e))
                .bind(&finished_at)
                .bind(duration_ms)
                .bind(&run_id_clone)
                .execute(&db)
                .await;
            }
        }
    });

    // Return the initial run record (status = running)
    let run = sqlx::query_as::<_, ScheduledJobRun>(
        r#"
        SELECT id, job_id, status, output, error_message, started_at, finished_at, duration_ms
        FROM scheduled_job_runs WHERE id = ?
        "#,
    )
    .bind(&run_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ScheduledJobRunResponse::from(run)))
}

/// List job run history
///
/// GET /api/apps/:app_id/jobs/:id/runs
pub async fn list_job_runs(
    State(state): State<Arc<AppState>>,
    Path((app_id, job_id)): Path<(String, String)>,
    Query(query): Query<JobRunsQuery>,
) -> Result<Json<Vec<ScheduledJobRunResponse>>, StatusCode> {
    // Verify job exists and belongs to app
    let job_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scheduled_jobs WHERE id = ? AND app_id = ?",
    )
    .bind(&job_id)
    .bind(&app_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if job_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let limit = query.limit.unwrap_or(50).min(200);
    let offset = query.offset.unwrap_or(0);

    let runs = sqlx::query_as::<_, ScheduledJobRun>(
        r#"
        SELECT id, job_id, status, output, error_message, started_at, finished_at, duration_ms
        FROM scheduled_job_runs
        WHERE job_id = ?
        ORDER BY started_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&job_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list job runs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<ScheduledJobRunResponse> = runs.into_iter().map(|r| r.into()).collect();
    Ok(Json(responses))
}
