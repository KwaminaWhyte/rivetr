//! Backup catalog, restore and volume backup endpoints.
//!
//! - `GET  /databases/{id}/backups/catalog` local and S3 backups of a database
//! - `POST /databases/{id}/backups/{backup_id}/restore` restore (local or S3)
//! - `POST /databases/{id}/backups/{backup_id}/upload-s3` copy a local backup to S3
//! - `GET  /databases/{id}/backups/restores` recent restore jobs
//! - `GET|POST /volumes/{id}/backups` history (local + S3) / start a backup
//! - `DELETE /volumes/{id}/backups/{backup_id}` delete a backup
//! - `POST /volumes/{id}/backups/{backup_id}/restore` restore (local or S3)
//! - `GET  /volumes/{id}/backups/restores` recent restore jobs
//! - `GET|PUT|DELETE /volumes/{id}/backup-schedule` per-volume schedule
//!
//! Resource access is checked by the path authz middleware (`/databases/{id}`,
//! `/volumes/{id}`); S3 storage configs named in a body are checked here.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::authz;
use crate::api::error::ApiError;
use crate::db::{
    format_bytes, DatabaseBackup, ManagedDatabase, S3Backup, S3BackupResponse, S3StorageConfig,
    User,
};
use crate::engine::backup_jobs::restore::{self, RestoreError, RestoreJob, RestoreSource};
use crate::engine::backup_jobs::volume::{
    self, Destination, SnapshotMode, VolumeBackup, VolumeBackupSchedule,
};
use crate::engine::backup_jobs::{database as db_jobs, s3_store, schedule, JobContext};
use crate::AppState;

/// Map a restore start error to an API error.
pub fn restore_error(e: RestoreError) -> ApiError {
    match e {
        RestoreError::NotFound(m) => ApiError::not_found(m),
        RestoreError::BadRequest(m) => ApiError::bad_request(m),
        RestoreError::Conflict(m) => ApiError::conflict(m),
        RestoreError::Internal(e) => ApiError::internal(format!("{e:#}")),
    }
}

/// One backup in a catalog, local or in S3.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    /// Local backup id or `s3_backups` id, depending on `source`
    pub id: String,
    /// `local` or `s3`
    pub source: String,
    /// manual, scheduled, pre_deploy, pre_restore, ... (unknown for imported S3 objects)
    pub trigger: Option<String>,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub size_human: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
    pub error_message: Option<String>,
    pub file_name: Option<String>,
    pub storage_config_id: Option<String>,
    pub storage_config_name: Option<String>,
    pub s3_key: Option<String>,
    pub checksum_sha256: Option<String>,
    /// Snapshot mode for volume backups (live or stop)
    pub snapshot_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CatalogResponse {
    pub entries: Vec<CatalogEntry>,
    /// S3 objects found in buckets and added to the catalog by `scan=true`
    pub imported: usize,
}

#[derive(Debug, Deserialize, Default)]
pub struct CatalogQuery {
    /// List the S3 buckets for objects the catalog does not know yet
    #[serde(default)]
    pub scan: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct RestoreRequest {
    /// `local` (default) or `s3`
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadToS3Request {
    pub storage_config_id: String,
}

fn file_name_of(path: &str) -> Option<String> {
    path.rsplit('/').next().map(str::to_string)
}

fn sort_newest_first(entries: &mut [CatalogEntry]) {
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));
}

async fn config_names(state: &AppState) -> Result<Vec<(String, String)>, ApiError> {
    Ok(sqlx::query_as("SELECT id, name FROM s3_storage_configs")
        .fetch_all(&state.db)
        .await?)
}

fn s3_entry(b: &S3Backup, names: &[(String, String)], trigger: Option<String>) -> CatalogEntry {
    CatalogEntry {
        id: b.id.clone(),
        source: "s3".into(),
        trigger,
        status: b.status.clone(),
        size_bytes: b.size_bytes,
        size_human: b.size_bytes.map(format_bytes),
        duration_ms: None,
        created_at: b.created_at.clone(),
        error_message: b.error_message.clone(),
        file_name: file_name_of(&b.s3_key),
        storage_config_id: Some(b.storage_config_id.clone()),
        storage_config_name: names
            .iter()
            .find(|(id, _)| id == &b.storage_config_id)
            .map(|(_, n)| n.clone()),
        s3_key: Some(b.s3_key.clone()),
        checksum_sha256: b.checksum_sha256.clone(),
        snapshot_mode: None,
    }
}

