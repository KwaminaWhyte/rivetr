//! Rollback with data: pre-deploy snapshots of an app's linked managed databases.
//!
//! When `apps.snapshot_before_deploy` is on, the deploy pipeline dumps every linked
//! managed database right before the new container starts, reusing
//! `DatabaseBackupTask::backup_database` (the dump is a normal `database_backups` row of
//! type `pre_deploy`). `deployment_db_snapshots` ties each dump to the deployment that
//! was about to start (`deployment_id`) and to the deployment that was running and had
//! last used the data (`source_deployment_id`).
//!
//! Rolling back to deployment T with `restore_data` restores the snapshots whose
//! `source_deployment_id` is T: the data as T left it. Everything written after the
//! snapshot (including by T and by the deployment being rolled back) is lost, so the
//! restore takes a safety backup of the current data first.
//!
//! Only text dump formats survive the existing dump path (stdout captured as a string),
//! so snapshots are limited to postgres, mysql and mariadb.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Serialize;

use super::add_deployment_log;
use super::database_backups::DatabaseBackupTask;
use crate::config::DatabaseBackupConfig;
use crate::db::{App, BackupType, DatabaseCredentials, ManagedDatabase};
use crate::runtime::ContainerRuntime;
use crate::DbPool;

/// Largest dump we keep or restore. Restores are streamed into the container in
/// base64 chunks over exec, so very large dumps are impractical.
pub const MAX_SNAPSHOT_BYTES: i64 = 256 * 1024 * 1024;

/// Upper bound on how long a deploy waits for one snapshot, whatever the backup timeout.
const SNAPSHOT_TIMEOUT_CAP_SECS: u64 = 900;

/// Base64 characters per exec when uploading a dump (multiple of 4, well under the
/// 128 KiB single-argument limit).
const UPLOAD_CHUNK_CHARS: usize = 48 * 1024;

/// Deployments pruned per pass, with a pause between passes.
const PRUNE_BATCH: usize = 50;
const PRUNE_PAUSE: Duration = Duration::from_millis(50);

const RESTORE_B64_PATH: &str = "/tmp/rivetr_snapshot_restore.b64";
const RESTORE_SQL_PATH: &str = "/tmp/rivetr_snapshot_restore.sql";

/// Where snapshot dumps are written; same layout as regular database backups.
#[derive(Debug, Clone)]
pub struct SnapshotContext {
    pub data_dir: PathBuf,
    pub backup_config: DatabaseBackupConfig,
}

impl SnapshotContext {
    fn task(&self, db: &DbPool, runtime: Arc<dyn ContainerRuntime>) -> DatabaseBackupTask {
        DatabaseBackupTask::new(
            db.clone(),
            runtime,
            self.backup_config.clone(),
            self.data_dir.clone(),
        )
    }
}

/// A snapshot joined with its dump file and database, as listed by the API.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DeploymentSnapshot {
    pub id: String,
    /// Deployment that was about to start when the snapshot was taken
    pub deployment_id: String,
    pub app_id: String,
    /// Deployment that was running at that moment; rolling back to it restores this snapshot
    pub source_deployment_id: Option<String>,
    pub database_id: String,
    pub database_name: String,
    pub db_type: String,
    pub backup_id: String,
    pub size_bytes: i64,
    pub created_at: String,
    #[serde(skip_serializing)]
    pub file_path: Option<String>,
    #[serde(skip_serializing)]
    pub backup_status: String,
    #[serde(skip_serializing)]
    pub database_status: String,
}

/// Database types whose dumps are plain text and can be restored with the client tools.
pub fn snapshot_supported(db_type: &str) -> bool {
    matches!(db_type, "postgres" | "mysql" | "mariadb")
}

/// Keep the first (newest) snapshot per database. Input must be newest first.
pub fn latest_per_database(snapshots: Vec<DeploymentSnapshot>) -> Vec<DeploymentSnapshot> {
    let mut seen = HashSet::new();
    snapshots
        .into_iter()
        .filter(|s| seen.insert(s.database_id.clone()))
        .collect()
}

