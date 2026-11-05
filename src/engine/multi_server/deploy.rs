//! The multi-server rollout: build once, distribute, rolling start with
//! per-target health checks, proxy pool updates per batch, and whole-deployment
//! rollback when the failure threshold is reached.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use arc_swap::ArcSwap;

use crate::api::metrics::{
    increment_deployments_total, observe_deployment_duration, record_deployment_failed,
    record_deployment_success,
};
use crate::crypto;
use crate::db::{
    App, DeploymentTarget, HealthcheckType, MultiServerSettings, Server, LOCAL_SERVER_ID,
};
use crate::engine::server_ops::HostTarget;
use crate::engine::{deployment_snapshots, BuildLimits, KEY_LENGTH};
use crate::notifications::{NotificationPayload, NotificationService};
use crate::proxy::RouteTable;
use crate::runtime::ContainerRuntime;
use crate::DbPool;

use super::super::{add_deployment_log, pipeline, retire, update_deployment_status};
use super::distribute::{self, DistributionStrategy, RegistryLogin, RunSpec};
use super::plan::{self, Addressing, TargetSlot};
use super::{routes, shell};

const DISTRIBUTE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const RUN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const REMOVE_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const TCP_READY_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_START_PERIOD_SECS: u32 = 300;

/// One server of the placement, resolved.
struct Host {
    name: String,
    target: HostTarget,
    addressing: Addressing,
}

/// One container slot being deployed.
struct Slot {
    slot: TargetSlot,
    row_id: String,
    label: String,
    container_name: String,
    host: Option<usize>,
    container_id: Option<String>,
    upstream: Option<String>,
    healthy: bool,
    failed: bool,
}

/// A container of an earlier deployment still serving traffic.
#[derive(Clone)]
struct Previous {
    row_id: String,
    server_id: String,
    replica_index: i64,
    container_name: Option<String>,
    upstream: Option<String>,
}

struct Ctx<'a> {
    db: &'a DbPool,
    deployment_id: &'a str,
    app: &'a App,
    local: HostTarget,
}

/// Rolled back because too many targets failed.
#[derive(Debug)]
struct RolledBack(String);

impl std::fmt::Display for RolledBack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RolledBack {}

async fn target_log(ctx: &Ctx<'_>, slot_row: &str, label: &str, level: &str, msg: &str) {
    let line = format!(
        "{} [{}] {}\n",
        chrono::Utc::now().format("%H:%M:%S"),
        level,
        msg
    );
    let _ = sqlx::query("UPDATE deployment_targets SET log = log || ? WHERE id = ?")
        .bind(&line)
        .bind(slot_row)
        .execute(ctx.db)
        .await;
    let _ = add_deployment_log(
        ctx.db,
        ctx.deployment_id,
        level,
        &format!("[{}] {}", label, msg),
    )
    .await;
}

async fn set_target(db: &DbPool, row: &str, status: &str, error: Option<&str>) {
    let terminal = matches!(status, "healthy" | "failed" | "rolled_back" | "replaced");
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "UPDATE deployment_targets SET status = ?, error_message = COALESCE(?, error_message), \
         started_at = COALESCE(started_at, ?), \
         finished_at = CASE WHEN ? THEN ? ELSE finished_at END WHERE id = ?",
    )
    .bind(status)
    .bind(error)
    .bind(&now)
    .bind(terminal)
    .bind(&now)
    .bind(row)
    .execute(db)
    .await;
}

fn decrypt(value: Option<&str>, key: Option<&[u8; KEY_LENGTH]>) -> Option<String> {
    let v = value.filter(|s| !s.is_empty())?;
    Some(crypto::decrypt_if_encrypted(v, key).unwrap_or_else(|_| v.to_string()))
}

/// Resolve each placement server. Unknown servers come back as errors so their
/// slots fail with a clear reason instead of aborting the whole rollout.
async fn resolve_hosts(
    db: &DbPool,
    server_ids: &[String],
    settings: &MultiServerSettings,
    local: &HostTarget,
    key: Option<&[u8; KEY_LENGTH]>,
) -> Vec<Result<Host, String>> {
    let mut out = Vec::new();
    for id in server_ids {
        if id == LOCAL_SERVER_ID {
            out.push(Ok(Host {
                name: "Rivetr host".to_string(),
                target: local.clone(),
                addressing: plan::local_addressing(),
            }));
            continue;
        }
        let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
        let Some(server) = server else {
            out.push(Err(format!("server {} no longer exists", id)));
            continue;
        };
        let target = HostTarget::Remote {
            host: server.ssh_host().to_string(),
            port: u16::try_from(server.port).unwrap_or(22),
            username: server.username.clone(),
            private_key: decrypt(server.ssh_private_key.as_deref(), key),
            password: decrypt(server.ssh_password.as_deref(), key),
        };
        out.push(Ok(Host {
            name: server.name.clone(),
            target,
            addressing: plan::remote_addressing(&server, settings.bind_private),
        }));
    }
    out
}

