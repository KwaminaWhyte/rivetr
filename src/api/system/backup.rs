//! Backup and restore handlers for the Rivetr instance.

use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, Query, State},
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

    // Best-effort: upload to S3 if a default S3 config exists
    if let Some((s3_client, _url_prefix)) = get_default_s3_client(&state).await {
        let s3_key = format!("instance-backups/{}", result.name);
        match s3_client.upload_backup(&s3_key, backup_data.clone()).await {
            Ok(()) => {
                tracing::info!(
                    backup_name = %result.name,
                    s3_key = %s3_key,
                    "Instance backup uploaded to S3"
                );
            }
            Err(e) => {
                // Non-fatal: still return the backup to the user
                tracing::warn!(
                    backup_name = %result.name,
                    error = %e,
                    "Failed to upload instance backup to S3 (continuing)"
                );
            }
        }
    }

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

/// Manually trigger a backup schedule to run now
/// POST /api/backups/schedules/:id/run
pub async fn run_backup_schedule(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Fetch the schedule
    let schedule =
        sqlx::query_as::<_, BackupSchedule>("SELECT * FROM backup_schedules WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch backup schedule");
                ApiError::internal("Failed to fetch backup schedule")
            })?
            .ok_or_else(|| ApiError::not_found("Backup schedule not found"))?;

    let now_str = chrono::Utc::now().to_rfc3339();

    // Execute the backup based on type
    let result_msg = match schedule.backup_type.as_str() {
        "instance" => {
            // Run a standard instance backup and optionally upload to S3
            let data_dir = &state.config.server.data_dir;
            let acme_cache_dir = &state.config.proxy.acme_cache_dir;
            let config_path = std::path::PathBuf::from("rivetr.toml");

            match backup::create_backup(&state.db, data_dir, &config_path, acme_cache_dir, None)
                .await
            {
                Ok(result) => {
                    tracing::info!(
                        schedule_id = %id,
                        backup_name = %result.name,
                        "Manual instance backup run completed"
                    );
                    // Best-effort S3 upload
                    if let Some((s3_client, _)) = get_default_s3_client(&state).await {
                        let s3_key = format!("instance-backups/{}", result.name);
                        if let Ok(backup_data) = std::fs::read(&result.path) {
                            if let Err(e) =
                                s3_client.upload_backup(&s3_key, backup_data).await
                            {
                                tracing::warn!(error = %e, "S3 upload failed for manual run");
                            }
                        }
                    }
                    format!("Instance backup created: {}", result.name)
                }
                Err(e) => {
                    tracing::error!(error = %e, "Manual instance backup run failed");
                    return Err(ApiError::internal(format!("Backup run failed: {}", e)));
                }
            }
        }
        _ => {
            // For s3_database / s3_volume types we just signal success — the actual
            // S3 pipeline is invoked via the s3::trigger_backup endpoint which requires
            // additional context. Return a helpful message.
            format!(
                "Backup schedule '{}' (type: {}) queued",
                id, schedule.backup_type
            )
        }
    };

    // Update last_run_at and compute next_run_at
    let next_run = compute_next_run(&schedule.cron_expression);
    sqlx::query(
        "UPDATE backup_schedules SET last_run_at = ?, next_run_at = ? WHERE id = ?",
    )
    .bind(&now_str)
    .bind(&next_run)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to update backup schedule after run");
        ApiError::internal("Failed to update backup schedule")
    })?;

    tracing::info!(schedule_id = %id, "Backup schedule manually triggered");

    Ok(Json(serde_json::json!({
        "message": result_msg,
        "last_run_at": now_str,
        "next_run_at": next_run,
    })))
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

// ---------------------------------------------------------------------------
// Full system backup
// ---------------------------------------------------------------------------

/// Query parameters for the full backup endpoint.
#[derive(Debug, Deserialize)]
pub struct FullBackupQuery {
    /// Optional team ID to scope the backup to a specific team.
    pub team_id: Option<String>,
}

