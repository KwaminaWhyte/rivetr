//! Cron-based job scheduler for running commands inside app containers.
//!
//! Checks every 60 seconds for jobs whose `next_run_at` has passed,
//! then executes them in the app's running container using the container runtime.

use crate::db::{App, ScheduledJob};
use crate::engine::backup_jobs::notify::{self, BackupEvent, BackupOutcome, BackupResourceKind};
use crate::engine::backup_jobs::{s3_store, JobContext};
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
#[allow(dead_code)]
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

/// Compute the next run time from a cron expression (5-field standard cron or the
/// crate's 6/7-field form).
fn next_run_from_cron_expr(cron_expression: &str) -> Option<String> {
    crate::engine::backup_jobs::schedule::next_run_after(cron_expression, Utc::now())
        .ok()
        .map(|t| t.to_rfc3339())
}

/// Execute a single backup schedule entry
async fn execute_backup_schedule(ctx: &JobContext, schedule: &BackupScheduleRow) {
    tracing::info!(
        schedule_id = %schedule.id,
        backup_type = %schedule.backup_type,
        "Running scheduled backup"
    );

    // Record the run first so a slow backup is not started again on the next tick.
    let now = Utc::now().to_rfc3339();
    let next_run = next_run_from_cron_expr(&schedule.cron_expression);
    if let Err(e) =
        sqlx::query("UPDATE backup_schedules SET last_run_at = ?, next_run_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&next_run)
            .bind(&schedule.id)
            .execute(&ctx.db)
            .await
    {
        tracing::warn!(
            schedule_id = %schedule.id,
            error = %e,
            "Failed to update backup schedule run times"
        );
    }

    let result = match schedule.backup_type.as_str() {
        "instance" => {
            let result = run_instance_backup(ctx, schedule.s3_config_id.as_deref(), "scheduled")
                .await
                .map(|_| ());
            prune_instance_backups(
                ctx,
                schedule.s3_config_id.as_deref(),
                schedule.retention_days,
            )
            .await;
            result
        }
        "s3_database" => run_s3_database_schedule(ctx, schedule).await,
        "s3_volume" => run_s3_volume_schedule(ctx, schedule).await,
        other => Err(anyhow::anyhow!("Unknown backup_type: {}", other)),
    };

    if let Err(e) = &result {
        tracing::error!(
            schedule_id = %schedule.id,
            error = %e,
            "Scheduled backup failed"
        );
    }
}

/// Run one `backup_schedules` entry now (manual "Run now" from the API). The S3
/// database/volume types go through the same code as the scheduler; run times are
/// updated by the caller.
pub async fn run_backup_schedule_now(ctx: &JobContext, schedule_id: &str) {
    let row: Option<BackupScheduleRow> = sqlx::query_as(
        r#"SELECT id, backup_type, cron_expression, target_id, s3_config_id,
                  retention_days, last_run_at, next_run_at
           FROM backup_schedules WHERE id = ?"#,
    )
    .bind(schedule_id)
    .fetch_optional(&ctx.db)
    .await
    .ok()
    .flatten();
    let Some(schedule) = row else {
        return;
    };
    let result = match schedule.backup_type.as_str() {
        "s3_database" => run_s3_database_schedule(ctx, &schedule).await,
        "s3_volume" => run_s3_volume_schedule(ctx, &schedule).await,
        "instance" => {
            let result = run_instance_backup(ctx, schedule.s3_config_id.as_deref(), "manual")
                .await
                .map(|_| ());
            prune_instance_backups(
                ctx,
                schedule.s3_config_id.as_deref(),
                schedule.retention_days,
            )
            .await;
            result
        }
        other => Err(anyhow::anyhow!("Unknown backup_type: {}", other)),
    };
    if let Err(e) = result {
        tracing::error!(schedule_id, error = %e, "Manual backup schedule run failed");
    }
}

/// Create an instance backup (database, config, certificates), optionally copy it to
/// S3, and notify. Shared by the scheduler and the API.
pub async fn run_instance_backup(
    ctx: &JobContext,
    s3_config_id: Option<&str>,
    trigger: &str,
) -> anyhow::Result<crate::backup::BackupResult> {
    let started = std::time::Instant::now();
    let result = async {
        let backup = crate::backup::create_backup(
            &ctx.db,
            &ctx.data_dir,
            &ctx.config_path,
            &ctx.acme_cache_dir,
            None,
        )
        .await?;
        let mut destination = "local".to_string();
        if let Some(config_id) = s3_config_id.filter(|s| !s.is_empty()) {
            let config = s3_store::load_config(&ctx.db, config_id).await?;
            s3_store::upload_and_record(
                &ctx.db,
                ctx.secret(),
                &config,
                "instance",
                None,
                &backup.path,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Backup saved locally but the S3 upload failed: {e:#}"))?;
            destination = format!("local + {}", notify::s3_destination(&config.name));
        }
        Ok::<_, anyhow::Error>((backup, destination))
    }
    .await;

    let event = |outcome| {
        BackupEvent::new(outcome, BackupResourceKind::Instance, "Rivetr instance").trigger(trigger)
    };
    match result {
        Ok((backup, destination)) => {
            notify::notify_backup(
                &ctx.db,
                event(BackupOutcome::Succeeded)
                    .size(Some(backup.size as i64))
                    .duration(Some(started.elapsed()))
                    .destination(destination),
            );
            Ok(backup)
        }
        Err(e) => {
            notify::notify_backup(
                &ctx.db,
                event(BackupOutcome::Failed).error(format!("{e:#}")),
            );
            Err(e)
        }
    }
}

/// `s3_database` schedule: dump the database and copy the dump to S3.
async fn run_s3_database_schedule(
    ctx: &JobContext,
    schedule: &BackupScheduleRow,
) -> anyhow::Result<()> {
    let (Some(config_id), Some(database_id)) = (&schedule.s3_config_id, &schedule.target_id) else {
        anyhow::bail!("s3 backup schedule missing s3_config_id or target_id");
    };
    let database: crate::db::ManagedDatabase =
        sqlx::query_as("SELECT * FROM databases WHERE id = ?")
            .bind(database_id)
            .fetch_optional(&ctx.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Database {database_id} not found"))?;
    let result = async {
        let config = s3_store::load_config(&ctx.db, config_id).await?;
        let task = crate::engine::database_backups::DatabaseBackupTask::new(
            ctx.db.clone(),
            ctx.runtime.clone(),
            ctx.db_backup.clone(),
            ctx.data_dir.clone(),
        );
        let backup = task
            .backup_database(&database, crate::db::BackupType::Scheduled)
            .await?;
        crate::engine::backup_jobs::database::upload_database_backup(
            ctx, &database, &backup, &config,
        )
        .await?;
        prune_s3_older_than(
            ctx,
            &config,
            "database",
            Some(database_id),
            schedule.retention_days,
            None,
        )
        .await;
        Ok::<_, anyhow::Error>((backup, config))
    }
    .await;
    match result {
        Ok((backup, config)) => {
            crate::engine::backup_jobs::database::notify_database_backup(
                &ctx.db,
                &database,
                "scheduled",
                Ok(&backup),
                crate::engine::backup_jobs::database::destination_label(Some(&config)),
            );
            Ok(())
        }
        Err(e) => {
            crate::engine::backup_jobs::database::notify_database_backup(
                &ctx.db,
                &database,
                "scheduled",
                Err(format!("{e:#}")),
                "S3".to_string(),
            );
            Err(e)
        }
    }
}

/// `s3_volume` schedule: back a volume up straight to S3.
async fn run_s3_volume_schedule(
    ctx: &JobContext,
    schedule: &BackupScheduleRow,
) -> anyhow::Result<()> {
    use crate::engine::backup_jobs::volume;
    let (Some(config_id), Some(volume_id)) = (&schedule.s3_config_id, &schedule.target_id) else {
        anyhow::bail!("s3 backup schedule missing s3_config_id or target_id");
    };
    let vol = volume::get_volume(&ctx.db, volume_id).await?;
    let destination = volume::resolve_destination(&ctx.db, "s3", Some(config_id)).await?;
    let config = match &destination {
        volume::Destination::S3(c) => (**c).clone(),
        volume::Destination::Local => unreachable!("resolve_destination(\"s3\") is S3"),
    };
    let record = volume::create_pending(
        &ctx.db,
        &vol.id,
        "scheduled",
        &destination,
        volume::SnapshotMode::Live,
    )
    .await?;
    volume::run_backup_job(ctx, &vol, record, destination, volume::SnapshotMode::Live).await?;
    prune_s3_older_than(
        ctx,
        &config,
        "volume",
        Some(volume_id),
        schedule.retention_days,
        None,
    )
    .await;
    Ok(())
}

/// Local instance backup names to delete: older than `cutoff`, never the newest one
/// (a schedule that has been failing must not leave the instance with no backup).
pub fn instance_backups_to_prune(
    backups: &[crate::backup::BackupInfo],
    cutoff: chrono::DateTime<Utc>,
) -> Vec<String> {
    let mut sorted: Vec<&crate::backup::BackupInfo> = backups.iter().collect();
    sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sorted
        .into_iter()
        .skip(1)
        .filter(|b| {
            crate::engine::backup_jobs::missed::parse_timestamp(&b.created_at)
                .is_some_and(|t| t < cutoff)
        })
        .map(|b| b.name.clone())
        .collect()
}

/// Apply an instance schedule's `retention_days` to local instance backups and, when
/// the schedule uploads to S3, to that config's instance backups.
async fn prune_instance_backups(ctx: &JobContext, s3_config_id: Option<&str>, days: i64) {
    if days <= 0 {
        return;
    }
    let cutoff = Utc::now() - chrono::Duration::days(days);
    match crate::backup::list_backups(&ctx.data_dir) {
        Ok(backups) => {
            for name in instance_backups_to_prune(&backups, cutoff) {
                if let Err(e) = crate::backup::delete_backup(&ctx.data_dir, &name) {
                    tracing::warn!(backup = %name, error = %e, "Instance retention: delete failed");
                } else {
                    tracing::info!(backup = %name, days, "Pruned instance backup past retention");
                }
            }
        }
        Err(e) => tracing::warn!(error = %e, "Instance retention: cannot list backups"),
    }
    if let Some(config_id) = s3_config_id.filter(|s| !s.is_empty()) {
        if let Ok(config) = s3_store::load_config(&ctx.db, config_id).await {
            // Keep the newest S3 copy whatever its age.
            let newest: Option<String> = sqlx::query_scalar(
                "SELECT id FROM s3_backups WHERE storage_config_id = ? AND backup_type = 'instance' \
                 AND status = 'completed' ORDER BY created_at DESC LIMIT 1",
            )
            .bind(&config.id)
            .fetch_optional(&ctx.db)
            .await
            .ok()
            .flatten();
            prune_s3_older_than(ctx, &config, "instance", None, days, newest.as_deref()).await;
        }
    }
}

/// Delete S3 backups of one resource older than `days` (the retention of the generic
/// backup schedules).
async fn prune_s3_older_than(
    ctx: &JobContext,
    config: &crate::db::S3StorageConfig,
    kind: &str,
    source_id: Option<&str>,
    days: i64,
    keep_id: Option<&str>,
) {
    if days <= 0 {
        return;
    }
    let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    let old: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM s3_backups WHERE storage_config_id = ? AND backup_type = ?          AND source_id IS ? AND status = 'completed' AND created_at < ?",
    )
    .bind(&config.id)
    .bind(kind)
    .bind(source_id)
    .bind(&cutoff)
    .fetch_all(&ctx.db)
    .await
    .unwrap_or_default();
    for id in old.into_iter().filter(|id| Some(id.as_str()) != keep_id) {
        crate::engine::backup_jobs::volume::delete_s3_backup(ctx, &id).await;
        let _ = sqlx::query("DELETE FROM volume_backups WHERE s3_backup_id = ?")
            .bind(&id)
            .execute(&ctx.db)
            .await;
    }
}

/// One cycle of the backup scheduler: find due schedules and run them
async fn backup_scheduler_cycle(ctx: &JobContext) {
    let now = Utc::now().to_rfc3339();

    let due: Vec<BackupScheduleRow> = match sqlx::query_as(
        r#"SELECT id, backup_type, cron_expression, target_id, s3_config_id,
                  retention_days, last_run_at, next_run_at
           FROM backup_schedules
           WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?"#,
    )
    .bind(&now)
    .fetch_all(&ctx.db)
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
        let ctx = ctx.clone();
        tokio::spawn(async move {
            execute_backup_schedule(&ctx, &schedule).await;
        });
    }
}

/// Spawn the background backup scheduler (checks every 60 seconds)
pub fn spawn_backup_scheduler(ctx: JobContext) {
    tracing::info!("Starting backup scheduler (60s interval)");

    tokio::spawn(async move {
        // Brief startup delay so the DB is fully ready
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            crate::utils::supervise::guarded("backup_scheduler", backup_scheduler_cycle(&ctx))
                .await;
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
            crate::utils::supervise::guarded("scheduler", scheduler_cycle(&db, &runtime)).await;
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
        let app: Option<App> = match sqlx::query_as("SELECT * FROM apps WHERE id = ?")
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
        if let Err(e) = sqlx::query("UPDATE deployments SET scheduled_at = NULL WHERE id = ?")
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
pub fn spawn_scheduled_deployment_checker(db: DbPool, deploy_tx: mpsc::Sender<(String, App)>) {
    tracing::info!("Starting scheduled deployment checker (60s interval)");

    tokio::spawn(async move {
        // Wait a bit before the first check so the engine is ready
        tokio::time::sleep(Duration::from_secs(20)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            crate::utils::supervise::guarded(
                "scheduled_deployment_checker",
                check_scheduled_deployments(&db, &deploy_tx),
            )
            .await;
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
            "cpu" => sqlx::query_scalar::<_, f64>(
                "SELECT cpu_percent FROM resource_metrics WHERE app_id = ? \
                     ORDER BY recorded_at DESC LIMIT 1",
            )
            .bind(&rule.app_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten(),
            "memory" => sqlx::query_scalar::<_, f64>(
                "SELECT memory_percent FROM resource_metrics WHERE app_id = ? \
                     ORDER BY recorded_at DESC LIMIT 1",
            )
            .bind(&rule.app_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten(),
            _ => None, // request_rate not yet implemented
        };

        let Some(value) = metric_value else {
            continue;
        };

        // Fetch current replica count
        let current_replicas: i64 =
            sqlx::query_scalar::<_, i64>("SELECT replica_count FROM apps WHERE id = ?")
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
            crate::utils::supervise::guarded("autoscaling", autoscaling_cycle(&db)).await;
        }
    });
}

#[cfg(test)]
mod backup_retention_tests {
    use super::*;
    use chrono::TimeZone;

    fn info(name: &str, created_at: &str) -> crate::backup::BackupInfo {
        crate::backup::BackupInfo {
            name: name.into(),
            size: 1,
            created_at: created_at.into(),
        }
    }

    #[test]
    fn instance_retention_prunes_old_but_keeps_newest() {
        let cutoff = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
        let backups = vec![
            info("a", "2026-10-01T00:00:00Z"),
            info("b", "2026-10-20T00:00:00Z"),
            info("c", "2026-11-03T00:00:00Z"),
        ];
        let mut pruned = instance_backups_to_prune(&backups, cutoff);
        pruned.sort();
        assert_eq!(pruned, vec!["a", "b"]);

        // All old: the newest survives anyway.
        let old = vec![
            info("x", "2026-01-01T00:00:00Z"),
            info("y", "2026-02-01T00:00:00Z"),
        ];
        assert_eq!(instance_backups_to_prune(&old, cutoff), vec!["x"]);
        assert!(instance_backups_to_prune(&[], cutoff).is_empty());
    }
}
