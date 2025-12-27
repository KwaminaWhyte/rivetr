mod cleanup;
mod container_monitor;
mod disk_monitor;
mod pipeline;
mod stats_collector;

pub use cleanup::*;
pub use container_monitor::*;
pub use disk_monitor::*;
pub use pipeline::*;
pub use stats_collector::*;

use arc_swap::ArcSwap;
use crate::api::metrics::{record_deployment_failed, record_deployment_success};
use crate::config::RuntimeConfig;
use crate::db::App;
use crate::proxy::{Backend, BasicAuthConfig, RouteTable};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use std::sync::Arc;
use tokio::sync::mpsc;

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
}

impl DeploymentEngine {
    pub fn new(
        db: DbPool,
        runtime: Arc<dyn ContainerRuntime>,
        routes: Arc<ArcSwap<RouteTable>>,
        rx: mpsc::Receiver<DeploymentJob>,
        build_limits: BuildLimits,
    ) -> Self {
        Self { db, runtime, routes, rx, build_limits }
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

            tokio::spawn(async move {
                match run_deployment(&db, runtime.clone(), &deployment_id, &app, &build_limits).await {
                    Ok(container_info) => {
                        // Record successful deployment metric
                        record_deployment_success();

                        // Mark all previous "running" deployments for this app as "replaced"
                        let _ = sqlx::query(
                            "UPDATE deployments SET status = 'replaced', finished_at = ?
                             WHERE app_id = ? AND status = 'running' AND id != ?"
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

                            // Helper to create backend with basic auth if configured
                            let create_backend = || {
                                let mut backend = Backend::new(
                                    container_info.container_id.clone(),
                                    "127.0.0.1".to_string(),
                                    port,
                                )
                                .with_healthcheck(app.healthcheck.clone());

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
                                    route_table.add_route(domain.clone(), create_backend());
                                }

                                tracing::info!(
                                    domains = ?all_domains,
                                    port = port,
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

                                route_table.add_route(domain.clone(), create_backend());
                                tracing::info!(
                                    domain = %domain,
                                    port = port,
                                    healthcheck = ?app.healthcheck,
                                    basic_auth = app.basic_auth_enabled != 0,
                                    "Proxy route updated for app {}",
                                    app.name
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Record failed deployment metric
                        record_deployment_failed();

                        tracing::error!("Deployment {} failed: {}", deployment_id, e);
                        let _ = update_deployment_status(&db, &deployment_id, "failed", Some(&e.to_string())).await;
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
            "UPDATE deployments SET status = ?, error_message = ?, finished_at = ? WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(&now)
        .bind(deployment_id)
        .execute(db)
        .await?;
    } else {
        sqlx::query("UPDATE deployments SET status = ?, error_message = ? WHERE id = ?")
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
    sqlx::query(
        "INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)",
    )
    .bind(deployment_id)
    .bind(level)
    .bind(message)
    .execute(db)
    .await?;

    Ok(())
}
