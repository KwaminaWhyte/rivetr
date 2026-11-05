//! Volume backups: snapshot, history, retention, schedules and restore.
//!
//! Snapshot strategy: a throwaway helper container mounts the volume read-only and
//! streams `tar czf -` to a file on the host (`live`, the default: no downtime, but
//! files written during the tar can be caught mid-write). With `stop`, the app's
//! containers are stopped for the duration of the tar and started again afterwards,
//! which gives a consistent snapshot at the cost of a short outage.
//!
//! Restore: the archive is checked (gzip/tar readable, no absolute or `..` paths),
//! the app's containers are stopped, a safety backup of the current content is
//! taken, the volume is emptied and the archive extracted by a helper container,
//! and the containers are started again whatever happened.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::maintenance::{self, pause_containers, resource_key};
use super::notify::{
    notify_backup, s3_destination, BackupEvent, BackupOutcome, BackupResourceKind,
};
use super::retention::{select_for_pruning, Retained};
use super::{s3_store, schedule, JobContext};
use crate::db::{S3Backup, S3StorageConfig, Volume};
use crate::DbPool;

/// Image for the helper container that reads and writes volume content.
pub const HELPER_IMAGE: &str = "alpine:3";
/// Where local volume archives live, relative to the data dir.
pub const VOLUME_BACKUP_DIR: &str = "backups/volumes";
/// Largest archive a restore downloads from S3.
pub const MAX_VOLUME_RESTORE_BYTES: u64 = 50 * 1024 * 1024 * 1024;
/// Safety backups taken before a restore that are kept per volume.
pub const PRE_RESTORE_KEEP: usize = 3;
/// Upper bound for one snapshot or extract.
const HELPER_TIMEOUT: Duration = Duration::from_secs(4 * 3600);
/// How long app containers get to stop before they are killed.
const STOP_TIMEOUT_SECS: i64 = 30;

/// A volume backup run (row of `volume_backups`).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VolumeBackup {
    pub id: String,
    pub volume_id: String,
    pub trigger: String,
    pub destination: String,
    pub status: String,
    pub file_path: Option<String>,
    pub s3_backup_id: Option<String>,
    pub size_bytes: Option<i64>,
    pub checksum_sha256: Option<String>,
    pub snapshot_mode: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: String,
}

impl Retained for VolumeBackup {
    fn created_at(&self) -> &str {
        &self.created_at
    }
    fn id(&self) -> &str {
        &self.id
    }
}

/// A per-volume schedule (row of `volume_backup_schedules`).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VolumeBackupSchedule {
    pub id: String,
    pub volume_id: String,
    pub enabled: i32,
    pub cron_expression: String,
    pub retention_count: i32,
    pub destination: String,
    pub s3_config_id: Option<String>,
    pub snapshot_mode: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// How the snapshot is taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotMode {
    /// Tar from a read-only helper container while the app keeps running
    Live,
    /// Stop the app's containers during the tar
    Stop,
}

impl SnapshotMode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "" | "live" => Ok(Self::Live),
            "stop" => Ok(Self::Stop),
            other => bail!("snapshot_mode must be 'live' or 'stop', got '{other}'"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Stop => "stop",
        }
    }
}

/// Where a backup goes.
#[derive(Debug, Clone)]
pub enum Destination {
    Local,
    S3(Box<S3StorageConfig>),
}

impl Destination {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3(_) => "s3",
        }
    }

    fn label(&self) -> String {
        match self {
            Self::Local => "local".to_string(),
            Self::S3(c) => s3_destination(&c.name),
        }
    }
}

/// `docker` or `podman`, matching the configured runtime.
pub fn runtime_cli(runtime_name: &str) -> &'static str {
    if runtime_name.eq_ignore_ascii_case("podman") {
        "podman"
    } else {
        "docker"
    }
}

