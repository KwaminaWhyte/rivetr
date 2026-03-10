//! Models for advanced monitoring: log retention, uptime checks, scheduled restarts.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------------------------------------------------------------------------
// Log Retention Policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogRetentionPolicy {
    pub id: String,
    pub app_id: String,
    pub retention_days: i32,
    pub max_size_mb: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRetentionPolicyResponse {
    pub id: String,
    pub app_id: String,
    pub retention_days: i32,
    pub max_size_mb: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<LogRetentionPolicy> for LogRetentionPolicyResponse {
    fn from(p: LogRetentionPolicy) -> Self {
        Self {
            id: p.id,
            app_id: p.app_id,
            retention_days: p.retention_days,
            max_size_mb: p.max_size_mb,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateLogRetentionRequest {
    pub retention_days: Option<i32>,
    pub max_size_mb: Option<Option<i32>>,
}

// ---------------------------------------------------------------------------
// Uptime Check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UptimeCheck {
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub response_time_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeCheckResponse {
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub response_time_ms: Option<i32>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub checked_at: String,
}

impl From<UptimeCheck> for UptimeCheckResponse {
    fn from(c: UptimeCheck) -> Self {
        Self {
            id: c.id,
            app_id: c.app_id,
            status: c.status,
            response_time_ms: c.response_time_ms,
            status_code: c.status_code,
            error_message: c.error_message,
            checked_at: c.checked_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UptimeSummary {
    pub app_id: String,
    pub availability_percent: f64,
    pub total_checks: i64,
    pub up_checks: i64,
    pub down_checks: i64,
    pub degraded_checks: i64,
    pub avg_response_time_ms: Option<f64>,
    pub recent_checks: Vec<UptimeCheckResponse>,
}

// ---------------------------------------------------------------------------
// Scheduled Restart
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledRestart {
    pub id: String,
    pub app_id: String,
    pub cron_expression: String,
    pub enabled: i32,
    pub last_restart: Option<String>,
    pub next_restart: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRestartResponse {
    pub id: String,
    pub app_id: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_restart: Option<String>,
    pub next_restart: Option<String>,
    pub created_at: String,
}

impl From<ScheduledRestart> for ScheduledRestartResponse {
    fn from(r: ScheduledRestart) -> Self {
        Self {
            id: r.id,
            app_id: r.app_id,
            cron_expression: r.cron_expression,
            enabled: r.enabled != 0,
            last_restart: r.last_restart,
            next_restart: r.next_restart,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateScheduledRestartRequest {
    pub cron_expression: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateScheduledRestartRequest {
    pub cron_expression: Option<String>,
    pub enabled: Option<bool>,
}

// ---------------------------------------------------------------------------
// Log Search
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LogSearchResult {
    pub id: i64,
    pub deployment_id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
}
