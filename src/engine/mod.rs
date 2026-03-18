mod alert_evaluator;
pub mod build_detect;
mod cleanup;
mod container_monitor;
mod cost_calculator;
pub mod database_backups;
pub mod database_config;
mod disk_monitor;
pub mod nixpacks;
pub mod pack_builder;
mod pipeline;
pub mod preview;
pub mod railpack;
pub mod remote;
mod resource_metrics_collector;
pub mod scheduler;
pub mod static_builder;
mod stats_collector;
pub mod updater;
pub mod zip_extract;

pub use alert_evaluator::*;
pub use build_detect::*;
pub use cleanup::*;
pub use container_monitor::*;
pub use cost_calculator::*;
pub use database_backups::*;
pub use disk_monitor::*;
pub use pipeline::*;
pub use preview::*;
pub use resource_metrics_collector::*;
pub use scheduler::*;
pub use static_builder::*;
pub use stats_collector::*;
pub use zip_extract::*;

use crate::api::metrics::{
    increment_deployments_total, observe_deployment_duration, record_deployment_failed,
    record_deployment_success,
};
use crate::config::{AuthConfig, RuntimeConfig};
use crate::crypto;
use crate::db::{App, NotificationEventType};
use crate::notifications::{NotificationPayload, NotificationService};
use crate::proxy::{Backend, BasicAuthConfig, RouteTable};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

pub type DeploymentJob = (String, App); // (deployment_id, app)

/// Build resource limits configuration
#[derive(Debug, Clone)]
pub struct BuildLimits {
    pub cpu_limit: Option<String>,
    pub memory_limit: Option<String>,
}

impl BuildLimits {
    pub fn from_runtime_config(config: &RuntimeConfig) -> Self {
        Self {
            cpu_limit: Some(config.build_cpu_limit.clone()),
            memory_limit: Some(config.build_memory_limit.clone()),
        }
    }
}

pub struct DeploymentEngine {
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes: Arc<ArcSwap<RouteTable>>,
    rx: mpsc::Receiver<DeploymentJob>,
    build_limits: BuildLimits,
    encryption_key: Option<[u8; KEY_LENGTH]>,
}

impl DeploymentEngine {
    pub fn new(
        db: DbPool,
        runtime: Arc<dyn ContainerRuntime>,
        routes: Arc<ArcSwap<RouteTable>>,
        rx: mpsc::Receiver<DeploymentJob>,
        build_limits: BuildLimits,
        auth_config: &AuthConfig,
    ) -> Self {
        // Derive encryption key from config if available
        let encryption_key = auth_config
            .encryption_key
            .as_ref()
            .map(|secret| crypto::derive_key(secret));

        Self {
            db,
            runtime,
            routes,
            rx,
            build_limits,
            encryption_key,
        }
    }