/// Storage configs the caller can see, for bucket scans.
async fn visible_configs(
    state: &Arc<AppState>,
    user: &User,
) -> Result<Vec<S3StorageConfig>, ApiError> {
    let configs: Vec<S3StorageConfig> = sqlx::query_as("SELECT * FROM s3_storage_configs")
        .fetch_all(&state.db)
        .await?;
    authz::retain_visible(state, user, configs, |c| c.team_id.as_deref()).await
}

/// Load a storage config named in a request and check the caller may use it.
async fn usable_config(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<S3StorageConfig, ApiError> {
    let config: S3StorageConfig = sqlx::query_as("SELECT * FROM s3_storage_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("S3 storage config not found"))?;
    authz::authorize_team_assignment(state, user, config.team_id.as_deref()).await?;
    Ok(config)
}

async fn s3_rows(state: &AppState, kind: &str, source_id: &str) -> Result<Vec<S3Backup>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT * FROM s3_backups WHERE backup_type = ? AND source_id = ? \
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(kind)
    .bind(source_id)
    .fetch_all(&state.db)
    .await?)
}

// ---------------------------------------------------------------------------
// Databases
// ---------------------------------------------------------------------------

/// Local and S3 backups of a database, newest first.
pub async fn database_catalog(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(database_id): Path<String>,
    Query(query): Query<CatalogQuery>,
) -> Result<Json<CatalogResponse>, ApiError> {
    let _db: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    let imported = if query.scan {
        let configs = visible_configs(&state, &user).await?;
        s3_store::import_bucket_backups(
            &state.db,
            state.config.auth.encryption_key.as_deref(),
            &configs,
            "database",
            Some(&database_id),
        )
        .await
    } else {
        0
    };

    let local: Vec<DatabaseBackup> = sqlx::query_as(
        "SELECT id, database_id, backup_type, status, file_path, file_size, backup_format, \
         started_at, completed_at, error_message, created_at, updated_at \
         FROM database_backups WHERE database_id = ? ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&database_id)
    .fetch_all(&state.db)
    .await?;

    let mut entries: Vec<CatalogEntry> = local
        .iter()
        .map(|b| CatalogEntry {
            id: b.id.clone(),
            source: "local".into(),
            trigger: Some(b.backup_type.clone()),
            status: b.status.clone(),
            size_bytes: b.file_size,
            size_human: b.file_size.map(format_bytes),
            duration_ms: db_jobs::backup_duration(b).map(|d| d.as_millis() as i64),
            created_at: b.created_at.clone(),
            error_message: b.error_message.clone(),
            file_name: b.file_path.as_deref().and_then(file_name_of),
            storage_config_id: None,
            storage_config_name: None,
            s3_key: None,
            checksum_sha256: None,
            snapshot_mode: None,
        })
        .collect();

    let names = config_names(&state).await?;
    let s3 = s3_rows(&state, "database", &database_id).await?;
    let s3 = authz::retain_visible(&state, &user, s3, |b| b.team_id.as_deref()).await?;
    entries.extend(s3.iter().map(|b| s3_entry(b, &names, None)));
    sort_newest_first(&mut entries);

    Ok(Json(CatalogResponse { entries, imported }))
}

/// Restore a database from a local or S3 backup (background job).
pub async fn restore_database_backup(
    State(state): State<Arc<AppState>>,
    Path((database_id, backup_id)): Path<(String, String)>,
    body: Option<Json<RestoreRequest>>,
) -> Result<(StatusCode, Json<RestoreJob>), ApiError> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let source =
        RestoreSource::parse(req.source.as_deref().unwrap_or("local")).map_err(restore_error)?;
    let job = restore::start_database_restore(
        &JobContext::from_state(&state),
        &database_id,
        source,
        &backup_id,
    )
    .await
    .map_err(restore_error)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

/// Copy a completed local database backup to S3.
pub async fn upload_database_backup(
    State(state): State<Arc<AppState>>,
    user: User,
    Path((database_id, backup_id)): Path<(String, String)>,
    Json(req): Json<UploadToS3Request>,
) -> Result<Json<S3BackupResponse>, ApiError> {
    let database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;
    let backup: DatabaseBackup = sqlx::query_as(
        "SELECT id, database_id, backup_type, status, file_path, file_size, backup_format, \
         started_at, completed_at, error_message, created_at, updated_at \
         FROM database_backups WHERE id = ? AND database_id = ?",
    )
    .bind(&backup_id)
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Backup not found"))?;
    let config = usable_config(&state, &user, &req.storage_config_id).await?;

    let row = db_jobs::upload_database_backup(
        &JobContext::from_state(&state),
        &database,
        &backup,
        &config,
    )
    .await
    .map_err(|e| ApiError::internal(format!("Upload failed: {e:#}")))?;
    Ok(Json(row.to_response(Some(config.name))))
}

/// Recent restore jobs of a database.
pub async fn database_restores(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
) -> Result<Json<Vec<RestoreJob>>, ApiError> {
    restore::list_jobs(&state.db, "database", &database_id, 20)
        .await
        .map(Json)
        .map_err(restore_error)
}

// ---------------------------------------------------------------------------
// Volumes
// ---------------------------------------------------------------------------

/// Backup history of a volume: local archives, S3 uploads and failed runs.
pub async fn volume_catalog(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(volume_id): Path<String>,
    Query(query): Query<CatalogQuery>,
) -> Result<Json<CatalogResponse>, ApiError> {
    volume::get_volume(&state.db, &volume_id)
        .await
        .map_err(|_| ApiError::not_found("Volume not found"))?;

    let imported = if query.scan {
        let configs = visible_configs(&state, &user).await?;
        s3_store::import_bucket_backups(
            &state.db,
            state.config.auth.encryption_key.as_deref(),
            &configs,
            "volume",
            Some(&volume_id),
        )
        .await
    } else {
        0
    };

    let history: Vec<VolumeBackup> = sqlx::query_as(
        "SELECT * FROM volume_backups WHERE volume_id = ? ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&volume_id)
    .fetch_all(&state.db)
    .await?;
    let names = config_names(&state).await?;
    let s3 = s3_rows(&state, "volume", &volume_id).await?;
    let s3 = authz::retain_visible(&state, &user, s3, |b| b.team_id.as_deref()).await?;

    let mut entries: Vec<CatalogEntry> = Vec::new();
    for b in &history {
        // Completed S3 runs are listed through their S3 row below.
        if b.status == "completed" && b.s3_backup_id.is_some() {
            continue;
        }
        entries.push(CatalogEntry {
            id: b.id.clone(),
            source: if b.destination == "s3" { "s3" } else { "local" }.into(),
            trigger: Some(b.trigger.clone()),
            status: b.status.clone(),
            size_bytes: b.size_bytes,
            size_human: b.size_bytes.map(format_bytes),
            duration_ms: b.duration_ms,
            created_at: b.created_at.clone(),
            error_message: b.error_message.clone(),
            file_name: b.file_path.as_deref().and_then(file_name_of),
            storage_config_id: None,
            storage_config_name: None,
            s3_key: None,
            checksum_sha256: b.checksum_sha256.clone(),
            snapshot_mode: Some(b.snapshot_mode.clone()),
        });
    }
    for b in &s3 {
        let run = history
            .iter()
            .find(|h| h.s3_backup_id.as_deref() == Some(b.id.as_str()));
        let mut entry = s3_entry(b, &names, run.map(|r| r.trigger.clone()));
        if let Some(run) = run {
            entry.duration_ms = run.duration_ms;
            entry.snapshot_mode = Some(run.snapshot_mode.clone());
        }
        entries.push(entry);
    }
    sort_newest_first(&mut entries);
    Ok(Json(CatalogResponse { entries, imported }))
}

#[derive(Debug, Deserialize)]
pub struct CreateVolumeBackupRequest {
    /// `local` (default) or `s3`
    #[serde(default)]
    pub destination: Option<String>,
    pub s3_config_id: Option<String>,
    /// `live` (default) or `stop`
    #[serde(default)]
    pub snapshot_mode: Option<String>,
}

/// Start a volume backup in the background; returns the pending history row.
pub async fn create_volume_backup(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(volume_id): Path<String>,
    body: Option<Json<CreateVolumeBackupRequest>>,
) -> Result<(StatusCode, Json<VolumeBackup>), ApiError> {
    let vol = volume::get_volume(&state.db, &volume_id)
        .await
        .map_err(|_| ApiError::not_found("Volume not found"))?;
    let req = body.map(|Json(b)| b).unwrap_or(CreateVolumeBackupRequest {
        destination: None,
        s3_config_id: None,
        snapshot_mode: None,
    });
    let mode = SnapshotMode::parse(req.snapshot_mode.as_deref().unwrap_or("live"))
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let destination = match req.destination.as_deref().unwrap_or("local") {
        "local" => Destination::Local,
        "s3" => {
            let id = req
                .s3_config_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| ApiError::bad_request("s3_config_id is required for S3"))?;
            Destination::S3(Box::new(usable_config(&state, &user, id).await?))
        }
        other => {
            return Err(ApiError::bad_request(format!(
                "destination must be 'local' or 's3', got '{other}'"
            )))
        }
    };
    if crate::engine::backup_jobs::maintenance::is_locked(
        &crate::engine::backup_jobs::maintenance::resource_key("volume", &volume_id),
    ) {
        return Err(ApiError::conflict(
            "Another backup or restore of this volume is running",
        ));
    }

    let record = volume::create_pending(&state.db, &vol.id, "manual", &destination, mode)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let ctx = JobContext::from_state(&state);
    let job_record = record.clone();
    tokio::spawn(async move {
        let _ = volume::run_backup_job(&ctx, &vol, job_record, destination, mode).await;
    });
    Ok((StatusCode::ACCEPTED, Json(record)))
}

#[derive(Debug, Deserialize, Default)]
pub struct DeleteVolumeBackupQuery {
    /// `local` (a history row, default) or `s3` (an `s3_backups` row)
    pub source: Option<String>,
}

/// Delete a volume backup (local archive or S3 object) and its record.
pub async fn delete_volume_backup(
    State(state): State<Arc<AppState>>,
    Path((volume_id, backup_id)): Path<(String, String)>,
    Query(query): Query<DeleteVolumeBackupQuery>,
) -> Result<StatusCode, ApiError> {
    let ctx = JobContext::from_state(&state);
    match query.source.as_deref().unwrap_or("local") {
        "s3" => {
            let row: S3Backup = sqlx::query_as(
                "SELECT * FROM s3_backups WHERE id = ? AND backup_type = 'volume' AND source_id = ?",
            )
            .bind(&backup_id)
            .bind(&volume_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Backup not found"))?;
            volume::delete_s3_backup(&ctx, &row.id).await;
            let _ = sqlx::query("DELETE FROM volume_backups WHERE s3_backup_id = ?")
                .bind(&row.id)
                .execute(&state.db)
                .await;
        }
        _ => {
            let backup = volume::get_backup(&state.db, &backup_id)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
                .filter(|b| b.volume_id == volume_id)
                .ok_or_else(|| ApiError::not_found("Backup not found"))?;
            if matches!(backup.status.as_str(), "pending" | "running") {
                return Err(ApiError::conflict("Backup is still running"));
            }
            volume::delete_backup(&ctx, &backup)
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Restore a volume from a local or S3 backup (background job).
pub async fn restore_volume_backup(
    State(state): State<Arc<AppState>>,
    Path((volume_id, backup_id)): Path<(String, String)>,
    body: Option<Json<RestoreRequest>>,
) -> Result<(StatusCode, Json<RestoreJob>), ApiError> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let source =
        RestoreSource::parse(req.source.as_deref().unwrap_or("local")).map_err(restore_error)?;
    let job = restore::start_volume_restore(
        &JobContext::from_state(&state),
        &volume_id,
        source,
        &backup_id,
    )
    .await
    .map_err(restore_error)?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

/// Recent restore jobs of a volume.
pub async fn volume_restores(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<Json<Vec<RestoreJob>>, ApiError> {
    restore::list_jobs(&state.db, "volume", &volume_id, 20)
        .await
        .map(Json)
        .map_err(restore_error)
}

/// The volume's backup schedule, if any.
pub async fn get_volume_schedule(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<Json<Option<VolumeBackupSchedule>>, ApiError> {
    Ok(Json(
        sqlx::query_as("SELECT * FROM volume_backup_schedules WHERE volume_id = ?")
            .bind(&volume_id)
            .fetch_optional(&state.db)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct UpsertVolumeScheduleRequest {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Standard 5-field cron, e.g. `0 3 * * *` (UTC)
    pub cron_expression: String,
    #[serde(default = "default_retention")]
    pub retention_count: i32,
    /// `local` or `s3`
    #[serde(default = "default_local")]
    pub destination: String,
    pub s3_config_id: Option<String>,
    /// `live` or `stop`
    #[serde(default = "default_live")]
    pub snapshot_mode: String,
}

fn default_true() -> bool {
    true
}
fn default_retention() -> i32 {
    7
}
fn default_local() -> String {
    "local".into()
}
fn default_live() -> String {
    "live".into()
}

/// Create or replace the volume's backup schedule.
pub async fn upsert_volume_schedule(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(volume_id): Path<String>,
    Json(req): Json<UpsertVolumeScheduleRequest>,
) -> Result<Json<VolumeBackupSchedule>, ApiError> {
    volume::get_volume(&state.db, &volume_id)
        .await
        .map_err(|_| ApiError::not_found("Volume not found"))?;
    let next = schedule::next_run_after(&req.cron_expression, chrono::Utc::now())
        .map_err(|e| ApiError::validation_field("cron_expression", e.to_string()))?;
    if !(1..=365).contains(&req.retention_count) {
        return Err(ApiError::validation_field(
            "retention_count",
            "retention_count must be between 1 and 365",
        ));
    }
    SnapshotMode::parse(&req.snapshot_mode)
        .map_err(|e| ApiError::validation_field("snapshot_mode", e.to_string()))?;
    let s3_config_id = match req.destination.as_str() {
        "local" => None,
        "s3" => {
            let id = req
                .s3_config_id
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    ApiError::validation_field("s3_config_id", "Pick an S3 storage config")
                })?;
            Some(usable_config(&state, &user, id).await?.id)
        }
        _ => {
            return Err(ApiError::validation_field(
                "destination",
                "destination must be 'local' or 's3'",
            ))
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO volume_backup_schedules (id, volume_id, enabled, cron_expression, retention_count, \
         destination, s3_config_id, snapshot_mode, next_run_at, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(volume_id) DO UPDATE SET enabled = excluded.enabled, \
         cron_expression = excluded.cron_expression, retention_count = excluded.retention_count, \
         destination = excluded.destination, s3_config_id = excluded.s3_config_id, \
         snapshot_mode = excluded.snapshot_mode, next_run_at = excluded.next_run_at, \
         updated_at = excluded.updated_at",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&volume_id)
    .bind(if req.enabled { 1 } else { 0 })
    .bind(req.cron_expression.trim())
    .bind(req.retention_count)
    .bind(&req.destination)
    .bind(&s3_config_id)
    .bind(&req.snapshot_mode)
    .bind(next.to_rfc3339())
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let saved: VolumeBackupSchedule =
        sqlx::query_as("SELECT * FROM volume_backup_schedules WHERE volume_id = ?")
            .bind(&volume_id)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(saved))
}

/// Remove the volume's backup schedule (existing backups are kept).
pub async fn delete_volume_schedule(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM volume_backup_schedules WHERE volume_id = ?")
        .bind(&volume_id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
