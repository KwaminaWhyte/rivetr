//! Log cleaner background task.
//!
//! Runs once per day (or on demand via API). For each app with a
//! `log_retention_policy`, deletes deployment_logs older than the
//! configured `retention_days`. Also enforces `max_size_mb` if set.

use crate::db::LogRetentionPolicy;
use crate::DbPool;
use std::time::Duration;
use tokio::time::interval;

/// Spawn the log cleaner background task (runs daily)
pub fn spawn_log_cleaner_task(db: DbPool) {
    tracing::info!("Starting log cleaner task (daily interval)");

    tokio::spawn(async move {
        // Wait 5 minutes before first run
        tokio::time::sleep(Duration::from_secs(300)).await;

        // Run every 24 hours
        let mut tick = interval(Duration::from_secs(86400));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            run_log_cleanup(&db).await;
        }
    });
}

/// Execute the log cleanup logic
pub async fn run_log_cleanup(db: &DbPool) {
    tracing::info!("Running log cleanup cycle");

    // Get all retention policies
    let policies: Vec<LogRetentionPolicy> = match sqlx::query_as(
        "SELECT id, app_id, retention_days, max_size_mb, created_at, updated_at FROM log_retention_policies",
    )
    .fetch_all(db)
    .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch log retention policies");
            return;
        }
    };

    let mut total_deleted: u64 = 0;

    for policy in &policies {
        // Time-based cleanup
        let cutoff = chrono::Utc::now() - chrono::Duration::days(policy.retention_days as i64);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        match sqlx::query(
            r#"
            DELETE FROM deployment_logs
            WHERE deployment_id IN (
                SELECT id FROM deployments WHERE app_id = ?
            ) AND timestamp < ?
            "#,
        )
        .bind(&policy.app_id)
        .bind(&cutoff_str)
        .execute(db)
        .await
        {
            Ok(result) => {
                let deleted = result.rows_affected();
                if deleted > 0 {
                    tracing::info!(
                        app_id = %policy.app_id,
                        deleted = deleted,
                        retention_days = policy.retention_days,
                        "Cleaned up logs by retention period"
                    );
                    total_deleted += deleted;
                }
            }
            Err(e) => {
                tracing::warn!(
                    app_id = %policy.app_id,
                    error = %e,
                    "Failed to clean up logs by retention"
                );
            }
        }

        // Size-based cleanup
        if let Some(max_size_mb) = policy.max_size_mb {
            enforce_max_size(db, &policy.app_id, max_size_mb as i64).await;
        }
    }

    // Default cleanup for apps without a policy (30-day default)
    let default_cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let default_cutoff_str = default_cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    match sqlx::query(
        r#"
        DELETE FROM deployment_logs
        WHERE deployment_id IN (
            SELECT d.id FROM deployments d
            LEFT JOIN log_retention_policies lrp ON lrp.app_id = d.app_id
            WHERE lrp.id IS NULL
        ) AND timestamp < ?
        "#,
    )
    .bind(&default_cutoff_str)
    .execute(db)
    .await
    {
        Ok(result) => {
            let deleted = result.rows_affected();
            if deleted > 0 {
                tracing::info!(
                    deleted = deleted,
                    "Cleaned up logs for apps using default retention"
                );
                total_deleted += deleted;
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to clean up default retention logs");
        }
    }

    if total_deleted > 0 {
        tracing::info!(total_deleted = total_deleted, "Log cleanup cycle completed");
    }
}

/// Enforce max_size_mb by deleting oldest logs if total exceeds the limit
async fn enforce_max_size(db: &DbPool, app_id: &str, max_size_mb: i64) {
    // Estimate total log size for this app (rough: length of message column)
    let total_bytes: Option<i64> = match sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT SUM(LENGTH(message))
        FROM deployment_logs
        WHERE deployment_id IN (SELECT id FROM deployments WHERE app_id = ?)
        "#,
    )
    .bind(app_id)
    .fetch_one(db)
    .await
    {
        Ok(val) => val,
        Err(e) => {
            tracing::warn!(app_id = %app_id, error = %e, "Failed to calculate log size");
            return;
        }
    };

    let total_bytes = total_bytes.unwrap_or(0);
    let max_bytes = max_size_mb * 1024 * 1024;

    if total_bytes <= max_bytes {
        return;
    }

    let excess = total_bytes - max_bytes;
    // Delete oldest logs until we're under the limit
    // Rough heuristic: delete logs in batches
    let avg_msg_size: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT AVG(LENGTH(message))
        FROM deployment_logs
        WHERE deployment_id IN (SELECT id FROM deployments WHERE app_id = ?)
        "#,
    )
    .bind(app_id)
    .fetch_one(db)
    .await
    .unwrap_or(Some(100));

    let avg_size = avg_msg_size.unwrap_or(100).max(1);
    let rows_to_delete = (excess / avg_size) + 100; // Delete a bit extra

    match sqlx::query(
        r#"
        DELETE FROM deployment_logs
        WHERE rowid IN (
            SELECT dl.rowid FROM deployment_logs dl
            INNER JOIN deployments d ON d.id = dl.deployment_id
            WHERE d.app_id = ?
            ORDER BY dl.timestamp ASC
            LIMIT ?
        )
        "#,
    )
    .bind(app_id)
    .bind(rows_to_delete)
    .execute(db)
    .await
    {
        Ok(result) => {
            tracing::info!(
                app_id = %app_id,
                deleted = result.rows_affected(),
                max_size_mb = max_size_mb,
                "Cleaned up logs by size limit"
            );
        }
        Err(e) => {
            tracing::warn!(
                app_id = %app_id,
                error = %e,
                "Failed to enforce log size limit"
            );
        }
    }
}