    pub async fn run(mut self) {
        tracing::info!("Deployment engine started");

        while let Some((deployment_id, app)) = self.rx.recv().await {
            tracing::info!(
                "Processing deployment {} for app {}",
                deployment_id,
                app.name
            );

            let db = self.db.clone();
            let runtime = self.runtime.clone();
            let routes = self.routes.clone();
            let build_limits = self.build_limits.clone();
            let encryption_key = self.encryption_key;

            tokio::spawn(async move {
                let deploy_start = std::time::Instant::now();
                let notification_service = NotificationService::new(db.clone());

                // Send deployment_started notification
                let started_payload = NotificationPayload::deployment_event(
                    NotificationEventType::DeploymentStarted,
                    app.id.clone(),
                    app.name.clone(),
                    deployment_id.clone(),
                    "started".to_string(),
                    format!("Deployment started for {}", app.name),
                    None,
                );
                if let Err(e) = notification_service.send(&started_payload).await {
                    tracing::warn!(error = %e, "Failed to send deployment_started notification");
                }

                match run_deployment(
                    &db,
                    runtime.clone(),
                    &deployment_id,
                    &app,
                    &build_limits,
                    encryption_key.as_ref(),
                )
                .await
                {
                    Ok(container_info) => {
                        // Record successful deployment metric
                        record_deployment_success();
                        let duration_secs = deploy_start.elapsed().as_secs_f64();
                        increment_deployments_total(&app.name, "success");
                        observe_deployment_duration(&app.name, duration_secs);

                        // Send deployment_success notification
                        let success_payload = NotificationPayload::deployment_event(
                            NotificationEventType::DeploymentSuccess,
                            app.id.clone(),
                            app.name.clone(),
                            deployment_id.clone(),
                            "success".to_string(),
                            format!("Deployment successful for {}", app.name),
                            None,
                        );
                        if let Err(e) = notification_service.send(&success_payload).await {
                            tracing::warn!(error = %e, "Failed to send deployment_success notification");
                        }

                        // Mark all previous "running" deployments for this app as "replaced"
                        let _ = sqlx::query(
                            "UPDATE deployments SET status = 'replaced', finished_at = ?
                             WHERE app_id = ? AND status = 'running' AND id != ?",
                        )
                        .bind(chrono::Utc::now().to_rfc3339())
                        .bind(&app.id)
                        .bind(&deployment_id)
                        .execute(&db)
                        .await;

                        // Update proxy routes on successful deployment for all domains
                        if let Some(port) = container_info.port {
                            let all_domains = app.get_all_domain_names();
                            let route_table = routes.load();

                            // Helper to create primary backend with basic auth if configured
                            let create_backend = || {
                                let mut backend = Backend::new(
                                    container_info.container_id.clone(),
                                    "127.0.0.1".to_string(),
                                    port,
                                )
                                .with_healthcheck(app.healthcheck.clone())
                                .with_strip_prefix(app.strip_prefix.clone());

                                // Configure HTTP Basic Auth if enabled
                                if app.basic_auth_enabled != 0 {
                                    if let (Some(username), Some(password_hash)) =
                                        (&app.basic_auth_username, &app.basic_auth_password_hash)
                                    {
                                        backend.set_basic_auth(BasicAuthConfig::new(
                                            username.clone(),
                                            password_hash.clone(),
                                        ));
                                    }
                                }
                                backend
                            };

                            // Collect all replica backend addresses for round-robin load balancing
                            let replica_backends: Vec<String> = {
                                let mut addrs = vec![format!("127.0.0.1:{}", port)];
                                // Fetch running replicas (index > 0)
                                if let Ok(replicas) = sqlx::query_as::<_, crate::db::AppReplica>(
                                    "SELECT * FROM app_replicas WHERE app_id = ? AND replica_index > 0 AND status = 'running'",
                                )
                                .bind(&app.id)
                                .fetch_all(&db)
                                .await
                                {
                                    for replica in &replicas {
                                        if let Some(ref cid) = replica.container_id {
                                            if let Ok(info) = runtime.inspect(cid).await {
                                                if let Some(rport) = info.port {
                                                    addrs.push(format!("127.0.0.1:{}", rport));
                                                }
                                            }
                                        }
                                    }
                                }
                                addrs
                            };

                            if !all_domains.is_empty() {
                                // Log basic auth status once
                                if app.basic_auth_enabled != 0 {
                                    if let Some(username) = &app.basic_auth_username {
                                        tracing::info!(
                                            username = %username,
                                            "HTTP Basic Auth enabled for app {}",
                                            app.name
                                        );
                                    }
                                }

                                for domain in &all_domains {
                                    route_table.add_backends(
                                        domain.clone(),
                                        replica_backends.clone(),
                                        create_backend(),
                                    );
                                }

                                tracing::info!(
                                    domains = ?all_domains,
                                    port = port,
                                    replicas = replica_backends.len(),
                                    healthcheck = ?app.healthcheck,
                                    basic_auth = app.basic_auth_enabled != 0,
                                    "Proxy routes updated for app {}",
                                    app.name
                                );
                            } else if let Some(domain) = &app.domain {
                                // Fallback for legacy domain field only
                                if app.basic_auth_enabled != 0 {
                                    if let Some(username) = &app.basic_auth_username {
                                        tracing::info!(
                                            domain = %domain,
                                            username = %username,
                                            "HTTP Basic Auth enabled for app {}",
                                            app.name
                                        );
                                    }
                                }

                                route_table.add_backends(
                                    domain.clone(),
                                    replica_backends.clone(),
                                    create_backend(),
                                );
                                tracing::info!(
                                    domain = %domain,
                                    port = port,
                                    replicas = replica_backends.len(),
                                    healthcheck = ?app.healthcheck,
                                    basic_auth = app.basic_auth_enabled != 0,
                                    "Proxy route updated for app {}",
                                    app.name
                                );
                            }
                        }

                        // Zero-downtime: stop old containers AFTER proxy routes are updated.
                        // New container is already serving traffic; old one can now be torn down.
                        if !container_info.old_container_ids.is_empty() {
                            tracing::info!(
                                old_containers = ?container_info.old_container_ids,
                                "Stopping old containers after proxy route swap (zero-downtime)"
                            );
                            for old_id in &container_info.old_container_ids {
                                // Skip if the old ID is the same as the new container (no previous deployment)
                                if old_id == &container_info.container_id {
                                    continue;
                                }
                                let _ = runtime.stop(old_id).await;
                                let _ = runtime.remove(old_id).await;
                            }
                        }
                    }
                    Err(e) => {
                        // Check if this is an auto-rollback triggered error
                        if let Some(auto_rollback) = e.downcast_ref::<AutoRollbackTriggered>() {
                            tracing::info!(
                                "Deployment {} failed but auto-rollback was triggered to {}",
                                deployment_id,
                                auto_rollback.target_deployment_id
                            );

                            // Mark the original deployment as failed
                            let _ = update_deployment_status(
                                &db,
                                &deployment_id,
                                "failed",
                                Some(&format!(
                                    "Health check failed. Auto-rollback triggered to {}",
                                    auto_rollback.target_deployment_id
                                )),
                            )
                            .await;

                            // Get the rollback deployment info to update routes
                            if let Ok(Some(rollback_deployment)) =
                                sqlx::query_as::<_, crate::db::Deployment>(
                                    "SELECT * FROM deployments WHERE id = ?",
                                )
                                .bind(&auto_rollback.rollback_deployment_id)
                                .fetch_optional(&db)
                                .await
                            {
                                if let Some(ref container_id) = rollback_deployment.container_id {
                                    // Get container port and update routes
                                    if let Ok(info) = runtime.inspect(container_id).await {
                                        if let Some(port) = info.port {
                                            let all_domains = app.get_all_domain_names();
                                            let route_table = routes.load();

                                            let create_backend = || {
                                                let mut backend = Backend::new(
                                                    container_id.clone(),
                                                    "127.0.0.1".to_string(),
                                                    port,
                                                )
                                                .with_healthcheck(app.healthcheck.clone())
                                                .with_strip_prefix(app.strip_prefix.clone());

                                                if app.basic_auth_enabled != 0 {
                                                    if let (Some(username), Some(password_hash)) = (
                                                        &app.basic_auth_username,
                                                        &app.basic_auth_password_hash,
                                                    ) {
                                                        backend.set_basic_auth(
                                                            BasicAuthConfig::new(
                                                                username.clone(),
                                                                password_hash.clone(),
                                                            ),
                                                        );
                                                    }
                                                }
                                                backend
                                            };

                                            for domain in &all_domains {
                                                route_table
                                                    .add_route(domain.clone(), create_backend());
                                            }

                                            tracing::info!(
                                                domains = ?all_domains,
                                                port = port,
                                                "Proxy routes updated after auto-rollback for app {}",
                                                app.name
                                            );
                                        }
                                    }
                                }
                            }

                            // Zero-downtime: stop old containers AFTER proxy routes are updated.
                            if !auto_rollback.old_container_ids.is_empty() {
                                tracing::info!(
                                    old_containers = ?auto_rollback.old_container_ids,
                                    "Stopping old containers after auto-rollback proxy route swap (zero-downtime)"
                                );
                                if let Ok(Some(rb_deployment)) =
                                    sqlx::query_as::<_, crate::db::Deployment>(
                                        "SELECT * FROM deployments WHERE id = ?",
                                    )
                                    .bind(&auto_rollback.rollback_deployment_id)
                                    .fetch_optional(&db)
                                    .await
                                {
                                    let new_container_id =
                                        rb_deployment.container_id.unwrap_or_default();
                                    for old_id in &auto_rollback.old_container_ids {
                                        if old_id == &new_container_id {
                                            continue;
                                        }
                                        let _ = runtime.stop(old_id).await;
                                        let _ = runtime.remove(old_id).await;
                                    }
                                }
                            }

                            // Mark previous running deployments as replaced (except the rollback)
                            let _ = sqlx::query(
                                "UPDATE deployments SET status = 'replaced', finished_at = ?
                                 WHERE app_id = ? AND status = 'running' AND id != ?",
                            )
                            .bind(chrono::Utc::now().to_rfc3339())
                            .bind(&app.id)
                            .bind(&auto_rollback.rollback_deployment_id)
                            .execute(&db)
                            .await;

                            // Send auto-rollback notification
                            let rollback_payload = NotificationPayload::deployment_event(
                                NotificationEventType::DeploymentFailed,
                                app.id.clone(),
                                app.name.clone(),
                                deployment_id.clone(),
                                "auto_rollback".to_string(),
                                format!(
                                    "Deployment failed for {}. Auto-rollback to previous version completed.",
                                    app.name
                                ),
                                Some(format!(
                                    "Health check failed. Rolled back to deployment {}",
                                    auto_rollback.target_deployment_id
                                )),
                            );
                            if let Err(notify_err) =
                                notification_service.send(&rollback_payload).await
                            {
                                tracing::warn!(error = %notify_err, "Failed to send auto-rollback notification");
                            }
                        } else {
                            // Regular failure - no auto-rollback
                            // Check if the deployment was cancelled — if so, preserve that status
                            let current_status: Option<String> =
                                sqlx::query_scalar("SELECT status FROM deployments WHERE id = ?")
                                    .bind(&deployment_id)
                                    .fetch_optional(&db)
                                    .await
                                    .ok()
                                    .flatten();

                            if current_status.as_deref() == Some("cancelled") {
                                tracing::info!(
                                    "Deployment {} was cancelled — skipping failed status update",
                                    deployment_id
                                );
                            } else {
                                record_deployment_failed();
                                let duration_secs = deploy_start.elapsed().as_secs_f64();
                                increment_deployments_total(&app.name, "failed");
                                observe_deployment_duration(&app.name, duration_secs);

                                tracing::error!("Deployment {} failed: {}", deployment_id, e);
                                let _ = update_deployment_status(
                                    &db,
                                    &deployment_id,
                                    "failed",
                                    Some(&e.to_string()),
                                )
                                .await;

                                // If the old container was renamed to "rivetr-<app>-prev" for the
                                // zero-downtime swap, rename it back now so it remains discoverable
                                // by its canonical name and restart logic works correctly.
                                let prev_name = format!("rivetr-{}-prev", app.name);
                                let canonical_name = format!("rivetr-{}", app.name);
                                if runtime.inspect(&prev_name).await.is_ok() {
                                    if let Err(e) =
                                        runtime.rename_container(&prev_name, &canonical_name).await
                                    {
                                        tracing::warn!(
                                            error = %e,
                                            "Failed to rename old container back after deployment failure"
                                        );
                                    }
                                }

                                // Send deployment_failed notification
                                let failed_payload = NotificationPayload::deployment_event(
                                    NotificationEventType::DeploymentFailed,
                                    app.id.clone(),
                                    app.name.clone(),
                                    deployment_id.clone(),
                                    "failed".to_string(),
                                    format!("Deployment failed for {}", app.name),
                                    Some(e.to_string()),
                                );
                                if let Err(notify_err) =
                                    notification_service.send(&failed_payload).await
                                {
                                    tracing::warn!(error = %notify_err, "Failed to send deployment_failed notification");
                                }
                            } // end else (not cancelled)
                        }
                    }
                }
            });
        }
    }
}

