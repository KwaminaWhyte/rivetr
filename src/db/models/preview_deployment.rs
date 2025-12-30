//! Preview deployment models for PR-based preview environments.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Status of a preview deployment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewDeploymentStatus {
    Pending,
    Cloning,
    Building,
    Starting,
    Running,
    Failed,
    Closed,
}

impl std::fmt::Display for PreviewDeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Cloning => write!(f, "cloning"),
            Self::Building => write!(f, "building"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Failed => write!(f, "failed"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

impl From<String> for PreviewDeploymentStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => Self::Pending,
            "cloning" => Self::Cloning,
            "building" => Self::Building,
            "starting" => Self::Starting,
            "running" => Self::Running,
            "failed" => Self::Failed,
            "closed" => Self::Closed,
            _ => Self::Pending,
        }
    }
}

impl From<&str> for PreviewDeploymentStatus {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

/// A preview deployment for a pull request
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PreviewDeployment {
    pub id: String,
    pub app_id: String,

    // PR information
    pub pr_number: i64,
    pub pr_title: Option<String>,
    pub pr_source_branch: String,
    pub pr_target_branch: String,
    pub pr_author: Option<String>,
    pub pr_url: Option<String>,

    // Git provider info
    pub provider_type: String,
    pub repo_full_name: String,

    // Deployment info
    pub preview_domain: String,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub image_tag: Option<String>,
    pub port: Option<i64>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub status: String,
    pub error_message: Option<String>,

    // Comment tracking
    pub github_comment_id: Option<i64>,

    // Resource limits
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,

    // Timestamps
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

impl PreviewDeployment {
    /// Get the status as an enum
    pub fn status_enum(&self) -> PreviewDeploymentStatus {
        PreviewDeploymentStatus::from(self.status.clone())
    }

    /// Check if the preview is active (not closed or failed)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status_enum(),
            PreviewDeploymentStatus::Pending
                | PreviewDeploymentStatus::Cloning
                | PreviewDeploymentStatus::Building
                | PreviewDeploymentStatus::Starting
                | PreviewDeploymentStatus::Running
        )
    }
}

/// Response DTO for preview deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewDeploymentResponse {
    pub id: String,
    pub app_id: String,
    pub pr_number: i64,
    pub pr_title: Option<String>,
    pub pr_source_branch: String,
    pub pr_target_branch: String,
    pub pr_author: Option<String>,
    pub pr_url: Option<String>,
    pub provider_type: String,
    pub repo_full_name: String,
    pub preview_domain: String,
    pub preview_url: String,
    pub container_id: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
}

impl From<PreviewDeployment> for PreviewDeploymentResponse {
    fn from(p: PreviewDeployment) -> Self {
        let preview_url = format!("https://{}", p.preview_domain);
        Self {
            id: p.id,
            app_id: p.app_id,
            pr_number: p.pr_number,
            pr_title: p.pr_title,
            pr_source_branch: p.pr_source_branch,
            pr_target_branch: p.pr_target_branch,
            pr_author: p.pr_author,
            pr_url: p.pr_url,
            provider_type: p.provider_type,
            repo_full_name: p.repo_full_name,
            preview_domain: p.preview_domain,
            preview_url,
            container_id: p.container_id,
            commit_sha: p.commit_sha,
            commit_message: p.commit_message,
            status: p.status,
            error_message: p.error_message,
            memory_limit: p.memory_limit,
            cpu_limit: p.cpu_limit,
            created_at: p.created_at,
            updated_at: p.updated_at,
            closed_at: p.closed_at,
        }
    }
}

/// Request to create or update a preview deployment
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePreviewRequest {
    pub pr_number: i64,
    pub pr_title: Option<String>,
    pub pr_source_branch: String,
    pub pr_target_branch: String,
    pub pr_author: Option<String>,
    pub pr_url: Option<String>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
}