/// Labels, resources, volumes and options shared by every container.
async fn base_spec(db: &DbPool, app: &App, deployment_id: &str, image: &str) -> RunSpec {
    let volumes: Vec<String> = sqlx::query_as::<_, crate::db::Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE app_id = ?",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
    .iter()
    .map(|v| v.to_bind_mount())
    .collect();
    let json_list = |s: &Option<String>| -> Vec<String> {
        s.as_ref()
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_default()
    };
    let mut labels: BTreeMap<String, String> = app.get_container_labels().into_iter().collect();
    if let Some(arr) = app
        .custom_labels
        .as_ref()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(s).ok())
    {
        for v in arr {
            if let (Some(k), Some(val)) = (
                v.get("key").and_then(|x| x.as_str()),
                v.get("value").and_then(|x| x.as_str()),
            ) {
                if !k.is_empty() {
                    labels.insert(k.to_string(), val.to_string());
                }
            }
        }
    }
    labels.insert("rivetr.managed".into(), "true".into());
    labels.insert("rivetr.app_id".into(), app.id.clone());
    labels.insert("rivetr.deployment_id".into(), deployment_id.to_string());
    labels.insert("rivetr.multi_server".into(), "true".into());
    labels.insert(
        crate::runtime::PRIMARY_PORT_LABEL.to_string(),
        app.port.to_string(),
    );
    RunSpec {
        name: String::new(),
        image: image.to_string(),
        app_port: app.port as u16,
        bind: None,
        memory_limit: app.memory_limit.clone(),
        cpu_limit: app.cpu_limit.clone(),
        restart_policy: app.restart_policy.clone(),
        labels,
        volumes,
        extra_hosts: app.get_extra_hosts(),
        cap_add: json_list(&app.cap_add),
        cap_drop: json_list(&app.docker_cap_drop),
        privileged: app.privileged != 0,
        init: app.init_process != 0,
        shm_size: app.shm_size.clone(),
    }
}

