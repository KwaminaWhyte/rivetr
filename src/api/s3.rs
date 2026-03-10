//! S3 backup storage API endpoints.
//!
//! Provides CRUD for S3 storage configurations and backup management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::backup::s3::S3Client;
use crate::crypto;
use crate::db::{
    CreateS3StorageConfigRequest, S3Backup, S3BackupResponse, S3StorageConfig,
    S3StorageConfigResponse, TriggerS3BackupRequest, UpdateS3StorageConfigRequest,
};
use crate::AppState;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// Build an S3Client from a stored config, decrypting credentials
fn build_s3_client(config: &S3StorageConfig, state: &AppState) -> Result<S3Client, StatusCode> {
    let encryption_key = get_encryption_key(state);

    let access_key =
        crypto::decrypt_if_encrypted(&config.access_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to decrypt S3 access key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let secret_key =
        crypto::decrypt_if_encrypted(&config.secret_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to decrypt S3 secret key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    S3Client::new(
        config.endpoint.as_deref(),
        &config.bucket,
        &config.region,
        &access_key,
        &secret_key,
        config.path_prefix.as_deref(),
    )
    .map_err(|e| {
        tracing::error!("Failed to create S3 client: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub reveal: bool,
    pub team_id: Option<String>,
}

// ---------------------------------------------------------------------------
// S3 Storage Config CRUD
// ---------------------------------------------------------------------------

/// Create a new S3 storage configuration
pub async fn create_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateS3StorageConfigRequest>,
) -> Result<Json<S3StorageConfigResponse>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let encryption_key = get_encryption_key(&state);

    // Encrypt access key and secret key before storing
    let encrypted_access_key =
        crypto::encrypt_if_key_available(&req.access_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt access key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let encrypted_secret_key =
        crypto::encrypt_if_key_available(&req.secret_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt secret key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let is_default: i32 = if req.is_default { 1 } else { 0 };
    let path_prefix = req.path_prefix.unwrap_or_default();

    // If setting as default, unset any existing default for the same team
    if req.is_default {
        if let Some(ref team_id) = req.team_id {
            sqlx::query("UPDATE s3_storage_configs SET is_default = 0 WHERE team_id = ?")
                .bind(team_id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to unset existing default: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        } else {
            sqlx::query("UPDATE s3_storage_configs SET is_default = 0 WHERE team_id IS NULL")
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to unset existing default: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    sqlx::query(
        "INSERT INTO s3_storage_configs (id, name, endpoint, bucket, region, access_key, secret_key, path_prefix, is_default, team_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.endpoint)
    .bind(&req.bucket)
    .bind(&req.region)
    .bind(&encrypted_access_key)
    .bind(&encrypted_secret_key)
    .bind(&path_prefix)
    .bind(is_default)
    .bind(&req.team_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create S3 storage config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let config = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch created S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(config.to_response(false)))
}

/// List S3 storage configurations
pub async fn list_configs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<S3StorageConfigResponse>>, StatusCode> {
    let configs = if let Some(team_id) = &query.team_id {
        sqlx::query_as::<_, S3StorageConfig>(
            "SELECT * FROM s3_storage_configs WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, S3StorageConfig>(
            "SELECT * FROM s3_storage_configs ORDER BY created_at DESC",
        )
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list S3 configs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<S3StorageConfigResponse> = configs
        .iter()
        .map(|c| c.to_response(query.reveal))
        .collect();

    Ok(Json(responses))
}

/// Update an S3 storage configuration
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateS3StorageConfigRequest>,
) -> Result<Json<S3StorageConfigResponse>, StatusCode> {
    // Check it exists
    let existing = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let existing = existing.ok_or(StatusCode::NOT_FOUND)?;
    let encryption_key = get_encryption_key(&state);

    let name = req.name.unwrap_or(existing.name);
    let endpoint = req.endpoint.or(existing.endpoint);
    let bucket = req.bucket.unwrap_or(existing.bucket);
    let region = req.region.unwrap_or(existing.region);
    let path_prefix = req.path_prefix.or(existing.path_prefix);

    let access_key = if let Some(ref new_key) = req.access_key {
        crypto::encrypt_if_key_available(new_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt access key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        existing.access_key
    };

    let secret_key = if let Some(ref new_key) = req.secret_key {
        crypto::encrypt_if_key_available(new_key, encryption_key.as_ref()).map_err(|e| {
            tracing::error!("Failed to encrypt secret key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        existing.secret_key
    };

    let is_default = req
        .is_default
        .map(|v| if v { 1 } else { 0 })
        .unwrap_or(existing.is_default);

    // If setting as default, unset any existing default for the same team
    if is_default == 1 && existing.is_default == 0 {
        if let Some(ref team_id) = existing.team_id {
            sqlx::query(
                "UPDATE s3_storage_configs SET is_default = 0 WHERE team_id = ? AND id != ?",
            )
            .bind(team_id)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to unset existing default: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        } else {
            sqlx::query(
                "UPDATE s3_storage_configs SET is_default = 0 WHERE team_id IS NULL AND id != ?",
            )
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to unset existing default: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
    }

    sqlx::query(
        "UPDATE s3_storage_configs SET name = ?, endpoint = ?, bucket = ?, region = ?, access_key = ?, secret_key = ?, path_prefix = ?, is_default = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&name)
    .bind(&endpoint)
    .bind(&bucket)
    .bind(&region)
    .bind(&access_key)
    .bind(&secret_key)
    .bind(&path_prefix)
    .bind(is_default)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let updated = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch updated S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(updated.to_response(false)))
}

/// Delete an S3 storage configuration
pub async fn delete_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check for existing backups using this config
    let backup_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM s3_backups WHERE storage_config_id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to count backups: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if backup_count > 0 {
        return Err(StatusCode::CONFLICT);
    }

    let result = sqlx::query("DELETE FROM s3_storage_configs WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete S3 config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(serde_json::json!({ "message": "S3 storage config deleted" })))
}

/// Test an S3 storage configuration's connection
pub async fn test_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let client = build_s3_client(&config, &state)?;

    match client.test_connection().await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Connection successful"
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "success": false,
            "message": format!("Connection failed: {}", e)
        }))),
    }
}

// ---------------------------------------------------------------------------
// S3 Backup Operations
// ---------------------------------------------------------------------------

/// Trigger a backup to S3
pub async fn trigger_backup(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TriggerS3BackupRequest>,
) -> Result<Json<S3BackupResponse>, StatusCode> {
    // Validate backup type
    if !["instance", "database", "volume"].contains(&req.backup_type.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Fetch the storage config
    let config = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&req.storage_config_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let backup_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let s3_key = match req.backup_type.as_str() {
        "instance" => format!("backups/instance/rivetr-backup-{}.tar.gz", timestamp),
        "database" => {
            let source = req.source_id.as_deref().unwrap_or("unknown");
            format!("backups/database/{}/backup-{}.sql.gz", source, timestamp)
        }
        "volume" => {
            let source = req.source_id.as_deref().unwrap_or("unknown");
            format!("backups/volume/{}/backup-{}.tar.gz", source, timestamp)
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Create backup record as pending
    sqlx::query(
        "INSERT INTO s3_backups (id, storage_config_id, backup_type, source_id, s3_key, status, team_id)
         VALUES (?, ?, ?, ?, ?, 'pending', ?)",
    )
    .bind(&backup_id)
    .bind(&req.storage_config_id)
    .bind(&req.backup_type)
    .bind(&req.source_id)
    .bind(&s3_key)
    .bind(&config.team_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create S3 backup record: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Clone what we need for the background task
    let config_name = config.name.clone();
    let db = state.db.clone();
    let state_clone = state.clone();
    let backup_id_clone = backup_id.clone();
    let s3_key_clone = s3_key.clone();
    let backup_type = req.backup_type.clone();

    // Perform the backup in a background task
    tokio::spawn(async move {
        // Update status to uploading
        let _ = sqlx::query("UPDATE s3_backups SET status = 'uploading' WHERE id = ?")
            .bind(&backup_id_clone)
            .execute(&db)
            .await;

        let result = async {
            let client = build_s3_client(&config, &state_clone)?;

            // Get backup data based on type
            let data = match backup_type.as_str() {
                "instance" => {
                    // Create an instance backup
                    let data_dir = &state_clone.config.server.data_dir;
                    let config_path = data_dir.join("../rivetr.toml");
                    let acme_cache_dir = &state_clone.config.proxy.acme_cache_dir;

                    let backup_result = crate::backup::create_backup(
                        &db,
                        data_dir,
                        &config_path,
                        acme_cache_dir,
                        None,
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    // Read the backup file
                    std::fs::read(&backup_result.path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                }
                _ => {
                    // For database/volume types, we read from the local backup directory
                    // A source_id-based lookup would be needed here
                    tracing::warn!("Database/volume S3 backup not yet fully integrated");
                    return Err(StatusCode::NOT_IMPLEMENTED);
                }
            };

            let size = data.len() as i64;

            // Upload to S3
            client
                .upload_backup(&s3_key_clone, data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Update record with success
            let _ = sqlx::query(
                "UPDATE s3_backups SET status = 'completed', size_bytes = ? WHERE id = ?",
            )
            .bind(size)
            .bind(&backup_id_clone)
            .execute(&db)
            .await;

            Ok::<(), StatusCode>(())
        }
        .await;

        if result.is_err() {
            let _ = sqlx::query(
                "UPDATE s3_backups SET status = 'failed', error_message = 'Backup upload failed' WHERE id = ?",
            )
            .bind(&backup_id_clone)
            .execute(&db)
            .await;
        }
    });

    // Return the pending backup record
    let backup =
        sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
            .bind(&backup_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch backup record: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    Ok(Json(backup.to_response(Some(config_name))))
}

/// List S3 backups
pub async fn list_backups(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<S3BackupResponse>>, StatusCode> {
    let backups = if let Some(team_id) = &query.team_id {
        sqlx::query_as::<_, S3Backup>(
            "SELECT * FROM s3_backups WHERE team_id = ? ORDER BY created_at DESC",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
    }
    .map_err(|e| {
        tracing::error!("Failed to list S3 backups: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Fetch config names for the response
    let mut responses = Vec::with_capacity(backups.len());
    for backup in &backups {
        let config_name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM s3_storage_configs WHERE id = ?",
        )
        .bind(&backup.storage_config_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch config name: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        responses.push(backup.to_response(config_name));
    }

    Ok(Json(responses))
}

/// Restore from an S3 backup
pub async fn restore_backup(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let backup = sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch S3 backup: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if backup.status != "completed" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Get the storage config
    let config = sqlx::query_as::<_, S3StorageConfig>(
        "SELECT * FROM s3_storage_configs WHERE id = ?",
    )
    .bind(&backup.storage_config_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch S3 config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let client = build_s3_client(&config, &state)?;

    // Download the backup from S3
    let data = client.download_backup(&backup.s3_key).await.map_err(|e| {
        tracing::error!("Failed to download backup from S3: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Restore based on type
    match backup.backup_type.as_str() {
        "instance" => {
            let data_dir = &state.config.server.data_dir;
            let config_path = data_dir.join("../rivetr.toml");
            let acme_cache_dir = &state.config.proxy.acme_cache_dir;

            let result = crate::backup::restore_from_backup(
                &data,
                data_dir,
                &config_path,
                acme_cache_dir,
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to restore from S3 backup: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            Ok(Json(serde_json::json!({
                "message": "Restore completed. Server restart recommended.",
                "database_restored": result.database_restored,
                "config_restored": result.config_restored,
                "certs_restored": result.certs_restored,
                "warnings": result.warnings
            })))
        }
        _ => {
            tracing::warn!("Database/volume S3 restore not yet fully integrated");
            Err(StatusCode::NOT_IMPLEMENTED)
        }
    }
}

/// Delete an S3 backup (from both S3 and the database record)
pub async fn delete_backup(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let backup = sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch S3 backup: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Try to delete from S3 if the backup was completed
    if backup.status == "completed" {
        let config = sqlx::query_as::<_, S3StorageConfig>(
            "SELECT * FROM s3_storage_configs WHERE id = ?",
        )
        .bind(&backup.storage_config_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch S3 config: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some(config) = config {
            if let Ok(client) = build_s3_client(&config, &state) {
                if let Err(e) = client.delete_backup(&backup.s3_key).await {
                    tracing::warn!("Failed to delete S3 object (continuing): {}", e);
                }
            }
        }
    }

    // Delete the database record
    sqlx::query("DELETE FROM s3_backups WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete S3 backup record: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(
        serde_json::json!({ "message": "S3 backup deleted" }),
    ))
}