/// Verify a restore can proceed. Called before the rollback deployment is created, so
/// nothing has changed yet when this fails. `file_exists` is injected for testing.
pub fn check_restore_preconditions(
    snapshots: &[DeploymentSnapshot],
    file_exists: impl Fn(&str) -> bool,
) -> Result<(), String> {
    if snapshots.is_empty() {
        return Err(
            "restore_data was requested but no database snapshot exists for the target \
             deployment. Snapshots are only taken when snapshot_before_deploy is enabled \
             for the app, and old ones are pruned (data_retention.deployment_snapshots_keep)."
                .to_string(),
        );
    }
    for s in snapshots {
        if !snapshot_supported(&s.db_type) {
            return Err(format!(
                "Snapshot restore is not supported for {} database '{}'",
                s.db_type, s.database_name
            ));
        }
        if s.backup_status != "completed" {
            return Err(format!(
                "Snapshot of database '{}' is not complete (status: {})",
                s.database_name, s.backup_status
            ));
        }
        match s.file_path.as_deref() {
            Some(p) if file_exists(p) => {}
            _ => {
                return Err(format!(
                    "Snapshot file for database '{}' is missing on disk",
                    s.database_name
                ))
            }
        }
        if s.size_bytes > MAX_SNAPSHOT_BYTES {
            return Err(format!(
                "Snapshot of database '{}' is too large to restore ({} bytes)",
                s.database_name, s.size_bytes
            ));
        }
        if s.database_status != "running" {
            return Err(format!(
                "Database '{}' must be running to restore its snapshot (status: {})",
                s.database_name, s.database_status
            ));
        }
    }
    Ok(())
}

const SNAPSHOT_SELECT: &str = r#"
    SELECT s.id, s.deployment_id, s.app_id, s.source_deployment_id, s.database_id,
           d.name AS database_name, d.db_type AS db_type, s.backup_id, s.size_bytes,
           s.created_at, b.file_path AS file_path, b.status AS backup_status,
           d.status AS database_status
    FROM deployment_db_snapshots s
    JOIN databases d ON d.id = s.database_id
    JOIN database_backups b ON b.id = s.backup_id
"#;

/// Snapshots to restore when rolling back to `target_deployment_id`: the newest one per
/// database whose source deployment is the target.
pub async fn load_restorable_snapshots(
    db: &DbPool,
    target_deployment_id: &str,
) -> Result<Vec<DeploymentSnapshot>, sqlx::Error> {
    let sql = format!(
        "{SNAPSHOT_SELECT} WHERE s.source_deployment_id = ? ORDER BY s.created_at DESC, s.rowid DESC"
    );
    let rows: Vec<DeploymentSnapshot> = sqlx::query_as(&sql)
        .bind(target_deployment_id)
        .fetch_all(db)
        .await?;
    Ok(latest_per_database(rows))
}