/// Arguments for the helper container that writes a gzip tar of `source` to stdout.
/// The volume is mounted read-only and the container has no network.
pub fn snapshot_args(source: &str) -> Vec<String> {
    [
        "run",
        "--rm",
        "--network",
        "none",
        "-v",
        &format!("{source}:/source:ro"),
        HELPER_IMAGE,
        "tar",
        "czf",
        "-",
        "-C",
        "/source",
        ".",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Arguments for the helper container that replaces `target`'s content with the
/// archive `archive_dir/archive_name`. The archive is listed first so a corrupt file
/// fails before anything is deleted.
pub fn restore_args(target: &str, archive_dir: &str, archive_name: &str) -> Vec<String> {
    let archive = format!("/backup/{}", archive_name.replace(['/', '\\'], "_"));
    let quoted = crate::engine::database_backups::shell_quote(&archive);
    let script = format!(
        "set -e; tar tzf {quoted} > /dev/null; find /target -mindepth 1 -delete; tar xzf {quoted} -C /target"
    );
    vec![
        "run".into(),
        "--rm".into(),
        "--network".into(),
        "none".into(),
        "-v".into(),
        format!("{target}:/target"),
        "-v".into(),
        format!("{archive_dir}:/backup:ro"),
        HELPER_IMAGE.into(),
        "sh".into(),
        "-c".into(),
        script,
    ]
}

/// File name for a new archive: `<volume>-<YYYYMMDD-HHMMSS>.tar.gz`.
pub fn archive_file_name(volume_name: &str, now: chrono::DateTime<chrono::Utc>) -> String {
    let safe: String = volume_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "volume".into()
    } else {
        safe
    };
    format!("{}-{}.tar.gz", safe, now.format("%Y%m%d-%H%M%S"))
}

/// Check that `path` is a readable gzip tar whose entries all stay inside the
/// extraction directory. Returns the entry count.
pub fn validate_archive(path: &Path) -> Result<usize> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open archive {}", path.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut count = 0usize;
    for entry in archive
        .entries()
        .context("Archive is not a readable gzip tar")?
    {
        let entry = entry.context("Archive is corrupt")?;
        let entry_path = entry.path().context("Archive entry has an invalid path")?;
        for component in entry_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    bail!(
                        "Archive entry '{}' escapes the volume",
                        entry_path.display()
                    )
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    bail!(
                        "Archive entry '{}' has an absolute path",
                        entry_path.display()
                    )
                }
                _ => {}
            }
        }
        count += 1;
    }
    if count == 0 {
        bail!("Archive is empty");
    }
    Ok(count)
}

/// Local directory for a volume's archives.
pub fn volume_backup_dir(data_dir: &Path, volume_id: &str) -> PathBuf {
    data_dir.join(VOLUME_BACKUP_DIR).join(volume_id)
}

pub async fn get_volume(db: &DbPool, id: &str) -> Result<Volume> {
    sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at \
         FROM volumes WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| anyhow!("Volume not found"))
}

