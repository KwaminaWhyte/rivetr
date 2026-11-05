//! Database backups on the S3 side, restore through the rollback-with-data restore
//! helpers, and backup notifications for manual and scheduled runs.

use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::notify::{
    notify_backup, s3_destination, BackupEvent, BackupOutcome, BackupResourceKind,
};
use super::{s3_store, JobContext};
use crate::db::{BackupType, DatabaseBackup, ManagedDatabase, S3Backup, S3StorageConfig};
use crate::engine::database_backups::DatabaseBackupTask;
use crate::engine::deployment_snapshots::{
    is_rdb_restore, rdb_restore_blocker, restore_dump_file, snapshot_supported,
    sync_running_status, MAX_SNAPSHOT_BYTES, SUPPORTED_TYPES,
};
use crate::DbPool;

/// Seconds between `started_at` and `completed_at` of a backup row.
pub fn backup_duration(backup: &DatabaseBackup) -> Option<Duration> {
    let start = super::missed::parse_timestamp(backup.started_at.as_deref()?)?;
    let end = super::missed::parse_timestamp(backup.completed_at.as_deref()?)?;
    (end - start).to_std().ok()
}

/// Notify the result of a manual or scheduled database backup.
pub fn notify_database_backup(
    db: &DbPool,
    database: &ManagedDatabase,
    trigger: &str,
    result: std::result::Result<&DatabaseBackup, String>,
    destination: String,
) {
    let base = |outcome| {
        BackupEvent::new(outcome, BackupResourceKind::Database, database.name.clone())
            .resource_id(database.id.clone())
            .trigger(trigger)
            .destination(destination.clone())
    };
    let event = match result {
        Ok(backup) => base(BackupOutcome::Succeeded)
            .size(backup.file_size)
            .duration(backup_duration(backup)),
        Err(e) => base(BackupOutcome::Failed).error(e),
    };
    notify_backup(db, event);
}

/// Upload a completed local database backup to S3 and record it.
pub async fn upload_database_backup(
    ctx: &JobContext,
    database: &ManagedDatabase,
    backup: &DatabaseBackup,
    config: &S3StorageConfig,
) -> Result<S3Backup> {
    if backup.status != "completed" {
        bail!("Only completed backups can be uploaded");
    }
    let path = backup
        .file_path
        .as_deref()
        .context("Backup has no file on disk")?;
    s3_store::upload_and_record(
        &ctx.db,
        ctx.secret(),
        config,
        "database",
        Some(&database.id),
        Path::new(path),
    )
    .await
}

/// Destination label for a database backup.
pub fn destination_label(config: Option<&S3StorageConfig>) -> String {
    match config {
        Some(c) => format!("local + {}", s3_destination(&c.name)),
        None => "local".to_string(),
    }
}

/// Checks that need no container access: type supported, database running, dump
/// small enough, and for Redis-family types the data volume usable.
pub fn check_restorable(database: &ManagedDatabase, dump_size: u64) -> Result<()> {
    if !snapshot_supported(&database.db_type) {
        bail!(
            "Restore is not supported for {} databases (supported: {SUPPORTED_TYPES})",
            database.db_type
        );
    }
    if database.status != "running" {
        bail!("Database must be running to restore a backup");
    }
    if dump_size == 0 {
        bail!("Backup file is empty");
    }
    if dump_size > MAX_SNAPSHOT_BYTES as u64 {
        bail!(
            "Backup is {} bytes; restores are limited to {} bytes",
            dump_size,
            MAX_SNAPSHOT_BYTES
        );
    }
    if is_rdb_restore(&database.db_type) {
        if let Some(why) =
            rdb_restore_blocker(database.volume_path.as_deref(), |p| Path::new(p).exists())
        {
            bail!("Cannot restore, {why}");
        }
    }
    Ok(())
}

/// Restore `dump` into `database`: a safety backup of the current data first (a
/// normal manual backup), then the same per-engine restore used by rollback with
/// data. Returns the safety backup's id. The caller holds the database lock.
pub async fn restore_database(
    ctx: &JobContext,
    database: &ManagedDatabase,
    dump: &Path,
) -> Result<String> {
    let size = tokio::fs::metadata(dump)
        .await
        .with_context(|| format!("Backup file {} is missing", dump.display()))?
        .len();
    check_restorable(database, size)?;

    let task = DatabaseBackupTask::new(
        ctx.db.clone(),
        ctx.runtime.clone(),
        ctx.db_backup.clone(),
        ctx.data_dir.clone(),
    );
    let safety = task
        .backup_database(database, BackupType::Manual)
        .await
        .context("Safety backup of the current data failed; nothing was changed")?;

    let restored = restore_dump_file(ctx.runtime.as_ref(), database, &dump.to_string_lossy()).await;
    if is_rdb_restore(&database.db_type) {
        sync_running_status(&ctx.db, ctx.runtime.as_ref(), database).await;
    }
    restored.with_context(|| {
        format!(
            "Restore failed; the previous data is in safety backup {}",
            safety.id
        )
    })?;
    Ok(safety.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(db_type: &str, status: &str) -> ManagedDatabase {
        let mut d: ManagedDatabase = serde_json::from_value(serde_json::json!({
            "id": "db-1",
            "name": "orders",
            "db_type": db_type,
            "version": "16",
            "container_id": "c1",
            "status": status,
            "internal_port": 5432,
            "external_port": 0,
            "public_access": 0,
            "credentials": "{}",
            "volume_name": null,
            "volume_path": null,
            "memory_limit": null,
            "cpu_limit": null,
            "error_message": null,
            "project_id": null,
            "team_id": null,
            "created_at": "2026-11-05T00:00:00Z",
            "updated_at": "2026-11-05T00:00:00Z",
            "container_slug": null,
            "ssl_enabled": 0,
            "ssl_mode": null,
            "custom_image": null,
            "init_commands": null
        }))
        .unwrap_or_else(|e| panic!("ManagedDatabase fixture: {e}"));
        d.status = status.to_string();
        d
    }

    #[test]
    fn restorable_checks() {
        assert!(check_restorable(&database("postgres", "running"), 10).is_ok());
        let err = check_restorable(&database("postgres", "stopped"), 10).unwrap_err();
        assert!(err.to_string().contains("running"));
        let err = check_restorable(&database("clickhouse", "running"), 10).unwrap_err();
        assert!(err.to_string().contains("not supported"));
        assert!(check_restorable(&database("postgres", "running"), 0).is_err());
        assert!(check_restorable(
            &database("postgres", "running"),
            MAX_SNAPSHOT_BYTES as u64 + 1
        )
        .is_err());
        // Redis without a data volume on this host cannot be restored.
        let err = check_restorable(&database("redis", "running"), 10).unwrap_err();
        assert!(err.to_string().contains("data volume"));
    }

    #[test]
    fn duration_from_timestamps() {
        let mut b = DatabaseBackup::new("db-1", BackupType::Manual);
        b.started_at = Some("2026-11-05T02:00:00Z".into());
        b.completed_at = Some("2026-11-05T02:01:30Z".into());
        assert_eq!(backup_duration(&b), Some(Duration::from_secs(90)));
        b.completed_at = None;
        assert_eq!(backup_duration(&b), None);
    }
}