/// Dump every linked managed database before `deployment_id` starts. Never fails the
/// deploy: problems are logged to the deployment log and the deploy continues.
pub async fn take_pre_deploy_snapshots(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    ctx: &SnapshotContext,
    app: &App,
    deployment_id: &str,
) {
    let links = crate::api::database_links::load_links_for_app(db, &app.id).await;
    if links.is_empty() {
        return;
    }

    let source_deployment_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM deployments WHERE app_id = ? AND status = 'running' AND id != ? \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&app.id)
    .bind(deployment_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();

    let task = ctx.task(db, runtime);
    let timeout = Duration::from_secs(
        ctx.backup_config
            .timeout_seconds
            .clamp(1, SNAPSHOT_TIMEOUT_CAP_SECS),
    );
    let total = links.len();
    let mut taken = 0usize;

    for (database, _prefix) in links {
        if !snapshot_supported(&database.db_type) {
            dlog(
                db,
                deployment_id,
                "warn",
                format!(
                    "Pre-deploy snapshot skipped for {} database '{}': type not supported",
                    database.db_type, database.name
                ),
            )
            .await;
            continue;
        }
        if database.status != "running" || database.container_id.is_none() {
            dlog(
                db,
                deployment_id,
                "warn",
                format!(
                    "Pre-deploy snapshot skipped for database '{}': not running",
                    database.name
                ),
            )
            .await;
            continue;
        }

        let _ = add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!(
                "Taking pre-deploy snapshot of database '{}'...",
                database.name
            ),
        )
        .await;

        let result = tokio::time::timeout(
            timeout,
            task.backup_database(&database, BackupType::PreDeploy),
        )
        .await;
        let backup = match result {
            Ok(Ok(b)) => b,
            Ok(Err(e)) => {
                // Drop the failed backup row; the error is in the deployment log.
                let _ = sqlx::query(
                    "DELETE FROM database_backups \
                     WHERE database_id = ? AND backup_type = 'pre_deploy' AND status = 'failed'",
                )
                .bind(&database.id)
                .execute(db)
                .await;
                dlog(
                    db,
                    deployment_id,
                    "warn",
                    format!(
                        "Pre-deploy snapshot of database '{}' failed, deploying anyway: {}",
                        database.name, e
                    ),
                )
                .await;
                continue;
            }
            Err(_) => {
                dlog(
                    db,
                    deployment_id,
                    "warn",
                    format!(
                    "Pre-deploy snapshot of database '{}' timed out after {}s, deploying anyway",
                    database.name,
                    timeout.as_secs()
                ),
                )
                .await;
                continue;
            }
        };

        let size = backup.file_size.unwrap_or(0);
        if size > MAX_SNAPSHOT_BYTES || size == 0 {
            remove_backup(db, &backup.id, backup.file_path.as_deref()).await;
            dlog(
                db,
                deployment_id,
                "warn",
                format!(
                    "Pre-deploy snapshot of database '{}' discarded ({} bytes is outside the \
                 supported range), deploying anyway",
                    database.name, size
                ),
            )
            .await;
            continue;
        }

        let inserted = sqlx::query(
            "INSERT INTO deployment_db_snapshots \
             (id, deployment_id, app_id, source_deployment_id, database_id, backup_id, size_bytes, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(deployment_id)
        .bind(&app.id)
        .bind(&source_deployment_id)
        .bind(&database.id)
        .bind(&backup.id)
        .bind(size)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(db)
        .await;
        match inserted {
            Ok(_) => {
                taken += 1;
                let _ = add_deployment_log(
                    db,
                    deployment_id,
                    "info",
                    &format!(
                        "Snapshot of database '{}' saved ({} bytes)",
                        database.name, size
                    ),
                )
                .await;
            }
            Err(e) => {
                remove_backup(db, &backup.id, backup.file_path.as_deref()).await;
                dlog(
                    db,
                    deployment_id,
                    "warn",
                    format!(
                        "Could not record snapshot of database '{}', deploying anyway: {}",
                        database.name, e
                    ),
                )
                .await;
            }
        }
    }

    let _ = add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!(
            "Pre-deploy snapshots: {} of {} linked databases",
            taken, total
        ),
    )
    .await;
}

/// Best-effort deployment log line; a failed log write must never affect the deploy.
async fn dlog(db: &DbPool, deployment_id: &str, level: &str, msg: String) {
    let _ = add_deployment_log(db, deployment_id, level, &msg).await;
}

async fn remove_backup(db: &DbPool, backup_id: &str, file_path: Option<&str>) {
    if let Some(p) = file_path {
        let _ = tokio::fs::remove_file(p).await;
    }
    let _ = sqlx::query("DELETE FROM deployment_db_snapshots WHERE backup_id = ?")
        .bind(backup_id)
        .execute(db)
        .await;
    let _ = sqlx::query("DELETE FROM database_backups WHERE id = ?")
        .bind(backup_id)
        .execute(db)
        .await;
}