pub async fn get_backup(db: &DbPool, id: &str) -> Result<Option<VolumeBackup>> {
    Ok(
        sqlx::query_as::<_, VolumeBackup>("SELECT * FROM volume_backups WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

/// Running containers of the volume's app (main deployment plus replicas).
pub async fn app_container_ids(db: &DbPool, app_id: &str) -> Vec<String> {
    let mut ids: Vec<String> = sqlx::query_scalar(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' \
         AND container_id IS NOT NULL AND container_id != ''",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let replicas: Vec<String> = sqlx::query_scalar(
        "SELECT container_id FROM app_replicas WHERE app_id = ? AND status = 'running' \
         AND container_id IS NOT NULL AND container_id != ''",
    )
    .bind(app_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    ids.extend(replicas);
    ids.sort();
    ids.dedup();
    ids
}

/// Stop containers (paused for the monitor), returning the ones that were running.
async fn stop_containers(ctx: &JobContext, ids: &[String]) -> Result<Vec<String>> {
    let mut stopped = Vec::new();
    for id in ids {
        match ctx.runtime.inspect(id).await {
            Ok(info) if info.running => {}
            _ => continue,
        }
        if let Err(e) = ctx.runtime.stop_timeout(id, Some(STOP_TIMEOUT_SECS)).await {
            // Bring back what was already stopped before failing.
            start_containers(ctx, &stopped).await;
            return Err(e.context(format!("Failed to stop container {id}")));
        }
        stopped.push(id.clone());
    }
    Ok(stopped)
}

/// Start containers again; failures are logged, not returned, so every one is tried.
async fn start_containers(ctx: &JobContext, ids: &[String]) -> Vec<String> {
    let mut failed = Vec::new();
    for id in ids {
        if let Err(e) = ctx.runtime.start(id).await {
            tracing::error!(container = %id, error = %e, "Failed to start container after volume job");
            failed.push(id.clone());
        }
    }
    failed
}

/// Run a helper container with `args`, sending stdout to `stdout_to` when given.
async fn run_helper(cli: &str, args: &[String], stdout_to: Option<&Path>) -> Result<()> {
    let mut cmd = tokio::process::Command::new(cli);
    cmd.args(args).stdin(Stdio::null()).stderr(Stdio::piped());
    match stdout_to {
        Some(path) => {
            let file = std::fs::File::create(path)
                .with_context(|| format!("Failed to create {}", path.display()))?;
            cmd.stdout(Stdio::from(file));
        }
        None => {
            cmd.stdout(Stdio::null());
        }
    }
    cmd.kill_on_drop(true);
    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to run {cli}; is it installed on this host?"))?;
    let output = tokio::time::timeout(HELPER_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| anyhow!("Helper container timed out after {:?}", HELPER_TIMEOUT))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Helper container exited with {}: {}",
            output.status,
            stderr.trim()
        );
    }
    Ok(())
}

/// Write a gzip tar of the volume to `dest`.
async fn snapshot_to_file(ctx: &JobContext, volume: &Volume, dest: &Path) -> Result<()> {
    if volume.host_path.starts_with('/') && !Path::new(&volume.host_path).exists() {
        bail!(
            "Volume path {} does not exist on this host (has the app been deployed?)",
            volume.host_path
        );
    }
    let cli = runtime_cli(ctx.runtime.name());
    let result = run_helper(cli, &snapshot_args(&volume.host_path), Some(dest)).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(dest).await;
    }
    result?;
    let size = tokio::fs::metadata(dest)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    if size == 0 {
        let _ = tokio::fs::remove_file(dest).await;
        bail!("Snapshot produced an empty archive");
    }
    Ok(())
}

async fn insert_record(
    db: &DbPool,
    volume_id: &str,
    trigger: &str,
    destination: &str,
    mode: SnapshotMode,
) -> Result<VolumeBackup> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO volume_backups (id, volume_id, trigger, destination, status, snapshot_mode, created_at) \
         VALUES (?, ?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&id)
    .bind(volume_id)
    .bind(trigger)
    .bind(destination)
    .bind(mode.as_str())
    .bind(&now)
    .execute(db)
    .await?;
    get_backup(db, &id)
        .await?
        .ok_or_else(|| anyhow!("Backup record vanished"))
}

/// Create the pending history row for a backup started from the API, so the
/// response can return it before the job finishes.
pub async fn create_pending(
    db: &DbPool,
    volume_id: &str,
    trigger: &str,
    destination: &Destination,
    mode: SnapshotMode,
) -> Result<VolumeBackup> {
    insert_record(db, volume_id, trigger, destination.as_str(), mode).await
}

