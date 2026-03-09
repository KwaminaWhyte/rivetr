//! Cron-based job scheduler for running commands inside app containers.
//!
//! Checks every 60 seconds for jobs whose `next_run_at` has passed,
//! then executes them in the app's running container using the container runtime.

use crate::db::ScheduledJob;
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

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
