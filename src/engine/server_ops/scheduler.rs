//! Background loop that runs scheduled Docker cleanups and package checks.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::{
    cleanup, load_all_settings, patching, resolve_target, validate_cron, MaintenanceSettings,
    LOCALHOST_ID,
};
use crate::config::Config;
use crate::DbPool;

/// Did `cron` fire in the window `(since, now]`?
pub fn cron_due(cron: &str, since: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    validate_cron(cron)
        .ok()
        .and_then(|s| s.after(&since).next())
        .is_some_and(|next| next <= now)
}

/// Spawn the scheduler. Checks every minute.
pub fn spawn_server_ops_scheduler(db: DbPool, config: Arc<Config>) {
    tokio::spawn(async move {
        if let Err(e) = super::fail_interrupted_jobs(&db).await {
            tracing::warn!("Failed to mark interrupted server jobs: {}", e);
        }
        let mut last = Utc::now();
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let now = Utc::now();
            match load_all_settings(&db).await {
                Ok(all) => {
                    for s in all {
                        tick_server(&db, &config, &s, last, now).await;
                    }
                }
                Err(e) => tracing::warn!("Server maintenance scheduler: {}", e),
            }
            last = now;
        }
    });
}

async fn tick_server(
    db: &DbPool,
    config: &Config,
    s: &MaintenanceSettings,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) {
    let cleanup_due = s.cleanup_enabled && cron_due(&s.cleanup_cron, since, now);
    let check_due = s.patch_check_enabled && cron_due(&s.patch_check_cron, since, now);
    if !cleanup_due && !check_due {
        return;
    }

    let target = match resolve_target(db, config, &s.server_id).await {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            // Server was deleted: drop its settings.
            if s.server_id != LOCALHOST_ID {
                let _ = sqlx::query("DELETE FROM server_maintenance_settings WHERE server_id = ?")
                    .bind(&s.server_id)
                    .execute(db)
                    .await;
            }
            return;
        }
        Err(e) => {
            tracing::warn!(server_id = %s.server_id, "Scheduler could not resolve server: {}", e);
            return;
        }
    };

    if cleanup_due && !cleanup::is_running(db, &s.server_id).await.unwrap_or(true) {
        let trigger = if s.cleanup_trigger == "threshold" {
            match cleanup::disk_usage_percent(&target).await {
                Ok(pct) if (pct as i64) >= s.cleanup_threshold_pct => Some("threshold"),
                Ok(pct) => {
                    tracing::debug!(
                        server_id = %s.server_id,
                        pct,
                        threshold = s.cleanup_threshold_pct,
                        "Disk below cleanup threshold"
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!(server_id = %s.server_id, "Disk usage check failed: {}", e);
                    None
                }
            }
        } else {
            Some("schedule")
        };
        if let Some(trigger) = trigger {
            if let Err(e) = cleanup::start_run(
                db.clone(),
                s.server_id.clone(),
                target.clone(),
                cleanup::CleanupOptions::from(s),
                trigger,
            )
            .await
            {
                tracing::warn!(server_id = %s.server_id, "Scheduled cleanup failed to start: {}", e);
            }
        }
    }

    if check_due
        && patching::running_job(db, &s.server_id)
            .await
            .ok()
            .flatten()
            .is_none()
    {
        if let Err(e) = patching::start_job(
            db.clone(),
            s.server_id.clone(),
            target,
            patching::PatchJobKind::Check,
        )
        .await
        {
            tracing::warn!(server_id = %s.server_id, "Scheduled patch check failed to start: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cron_due_window() {
        let since = Utc.with_ymd_and_hms(2026, 1, 1, 2, 59, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap();
        assert!(cron_due("0 3 * * *", since, now));
        let later = Utc.with_ymd_and_hms(2026, 1, 1, 3, 1, 0).unwrap();
        assert!(!cron_due("0 3 * * *", now, later));
        assert!(!cron_due("garbage", since, now));
    }
}
