//! Backup jobs shared by databases, volumes and the instance: notifications,
//! missed-run detection, S3 upload/download with checksums, scheduled volume
//! backups and restore jobs (local or from S3).

pub mod database;
pub mod maintenance;
pub mod missed;
pub mod notify;
pub mod restore;
pub mod retention;
pub mod s3_store;
pub mod schedule;
pub mod volume;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::config::DatabaseBackupConfig;
use crate::runtime::ContainerRuntime;
use crate::{AppState, DbPool};

/// Everything a backup or restore job needs, cloned out of `AppState` so jobs can run
/// in background tasks.
#[derive(Clone)]
pub struct JobContext {
    pub db: DbPool,
    pub runtime: Arc<dyn ContainerRuntime>,
    pub data_dir: PathBuf,
    pub acme_cache_dir: PathBuf,
    /// rivetr.toml, included in instance backups
    pub config_path: PathBuf,
    pub db_backup: DatabaseBackupConfig,
    /// `auth.encryption_key`, used to decrypt stored S3 credentials
    pub encryption_secret: Option<String>,
}

impl JobContext {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
            runtime: state.runtime.clone(),
            data_dir: state.config.server.data_dir.clone(),
            acme_cache_dir: state.config.proxy.acme_cache_dir.clone(),
            config_path: PathBuf::from("rivetr.toml"),
            db_backup: state.config.database_backup.clone(),
            encryption_secret: state.config.auth.encryption_key.clone(),
        }
    }

    pub fn secret(&self) -> Option<&str> {
        self.encryption_secret.as_deref()
    }
}

/// How often due volume schedules are checked.
const VOLUME_SCHEDULE_INTERVAL_SECS: u64 = 60;
/// How often missed runs are looked for.
const MISSED_CHECK_INTERVAL_SECS: u64 = 300;

/// Start the volume backup scheduler and the missed-backup checker.
pub fn spawn_backup_jobs(ctx: JobContext) {
    tracing::info!("Starting volume backup scheduler and missed-backup checker");

    let schedules_ctx = ctx.clone();
    tokio::spawn(async move {
        restore::fail_interrupted(&schedules_ctx.db).await;
        tokio::time::sleep(Duration::from_secs(45)).await;
        let mut tick = tokio::time::interval(Duration::from_secs(VOLUME_SCHEDULE_INTERVAL_SECS));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            crate::utils::supervise::guarded(
                "volume_backup_scheduler",
                volume::run_due_schedules(&schedules_ctx),
            )
            .await;
        }
    });

    tokio::spawn(async move {
        // Give the schedulers a full cycle after startup before judging them.
        tokio::time::sleep(Duration::from_secs(120)).await;
        let grace = missed::grace_window(
            ctx.db_backup
                .check_interval_seconds
                .max(VOLUME_SCHEDULE_INTERVAL_SECS),
        );
        let mut tick = tokio::time::interval(Duration::from_secs(MISSED_CHECK_INTERVAL_SECS));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            crate::utils::supervise::guarded(
                "missed_backup_checker",
                missed::check_missed_backups(&ctx.db, grace),
            )
            .await;
        }
    });
}
