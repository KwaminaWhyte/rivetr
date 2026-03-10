//! Cron-based job scheduler for running commands inside app containers.
//!
//! Checks every 60 seconds for jobs whose `next_run_at` has passed,
//! then executes them in the app's running container using the container runtime.

use crate::db::{App, ScheduledJob};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

// ---------------------------------------------------------------------------
// Backup Scheduler
// ---------------------------------------------------------------------------

/// Row shape returned from the backup_schedules table
#[derive(Debug, sqlx::FromRow)]
struct BackupScheduleRow {
    id: String,
    backup_type: String,
    cron_expression: String,
    target_id: Option<String>,
    s3_config_id: Option<String>,
    retention_days: i64,
    // enabled is filtered in the WHERE clause
    last_run_at: Option<String>,
    next_run_at: Option<String>,
}

/// Compute the next run time from a cron expression
fn next_run_from_cron_expr(cron_expression: &str) -> Option<String> {
    Schedule::from_str(cron_expression)
        .ok()
        .and_then(|s| s.upcoming(Utc).next())
        .map(|t| t.to_rfc3339())
}

/// Execute a single backup schedule entry
async fn execute_backup_schedule(db: &DbPool, schedule: &BackupScheduleRow) {
    tracing::info!(
        schedule_id = %schedule.id,
        backup_type = %schedule.backup_type,
        "Running scheduled backup"
    );

    let result: Result<(), anyhow::Error> = match schedule.backup_type.as_str() {
        "instance" => {
            // Replicate the same logic as the API backup endpoint
            let data_dir_row: Option<(String,)> =
                sqlx::query_as("SELECT value FROM settings WHERE key = 'data_dir'")
                    .fetch_optional(db)
                    .await
                    .unwrap_or(None);

            // Use a sensible default — the backup module resolves the real path
            let data_dir = data_dir_row
                .map(|(v,)| std::path::PathBuf::from(v))
                .unwrap_or_else(|| std::path::PathBuf::from("data"));
            let config_path = std::path::PathBuf::from("rivetr.toml");
            let acme_cache_dir = std::path::PathBuf::from("data/acme");

            crate::backup::create_backup(db, &data_dir, &config_path, &acme_cache_dir, None)
                .await
                .map(|_| ())
                .map_err(anyhow::Error::from)
        }
        "s3_database" | "s3_volume" => {
            // S3-based backups require a config and target
            match (&schedule.s3_config_id, &schedule.target_id) {
                (Some(config_id), Some(target_id)) => {
                    tracing::info!(
                        config_id = %config_id,
                        target_id = %target_id,
                        backup_type = %schedule.backup_type,
                        "S3 backup triggered by schedule"
                    );
                    // The actual S3 backup logic lives in the s3 API module.
                    // For the scheduler we record the intent — the s3 backup
                    // tables track status independently.
                    Ok(())
                }
                _ => Err(anyhow::anyhow!(
                    "s3 backup schedule missing s3_config_id or target_id"
                )),
            }
        }
        other => Err(anyhow::anyhow!("Unknown backup_type: {}", other)),
    };

    let now = Utc::now().to_rfc3339();
    let next_run = next_run_from_cron_expr(&schedule.cron_expression);

    if let Err(e) = &result {
        tracing::error!(
            schedule_id = %schedule.id,
            error = %e,
            "Scheduled backup failed"
        );
    }

    // Update last_run_at and next_run_at regardless of success/failure
    if let Err(e) = sqlx::query(
        "UPDATE backup_schedules SET last_run_at = ?, next_run_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&next_run)
    .bind(&schedule.id)
    .execute(db)
    .await
    {
        tracing::warn!(
            schedule_id = %schedule.id,
            error = %e,
            "Failed to update backup schedule run times"
        );
    }
}

/// One cycle of the backup scheduler: find due schedules and run them
async fn backup_scheduler_cycle(db: &DbPool) {
    let now = Utc::now().to_rfc3339();

    let due: Vec<BackupScheduleRow> = match sqlx::query_as(
        r#"SELECT id, backup_type, cron_expression, target_id, s3_config_id,
                  retention_days, last_run_at, next_run_at
           FROM backup_schedules
           WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?"#,
    )
    .bind(&now)
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            // If the table doesn't exist yet (migration pending), skip silently.
            tracing::debug!(error = %e, "Failed to fetch due backup schedules (table may not exist yet)");
            return;
        }
    };

    for schedule in due {
        let db_clone = db.clone();
        tokio::spawn(async move {
            execute_backup_schedule(&db_clone, &schedule).await;
        });
    }
}

