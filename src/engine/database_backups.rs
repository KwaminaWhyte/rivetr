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
        let database: ManagedDatabase = sqlx::query_as(
            "SELECT * FROM databases WHERE id = ?",
        )
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
        self.backup_database(&database, BackupType::Scheduled).await?;

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
        sqlx::query("UPDATE database_backups SET status = ?, started_at = ?, updated_at = ? WHERE id = ?")
            .bind(BackupStatus::Running.to_string())
            .bind(&started_at)
            .bind(&started_at)
            .bind(&backup.id)
            .execute(&self.db)
            .await?;
        backup.status = BackupStatus::Running.to_string();
        backup.started_at = Some(started_at);

        // Create backup directory
        let backup_dir = self.data_dir.join(&self.config.backup_dir).join(&database.id);
        tokio::fs::create_dir_all(&backup_dir).await?;

        // Generate backup filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let (backup_format, extension) = match database.db_type.as_str() {
            "postgres" => ("sql", "sql"),
            "mysql" => ("sql", "sql"),
            "mongodb" => ("archive", "archive"),
            "redis" => ("rdb", "rdb"),
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
            "mysql" => {
                self.backup_mysql(container_id, &creds, &backup_path).await
            }
            "mongodb" => {
                self.backup_mongodb(container_id, &creds, &backup_path)
                    .await
            }
            "redis" => self.backup_redis(container_id, &backup_path).await,
            _ => Err(anyhow::anyhow!("Unsupported database type: {}", database.db_type)),
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
        let cmd = vec![
            "pg_dump".to_string(),
            "-U".to_string(),
            creds.username.clone(),
            "-d".to_string(),
            creds.database.clone().unwrap_or_else(|| "postgres".to_string()),
            "-f".to_string(),
            "/tmp/backup.sql".to_string(),
        ];

        // Set PGPASSWORD environment variable
        let full_cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!(
                "PGPASSWORD='{}' {}",
                creds.password,
                cmd.join(" ")
            ),
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
            .run_command(container_id, vec!["rm".to_string(), "-f".to_string(), "/tmp/backup.sql".to_string()])
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
        let database = creds.database.clone().unwrap_or_else(|| creds.username.clone());
        let password = creds.root_password.clone().unwrap_or_else(|| creds.password.clone());

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
            .run_command(container_id, vec!["rm".to_string(), "-f".to_string(), "/tmp/backup.sql".to_string()])
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
        let database = creds.database.clone().unwrap_or_else(|| "admin".to_string());

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

        // Copy the backup file from container
        self.copy_from_container(container_id, "/tmp/backup.archive", backup_path)
            .await?;

        // Clean up temp file in container
        let _ = self
            .runtime
            .run_command(
                container_id,
                vec!["rm".to_string(), "-f".to_string(), "/tmp/backup.archive".to_string()],
            )
            .await;

        Ok(())
    }

    /// Backup Redis database
    async fn backup_redis(&self, container_id: &str, backup_path: &PathBuf) -> Result<()> {
        // Trigger BGSAVE
        let cmd = vec!["redis-cli".to_string(), "BGSAVE".to_string()];
        let result = self.runtime.run_command(container_id, cmd).await?;

        if result.exit_code != 0 {
            return Err(anyhow::anyhow!(
                "redis-cli BGSAVE failed with exit code {}: {}",
                result.exit_code,
                result.stderr
            ));
        }

        // Wait for BGSAVE to complete
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check if save is complete
        let check_cmd = vec![
            "sh".to_string(),
            "-c".to_string(),
            "redis-cli LASTSAVE".to_string(),
        ];
        let _ = self.runtime.run_command(container_id, check_cmd).await?;

        // Copy the RDB file from container
        self.copy_from_container(container_id, "/data/dump.rdb", backup_path)
            .await?;

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
        let cmd = vec![
            "cat".to_string(),
            container_path.to_string(),
        ];

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
        let schedule: DatabaseBackupSchedule = sqlx::query_as(
            "SELECT * FROM database_backup_schedules WHERE id = ?",
        )
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
    pub async fn cleanup_old_backups(&self, database_id: &str, retention_count: usize) -> Result<u64> {
        // Get all completed backups for this database, ordered by creation date
        let backups: Vec<DatabaseBackup> = sqlx::query_as(
            r#"
            SELECT id, database_id, backup_type, status, file_path, file_size,
                   backup_format, started_at, completed_at, error_message, created_at, updated_at
            FROM database_backups
            WHERE database_id = ? AND status = 'completed'
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
            if let Err(e) = task.run_backup_cycle().await {
                error!(error = %e, "Backup cycle failed");
            }
        }
    });
}