/// Restore snapshots into their databases, logging to `rollback_deployment_id`. Call only
/// after the rollback container serves traffic and the old containers are retired, so
/// nothing is writing the old schema. Each database first gets a safety backup of its
/// current data (a normal manual backup); if that fails, that database is left untouched.
pub async fn restore_snapshots(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    ctx: &SnapshotContext,
    rollback_deployment_id: &str,
    snapshots: &[DeploymentSnapshot],
) -> usize {
    let task = ctx.task(db, runtime.clone());
    let mut restored = 0usize;

    for snap in snapshots {
        let database: ManagedDatabase = match sqlx::query_as("SELECT * FROM databases WHERE id = ?")
            .bind(&snap.database_id)
            .fetch_optional(db)
            .await
        {
            Ok(Some(d)) => d,
            _ => {
                dlog(
                    db,
                    rollback_deployment_id,
                    "error",
                    format!(
                        "Data restore skipped: database '{}' no longer exists",
                        snap.database_name
                    ),
                )
                .await;
                continue;
            }
        };

        dlog(db, rollback_deployment_id, "info",
            format!(
                "Restoring database '{}' from snapshot taken {} (data written since then will be replaced)",
                database.name, snap.created_at
            ),
        )
        .await;

        match task.backup_database(&database, BackupType::Manual).await {
            Ok(b) => {
                dlog(
                    db,
                    rollback_deployment_id,
                    "info",
                    format!(
                        "Safety backup of current '{}' data saved (backup id {})",
                        database.name, b.id
                    ),
                )
                .await;
            }
            Err(e) => {
                dlog(
                    db,
                    rollback_deployment_id,
                    "error",
                    format!(
                        "Data restore skipped for '{}': safety backup of current data failed: {}",
                        database.name, e
                    ),
                )
                .await;
                continue;
            }
        }

        match restore_one(runtime.as_ref(), &database, snap).await {
            Ok(()) => {
                restored += 1;
                dlog(
                    db,
                    rollback_deployment_id,
                    "info",
                    format!("Database '{}' restored from snapshot", database.name),
                )
                .await;
            }
            Err(e) => {
                dlog(
                    db,
                    rollback_deployment_id,
                    "error",
                    format!(
                        "Restore of database '{}' failed: {}. Use the safety backup to recover \
                         the previous data; restart the app if it holds stale connections.",
                        database.name, e
                    ),
                )
                .await;
            }
        }
    }

    if restored > 0 {
        dlog(
            db,
            rollback_deployment_id,
            "info",
            "Data restore finished. Restart the app if it caches database connections or schema."
                .to_string(),
        )
        .await;
    }
    restored
}

async fn restore_one(
    runtime: &dyn ContainerRuntime,
    database: &ManagedDatabase,
    snap: &DeploymentSnapshot,
) -> Result<()> {
    let container_id = database
        .container_id
        .as_deref()
        .context("Database has no container")?;
    let creds = database
        .get_credentials()
        .context("Database has no credentials")?;
    let path = snap.file_path.as_deref().context("Snapshot has no file")?;
    let bytes = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read snapshot file {path}"))?;

    upload_dump(runtime, container_id, &bytes).await?;
    let result = runtime
        .run_command(container_id, restore_command(&database.db_type, &creds)?)
        .await;

    // Always remove the temp files, whatever the outcome.
    let _ = runtime
        .run_command(
            container_id,
            vec![
                "rm".into(),
                "-f".into(),
                RESTORE_B64_PATH.into(),
                RESTORE_SQL_PATH.into(),
            ],
        )
        .await;

    let result = result?;
    if result.exit_code != 0 {
        anyhow::bail!(
            "restore command exited {}: {}",
            result.exit_code,
            result.stderr.trim()
        );
    }
    Ok(())
}