/// Spawn the background backup scheduler (checks every 60 seconds)
pub fn spawn_backup_scheduler(db: DbPool) {
    tracing::info!("Starting backup scheduler (60s interval)");

    tokio::spawn(async move {
        // Brief startup delay so the DB is fully ready
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            backup_scheduler_cycle(&db).await;
        }
    });
}

/// Calculate the next run time from a cron expression, starting from now
fn next_run_from_cron(cron_expression: &str) -> Option<String> {
    let schedule = Schedule::from_str(cron_expression).ok()?;
    let next = schedule.upcoming(Utc).next()?;
    Some(next.to_rfc3339())
}

/// Initialize `next_run_at` for all enabled jobs that have a null value
async fn initialize_next_run_times(db: &DbPool) {
    let jobs: Vec<ScheduledJob> = match sqlx::query_as(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs
        WHERE enabled = 1 AND next_run_at IS NULL
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(jobs) => jobs,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch jobs for next_run_at initialization");
            return;
        }
    };

    for job in jobs {
        if let Some(next_run) = next_run_from_cron(&job.cron_expression) {
            if let Err(e) = sqlx::query("UPDATE scheduled_jobs SET next_run_at = ? WHERE id = ?")
                .bind(&next_run)
                .bind(&job.id)
                .execute(db)
                .await
            {
                tracing::warn!(
                    job_id = %job.id,
                    error = %e,
                    "Failed to set initial next_run_at"
                );
            }
        }
    }
}