/// Take a backup into the existing `record`. The caller holds the volume lock.
/// In `Stop` mode the app's containers are stopped around the snapshot.
async fn backup_into(
    ctx: &JobContext,
    volume: &Volume,
    record: &VolumeBackup,
    destination: &Destination,
    mode: SnapshotMode,
) -> Result<VolumeBackup> {
    let started = Instant::now();
    let started_at = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE volume_backups SET status = 'running', started_at = ? WHERE id = ?")
        .bind(&started_at)
        .bind(&record.id)
        .execute(&ctx.db)
        .await?;

    let result: Result<(Option<String>, Option<String>, i64, String)> = async {
        let dir = volume_backup_dir(&ctx.data_dir, &volume.id);
        tokio::fs::create_dir_all(&dir)
            .await
            .with_context(|| format!("Failed to create {}", dir.display()))?;
        let path = dir.join(archive_file_name(&volume.name, chrono::Utc::now()));

        if mode == SnapshotMode::Stop {
            let ids = app_container_ids(&ctx.db, &volume.app_id).await;
            let _paused = pause_containers(&ids);
            let stopped = stop_containers(ctx, &ids).await?;
            let snap = snapshot_to_file(ctx, volume, &path).await;
            let failed = start_containers(ctx, &stopped).await;
            snap?;
            if !failed.is_empty() {
                tracing::warn!(
                    volume = %volume.name,
                    containers = ?failed,
                    "Backup finished but some app containers did not start again"
                );
            }
        } else {
            snapshot_to_file(ctx, volume, &path).await?;
        }

        let (sha256, size) = s3_store::sha256_file(&path).await?;
        match destination {
            Destination::Local => Ok((
                Some(path.to_string_lossy().to_string()),
                None,
                size as i64,
                sha256,
            )),
            Destination::S3(config) => {
                let uploaded = s3_store::upload_and_record(
                    &ctx.db,
                    ctx.secret(),
                    config,
                    "volume",
                    Some(&volume.id),
                    &path,
                )
                .await;
                // The archive lives in S3 now; keep no local copy either way.
                let _ = tokio::fs::remove_file(&path).await;
                let s3 = uploaded?;
                Ok((None, Some(s3.id), size as i64, sha256))
            }
        }
    }
    .await;

    let duration_ms = started.elapsed().as_millis() as i64;
    let completed_at = chrono::Utc::now().to_rfc3339();
    match result {
        Ok((file_path, s3_id, size, sha256)) => {
            sqlx::query(
                "UPDATE volume_backups SET status = 'completed', file_path = ?, s3_backup_id = ?, \
                 size_bytes = ?, checksum_sha256 = ?, completed_at = ?, duration_ms = ? WHERE id = ?",
            )
            .bind(&file_path)
            .bind(&s3_id)
            .bind(size)
            .bind(&sha256)
            .bind(&completed_at)
            .bind(duration_ms)
            .bind(&record.id)
            .execute(&ctx.db)
            .await?;
        }
        Err(e) => {
            sqlx::query(
                "UPDATE volume_backups SET status = 'failed', error_message = ?, completed_at = ?, \
                 duration_ms = ? WHERE id = ?",
            )
            .bind(e.to_string())
            .bind(&completed_at)
            .bind(duration_ms)
            .bind(&record.id)
            .execute(&ctx.db)
            .await?;
            return Err(e);
        }
    }
    get_backup(&ctx.db, &record.id)
        .await?
        .ok_or_else(|| anyhow!("Backup record vanished"))
}

/// Run a backup job for `record` (created with [`create_pending`]): lock the volume,
/// snapshot, upload when the destination is S3, record the result and notify.
pub async fn run_backup_job(
    ctx: &JobContext,
    volume: &Volume,
    record: VolumeBackup,
    destination: Destination,
    mode: SnapshotMode,
) -> Result<VolumeBackup> {
    let key = resource_key("volume", &volume.id);
    let Some(_lock) = maintenance::try_lock(&key) else {
        let msg = "Another backup or restore of this volume is running";
        let _ = sqlx::query(
            "UPDATE volume_backups SET status = 'failed', error_message = ?, completed_at = ? WHERE id = ?",
        )
        .bind(msg)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&record.id)
        .execute(&ctx.db)
        .await;
        notify_result(ctx, volume, &record, &destination, Err(msg.to_string()));
        bail!(msg);
    };

    let result = backup_into(ctx, volume, &record, &destination, mode).await;
    notify_result(
        ctx,
        volume,
        &record,
        &destination,
        result
            .as_ref()
            .map(|b| b.clone())
            .map_err(|e| e.to_string()),
    );
    result
}

fn notify_result(
    ctx: &JobContext,
    volume: &Volume,
    record: &VolumeBackup,
    destination: &Destination,
    result: std::result::Result<VolumeBackup, String>,
) {
    if record.trigger == "pre_restore" {
        return;
    }
    let base = |outcome| {
        BackupEvent::new(outcome, BackupResourceKind::Volume, volume.name.clone())
            .resource_id(volume.id.clone())
            .app_id(Some(volume.app_id.clone()))
            .trigger(record.trigger.clone())
            .destination(destination.label())
    };
    let event = match result {
        Ok(done) => base(BackupOutcome::Succeeded)
            .size(done.size_bytes)
            .duration(
                done.duration_ms
                    .map(|ms| Duration::from_millis(ms.max(0) as u64)),
            ),
        Err(e) => base(BackupOutcome::Failed).error(e),
    };
    notify_backup(&ctx.db, event);
}

/// Delete one volume backup: its local archive, its S3 object and records.
pub async fn delete_backup(ctx: &JobContext, backup: &VolumeBackup) -> Result<()> {
    if let Some(ref path) = backup.file_path {
        if let Err(e) = tokio::fs::remove_file(path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %path, error = %e, "Failed to delete volume backup file");
            }
        }
    }
    if let Some(ref s3_id) = backup.s3_backup_id {
        delete_s3_backup(ctx, s3_id).await;
    }
    sqlx::query("DELETE FROM volume_backups WHERE id = ?")
        .bind(&backup.id)
        .execute(&ctx.db)
        .await?;
    Ok(())
}

