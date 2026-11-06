//! Scheduled restart execution and run history.
//!
//! The CRUD endpoints for schedules live in `monitoring.rs`; the timing loop in
//! `crate::engine::restart_scheduler`. This module performs one run: it skips
//! stopped apps, restarts through the same path as the Restart button
//! (`apps::restart_app_with`: zero-downtime swap, multi-server placements,
//! proxy routes), then restarts extra replicas one at a time, records the run
//! and notifies on failure.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{App, NotificationEventType};
use crate::engine::restart_scheduler::{DueRun, ScheduleRow};
use crate::notifications::{NotificationPayload, NotificationService};
use crate::AppState;

use super::apps::{restart_app_with, RestartActor};
use super::error::ApiError;
use super::validation::validate_uuid;

/// One scheduled restart run.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduledRestartRun {
    pub id: String,
    pub schedule_id: String,
    pub app_id: String,
    pub scheduled_for: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// running | succeeded | failed | skipped
    pub status: String,
    pub catch_up: i32,
    pub deployment_id: Option<String>,
    pub error_message: Option<String>,
}

/// Final outcome of a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    Succeeded { deployment_id: Option<String> },
    Skipped(String),
    Failed(String),
}

impl RunOutcome {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Succeeded { .. } => "succeeded",
            Self::Skipped(_) => "skipped",
            Self::Failed(_) => "failed",
        }
    }
}

/// Is the app currently running? Stopped apps (or never deployed ones) are
/// skipped. Multi-server start/stop keeps the latest deployment's status in
/// step, so one check covers both kinds.
pub async fn app_is_running(db: &crate::DbPool, app_id: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM deployments WHERE app_id = ? AND status = 'running'",
    )
    .bind(app_id)
    .fetch_one(db)
    .await
    .map(|n| n > 0)
    .unwrap_or(false)
}

async fn insert_run(db: &crate::DbPool, row: &ScheduleRow, run: &DueRun) -> Option<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let res = sqlx::query(
        "INSERT INTO scheduled_restart_runs \
         (id, schedule_id, app_id, scheduled_for, started_at, status, catch_up) \
         VALUES (?, ?, ?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(&row.id)
    .bind(&row.app_id)
    .bind(run.scheduled_for.to_rfc3339())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(run.catch_up as i32)
    .execute(db)
    .await;
    match res {
        Ok(_) => Some(id),
        Err(e) => {
            tracing::warn!(schedule = %row.id, "Failed to record scheduled restart run: {}", e);
            None
        }
    }
}

/// Store a run's outcome.
pub async fn finish_run(db: &crate::DbPool, run_id: &str, outcome: &RunOutcome) {
    let (deployment_id, error) = match outcome {
        RunOutcome::Succeeded { deployment_id } => (deployment_id.clone(), None),
        RunOutcome::Skipped(r) | RunOutcome::Failed(r) => (None, Some(r.clone())),
    };
    let _ = sqlx::query(
        "UPDATE scheduled_restart_runs SET status = ?, finished_at = ?, deployment_id = ?, \
         error_message = ? WHERE id = ?",
    )
    .bind(outcome.status())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(deployment_id)
    .bind(error)
    .bind(run_id)
    .execute(db)
    .await;
}

