//! Health monitoring: per-cycle check-and-restart logic for deployments, databases, and services.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::api::metrics::{increment_container_restarts, set_container_restart_backoff_seconds};
use crate::config::ContainerMonitorConfig;
use crate::db::{App, Deployment, ManagedDatabase, NotificationEventType, Service};
use crate::notifications::{NotificationPayload, NotificationService};
use crate::runtime::ContainerRuntime;
use crate::DbPool;

use super::recovery::ContainerRestartState;
use super::recovery::{
    add_deployment_log, mark_database_failed, mark_database_stopped, mark_deployment_failed,
    mark_service_stopped,
};
use super::stats::check_compose_service_running;
use super::MonitorResult;

/// Run a single monitoring cycle: check all deployments, databases, and services.
pub(super) async fn check_and_restart(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    config: &ContainerMonitorConfig,
    restart_states: &mut HashMap<String, ContainerRestartState>,
    healthy_containers: &mut HashMap<String, Instant>,
) -> MonitorResult {
    let mut result = MonitorResult::default();

    if !config.enabled {
        return result;
    }

    // Get all deployments that should be running
    let running_deployments: Vec<Deployment> = match sqlx::query_as(
        "SELECT * FROM deployments WHERE status = 'running' AND container_id IS NOT NULL",
    )
    .fetch_all(db)
    .await
    {
        Ok(deployments) => deployments,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running deployments");
            return result;
        }
    };

    result.deployments_checked = running_deployments.len();

    for deployment in &running_deployments {
        let container_id = match &deployment.container_id {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        // Get app name for logging and metrics
        let app_name: Option<(String,)> = sqlx::query_as("SELECT name FROM apps WHERE id = ?")
            .bind(&deployment.app_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

        let app_name = app_name
            .map(|(name,)| name)
            .unwrap_or_else(|| "unknown".to_string());

        let container_name = format!("rivetr-{}", app_name);

        // Check if container is running
        match runtime.inspect(container_id).await {
            Ok(info) => {
                // Even if running, a high Docker-level restart count means a crash loop:
                // the container keeps crashing and Docker restarts it before our monitor fires.
                let crash_loop = info.running && info.restart_count >= config.max_restart_attempts;

                if crash_loop {
                    result.containers_crashed += 1;
                    tracing::warn!(
                        container = %container_name,
                        app = %app_name,
                        restart_count = info.restart_count,
                        "Container is in a crash loop (Docker restart count exceeded threshold), stopping"
                    );
                    let _ = runtime.stop(container_id).await;
                    handle_crashed_container(
                        db,
                        runtime,
                        deployment,
                        container_id,
                        &container_name,
                        &app_name,
                        config,
                        restart_states,
                        healthy_containers,
                        &mut result,
                    )
                    .await;
                } else if info.running {
                    handle_running_container(
                        container_name.as_str(),
                        &app_name,
                        config,
                        restart_states,
                        healthy_containers,
                    );
                    result.containers_running += 1;
                } else {
                    // Container exists but is not running - it crashed
                    result.containers_crashed += 1;
                    handle_crashed_container(
                        db,
                        runtime,
                        deployment,
                        container_id,
                        &container_name,
                        &app_name,
                        config,
                        restart_states,
                        healthy_containers,
                        &mut result,
                    )
                    .await;
                }
            }
            Err(e) => {
                // Container doesn't exist or can't be inspected
                tracing::debug!(
                    container = %container_id,
                    app = %app_name,
                    error = %e,
                    "Failed to inspect container"
                );
                if let Err(e) =
                    mark_deployment_failed(db, &deployment.id, "Container not found").await
                {
                    tracing::warn!(
                        deployment = %deployment.id,
                        error = %e,
                        "Failed to update deployment status"
                    );
                }
            }
        }
    }

    // Clean up old restart states for containers that no longer exist
    super::recovery::cleanup_stale_states(restart_states, &running_deployments);

    // Check managed databases
    check_databases(db, runtime, &mut result).await;

    // Check Docker Compose services
    check_services(db, runtime, &mut result).await;

    result
}

/// Handle a container that is currently running
fn handle_running_container(
    container_name: &str,
    app_name: &str,
    config: &ContainerMonitorConfig,
    restart_states: &mut HashMap<String, ContainerRestartState>,
    healthy_containers: &mut HashMap<String, Instant>,
) {
    if !healthy_containers.contains_key(container_name) {
        healthy_containers.insert(container_name.to_string(), Instant::now());
    }

    if let Some(first_seen) = healthy_containers.get(container_name) {
        let stable_duration = Duration::from_secs(config.stable_duration_secs);
        if first_seen.elapsed() >= stable_duration {
            if let Some(state) = restart_states.get_mut(container_name) {
                if state.restart_count > 0 {
                    tracing::info!(
                        container = %container_name,
                        app = %app_name,
                        "Container stable, resetting restart counter"
                    );
                    state.reset(config.initial_backoff_secs);
                    set_container_restart_backoff_seconds(app_name, 0.0);
                }
            }
        }
    }
}

/// Send a ContainerCrash notification, rate-limited to once per 5 minutes per app.
async fn send_crash_notification_if_due(db: &DbPool, app_id: &str, app_name: &str, message: &str) {
    // Look up the app to check last_crash_notified_at
    let app: Option<App> = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();

    let should_notify = match app {
        None => true, // app not found, send anyway
        Some(ref a) => match &a.last_crash_notified_at {
            None => true,
            Some(last_notified) => {
                // Parse the stored timestamp and check if 5 minutes have elapsed
                chrono::DateTime::parse_from_rfc3339(last_notified)
                    .map(|dt| {
                        let elapsed = chrono::Utc::now()
                            .signed_duration_since(dt.with_timezone(&chrono::Utc));
                        elapsed.num_seconds() >= 300
                    })
                    .unwrap_or(true)
            }
        },
    };

    if !should_notify {
        return;
    }

    // Update last_crash_notified_at before sending
    if let Err(e) =
        sqlx::query("UPDATE apps SET last_crash_notified_at = datetime('now') WHERE id = ?")
            .bind(app_id)
            .execute(db)
            .await
    {
        tracing::warn!(
            app_id = %app_id,
            error = %e,
            "Failed to update last_crash_notified_at"
        );
    }

    let notification_service = NotificationService::new(db.clone());
    let payload = NotificationPayload::app_event(
        NotificationEventType::ContainerCrash,
        app_id.to_string(),
        app_name.to_string(),
        message.to_string(),
    );
    if let Err(e) = notification_service.send(&payload).await {
        tracing::warn!(error = %e, "Failed to send container_crash notification");
    }
}

/// Handle a crashed container
#[allow(clippy::too_many_arguments)]
async fn handle_crashed_container(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    deployment: &Deployment,
    container_id: &str,
    container_name: &str,
    app_name: &str,
    config: &ContainerMonitorConfig,
    restart_states: &mut HashMap<String, ContainerRestartState>,
    healthy_containers: &mut HashMap<String, Instant>,
    result: &mut MonitorResult,
) {
    healthy_containers.remove(container_name);

    let initial_backoff = config.initial_backoff_secs;
    let max_backoff = config.max_backoff_secs;
    let max_attempts = config.max_restart_attempts;

    let state = restart_states
        .entry(container_name.to_string())
        .or_insert_with(|| ContainerRestartState::new(initial_backoff));

    if state.restart_count >= max_attempts {
        if !state.failed {
            let restart_count = state.restart_count;
            state.mark_failed();

            tracing::error!(
                container = %container_name,
                app = %app_name,
                attempts = restart_count,
                "Container exceeded maximum restart attempts, marking as failed"
            );

            if let Err(e) = mark_deployment_failed(
                db,
                &deployment.id,
                &format!("Exceeded maximum restart attempts ({})", max_attempts),
            )
            .await
            {
                tracing::warn!(
                    deployment = %deployment.id,
                    error = %e,
                    "Failed to update deployment status"
                );
            }

            add_deployment_log(
                db,
                &deployment.id,
                "ERROR",
                &format!(
                    "Container crashed and exceeded maximum restart attempts ({}). Manual intervention required.",
                    max_attempts
                ),
            )
            .await;

            // Send ContainerCrash notification (rate-limited to once per 5 minutes)
            send_crash_notification_if_due(
                db,
                &deployment.app_id,
                app_name,
                &format!(
                    "Container for {} has crashed and exceeded {} restart attempts. Manual intervention required.",
                    app_name, max_attempts
                ),
            )
            .await;
        }
        return;
    }

    if !state.should_restart() {
        let remaining = state
            .last_restart
            .map(|last| {
                let elapsed = last.elapsed().as_secs();
                state.current_backoff_secs.saturating_sub(elapsed)
            })
            .unwrap_or(0);

        tracing::debug!(
            container = %container_name,
            app = %app_name,
            remaining_secs = remaining,
            "Waiting for backoff before restart"
        );
        return;
    }

    let current_attempt = state.restart_count + 1;

    tracing::info!(
        container = %container_name,
        app = %app_name,
        attempt = current_attempt,
        max_attempts = max_attempts,
        "Attempting to restart crashed container"
    );

    add_deployment_log(
        db,
        &deployment.id,
        "WARN",
        &format!(
            "Container crashed, attempting restart (attempt {}/{})",
            current_attempt, max_attempts
        ),
    )
    .await;

    let restart_result = runtime.start(container_id).await;

    // Update state after restart attempt
    let state = restart_states
        .get_mut(container_name)
        .expect("State should exist");

    match restart_result {
        Ok(_) => {
            state.record_restart(max_backoff);
            let new_backoff = state.current_backoff_secs;
            let restart_count = state.restart_count;
            result.containers_restarted += 1;

            increment_container_restarts(app_name);
            set_container_restart_backoff_seconds(app_name, new_backoff as f64);

            tracing::info!(
                container = %container_name,
                app = %app_name,
                next_backoff_secs = new_backoff,
                "Container restarted successfully"
            );

            add_deployment_log(
                db,
                &deployment.id,
                "INFO",
                &format!(
                    "Container restarted successfully (attempt {}/{}). Next backoff: {}s",
                    restart_count, max_attempts, new_backoff
                ),
            )
            .await;

            // Send ContainerRestarted notification
            let notification_service = NotificationService::new(db.clone());
            let payload = NotificationPayload::app_event(
                NotificationEventType::ContainerRestarted,
                deployment.app_id.clone(),
                app_name.to_string(),
                format!(
                    "Container for {} was automatically restarted (attempt {}/{}). Next backoff: {}s.",
                    app_name, restart_count, max_attempts, new_backoff
                ),
            );
            if let Err(e) = notification_service.send(&payload).await {
                tracing::warn!(error = %e, "Failed to send container_restarted notification");
            }
        }
        Err(e) => {
            state.record_restart(max_backoff);
            result.restart_failures += 1;

            tracing::error!(
                container = %container_name,
                app = %app_name,
                error = %e,
                "Failed to restart container"
            );

            add_deployment_log(
                db,
                &deployment.id,
                "ERROR",
                &format!("Failed to restart container: {}", e),
            )
            .await;
        }
    }
}

/// Check managed databases for stopped/crashed containers
pub(super) async fn check_databases(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    result: &mut MonitorResult,
) {
    let running_databases: Vec<ManagedDatabase> = match sqlx::query_as(
        r#"
        SELECT id, name, db_type, version, container_id, container_slug, status, internal_port,
               external_port, public_access, credentials, volume_name, volume_path,
               memory_limit, cpu_limit, error_message, project_id, team_id, created_at, updated_at,
               COALESCE(ssl_enabled, 0) AS ssl_enabled,
               ssl_mode,
               custom_image,
               init_commands
        FROM databases
        WHERE status = 'running' AND container_id IS NOT NULL
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(databases) => databases,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running databases");
            return;
        }
    };

    result.databases_checked = running_databases.len();

    for database in &running_databases {
        let container_id = match &database.container_id {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        let container_name = database.container_name();

        match runtime.inspect(container_id).await {
            Ok(info) => {
                if info.running {
                    result.databases_running += 1;
                } else {
                    result.databases_stopped += 1;
                    tracing::warn!(
                        database = %database.name,
                        container = %container_name,
                        "Database container stopped"
                    );

                    if let Err(e) = mark_database_stopped(db, &database.id).await {
                        tracing::warn!(
                            database = %database.id,
                            error = %e,
                            "Failed to update database status"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    container = %container_id,
                    database = %database.name,
                    error = %e,
                    "Failed to inspect database container"
                );

                result.databases_stopped += 1;

                if let Err(e) = mark_database_failed(db, &database.id, "Container not found").await
                {
                    tracing::warn!(
                        database = %database.id,
                        error = %e,
                        "Failed to update database status"
                    );
                }
            }
        }
    }
}

/// Check Docker Compose services for stopped/crashed containers
pub(super) async fn check_services(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
    result: &mut MonitorResult,
) {
    let running_services: Vec<Service> = match sqlx::query_as(
        r#"
        SELECT id, name, project_id, team_id, compose_content, domain, port, status, error_message, created_at, updated_at,
               COALESCE(isolated_network, 1) AS isolated_network,
               COALESCE(raw_compose_mode, 0) AS raw_compose_mode,
               COALESCE(public_access, 0) AS public_access,
               COALESCE(external_port, 0) AS external_port,
               COALESCE(expose_container_port, 0) AS expose_container_port
        FROM services
        WHERE status = 'running'
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(services) => services,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running services");
            return;
        }
    };

    result.services_checked = running_services.len();

    for service in &running_services {
        let project_name = service.compose_project_name();

        let is_running = check_compose_service_running(&project_name, &service.name, runtime).await;

        if is_running {
            result.services_running += 1;
        } else {
            result.services_stopped += 1;
            tracing::warn!(
                service = %service.name,
                project = %project_name,
                "Docker Compose service stopped"
            );

            if let Err(e) = mark_service_stopped(db, &service.id).await {
                tracing::warn!(
                    service = %service.id,
                    error = %e,
                    "Failed to update service status"
                );
            }
        }
    }
}
