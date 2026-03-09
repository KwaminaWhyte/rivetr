//! Scheduled job models for cron-based commands inside app containers.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A scheduled job that runs a command inside an app's container on a cron schedule
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledJob {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub command: String,
    pub cron_expression: String,
    pub enabled: i32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO for ScheduledJob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJobResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub command: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ScheduledJob> for ScheduledJobResponse {
    fn from(j: ScheduledJob) -> Self {
        Self {
            id: j.id,
            app_id: j.app_id,
            name: j.name,
            command: j.command,
            cron_expression: j.cron_expression,
            enabled: j.enabled != 0,
            last_run_at: j.last_run_at,
            next_run_at: j.next_run_at,
            created_at: j.created_at,
            updated_at: j.updated_at,
        }
    }
}

/// A single execution record of a scheduled job
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledJobRun {
    pub id: String,
    pub job_id: String,
    pub status: String,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
}

/// Response DTO for ScheduledJobRun
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJobRunResponse {
    pub id: String,
    pub job_id: String,
    pub status: String,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
}

impl From<ScheduledJobRun> for ScheduledJobRunResponse {
    fn from(r: ScheduledJobRun) -> Self {
        Self {
            id: r.id,
            job_id: r.job_id,
            status: r.status,
            output: r.output,
            error_message: r.error_message,
            started_at: r.started_at,
            finished_at: r.finished_at,
            duration_ms: r.duration_ms,
        }
    }
}

/// Request to create a new scheduled job
#[derive(Debug, Deserialize)]
pub struct CreateScheduledJobRequest {
    pub name: String,
    pub command: String,
    pub cron_expression: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update an existing scheduled job
#[derive(Debug, Deserialize)]
pub struct UpdateScheduledJobRequest {
    pub name: Option<String>,
    pub command: Option<String>,
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}
