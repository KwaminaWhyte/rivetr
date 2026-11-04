//! Deploy pipeline for Docker Compose services.
//!
//! A deployment pulls the service's images (or builds them from source when a
//! build config exists), runs pre-deploy hooks, then `docker compose up -d`,
//! which recreates only the containers whose image or config changed. Every run
//! is a `service_deployments` row with its log lines, who started it and the
//! image digests per compose service before and after.
//!
//! All external commands go through [`runner::CommandRunner`] so the pipeline
//! is tested with a recording mock instead of a Docker daemon.

pub mod images;
pub mod runner;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::api::services::compose::{
    get_compose_dir, render_service_compose, write_service_compose, RenderError,
};
use crate::db::{
    service_deployment_status as status, Service, ServiceDeployment, ServiceImageMap, ServiceStatus,
};
use crate::proxy::RouteTable;
use crate::DbPool;

use images::{capture_images, changed_services};
use runner::{CliRunner, CommandOutput, CommandRunner, CommandSpec, DeployLog};

/// Everything the pipeline needs, without the whole `AppState` (so tests can
/// build one from a temp database and a mock runner).
pub struct DeployEnv {
    pub db: DbPool,
    pub data_dir: PathBuf,
    pub encryption_key: Option<[u8; 32]>,
    pub runner: Arc<dyn CommandRunner>,
    pub routes: Option<Arc<ArcSwap<RouteTable>>>,
}

impl DeployEnv {
    pub fn from_state(state: &crate::AppState) -> Self {
        Self {
            db: state.db.clone(),
            data_dir: state.config.server.data_dir.clone(),
            encryption_key: state
                .config
                .auth
                .encryption_key
                .as_ref()
                .map(|s| crate::crypto::derive_key(s)),
            runner: Arc::new(CliRunner),
            routes: Some(state.routes.clone()),
        }
    }
}

/// What to deploy and who asked for it.
#[derive(Debug, Clone, Default)]
pub struct DeployRequest {
    pub trigger: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    /// Deployment whose images should be redeployed (rollback).
    pub rollback_of: Option<String>,
    /// Commit reported by a webhook (informational until the source is fetched).
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
}

#[derive(Debug)]
pub enum QueueError {
    NotFound,
    /// Another deployment of this service is still running.
    Busy,
    Invalid(String),
    Db(String),
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Service not found"),
            Self::Busy => write!(f, "A deployment of this service is already running"),
            Self::Invalid(e) => write!(f, "{}", e),
            Self::Db(e) => write!(f, "Database error: {}", e),
        }
    }
}

/// Services with a deployment in flight in this process.
static IN_FLIGHT: std::sync::LazyLock<dashmap::DashSet<String>> =
    std::sync::LazyLock::new(dashmap::DashSet::new);

struct FlightGuard(String);

impl Drop for FlightGuard {
    fn drop(&mut self) {
        IN_FLIGHT.remove(&self.0);
    }
}

/// Record a new deployment and run it in the background. Returns the row.
pub async fn queue_deployment(
    env: Arc<DeployEnv>,
    service_id: &str,
    req: DeployRequest,
) -> Result<ServiceDeployment, QueueError> {
    let (dep, guard) = create_deployment(&env, service_id, req).await?;
    let id = dep.id.clone();
    tokio::spawn(async move {
        let _guard = guard;
        if let Err(e) = run_deployment(&env, &id).await {
            tracing::error!(deployment = %id, "Service deployment failed to run: {:#}", e);
        }
    });
    Ok(dep)
}

