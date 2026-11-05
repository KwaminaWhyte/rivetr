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

use crate::api::authz;
use crate::api::error::ApiError;
use crate::backup::s3::S3Client;
use crate::crypto;
use crate::db::{
    CreateS3StorageConfigRequest, S3Backup, S3BackupResponse, S3StorageConfig,
    S3StorageConfigResponse, TriggerS3BackupRequest, UpdateS3StorageConfigRequest, User,
};
use crate::engine::backup_jobs::s3_store;
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

    let access_key = crypto::decrypt_if_encrypted(&config.access_key, encryption_key.as_ref())
        .map_err(|e| {
            tracing::error!("Failed to decrypt S3 access key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let secret_key = crypto::decrypt_if_encrypted(&config.secret_key, encryption_key.as_ref())
        .map_err(|e| {
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
    user: User,
    Json(req): Json<CreateS3StorageConfigRequest>,
) -> Result<Json<S3StorageConfigResponse>, StatusCode> {
    authz::authorize_team_assignment(&state, &user, req.team_id.as_deref())
        .await
        .map_err(|e| e.status())?;
    let id = Uuid::new_v4().to_string();
    let encryption_key = get_encryption_key(&state);

    // Encrypt access key and secret key before storing
    let encrypted_access_key =
        crypto::encrypt_if_key_available(&req.access_key, encryption_key.as_ref()).map_err(
            |e| {
                tracing::error!("Failed to encrypt access key: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            },
        )?;

    let encrypted_secret_key =
        crypto::encrypt_if_key_available(&req.secret_key, encryption_key.as_ref()).map_err(
            |e| {
                tracing::error!("Failed to encrypt secret key: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            },
        )?;

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

    let config =
        sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
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
    user: User,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<S3StorageConfigResponse>>, StatusCode> {
    authz::authorize_team_filter(&state, &user, query.team_id.as_deref())
        .await
        .map_err(|e| e.status())?;

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

    let configs = authz::retain_visible(&state, &user, configs, |c| c.team_id.as_deref())
        .await
        .map_err(|e| e.status())?;

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
    let existing =
        sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
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

    let updated =
        sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
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
    let backup_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM s3_backups WHERE storage_config_id = ?")
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

    Ok(Json(
        serde_json::json!({ "message": "S3 storage config deleted" }),
    ))
}

/// Test an S3 storage configuration's connection
pub async fn test_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config =
        sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
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
///
/// `instance` uploads a fresh instance backup, `database` dumps the database
/// (`source_id`) and uploads the dump, `volume` archives the volume (`source_id`)
/// straight to S3. The work runs in the background; the returned row is `pending`
/// and moves to `uploading`, then `completed` or `failed`.
pub async fn trigger_backup(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<TriggerS3BackupRequest>,
) -> Result<Json<S3BackupResponse>, ApiError> {
    use crate::engine::backup_jobs::{database as db_jobs, notify, volume, JobContext};

    if !["instance", "database", "volume"].contains(&req.backup_type.as_str()) {
        return Err(ApiError::bad_request(
            "backup_type must be instance, database or volume",
        ));
    }
    let source_id = req.source_id.clone().filter(|s| !s.is_empty());
    if req.backup_type != "instance" && source_id.is_none() {
        return Err(ApiError::bad_request(
            "source_id is required for database and volume backups",
        ));
    }

    let config =
        sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
            .bind(&req.storage_config_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("S3 storage config not found"))?;

    // SEC-C3: the caller must have access to the storage config they target.
    authz::authorize_team_assignment(&state, &user, config.team_id.as_deref()).await?;

    let ctx = JobContext::from_state(&state);
    let config_name = config.name.clone();

    if req.backup_type == "volume" {
        let volume_id = source_id.clone().unwrap_or_default();
        let vol = volume::get_volume(&state.db, &volume_id)
            .await
            .map_err(|_| ApiError::not_found("Volume not found"))?;
        authz::authorize_app_write(&state, &user, &vol.app_id).await?;
        let destination = volume::Destination::S3(Box::new(config.clone()));
        let record = volume::create_pending(
            &state.db,
            &vol.id,
            "manual",
            &destination,
            volume::SnapshotMode::Live,
        )
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
        // The volume job creates the s3_backups row once the archive exists; answer
        // with a pending placeholder that mirrors the request.
        let job_ctx = ctx.clone();
        tokio::spawn(async move {
            let _ = volume::run_backup_job(
                &job_ctx,
                &vol,
                record,
                destination,
                volume::SnapshotMode::Live,
            )
            .await;
        });
        return Ok(Json(S3BackupResponse {
            id: String::new(),
            storage_config_id: config.id.clone(),
            storage_config_name: Some(config_name),
            backup_type: "volume".into(),
            source_id,
            s3_key: s3_store::listing_prefix("volume", Some(&volume_id)),
            size_bytes: None,
            size_human: None,
            status: "pending".into(),
            error_message: None,
            team_id: config.team_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            checksum_sha256: None,
        }));
    }

    let database = if req.backup_type == "database" {
        let database_id = source_id.clone().unwrap_or_default();
        let database = authz::authorize_database(&state, &user, &database_id).await?;
        if database.status != "running" {
            return Err(ApiError::bad_request(
                "Database must be running to create a backup",
            ));
        }
        Some(database)
    } else {
        None
    };

    let backup_id = s3_store::insert_pending(
        &state.db,
        &config,
        &req.backup_type,
        source_id.as_deref(),
        &s3_store::listing_prefix(&req.backup_type, source_id.as_deref()),
    )
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?;

    let row_id = backup_id.clone();
    let backup_type = req.backup_type.clone();
    tokio::spawn(async move {
        match database {
            Some(database) => {
                let result = async {
                    let task = crate::engine::database_backups::DatabaseBackupTask::new(
                        ctx.db.clone(),
                        ctx.runtime.clone(),
                        ctx.db_backup.clone(),
                        ctx.data_dir.clone(),
                    );
                    let local = task
                        .backup_database(&database, crate::db::BackupType::Manual)
                        .await?;
                    let path = local
                        .file_path
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("Backup has no file on disk"))?;
                    s3_store::upload_into_record(
                        &ctx.db,
                        ctx.secret(),
                        &config,
                        &row_id,
                        "database",
                        Some(&database.id),
                        std::path::Path::new(&path),
                    )
                    .await?;
                    Ok::<_, anyhow::Error>(local)
                }
                .await;
                match result {
                    Ok(local) => db_jobs::notify_database_backup(
                        &ctx.db,
                        &database,
                        "manual",
                        Ok(&local),
                        db_jobs::destination_label(Some(&config)),
                    ),
                    Err(e) => {
                        s3_store::mark_failed(&ctx.db, &row_id, &format!("{e:#}")).await;
                        db_jobs::notify_database_backup(
                            &ctx.db,
                            &database,
                            "manual",
                            Err(format!("{e:#}")),
                            notify::s3_destination(&config.name),
                        );
                    }
                }
            }
            None if backup_type == "instance" => {
                let started = std::time::Instant::now();
                let result = async {
                    let backup = crate::backup::create_backup(
                        &ctx.db,
                        &ctx.data_dir,
                        &ctx.config_path,
                        &ctx.acme_cache_dir,
                        None,
                    )
                    .await?;
                    s3_store::upload_into_record(
                        &ctx.db,
                        ctx.secret(),
                        &config,
                        &row_id,
                        "instance",
                        None,
                        &backup.path,
                    )
                    .await
                }
                .await;
                let event = |outcome| {
                    notify::BackupEvent::new(
                        outcome,
                        notify::BackupResourceKind::Instance,
                        "Rivetr instance",
                    )
                    .trigger("manual")
                    .destination(notify::s3_destination(&config.name))
                };
                match result {
                    Ok(row) => notify::notify_backup(
                        &ctx.db,
                        event(notify::BackupOutcome::Succeeded)
                            .size(row.size_bytes)
                            .duration(Some(started.elapsed())),
                    ),
                    Err(e) => {
                        s3_store::mark_failed(&ctx.db, &row_id, &format!("{e:#}")).await;
                        notify::notify_backup(
                            &ctx.db,
                            event(notify::BackupOutcome::Failed).error(format!("{e:#}")),
                        );
                    }
                }
            }
            None => {}
        }
    });

    let backup = sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
        .bind(&backup_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(backup.to_response(Some(config_name))))
}

/// List S3 backups
pub async fn list_backups(
    State(state): State<Arc<AppState>>,
    user: User,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<S3BackupResponse>>, StatusCode> {
    authz::authorize_team_filter(&state, &user, query.team_id.as_deref())
        .await
        .map_err(|e| e.status())?;

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

    let backups = authz::retain_visible(&state, &user, backups, |b| b.team_id.as_deref())
        .await
        .map_err(|e| e.status())?;

    // Fetch config names for the response
    let mut responses = Vec::with_capacity(backups.len());
    for backup in &backups {
        let config_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM s3_storage_configs WHERE id = ?")
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
///
/// Database and volume backups start a background restore job (safety backup of
/// the current data first, then the same restore used for local backups) and
/// return it. Instance backups are downloaded, verified, preceded by a local safety
/// backup of the instance, and restored in the request.
pub async fn restore_backup(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use crate::engine::backup_jobs::{restore, JobContext};

    let backup = sqlx::query_as::<_, S3Backup>("SELECT * FROM s3_backups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("S3 backup not found"))?;

    if backup.status != "completed" {
        return Err(ApiError::bad_request(format!(
            "Backup is not complete (status: {})",
            backup.status
        )));
    }

    let ctx = JobContext::from_state(&state);
    match backup.backup_type.as_str() {
        "database" | "volume" => {
            let source_id = backup
                .source_id
                .clone()
                .ok_or_else(|| ApiError::bad_request("Backup has no source resource"))?;
            let job = if backup.backup_type == "database" {
                restore::start_database_restore(&ctx, &source_id, restore::RestoreSource::S3, &id)
                    .await
            } else {
                restore::start_volume_restore(&ctx, &source_id, restore::RestoreSource::S3, &id)
                    .await
            }
            .map_err(super::backup_jobs::restore_error)?;
            Ok(Json(serde_json::json!({
                "message": "Restore started",
                "job": job,
            })))
        }
        "instance" => {
            let tmp_dir = state.config.server.data_dir.join("backups");
            tokio::fs::create_dir_all(&tmp_dir)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let tmp = tmp_dir.join(format!(".restore-{}.tar.gz", Uuid::new_v4()));
            let result = async {
                s3_store::download_verified(
                    &state.db,
                    ctx.secret(),
                    &backup,
                    &tmp,
                    crate::engine::backup_jobs::volume::MAX_VOLUME_RESTORE_BYTES,
                )
                .await?;
                let data = tokio::fs::read(&tmp).await?;
                // Safety copy of the current instance before it is overwritten.
                let safety = crate::backup::create_backup(
                    &state.db,
                    &ctx.data_dir,
                    &ctx.config_path,
                    &ctx.acme_cache_dir,
                    None,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Safety backup failed, nothing was restored: {e:#}")
                })?;
                let result = crate::backup::restore_from_backup(
                    &data,
                    &ctx.data_dir,
                    &ctx.config_path,
                    &ctx.acme_cache_dir,
                )
                .await?;
                Ok::<_, anyhow::Error>((result, safety.name))
            }
            .await;
            let _ = tokio::fs::remove_file(&tmp).await;
            let (result, safety_name) = result.map_err(|e| {
                tracing::error!("Failed to restore from S3 backup: {e:#}");
                ApiError::internal(format!("Restore failed: {e:#}"))
            })?;

            Ok(Json(serde_json::json!({
                "message": "Restore completed. Server restart recommended.",
                "safety_backup": safety_name,
                "database_restored": result.database_restored,
                "config_restored": result.config_restored,
                "certs_restored": result.certs_restored,
                "warnings": result.warnings
            })))
        }
        other => Err(ApiError::bad_request(format!(
            "Unknown backup type '{other}'"
        ))),
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
        let config =
            sqlx::query_as::<_, S3StorageConfig>("SELECT * FROM s3_storage_configs WHERE id = ?")
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

    // Volume history rows that pointed at this upload go with it.
    let _ = sqlx::query("DELETE FROM volume_backups WHERE s3_backup_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;

    // Delete the database record
    sqlx::query("DELETE FROM s3_backups WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete S3 backup record: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({ "message": "S3 backup deleted" })))
}
