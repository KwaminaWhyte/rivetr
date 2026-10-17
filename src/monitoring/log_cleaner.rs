//! Log cleaner background task.
//!
//! Runs once per day (or on demand via API). For each app with a
//! `log_retention_policy`, deletes deployment_logs older than the
//! configured `retention_days`. Also enforces `max_size_mb` if set.
//!
//! The same daily cycle prunes the other unbounded operational tables
//! (`proxy_logs`, `webhook_events`, audit logs, `scheduled_job_runs`, expired
//! `sessions`) according to `[data_retention]`. Deletes run in bounded batches
//! so a single run never holds the SQLite write lock for long.

use crate::config::DataRetentionConfig;
use crate::db::LogRetentionPolicy;
use crate::DbPool;
use std::time::Duration;
use tokio::time::interval;

/// Rows deleted per statement; keeps each write transaction short.
const DELETE_BATCH_SIZE: i64 = 10_000;

/// Pause between batches so API and pipeline writers can interleave.
const BATCH_PAUSE: Duration = Duration::from_millis(50);

/// Spawn the log cleaner background task (runs daily)
pub fn spawn_log_cleaner_task(db: DbPool, retention: DataRetentionConfig) {
    tracing::info!("Starting log cleaner task (daily interval)");

    tokio::spawn(async move {
        // Wait 5 minutes before first run
        tokio::time::sleep(Duration::from_secs(300)).await;

        // Run every 24 hours
        let mut tick = interval(Duration::from_secs(86400));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            crate::utils::supervise::guarded("log_cleaner", run_log_cleanup(&db)).await;
            crate::utils::supervise::guarded("data_retention", run_data_retention(&db, &retention))
                .await;
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

/// Prune high-volume operational tables according to `[data_retention]`.
/// Returns the total number of rows deleted.
pub async fn run_data_retention(db: &DbPool, cfg: &DataRetentionConfig) -> u64 {
    let mut total: u64 = 0;

    if cfg.proxy_logs_days > 0 {
        total += prune_older_than(db, "proxy_logs", "ts", cfg.proxy_logs_days).await;
    }
    if cfg.webhook_events_days > 0 {
        total +=
            prune_older_than(db, "webhook_events", "received_at", cfg.webhook_events_days).await;
    }
    if cfg.audit_logs_days > 0 {
        total += prune_older_than(db, "audit_logs", "created_at", cfg.audit_logs_days).await;
        total += prune_team_audit_logs(db, cfg.audit_logs_days).await;
    }
    if cfg.scheduled_job_runs_keep > 0 {
        total += prune_scheduled_job_runs(db, cfg.scheduled_job_runs_keep).await;
    }
    total += prune_expired_sessions(db).await;
    // Dump files are deleted along with their rows; 0 keeps everything.
    total += crate::engine::deployment_snapshots::prune_deployment_snapshots(
        db,
        cfg.deployment_snapshots_keep,
    )
    .await;

    if total > 0 {
        tracing::info!(total_deleted = total, "Data retention cycle completed");
    }
    total
}

/// Run `sql` repeatedly (each statement must delete at most `DELETE_BATCH_SIZE`
/// rows) until a batch deletes fewer rows than the batch size.
async fn delete_in_batches(db: &DbPool, label: &str, sql: &str, bind: Option<&str>) -> u64 {
    let mut total: u64 = 0;
    loop {
        let mut q = sqlx::query(sql);
        if let Some(b) = bind {
            q = q.bind(b);
        }
        match q.bind(DELETE_BATCH_SIZE).execute(db).await {
            Ok(r) => {
                let n = r.rows_affected();
                total += n;
                if (n as i64) < DELETE_BATCH_SIZE {
                    break;
                }
                tokio::time::sleep(BATCH_PAUSE).await;
            }
            Err(e) => {
                tracing::warn!(table = %label, error = %e, "Retention cleanup failed");
                break;
            }
        }
    }
    if total > 0 {
        tracing::info!(table = %label, deleted = total, "Pruned rows past retention");
    }
    total
}

/// Delete rows from a table whose `column` holds a `YYYY-MM-DD HH:MM:SS` timestamp
/// (SQLite `datetime('now')` format) older than `days`.
async fn prune_older_than(db: &DbPool, table: &str, column: &str, days: i64) -> u64 {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    // Table/column names are internal constants, never user input.
    let sql = format!(
        "DELETE FROM {table} WHERE rowid IN \
         (SELECT rowid FROM {table} WHERE {column} < ? LIMIT ?)"
    );
    delete_in_batches(db, table, &sql, Some(&cutoff)).await
}

/// `team_audit_logs.created_at` is written as RFC 3339, so compare against an
/// RFC 3339 cutoff rather than the SQLite datetime format.
async fn prune_team_audit_logs(db: &DbPool, days: i64) -> u64 {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    let sql = "DELETE FROM team_audit_logs WHERE rowid IN \
               (SELECT rowid FROM team_audit_logs WHERE created_at < ? LIMIT ?)";
    delete_in_batches(db, "team_audit_logs", sql, Some(&cutoff)).await
}

/// Delete sessions (including `oauth_state` rows) that expired more than a day
/// ago. `expires_at` is RFC 3339; the day of grace avoids any format edge cases.
async fn prune_expired_sessions(db: &DbPool) -> u64 {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();
    let sql = "DELETE FROM sessions WHERE rowid IN \
               (SELECT rowid FROM sessions WHERE expires_at < ? LIMIT ?)";
    delete_in_batches(db, "sessions", sql, Some(&cutoff)).await
}

/// Keep only the `keep` most recent runs for each scheduled job.
async fn prune_scheduled_job_runs(db: &DbPool, keep: i64) -> u64 {
    let job_ids: Vec<String> = match sqlx::query_scalar(
        "SELECT job_id FROM scheduled_job_runs GROUP BY job_id HAVING COUNT(*) > ?",
    )
    .bind(keep)
    .fetch_all(db)
    .await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to list scheduled job runs for retention");
            return 0;
        }
    };

    let mut total: u64 = 0;
    for job_id in job_ids {
        loop {
            // The OFFSET window always starts at the first stale row, so each
            // pass deletes the next batch of rows beyond the newest `keep`.
            let res = sqlx::query(
                "DELETE FROM scheduled_job_runs WHERE rowid IN \
                 (SELECT rowid FROM scheduled_job_runs WHERE job_id = ? \
                  ORDER BY started_at DESC LIMIT ? OFFSET ?)",
            )
            .bind(&job_id)
            .bind(DELETE_BATCH_SIZE)
            .bind(keep)
            .execute(db)
            .await;
            match res {
                Ok(r) => {
                    let n = r.rows_affected();
                    total += n;
                    if (n as i64) < DELETE_BATCH_SIZE {
                        break;
                    }
                    tokio::time::sleep(BATCH_PAUSE).await;
                }
                Err(e) => {
                    tracing::warn!(job_id = %job_id, error = %e, "Failed to prune scheduled job runs");
                    break;
                }
            }
        }
    }
    if total > 0 {
        tracing::info!(
            deleted = total,
            keep = keep,
            "Pruned scheduled_job_runs past retention"
        );
    }
    total
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_db() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for ddl in [
            "CREATE TABLE proxy_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, ts TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE webhook_events (id TEXT PRIMARY KEY, received_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE audit_logs (id TEXT PRIMARY KEY, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
            "CREATE TABLE team_audit_logs (id TEXT PRIMARY KEY, created_at TEXT NOT NULL)",
            "CREATE TABLE scheduled_job_runs (id TEXT PRIMARY KEY, job_id TEXT NOT NULL, started_at TEXT NOT NULL)",
            "CREATE TABLE sessions (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, expires_at TEXT NOT NULL)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn count(db: &DbPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(db)
            .await
            .unwrap()
    }

    fn ago_sql(days: i64) -> String {
        (chrono::Utc::now() - chrono::Duration::days(days))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    }

    fn ago_rfc(days: i64) -> String {
        (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339()
    }

    #[tokio::test]
    async fn prunes_old_rows_and_keeps_recent() {
        let db = test_db().await;
        for (i, days) in [1i64, 20, 40, 120].iter().enumerate() {
            sqlx::query("INSERT INTO proxy_logs (ts) VALUES (?)")
                .bind(ago_sql(*days))
                .execute(&db)
                .await
                .unwrap();
            sqlx::query("INSERT INTO webhook_events (id, received_at) VALUES (?, ?)")
                .bind(format!("w{i}"))
                .bind(ago_sql(*days))
                .execute(&db)
                .await
                .unwrap();
            sqlx::query("INSERT INTO audit_logs (id, created_at) VALUES (?, ?)")
                .bind(format!("a{i}"))
                .bind(ago_sql(*days))
                .execute(&db)
                .await
                .unwrap();
            sqlx::query("INSERT INTO team_audit_logs (id, created_at) VALUES (?, ?)")
                .bind(format!("t{i}"))
                .bind(ago_rfc(*days))
                .execute(&db)
                .await
                .unwrap();
        }

        let cfg = DataRetentionConfig::default(); // 14 / 30 / 90
        run_data_retention(&db, &cfg).await;

        assert_eq!(count(&db, "proxy_logs").await, 1);
        assert_eq!(count(&db, "webhook_events").await, 2);
        assert_eq!(count(&db, "audit_logs").await, 3);
        assert_eq!(count(&db, "team_audit_logs").await, 3);
    }

    #[tokio::test]
    async fn zero_disables_pruning() {
        let db = test_db().await;
        sqlx::query("INSERT INTO proxy_logs (ts) VALUES (?)")
            .bind(ago_sql(400))
            .execute(&db)
            .await
            .unwrap();
        let cfg = DataRetentionConfig {
            proxy_logs_days: 0,
            ..DataRetentionConfig::default()
        };
        run_data_retention(&db, &cfg).await;
        assert_eq!(count(&db, "proxy_logs").await, 1);
    }

    #[tokio::test]
    async fn keeps_last_n_runs_per_job() {
        let db = test_db().await;
        for job in ["j1", "j2"] {
            let n = if job == "j1" { 12 } else { 3 };
            for i in 0..n {
                sqlx::query(
                    "INSERT INTO scheduled_job_runs (id, job_id, started_at) VALUES (?, ?, ?)",
                )
                .bind(format!("{job}-{i}"))
                .bind(job)
                .bind(ago_rfc(n - i))
                .execute(&db)
                .await
                .unwrap();
            }
        }
        let deleted = prune_scheduled_job_runs(&db, 5).await;
        assert_eq!(deleted, 7);
        let j1: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_job_runs WHERE job_id = 'j1'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(j1, 5);
        // The newest run survives, the oldest is gone.
        let newest: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_job_runs WHERE id = 'j1-11'")
                .fetch_one(&db)
                .await
                .unwrap();
        let oldest: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scheduled_job_runs WHERE id = 'j1-0'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!((newest, oldest), (1, 0));
        assert_eq!(count(&db, "scheduled_job_runs").await, 8);
    }

    #[tokio::test]
    async fn deletes_expired_sessions_including_oauth_state() {
        let db = test_db().await;
        for (id, user, exp) in [
            ("s1", "u1", ago_rfc(-7)),
            ("s2", "u1", ago_rfc(3)),
            ("s3", "oauth_state", ago_rfc(2)),
            ("s4", "oauth_state", ago_rfc(-1)),
        ] {
            sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, ?)")
                .bind(id)
                .bind(user)
                .bind(exp)
                .execute(&db)
                .await
                .unwrap();
        }
        assert_eq!(prune_expired_sessions(&db).await, 2);
        assert_eq!(count(&db, "sessions").await, 2);
    }

    #[tokio::test]
    async fn batches_across_multiple_statements() {
        let db = test_db().await;
        let old = ago_sql(100);
        sqlx::query(
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 25000) \
             INSERT INTO proxy_logs (ts) SELECT ? FROM n",
        )
        .bind(&old)
        .execute(&db)
        .await
        .unwrap();
        let deleted = prune_older_than(&db, "proxy_logs", "ts", 14).await;
        assert_eq!(deleted, 25_000);
        assert_eq!(count(&db, "proxy_logs").await, 0);
    }
}
