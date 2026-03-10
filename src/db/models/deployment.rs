//! Deployment models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Cloning,
    Building,
    Starting,
    Checking,
    Running,
    Failed,
    Stopped,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Cloning => write!(f, "cloning"),
            Self::Building => write!(f, "building"),
            Self::Starting => write!(f, "starting"),
            Self::Checking => write!(f, "checking"),
            Self::Running => write!(f, "running"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

impl From<String> for DeploymentStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => Self::Pending,
            "cloning" => Self::Cloning,
            "building" => Self::Building,
            "starting" => Self::Starting,
            "checking" => Self::Checking,
            "running" => Self::Running,
            "failed" => Self::Failed,
            "stopped" => Self::Stopped,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub status: String,
    pub container_id: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub image_tag: Option<String>,
    /// ID of the deployment that triggered this auto-rollback (if any)
    pub rollback_from_deployment_id: Option<String>,
    /// Whether this deployment was an automatic rollback triggered by health check failure
    #[serde(default)]
    pub is_auto_rollback: i32,
    /// Git tag name when deploying from a specific tag
    #[sqlx(default)]
    pub git_tag: Option<String>,
    // Approval workflow fields (added in migration 049)
    /// Approval status: 'pending', 'approved', or 'rejected'
    #[sqlx(default)]
    pub approval_status: Option<String>,
    /// ID of the user who approved/rejected
    #[sqlx(default)]
    pub approved_by: Option<String>,
    /// Timestamp when the deployment was approved/rejected
    #[sqlx(default)]
    pub approved_at: Option<String>,
    /// Reason for rejection (if rejected)
    #[sqlx(default)]
    pub rejection_reason: Option<String>,
    /// Scheduled deployment time (ISO 8601) — scheduler picks this up
    #[sqlx(default)]
    pub scheduled_at: Option<String>,
}

impl Deployment {
    pub fn status_enum(&self) -> DeploymentStatus {
        DeploymentStatus::from(self.status.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentLog {
    pub id: i64,
    pub deployment_id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Freeze window for preventing deployments during specific time windows.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentFreezeWindow {
    pub id: String,
    pub app_id: Option<String>,
    pub team_id: Option<String>,
    pub name: String,
    /// Start time in HH:MM UTC format
    pub start_time: String,
    /// End time in HH:MM UTC format
    pub end_time: String,
    /// Comma-separated days of week: 0=Sun, 1=Mon, ..., 6=Sat
    pub days_of_week: String,
    #[serde(default)]
    pub is_active: i32,
    pub created_at: String,
}