/// Write `bytes` to `RESTORE_SQL_PATH` inside the container via base64 chunks.
async fn upload_dump(
    runtime: &dyn ContainerRuntime,
    container_id: &str,
    bytes: &[u8],
) -> Result<()> {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

    let sh = |script: String| vec!["sh".to_string(), "-c".to_string(), script];
    let r = runtime
        .run_command(container_id, sh(format!(": > {RESTORE_B64_PATH}")))
        .await?;
    if r.exit_code != 0 {
        anyhow::bail!("Failed to prepare restore file: {}", r.stderr.trim());
    }
    for chunk in encoded.as_bytes().chunks(UPLOAD_CHUNK_CHARS) {
        // base64 output is ASCII, so this never fails.
        let chunk = std::str::from_utf8(chunk).unwrap_or_default();
        let r = runtime
            .run_command(
                container_id,
                sh(format!("printf %s '{chunk}' >> {RESTORE_B64_PATH}")),
            )
            .await?;
        if r.exit_code != 0 {
            anyhow::bail!("Failed to upload dump chunk: {}", r.stderr.trim());
        }
    }
    let r = runtime
        .run_command(
            container_id,
            sh(format!("base64 -d {RESTORE_B64_PATH} > {RESTORE_SQL_PATH}")),
        )
        .await?;
    if r.exit_code != 0 {
        anyhow::bail!("Failed to decode dump in container: {}", r.stderr.trim());
    }
    Ok(())
}

/// Command that loads `RESTORE_SQL_PATH` over the current data.
/// Postgres: the public schema is dropped and the dump replayed in one transaction with
/// ON_ERROR_STOP, so a failed restore leaves the data as it was. MySQL/MariaDB dumps
/// carry DROP TABLE IF EXISTS, but tables created after the snapshot are left in place.
pub fn restore_command(db_type: &str, creds: &DatabaseCredentials) -> Result<Vec<String>> {
    let sh = |script: String| vec!["sh".to_string(), "-c".to_string(), script];
    match db_type {
        "postgres" => {
            let name = creds
                .database
                .clone()
                .unwrap_or_else(|| "postgres".to_string());
            Ok(sh(format!(
                "{{ echo 'BEGIN; DROP SCHEMA public CASCADE; CREATE SCHEMA public;'; \
                 cat {RESTORE_SQL_PATH}; echo 'COMMIT;'; }} | \
                 PGPASSWORD='{}' psql -q -v ON_ERROR_STOP=1 -U {} -d {}",
                creds.password, creds.username, name
            )))
        }
        "mysql" | "mariadb" => {
            let name = creds
                .database
                .clone()
                .unwrap_or_else(|| creds.username.clone());
            let password = creds
                .root_password
                .clone()
                .unwrap_or_else(|| creds.password.clone());
            let client = if db_type == "mysql" {
                "mysql"
            } else {
                "mariadb"
            };
            Ok(sh(format!(
                "{client} -u root -p'{password}' {name} < {RESTORE_SQL_PATH}"
            )))
        }
        other => anyhow::bail!("Snapshot restore is not supported for {other}"),
    }
}

/// Deployment ids whose snapshots should be pruned: per app, everything beyond the
/// `keep` most recent deployments, plus snapshots whose deployment no longer exists
/// (`started_at` is None). `keep == 0` prunes nothing.
pub fn select_deployments_to_prune(
    entries: &[(String, String, Option<String>)],
    keep: i64,
) -> Vec<String> {
    if keep <= 0 {
        return Vec::new();
    }
    let mut by_app: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    let mut prune: Vec<String> = Vec::new();
    for (app_id, deployment_id, started_at) in entries {
        match started_at {
            Some(ts) => by_app
                .entry(app_id.as_str())
                .or_default()
                .push((deployment_id.as_str(), ts.as_str())),
            None => prune.push(deployment_id.clone()),
        }
    }
    for (_, mut deployments) in by_app {
        deployments.sort_by(|a, b| b.1.cmp(a.1).then_with(|| b.0.cmp(a.0)));
        deployments.dedup_by(|a, b| a.0 == b.0);
        prune.extend(
            deployments
                .iter()
                .skip(keep as usize)
                .map(|d| d.0.to_string()),
        );
    }
    prune.sort();
    prune.dedup();
    prune
}

