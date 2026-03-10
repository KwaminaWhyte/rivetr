//! Auto-update handlers: version info, update check, download, and apply.

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

use super::super::error::ApiError;

/// Download update response
#[derive(Debug, Clone, Serialize)]
pub struct DownloadUpdateResponse {
    pub success: bool,
    pub message: String,
    pub version: Option<String>,
    pub download_path: Option<String>,
}

/// Apply update response
#[derive(Debug, Clone, Serialize)]
pub struct ApplyUpdateResponse {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
    pub restart_required: bool,
}

/// Get system version and update status
/// GET /api/system/version
///
/// Returns the current version, latest available version, and update status.
pub async fn get_version_info(
    State(state): State<Arc<AppState>>,
) -> Json<crate::engine::updater::UpdateStatus> {
    let status = state.update_checker.get_status();
    Json(status)
}

/// Check for updates
/// POST /api/system/update/check
///
/// Triggers an immediate update check and returns the result.
pub async fn check_for_updates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<crate::engine::updater::UpdateStatus>, ApiError> {
    state.update_checker.run_check().await;
    let status = state.update_checker.get_status();
    Ok(Json(status))
}

/// Download update binary
/// POST /api/system/update/download
///
/// Downloads the latest update binary to a temporary location.
/// Does not apply the update - use /api/system/update/apply for that.
pub async fn download_update(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DownloadUpdateResponse>, ApiError> {
    // Check if update is available
    let status = state.update_checker.get_status();
    if !status.update_available {
        return Ok(Json(DownloadUpdateResponse {
            success: false,
            message: "No update available".to_string(),
            version: None,
            download_path: None,
        }));
    }

    let version = status.latest_version.clone();

    match state.update_checker.download_update().await {
        Ok(path) => Ok(Json(DownloadUpdateResponse {
            success: true,
            message: format!("Update downloaded to {}", path.display()),
            version,
            download_path: Some(path.display().to_string()),
        })),
        Err(e) => Err(ApiError::internal(format!(
            "Failed to download update: {}",
            e
        ))),
    }
}

/// Apply downloaded update
/// POST /api/system/update/apply
///
/// Applies a previously downloaded update by replacing the binary.
/// Requires service restart to take effect.
pub async fn apply_update(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApplyUpdateResponse>, ApiError> {
    let temp_path = std::env::temp_dir().join("rivetr-update");

    if !temp_path.exists() {
        return Ok(Json(ApplyUpdateResponse {
            success: false,
            message: "No downloaded update found. Run download first.".to_string(),
            backup_path: None,
            restart_required: false,
        }));
    }

    match state.update_checker.apply_update(&temp_path).await {
        Ok(backup_path) => Ok(Json(ApplyUpdateResponse {
            success: true,
            message: "Update applied successfully. Service restart required.".to_string(),
            backup_path: Some(backup_path.display().to_string()),
            restart_required: true,
        })),
        Err(e) => Err(ApiError::internal(format!("Failed to apply update: {}", e))),
    }
}
