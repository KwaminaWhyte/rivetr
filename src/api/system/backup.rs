//! Backup and restore handlers for the Rivetr instance.

use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use std::sync::Arc;

use crate::backup::{self, BackupInfo, RestoreResult};
use crate::AppState;

use super::super::error::ApiError;

/// Create a backup of the Rivetr instance and return it as a file download
/// POST /api/system/backup
///
/// Creates a .tar.gz archive containing:
/// - SQLite database (after WAL checkpoint)
/// - Configuration file (rivetr.toml)
/// - SSL/ACME certificates (if present)
pub async fn create_backup(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    let data_dir = &state.config.server.data_dir;
    let acme_cache_dir = &state.config.proxy.acme_cache_dir;

    // Determine the config file path - use the same logic as main.rs
    // The config path is stored in the Config, but we default to "rivetr.toml"
    let config_path = std::path::PathBuf::from("rivetr.toml");

    let result = backup::create_backup(&state.db, data_dir, &config_path, acme_cache_dir, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create backup");
            ApiError::internal(format!("Failed to create backup: {}", e))
        })?;

    // Read the backup file and return it as a download
    let backup_data = std::fs::read(&result.path)
        .map_err(|e| ApiError::internal(format!("Failed to read backup file: {}", e)))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", result.name),
        )
        .header(header::CONTENT_LENGTH, backup_data.len().to_string())
        .body(Body::from(backup_data))
        .map_err(|e| ApiError::internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// List existing backups
/// GET /api/system/backups
///
/// Returns a list of backup files in the data/backups/ directory.
pub async fn list_backups(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<BackupInfo>>, ApiError> {
    let data_dir = &state.config.server.data_dir;

    let backups = backup::list_backups(data_dir).map_err(|e| {
        tracing::error!(error = %e, "Failed to list backups");
        ApiError::internal(format!("Failed to list backups: {}", e))
    })?;

    Ok(Json(backups))
}

/// Delete a specific backup
/// DELETE /api/system/backups/:name
///
/// Deletes a backup file from the data/backups/ directory.
pub async fn delete_backup(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let data_dir = &state.config.server.data_dir;

    backup::delete_backup(data_dir, &name).map_err(|e| {
        tracing::error!(error = %e, "Failed to delete backup: {}", name);
        ApiError::internal(format!("Failed to delete backup: {}", e))
    })?;

    Ok(Json(serde_json::json!({
        "message": format!("Backup '{}' deleted successfully", name)
    })))
}

/// Restore from an uploaded backup archive
/// POST /api/system/restore
///
/// Accepts a multipart file upload of a .tar.gz backup archive.
/// Extracts and restores the database, config, and SSL certificates.
///
/// WARNING: This replaces the current database and config. A server restart is recommended.
pub async fn restore_backup(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<RestoreResult>, ApiError> {
    let data_dir = &state.config.server.data_dir;
    let acme_cache_dir = &state.config.proxy.acme_cache_dir;
    let config_path = std::path::PathBuf::from("rivetr.toml");

    // Read the uploaded file
    let mut backup_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to read upload: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "backup" {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read file data: {}", e)))?;
            backup_data = Some(data.to_vec());
            break;
        }
    }

    let data = backup_data.ok_or_else(|| {
        ApiError::bad_request(
            "No backup file provided. Upload a .tar.gz file with field name 'file' or 'backup'",
        )
    })?;

    if data.is_empty() {
        return Err(ApiError::bad_request("Uploaded file is empty"));
    }

    let result = backup::restore_from_backup(&data, data_dir, &config_path, acme_cache_dir)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to restore backup");
            ApiError::internal(format!("Failed to restore backup: {}", e))
        })?;

    Ok(Json(result))
}

/// Download a specific backup file
/// GET /api/system/backups/:name/download
///
/// Returns the backup file as a download.
pub async fn download_backup(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Response, ApiError> {
    let data_dir = &state.config.server.data_dir;

    let backup_data = backup::read_backup_file(data_dir, &name).map_err(|e| {
        tracing::error!(error = %e, "Failed to read backup: {}", name);
        ApiError::not_found(format!("Backup not found: {}", name))
    })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", name),
        )
        .header(header::CONTENT_LENGTH, backup_data.len().to_string())
        .body(Body::from(backup_data))
        .map_err(|e| ApiError::internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}
