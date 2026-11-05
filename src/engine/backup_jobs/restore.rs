//! Restore jobs for databases and volumes, from a local backup or from S3.
//!
//! Everything that can be checked synchronously (backup exists, belongs to the
//! resource, is complete, the engine supports restore, nothing else is running on
//! the resource) is checked before the job is created, so the API can answer with a
//! clear 4xx. The restore itself runs in the background and is tracked in
//! `backup_restores`.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use super::maintenance::{self, resource_key, ResourceLock};
use super::volume::{self, VolumeBackup, MAX_VOLUME_RESTORE_BYTES};
use super::{database, s3_store, JobContext};
use crate::db::{DatabaseBackup, ManagedDatabase, S3Backup};
use crate::engine::deployment_snapshots::MAX_SNAPSHOT_BYTES;
use crate::DbPool;

/// A restore job (row of `backup_restores`).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RestoreJob {
    pub id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub source: String,
    pub backup_id: String,
    pub status: String,
    pub safety_backup_id: Option<String>,
    pub message: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

/// Why a restore could not start.
#[derive(Debug)]
pub enum RestoreError {
    NotFound(String),
    BadRequest(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::BadRequest(m) | Self::Conflict(m) => f.write_str(m),
            Self::Internal(e) => write!(f, "{e:#}"),
        }
    }
}

impl From<sqlx::Error> for RestoreError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.into())
    }
}

/// Where the backup to restore lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreSource {
    Local,
    S3,
}

