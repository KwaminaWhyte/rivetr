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

pub mod build;
pub mod hooks;
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
    get_compose_dir, render_service_compose_with, write_service_compose, write_service_env_file,
    RenderError, ENV_FILE_NAME,
};
use crate::db::{
    service_deployment_status as status, Service, ServiceDeployConfig, ServiceDeployment,
    ServiceImageMap, ServiceStatus,
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
    /// Problem that did not fail the deployment (a post-deploy hook).
    warning: Option<String>,
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
            (status::SUCCEEDED, progress.warning.clone())
        }
        Err(e) => {
            let msg = log.redact(&format!("{:#}", e));
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

/// Load a service's deploy config; services without one deploy pull-only.
pub async fn load_deploy_config(db: &DbPool, service_id: &str) -> Result<ServiceDeployConfig> {
    let cfg: Option<ServiceDeployConfig> =
        sqlx::query_as("SELECT * FROM service_deploy_configs WHERE service_id = ?")
            .bind(service_id)
            .fetch_optional(db)
            .await?;
    Ok(cfg.unwrap_or_else(|| ServiceDeployConfig::default_for(service_id)))
}

async fn pipeline(
    env: &DeployEnv,
    service: &Service,
    dep: &ServiceDeployment,
    log: &DbLog,
    progress: &mut Progress,
) -> Result<()> {
    let mut cleanup: Option<PathBuf> = None;
    let result = pipeline_inner(env, service, dep, log, progress, &mut cleanup).await;
    if let Some(dir) = cleanup {
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
    result
}

async fn pipeline_inner(
    env: &DeployEnv,
    service: &Service,
    dep: &ServiceDeployment,
    log: &DbLog,
    progress: &mut Progress,
    cleanup: &mut Option<PathBuf>,
) -> Result<()> {
    let runner = env.runner.as_ref();
    let cfg = load_deploy_config(&env.db, &service.id).await?;
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

    // A rollback redeploys the images a previous deployment ran, pinned by
    // digest, so `latest` moving since then does not matter.
    let rollback_pins = match &dep.rollback_of {
        Some(target_id) => {
            let target: ServiceDeployment =
                sqlx::query_as("SELECT * FROM service_deployments WHERE id = ?")
                    .bind(target_id)
                    .fetch_one(&env.db)
                    .await
                    .context("Deployment to roll back to not found")?;
            let target_images = target.images_after_map();
            if target_images.values().all(|s| s.pin_ref().is_none()) {
                anyhow::bail!("That deployment has no recorded image digests");
            }
            let current = progress.images_before.clone().unwrap_or_default();
            let pins = rollback_pins_for(&target_images, &current);
            log.info(&format!(
                "Rolling back to deployment {} ({})",
                &target.id[..target.id.len().min(8)],
                pins.iter()
                    .map(|(s, p)| format!("{}={}", s, p))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .await;
            progress.image_tag = target.image_tag.clone();
            progress.commit_sha = target.commit_sha.clone();
            progress.commit_message = target.commit_message.clone();
            Some(pins)
        }
        None => None,
    };
    let is_rollback = rollback_pins.is_some();

    // 2. Build from source when configured.
    let mut built_repos = std::collections::BTreeSet::new();
    if is_rollback {
        log.info("Rollback: skipping build and hooks").await;
    } else if cfg.builds_from_source() {
        set_status(env, &dep.id, status::BUILDING).await;
        let source = build::fetch_source(env, &cfg, &service.name, &dep.id, log).await?;
        *cleanup = source.cleanup.clone();
        progress.image_tag = Some(source.tag.clone());
        if source.commit_sha.is_some() {
            progress.commit_sha = source.commit_sha.clone();
            progress.commit_message = source.commit_message.clone();
        }
        let password = cfg.registry_password.as_deref().map(|p| {
            crate::crypto::decrypt_if_encrypted(p, env.encryption_key.as_ref())
                .unwrap_or_else(|_| p.to_string())
        });
        built_repos =
            build::build_and_push(runner, &cfg, password.as_deref(), &source, log).await?;
        set_status(env, &dep.id, status::DEPLOYING).await;
    } else {
        log.info("No build source configured; deploying by pulling the images")
            .await;
    }

    // 3. Render and write the compose file, keeping the previous one so a
    //    failure before `up` leaves the stack exactly as it was.
    let compose_file = compose_dir.join("docker-compose.yml");
    let previous_compose = tokio::fs::read_to_string(&compose_file).await.ok();
    let pins = rollback_pins.clone().unwrap_or_default();
    let content =
        render_service_compose_with(&env.db, service, Some(&pins), env.encryption_key.as_ref())
            .await
            .map_err(|e| match e {
                RenderError::Substitution(e) => anyhow::anyhow!("Compose variables: {}", e),
                other => anyhow::anyhow!("{}", other),
            })?;
    let env_file = compose_dir.join(ENV_FILE_NAME);
    let previous_env = tokio::fs::read(&env_file).await.ok();
    write_service_compose(&env.data_dir, service, &content)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_service_env_file(&env.db, &env.data_dir, service, env.encryption_key.as_ref())
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let restore = |reason: String| {
        let compose_file = compose_file.clone();
        let previous = previous_compose.clone();
        let env_file = env_file.clone();
        let previous_env = previous_env.clone();
        async move {
            if let Some(prev) = previous {
                let _ = tokio::fs::write(&compose_file, prev).await;
            }
            match previous_env {
                Some(prev) => {
                    let _ = tokio::fs::write(&env_file, prev).await;
                }
                None => {
                    let _ = tokio::fs::remove_file(&env_file).await;
                }
            }
            anyhow::anyhow!(reason)
        }
    };

    // 4. Pull. Images that only exist locally (built by hand or by Rivetr
    //    without a registry) cannot be pulled; `up` then uses the local copy.
    //    Images this deployment just built are not pulled at all.
    let pull_targets = if is_rollback {
        // Only pinned images that are no longer on this server need a pull.
        let mut missing = vec![];
        for (svc, pin) in &pins {
            let present = runner
                .run(
                    &CommandSpec::new("docker", &["image", "inspect", pin]),
                    None,
                )
                .await
                .map(|o| o.success)
                .unwrap_or(false);
            if !present {
                missing.push(svc.clone());
            }
        }
        Some(missing)
    } else {
        build::services_to_pull(&content, &built_repos)
    };
    if matches!(&pull_targets, Some(list) if list.is_empty()) {
        log.info(if is_rollback {
            "Nothing to pull: the pinned images are on this server"
        } else {
            "Nothing to pull: every image was just built"
        })
        .await;
    } else {
        let mut args: Vec<String> = vec!["pull".into(), "--ignore-pull-failures".into()];
        args.extend(pull_targets.unwrap_or_default());
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        log.info("Pulling images (docker compose pull)").await;
        let pull = run_logged(
            runner,
            compose_cmd(&compose_dir, &project, &arg_refs).streamed(),
            log,
        )
        .await?;
        if !pull.success {
            return Err(
                restore(format!("docker compose pull failed: {}", pull.error_tail())).await,
            );
        }
    }

    // 5. Pre-deploy hooks (migrations). A failure stops here: nothing has
    //    been recreated, and the compose file and `latest` tags are restored.
    let pre_hooks = if is_rollback { vec![] } else { cfg.pre_hooks() };
    if !pre_hooks.is_empty() {
        if let Err(msg) = hooks::run_hooks(
            runner,
            &compose_dir,
            &project,
            &dep.id,
            "pre",
            &pre_hooks,
            log,
        )
        .await
        {
            revert_latest_tags(runner, &cfg, &built_repos, progress, log).await;
            return Err(restore(format!("{}; the running containers were not changed", msg)).await);
        }
    }

    // 6. Recreate what changed.
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

    // 7. What runs now.
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

    // Keep the pins so start/restart run the rolled-back images too; a
    // regular deploy clears them and the stack follows its tags again.
    let stored = if pins.is_empty() {
        None
    } else {
        serde_json::to_string(&pins).ok()
    };
    if stored.is_some() || cfg.pinned_images.is_some() {
        sqlx::query(
            "INSERT INTO service_deploy_configs (service_id, pinned_images) VALUES (?, ?) \
             ON CONFLICT(service_id) DO UPDATE SET pinned_images = excluded.pinned_images, \
             updated_at = datetime('now')",
        )
        .bind(&service.id)
        .bind(&stored)
        .execute(&env.db)
        .await?;
        if stored.is_none() {
            log.info("Cleared the image pins of an earlier rollback")
                .await;
        }
    }

    // 8. Post-deploy hooks. The new containers are already running, so a
    //    failure is reported on the deployment but does not fail it.
    let post_hooks = if is_rollback {
        vec![]
    } else {
        cfg.post_hooks()
    };
    if !post_hooks.is_empty() {
        if let Err(msg) = hooks::run_hooks(
            runner,
            &compose_dir,
            &project,
            &dep.id,
            "post",
            &post_hooks,
            log,
        )
        .await
        {
            log.error(&format!("{}; the new containers stay up", msg))
                .await;
            progress.warning = Some(msg);
        }
    }

    Ok(())
}

/// After a failed pre-deploy hook, point `<repo>:latest` back at the image the
/// stack was running, so a later restart does not start the unmigrated build.
async fn revert_latest_tags(
    runner: &dyn CommandRunner,
    cfg: &ServiceDeployConfig,
    built_repos: &std::collections::BTreeSet<String>,
    progress: &Progress,
    log: &DbLog,
) {
    let Some(before) = &progress.images_before else {
        return;
    };
    for repo in built_repos {
        let previous = before
            .values()
            .find(|s| images::image_repo(&s.image) == repo)
            .and_then(|s| s.image_id.clone());
        let Some(previous) = previous else {
            continue;
        };
        let latest = format!("{}:latest", repo);
        let tag = runner
            .run(
                &CommandSpec::new("docker", &["tag", &previous, &latest]),
                None,
            )
            .await;
        if !matches!(tag, Ok(ref o) if o.success) {
            log.warn(&format!(
                "Could not point {} back at the previous image",
                latest
            ))
            .await;
            continue;
        }
        if cfg.push_images != 0 {
            let _ = runner
                .run(&CommandSpec::new("docker", &["push", &latest]), Some(log))
                .await;
        }
        log.info(&format!("{} points at the previous image again", latest))
            .await;
    }
}

/// Pins for a rollback: the target deployment's image (by digest, else by
/// local image ID) for every compose service that runs a different image now.
/// Services already on the target image keep their compose reference, so
/// they are not recreated.
pub fn rollback_pins_for(
    target: &ServiceImageMap,
    current: &ServiceImageMap,
) -> std::collections::BTreeMap<String, String> {
    target
        .iter()
        .filter(|(svc, state)| {
            let same = current
                .get(*svc)
                .map(|c| c.image_id.is_some() && c.image_id == state.image_id)
                .unwrap_or(false);
            !same
        })
        .filter_map(|(svc, state)| state.pin_ref().map(|p| (svc.clone(), p)))
        .collect()
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

/// Services with a git source and auto deploy on whose repo matches one of
/// `repo_urls` and whose branch is `branch`.
pub async fn services_for_push(db: &DbPool, repo_urls: &[&str], branch: &str) -> Vec<String> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT service_id, git_url FROM service_deploy_configs \
         WHERE source_type = 'git' AND auto_deploy = 1 AND branch = ?",
    )
    .bind(branch)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let wanted: Vec<String> = repo_urls
        .iter()
        .filter(|u| !u.is_empty())
        .map(|u| build::normalize_git_url(u))
        .collect();
    rows.into_iter()
        .filter(|(_, url)| {
            url.as_deref()
                .map(|u| wanted.contains(&build::normalize_git_url(u)))
                .unwrap_or(false)
        })
        .map(|(id, _)| id)
        .collect()
}

/// Queue a webhook deployment for every compose service built from the pushed
/// repo and branch. Returns how many were queued.
pub async fn queue_push_deploys(
    state: &crate::AppState,
    repo_urls: &[&str],
    branch: &str,
    commit_sha: Option<String>,
    commit_message: Option<String>,
) -> usize {
    let ids = services_for_push(&state.db, repo_urls, branch).await;
    if ids.is_empty() {
        return 0;
    }
    let env = Arc::new(DeployEnv::from_state(state));
    let mut queued = 0;
    for id in ids {
        // Team-wide freeze windows block compose auto-deploys; the skip is
        // recorded in the service's deployment history.
        let team_id: Option<String> =
            sqlx::query_scalar::<_, Option<String>>("SELECT team_id FROM services WHERE id = ?")
                .bind(&id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten()
                .flatten();
        match crate::engine::freeze::check_team(
            &state.db,
            team_id.as_deref(),
            crate::engine::freeze::DeployTrigger::ComposeAutoDeploy,
        )
        .await
        {
            Ok(Some(hit)) => {
                if let Err(e) = crate::engine::freeze::record_skipped_service_deployment(
                    &state.db,
                    &id,
                    commit_sha.as_deref(),
                    commit_message.as_deref(),
                    &hit,
                )
                .await
                {
                    tracing::warn!(service = %id, "Failed to record frozen service deployment: {}", e);
                }
                tracing::info!(service = %id, "Push deploy skipped: {}", hit.reason());
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(service = %id, "Freeze window check failed, allowing deploy: {}", e)
            }
        }

        let req = DeployRequest {
            trigger: crate::db::service_deployment_trigger::WEBHOOK.to_string(),
            user_name: Some("push webhook".to_string()),
            commit_sha: commit_sha.clone(),
            commit_message: commit_message.clone(),
            ..Default::default()
        };
        match queue_deployment(env.clone(), &id, req).await {
            Ok(dep) => {
                tracing::info!(service = %id, deployment = %dep.id, "Queued service deployment from push");
                queued += 1;
            }
            Err(e) => {
                tracing::warn!(service = %id, "Push did not start a service deployment: {}", e)
            }
        }
    }
    queued
}