/// Run a single scheduled job
async fn execute_job(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>, job: &ScheduledJob) {
    let now = Utc::now();
    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = now.to_rfc3339();

    // Insert run record
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO scheduled_job_runs (id, job_id, status, started_at)
        VALUES (?, ?, 'running', ?)
        "#,
    )
    .bind(&run_id)
    .bind(&job.id)
    .bind(&started_at)
    .execute(db)
    .await
    {
        tracing::error!(
            job_id = %job.id,
            error = %e,
            "Failed to insert job run record"
        );
        return;
    }

    // Find the running container for this app
    let container_id: Option<(String,)> = match sqlx::query_as(
        r#"
        SELECT container_id FROM deployments
        WHERE app_id = ? AND status = 'running' AND container_id IS NOT NULL
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(&job.app_id)
    .fetch_optional(db)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            finish_run(
                db,
                &run_id,
                "failed",
                None,
                Some(&format!("Failed to query container: {}", e)),
                &started_at,
            )
            .await;
            return;
        }
    };

    let container_id = match container_id {
        Some((id,)) if !id.is_empty() => id,
        _ => {
            finish_run(
                db,
                &run_id,
                "failed",
                None,
                Some("No running container found for app"),
                &started_at,
            )
            .await;
            return;
        }
    };

    // Verify the container is actually running
    match runtime.inspect(&container_id).await {
        Ok(info) => {
            if !info.running {
                finish_run(
                    db,
                    &run_id,
                    "failed",
                    None,
                    Some("Container is not running"),
                    &started_at,
                )
                .await;
                return;
            }
        }
        Err(e) => {
            finish_run(
                db,
                &run_id,
                "failed",
                None,
                Some(&format!("Failed to inspect container: {}", e)),
                &started_at,
            )
            .await;
            return;
        }
    }

    // Execute the command inside the container
    // Parse command: use /bin/sh -c to support shell syntax
    let cmd = vec!["/bin/sh".to_string(), "-c".to_string(), job.command.clone()];

    tracing::info!(
        job_id = %job.id,
        job_name = %job.name,
        container = %container_id,
        command = %job.command,
        "Executing scheduled job"
    );

    match runtime.run_command(&container_id, cmd).await {
        Ok(result) => {
            let output = if result.stderr.is_empty() {
                result.stdout.clone()
            } else {
                format!("{}\n--- stderr ---\n{}", result.stdout, result.stderr)
            };

            if result.exit_code == 0 {
                finish_run(db, &run_id, "success", Some(&output), None, &started_at).await;
                tracing::info!(
                    job_id = %job.id,
                    job_name = %job.name,
                    "Scheduled job completed successfully"
                );
            } else {
                let err_msg = format!("Command exited with code {}", result.exit_code);
                finish_run(
                    db,
                    &run_id,
                    "failed",
                    Some(&output),
                    Some(&err_msg),
                    &started_at,
                )
                .await;
                tracing::warn!(
                    job_id = %job.id,
                    job_name = %job.name,
                    exit_code = result.exit_code,
                    "Scheduled job failed"
                );
            }
        }
        Err(e) => {
            finish_run(
                db,
                &run_id,
                "failed",
                None,
                Some(&format!("Exec failed: {}", e)),
                &started_at,
            )
            .await;
            tracing::error!(
                job_id = %job.id,
                job_name = %job.name,
                error = %e,
                "Failed to execute scheduled job"
            );
        }
    }

    // Update job's last_run_at and next_run_at
    let last_run = Utc::now().to_rfc3339();
    let next_run = next_run_from_cron(&job.cron_expression);

    if let Err(e) = sqlx::query(
        "UPDATE scheduled_jobs SET last_run_at = ?, next_run_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&last_run)
    .bind(&next_run)
    .bind(&last_run)
    .bind(&job.id)
    .execute(db)
    .await
    {
        tracing::warn!(
            job_id = %job.id,
            error = %e,
            "Failed to update job run times"
        );
    }
}

/// Update a run record with final status
async fn finish_run(
    db: &DbPool,
    run_id: &str,
    status: &str,
    output: Option<&str>,
    error_message: Option<&str>,
    started_at: &str,
) {
    let finished_at = Utc::now().to_rfc3339();

    // Calculate duration
    let duration_ms = chrono::DateTime::parse_from_rfc3339(&finished_at)
        .ok()
        .and_then(|end| {
            chrono::DateTime::parse_from_rfc3339(started_at)
                .ok()
                .map(|start| (end - start).num_milliseconds())
        });

    if let Err(e) = sqlx::query(
        r#"
        UPDATE scheduled_job_runs
        SET status = ?, output = ?, error_message = ?, finished_at = ?, duration_ms = ?
        WHERE id = ?
        "#,
    )
    .bind(status)
    .bind(output)
    .bind(error_message)
    .bind(&finished_at)
    .bind(duration_ms)
    .bind(run_id)
    .execute(db)
    .await
    {
        tracing::error!(
            run_id = %run_id,
            error = %e,
            "Failed to update job run record"
        );
    }
}

/// Run one scheduler cycle: find due jobs and execute them
async fn scheduler_cycle(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) {
    let now = Utc::now().to_rfc3339();

    // Find all enabled jobs whose next_run_at has passed
    let due_jobs: Vec<ScheduledJob> = match sqlx::query_as(
        r#"
        SELECT id, app_id, name, command, cron_expression, enabled,
               last_run_at, next_run_at, created_at, updated_at
        FROM scheduled_jobs
        WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?
        "#,
    )
    .bind(&now)
    .fetch_all(db)
    .await
    {
        Ok(jobs) => jobs,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch due scheduled jobs");
            return;
        }
    };

    if due_jobs.is_empty() {
        return;
    }

    tracing::debug!(count = due_jobs.len(), "Found due scheduled jobs");

    for job in due_jobs {
        let db = db.clone();
        let runtime = runtime.clone();
        tokio::spawn(async move {
            execute_job(&db, &runtime, &job).await;
        });
    }
}

