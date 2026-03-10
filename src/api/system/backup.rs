//! Backup and restore handlers for the Rivetr instance.

use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use uuid::Uuid;

use crate::backup::{self, BackupInfo, RestoreResult};
use crate::AppState;

use super::super::error::ApiError;

// ---------------------------------------------------------------------------
// S3 upload helpers (used by create_backup and upload_backup_to_s3)
// ---------------------------------------------------------------------------

/// Build an S3Client from the first available (default) S3 config.
async fn get_default_s3_client(state: &AppState) -> Option<(crate::backup::s3::S3Client, String)> {
    use crate::crypto;
    use crate::db::S3StorageConfig;

    let config: Option<S3StorageConfig> =
        sqlx::query_as("SELECT * FROM s3_storage_configs WHERE is_default = 1 LIMIT 1")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();

    let config = config?;

    let encryption_key = state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|s| crate::crypto::derive_key(s));

    let access_key =
        crypto::decrypt_if_encrypted(&config.access_key, encryption_key.as_ref()).ok()?;
    let secret_key =
        crypto::decrypt_if_encrypted(&config.secret_key, encryption_key.as_ref()).ok()?;

    let client = crate::backup::s3::S3Client::new(
        config.endpoint.as_deref(),
        &config.bucket,
        &config.region,
        &access_key,
        &secret_key,
        config.path_prefix.as_deref(),
    )
    .ok()?;

    // Return the S3 URL prefix for display
    let url_prefix = format!(
        "s3://{}/{}",
        config.bucket,
        config.path_prefix.as_deref().unwrap_or("")
    );
    Some((client, url_prefix))
}

// ---------------------------------------------------------------------------
// Backup Schedule models
// ---------------------------------------------------------------------------

/// A scheduled backup record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupSchedule {
    pub id: String,
    pub backup_type: String,
    pub cron_expression: String,
    pub target_id: Option<String>,
    pub s3_config_id: Option<String>,
    pub retention_days: i64,
    pub enabled: i64,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
}

/// Request to create a backup schedule
#[derive(Debug, Deserialize)]
pub struct CreateBackupScheduleRequest {
    pub backup_type: String,
    pub cron_expression: String,
    pub target_id: Option<String>,
    pub s3_config_id: Option<String>,
    pub retention_days: Option<i64>,
}

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

/// Response for backup operations that may include an S3 URL
#[derive(Debug, Serialize)]
pub struct BackupWithS3Response {
    pub name: String,
    pub local_path: String,
    pub size: u64,
    /// S3 URL if the backup was uploaded to S3, or null
    pub s3_url: Option<String>,
}

/// Upload an existing local backup to S3
/// POST /api/system/backups/:name/upload-to-s3
///
/// Reads a backup file from the local data/backups/ directory and uploads it
/// to the default S3 storage configuration.
pub async fn upload_backup_to_s3(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<BackupWithS3Response>, ApiError> {
    let data_dir = &state.config.server.data_dir;

    // Read the backup file
    let backup_data = backup::read_backup_file(data_dir, &name).map_err(|e| {
        tracing::error!(error = %e, "Failed to read backup for S3 upload: {}", name);
        ApiError::not_found(format!("Backup not found: {}", name))
    })?;

    let size = backup_data.len() as u64;

    // Get the default S3 client
    let (s3_client, url_prefix) = get_default_s3_client(&state).await.ok_or_else(|| {
        ApiError::bad_request(
            "No default S3 storage configuration found. Please configure S3 storage first.",
        )
    })?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let s3_key = format!("backups/instance/{}", name);

    s3_client
        .upload_backup(&s3_key, backup_data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to upload backup to S3: {}", name);
            ApiError::internal(format!("Failed to upload backup to S3: {}", e))
        })?;

    let s3_url = format!("{}/{}", url_prefix.trim_end_matches('/'), s3_key);
    let local_path = data_dir
        .join("backups")
        .join(&name)
        .to_string_lossy()
        .to_string();

    tracing::info!(
        backup_name = %name,
        s3_key = %s3_key,
        "Uploaded instance backup to S3"
    );

    let _ = timestamp; // suppress unused warning

    Ok(Json(BackupWithS3Response {
        name,
        local_path,
        size,
        s3_url: Some(s3_url),
    }))
}

