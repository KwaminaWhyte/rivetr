//! Docker/Podman resource cleanup handler.

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

use super::super::error::ApiError;

/// Response from a Docker cleanup operation
#[derive(Debug, Clone, Serialize)]
pub struct DockerCleanupResponse {
    pub success: bool,
    pub output: String,
}

/// Run Docker resource cleanup (remove dangling images)
/// POST /api/system/docker-cleanup
///
/// Runs `docker image prune -f` to remove only dangling (untagged) images,
/// freeing disk space without affecting running containers or named images.
/// Note: `docker system prune --filter dangling=true` is not a valid filter for
/// system prune; use `docker image prune` instead.
pub async fn run_docker_cleanup(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DockerCleanupResponse>, ApiError> {
    let output = tokio::process::Command::new("docker")
        .args(["image", "prune", "-f"])
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to run docker cleanup: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok(Json(DockerCleanupResponse {
        success: output.status.success(),
        output: combined.trim().to_string(),
    }))
}
