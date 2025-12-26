mod pipeline;

pub use pipeline::*;

use arc_swap::ArcSwap;
use crate::db::App;
use crate::proxy::{Backend, RouteTable};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub type DeploymentJob = (String, App); // (deployment_id, app)

pub struct DeploymentEngine {
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes: Arc<ArcSwap<RouteTable>>,
    rx: mpsc::Receiver<DeploymentJob>,
}

impl DeploymentEngine {
    pub fn new(
        db: DbPool,
        runtime: Arc<dyn ContainerRuntime>,
        routes: Arc<ArcSwap<RouteTable>>,
        rx: mpsc::Receiver<DeploymentJob>,
    ) -> Self {
        Self { db, runtime, routes, rx }
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

            tokio::spawn(async move {
                match run_deployment(&db, runtime.clone(), &deployment_id, &app).await {
                    Ok(container_info) => {
                        // Update proxy routes on successful deployment
                        if let Some(domain) = &app.domain {
                            if let Some(port) = container_info.port {
                                let backend = Backend::new(
                                    container_info.container_id.clone(),
                                    "127.0.0.1".to_string(),
                                    port,
                                );
                                routes.load().add_route(domain.clone(), backend);
                                tracing::info!(
                                    domain = %domain,
                                    port = port,
                                    "Proxy route updated for app {}",
                                    app.name
                                );
                            }
                        }
                    }
                    Err(e) => {
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