/// Containers of earlier deployments that serve the app now: multi-server
/// targets, or for an app switching from single-host the legacy containers.
async fn previous_state(
    db: &DbPool,
    routes: &Arc<ArcSwap<RouteTable>>,
    app: &App,
    deployment_id: &str,
) -> (Vec<Previous>, Vec<String>, Vec<String>) {
    let prev: Vec<DeploymentTarget> = sqlx::query_as(
        "SELECT * FROM deployment_targets WHERE app_id = ? AND status = 'healthy' AND deployment_id != ?",
    )
    .bind(&app.id)
    .bind(deployment_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let previous: Vec<Previous> = prev
        .into_iter()
        .map(|t| Previous {
            row_id: t.id,
            server_id: t.server_id,
            replica_index: t.replica_index,
            container_name: t.container_name,
            upstream: t.upstream,
        })
        .collect();

    // Legacy single-host containers (primary and replicas) and their upstreams.
    let mut legacy_ids = Vec::new();
    let mut legacy_upstreams = Vec::new();
    if previous.is_empty() {
        let running: Option<Option<String>> = sqlx::query_scalar(
            "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' AND id != ? \
             ORDER BY started_at DESC LIMIT 1",
        )
        .bind(&app.id)
        .bind(deployment_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
        if let Some(Some(cid)) = running {
            legacy_ids.push(cid);
        }
        let replicas: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT container_id FROM app_replicas WHERE app_id = ? AND status = 'running' AND replica_index > 0",
        )
        .bind(&app.id)
        .fetch_all(db)
        .await
        .unwrap_or_default();
        legacy_ids.extend(replicas.into_iter().flatten());
        if let Some(domain) = app.get_primary_domain() {
            let table = routes.load();
            if let Some(pool) = table.pool(&domain) {
                legacy_upstreams = pool.upstreams().to_vec();
            } else if let Some(b) = table.get_backend(&domain) {
                if b.www_redirect_target.is_none() {
                    legacy_upstreams.push(b.addr());
                }
            }
        }
    }
    (previous, legacy_ids, legacy_upstreams)
}

/// Get the image onto one remote host.
async fn distribute_to(
    ctx: &Ctx<'_>,
    host: &Host,
    strategy: &DistributionStrategy,
) -> Result<(), String> {
    let out = match strategy {
        DistributionStrategy::Registry { image_ref, login } => {
            let script = distribute::registry_pull_script(image_ref, login.as_ref());
            let stdin = login.as_ref().map(|l| l.password.clone().into_bytes());
            shell::exec(&host.target, &script, stdin.as_deref(), DISTRIBUTE_TIMEOUT).await
        }
        DistributionStrategy::SshStream { image } => {
            shell::pipe_to(
                &ctx.local,
                &distribute::local_save_script(image),
                &host.target,
                distribute::REMOTE_LOAD_SCRIPT,
                DISTRIBUTE_TIMEOUT,
            )
            .await
        }
    };
    match out {
        Ok(o) if o.success() => Ok(()),
        Ok(o) => Err(o.combined()),
        Err(e) => Err(format!("{:#}", e)),
    }
}

/// Start one container and wait for it to pass its health check.
async fn start_slot(
    ctx: &Ctx<'_>,
    host: &Host,
    slot: &Slot,
    spec: &RunSpec,
    env: &[u8],
    run_pre_deploy: bool,
) -> Result<(String, String), (String, Option<String>)> {
    set_target(ctx.db, &slot.row_id, "starting", None).await;
    target_log(
        ctx,
        &slot.row_id,
        &slot.label,
        "info",
        &format!("Starting container {}", slot.container_name),
    )
    .await;

    let mut spec = spec.clone();
    spec.name = slot.container_name.clone();
    spec.bind = Some(host.addressing.bind.clone());
    let out = shell::exec(
        &host.target,
        &distribute::run_script(&spec),
        Some(env),
        RUN_TIMEOUT,
    )
    .await
    .map_err(|e| (format!("{:#}", e), None))?;
    if !out.success() {
        return Err((format!("container start failed: {}", out.combined()), None));
    }
    let (container_id, port) = distribute::parse_run_output(&out.stdout).ok_or_else(|| {
        (
            format!("could not read the published port: {}", out.combined()),
            Some(slot.container_name.clone()),
        )
    })?;
    let upstream = format!("{}:{}", host.addressing.upstream_host, port);
    let _ =
        sqlx::query("UPDATE deployment_targets SET container_id = ?, upstream = ? WHERE id = ?")
            .bind(&container_id)
            .bind(&upstream)
            .bind(&slot.row_id)
            .execute(ctx.db)
            .await;
    target_log(
        ctx,
        &slot.row_id,
        &slot.label,
        "info",
        &format!("Container started, upstream {}", upstream),
    )
    .await;

    if run_pre_deploy {
        for cmd in ctx.app.get_pre_deploy_commands() {
            target_log(
                ctx,
                &slot.row_id,
                &slot.label,
                "info",
                &format!("Pre-deploy: {}", cmd),
            )
            .await;
            let res = shell::exec(
                &host.target,
                &distribute::exec_check_script(&slot.container_name, &cmd),
                None,
                RUN_TIMEOUT,
            )
            .await;
            match res {
                Ok(o) if o.success() => {}
                Ok(o) => {
                    return Err((
                        format!("pre-deploy command failed: {}", o.combined()),
                        Some(slot.container_name.clone()),
                    ))
                }
                Err(e) => return Err((format!("{:#}", e), Some(slot.container_name.clone()))),
            }
        }
    }

    set_target(ctx.db, &slot.row_id, "checking", None).await;
    match check_health(ctx, host, slot, &upstream, port).await {
        Ok(()) => Ok((container_id, upstream)),
        Err(reason) => Err((reason, Some(slot.container_name.clone()))),
    }
}

/// The app's structured health check, run from the Rivetr host against the
/// address the proxy will use, or a TCP readiness wait when none is set.
async fn check_health(
    ctx: &Ctx<'_>,
    host: &Host,
    slot: &Slot,
    upstream: &str,
    port: u16,
) -> Result<(), String> {
    let Some(hc) = ctx.app.effective_healthcheck().filter(|h| h.enabled) else {
        return if retire::wait_for_tcp(upstream, TCP_READY_TIMEOUT).await {
            target_log(
                ctx,
                &slot.row_id,
                &slot.label,
                "info",
                "Port is accepting connections",
            )
            .await;
            Ok(())
        } else {
            Err(format!(
                "{} did not accept connections within {}s (check the firewall between the Rivetr host and this server)",
                upstream,
                TCP_READY_TIMEOUT.as_secs()
            ))
        };
    };

    let start_period = hc.start_period_secs.min(MAX_START_PERIOD_SECS);
    if start_period > 0 {
        tokio::time::sleep(Duration::from_secs(u64::from(start_period))).await;
    }
    let timeout = Duration::from_secs(u64::from(hc.timeout_secs.max(1)));
    let retries = pipeline::effective_health_check_retries(hc.retries, hc.interval_secs);
    let app_port = ctx.app.port as u16;
    if hc.check_type == HealthcheckType::Http && hc.target_port(app_port) != app_port {
        target_log(
            ctx,
            &slot.row_id,
            &slot.label,
            "warn",
            "Only the app port is published on multi-server targets; checking the app port",
        )
        .await;
    }
    let (host_part, _) = upstream.rsplit_once(':').unwrap_or((upstream, ""));
    let url = format!("{}://{}:{}{}", hc.scheme, host_part, port, hc.path);
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;

    let mut last = String::new();
    for attempt in 1..=retries {
        let outcome = if hc.check_type == HealthcheckType::Http {
            pipeline::http_attempt(&client, &url, &hc).await
        } else {
            let cmd = hc.command.clone().unwrap_or_default();
            match shell::exec(
                &host.target,
                &distribute::exec_check_script(&slot.container_name, &cmd),
                None,
                timeout + Duration::from_secs(15),
            )
            .await
            {
                Ok(o) if o.success() => Ok(()),
                Ok(o) => Err(format!("exit code {:?}: {}", o.code, o.combined())),
                Err(e) => Err(format!("{:#}", e)),
            }
        };
        match outcome {
            Ok(()) => {
                target_log(
                    ctx,
                    &slot.row_id,
                    &slot.label,
                    "info",
                    "Health check passed",
                )
                .await;
                return Ok(());
            }
            Err(reason) => {
                target_log(
                    ctx,
                    &slot.row_id,
                    &slot.label,
                    "warn",
                    &format!("Health check attempt {}/{}: {}", attempt, retries, reason),
                )
                .await;
                last = reason;
            }
        }
        if attempt < retries {
            tokio::time::sleep(Duration::from_secs(u64::from(hc.interval_secs.max(1)))).await;
        }
    }
    Err(format!(
        "health check failed after {} attempts: {}",
        retries, last
    ))
}

async fn remove_on(target: &HostTarget, names: &[String]) {
    if names.is_empty() {
        return;
    }
    if let Err(e) = shell::exec(
        target,
        &distribute::remove_script(names),
        None,
        REMOVE_TIMEOUT,
    )
    .await
    {
        tracing::warn!(error = %e, containers = ?names, "Failed to remove multi-server containers");
    }
}

/// Deploy an app to every server of its placement. Returns a short summary.
#[allow(clippy::too_many_arguments)]
async fn run(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes_table: &Arc<ArcSwap<RouteTable>>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
    snapshots: Option<&deployment_snapshots::SnapshotContext>,
) -> Result<String> {
    pipeline::bail_if_cancelled(db, deployment_id).await?;
    let placements = super::load_placements(db, &app.id).await;
    let settings = super::load_settings(db, &app.id).await;
    let slots_plan = plan::plan_slots(&placements);
    if slots_plan.is_empty() {
        anyhow::bail!("The app has no servers in its placement");
    }
    let ctx = Ctx {
        db,
        deployment_id,
        app,
        local: HostTarget::Local {
            prefer_podman: runtime.name() == "podman",
        },
    };
    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!(
            "Multi-server deployment: {} container(s) on {} server(s), batches of {}, rollback at {}% failed",
            slots_plan.len(),
            plan::server_count(&placements),
            settings.max_unavailable.max(1),
            settings.failure_threshold_pct
        ),
    )
    .await?;

    // 1. Build once.
    let (local_image, pushed_ref) = pipeline::produce_image(
        db,
        runtime.clone(),
        deployment_id,
        app,
        build_limits,
        encryption_key,
    )
    .await?;
    pipeline::bail_if_cancelled(db, deployment_id).await?;

    if app.snapshot_before_deploy != 0 {
        if let Some(ctx) = snapshots {
            deployment_snapshots::take_pre_deploy_snapshots(
                db,
                runtime.clone(),
                ctx,
                app,
                deployment_id,
            )
            .await;
        }
    }

    // 2. Resolve servers and record one target row per slot.
    let mut server_ids: Vec<String> = Vec::new();
    for s in &slots_plan {
        if !server_ids.contains(&s.server_id) {
            server_ids.push(s.server_id.clone());
        }
    }
    let resolved = resolve_hosts(db, &server_ids, &settings, &ctx.local, encryption_key).await;
    let mut hosts: Vec<Host> = Vec::new();
    let mut host_index: HashMap<String, Result<usize, String>> = HashMap::new();
    for (id, r) in server_ids.iter().zip(resolved) {
        match r {
            Ok(h) => {
                if h.addressing.public_exposure {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!(
                            "{}: the app port is published on all interfaces and reached over {}; restrict it with a firewall or use Tailscale",
                            h.name, h.addressing.upstream_host
                        ),
                    )
                    .await?;
                }
                host_index.insert(id.clone(), Ok(hosts.len()));
                hosts.push(h);
            }
            Err(e) => {
                host_index.insert(id.clone(), Err(e));
            }
        }
    }

    let batches = plan::rolling_batches(slots_plan.len(), settings.max_unavailable.max(1) as usize);
    let mut batch_of = vec![0usize; slots_plan.len()];
    for (b, members) in batches.iter().enumerate() {
        for i in members {
            batch_of[*i] = b;
        }
    }
    let mut slots: Vec<Slot> = Vec::new();
    for (i, s) in slots_plan.iter().enumerate() {
        let row_id = uuid::Uuid::new_v4().to_string();
        let host = host_index
            .get(&s.server_id)
            .and_then(|r| r.as_ref().ok().copied());
        let server_name = host
            .map(|h| hosts[h].name.clone())
            .unwrap_or_else(|| s.server_id.clone());
        let container_name =
            plan::container_name(&app.name, &s.server_id, s.replica_index, deployment_id);
        sqlx::query(
            "INSERT INTO deployment_targets (id, deployment_id, app_id, server_id, server_name, replica_index, batch, status, container_name) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)",
        )
        .bind(&row_id)
        .bind(deployment_id)
        .bind(&app.id)
        .bind(&s.server_id)
        .bind(&server_name)
        .bind(i64::from(s.replica_index))
        .bind(batch_of[i] as i64)
        .bind(&container_name)
        .execute(db)
        .await?;
        slots.push(Slot {
            slot: s.clone(),
            row_id,
            label: format!("{} #{}", server_name, s.replica_index),
            container_name,
            host,
            container_id: None,
            upstream: None,
            healthy: false,
            failed: false,
        });
    }
    for slot in slots.iter_mut() {
        if slot.host.is_none() {
            let reason = match host_index.get(&slot.slot.server_id) {
                Some(Err(e)) => e.clone(),
                _ => "server not found".to_string(),
            };
            set_target(db, &slot.row_id, "failed", Some(&reason)).await;
            target_log(&ctx, &slot.row_id, &slot.label, "error", &reason).await;
            slot.failed = true;
        }
    }

    // 3. Distribute the image to every remote server in parallel.
    let registry_login = || -> Option<RegistryLogin> {
        let user = app.registry_username.clone().filter(|s| !s.is_empty())?;
        let password = decrypt(app.registry_password.as_deref(), encryption_key)?;
        let registry = app
            .registry_url
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "docker.io".to_string());
        Some(RegistryLogin {
            registry,
            username: user,
            password,
        })
    };
    let prebuilt = app.uses_registry_image().then(|| local_image.clone());
    let (strategy, note) = distribute::choose_strategy(
        settings.distribution_mode(),
        &local_image,
        pushed_ref.as_deref(),
        prebuilt.as_deref(),
        registry_login(),
    );
    if let Some(note) = note {
        add_deployment_log(db, deployment_id, "warn", note).await?;
    }
    let run_image = distribute::run_image(&strategy).to_string();
    let remote_hosts: Vec<usize> = (0..hosts.len())
        .filter(|i| !hosts[*i].target.is_local())
        .collect();
    if !remote_hosts.is_empty() {
        update_deployment_status(db, deployment_id, "starting", None).await?;
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &match &strategy {
                DistributionStrategy::Registry { image_ref, .. } => {
                    format!(
                        "Distributing {} to {} server(s) by registry pull",
                        image_ref,
                        remote_hosts.len()
                    )
                }
                DistributionStrategy::SshStream { image } => format!(
                    "Streaming {} to {} server(s) over SSH (docker save | docker load)",
                    image,
                    remote_hosts.len()
                ),
            },
        )
        .await?;
        for slot in slots
            .iter()
            .filter(|s| s.host.is_some_and(|h| remote_hosts.contains(&h)))
        {
            set_target(db, &slot.row_id, "distributing", None).await;
        }
        let results = futures::future::join_all(
            remote_hosts
                .iter()
                .map(|h| distribute_to(&ctx, &hosts[*h], &strategy)),
        )
        .await;
        for (h, res) in remote_hosts.iter().zip(results) {
            match res {
                Ok(()) => {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("{}: image ready", hosts[*h].name),
                    )
                    .await?;
                }
                Err(reason) => {
                    let reason = format!("image distribution failed: {}", reason.trim());
                    for slot in slots.iter_mut().filter(|s| s.host == Some(*h)) {
                        set_target(db, &slot.row_id, "failed", Some(&reason)).await;
                        target_log(&ctx, &slot.row_id, &slot.label, "error", &reason).await;
                        slot.failed = true;
                    }
                }
            }
        }
    }

    // 4. Previous containers serving the app now.
    let (previous, legacy_ids, legacy_upstreams) =
        previous_state(db, routes_table, app, deployment_id).await;
    let prev_by_slot: HashMap<(String, i64), Previous> = previous
        .iter()
        .map(|p| ((p.server_id.clone(), p.replica_index), p.clone()))
        .collect();
    let previous_upstreams: Vec<String> = previous
        .iter()
        .filter_map(|p| p.upstream.clone())
        .chain(legacy_upstreams.iter().cloned())
        .collect();
    let mut serving = previous_upstreams.clone();

    let total = slots.len();
    let threshold = settings.failure_threshold_pct.clamp(0, 100) as u8;
    let env = pipeline::collect_env_vars(db, app, encryption_key, Some(deployment_id)).await?;
    let (env_content, skipped) = distribute::env_file(&env);
    if !skipped.is_empty() {
        add_deployment_log(
            db,
            deployment_id,
            "warn",
            &format!(
                "Env vars not sent to multi-server targets (multi-line values or invalid names): {}",
                skipped.join(", ")
            ),
        )
        .await?;
    }
    let spec = base_spec(db, app, deployment_id, &run_image).await;
    let env_bytes = env_content.into_bytes();

    // 5. Rolling start.
    let mut rollback_reason: Option<String> = None;
    let mut pre_deploy_done = app.get_pre_deploy_commands().is_empty();
    let failed_now = slots.iter().filter(|s| s.failed).count();
    if plan::should_rollback(failed_now, total, threshold) {
        rollback_reason = Some(format!(
            "{} of {} targets failed before start (threshold {}%)",
            failed_now, total, threshold
        ));
    }
    if rollback_reason.is_none() {
        update_deployment_status(db, deployment_id, "starting", None).await?;
    }
    for (b, members) in batches.iter().enumerate() {
        if rollback_reason.is_some() {
            break;
        }
        if pipeline::bail_if_cancelled(db, deployment_id)
            .await
            .is_err()
        {
            rollback_reason = Some("deployment was cancelled".to_string());
            break;
        }
        let runnable: Vec<usize> = members
            .iter()
            .copied()
            .filter(|i| !slots[*i].failed)
            .collect();
        if runnable.is_empty() {
            continue;
        }
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!(
                "Batch {}/{}: starting {} container(s)",
                b + 1,
                batches.len(),
                runnable.len()
            ),
        )
        .await?;
        // The first container runs the pre-deploy commands once, before the rest start.
        let mut order: Vec<Vec<usize>> = Vec::new();
        if !pre_deploy_done {
            order.push(vec![runnable[0]]);
            order.push(runnable[1..].to_vec());
        } else {
            order.push(runnable.clone());
        }
        for group in order.into_iter().filter(|g| !g.is_empty()) {
            let with_pre = !pre_deploy_done;
            let results = futures::future::join_all(group.iter().map(|i| {
                let slot = &slots[*i];
                let host = &hosts[slot.host.unwrap_or_default()];
                start_slot(&ctx, host, slot, &spec, &env_bytes, with_pre)
            }))
            .await;
            pre_deploy_done = true;
            for (i, res) in group.iter().zip(results) {
                let slot = &mut slots[*i];
                match res {
                    Ok((cid, upstream)) => {
                        set_target(db, &slot.row_id, "healthy", None).await;
                        slot.container_id = Some(cid);
                        slot.upstream = Some(upstream.clone());
                        slot.healthy = true;
                        if let Some(prev) = prev_by_slot.get(&(
                            slot.slot.server_id.clone(),
                            i64::from(slot.slot.replica_index),
                        )) {
                            if let Some(pu) = &prev.upstream {
                                serving.retain(|u| u != pu);
                            }
                        }
                        if !serving.contains(&upstream) {
                            serving.push(upstream);
                        }
                    }
                    Err((reason, started)) => {
                        slot.failed = true;
                        set_target(db, &slot.row_id, "failed", Some(&reason)).await;
                        target_log(&ctx, &slot.row_id, &slot.label, "error", &reason).await;
                        if let Some(name) = started {
                            let host = &hosts[slot.host.unwrap_or_default()];
                            remove_on(&host.target, &[name]).await;
                        }
                    }
                }
            }
        }
        // Shift traffic for this batch.
        routes::install_routes(db, routes_table, app, &serving, &settings).await;
        let failed = slots.iter().filter(|s| s.failed).count();
        if plan::should_rollback(failed, total, threshold) {
            rollback_reason = Some(format!(
                "{} of {} targets failed (rollback threshold {}%)",
                failed, total, threshold
            ));
        }
    }

    // 6a. Roll back: remove every new container, restore previous upstreams.
    if let Some(reason) = rollback_reason {
        add_deployment_log(
            db,
            deployment_id,
            "error",
            &format!("Rolling back the whole deployment: {}", reason),
        )
        .await?;
        for (h, host) in hosts.iter().enumerate() {
            let names: Vec<String> = slots
                .iter()
                .filter(|s| s.host == Some(h) && s.healthy)
                .map(|s| s.container_name.clone())
                .collect();
            remove_on(&host.target, &names).await;
        }
        for slot in slots.iter().filter(|s| s.healthy) {
            set_target(db, &slot.row_id, "rolled_back", None).await;
        }
        if previous_upstreams.is_empty() {
            let table = routes_table.load();
            for (d, _) in app.get_all_domains_with_redirects() {
                if table.pool(&d).is_some() {
                    table.remove_route(&d);
                }
            }
        } else {
            routes::install_routes(db, routes_table, app, &previous_upstreams, &settings).await;
            add_deployment_log(
                db,
                deployment_id,
                "info",
                "Previous containers keep serving traffic",
            )
            .await?;
        }
        return Err(RolledBack(format!("Rolled back: {}", reason)).into());
    }

    // 6b. Success: final pool, retire replaced containers.
    let new_healthy: Vec<String> = slots.iter().filter_map(|s| s.upstream.clone()).collect();
    let mut kept: Vec<String> = Vec::new();
    let mut kept_rows: Vec<String> = Vec::new();
    for slot in slots.iter().filter(|s| s.failed) {
        if let Some(prev) = prev_by_slot.get(&(
            slot.slot.server_id.clone(),
            i64::from(slot.slot.replica_index),
        )) {
            if let Some(u) = &prev.upstream {
                kept.push(u.clone());
                kept_rows.push(prev.row_id.clone());
                target_log(
                    &ctx,
                    &slot.row_id,
                    &slot.label,
                    "warn",
                    "Previous container on this slot keeps serving",
                )
                .await;
            }
        }
    }
    let final_set = plan::final_upstreams(&new_healthy, &kept, &previous_upstreams);
    routes::install_routes(db, routes_table, app, &final_set, &settings).await;

    // Old multi-server containers that were replaced (or whose server left the placement).
    let mut by_server: HashMap<String, Vec<String>> = HashMap::new();
    for p in previous.iter().filter(|p| !kept_rows.contains(&p.row_id)) {
        if let Some(name) = &p.container_name {
            by_server
                .entry(p.server_id.clone())
                .or_default()
                .push(name.clone());
        }
        set_target(db, &p.row_id, "replaced", None).await;
    }
    if !by_server.is_empty() {
        let retired_hosts = resolve_hosts(
            db,
            &by_server.keys().cloned().collect::<Vec<_>>(),
            &settings,
            &ctx.local,
            encryption_key,
        )
        .await;
        for ((_, names), host) in by_server.iter().zip(retired_hosts) {
            if let Ok(host) = host {
                // Let in-flight requests finish before stopping, as single-host deploys do.
                for p in previous
                    .iter()
                    .filter(|p| p.container_name.as_ref().is_some_and(|n| names.contains(n)))
                {
                    if let Some(u) = &p.upstream {
                        crate::proxy::inflight::drain(u, crate::proxy::inflight::drain_timeout())
                            .await;
                    }
                }
                remove_on(&host.target, names).await;
            }
        }
    }
    if !legacy_ids.is_empty() {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            "Retiring the single-host container(s) this app ran before",
        )
        .await?;
        retire::retire_old_containers(&*runtime, &legacy_ids, "").await;
        let _ = sqlx::query("DELETE FROM app_replicas WHERE app_id = ?")
            .bind(&app.id)
            .execute(db)
            .await;
    }

    // Post-deploy commands once, in the first healthy container.
    if let Some(first) = slots.iter().find(|s| s.healthy) {
        let host = &hosts[first.host.unwrap_or_default()];
        for cmd in app.get_post_deploy_commands() {
            target_log(
                &ctx,
                &first.row_id,
                &first.label,
                "info",
                &format!("Post-deploy: {}", cmd),
            )
            .await;
            match shell::exec(
                &host.target,
                &distribute::exec_check_script(&first.container_name, &cmd),
                None,
                RUN_TIMEOUT,
            )
            .await
            {
                Ok(o) if o.success() => {}
                Ok(o) => {
                    target_log(
                        &ctx,
                        &first.row_id,
                        &first.label,
                        "error",
                        &format!("Post-deploy command failed: {}", o.combined()),
                    )
                    .await
                }
                Err(e) => {
                    target_log(
                        &ctx,
                        &first.row_id,
                        &first.label,
                        "error",
                        &format!("Post-deploy command failed: {:#}", e),
                    )
                    .await
                }
            }
        }
    }

    let local_container = slots
        .iter()
        .find(|s| s.healthy && s.slot.server_id == LOCAL_SERVER_ID)
        .and_then(|s| s.container_id.clone());
    sqlx::query("UPDATE deployments SET container_id = ?, image_tag = ? WHERE id = ?")
        .bind(&local_container)
        .bind(&run_image)
        .bind(deployment_id)
        .execute(db)
        .await?;

    let healthy = slots.iter().filter(|s| s.healthy).count();
    let failed = slots.iter().filter(|s| s.failed).count();
    let summary = if failed == 0 {
        format!("{} of {} containers healthy", healthy, total)
    } else {
        format!(
            "{} of {} containers healthy, {} failed (below the {}% rollback threshold)",
            healthy, total, failed, threshold
        )
    };
    Ok(summary)
}