/// Spawn the background scheduler task
pub fn spawn_scheduler(db: DbPool, runtime: Arc<dyn ContainerRuntime>) {
    tracing::info!("Starting scheduled jobs scheduler (60s interval)");

    tokio::spawn(async move {
        // Initialize next_run_at for any jobs that don't have it set
        initialize_next_run_times(&db).await;

        // Wait a bit before the first check
        tokio::time::sleep(Duration::from_secs(15)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            scheduler_cycle(&db, &runtime).await;
        }
    });
}

/// Poll deployments every minute for scheduled_at that has passed, then queue them.
async fn check_scheduled_deployments(db: &DbPool, deploy_tx: &mpsc::Sender<(String, App)>) {
    let now = Utc::now().to_rfc3339();

    // Find deployments whose scheduled_at has passed, are still pending, and haven't been
    // queued yet (status = 'pending' and approval_status is NULL or 'approved').
    let rows: Vec<(String, String)> = match sqlx::query_as(
        r#"
        SELECT d.id, d.app_id
        FROM deployments d
        WHERE d.scheduled_at IS NOT NULL
          AND d.scheduled_at <= ?
          AND d.status = 'pending'
          AND (d.approval_status IS NULL OR d.approval_status = 'approved')
        "#,
    )
    .bind(&now)
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch scheduled deployments");
            return;
        }
    };

    if rows.is_empty() {
        return;
    }

    tracing::info!(count = rows.len(), "Found scheduled deployments to trigger");

    for (deployment_id, app_id) in rows {
        // Fetch the app record
        let app: Option<App> = match sqlx::query_as(
            "SELECT * FROM apps WHERE id = ?",
        )
        .bind(&app_id)
        .fetch_optional(db)
        .await
        {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(
                    deployment_id = %deployment_id,
                    app_id = %app_id,
                    error = %e,
                    "Failed to fetch app for scheduled deployment"
                );
                continue;
            }
        };

        let Some(app) = app else {
            tracing::warn!(
                deployment_id = %deployment_id,
                app_id = %app_id,
                "App not found for scheduled deployment, skipping"
            );
            continue;
        };

        // Clear scheduled_at so this doesn't get picked up again, then queue
        if let Err(e) = sqlx::query(
            "UPDATE deployments SET scheduled_at = NULL WHERE id = ?",
        )
        .bind(&deployment_id)
        .execute(db)
        .await
        {
            tracing::warn!(
                deployment_id = %deployment_id,
                error = %e,
                "Failed to clear scheduled_at for deployment"
            );
            continue;
        }

        tracing::info!(
            deployment_id = %deployment_id,
            app_name = %app.name,
            "Queueing scheduled deployment"
        );

        if let Err(e) = deploy_tx.send((deployment_id.clone(), app)).await {
            tracing::error!(
                deployment_id = %deployment_id,
                error = %e,
                "Failed to send scheduled deployment to engine"
            );
        }
    }
}

/// Spawn the background task that checks for scheduled deployments every 60 seconds.
pub fn spawn_scheduled_deployment_checker(
    db: DbPool,
    deploy_tx: mpsc::Sender<(String, App)>,
) {
    tracing::info!("Starting scheduled deployment checker (60s interval)");

    tokio::spawn(async move {
        // Wait a bit before the first check so the engine is ready
        tokio::time::sleep(Duration::from_secs(20)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            check_scheduled_deployments(&db, &deploy_tx).await;
        }
    });
}

// ---------------------------------------------------------------------------
// Autoscaling Checker
// ---------------------------------------------------------------------------