// ---------------------------------------------------------------------------
// Backup Schedule endpoints
// ---------------------------------------------------------------------------

/// List all backup schedules
/// GET /api/backups/schedules
pub async fn list_backup_schedules(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<BackupSchedule>>, ApiError> {
    let schedules = sqlx::query_as::<_, BackupSchedule>(
        "SELECT * FROM backup_schedules ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to list backup schedules");
        ApiError::internal("Failed to list backup schedules")
    })?;

    Ok(Json(schedules))
}

/// Create a new backup schedule
/// POST /api/backups/schedules
pub async fn create_backup_schedule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBackupScheduleRequest>,
) -> Result<(StatusCode, Json<BackupSchedule>), ApiError> {
    // Validate backup_type
    if !["instance", "s3_database", "s3_volume"].contains(&req.backup_type.as_str()) {
        return Err(ApiError::bad_request(
            "backup_type must be one of: instance, s3_database, s3_volume",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let retention_days = req.retention_days.unwrap_or(30);

    // Compute initial next_run_at from cron expression
    let next_run_at = compute_next_run(&req.cron_expression);

    sqlx::query(
        r#"INSERT INTO backup_schedules
           (id, backup_type, cron_expression, target_id, s3_config_id, retention_days, enabled, next_run_at)
           VALUES (?, ?, ?, ?, ?, ?, 1, ?)"#,
    )
    .bind(&id)
    .bind(&req.backup_type)
    .bind(&req.cron_expression)
    .bind(&req.target_id)
    .bind(&req.s3_config_id)
    .bind(retention_days)
    .bind(&next_run_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create backup schedule");
        ApiError::internal("Failed to create backup schedule")
    })?;

    let schedule =
        sqlx::query_as::<_, BackupSchedule>("SELECT * FROM backup_schedules WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch created backup schedule");
                ApiError::internal("Failed to fetch created backup schedule")
            })?;

    tracing::info!(
        schedule_id = %id,
        backup_type = %req.backup_type,
        "Created backup schedule"
    );

    Ok((StatusCode::CREATED, Json(schedule)))
}

/// Delete a backup schedule
/// DELETE /api/backups/schedules/:id
pub async fn delete_backup_schedule(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM backup_schedules WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete backup schedule");
            ApiError::internal("Failed to delete backup schedule")
        })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Backup schedule not found"));
    }

    tracing::info!(schedule_id = %id, "Deleted backup schedule");
    Ok(StatusCode::NO_CONTENT)
}

/// Toggle enable/disable for a backup schedule
/// PUT /api/backups/schedules/:id/toggle
pub async fn toggle_backup_schedule(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<BackupSchedule>, ApiError> {
    // Flip enabled flag
    let result = sqlx::query(
        "UPDATE backup_schedules SET enabled = CASE WHEN enabled = 1 THEN 0 ELSE 1 END WHERE id = ?",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to toggle backup schedule");
        ApiError::internal("Failed to toggle backup schedule")
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Backup schedule not found"));
    }

    let schedule =
        sqlx::query_as::<_, BackupSchedule>("SELECT * FROM backup_schedules WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch backup schedule after toggle");
                ApiError::internal("Failed to fetch backup schedule")
            })?;

    Ok(Json(schedule))
}

/// Compute the next run time from a cron expression using the `cron` crate.
/// Falls back to 24-hours-from-now for invalid expressions.
fn compute_next_run(cron_expression: &str) -> Option<String> {
    use cron::Schedule;
    use std::str::FromStr;

    match Schedule::from_str(cron_expression) {
        Ok(schedule) => schedule
            .upcoming(chrono::Utc)
            .next()
            .map(|t| t.to_rfc3339()),
        Err(_) => {
            // Fall back to 24 hours from now for unrecognised expressions
            Some((chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339())
        }
    }
}