/// Run a multi-server deployment and record its outcome (status, metrics,
/// notifications), mirroring what the engine does for single-host deploys.
#[allow(clippy::too_many_arguments)]
pub async fn run_and_finalize(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    routes_table: Arc<ArcSwap<RouteTable>>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
    snapshots: Option<&deployment_snapshots::SnapshotContext>,
    deploy_start: std::time::Instant,
) {
    let notifications = NotificationService::new(db.clone());
    let result = run(
        db,
        runtime,
        &routes_table,
        deployment_id,
        app,
        build_limits,
        encryption_key,
        snapshots,
    )
    .await;
    let duration = deploy_start.elapsed().as_secs_f64();
    match result {
        Ok(summary) => {
            record_deployment_success();
            increment_deployments_total(&app.name, "success");
            observe_deployment_duration(&app.name, duration);
            let _ = add_deployment_log(
                db,
                deployment_id,
                "info",
                &format!("Deployment completed: {}", summary),
            )
            .await;
            let _ = update_deployment_status(db, deployment_id, "running", None).await;
            let _ = sqlx::query(
                "UPDATE deployments SET status = 'replaced', finished_at = ? \
                 WHERE app_id = ? AND status = 'running' AND id != ?",
            )
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&app.id)
            .bind(deployment_id)
            .execute(db)
            .await;
            let payload = NotificationPayload::deployment_event(
                crate::db::NotificationEventType::DeploymentSuccess,
                app.id.clone(),
                app.name.clone(),
                deployment_id.to_string(),
                "success".to_string(),
                format!("Deployment successful for {} ({})", app.name, summary),
                None,
            );
            if let Err(e) = notifications.send(&payload).await {
                tracing::warn!(error = %e, "Failed to send deployment_success notification");
            }
        }
        Err(e) => {
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM deployments WHERE id = ?")
                    .bind(deployment_id)
                    .fetch_optional(db)
                    .await
                    .ok()
                    .flatten();
            if status.as_deref() == Some("cancelled") {
                return;
            }
            record_deployment_failed();
            increment_deployments_total(&app.name, "failed");
            observe_deployment_duration(&app.name, duration);
            let reason = format!("{:#}", e);
            let rolled_back = e.downcast_ref::<RolledBack>().is_some();
            tracing::error!(
                "Multi-server deployment {} failed: {}",
                deployment_id,
                reason
            );
            let _ = update_deployment_status(db, deployment_id, "failed", Some(&reason)).await;
            let _ = add_deployment_log(
                db,
                deployment_id,
                "error",
                &format!("Deployment failed: {}", reason),
            )
            .await;
            let payload = NotificationPayload::deployment_event(
                crate::db::NotificationEventType::DeploymentFailed,
                app.id.clone(),
                app.name.clone(),
                deployment_id.to_string(),
                if rolled_back {
                    "auto_rollback"
                } else {
                    "failed"
                }
                .to_string(),
                format!("Deployment failed for {}", app.name),
                Some(reason),
            );
            if let Err(err) = notifications.send(&payload).await {
                tracing::warn!(error = %err, "Failed to send deployment_failed notification");
            }
        }
    }
}