/// Delete an `s3_backups` row and its object (object deletion is best effort).
pub async fn delete_s3_backup(ctx: &JobContext, s3_backup_id: &str) {
    let row: Option<S3Backup> = sqlx::query_as("SELECT * FROM s3_backups WHERE id = ?")
        .bind(s3_backup_id)
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten();
    if let Some(row) = row {
        if let Ok(config) = s3_store::load_config(&ctx.db, &row.storage_config_id).await {
            if let Ok(client) = s3_store::client_for(&config, ctx.secret()) {
                if let Err(e) = client.delete_backup(&row.s3_key).await {
                    tracing::warn!(key = %row.s3_key, error = %e, "Failed to delete S3 object");
                }
            }
        }
        let _ = sqlx::query("DELETE FROM s3_backups WHERE id = ?")
            .bind(s3_backup_id)
            .execute(&ctx.db)
            .await;
    }
}

/// Apply retention to a volume's history: keep the newest `keep` completed backups
/// (manual and scheduled), the newest [`PRE_RESTORE_KEEP`] safety backups, and drop
/// failed runs older than the newest `keep`. Returns how many were deleted.
pub async fn prune_volume_backups(ctx: &JobContext, volume_id: &str, keep: usize) -> usize {
    let all: Vec<VolumeBackup> = sqlx::query_as(
        "SELECT * FROM volume_backups WHERE volume_id = ? AND status IN ('completed', 'failed')",
    )
    .bind(volume_id)
    .fetch_all(&ctx.db)
    .await
    .unwrap_or_default();

    let (regular, rest): (Vec<VolumeBackup>, Vec<VolumeBackup>) = all
        .into_iter()
        .partition(|b| b.status == "completed" && b.trigger != "pre_restore");
    let (safety, failed): (Vec<VolumeBackup>, Vec<VolumeBackup>) = rest
        .into_iter()
        .partition(|b| b.status == "completed" && b.trigger == "pre_restore");

    let mut doomed: Vec<VolumeBackup> = select_for_pruning(&regular, keep)
        .into_iter()
        .cloned()
        .collect();
    doomed.extend(
        select_for_pruning(&safety, PRE_RESTORE_KEEP)
            .into_iter()
            .cloned(),
    );
    doomed.extend(select_for_pruning(&failed, keep).into_iter().cloned());

    let mut deleted = 0;
    for backup in doomed {
        if delete_backup(ctx, &backup).await.is_ok() {
            deleted += 1;
        }
    }
    if deleted > 0 {
        tracing::info!(
            volume_id,
            deleted,
            keep,
            "Pruned volume backups past retention"
        );
    }
    deleted
}

/// Run every enabled schedule that is due. The next run is recorded before the job
/// starts so a slow backup is not started twice.
pub async fn run_due_schedules(ctx: &JobContext) {
    let now = chrono::Utc::now();
    let due: Vec<VolumeBackupSchedule> = match sqlx::query_as(
        "SELECT * FROM volume_backup_schedules WHERE enabled = 1 AND next_run_at IS NOT NULL",
    )
    .fetch_all(&ctx.db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::debug!(error = %e, "Failed to load volume backup schedules");
            return;
        }
    };

    for sched in due {
        let Some(next) = sched
            .next_run_at
            .as_deref()
            .and_then(super::missed::parse_timestamp)
        else {
            continue;
        };
        if next > now {
            continue;
        }
        let new_next = match schedule::next_run_after(&sched.cron_expression, now) {
            Ok(t) => Some(t.to_rfc3339()),
            Err(e) => {
                tracing::warn!(schedule = %sched.id, error = %e, "Invalid volume backup cron; schedule paused");
                None
            }
        };
        let _ = sqlx::query(
            "UPDATE volume_backup_schedules SET last_run_at = ?, next_run_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(now.to_rfc3339())
        .bind(&new_next)
        .bind(now.to_rfc3339())
        .bind(&sched.id)
        .execute(&ctx.db)
        .await;

        let ctx = ctx.clone();
        tokio::spawn(async move {
            if let Err(e) = run_scheduled(&ctx, &sched).await {
                tracing::warn!(schedule = %sched.id, error = %e, "Scheduled volume backup failed");
            }
        });
    }
}