impl RestoreSource {
    pub fn parse(s: &str) -> Result<Self, RestoreError> {
        match s {
            "local" => Ok(Self::Local),
            "s3" => Ok(Self::S3),
            other => Err(RestoreError::BadRequest(format!(
                "source must be 'local' or 's3', got '{other}'"
            ))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }
}

fn lock(kind: &str, id: &str) -> Result<ResourceLock, RestoreError> {
    maintenance::try_lock(&resource_key(kind, id)).ok_or_else(|| {
        RestoreError::Conflict(format!(
            "Another backup or restore of this {kind} is running; try again when it finishes"
        ))
    })
}

async fn insert_job(
    db: &DbPool,
    resource_type: &str,
    resource_id: &str,
    source: RestoreSource,
    backup_id: &str,
) -> Result<RestoreJob, RestoreError> {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO backup_restores (id, resource_type, resource_id, source, backup_id, status, started_at) \
         VALUES (?, ?, ?, ?, ?, 'running', ?)",
    )
    .bind(&id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(source.as_str())
    .bind(backup_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db)
    .await?;
    get_job(db, &id)
        .await?
        .ok_or_else(|| RestoreError::Internal(anyhow!("Restore job vanished")))
}

async fn finish_job(db: &DbPool, id: &str, result: anyhow::Result<(String, String)>) {
    let now = chrono::Utc::now().to_rfc3339();
    let update = match result {
        Ok((safety_id, message)) => sqlx::query(
            "UPDATE backup_restores SET status = 'completed', safety_backup_id = ?, message = ?, \
             completed_at = ? WHERE id = ?",
        )
        .bind(safety_id)
        .bind(message)
        .bind(&now)
        .bind(id),
        Err(e) => {
            tracing::error!(job = %id, error = %format!("{e:#}"), "Backup restore failed");
            sqlx::query(
                "UPDATE backup_restores SET status = 'failed', error_message = ?, completed_at = ? \
                 WHERE id = ?",
            )
            .bind(format!("{e:#}"))
            .bind(&now)
            .bind(id)
        }
    };
    if let Err(e) = update.execute(db).await {
        tracing::warn!(job = %id, error = %e, "Failed to record restore result");
    }
}

pub async fn get_job(db: &DbPool, id: &str) -> Result<Option<RestoreJob>, RestoreError> {
    Ok(
        sqlx::query_as::<_, RestoreJob>("SELECT * FROM backup_restores WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

/// Most recent restore jobs of a resource.
pub async fn list_jobs(
    db: &DbPool,
    resource_type: &str,
    resource_id: &str,
    limit: i64,
) -> Result<Vec<RestoreJob>, RestoreError> {
    Ok(sqlx::query_as::<_, RestoreJob>(
        "SELECT * FROM backup_restores WHERE resource_type = ? AND resource_id = ? \
         ORDER BY started_at DESC LIMIT ?",
    )
    .bind(resource_type)
    .bind(resource_id)
    .bind(limit)
    .fetch_all(db)
    .await?)
}

/// Mark jobs and backup runs left `running` by a previous process as failed.
pub async fn fail_interrupted(db: &DbPool) {
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "UPDATE backup_restores SET status = 'failed', error_message = 'Interrupted by a server restart', \
         completed_at = ? WHERE status = 'running'",
    )
    .bind(&now)
    .execute(db)
    .await;
    let _ = sqlx::query(
        "UPDATE volume_backups SET status = 'failed', error_message = 'Interrupted by a server restart', \
         completed_at = ? WHERE status IN ('pending', 'running')",
    )
    .bind(&now)
    .execute(db)
    .await;
}

/// An S3 backup row that belongs to `kind`/`source_id` and is complete.
async fn s3_backup_for(
    db: &DbPool,
    backup_id: &str,
    kind: &str,
    source_id: &str,
) -> Result<S3Backup, RestoreError> {
    let backup: S3Backup = sqlx::query_as("SELECT * FROM s3_backups WHERE id = ?")
        .bind(backup_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| RestoreError::NotFound("S3 backup not found".into()))?;
    if backup.backup_type != kind || backup.source_id.as_deref() != Some(source_id) {
        return Err(RestoreError::BadRequest(format!(
            "S3 backup {backup_id} does not belong to this {kind}"
        )));
    }
    if backup.status != "completed" {
        return Err(RestoreError::BadRequest(format!(
            "S3 backup is not complete (status: {})",
            backup.status
        )));
    }
    Ok(backup)
}

/// Temp file next to the resource's local backups for an S3 download.
fn temp_download_path(dir: &Path, s3_key: &str) -> PathBuf {
    let name = s3_key.rsplit('/').next().unwrap_or("backup");
    dir.join(format!(".restore-{}-{}", Uuid::new_v4(), name))
}

/// Start restoring a database from one of its backups.
pub async fn start_database_restore(
    ctx: &JobContext,
    database_id: &str,
    source: RestoreSource,
    backup_id: &str,
) -> Result<RestoreJob, RestoreError> {
    let database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(database_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or_else(|| RestoreError::NotFound("Database not found".into()))?;

    // Resolve the backup and run the cheap checks before anything starts.
    let (local, s3): (Option<PathBuf>, Option<S3Backup>) = match source {
        RestoreSource::Local => {
            let backup: DatabaseBackup = sqlx::query_as(
                "SELECT id, database_id, backup_type, status, file_path, file_size, backup_format, \
                 started_at, completed_at, error_message, created_at, updated_at \
                 FROM database_backups WHERE id = ? AND database_id = ?",
            )
            .bind(backup_id)
            .bind(database_id)
            .fetch_optional(&ctx.db)
            .await?
            .ok_or_else(|| RestoreError::NotFound("Backup not found".into()))?;
            if backup.status != "completed" {
                return Err(RestoreError::BadRequest("Backup is not completed".into()));
            }
            let path = backup
                .file_path
                .map(PathBuf::from)
                .filter(|p| p.exists())
                .ok_or_else(|| RestoreError::BadRequest("Backup file is missing on disk".into()))?;
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            database::check_restorable(&database, size)
                .map_err(|e| RestoreError::BadRequest(e.to_string()))?;
            (Some(path), None)
        }
        RestoreSource::S3 => {
            let backup = s3_backup_for(&ctx.db, backup_id, "database", database_id).await?;
            let size = backup.size_bytes.filter(|s| *s > 0).unwrap_or(1) as u64;
            database::check_restorable(&database, size)
                .map_err(|e| RestoreError::BadRequest(e.to_string()))?;
            (None, Some(backup))
        }
    };

    let guard = lock("database", database_id)?;
    let job = insert_job(&ctx.db, "database", database_id, source, backup_id).await?;
    let job_id = job.id.clone();
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let _guard = guard;
        let result = async {
            if let Some(path) = local {
                let safety = database::restore_database(&ctx, &database, &path).await?;
                return Ok((safety, format!("Database '{}' restored", database.name)));
            }
            let backup = s3.ok_or_else(|| anyhow!("No backup to restore"))?;
            let dir = ctx
                .data_dir
                .join(&ctx.db_backup.backup_dir)
                .join(&database.id);
            tokio::fs::create_dir_all(&dir).await?;
            let temp = temp_download_path(&dir, &backup.s3_key);
            let restored = async {
                s3_store::download_verified(
                    &ctx.db,
                    ctx.secret(),
                    &backup,
                    &temp,
                    MAX_SNAPSHOT_BYTES as u64,
                )
                .await?;
                database::restore_database(&ctx, &database, &temp).await
            }
            .await;
            let _ = tokio::fs::remove_file(&temp).await;
            let safety = restored?;
            Ok((
                safety,
                format!("Database '{}' restored from S3", database.name),
            ))
        }
        .await;
        finish_job(&ctx.db, &job_id, result).await;
    });
    Ok(job)
}

/// Start restoring a volume from one of its backups.
pub async fn start_volume_restore(
    ctx: &JobContext,
    volume_id: &str,
    source: RestoreSource,
    backup_id: &str,
) -> Result<RestoreJob, RestoreError> {
    let vol = volume::get_volume(&ctx.db, volume_id)
        .await
        .map_err(|_| RestoreError::NotFound("Volume not found".into()))?;
    let (local, s3): (Option<PathBuf>, Option<S3Backup>) = match source {
        RestoreSource::Local => {
            let backup: VolumeBackup = volume::get_backup(&ctx.db, backup_id)
                .await
                .map_err(RestoreError::Internal)?
                .filter(|b| b.volume_id == volume_id)
                .ok_or_else(|| RestoreError::NotFound("Backup not found".into()))?;
            if backup.status != "completed" {
                return Err(RestoreError::BadRequest("Backup is not completed".into()));
            }
            if let (None, Some(s3_id)) = (&backup.file_path, &backup.s3_backup_id) {
                // A history row for an S3 upload: restore from its S3 copy.
                let s3 = s3_backup_for(&ctx.db, s3_id, "volume", volume_id).await?;
                (None, Some(s3))
            } else {
                let path = backup
                    .file_path
                    .map(PathBuf::from)
                    .filter(|p| p.exists())
                    .ok_or_else(|| {
                        RestoreError::BadRequest("Backup file is missing on disk".into())
                    })?;
                (Some(path), None)
            }
        }
        RestoreSource::S3 => (
            None,
            Some(s3_backup_for(&ctx.db, backup_id, "volume", volume_id).await?),
        ),
    };

    let guard = lock("volume", volume_id)?;
    let job = insert_job(&ctx.db, "volume", volume_id, source, backup_id).await?;
    let job_id = job.id.clone();
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let _guard = guard;
        let result = async {
            let report = if let Some(path) = local {
                volume::restore_from_archive(&ctx, &vol, &path).await?
            } else {
                let backup = s3.ok_or_else(|| anyhow!("No backup to restore"))?;
                let dir = volume::volume_backup_dir(&ctx.data_dir, &vol.id);
                tokio::fs::create_dir_all(&dir).await?;
                let temp = temp_download_path(&dir, &backup.s3_key);
                let restored = async {
                    s3_store::download_verified(
                        &ctx.db,
                        ctx.secret(),
                        &backup,
                        &temp,
                        MAX_VOLUME_RESTORE_BYTES,
                    )
                    .await?;
                    volume::restore_from_archive(&ctx, &vol, &temp).await
                }
                .await;
                let _ = tokio::fs::remove_file(&temp).await;
                restored?
            };
            let mut message = format!(
                "Volume '{}' restored; {} container(s) restarted",
                vol.name, report.restarted_containers
            );
            if !report.failed_to_start.is_empty() {
                message.push_str(&format!(
                    "; failed to start {}, start the app manually",
                    report.failed_to_start.join(", ")
                ));
            }
            Ok((report.safety_backup_id, message))
        }
        .await;
        finish_job(&ctx.db, &job_id, result).await;
    });
    Ok(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_parsing() {
        assert_eq!(RestoreSource::parse("local").unwrap(), RestoreSource::Local);
        assert_eq!(RestoreSource::parse("s3").unwrap(), RestoreSource::S3);
        assert!(matches!(
            RestoreSource::parse("ftp"),
            Err(RestoreError::BadRequest(_))
        ));
    }

    #[test]
    fn temp_paths_stay_in_the_backup_dir() {
        let dir = Path::new("/data/backups/volumes/v1");
        let p = temp_download_path(dir, "backups/volume/v1/data-20261105-020000.tar.gz");
        assert_eq!(p.parent(), Some(dir));
        let name = p.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".restore-"));
        assert!(name.ends_with("-data-20261105-020000.tar.gz"));
    }

    #[test]
    fn lock_conflict_is_reported() {
        let held = lock("volume", "restore-lock-test").unwrap();
        assert!(matches!(
            lock("volume", "restore-lock-test"),
            Err(RestoreError::Conflict(_))
        ));
        drop(held);
        assert!(lock("volume", "restore-lock-test").is_ok());
    }
}