/// Delete snapshot rows and dump files beyond the retention window, in small batches.
/// Returns the number of snapshots removed.
pub async fn prune_deployment_snapshots(db: &DbPool, keep: i64) -> u64 {
    if keep <= 0 {
        return 0;
    }
    let entries: Vec<(String, String, Option<String>)> = match sqlx::query_as(
        "SELECT DISTINCT s.app_id, s.deployment_id, d.started_at \
         FROM deployment_db_snapshots s LEFT JOIN deployments d ON d.id = s.deployment_id",
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to list deployment snapshots for retention");
            return 0;
        }
    };

    let to_prune = select_deployments_to_prune(&entries, keep);
    let mut removed: u64 = 0;
    for batch in to_prune.chunks(PRUNE_BATCH) {
        for deployment_id in batch {
            let rows: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM deployment_db_snapshots WHERE deployment_id = ?",
            )
            .bind(deployment_id)
            .fetch_one(db)
            .await
            .unwrap_or(0);
            let backups: Vec<(String, Option<String>)> = sqlx::query_as(
                "SELECT b.id, b.file_path FROM deployment_db_snapshots s \
                 JOIN database_backups b ON b.id = s.backup_id WHERE s.deployment_id = ?",
            )
            .bind(deployment_id)
            .fetch_all(db)
            .await
            .unwrap_or_default();
            for (backup_id, file_path) in backups {
                remove_backup(db, &backup_id, file_path.as_deref()).await;
            }
            // Clear rows whose backup row was already gone.
            if let Err(e) =
                sqlx::query("DELETE FROM deployment_db_snapshots WHERE deployment_id = ?")
                    .bind(deployment_id)
                    .execute(db)
                    .await
            {
                tracing::warn!(error = %e, "Snapshot retention cleanup failed");
            }
            removed += rows as u64;
        }
        tokio::time::sleep(PRUNE_PAUSE).await;
    }
    if removed > 0 {
        tracing::info!(removed, "Pruned deployment snapshots past retention");
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn snap(db: &str, db_type: &str) -> DeploymentSnapshot {
        DeploymentSnapshot {
            id: format!("s-{db}"),
            deployment_id: "d2".into(),
            app_id: "a1".into(),
            source_deployment_id: Some("d1".into()),
            database_id: db.into(),
            database_name: format!("name-{db}"),
            db_type: db_type.into(),
            backup_id: format!("b-{db}"),
            size_bytes: 1024,
            created_at: "2026-10-01T00:00:00+00:00".into(),
            file_path: Some(format!("/data/{db}.sql")),
            backup_status: "completed".into(),
            database_status: "running".into(),
        }
    }

    fn e(app: &str, dep: &str, ts: Option<&str>) -> (String, String, Option<String>) {
        (app.into(), dep.into(), ts.map(String::from))
    }

    #[test]
    fn restore_requires_a_snapshot() {
        let err = check_restore_preconditions(&[], |_| true).unwrap_err();
        assert!(err.contains("no database snapshot exists"));
    }

    #[test]
    fn restore_preconditions_accept_good_snapshot() {
        assert!(check_restore_preconditions(&[snap("a", "postgres")], |_| true).is_ok());
    }

    #[test]
    fn restore_preconditions_reject_bad_snapshots() {
        let mut s = snap("a", "redis");
        assert!(check_restore_preconditions(&[s.clone()], |_| true)
            .unwrap_err()
            .contains("not supported"));

        s = snap("a", "postgres");
        s.backup_status = "failed".into();
        assert!(check_restore_preconditions(&[s], |_| true)
            .unwrap_err()
            .contains("not complete"));

        assert!(
            check_restore_preconditions(&[snap("a", "mysql")], |_| false)
                .unwrap_err()
                .contains("missing on disk")
        );

        let mut s = snap("a", "mariadb");
        s.file_path = None;
        assert!(check_restore_preconditions(&[s], |_| true)
            .unwrap_err()
            .contains("missing on disk"));

        let mut s = snap("a", "postgres");
        s.size_bytes = MAX_SNAPSHOT_BYTES + 1;
        assert!(check_restore_preconditions(&[s], |_| true)
            .unwrap_err()
            .contains("too large"));

        let mut s = snap("a", "postgres");
        s.database_status = "stopped".into();
        assert!(check_restore_preconditions(&[s], |_| true)
            .unwrap_err()
            .contains("must be running"));
    }

    #[test]
    fn one_bad_snapshot_blocks_the_whole_restore() {
        let ok = snap("a", "postgres");
        let mut bad = snap("b", "postgres");
        bad.database_status = "stopped".into();
        assert!(check_restore_preconditions(&[ok, bad], |_| true).is_err());
    }

    #[test]
    fn keeps_newest_snapshot_per_database() {
        let mut older = snap("a", "postgres");
        older.id = "old".into();
        let mut newer = snap("a", "postgres");
        newer.id = "new".into();
        let other = snap("b", "mysql");
        // Input is newest first.
        let out = latest_per_database(vec![newer, older, other]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].id, "new");
    }

    #[test]
    fn prune_keeps_last_n_deployments_per_app() {
        let entries = vec![
            e("a1", "d1", Some("2026-10-01T00:00:00+00:00")),
            e("a1", "d2", Some("2026-10-02T00:00:00+00:00")),
            e("a1", "d3", Some("2026-10-03T00:00:00+00:00")),
            e("a1", "d4", Some("2026-10-04T00:00:00+00:00")),
            e("a2", "x1", Some("2026-10-01T00:00:00+00:00")),
            e("a2", "x2", Some("2026-10-02T00:00:00+00:00")),
        ];
        assert_eq!(select_deployments_to_prune(&entries, 2), vec!["d1", "d2"]);
        assert!(select_deployments_to_prune(&entries, 3).contains(&"d1".to_string()));
        assert_eq!(
            select_deployments_to_prune(&entries, 10),
            Vec::<String>::new()
        );
    }

    #[test]
    fn prune_zero_disables_and_orphans_go_first() {
        let entries = vec![
            e("a1", "d1", Some("2026-10-01T00:00:00+00:00")),
            e("a1", "gone", None),
        ];
        assert!(select_deployments_to_prune(&entries, 0).is_empty());
        assert_eq!(select_deployments_to_prune(&entries, 5), vec!["gone"]);
    }

    #[test]
    fn restore_commands() {
        let creds = DatabaseCredentials {
            username: "app".into(),
            password: "pw".into(),
            database: Some("appdb".into()),
            root_password: Some("rootpw".into()),
        };
        let pg = restore_command("postgres", &creds).unwrap().join(" ");
        assert!(pg.contains("ON_ERROR_STOP=1"));
        assert!(pg.contains("DROP SCHEMA public CASCADE"));
        assert!(pg.contains("-d appdb"));
        let my = restore_command("mysql", &creds).unwrap().join(" ");
        assert!(my.contains("mysql -u root -p'rootpw' appdb <"));
        let ma = restore_command("mariadb", &creds).unwrap().join(" ");
        assert!(ma.contains("mariadb -u root"));
        assert!(restore_command("redis", &creds).is_err());
    }

    #[test]
    fn only_text_dump_types_are_supported() {
        for t in ["postgres", "mysql", "mariadb"] {
            assert!(snapshot_supported(t));
        }
        for t in ["mongodb", "redis", "clickhouse", "dragonfly", "keydb"] {
            assert!(!snapshot_supported(t));
        }
    }

    async fn test_db() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for ddl in [
            "CREATE TABLE databases (id TEXT PRIMARY KEY, name TEXT NOT NULL, db_type TEXT NOT NULL, status TEXT NOT NULL)",
            "CREATE TABLE database_backups (id TEXT PRIMARY KEY, database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE, backup_type TEXT NOT NULL, status TEXT NOT NULL, file_path TEXT)",
            "CREATE TABLE deployments (id TEXT PRIMARY KEY, app_id TEXT NOT NULL, started_at TEXT)",
            "CREATE TABLE deployment_db_snapshots (id TEXT PRIMARY KEY, deployment_id TEXT NOT NULL, app_id TEXT NOT NULL, source_deployment_id TEXT, database_id TEXT NOT NULL REFERENCES databases(id) ON DELETE CASCADE, backup_id TEXT NOT NULL REFERENCES database_backups(id) ON DELETE CASCADE, size_bytes INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO databases VALUES ('db1', 'main', 'postgres', 'running')")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    async fn add_snapshot(db: &DbPool, app: &str, dep: &str, source: Option<&str>, n: u32) {
        let backup_id = format!("b-{dep}-{n}");
        sqlx::query("INSERT INTO database_backups VALUES (?, 'db1', 'pre_deploy', 'completed', ?)")
            .bind(&backup_id)
            .bind(format!("/nonexistent/{backup_id}.sql"))
            .execute(db)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO deployment_db_snapshots \
             (id, deployment_id, app_id, source_deployment_id, database_id, backup_id, size_bytes) \
             VALUES (?, ?, ?, ?, 'db1', ?, 10)",
        )
        .bind(format!("s-{dep}-{n}"))
        .bind(dep)
        .bind(app)
        .bind(source)
        .bind(&backup_id)
        .execute(db)
        .await
        .unwrap();
    }

    async fn count(db: &DbPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn loads_snapshots_by_source_deployment() {
        let db = test_db().await;
        add_snapshot(&db, "a1", "d2", Some("d1"), 0).await;
        add_snapshot(&db, "a1", "d3", Some("d2"), 0).await;
        add_snapshot(&db, "a1", "d4", Some("d1"), 0).await;

        let for_d1 = load_restorable_snapshots(&db, "d1").await.unwrap();
        // Two snapshots share source d1 but are for the same database: newest wins.
        assert_eq!(for_d1.len(), 1);
        assert_eq!(for_d1[0].deployment_id, "d4");
        assert_eq!(for_d1[0].database_name, "main");
        assert!(load_restorable_snapshots(&db, "unknown")
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn prune_removes_old_and_orphaned_snapshots() {
        let db = test_db().await;
        for (i, dep) in ["d1", "d2", "d3", "d4"].iter().enumerate() {
            sqlx::query("INSERT INTO deployments VALUES (?, 'a1', ?)")
                .bind(dep)
                .bind(format!("2026-10-0{}T00:00:00+00:00", i + 1))
                .execute(&db)
                .await
                .unwrap();
            add_snapshot(&db, "a1", dep, None, 0).await;
        }
        // Snapshot whose deployment row was trimmed away.
        add_snapshot(&db, "a1", "trimmed", None, 0).await;

        let removed = prune_deployment_snapshots(&db, 2).await;
        assert_eq!(removed, 3);
        assert_eq!(count(&db, "deployment_db_snapshots").await, 2);
        // Dump records go with their snapshots.
        assert_eq!(count(&db, "database_backups").await, 2);
        let kept: Vec<String> = sqlx::query_scalar(
            "SELECT deployment_id FROM deployment_db_snapshots ORDER BY deployment_id",
        )
        .fetch_all(&db)
        .await
        .unwrap();
        assert_eq!(kept, vec!["d3", "d4"]);
    }

    #[tokio::test]
    async fn prune_zero_keeps_everything() {
        let db = test_db().await;
        add_snapshot(&db, "a1", "trimmed", None, 0).await;
        assert_eq!(prune_deployment_snapshots(&db, 0).await, 0);
        assert_eq!(count(&db, "deployment_db_snapshots").await, 1);
    }
}
