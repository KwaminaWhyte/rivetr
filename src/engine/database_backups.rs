//! Database backup scheduling module
//!
//! This module handles automatic database backups:
//! - Runs as a background task checking for due backups
//! - Executes backup commands based on database type
//! - Manages backup retention (cleanup of old backups)
//! - Stores backup metadata in the database

use crate::config::DatabaseBackupConfig;
use crate::db::{
    BackupStatus, BackupType, DatabaseBackup, DatabaseBackupSchedule, DatabaseCredentials,
    ManagedDatabase,
};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Scratch path for an RDB dump inside a Redis-family container.
pub(crate) const RDB_TMP_PATH: &str = "/tmp/rivetr_backup.rdb";

/// Single-quote `value` for a POSIX shell.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Shell prelude for Redis-family exec commands. Picks `redis-cli` or `keydb-cli` into
/// `$CLI` and enables AUTH only when the server demands it (the managed containers are
/// usually started without a password, and AUTH against a password-less server errors).
pub(crate) fn redis_cli_prelude(password: &str) -> String {
    format!(
        "CLI=$(command -v redis-cli || command -v keydb-cli) || \
         {{ echo 'neither redis-cli nor keydb-cli found in this container' >&2; exit 127; }}; \
         case \"$($CLI PING 2>&1)\" in *NOAUTH*) export REDISCLI_AUTH={};; esac; ",
        shell_quote(password)
    )
}

/// True when `bytes` starts with the RDB magic.
pub(crate) fn is_rdb(bytes: &[u8]) -> bool {
    bytes.starts_with(b"REDIS")
}

/// Handles database backup operations
pub struct DatabaseBackupTask {
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: DatabaseBackupConfig,
    data_dir: PathBuf,
}