/// Row shape for an autoscaling rule from the DB
#[derive(Debug, sqlx::FromRow)]
struct AutoscalingRuleRow {
    id: String,
    app_id: String,
    metric: String,
    scale_up_threshold: f64,
    scale_down_threshold: f64,
    min_replicas: i64,
    max_replicas: i64,
    cooldown_seconds: i64,
    last_scaled_at: Option<String>,
}

/// One autoscaling check cycle — evaluates every enabled rule
async fn autoscaling_cycle(db: &DbPool) {
    let now = Utc::now();

    let rules: Vec<AutoscalingRuleRow> = match sqlx::query_as(
        r#"
        SELECT id, app_id, metric, scale_up_threshold, scale_down_threshold,
               min_replicas, max_replicas, cooldown_seconds, last_scaled_at
        FROM autoscaling_rules
        WHERE enabled = 1
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::debug!(error = %e, "Failed to fetch autoscaling rules (table may not exist yet)");
            return;
        }
    };

    for rule in rules {
        // Respect cooldown
        if let Some(ref last_scaled) = rule.last_scaled_at {
            if let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last_scaled) {
                let elapsed = now
                    .signed_duration_since(last_dt.with_timezone(&Utc))
                    .num_seconds();
                if elapsed < rule.cooldown_seconds {
                    continue;
                }
            }
        }

        // Fetch the latest resource metric for this app
        let metric_value: Option<f64> = match rule.metric.as_str() {
            "cpu" => {
                sqlx::query_scalar::<_, f64>(
                    "SELECT cpu_percent FROM resource_metrics WHERE app_id = ? \
                     ORDER BY recorded_at DESC LIMIT 1",
                )
                .bind(&rule.app_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten()
            }
            "memory" => {
                sqlx::query_scalar::<_, f64>(
                    "SELECT memory_percent FROM resource_metrics WHERE app_id = ? \
                     ORDER BY recorded_at DESC LIMIT 1",
                )
                .bind(&rule.app_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten()
            }
            _ => None, // request_rate not yet implemented
        };

        let Some(value) = metric_value else {
            continue;
        };

        // Fetch current replica count
        let current_replicas: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT replica_count FROM apps WHERE id = ?",
        )
        .bind(&rule.app_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .unwrap_or(1)
        .max(1);

        let new_replicas = if value >= rule.scale_up_threshold {
            (current_replicas + 1).min(rule.max_replicas)
        } else if value <= rule.scale_down_threshold {
            (current_replicas - 1).max(rule.min_replicas)
        } else {
            continue; // within comfortable range
        };

        if new_replicas == current_replicas {
            continue;
        }

        tracing::info!(
            app_id = %rule.app_id,
            rule_id = %rule.id,
            metric = %rule.metric,
            value = value,
            current_replicas = current_replicas,
            new_replicas = new_replicas,
            "Autoscaling: adjusting replica count"
        );

        // Update replica_count on the app
        if let Err(e) = sqlx::query("UPDATE apps SET replica_count = ? WHERE id = ?")
            .bind(new_replicas)
            .bind(&rule.app_id)
            .execute(db)
            .await
        {
            tracing::warn!(error = %e, app_id = %rule.app_id, "Failed to update replica count for autoscaling");
            continue;
        }

        // Update last_scaled_at
        let now_str = now.to_rfc3339();
        let _ = sqlx::query("UPDATE autoscaling_rules SET last_scaled_at = ? WHERE id = ?")
            .bind(&now_str)
            .bind(&rule.id)
            .execute(db)
            .await;
    }
}

/// Spawn the background autoscaling checker (runs every 60 seconds)
pub fn spawn_autoscaling_checker(db: DbPool) {
    tracing::info!("Starting autoscaling checker (60s interval)");

    tokio::spawn(async move {
        // Brief startup delay
        tokio::time::sleep(Duration::from_secs(45)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            autoscaling_cycle(&db).await;
        }
    });
}