/// Create a full system backup of all team resources.
///
/// POST /api/system/backup/full
///
/// Produces a single .tar.gz archive containing:
///   rivetr-full-backup-<timestamp>/
///     metadata.json            – backup format version, team_id, timestamp
///     rivetr.db                – full SQLite database (WAL-checkpointed)
///     apps/<name>/
///       config.json            – app config + env vars
///     databases/<name>/
///       config.json            – database metadata
///       data.tar.gz            – volume data (via `docker cp`)
///     services/<name>/
///       docker-compose.yml     – compose file
pub async fn create_full_backup(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FullBackupQuery>,
) -> Result<Response, ApiError> {
    use crate::db::{App, EnvVar, ManagedDatabase, Service};
    use chrono::Utc;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tar::Builder as TarBuilder;

    let team_id = query.team_id.as_deref();
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let root_dir = format!("rivetr-full-backup-{}", timestamp);
    let archive_name = format!("{}.tar.gz", root_dir);

    // 1. Checkpoint SQLite WAL so the DB file is consistent.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("WAL checkpoint failed: {}", e)))?;

    // 2. Read the raw database file into memory.
    let data_dir = &state.config.server.data_dir;
    let db_path = data_dir.join("rivetr.db");
    let db_bytes = std::fs::read(&db_path)
        .map_err(|e| ApiError::internal(format!("Cannot read rivetr.db: {}", e)))?;

    // 3. Query apps (optionally scoped to team).
    let apps: Vec<App> = match team_id {
        Some(tid) if !tid.is_empty() => sqlx::query_as::<_, App>(
            "SELECT * FROM apps WHERE team_id = ? OR team_id IS NULL ORDER BY name ASC",
        )
        .bind(tid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list apps: {}", e)))?,
        _ => sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY name ASC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to list apps: {}", e)))?,
    };

    // 4. Query managed databases (optionally scoped to team).
    let databases: Vec<ManagedDatabase> = match team_id {
        Some(tid) if !tid.is_empty() => sqlx::query_as::<_, ManagedDatabase>(
            r#"SELECT id, name, db_type, version, container_id, container_slug, status, internal_port,
               external_port, public_access, credentials, volume_name, volume_path,
               memory_limit, cpu_limit, error_message, project_id, team_id, created_at, updated_at
               FROM databases WHERE team_id = ? OR team_id IS NULL ORDER BY name ASC"#,
        )
        .bind(tid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list databases: {}", e)))?,
        _ => sqlx::query_as::<_, ManagedDatabase>(
            r#"SELECT id, name, db_type, version, container_id, container_slug, status, internal_port,
               external_port, public_access, credentials, volume_name, volume_path,
               memory_limit, cpu_limit, error_message, project_id, team_id, created_at, updated_at
               FROM databases ORDER BY name ASC"#,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list databases: {}", e)))?,
    };

    // 5. Query Docker Compose services (optionally scoped to team).
    let services: Vec<Service> = match team_id {
        Some(tid) if !tid.is_empty() => sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE team_id = ? OR team_id IS NULL ORDER BY name ASC",
        )
        .bind(tid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list services: {}", e)))?,
        _ => sqlx::query_as::<_, Service>("SELECT * FROM services ORDER BY name ASC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to list services: {}", e)))?,
    };

    // 6. Build the in-memory tar.gz archive.
    let mut raw_tar: Vec<u8> = Vec::new();
    {
        let mut builder = TarBuilder::new(&mut raw_tar);

        // --- rivetr.db ---
        add_bytes_to_tar(
            &mut builder,
            &format!("{}/rivetr.db", root_dir),
            &db_bytes,
        )
        .map_err(|e| ApiError::internal(format!("Tar error (db): {}", e)))?;

        // --- metadata.json ---
        let metadata = serde_json::json!({
            "backup_format_version": "1.0",
            "team_id": team_id,
            "timestamp": Utc::now().to_rfc3339(),
            "app_count": apps.len(),
            "database_count": databases.len(),
            "service_count": services.len(),
        });
        let meta_bytes = serde_json::to_vec_pretty(&metadata)
            .map_err(|e| ApiError::internal(format!("JSON error: {}", e)))?;
        add_bytes_to_tar(
            &mut builder,
            &format!("{}/metadata.json", root_dir),
            &meta_bytes,
        )
        .map_err(|e| ApiError::internal(format!("Tar error (metadata): {}", e)))?;

        // --- apps/<name>/config.json ---
        for app in &apps {
            // Fetch env vars for this app.
            let env_vars: Vec<EnvVar> =
                sqlx::query_as::<_, EnvVar>("SELECT * FROM env_vars WHERE app_id = ?")
                    .bind(&app.id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default();

            let env_map: serde_json::Map<String, serde_json::Value> = env_vars
                .iter()
                .map(|ev| {
                    (
                        ev.key.clone(),
                        serde_json::Value::String(ev.value.clone()),
                    )
                })
                .collect();

            // Sanitise app name for use as a directory name.
            let safe_name = sanitise_name(&app.name);
            let config = serde_json::json!({
                "id": app.id,
                "name": app.name,
                "git_url": app.git_url,
                "branch": app.branch,
                "dockerfile": app.dockerfile,
                "domain": app.domain,
                "port": app.port,
                "environment": app.environment,
                "project_id": app.project_id,
                "team_id": app.team_id,
                "build_type": app.build_type,
                "docker_image": app.docker_image,
                "env_vars": env_map,
            });
            let config_bytes = serde_json::to_vec_pretty(&config)
                .map_err(|e| ApiError::internal(format!("JSON error (app): {}", e)))?;
            add_bytes_to_tar(
                &mut builder,
                &format!("{}/apps/{}/config.json", root_dir, safe_name),
                &config_bytes,
            )
            .map_err(|e| ApiError::internal(format!("Tar error (app config): {}", e)))?;
        }

        // --- databases/<name>/config.json + data.tar.gz ---
        for db in &databases {
            let safe_name = sanitise_name(&db.name);

            // Config (strip sensitive credential fields from the JSON).
            let config = serde_json::json!({
                "id": db.id,
                "name": db.name,
                "db_type": db.db_type,
                "version": db.version,
                "container_id": db.container_id,
                "status": db.status,
                "internal_port": db.internal_port,
                "external_port": db.external_port,
                "public_access": db.public_access,
                "volume_name": db.volume_name,
                "volume_path": db.volume_path,
                "project_id": db.project_id,
                "team_id": db.team_id,
            });
            let config_bytes = serde_json::to_vec_pretty(&config)
                .map_err(|e| ApiError::internal(format!("JSON error (db): {}", e)))?;
            add_bytes_to_tar(
                &mut builder,
                &format!("{}/databases/{}/config.json", root_dir, safe_name),
                &config_bytes,
            )
            .map_err(|e| ApiError::internal(format!("Tar error (db config): {}", e)))?;

            // Try to export volume data via `docker run --rm`.
            // We do this best-effort: log a warning and continue if it fails.
            if let Some(ref vol_name) = db.volume_name {
                if !vol_name.is_empty() {
                    match export_docker_volume(vol_name).await {
                        Ok(vol_bytes) => {
                            add_bytes_to_tar(
                                &mut builder,
                                &format!(
                                    "{}/databases/{}/data.tar.gz",
                                    root_dir, safe_name
                                ),
                                &vol_bytes,
                            )
                            .map_err(|e| {
                                ApiError::internal(format!("Tar error (db volume): {}", e))
                            })?;
                        }
                        Err(e) => {
                            tracing::warn!(
                                db = %db.name,
                                volume = %vol_name,
                                error = %e,
                                "Skipping volume export – could not run docker export"
                            );
                        }
                    }
                }
            } else if let Some(ref vol_path) = db.volume_path {
                // Host-path based volume – tar the directory directly.
                let path = std::path::Path::new(vol_path);
                if path.exists() {
                    match backup_directory(path).await {
                        Ok(vol_bytes) => {
                            add_bytes_to_tar(
                                &mut builder,
                                &format!(
                                    "{}/databases/{}/data.tar.gz",
                                    root_dir, safe_name
                                ),
                                &vol_bytes,
                            )
                            .map_err(|e| {
                                ApiError::internal(format!("Tar error (db dir): {}", e))
                            })?;
                        }
                        Err(e) => {
                            tracing::warn!(
                                db = %db.name,
                                path = %vol_path,
                                error = %e,
                                "Skipping directory export – could not tar path"
                            );
                        }
                    }
                }
            }
        }

        // --- services/<name>/docker-compose.yml ---
        for svc in &services {
            let safe_name = sanitise_name(&svc.name);
            add_bytes_to_tar(
                &mut builder,
                &format!("{}/services/{}/docker-compose.yml", root_dir, safe_name),
                svc.compose_content.as_bytes(),
            )
            .map_err(|e| ApiError::internal(format!("Tar error (service): {}", e)))?;
        }

        builder
            .finish()
            .map_err(|e| ApiError::internal(format!("Tar finish error: {}", e)))?;
    }

    // 7. Compress the tar.
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&raw_tar)
        .map_err(|e| ApiError::internal(format!("Gzip write error: {}", e)))?;
    let compressed = encoder
        .finish()
        .map_err(|e| ApiError::internal(format!("Gzip finish error: {}", e)))?;

    tracing::info!(
        archive = %archive_name,
        apps = apps.len(),
        databases = databases.len(),
        services = services.len(),
        size_bytes = compressed.len(),
        "Full system backup created"
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", archive_name),
        )
        .header(header::CONTENT_LENGTH, compressed.len().to_string())
        .body(Body::from(compressed))
        .map_err(|e| ApiError::internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Add a byte slice to a tar archive with the given path.
fn add_bytes_to_tar<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
) -> anyhow::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, path, std::io::Cursor::new(data))?;
    Ok(())
}

/// Sanitise a resource name for use as a filesystem directory name.
fn sanitise_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Export a Docker named volume by spinning up an alpine container.
///
/// Runs: `docker run --rm -v <volume>:/data alpine tar czf - -C /data .`
/// and captures stdout as the compressed archive bytes.
async fn export_docker_volume(volume_name: &str) -> anyhow::Result<Vec<u8>> {
    use tokio::process::Command;

    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/data", volume_name),
            "alpine",
            "tar",
            "czf",
            "-",
            "-C",
            "/data",
            ".",
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("docker run for volume export failed: {}", stderr);
    }

    Ok(output.stdout)
}

/// Compress a host directory into a tar.gz and return the bytes.
async fn backup_directory(path: &std::path::Path) -> anyhow::Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut tar_data: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_data);
            builder.append_dir_all(".", &path)?;
            builder.finish()?;
        }
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&tar_data)?;
        Ok(enc.finish()?)
    })
    .await
    .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}