async fn run_scheduled(ctx: &JobContext, sched: &VolumeBackupSchedule) -> Result<()> {
    let volume = get_volume(&ctx.db, &sched.volume_id).await?;
    let destination =
        resolve_destination(&ctx.db, &sched.destination, sched.s3_config_id.as_deref()).await;
    let mode = SnapshotMode::parse(&sched.snapshot_mode).unwrap_or(SnapshotMode::Live);
    let destination = match destination {
        Ok(d) => d,
        Err(e) => {
            // Record the failed run so it shows in history and notifies.
            let record =
                insert_record(&ctx.db, &volume.id, "scheduled", &sched.destination, mode).await?;
            let _ = sqlx::query(
                "UPDATE volume_backups SET status = 'failed', error_message = ?, completed_at = ? WHERE id = ?",
            )
            .bind(e.to_string())
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&record.id)
            .execute(&ctx.db)
            .await;
            notify_result(
                ctx,
                &volume,
                &record,
                &Destination::Local,
                Err(e.to_string()),
            );
            return Err(e);
        }
    };
    let record =
        insert_record(&ctx.db, &volume.id, "scheduled", destination.as_str(), mode).await?;
    let result = run_backup_job(ctx, &volume, record, destination, mode).await;
    prune_volume_backups(ctx, &volume.id, sched.retention_count.max(1) as usize).await;
    result.map(|_| ())
}

/// Turn a stored destination into a [`Destination`].
pub async fn resolve_destination(
    db: &DbPool,
    destination: &str,
    s3_config_id: Option<&str>,
) -> Result<Destination> {
    match destination {
        "local" => Ok(Destination::Local),
        "s3" => {
            let id = s3_config_id.filter(|s| !s.is_empty()).ok_or_else(|| {
                anyhow!("S3 destination selected but no S3 storage config is set")
            })?;
            Ok(Destination::S3(Box::new(
                s3_store::load_config(db, id).await?,
            )))
        }
        other => bail!("Unknown backup destination '{other}'"),
    }
}

/// Outcome of a volume restore.
#[derive(Debug, Clone, Serialize)]
pub struct VolumeRestoreReport {
    /// Safety backup of the content that was replaced
    pub safety_backup_id: String,
    /// Containers that were stopped and started again
    pub restarted_containers: usize,
    /// Containers that failed to start again (start them manually)
    pub failed_to_start: Vec<String>,
}