impl DatabaseBackupTask {
    pub fn new(
        db: DbPool,
        runtime: Arc<dyn ContainerRuntime>,
        config: DatabaseBackupConfig,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            runtime,
            config,
            data_dir,
        }
    }

    /// Run a single backup check cycle
    pub async fn run_backup_cycle(&self) -> Result<BackupStats> {
        let mut stats = BackupStats::default();

        if !self.config.enabled {
            debug!("Database backup scheduling is disabled, skipping");
            return Ok(stats);
        }

        // Get all enabled schedules that are due
        let due_schedules: Vec<DatabaseBackupSchedule> = sqlx::query_as(
            r#"
            SELECT id, database_id, enabled, schedule_type, schedule_hour, schedule_day,
                   retention_count, last_run_at, next_run_at, created_at, updated_at
            FROM database_backup_schedules
            WHERE enabled = 1
              AND next_run_at IS NOT NULL
              AND datetime(next_run_at) <= datetime('now')
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        if due_schedules.is_empty() {
            debug!("No scheduled backups are due");
            return Ok(stats);
        }

        info!(count = due_schedules.len(), "Found scheduled backups due");

        for schedule in due_schedules {
            stats.schedules_checked += 1;

            match self.run_scheduled_backup(&schedule).await {
                Ok(_) => {
                    stats.backups_completed += 1;
                    info!(
                        database_id = %schedule.database_id,
                        "Scheduled backup completed successfully"
                    );
                }
                Err(e) => {
                    stats.backups_failed += 1;
                    error!(
                        database_id = %schedule.database_id,
                        error = %e,
                        "Scheduled backup failed"
                    );
                }
            }

            // Clean up old backups based on retention
            if let Err(e) = self
                .cleanup_old_backups(&schedule.database_id, schedule.retention_count as usize)
                .await
            {
                warn!(
                    database_id = %schedule.database_id,
                    error = %e,
                    "Failed to cleanup old backups"
                );
            }
        }

        Ok(stats)
    }

    /// Run a scheduled backup for a database
    async fn run_scheduled_backup(&self, schedule: &DatabaseBackupSchedule) -> Result<()> {
        // Get the database
        let database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
            .bind(&schedule.database_id)
            .fetch_optional(&self.db)
            .await?
            .context("Database not found")?;

        // Check if database is running
        if database.status != "running" {
            warn!(
                database = %database.name,
                status = %database.status,
                "Skipping backup for non-running database"
            );
            // Still update the schedule to avoid repeated attempts
            self.update_schedule_next_run(&schedule.id).await?;
            return Ok(());
        }

        // Run the backup
        self.backup_database(&database, BackupType::Scheduled)
            .await?;

        // Update the schedule's next run time
        self.update_schedule_next_run(&schedule.id).await?;

        Ok(())
    }

    /// Backup a specific database
    pub async fn backup_database(
        &self,
        database: &ManagedDatabase,
        backup_type: BackupType,
    ) -> Result<DatabaseBackup> {
        let container_id = database
            .container_id
            .as_ref()
            .context("Database has no container")?;

        // Create backup record
        let mut backup = DatabaseBackup::new(&database.id, backup_type);

        // Insert the backup record
        sqlx::query(
            r#"
            INSERT INTO database_backups (id, database_id, backup_type, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&backup.id)
        .bind(&backup.database_id)
        .bind(&backup.backup_type)
        .bind(&backup.status)
        .bind(&backup.created_at)
        .bind(&backup.updated_at)
        .execute(&self.db)
        .await?;

        // Update status to running
        let started_at = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE database_backups SET status = ?, started_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(BackupStatus::Running.to_string())
        .bind(&started_at)
        .bind(&started_at)
        .bind(&backup.id)
        .execute(&self.db)
        .await?;
        backup.status = BackupStatus::Running.to_string();
        backup.started_at = Some(started_at);

        // Create backup directory
        let backup_dir = self
            .data_dir
            .join(&self.config.backup_dir)
            .join(&database.id);
        tokio::fs::create_dir_all(&backup_dir).await?;

        // Generate backup filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let (backup_format, extension) = match database.db_type.as_str() {
            "postgres" => ("sql", "sql"),
            "mysql" => ("sql", "sql"),
            "mariadb" => ("sql", "sql"),
            "mongodb" => ("archive", "archive"),
            "redis" | "dragonfly" | "keydb" => ("rdb", "rdb"),
            "clickhouse" => ("dump", "dump"),
            _ => ("dump", "dump"),
        };
        let backup_filename = format!("{}_{}.{}", database.name, timestamp, extension);
        let backup_path = backup_dir.join(&backup_filename);

        // Get credentials
        let creds = database
            .get_credentials()
            .context("Database has no credentials")?;

        // Execute backup command based on database type
        let result = match database.db_type.as_str() {
            "postgres" => {
                self.backup_postgres(container_id, &creds, &backup_path)
                    .await
            }
            "mysql" => self.backup_mysql(container_id, &creds, &backup_path).await,
            "mariadb" => {
                self.backup_mariadb(container_id, &creds, &backup_path)
                    .await
            }
            "mongodb" => {
                self.backup_mongodb(container_id, &creds, &backup_path)
                    .await
            }
            "redis" | "dragonfly" | "keydb" => {
                self.backup_redis(container_id, &creds, &backup_path).await
            }
            "clickhouse" => Err(anyhow::anyhow!(
                "Automated backup is not yet supported for ClickHouse"
            )),
            _ => Err(anyhow::anyhow!(
                "Unsupported database type: {}",
                database.db_type
            )),
        };

        let completed_at = chrono::Utc::now().to_rfc3339();

        match result {
            Ok(_) => {
                // Get file size
                let file_size = tokio::fs::metadata(&backup_path)
                    .await
                    .map(|m| m.len() as i64)
                    .unwrap_or(0);

                // Update backup record with success
                sqlx::query(
                    r#"
                    UPDATE database_backups
                    SET status = ?, file_path = ?, file_size = ?, backup_format = ?,
                        completed_at = ?, updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(BackupStatus::Completed.to_string())
                .bind(backup_path.to_string_lossy().to_string())
                .bind(file_size)
                .bind(backup_format)
                .bind(&completed_at)
                .bind(&completed_at)
                .bind(&backup.id)
                .execute(&self.db)
                .await?;

                backup.status = BackupStatus::Completed.to_string();
                backup.file_path = Some(backup_path.to_string_lossy().to_string());
                backup.file_size = Some(file_size);
                backup.backup_format = Some(backup_format.to_string());
                backup.completed_at = Some(completed_at);

                info!(
                    database = %database.name,
                    backup_id = %backup.id,
                    file_size = file_size,
                    "Database backup completed successfully"
                );
            }
            Err(e) => {
                // Update backup record with failure
                let error_msg = e.to_string();
                sqlx::query(
                    r#"
                    UPDATE database_backups
                    SET status = ?, error_message = ?, completed_at = ?, updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(BackupStatus::Failed.to_string())
                .bind(&error_msg)
                .bind(&completed_at)
                .bind(&completed_at)
                .bind(&backup.id)
                .execute(&self.db)
                .await?;

                backup.status = BackupStatus::Failed.to_string();
                backup.error_message = Some(error_msg.clone());
                backup.completed_at = Some(completed_at);

                return Err(e);
            }
        }

        Ok(backup)
    }

    /// Backup PostgreSQL database
    async fn backup_postgres(
        &self,
        container_id: &str,
        creds: &DatabaseCredentials,
        backup_path: &PathBuf,
    ) -> Result<()> {
        let cmd = [
            "pg_dump".to_string(),
            "-U".to_string(),
            creds.username.clone(),
            "-d".to_string(),
            creds
                .database
                .clone()
                .unwrap_or_else(|| "postgres".to_string()),
            "-f".to_string(),
            "/tmp/backup.sql".to_string(),
        ];

        // Set PGPASSWORD environment variable
        let full_cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("PGPASSWORD='{}' {}", creds.password, cmd.join(" ")),
        ];

        let result = self.runtime.run_command(container_id, full_cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "pg_dump failed with exit code {}: {}",
                result.exit_code,
                result.stderr
            ));
        }

        // Copy the backup file from container
        self.copy_from_container(container_id, "/tmp/backup.sql", backup_path)
            .await?;

        // Clean up temp file in container
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/tmp/backup.sql".to_string(),
                ],
            )
            .await;

        Ok(())
    }

    /// Backup MySQL database
    async fn backup_mysql(
        &self,
        container_id: &str,
        creds: &DatabaseCredentials,
        backup_path: &PathBuf,
    ) -> Result<()> {
        let database = creds
            .database
            .clone()
            .unwrap_or_else(|| creds.username.clone());
        let password = creds
            .root_password
            .clone()
            .unwrap_or_else(|| creds.password.clone());

        let cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "mysqldump -u root -p'{}' {} > /tmp/backup.sql",
                password, database
            ),
        ];

        let result = self.runtime.run_command(container_id, cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "mysqldump failed with exit code {}: {}",
                result.exit_code,
                result.stderr
            ));
        }

        // Copy the backup file from container
        self.copy_from_container(container_id, "/tmp/backup.sql", backup_path)
            .await?;

        // Clean up temp file in container
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/tmp/backup.sql".to_string(),
                ],
            )
            .await;

        Ok(())
    }

    /// Backup MariaDB database
    async fn backup_mariadb(
        &self,
        container_id: &str,
        creds: &DatabaseCredentials,
        backup_path: &PathBuf,
    ) -> Result<()> {
        let database = creds
            .database
            .clone()
            .unwrap_or_else(|| creds.username.clone());
        let password = creds
            .root_password
            .clone()
            .unwrap_or_else(|| creds.password.clone());

        let cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "mariadb-dump -u root -p'{}' {} > /tmp/backup.sql",
                password, database
            ),
        ];

        let result = self.runtime.run_command(container_id, cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "mariadb-dump failed with exit code {}: {}",
                result.exit_code,
                result.stderr
            ));
        }

        // Copy the backup file from container
        self.copy_from_container(container_id, "/tmp/backup.sql", backup_path)
            .await?;

        // Clean up temp file in container
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/tmp/backup.sql".to_string(),
                ],
            )
            .await;

        Ok(())
    }

    /// Backup MongoDB database
    async fn backup_mongodb(
        &self,
        container_id: &str,
        creds: &DatabaseCredentials,
        backup_path: &PathBuf,
    ) -> Result<()> {
        let database = creds
            .database
            .clone()
            .unwrap_or_else(|| "admin".to_string());

        let cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "mongodump --username {} --password {} --authenticationDatabase admin --db {} --archive=/tmp/backup.archive",
                creds.username, creds.password, database
            ),
        ];

        let result = self.runtime.run_command(container_id, cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "mongodump failed with exit code {}: {}",
                result.exit_code,
                result.stderr
            ));
        }

        // The archive is binary: copy it through base64, not as a UTF-8 string.
        self.copy_binary_from_container(container_id, "/tmp/backup.archive", backup_path)
            .await?;

        // Clean up temp file in container
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/tmp/backup.archive".to_string(),
                ],
            )
            .await;

        Ok(())
    }

    /// Backup a Redis-family database (redis, keydb) as an RDB file.
    ///
    /// `redis-cli --rdb` pulls a consistent RDB over the replication protocol, so there
    /// is no BGSAVE race and the data directory is not touched. The file is binary and
    /// is copied out through base64.
    async fn backup_redis(
        &self,
        container_id: &str,
        creds: &DatabaseCredentials,
        backup_path: &PathBuf,
    ) -> Result<()> {
        let script = format!(
            "{}rm -f {RDB_TMP_PATH}; $CLI --rdb {RDB_TMP_PATH}",
            redis_cli_prelude(&creds.password)
        );
        let result = self
            .runtime
            .run_command(container_id, vec!["sh".into(), "-c".into(), script])
            .await?;

        if result.exit_code != 0 {
            let _ = self
                .runtime
                .run_command(
                    container_id,
                    vec!["rm".into(), "-f".into(), RDB_TMP_PATH.into()],
                )
                .await;
            return Err(anyhow::anyhow!(
                "redis-cli --rdb failed with exit code {}: {}{}",
                result.exit_code,
                result.stderr.trim(),
                result.stdout.trim()
            ));
        }

        let copied = self
            .copy_binary_from_container(container_id, RDB_TMP_PATH, backup_path)
            .await;
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec!["rm".into(), "-f".into(), RDB_TMP_PATH.into()],
            )
            .await;
        copied?;

        let bytes = tokio::fs::read(backup_path).await?;
        if !is_rdb(&bytes) {
            let _ = tokio::fs::remove_file(backup_path).await;
            return Err(anyhow::anyhow!(
                "redis-cli --rdb produced a file that is not a valid RDB dump"
            ));
        }

        Ok(())
    }

    /// Copy a binary file from the container to the host. Exec output is captured as
    /// text, so the file travels as base64 and its decoded length is checked against
    /// `wc -c` to catch truncated output.
    async fn copy_binary_from_container(
        &self,
        container_id: &str,
        container_path: &str,
        host_path: &PathBuf,
    ) -> Result<()> {
        use base64::Engine as _;

        let size = self
            .runtime
            .run_command(
                container_id,
                vec![
                    "sh".into(),
                    "-c".into(),
                    format!("wc -c < '{container_path}'"),
                ],
            )
            .await?;
        if size.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to stat backup file in container: {}",
                size.stderr.trim()
            ));
        }
        let expected: usize = size
            .stdout
            .trim()
            .parse()
            .context("Could not parse backup file size")?;

        let result = self
            .runtime
            .run_command(container_id, vec!["base64".into(), container_path.into()])
            .await?;
        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to read backup file from container: {}",
                result.stderr.trim()
            ));
        }

        let compact: String = result
            .stdout
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(compact.as_bytes())
            .context("Backup file from container is not valid base64")?;
        if bytes.len() != expected {
            return Err(anyhow::anyhow!(
                "Backup file copy is incomplete ({} of {} bytes)",
                bytes.len(),
                expected
            ));
        }

        tokio::fs::write(host_path, bytes).await?;
        Ok(())
    }

    /// Copy a file from container to host
    async fn copy_from_container(
        &self,
        container_id: &str,
        container_path: &str,
        host_path: &PathBuf,
    ) -> Result<()> {
        // Use cat to read the file and pipe to host
        let cmd = vec!["cat".to_string(), container_path.to_string()];

        let result = self.runtime.run_command(container_id, cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to read backup file from container: {}",
                result.stderr
            ));
        }

        // Write stdout to file
        tokio::fs::write(host_path, result.stdout.as_bytes()).await?;

        Ok(())
    }

    /// Update schedule's next run time
    async fn update_schedule_next_run(&self, schedule_id: &str) -> Result<()> {
        let schedule: DatabaseBackupSchedule =
            sqlx::query_as("SELECT * FROM database_backup_schedules WHERE id = ?")
                .bind(schedule_id)
                .fetch_one(&self.db)
                .await?;

        let mut schedule = schedule;
        schedule.update_next_run();

        sqlx::query(
            r#"
            UPDATE database_backup_schedules
            SET last_run_at = ?, next_run_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&schedule.last_run_at)
        .bind(&schedule.next_run_at)
        .bind(&schedule.updated_at)
        .bind(schedule_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Cleanup old backups based on retention policy
    pub async fn cleanup_old_backups(
        &self,
        database_id: &str,
        retention_count: usize,
    ) -> Result<u64> {
        // Get all completed backups for this database, ordered by creation date
        let backups: Vec<DatabaseBackup> = sqlx::query_as(
            r#"
            SELECT id, database_id, backup_type, status, file_path, file_size,
                   backup_format, started_at, completed_at, error_message, created_at, updated_at
            FROM database_backups
            WHERE database_id = ? AND status = 'completed' AND backup_type != 'pre_deploy'
            ORDER BY created_at DESC
            "#,
        )
        .bind(database_id)
        .fetch_all(&self.db)
        .await?;

        if backups.len() <= retention_count {
            return Ok(0);
        }

        let mut deleted = 0u64;
        let to_delete = &backups[retention_count..];

        for backup in to_delete {
            // Delete the backup file
            if let Some(file_path) = &backup.file_path {
                if let Err(e) = tokio::fs::remove_file(file_path).await {
                    warn!(
                        backup_id = %backup.id,
                        file_path = %file_path,
                        error = %e,
                        "Failed to delete backup file"
                    );
                }
            }

            // Delete the backup record
            sqlx::query("DELETE FROM database_backups WHERE id = ?")
                .bind(&backup.id)
                .execute(&self.db)
                .await?;

            deleted += 1;
            debug!(
                backup_id = %backup.id,
                database_id = %database_id,
                "Deleted old backup"
            );
        }

        if deleted > 0 {
            info!(
                database_id = %database_id,
                deleted = deleted,
                retention = retention_count,
                "Cleaned up old backups"
            );
        }

        Ok(deleted)
    }
}

/// Statistics from a backup cycle
#[derive(Debug, Default)]
pub struct BackupStats {
    pub schedules_checked: u64,
    pub backups_completed: u64,
    pub backups_failed: u64,
}

/// Spawn the background backup task
pub fn spawn_database_backup_task(
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: DatabaseBackupConfig,
    data_dir: PathBuf,
) {
    if !config.enabled {
        info!("Database backup scheduling is disabled");
        return;
    }

    let interval_secs = config.check_interval_seconds;
    info!(
        interval_secs = interval_secs,
        backup_dir = %config.backup_dir,
        "Starting database backup scheduler"
    );

    let task = DatabaseBackupTask::new(db, runtime, config, data_dir);

    tokio::spawn(async move {
        // Wait a bit before the first check to let the system stabilize
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut tick = interval(Duration::from_secs(interval_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            if let Some(Err(e)) =
                crate::utils::supervise::guarded("database_backups", task.run_backup_cycle()).await
            {
                error!(error = %e, "Backup cycle failed");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn redis_prelude_only_authenticates_when_asked() {
        let script = redis_cli_prelude("p@ss w'rd");
        assert!(script.contains("redis-cli"));
        assert!(script.contains("keydb-cli"));
        assert!(script.contains("*NOAUTH*) export REDISCLI_AUTH='p@ss w'\\''rd';;"));
        // AUTH is never exported unconditionally.
        assert!(!script.starts_with("export"));
    }

    #[test]
    fn rdb_magic_is_checked() {
        assert!(is_rdb(b"REDIS0011\xfa"));
        assert!(!is_rdb(b"not an rdb"));
        assert!(!is_rdb(b""));
    }
}
