//! Deployment history for Docker Compose services.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::BTreeMap;

/// Status of a service deployment.
pub mod service_deployment_status {
    pub const PENDING: &str = "pending";
    pub const BUILDING: &str = "building";
    pub const DEPLOYING: &str = "deploying";
    pub const SUCCEEDED: &str = "succeeded";
    pub const FAILED: &str = "failed";

    /// Statuses of a deployment that has not finished yet.
    pub const ACTIVE: [&str; 3] = [PENDING, BUILDING, DEPLOYING];

    pub fn is_active(status: &str) -> bool {
        ACTIVE.contains(&status)
    }
}

/// What started a service deployment.
pub mod service_deployment_trigger {
    pub const MANUAL: &str = "manual";
    pub const WEBHOOK: &str = "webhook";
    pub const ROLLBACK: &str = "rollback";
    pub const API: &str = "api";
}

/// The image a compose service ran at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceImageState {
    /// The image reference from the compose file (e.g. `localhost:5000/app:latest`).
    pub image: String,
    /// Local image ID (`sha256:...`). Always present for a created container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    /// Registry digest reference (`repo@sha256:...`) when the image came from
    /// or was pushed to a registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

impl ServiceImageState {
    /// Reference that runs exactly this image again: the registry digest when
    /// known (survives `latest` moving and a local prune), else the local image ID.
    pub fn pin_ref(&self) -> Option<String> {
        self.digest.clone().or_else(|| self.image_id.clone())
    }
}

/// Compose service name -> image state. A BTreeMap keeps the JSON stable.
pub type ServiceImageMap = BTreeMap<String, ServiceImageState>;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceDeployment {
    pub id: String,
    pub service_id: String,
    pub trigger: String,
    pub triggered_by: Option<String>,
    pub triggered_by_name: Option<String>,
    pub status: String,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub image_tag: Option<String>,
    pub images_before: Option<String>,
    pub images_after: Option<String>,
    pub rollback_of: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
}

impl ServiceDeployment {
    pub fn images_after_map(&self) -> ServiceImageMap {
        parse_image_map(self.images_after.as_deref())
    }

    pub fn to_response(&self) -> ServiceDeploymentResponse {
        ServiceDeploymentResponse {
            id: self.id.clone(),
            service_id: self.service_id.clone(),
            trigger: self.trigger.clone(),
            triggered_by: self.triggered_by.clone(),
            triggered_by_name: self.triggered_by_name.clone(),
            status: self.status.clone(),
            commit_sha: self.commit_sha.clone(),
            commit_message: self.commit_message.clone(),
            image_tag: self.image_tag.clone(),
            images_before: parse_image_map(self.images_before.as_deref()),
            images_after: parse_image_map(self.images_after.as_deref()),
            rollback_of: self.rollback_of.clone(),
            error_message: self.error_message.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            duration_ms: self.duration_ms,
        }
    }
}

pub fn parse_image_map(json: Option<&str>) -> ServiceImageMap {
    json.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeploymentResponse {
    pub id: String,
    pub service_id: String,
    pub trigger: String,
    pub triggered_by: Option<String>,
    pub triggered_by_name: Option<String>,
    pub status: String,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub image_tag: Option<String>,
    pub images_before: ServiceImageMap,
    pub images_after: ServiceImageMap,
    pub rollback_of: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceDeploymentLog {
    pub id: i64,
    pub deployment_id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
}