/// Replace a volume's content with `archive` (a gzip tar on this host). The caller
/// holds the volume lock. Stops the app's containers, takes a safety backup, empties
/// the volume, extracts the archive and starts the containers again.
pub async fn restore_from_archive(
    ctx: &JobContext,
    volume: &Volume,
    archive: &Path,
) -> Result<VolumeRestoreReport> {
    let archive_owned = archive.to_path_buf();
    tokio::task::spawn_blocking(move || validate_archive(&archive_owned))
        .await
        .map_err(|e| anyhow!("Archive check panicked: {e}"))?
        .context("Backup archive failed validation; nothing was changed")?;

    if volume.host_path.starts_with('/') {
        tokio::fs::create_dir_all(&volume.host_path)
            .await
            .with_context(|| format!("Failed to create {}", volume.host_path))?;
    }

    let ids = app_container_ids(&ctx.db, &volume.app_id).await;
    let _paused = pause_containers(&ids);
    let stopped = stop_containers(ctx, &ids).await?;

    let outcome: Result<String> = async {
        // Safety backup of what is about to be replaced, taken with the app stopped.
        let record = insert_record(
            &ctx.db,
            &volume.id,
            "pre_restore",
            "local",
            SnapshotMode::Live,
        )
        .await?;
        let safety = backup_into(
            ctx,
            volume,
            &record,
            &Destination::Local,
            SnapshotMode::Live,
        )
        .await
        .context("Safety backup of the current content failed; nothing was changed")?;

        let dir = archive
            .parent()
            .ok_or_else(|| anyhow!("Archive has no parent directory"))?;
        let name = archive
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Archive has no usable name"))?;
        let cli = runtime_cli(ctx.runtime.name());
        run_helper(
            cli,
            &restore_args(&volume.host_path, &dir.to_string_lossy(), name),
            None,
        )
        .await
        .with_context(|| {
            format!(
                "Restore failed; the previous content is in safety backup {}",
                safety.id
            )
        })?;
        Ok(safety.id)
    }
    .await;

    let failed_to_start = start_containers(ctx, &stopped).await;
    let safety_backup_id = outcome?;
    Ok(VolumeRestoreReport {
        safety_backup_id,
        restarted_containers: stopped.len() - failed_to_start.len(),
        failed_to_start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn snapshot_reads_volume_read_only_without_network() {
        let args = snapshot_args("/var/lib/rivetr/volumes/app/data");
        assert_eq!(
            args,
            vec![
                "run",
                "--rm",
                "--network",
                "none",
                "-v",
                "/var/lib/rivetr/volumes/app/data:/source:ro",
                HELPER_IMAGE,
                "tar",
                "czf",
                "-",
                "-C",
                "/source",
                "."
            ]
        );
        // Named volumes work the same way.
        assert!(snapshot_args("pgdata").contains(&"pgdata:/source:ro".to_string()));
    }

    #[test]
    fn restore_lists_before_deleting_and_quotes_the_name() {
        let args = restore_args("uploads", "/data/backups/volumes/v1", "it's.tar.gz");
        assert!(args.contains(&"uploads:/target".to_string()));
        assert!(args.contains(&"/data/backups/volumes/v1:/backup:ro".to_string()));
        let script = args.last().unwrap();
        let list = script.find("tar tzf").unwrap();
        let delete = script.find("find /target -mindepth 1 -delete").unwrap();
        let extract = script.find("tar xzf").unwrap();
        assert!(list < delete && delete < extract);
        assert!(script.starts_with("set -e;"));
        assert!(script.contains("'/backup/it'\\''s.tar.gz'"));
        // Path separators in the name cannot escape /backup.
        let args = restore_args("v", "/d", "../../etc/x.tar.gz");
        assert!(args.last().unwrap().contains("/backup/.._.._etc_x.tar.gz"));
    }

    #[test]
    fn runtime_cli_matches_runtime() {
        assert_eq!(runtime_cli("Docker"), "docker");
        assert_eq!(runtime_cli("Podman"), "podman");
        assert_eq!(runtime_cli("none"), "docker");
    }

    #[test]
    fn archive_names_are_safe_and_timestamped() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 11, 5, 2, 0, 0).unwrap();
        assert_eq!(
            archive_file_name("data", now),
            "data-20261105-020000.tar.gz"
        );
        assert_eq!(
            archive_file_name("my vol/../x", now),
            "my_vol____x-20261105-020000.tar.gz"
        );
        assert_eq!(archive_file_name("", now), "volume-20261105-020000.tar.gz");
        let parsed = s3_store::timestamp_from_file_name(&archive_file_name("data", now));
        assert_eq!(parsed, Some(now));
    }

    #[test]
    fn snapshot_mode_parsing() {
        assert_eq!(SnapshotMode::parse("live").unwrap(), SnapshotMode::Live);
        assert_eq!(SnapshotMode::parse("").unwrap(), SnapshotMode::Live);
        assert_eq!(SnapshotMode::parse("stop").unwrap(), SnapshotMode::Stop);
        assert!(SnapshotMode::parse("pause").is_err());
    }

    fn write_archive(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn archive_validation() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.tar.gz");
        write_archive(&good, &[("./a.txt", b"a"), ("sub/b.txt", b"bb")]);
        assert_eq!(validate_archive(&good).unwrap(), 2);

        let not_gzip = dir.path().join("plain.tar.gz");
        std::fs::write(&not_gzip, b"definitely not gzip").unwrap();
        assert!(validate_archive(&not_gzip).is_err());

        let empty = dir.path().join("empty.tar.gz");
        write_archive(&empty, &[]);
        assert!(validate_archive(&empty)
            .unwrap_err()
            .to_string()
            .contains("empty"));
    }

    #[test]
    fn archive_validation_rejects_escaping_paths() {
        // tar::Builder refuses `..` itself, so write the raw header name by hand.
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.tar.gz");
        let file = std::fs::File::create(&bad).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.as_gnu_mut().unwrap().name[..12].copy_from_slice(b"../evil.txt\0");
        header.set_size(1);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, &b"x"[..]).unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        let err = validate_archive(&bad).unwrap_err().to_string();
        assert!(err.contains("escapes"), "{err}");
    }
}