async fn update_deployment_status(
    db: &DbPool,
    deployment_id: &str,
    status: &str,
    error: Option<&str>,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    if status == "running" || status == "failed" || status == "stopped" {
        sqlx::query(
            "UPDATE deployments SET status = ?, error_message = ?, finished_at = ? WHERE id = ? AND status != 'cancelled'",
        )
        .bind(status)
        .bind(error)
        .bind(&now)
        .bind(deployment_id)
        .execute(db)
        .await?;
    } else {
        sqlx::query("UPDATE deployments SET status = ?, error_message = ? WHERE id = ? AND status != 'cancelled'")
            .bind(status)
            .bind(error)
            .bind(deployment_id)
            .execute(db)
            .await?;
    }

    Ok(())
}

async fn add_deployment_log(
    db: &DbPool,
    deployment_id: &str,
    level: &str,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)")
        .bind(deployment_id)
        .bind(level)
        .bind(message)
        .execute(db)
        .await?;

    // Also forward to log drains for the app
    // Look up the app_id from the deployment
    let app_id: Option<(String,)> = sqlx::query_as("SELECT app_id FROM deployments WHERE id = ?")
        .bind(deployment_id)
        .fetch_optional(db)
        .await
        .unwrap_or(None);

    if let Some((app_id,)) = app_id {
        let db_clone = db.clone();
        let app_id = app_id.clone();
        let level = level.to_string();
        let message = message.to_string();
        // Fire-and-forget: send to log drains without blocking deployment
        tokio::spawn(async move {
            let manager = crate::logging::LogDrainManager::new(db_clone);
            let timestamp = chrono::Utc::now().to_rfc3339();
            manager
                .send_log(&app_id, &message, &level, &timestamp)
                .await;
        });
    }

    Ok(())
}