/// Restart extra replicas (index 1 and up) one at a time, so the others keep
/// serving. Replica 0 is the primary container restarted by the main path.
async fn restart_replicas(state: &Arc<AppState>, app: &App) -> Result<usize, String> {
    if crate::engine::multi_server::is_enabled(&state.db, &app.id).await {
        return Ok(0); // placements were restarted by the main path
    }
    let indexes: Vec<i64> = sqlx::query_scalar(
        "SELECT replica_index FROM app_replicas WHERE app_id = ? AND replica_index > 0 \
         ORDER BY replica_index",
    )
    .bind(&app.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    for index in &indexes {
        if let Err(e) =
            super::replicas::restart_replica(State(state.clone()), Path((app.id.clone(), *index)))
                .await
        {
            errors.push(format!("replica {}: {}", index, e.message()));
        }
    }
    if errors.is_empty() {
        Ok(indexes.len())
    } else {
        Err(errors.join("; "))
    }
}

/// Perform one claimed run.
pub async fn execute_run(state: &Arc<AppState>, row: &ScheduleRow, run: &DueRun) {
    let Some(run_id) = insert_run(&state.db, row, run).await else {
        return;
    };
    let app: Option<App> = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(&row.app_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    let Some(app) = app else {
        finish_run(
            &state.db,
            &run_id,
            &RunOutcome::Failed("App not found".into()),
        )
        .await;
        return;
    };

    let outcome = if !app_is_running(&state.db, &app.id).await {
        RunOutcome::Skipped("App is stopped".into())
    } else {
        let label = format!(
            "scheduled restart ({}){}",
            row.cron_expression,
            if run.catch_up { ", catch-up" } else { "" }
        );
        let actor = RestartActor {
            triggered_by: &label,
            user_id: None,
            client_ip: None,
        };
        tracing::info!(app = %app.name, schedule = %row.id, catch_up = run.catch_up, "Running scheduled restart");
        match restart_app_with(state, &app, &actor).await {
            Ok(status) => match restart_replicas(state, &app).await {
                Ok(_) => RunOutcome::Succeeded {
                    deployment_id: status.active_deployment_id,
                },
                Err(e) => RunOutcome::Failed(format!("Primary restarted, but {}", e)),
            },
            Err(e) => RunOutcome::Failed(e.message().to_string()),
        }
    };

    finish_run(&state.db, &run_id, &outcome).await;

    if let RunOutcome::Failed(reason) = &outcome {
        tracing::warn!(app = %app.name, schedule = %row.id, "Scheduled restart failed: {}", reason);
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentFailed,
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            deployment_id: None,
            status: "failed".into(),
            message: format!("Scheduled restart ({}) failed", row.cron_expression),
            error_message: Some(reason.clone()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            extras: Default::default(),
        };
        if let Err(e) = NotificationService::new(state.db.clone())
            .send(&payload)
            .await
        {
            tracing::warn!(app = %app.name, "Failed to send scheduled restart notification: {}", e);
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RunsQuery {
    pub limit: Option<i64>,
}

/// GET /api/apps/:id/scheduled-restarts/runs
///
/// Recent runs of every restart schedule of the app, newest first.
pub async fn list_runs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(q): Query<RunsQuery>,
) -> Result<Json<Vec<ScheduledRestartRun>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    let limit = q.limit.unwrap_or(20).clamp(1, 200);
    let runs = sqlx::query_as::<_, ScheduledRestartRun>(
        "SELECT * FROM scheduled_restart_runs WHERE app_id = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(&app_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(runs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_are_recorded_and_stopped_apps_detected() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO apps (id, name, git_url, branch, port) VALUES ('a1', 'one', 'https://e.com/r.git', 'main', 3000)",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO scheduled_restarts (id, app_id, cron_expression, enabled) VALUES ('s1', 'a1', '0 3 * * *', 1)",
        )
        .execute(&db)
        .await
        .unwrap();

        assert!(!app_is_running(&db, "a1").await, "never deployed");
        sqlx::query("INSERT INTO deployments (id, app_id, status) VALUES ('d1', 'a1', 'stopped')")
            .execute(&db)
            .await
            .unwrap();
        assert!(!app_is_running(&db, "a1").await, "stopped");
        sqlx::query("UPDATE deployments SET status = 'running' WHERE id = 'd1'")
            .execute(&db)
            .await
            .unwrap();
        assert!(app_is_running(&db, "a1").await);

        let row = ScheduleRow {
            id: "s1".into(),
            app_id: "a1".into(),
            cron_expression: "0 3 * * *".into(),
            next_restart: None,
        };
        let due = DueRun {
            scheduled_for: chrono::Utc::now(),
            catch_up: true,
        };
        let run_id = insert_run(&db, &row, &due).await.unwrap();
        finish_run(&db, &run_id, &RunOutcome::Failed("boom".into())).await;
        let r: ScheduledRestartRun =
            sqlx::query_as("SELECT * FROM scheduled_restart_runs WHERE id = ?")
                .bind(&run_id)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(r.status, "failed");
        assert_eq!(r.catch_up, 1);
        assert_eq!(r.error_message.as_deref(), Some("boom"));
        assert!(r.finished_at.is_some());
    }
}