/// Insert the deployment row and take the per-service lock.
async fn create_deployment(
    env: &DeployEnv,
    service_id: &str,
    req: DeployRequest,
) -> Result<(ServiceDeployment, FlightGuard), QueueError> {
    let service: Option<Service> = sqlx::query_as("SELECT * FROM services WHERE id = ?")
        .bind(service_id)
        .fetch_optional(&env.db)
        .await
        .map_err(|e| QueueError::Db(e.to_string()))?;
    let service = service.ok_or(QueueError::NotFound)?;

    if !IN_FLIGHT.insert(service.id.clone()) {
        return Err(QueueError::Busy);
    }
    let guard = FlightGuard(service.id.clone());

    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM service_deployments WHERE service_id = ? AND status IN ('pending', 'building', 'deploying')",
    )
    .bind(&service.id)
    .fetch_one(&env.db)
    .await
    .map_err(|e| QueueError::Db(e.to_string()))?;
    if active > 0 {
        return Err(QueueError::Busy);
    }

    if let Some(target) = &req.rollback_of {
        let target: Option<ServiceDeployment> =
            sqlx::query_as("SELECT * FROM service_deployments WHERE id = ? AND service_id = ?")
                .bind(target)
                .bind(&service.id)
                .fetch_optional(&env.db)
                .await
                .map_err(|e| QueueError::Db(e.to_string()))?;
        let target = target
            .ok_or_else(|| QueueError::Invalid("Deployment to roll back to not found".into()))?;
        if target
            .images_after_map()
            .values()
            .all(|s| s.pin_ref().is_none())
        {
            return Err(QueueError::Invalid(
                "That deployment has no recorded image digests to roll back to".into(),
            ));
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let trigger = if req.trigger.is_empty() {
        crate::db::service_deployment_trigger::MANUAL.to_string()
    } else {
        req.trigger.clone()
    };
    sqlx::query(
        "INSERT INTO service_deployments \
         (id, service_id, trigger, triggered_by, triggered_by_name, status, commit_sha, commit_message, rollback_of, started_at) \
         VALUES (?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&service.id)
    .bind(&trigger)
    .bind(&req.user_id)
    .bind(&req.user_name)
    .bind(&req.commit_sha)
    .bind(&req.commit_message)
    .bind(&req.rollback_of)
    .bind(&now)
    .execute(&env.db)
    .await
    .map_err(|e| QueueError::Db(e.to_string()))?;

    let dep: ServiceDeployment = sqlx::query_as("SELECT * FROM service_deployments WHERE id = ?")
        .bind(&id)
        .fetch_one(&env.db)
        .await
        .map_err(|e| QueueError::Db(e.to_string()))?;
    Ok((dep, guard))
}

/// Deploy log backed by `service_deployment_logs`. Known secrets (registry
/// passwords, git tokens) are masked before a line is stored.
pub struct DbLog {
    db: DbPool,
    deployment_id: String,
    secrets: parking_lot::Mutex<Vec<String>>,
}

impl DbLog {
    pub fn new(db: DbPool, deployment_id: &str) -> Self {
        Self {
            db,
            deployment_id: deployment_id.to_string(),
            secrets: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn add_secret(&self, secret: &str) {
        if secret.len() >= 4 {
            self.secrets.lock().push(secret.to_string());
        }
    }

    fn redact(&self, message: &str) -> String {
        let mut out = message.to_string();
        for s in self.secrets.lock().iter() {
            out = out.replace(s.as_str(), "********");
        }
        out
    }
}

#[async_trait]
impl DeployLog for DbLog {
    async fn line(&self, level: &str, message: &str) {
        let message = self.redact(message);
        let _ = sqlx::query(
            "INSERT INTO service_deployment_logs (deployment_id, timestamp, level, message) VALUES (?, ?, ?, ?)",
        )
        .bind(&self.deployment_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(level)
        .bind(&message)
        .execute(&self.db)
        .await;
    }
}

/// Filled in as the pipeline runs, so a failed run still records what it saw.
#[derive(Debug, Default)]
struct Progress {
    images_before: Option<ServiceImageMap>,
    images_after: Option<ServiceImageMap>,
    image_tag: Option<String>,
    commit_sha: Option<String>,
    commit_message: Option<String>,
}

/// Run a recorded deployment to completion and store the outcome.
pub async fn run_deployment(env: &DeployEnv, deployment_id: &str) -> Result<()> {
    let dep: ServiceDeployment = sqlx::query_as("SELECT * FROM service_deployments WHERE id = ?")
        .bind(deployment_id)
        .fetch_one(&env.db)
        .await
        .context("deployment not found")?;
    let service: Service = sqlx::query_as("SELECT * FROM services WHERE id = ?")
        .bind(&dep.service_id)
        .fetch_one(&env.db)
        .await
        .context("service not found")?;

    let log = DbLog::new(env.db.clone(), deployment_id);
    let started = Instant::now();
    let mut progress = Progress {
        commit_sha: dep.commit_sha.clone(),
        commit_message: dep.commit_message.clone(),
        ..Default::default()
    };

    let result = pipeline(env, &service, &dep, &log, &mut progress).await;

    let (final_status, error) = match &result {
        Ok(()) => {
            log.info(&format!(
                "Deployment succeeded in {:.1}s",
                started.elapsed().as_secs_f64()
            ))
            .await;
            (status::SUCCEEDED, None)
        }
        Err(e) => {
            let msg = format!("{:#}", e);
            log.error(&format!("Deployment failed: {}", msg)).await;
            (status::FAILED, Some(msg))
        }
    };

    let to_json =
        |m: &Option<ServiceImageMap>| m.as_ref().and_then(|m| serde_json::to_string(m).ok());
    sqlx::query(
        "UPDATE service_deployments SET status = ?, error_message = ?, finished_at = ?, duration_ms = ?, \
         images_before = COALESCE(?, images_before), images_after = COALESCE(?, images_after), \
         image_tag = COALESCE(?, image_tag), commit_sha = COALESCE(?, commit_sha), \
         commit_message = COALESCE(?, commit_message) WHERE id = ?",
    )
    .bind(final_status)
    .bind(&error)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(started.elapsed().as_millis() as i64)
    .bind(to_json(&progress.images_before))
    .bind(to_json(&progress.images_after))
    .bind(&progress.image_tag)
    .bind(&progress.commit_sha)
    .bind(&progress.commit_message)
    .bind(deployment_id)
    .execute(&env.db)
    .await?;
    Ok(())
}

async fn set_status(env: &DeployEnv, deployment_id: &str, new_status: &str) {
    let _ = sqlx::query("UPDATE service_deployments SET status = ? WHERE id = ?")
        .bind(new_status)
        .bind(deployment_id)
        .execute(&env.db)
        .await;
}

/// `docker compose -p <project> <args...>` in the service's compose directory.
pub fn compose_cmd(compose_dir: &Path, project: &str, args: &[&str]) -> CommandSpec {
    let mut all = vec!["compose", "-p", project];
    all.extend_from_slice(args);
    CommandSpec::new("docker", &all).cwd(compose_dir)
}

fn summarize_images(map: &ServiceImageMap) -> String {
    if map.is_empty() {
        return "none (no containers yet)".to_string();
    }
    map.iter()
        .map(|(svc, s)| {
            let id = s
                .digest
                .as_deref()
                .and_then(|d| d.split('@').nth(1))
                .or(s.image_id.as_deref())
                .map(short_digest)
                .unwrap_or_else(|| "unknown".to_string());
            format!("{}={} ({})", svc, s.image, id)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn short_digest(d: &str) -> String {
    let hex = d.strip_prefix("sha256:").unwrap_or(d);
    format!("sha256:{}", &hex[..hex.len().min(12)])
}

async fn pipeline(
    env: &DeployEnv,
    service: &Service,
    dep: &ServiceDeployment,
    log: &DbLog,
    progress: &mut Progress,
) -> Result<()> {
    let runner = env.runner.as_ref();
    let compose_dir = get_compose_dir(&env.data_dir, &service.name);
    let project = service.compose_project_name();

    log.info(&format!(
        "Deploying service {} (trigger: {}{})",
        service.name,
        dep.trigger,
        dep.triggered_by_name
            .as_deref()
            .map(|n| format!(", by {}", n))
            .unwrap_or_default()
    ))
    .await;
    set_status(env, &dep.id, status::DEPLOYING).await;

    // 1. What runs now.
    match capture_images(runner, &compose_dir, &project).await {
        Ok(before) => {
            log.info(&format!("Current images: {}", summarize_images(&before)))
                .await;
            progress.images_before = Some(before);
        }
        Err(e) => {
            log.warn(&format!("Could not read the current images: {:#}", e))
                .await;
        }
    }

    // 2. Render and write the compose file, keeping the previous one so a
    //    failure before `up` leaves the stack exactly as it was.
    let compose_file = compose_dir.join("docker-compose.yml");
    let previous_compose = tokio::fs::read_to_string(&compose_file).await.ok();
    let content = render_service_compose(&env.db, service)
        .await
        .map_err(|e| match e {
            RenderError::Substitution(e) => anyhow::anyhow!("Compose variables: {}", e),
            other => anyhow::anyhow!("{}", other),
        })?;
    write_service_compose(&env.data_dir, service, &content)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let restore = |reason: String| {
        let compose_file = compose_file.clone();
        let previous = previous_compose.clone();
        async move {
            if let Some(prev) = previous {
                let _ = tokio::fs::write(&compose_file, prev).await;
            }
            anyhow::anyhow!(reason)
        }
    };

    // 3. Pull. Images that only exist locally (built by hand or by Rivetr
    //    without a registry) cannot be pulled; `up` then uses the local copy.
    log.info("Pulling images (docker compose pull)").await;
    let pull = run_logged(
        runner,
        compose_cmd(&compose_dir, &project, &["pull", "--ignore-pull-failures"]).streamed(),
        log,
    )
    .await?;
    if !pull.success {
        return Err(restore(format!("docker compose pull failed: {}", pull.error_tail())).await);
    }

    // 4. Recreate what changed.
    log.info("Recreating changed containers (docker compose up -d)")
        .await;
    let up = run_logged(
        runner,
        compose_cmd(&compose_dir, &project, &["up", "-d", "--remove-orphans"]).streamed(),
        log,
    )
    .await?;
    if !up.success {
        let msg = format!("docker compose up failed: {}", up.error_tail());
        let _ = sqlx::query(
            "UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(ServiceStatus::Failed.to_string())
        .bind(&msg)
        .bind(&service.id)
        .execute(&env.db)
        .await;
        anyhow::bail!(msg);
    }

    // 5. What runs now.
    match capture_images(runner, &compose_dir, &project).await {
        Ok(after) => {
            if let Some(before) = &progress.images_before {
                let changed = changed_services(before, &after);
                if changed.is_empty() {
                    log.info("No image changed").await;
                } else {
                    log.info(&format!("Updated: {}", changed.join(", "))).await;
                }
            }
            log.info(&format!("Running images: {}", summarize_images(&after)))
                .await;
            progress.images_after = Some(after);
        }
        Err(e) => {
            log.warn(&format!("Could not read the new images: {:#}", e))
                .await;
        }
    }

    sqlx::query(
        "UPDATE services SET status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(ServiceStatus::Running.to_string())
    .bind(&service.id)
    .execute(&env.db)
    .await?;

    if let (Some(routes), Some(domain)) = (&env.routes, service.domain.as_deref()) {
        if !domain.is_empty() {
            let backend = crate::proxy::Backend::new(
                format!("rivetr-svc-{}", service.name),
                "127.0.0.1".to_string(),
                service.port as u16,
            );
            routes.load().add_route(domain.to_string(), backend);
        }
    }

    Ok(())
}

/// Run a command, logging its command line first. Spawn errors become `Err`.
async fn run_logged(
    runner: &dyn CommandRunner,
    spec: CommandSpec,
    log: &DbLog,
) -> Result<CommandOutput> {
    log.info(&format!("$ {}", spec.display())).await;
    runner.run(&spec, Some(log)).await
}
