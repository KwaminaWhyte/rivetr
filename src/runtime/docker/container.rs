use anyhow::{Context, Result};
use bollard::container::LogOutput;
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{
    ContainerCreateBody, ContainerStatsResponse, ContainerSummaryStateEnum, ContainerUpdateBody,
    EndpointSettings, NetworkConnectRequest, NetworkCreateRequest, NetworkDisconnectRequest,
};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, InspectContainerOptionsBuilder,
    ListContainersOptionsBuilder, PruneImagesOptionsBuilder, RemoveContainerOptionsBuilder,
    RemoveImageOptionsBuilder, RenameContainerOptionsBuilder, ResizeExecOptionsBuilder,
    StatsOptionsBuilder, StopContainerOptionsBuilder,
};
use bytes::Bytes;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::runtime::{
    CommandResult, ContainerInfo, ContainerStats, DiskUsage, ExecConfig, ExecHandle, RegistryAuth,
    RunConfig, TtySize,
};

use super::DockerRuntime;

pub(super) fn parse_memory(s: &str) -> Option<i64> {
    let s = s.to_lowercase();
    if s.ends_with("gb") {
        s.trim_end_matches("gb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else if s.ends_with("g") {
        s.trim_end_matches("g")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else if s.ends_with("mb") {
        s.trim_end_matches("mb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else if s.ends_with("m") {
        s.trim_end_matches("m")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else if s.ends_with("kb") {
        s.trim_end_matches("kb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024)
    } else if s.ends_with("k") {
        s.trim_end_matches("k")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024)
    } else if s.ends_with("b") {
        s.trim_end_matches("b").parse().ok()
    } else {
        // Assume raw bytes if no suffix
        s.parse().ok()
    }
}

fn parse_cpu(s: &str) -> Option<i64> {
    s.parse::<f64>().ok().map(|n| (n * 1_000_000_000.0) as i64)
}

/// Parse a human-readable size string like "128m", "1g", "256k" into bytes.
pub fn parse_shm_size(s: &str) -> Option<i64> {
    parse_memory(s)
}

pub async fn run(runtime: &DockerRuntime, config: &RunConfig) -> Result<String> {
    let env: Vec<String> = config
        .env
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let mut port_bindings: HashMap<String, Option<Vec<bollard::service::PortBinding>>> =
        HashMap::new();
    let mut exposed_ports: Vec<String> = Vec::new();

    // Add primary port (legacy).
    // Pin the host port: a dynamic binding (host port 0) makes Docker hand out a
    // fresh port on every start, which silently orphans the proxy route that was
    // registered against the previous port. That is how an app went down after a
    // reboot: the route pointed at 32769 while the container had come back on 32771.
    let primary_port_binding = format!("{}/tcp", config.port);
    // Keys whose host port Rivetr picked itself; re-picked on a port conflict.
    let mut auto_pinned: Vec<String> = vec![primary_port_binding.clone()];
    port_bindings.insert(
        primary_port_binding.clone(),
        Some(vec![bollard::service::PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: allocate_host_port().map(|p| p.to_string()),
        }]),
    );
    exposed_ports.push(primary_port_binding);

    // Add additional port mappings
    for mapping in &config.port_mappings {
        let port_key = format!("{}/{}", mapping.container_port, mapping.protocol);
        let host_port = if mapping.host_port == 0 {
            None // Auto-assign
        } else {
            Some(mapping.host_port.to_string())
        };

        port_bindings.insert(
            port_key.clone(),
            Some(vec![bollard::service::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port,
            }]),
        );
        exposed_ports.push(port_key);
    }

    // Extra container ports routed to by individual domains. Pin the host port
    // for the same reason as the primary port: a dynamic binding changes on every
    // start and strands the per-domain route that was built from the old port.
    // A port already listed above keeps its existing binding.
    for port in &config.extra_ports {
        let key = format!("{}/tcp", port);
        if port_bindings.contains_key(&key) {
            continue;
        }
        port_bindings.insert(
            key.clone(),
            Some(vec![bollard::service::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: allocate_host_port().map(|p| p.to_string()),
            }]),
        );
        auto_pinned.push(key.clone());
        exposed_ports.push(key);
    }

    // Convert extra_hosts to the format Docker expects
    let extra_hosts: Option<Vec<String>> = if config.extra_hosts.is_empty() {
        None
    } else {
        Some(config.extra_hosts.clone())
    };

    // Convert binds (volume mounts) to the format Docker expects
    let binds: Option<Vec<String>> = if config.binds.is_empty() {
        None
    } else {
        Some(config.binds.clone())
    };

    // Map the app's restart_policy string to the Docker API enum.
    // Crash loops are detected by monitoring Docker's restart_count field.
    let (policy_name, max_retry) = match config.restart_policy.as_str() {
        "always" => (bollard::service::RestartPolicyNameEnum::ALWAYS, None),
        "on-failure" => (bollard::service::RestartPolicyNameEnum::ON_FAILURE, Some(5)),
        "never" => (bollard::service::RestartPolicyNameEnum::EMPTY, None),
        _ => (
            bollard::service::RestartPolicyNameEnum::UNLESS_STOPPED,
            None,
        ), // default: unless-stopped
    };
    let restart_policy = bollard::service::RestartPolicy {
        name: Some(policy_name),
        maximum_retry_count: max_retry,
    };

    // Build device mappings
    let devices: Option<Vec<bollard::models::DeviceMapping>> = if config.devices.is_empty() {
        None
    } else {
        Some(
            config
                .devices
                .iter()
                .map(|d| {
                    let parts: Vec<&str> = d.splitn(2, ':').collect();
                    bollard::models::DeviceMapping {
                        path_on_host: Some(parts[0].to_string()),
                        path_in_container: Some(
                            parts.get(1).copied().unwrap_or(parts[0]).to_string(),
                        ),
                        cgroup_permissions: Some("rwm".to_string()),
                    }
                })
                .collect(),
        )
    };

    let cap_add: Option<Vec<String>> = if config.cap_add.is_empty() {
        None
    } else {
        Some(config.cap_add.clone())
    };

    let cap_drop: Option<Vec<String>> = if config.cap_drop.is_empty() {
        None
    } else {
        Some(config.cap_drop.clone())
    };

    let security_opt: Option<Vec<String>> = if config.security_opt.is_empty() {
        None
    } else {
        Some(config.security_opt.clone())
    };

    // Parse ulimits: "nofile=1024:1024" → ResourcesUlimits { name, soft, hard }
    let ulimits: Option<Vec<bollard::service::ResourcesUlimits>> = if config.ulimits.is_empty() {
        None
    } else {
        let parsed: Vec<bollard::service::ResourcesUlimits> = config
            .ulimits
            .iter()
            .filter_map(|u| {
                let (name, rest) = u.split_once('=')?;
                let (soft_str, hard_str) = rest.split_once(':')?;
                let soft: i64 = soft_str.parse().ok()?;
                let hard: i64 = hard_str.parse().ok()?;
                Some(bollard::service::ResourcesUlimits {
                    name: Some(name.to_string()),
                    soft: Some(soft),
                    hard: Some(hard),
                })
            })
            .collect();
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    };

    // GPU device requests
    let device_requests: Option<Vec<bollard::service::DeviceRequest>> =
        config.gpus.as_ref().map(|gpus| {
            if gpus == "all" {
                vec![bollard::service::DeviceRequest {
                    driver: Some("nvidia".to_string()),
                    count: Some(-1),
                    device_ids: None,
                    capabilities: Some(vec![vec!["gpu".to_string()]]),
                    options: None,
                }]
            } else {
                let ids_str = gpus.trim_start_matches("device=");
                let ids: Vec<String> = ids_str.split(',').map(|s| s.trim().to_string()).collect();
                vec![bollard::service::DeviceRequest {
                    driver: Some("nvidia".to_string()),
                    count: None,
                    device_ids: Some(ids),
                    capabilities: Some(vec![vec!["gpu".to_string()]]),
                    options: None,
                }]
            }
        });

    // Host-protection: fall back to the runtime-wide default memory cap when this
    // container sets none, so a runaway container is OOM-killed at the cap instead
    // of exhausting host RAM. memory_swap = memory disables swap (no escape past
    // the cap). pids_limit guards against fork bombs; oom_score_adj biases the
    // kernel OOM killer toward the container and away from Rivetr/dockerd.
    let memory = config
        .memory_limit
        .as_ref()
        .and_then(|m| parse_memory(m))
        .or_else(|| {
            runtime
                .defaults
                .default_memory
                .as_deref()
                .and_then(parse_memory)
        });

    // Host-protection: cap container log growth via the json-file driver's
    // max-size/max-file so unbounded logs cannot fill the host disk. Left unset
    // (Docker default = unbounded) when either default is disabled.
    let log_config = match (
        runtime.defaults.log_max_size.as_deref(),
        runtime.defaults.log_max_file,
    ) {
        (Some(size), Some(file)) => {
            let mut opts = HashMap::new();
            opts.insert("max-size".to_string(), size.to_string());
            opts.insert("max-file".to_string(), file.to_string());
            Some(bollard::service::HostConfigLogConfig {
                typ: Some("json-file".into()),
                config: Some(opts),
            })
        }
        _ => None,
    };

    let host_config = bollard::service::HostConfig {
        port_bindings: Some(port_bindings),
        memory,
        memory_swap: memory, // equal to memory => swap disabled for this container
        nano_cpus: config.cpu_limit.as_ref().and_then(|c| parse_cpu(c)),
        pids_limit: runtime.defaults.pids_limit,
        oom_score_adj: runtime.defaults.oom_score_adj,
        log_config,
        extra_hosts,
        binds,
        restart_policy: Some(restart_policy),
        privileged: Some(config.privileged),
        cap_add,
        cap_drop,
        devices,
        shm_size: config.shm_size,
        init: Some(config.init),
        security_opt,
        ulimits,
        device_requests,
        ..Default::default()
    };

    let final_env = env;

    // Set up container labels: merge existing labels with custom_labels
    let mut merged_labels = config.labels.clone();
    for (k, v) in &config.custom_labels {
        merged_labels.insert(k.clone(), v.clone());
    }
    merged_labels.insert(
        crate::runtime::PRIMARY_PORT_LABEL.to_string(),
        config.port.to_string(),
    );
    let labels: Option<HashMap<String, String>> = if merged_labels.is_empty() {
        None
    } else {
        Some(merged_labels)
    };

    // A mapping that repeats the primary port must not list it twice.
    exposed_ports.sort();
    exposed_ports.dedup();

    let mut container_config = ContainerCreateBody {
        image: Some(config.image.clone()),
        env: Some(final_env),
        exposed_ports: Some(exposed_ports),
        host_config: Some(host_config),
        labels,
        cmd: config.cmd.clone(),
        healthcheck: config.healthcheck.as_ref().map(docker_health_config),
        ..Default::default()
    };

    let options = CreateContainerOptionsBuilder::new()
        .name(&config.name)
        .build();

    // A pinned port is picked by binding to port 0 briefly, so another process can
    // take it before Docker binds it for real. Rather than failing the deploy, re-pick
    // the ports Rivetr chose and try again (explicit user mappings are never changed).
    let mut container_id: Option<String> = None;
    let mut last_error: Option<anyhow::Error> = None;
    for attempt in 1..=3 {
        match runtime
            .client
            .create_container(Some(options.clone()), container_config.clone())
            .await
        {
            Ok(response) => match runtime.client.start_container(&response.id, None).await {
                Ok(()) => {
                    container_id = Some(response.id);
                    break;
                }
                Err(e) if attempt < 3 && is_port_conflict(&e) => {
                    let _ = runtime
                        .client
                        .remove_container(
                            &response.id,
                            Some(
                                bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                                    .force(true)
                                    .build(),
                            ),
                        )
                        .await;
                    repin_ports(&mut container_config, &auto_pinned);
                    tracing::warn!(
                        attempt,
                        name = %config.name,
                        "Host port was taken between allocation and start; retrying with new ports: {}",
                        e
                    );
                    last_error = Some(anyhow::anyhow!("Failed to start container: {}", e));
                }
                Err(e) => {
                    let _ = runtime
                        .client
                        .remove_container(
                            &response.id,
                            Some(
                                bollard::query_parameters::RemoveContainerOptionsBuilder::new()
                                    .force(true)
                                    .build(),
                            ),
                        )
                        .await;
                    last_error = Some(anyhow::anyhow!("Failed to start container: {}", e));
                    break;
                }
            },
            Err(e) if attempt < 3 && is_port_conflict(&e) => {
                repin_ports(&mut container_config, &auto_pinned);
                tracing::warn!(
                    attempt,
                    name = %config.name,
                    "Host port was taken between allocation and create; retrying with new ports: {}",
                    e
                );
                last_error = Some(anyhow::anyhow!("Failed to create container: {}", e));
            }
            Err(e) => {
                last_error = Some(anyhow::anyhow!("Failed to create container: {}", e));
                break;
            }
        }
    }
    let Some(container_id) = container_id else {
        return Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to create container")));
    };

    let response_id = container_id;

    // Connect the container to the primary network.
    // If a destination network is specified in RunConfig, use that instead of the default "rivetr" network.
    let primary_network = config.network.as_deref().unwrap_or(RIVETR_NETWORK);
    ensure_named_network(runtime, primary_network).await;
    let aliases = if !config.network_aliases.is_empty() {
        config.network_aliases.clone()
    } else {
        vec![config.name.clone()]
    };
    connect_to_named_network(runtime, &response_id, primary_network, aliases.clone()).await;

    // Also connect to the per-app isolated network so that containers belonging
    // to the same app (e.g. replicas) can communicate privately while remaining
    // isolated from other apps on the shared bridge.
    if let Some(ref app_id) = config.app_id {
        let per_app_net = format!("rivetr-app-{}", app_id);
        ensure_named_network(runtime, &per_app_net).await;
        connect_to_named_network(runtime, &response_id, &per_app_net, aliases).await;
    }

    Ok(response_id)
}

/// The shared Docker network that all Rivetr-managed containers join.
const RIVETR_NETWORK: &str = "rivetr";

/// Whether a Docker network with this name already exists. The create request no
/// longer carries `CheckDuplicate`, which engines older than Docker 25 need to
/// refuse a second network of the same name, so look first.
async fn network_exists(runtime: &DockerRuntime, name: &str) -> bool {
    runtime.client.inspect_network(name, None).await.is_ok()
}

/// Create the `rivetr` Docker network if it does not already exist.
pub async fn ensure_rivetr_network(runtime: &DockerRuntime) {
    if network_exists(runtime, RIVETR_NETWORK).await {
        return;
    }
    let result = runtime
        .client
        .create_network(NetworkCreateRequest {
            name: RIVETR_NETWORK.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        })
        .await;

    match result {
        Ok(_) => tracing::info!("Created Docker network '{}'", RIVETR_NETWORK),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, ..
        }) => {
            // 409 Conflict = network already exists, which is fine.
        }
        Err(e) => tracing::warn!(
            "Could not create Docker network '{}': {}",
            RIVETR_NETWORK,
            e
        ),
    }
}

/// Connect a container to the shared `rivetr` network with the given aliases.
pub async fn connect_to_rivetr_network(
    runtime: &DockerRuntime,
    container_id: &str,
    aliases: Vec<String>,
) {
    let result = runtime
        .client
        .connect_network(
            RIVETR_NETWORK,
            NetworkConnectRequest {
                container: container_id.to_string(),
                endpoint_config: Some(EndpointSettings {
                    aliases: Some(aliases),
                    ..Default::default()
                }),
            },
        )
        .await;

    match result {
        Ok(_) => {}
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 403,
            message,
        }) if message.contains("already exists") => {
            // Container already attached to the network — idempotent no-op.
        }
        Err(e) => {
            tracing::warn!(
                "Could not connect container {} to network '{}': {}",
                container_id,
                RIVETR_NETWORK,
                e
            );
        }
    }
}

/// Networks whose container aliases Rivetr owns: the shared network and the
/// per-app ones. Anything else (the default `bridge`, a compose network) is left
/// alone.
pub(super) fn is_rivetr_network(name: &str) -> bool {
    name == RIVETR_NETWORK || name.starts_with("rivetr-app-")
}

/// Rebuild the DNS records for every container on Rivetr's networks.
///
/// Entering or leaving swarm mode re-initialises libnetwork, which drops the
/// endpoint records behind Docker's embedded DNS while leaving the containers
/// attached. The next `connect_network` is then a no-op ("endpoint already
/// exists"), so container names silently stop resolving and apps that talk to a
/// database by container name start failing. Detaching and reattaching each
/// endpoint makes Docker register the aliases again.
pub async fn refresh_network_aliases(runtime: &DockerRuntime) {
    let network = match runtime.client.inspect_network(RIVETR_NETWORK, None).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("Could not inspect network '{}': {}", RIVETR_NETWORK, e);
            return;
        }
    };
    let members = network.containers.unwrap_or_default();

    let mut repaired = 0usize;
    for (container_id, member) in members {
        let name = member
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();
        if name.is_empty() {
            continue;
        }

        let networks = match runtime
            .client
            .inspect_container(&container_id, None)
            .await
            .map(|info| {
                info.network_settings
                    .and_then(|s| s.networks)
                    .unwrap_or_default()
            }) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("Could not inspect container {}: {}", name, e);
                continue;
            }
        };

        for (net_name, endpoint) in networks {
            if !is_rivetr_network(&net_name) {
                continue;
            }
            let aliases = endpoint
                .aliases
                .filter(|a| !a.is_empty())
                .unwrap_or_else(|| vec![name.clone()]);
            reconnect_on_network(runtime, &net_name, &container_id, aliases).await;
            repaired += 1;
        }
    }

    tracing::info!(
        endpoints = repaired,
        "Refreshed Docker DNS records on Rivetr networks"
    );
}

/// Detach a container from a network and attach it again with the same aliases.
/// A missing endpoint is not an error: the attach below is what registers it.
async fn reconnect_on_network(
    runtime: &DockerRuntime,
    net_name: &str,
    container_id: &str,
    aliases: Vec<String>,
) {
    match runtime
        .client
        .disconnect_network(
            net_name,
            NetworkDisconnectRequest {
                container: container_id.to_string(),
                force: Some(false),
            },
        )
        .await
    {
        Ok(()) => {}
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 403, ..
        }) => {
            // Not attached, nothing to undo.
        }
        Err(e) => tracing::warn!(
            "Could not detach container {} from network '{}': {}",
            container_id,
            net_name,
            e
        ),
    }

    match runtime
        .client
        .connect_network(
            net_name,
            NetworkConnectRequest {
                container: container_id.to_string(),
                endpoint_config: Some(EndpointSettings {
                    aliases: Some(aliases),
                    ..Default::default()
                }),
            },
        )
        .await
    {
        Ok(()) => {}
        Err(e) => tracing::warn!(
            "Could not attach container {} to network '{}': {}",
            container_id,
            net_name,
            e
        ),
    }
}

/// Create a Docker bridge network with the given name if it does not already exist.
pub async fn ensure_named_network(runtime: &DockerRuntime, net_name: &str) {
    if network_exists(runtime, net_name).await {
        return;
    }
    let result = runtime
        .client
        .create_network(NetworkCreateRequest {
            name: net_name.to_string(),
            driver: Some("bridge".to_string()),
            ..Default::default()
        })
        .await;

    match result {
        Ok(_) => tracing::info!("Created Docker network '{}'", net_name),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409, ..
        }) => {
            // Network already exists – nothing to do.
        }
        Err(e) => tracing::warn!("Could not create Docker network '{}': {}", net_name, e),
    }
}

/// Connect a container to the named Docker network with the given aliases.
pub async fn connect_to_named_network(
    runtime: &DockerRuntime,
    container_id: &str,
    net_name: &str,
    aliases: Vec<String>,
) {
    let result = runtime
        .client
        .connect_network(
            net_name,
            NetworkConnectRequest {
                container: container_id.to_string(),
                endpoint_config: Some(EndpointSettings {
                    aliases: Some(aliases),
                    ..Default::default()
                }),
            },
        )
        .await;

    match result {
        Ok(_) => {}
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 403,
            message,
        }) if message.contains("already exists") => {
            // Container already attached to the network — idempotent no-op.
        }
        Err(e) => {
            tracing::warn!(
                "Could not connect container {} to network '{}': {}",
                container_id,
                net_name,
                e
            );
        }
    }
}

/// Rename a container to a new name.
pub async fn rename_container(
    runtime: &DockerRuntime,
    container_id: &str,
    new_name: &str,
) -> Result<()> {
    runtime
        .client
        .rename_container(
            container_id,
            RenameContainerOptionsBuilder::new().name(new_name).build(),
        )
        .await
        .context("Failed to rename container")?;
    Ok(())
}

/// Read `path` from a container as a tar archive, failing past `max_bytes`.
pub async fn copy_from_container(
    runtime: &DockerRuntime,
    container_id: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    let options = bollard::query_parameters::DownloadFromContainerOptionsBuilder::new()
        .path(path)
        .build();
    let mut stream = runtime
        .client
        .download_from_container(container_id, Some(options));
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read file from container")?;
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > max_bytes {
            anyhow::bail!("File is larger than the {} byte limit", max_bytes);
        }
    }
    Ok(buf)
}

/// Extract a tar archive into `dest_dir` inside a container.
pub async fn copy_to_container(
    runtime: &DockerRuntime,
    container_id: &str,
    dest_dir: &str,
    tar: Vec<u8>,
) -> Result<()> {
    let options = bollard::query_parameters::UploadToContainerOptionsBuilder::new()
        .path(dest_dir)
        .build();
    runtime
        .client
        .upload_to_container(
            container_id,
            Some(options),
            bollard::body_full(Bytes::from(tar)),
        )
        .await
        .context("Failed to write file into container")?;
    Ok(())
}

pub async fn start(runtime: &DockerRuntime, container_id: &str) -> Result<()> {
    runtime
        .client
        .start_container(container_id, None)
        .await
        .context("Failed to start container")?;
    Ok(())
}

pub async fn stop(runtime: &DockerRuntime, container_id: &str) -> Result<()> {
    stop_timeout(runtime, container_id, None).await
}

/// Stop a container, waiting up to `timeout_secs` for graceful shutdown before
/// SIGKILL. Defaults to Docker's conventional 10s when `None`.
pub async fn stop_timeout(
    runtime: &DockerRuntime,
    container_id: &str,
    timeout_secs: Option<i64>,
) -> Result<()> {
    let options = StopContainerOptionsBuilder::new()
        .t(timeout_secs
            .unwrap_or(10)
            .clamp(i32::MIN as i64, i32::MAX as i64) as i32)
        .build();
    runtime
        .client
        .stop_container(container_id, Some(options))
        .await
        .context("Failed to stop container")?;
    Ok(())
}

pub async fn remove(runtime: &DockerRuntime, container_id: &str) -> Result<()> {
    let options = RemoveContainerOptionsBuilder::new().force(true).build();
    runtime
        .client
        .remove_container(container_id, Some(options))
        .await
        .context("Failed to remove container")?;
    Ok(())
}

/// Map a runtime-neutral healthcheck to Docker's `HealthConfig` (durations in ns).
/// Docker reports a taken host port differently at create and start time.
fn is_port_conflict(err: &bollard::errors::Error) -> bool {
    let text = err.to_string().to_ascii_lowercase();
    text.contains("port is already allocated") || text.contains("address already in use")
}

/// Re-pick the host ports Rivetr allocated for `keys`.
fn repin_ports(container_config: &mut bollard::models::ContainerCreateBody, keys: &[String]) {
    let Some(host_config) = container_config.host_config.as_mut() else {
        return;
    };
    let Some(bindings) = host_config.port_bindings.as_mut() else {
        return;
    };
    for key in keys {
        let Some(port) = allocate_host_port() else {
            continue;
        };
        bindings.insert(
            key.clone(),
            Some(vec![bollard::service::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(port.to_string()),
            }]),
        );
    }
}

fn docker_health_config(
    hc: &crate::runtime::ContainerHealthcheck,
) -> bollard::models::HealthConfig {
    const NS: i64 = 1_000_000_000;
    bollard::models::HealthConfig {
        test: Some(hc.test.clone()),
        interval: Some(i64::from(hc.interval_secs) * NS),
        timeout: Some(i64::from(hc.timeout_secs) * NS),
        retries: Some(i64::from(hc.retries)),
        start_period: Some(i64::from(hc.start_period_secs) * NS),
        ..Default::default()
    }
}

/// Pick a host port to publish a container port on.
///
/// Binding to port 0 lets the kernel pick a free port; the listener is dropped
/// immediately so Docker can bind it. Returns `None` if no port could be found,
/// in which case the caller falls back to Docker's dynamic allocation.
fn allocate_host_port() -> Option<u16> {
    std::net::TcpListener::bind(("0.0.0.0", 0))
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .ok()
}

/// Published TCP ports as `(container_port, host_port)` pairs.
fn published_port_map(
    ports: &HashMap<String, Option<Vec<bollard::service::PortBinding>>>,
) -> Vec<(u16, u16)> {
    let mut out: Vec<(u16, u16)> = ports
        .iter()
        .filter_map(|(key, bindings)| {
            let (port, proto) = key.split_once('/').unwrap_or((key.as_str(), "tcp"));
            if proto != "tcp" {
                return None;
            }
            let container_port: u16 = port.parse().ok()?;
            let host_port = bindings.as_ref()?.iter().find_map(|b| {
                b.host_port
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .and_then(|p| p.parse::<u16>().ok())
            })?;
            Some((container_port, host_port))
        })
        .collect();
    out.sort_unstable();
    out
}

/// Merge the ports pinned in `HostConfig` with the ports Docker reports in
/// `NetworkSettings`.
///
/// `HostConfig` is filled in when the container is created, so it is the only
/// source that is correct while a container is still starting. `NetworkSettings`
/// reflects what is actually bound once the container is up and wins on conflict.
fn merge_port_maps(
    pinned: &HashMap<String, Option<Vec<bollard::service::PortBinding>>>,
    actual: &HashMap<String, Option<Vec<bollard::service::PortBinding>>>,
) -> Vec<(u16, u16)> {
    let mut merged: HashMap<u16, u16> = published_port_map(pinned).into_iter().collect();
    merged.extend(published_port_map(actual));
    let mut out: Vec<(u16, u16)> = merged.into_iter().collect();
    out.sort_unstable();
    out
}

/// The host port a container is published on.
///
/// `NetworkSettings.Ports` lists every port the image EXPOSEs, including ones that are not
/// published (their binding list is null). Taking the first map entry was order-dependent:
/// when it landed on an unpublished port the result was `None`, and a successful deploy
/// silently skipped the proxy route swap, leaving the proxy on the old, removed container.
/// Look through all entries, in a stable order, for the first one actually bound to a host port.
fn published_host_port(
    ports: &HashMap<String, Option<Vec<bollard::service::PortBinding>>>,
) -> Option<u16> {
    let mut keys: Vec<&String> = ports.keys().collect();
    keys.sort();
    keys.into_iter().find_map(|key| {
        ports.get(key)?.as_ref()?.iter().find_map(|binding| {
            binding
                .host_port
                .as_deref()
                .filter(|p| !p.is_empty())
                .and_then(|p| p.parse().ok())
        })
    })
}

pub async fn inspect(runtime: &DockerRuntime, container_id: &str) -> Result<ContainerInfo> {
    let info = runtime
        .client
        .inspect_container(container_id, None)
        .await
        .context("Failed to inspect container")?;

    let ports = info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref());
    // Ports pinned at creation. They survive daemon restarts and are present even
    // before the container has started, unlike NetworkSettings.
    let pinned_ports = info
        .host_config
        .as_ref()
        .and_then(|hc| hc.port_bindings.as_ref());
    let empty = HashMap::new();
    let port_map = merge_port_maps(pinned_ports.unwrap_or(&empty), ports.unwrap_or(&empty));
    // Containers created by this version record their primary port in a label; prefer
    // its binding so extra published ports never take over the route.
    let primary_port: Option<u16> = info
        .config
        .as_ref()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get(crate::runtime::PRIMARY_PORT_LABEL))
        .and_then(|p| p.parse().ok());
    let host_port = primary_port
        .and_then(|p| crate::runtime::primary_host_port(&port_map, Some(p)))
        .or_else(|| ports.and_then(published_host_port))
        .or_else(|| pinned_ports.and_then(published_host_port));

    let (running, status) = info
        .state
        .as_ref()
        .map(|s| {
            let is_running = s.running.unwrap_or(false);
            let status_str = s
                .status
                .as_ref()
                .map(|st| format!("{:?}", st))
                .unwrap_or_default();
            (is_running, status_str)
        })
        .unwrap_or((false, String::new()));

    let restart_count = info.restart_count.unwrap_or(0).max(0) as u32;

    Ok(ContainerInfo {
        id: info.id.unwrap_or_default(),
        name: info.name.unwrap_or_default(),
        status,
        port: host_port,
        running,
        host_port,
        restart_count,
        port_map,
    })
}

pub async fn list_containers(
    runtime: &DockerRuntime,
    name_prefix: &str,
) -> Result<Vec<ContainerInfo>> {
    let mut filters = HashMap::new();
    filters.insert("status".to_string(), vec!["running".to_string()]);
    filters.insert("name".to_string(), vec![name_prefix.to_string()]);

    let options = ListContainersOptionsBuilder::new()
        .all(false)
        .filters(&filters)
        .build();

    let containers = runtime
        .client
        .list_containers(Some(options))
        .await
        .context("Failed to list containers")?;

    let mut result = Vec::new();
    for container in containers {
        let name = container
            .names
            .and_then(|names| names.first().cloned())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        // Get port mapping
        let port = container.ports.and_then(|ports| {
            ports
                .iter()
                .find(|p| p.public_port.is_some())
                .and_then(|p| p.public_port)
        });

        // Check if running based on status
        let is_running = container.state == Some(ContainerSummaryStateEnum::RUNNING);
        let status = container
            .state
            .map(|state| state.to_string())
            .unwrap_or_default();

        result.push(ContainerInfo {
            id: container.id.unwrap_or_default(),
            name,
            status,
            port,
            running: is_running,
            host_port: port,
            restart_count: 0,
            port_map: Vec::new(),
        });
    }

    Ok(result)
}

pub async fn list_compose_containers(
    runtime: &DockerRuntime,
    project_name: &str,
) -> Result<Vec<ContainerInfo>> {
    let mut filters = HashMap::new();
    filters.insert("status".to_string(), vec!["running".to_string()]);
    // Filter by Docker Compose project label
    filters.insert(
        "label".to_string(),
        vec![format!("com.docker.compose.project={}", project_name)],
    );

    let options = ListContainersOptionsBuilder::new()
        .all(false)
        .filters(&filters)
        .build();

    let containers = runtime
        .client
        .list_containers(Some(options))
        .await
        .context("Failed to list compose containers")?;

    let mut result = Vec::new();
    for container in containers {
        let name = container
            .names
            .and_then(|names| names.first().cloned())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        let port = container.ports.and_then(|ports| {
            ports
                .iter()
                .find(|p| p.public_port.is_some())
                .and_then(|p| p.public_port)
        });

        let is_running = container.state == Some(ContainerSummaryStateEnum::RUNNING);
        let status = container
            .state
            .map(|state| state.to_string())
            .unwrap_or_default();

        result.push(ContainerInfo {
            id: container.id.unwrap_or_default(),
            name,
            status,
            port,
            running: is_running,
            host_port: port,
            restart_count: 0,
            port_map: Vec::new(),
        });
    }

    Ok(result)
}

pub async fn stats(runtime: &DockerRuntime, container_id: &str) -> Result<ContainerStats> {
    // Use stream mode and take two samples to calculate CPU delta properly.
    // The one_shot mode doesn't provide valid precpu_stats for delta calculation.
    let options = StatsOptionsBuilder::new()
        .stream(true)
        .one_shot(false)
        .build();

    let mut stream = runtime.client.stats(container_id, Some(options));

    // Get first sample
    let first_stats = stream
        .next()
        .await
        .context("No stats received for container")?
        .context("Failed to get first stats sample")?;

    // Wait a short interval for meaningful CPU delta
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Get second sample
    let second_stats = stream
        .next()
        .await
        .context("No second stats sample received")?
        .context("Failed to get second stats sample")?;

    // Calculate CPU percentage using delta between two samples
    // CPU percentage = (container_delta / system_delta) * num_cpus * 100
    // Every field in the stats payload is optional in the current Docker client.
    let total_usage = |s: &ContainerStatsResponse| -> u64 {
        s.cpu_stats
            .as_ref()
            .and_then(|c| c.cpu_usage.as_ref())
            .and_then(|u| u.total_usage)
            .unwrap_or(0)
    };
    let system_usage = |s: &ContainerStatsResponse| -> u64 {
        s.cpu_stats
            .as_ref()
            .and_then(|c| c.system_cpu_usage)
            .unwrap_or(0)
    };

    let cpu_delta = total_usage(&second_stats) as f64 - total_usage(&first_stats) as f64;
    let system_delta = system_usage(&second_stats) as f64 - system_usage(&first_stats) as f64;

    let num_cpus = second_stats
        .cpu_stats
        .as_ref()
        .and_then(|c| {
            c.online_cpus.map(u64::from).or_else(|| {
                c.cpu_usage
                    .as_ref()
                    .and_then(|u| u.percpu_usage.as_ref())
                    .map(|v| v.len() as u64)
            })
        })
        .unwrap_or(1) as f64;

    let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 {
        (cpu_delta / system_delta) * num_cpus * 100.0
    } else {
        0.0
    };

    // Get memory stats from the latest sample
    let memory_usage = second_stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.usage)
        .unwrap_or(0);
    let memory_limit = second_stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.limit)
        .unwrap_or(0);

    // Get network stats from the latest sample
    let (network_rx, network_tx) = if let Some(networks) = &second_stats.networks {
        let mut rx: u64 = 0;
        let mut tx: u64 = 0;
        for net_stats in networks.values() {
            rx += net_stats.rx_bytes.unwrap_or(0);
            tx += net_stats.tx_bytes.unwrap_or(0);
        }
        (rx, tx)
    } else {
        (0, 0)
    };

    Ok(ContainerStats {
        cpu_percent,
        memory_usage,
        memory_limit,
        network_rx,
        network_tx,
        storage_bytes: None,
    })
}

/// The container's writable layer (from the daemon) plus the size of each mount,
/// measured with `du -sk` inside the container so volumes owned by root are
/// readable even though Rivetr itself does not run as root.
pub async fn disk_usage(runtime: &DockerRuntime, container_id: &str) -> Result<DiskUsage> {
    let options = InspectContainerOptionsBuilder::new().size(true).build();
    let info = runtime
        .client
        .inspect_container(container_id, Some(options))
        .await
        .context("Failed to inspect container size")?;

    let writable_layer = info.size_rw.unwrap_or(0).max(0) as u64;

    let mounts: Vec<(String, String)> = info
        .mounts
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let destination = m.destination?;
            let source = m.name.or(m.source).unwrap_or_else(|| destination.clone());
            Some((source, destination))
        })
        // The root and runtime sockets are host plumbing, not the container's data.
        .filter(|(_, destination)| destination != "/" && !destination.ends_with(".sock"))
        .collect();

    let mut measured = Vec::with_capacity(mounts.len());
    if !mounts.is_empty() {
        let mut cmd = vec!["du".to_string(), "-sk".to_string()];
        cmd.extend(mounts.iter().map(|(_, destination)| destination.clone()));
        // A non-zero exit only means some entry was unreadable; the lines that
        // were printed are still correct. A container without `du` gives none.
        if let Ok(result) = run_command(runtime, container_id, cmd).await {
            let sizes = parse_du_kib(&result.stdout);
            for (source, destination) in mounts {
                if let Some(kib) = sizes.get(destination.as_str()) {
                    measured.push((source, kib * 1024));
                }
            }
        }
    }

    Ok(DiskUsage {
        writable_layer,
        mounts: measured,
    })
}

/// Parse `du -sk` output (`<KiB>\t<path>` per line) into path -> KiB.
fn parse_du_kib(stdout: &str) -> HashMap<&str, u64> {
    stdout
        .lines()
        .filter_map(|line| {
            let (size, path) = line.split_once(char::is_whitespace)?;
            Some((path.trim(), size.trim().parse::<u64>().ok()?))
        })
        .collect()
}

/// Remove a named volume without force, so the daemon refuses one that a
/// container (running or stopped) still uses. A missing volume is not an error.
pub async fn remove_volume(runtime: &DockerRuntime, name: &str) -> Result<()> {
    match runtime
        .client
        .remove_volume(name, None::<bollard::query_parameters::RemoveVolumeOptions>)
        .await
    {
        Ok(()) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(()),
        Err(e) => Err(e).context("Failed to remove volume"),
    }
}

pub async fn remove_image(runtime: &DockerRuntime, image: &str) -> Result<()> {
    let options = RemoveImageOptionsBuilder::new()
        .force(true)
        .noprune(false)
        .build();

    runtime
        .client
        .remove_image(image, Some(options), None)
        .await
        .context("Failed to remove image")?;

    Ok(())
}

pub async fn prune_build_cache(_runtime: &DockerRuntime) -> Result<u64> {
    let output = tokio::process::Command::new("docker")
        .args(["builder", "prune", "-f"])
        .output()
        .await
        .context("Failed to run docker builder prune")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("docker builder prune failed: {}", stderr);
        return Ok(0);
    }

    // Parse "Total: X.XXgB" from stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::debug!("docker builder prune output: {}", stdout.trim());
    Ok(0) // Docker CLI doesn't give us a machine-readable byte count easily
}

pub async fn prune_images(runtime: &DockerRuntime) -> Result<u64> {
    prune_images_inner(runtime, false).await
}

/// Prune ALL unused images (equivalent to `docker image prune -a`): every image
/// not referenced by a container, not just dangling ones. Reclaims orphaned
/// tagged per-deployment images that a dangling-only prune leaves behind (the
/// main driver of disk-fill on long-lived hosts). Only used under disk
/// pressure — base images will re-pull on the next build.
pub async fn prune_all_images(runtime: &DockerRuntime) -> Result<u64> {
    prune_images_inner(runtime, true).await
}

async fn prune_images_inner(runtime: &DockerRuntime, all: bool) -> Result<u64> {
    let mut filters: HashMap<String, Vec<String>> = HashMap::new();
    if all {
        // dangling=false ⇒ remove every unused image, not only dangling ones.
        filters.insert("dangling".to_string(), vec!["false".to_string()]);
    }
    let options = PruneImagesOptionsBuilder::new().filters(&filters).build();

    let result = runtime
        .client
        .prune_images(Some(options))
        .await
        .context("Failed to prune images")?;

    let space_reclaimed = result.space_reclaimed.unwrap_or(0) as u64;

    if let Some(images) = result.images_deleted {
        tracing::debug!("Pruned {} images", images.len());
    }

    Ok(space_reclaimed)
}

pub async fn exec(runtime: &DockerRuntime, config: &ExecConfig) -> Result<ExecHandle> {
    // Create exec instance
    let exec_options = CreateExecOptions {
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(config.tty),
        cmd: Some(config.cmd.clone()),
        ..Default::default()
    };

    let exec_instance = runtime
        .client
        .create_exec(&config.container_id, exec_options)
        .await
        .context("Failed to create exec instance")?;

    let exec_id = exec_instance.id.clone();

    // Start exec and get streams
    let start_result = runtime
        .client
        .start_exec(&exec_id, None)
        .await
        .context("Failed to start exec")?;

    // Create channels for bidirectional communication
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<Bytes>(32);
    let (stdout_tx, stdout_rx) = mpsc::channel::<Bytes>(32);
    let (resize_tx, mut resize_rx) = mpsc::channel::<TtySize>(8);

    // Clone client for resize task
    let resize_client = runtime.client.clone();
    let resize_exec_id = exec_id.clone();

    // Spawn resize handler
    tokio::spawn(async move {
        while let Some(size) = resize_rx.recv().await {
            let options = ResizeExecOptionsBuilder::new()
                .h(size.rows as i32)
                .w(size.cols as i32)
                .build();
            if let Err(e) = resize_client.resize_exec(&resize_exec_id, options).await {
                tracing::warn!("Failed to resize exec: {}", e);
            }
        }
    });

    match start_result {
        StartExecResults::Attached {
            mut output,
            mut input,
        } => {
            // Spawn stdin writer task
            tokio::spawn(async move {
                while let Some(data) = stdin_rx.recv().await {
                    if input.write_all(&data).await.is_err() {
                        break;
                    }
                }
            });

            // Spawn stdout reader task
            tokio::spawn(async move {
                while let Some(result) = output.next().await {
                    match result {
                        Ok(output) => {
                            let data = match output {
                                LogOutput::StdOut { message } => message,
                                LogOutput::StdErr { message } => message,
                                LogOutput::StdIn { message } => message,
                                LogOutput::Console { message } => message,
                            };
                            if stdout_tx.send(Bytes::from(data.to_vec())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Exec output error: {}", e);
                            break;
                        }
                    }
                }
            });

            Ok(ExecHandle {
                stdin_tx,
                stdout_rx,
                resize_tx,
            })
        }
        StartExecResults::Detached => {
            anyhow::bail!("Exec started in detached mode, but attached mode was expected")
        }
    }
}

pub async fn run_command(
    runtime: &DockerRuntime,
    container_id: &str,
    cmd: Vec<String>,
) -> Result<CommandResult> {
    // Create exec instance
    let exec_options = CreateExecOptions {
        attach_stdin: Some(false),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        cmd: Some(cmd),
        ..Default::default()
    };

    let exec_instance = runtime
        .client
        .create_exec(container_id, exec_options)
        .await
        .context("Failed to create exec instance")?;

    let exec_id = exec_instance.id.clone();

    // Start exec and collect output
    let start_result = runtime
        .client
        .start_exec(&exec_id, None)
        .await
        .context("Failed to start exec")?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    match start_result {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(result) = output.next().await {
                match result {
                    Ok(LogOutput::StdOut { message }) => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    Ok(LogOutput::StdErr { message }) => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("Exec output error: {}", e);
                        break;
                    }
                }
            }
        }
        StartExecResults::Detached => {
            anyhow::bail!("Exec started in detached mode")
        }
    }

    // Get exit code
    let inspect = runtime.client.inspect_exec(&exec_id).await?;
    let exit_code = inspect.exit_code.unwrap_or(-1) as i32;

    Ok(CommandResult {
        exit_code,
        stdout,
        stderr,
    })
}

pub async fn pull_image(
    runtime: &DockerRuntime,
    image: &str,
    auth: Option<&RegistryAuth>,
) -> Result<()> {
    use bollard::auth::DockerCredentials;

    tracing::info!(image = %image, "Pulling image from registry");

    // Parse image reference to extract image name and tag
    // Format: [registry/]name[:tag]
    let (from_image, tag) = if image.contains('@') {
        // Digest format: image@sha256:... - use as-is
        (image.to_string(), None)
    } else if let Some((name, tag_part)) = image.rsplit_once(':') {
        // Check if the colon is for a registry port (registry:5000/image) rather than a tag
        // If there's a / after the colon, it's a port number, not a tag
        if tag_part.contains('/') {
            // This is registry:port/image format, no tag specified
            (image.to_string(), Some("latest".to_string()))
        } else {
            // Normal image:tag format
            (name.to_string(), Some(tag_part.to_string()))
        }
    } else {
        // No tag specified, use latest
        (image.to_string(), Some("latest".to_string()))
    };

    let options = CreateImageOptionsBuilder::new()
        .from_image(&from_image)
        .tag(tag.as_deref().unwrap_or("latest"))
        .build();

    tracing::debug!(from_image = %from_image, tag = ?tag, "Parsed image reference");

    // Set up authentication if provided
    let credentials = auth.and_then(|a| {
        if a.is_empty() {
            None
        } else {
            Some(DockerCredentials {
                username: a.username.clone(),
                password: a.password.clone(),
                serveraddress: a.server.clone(),
                ..Default::default()
            })
        }
    });

    let mut stream = runtime
        .client
        .create_image(Some(options), None, credentials);

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                // Log progress
                if let Some(status) = info.status {
                    match info.progress_detail {
                        Some(p) if p.total.is_some() => tracing::debug!(
                            "{}: {}/{}",
                            status,
                            p.current.unwrap_or(0),
                            p.total.unwrap_or(0)
                        ),
                        _ => tracing::debug!("{}", status),
                    }
                }
                if let Some(detail) = info.error_detail {
                    anyhow::bail!(
                        "Failed to pull image: {}",
                        detail
                            .message
                            .unwrap_or_else(|| "unknown pull error".to_string())
                    );
                }
            }
            Err(e) => anyhow::bail!("Failed to pull image: {}", e),
        }
    }

    tracing::info!(image = %image, "Successfully pulled image");
    Ok(())
}

/// Update CPU and memory limits on a running container without restarting it.
/// Uses the Docker `update_container` API which applies cgroup-level changes live.
pub async fn update_container_limits(
    runtime: &DockerRuntime,
    container_id: &str,
    memory_limit: Option<&str>,
    cpu_limit: Option<&str>,
) -> Result<()> {
    let memory = memory_limit.and_then(parse_memory);
    let nano_cpus = cpu_limit.and_then(parse_cpu);

    let options = ContainerUpdateBody {
        memory,
        // Docker rejects a memory update that would leave a larger pre-existing
        // memory-swap limit ("update the memoryswap at the same time"), which
        // happens whenever a container started without a memory cap (unlimited
        // swap). Set memory_swap == memory so the cap is hard (no extra swap)
        // and the update is accepted. Only set it when capping memory.
        memory_swap: memory,
        nano_cpus,
        ..Default::default()
    };

    runtime
        .client
        .update_container(container_id, options)
        .await
        .context("Failed to update container resource limits")?;

    tracing::info!(
        container_id = %container_id,
        memory = ?memory,
        nano_cpus = ?nano_cpus,
        "Updated container resource limits"
    );

    Ok(())
}

#[cfg(test)]
mod published_port_tests {
    use super::published_host_port;
    use bollard::service::PortBinding;
    use std::collections::HashMap;

    fn bound(port: &str) -> Option<Vec<PortBinding>> {
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".into()),
            host_port: Some(port.into()),
        }])
    }

    #[test]
    fn skips_exposed_but_unpublished_ports() {
        // FrankenPHP images expose 80, 443 and 443/udp; only the app port is published.
        let mut ports = HashMap::new();
        ports.insert("80/tcp".to_string(), None);
        ports.insert("443/tcp".to_string(), None);
        ports.insert("443/udp".to_string(), None);
        ports.insert("8000/tcp".to_string(), bound("33341"));
        ports.insert("8080/tcp".to_string(), Some(vec![]));

        // HashMap order varies run to run; the answer must not.
        for _ in 0..50 {
            assert_eq!(published_host_port(&ports.clone()), Some(33341));
        }
    }

    #[test]
    fn none_when_nothing_is_published() {
        let mut ports = HashMap::new();
        ports.insert("80/tcp".to_string(), None);
        ports.insert(
            "3000/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: None,
                host_port: Some(String::new()),
            }]),
        );
        assert_eq!(published_host_port(&ports), None);
    }

    #[test]
    fn detects_port_conflicts_from_docker_errors() {
        let taken = bollard::errors::Error::DockerResponseServerError {
            status_code: 500,
            message: "driver failed programming external connectivity: Bind for 0.0.0.0:32769 failed: port is already allocated".to_string(),
        };
        assert!(super::is_port_conflict(&taken));
        let other = bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            message: "No such image".to_string(),
        };
        assert!(!super::is_port_conflict(&other));
    }

    #[test]
    fn only_rivetr_networks_are_repaired() {
        assert!(super::is_rivetr_network("rivetr"));
        assert!(super::is_rivetr_network(
            "rivetr-app-f20be694-5f62-4cf0-866b-e658ea0f2f15"
        ));
        // Foreign networks must be left alone: detaching from them could drop
        // connectivity Rivetr does not own.
        assert!(!super::is_rivetr_network("bridge"));
        assert!(!super::is_rivetr_network("host"));
        assert!(!super::is_rivetr_network("coolify"));
        assert!(!super::is_rivetr_network("rivetr-tools"));
    }

    #[test]
    fn merge_keeps_pinned_ports_before_the_container_starts() {
        // NetworkSettings is empty while the container is still starting; the
        // ports pinned in HostConfig are the only correct answer then.
        let mut pinned = HashMap::new();
        pinned.insert("8080/tcp".to_string(), bound("32769"));
        let empty = HashMap::new();
        assert_eq!(super::merge_port_maps(&pinned, &empty), vec![(8080, 32769)]);
    }

    #[test]
    fn merge_prefers_the_port_docker_actually_bound() {
        let mut pinned = HashMap::new();
        pinned.insert("8080/tcp".to_string(), bound("32769"));
        let mut actual = HashMap::new();
        actual.insert("8080/tcp".to_string(), bound("32771"));
        assert_eq!(
            super::merge_port_maps(&pinned, &actual),
            vec![(8080, 32771)]
        );
    }

    #[test]
    fn merge_keeps_per_domain_ports_from_both_sources() {
        let mut pinned = HashMap::new();
        pinned.insert("8080/tcp".to_string(), bound("32769"));
        pinned.insert("9000/tcp".to_string(), bound("32770"));
        let mut actual = HashMap::new();
        actual.insert("8080/tcp".to_string(), bound("32771"));
        assert_eq!(
            super::merge_port_maps(&pinned, &actual),
            vec![(8080, 32771), (9000, 32770)]
        );
    }
}

#[cfg(test)]
mod disk_usage_tests {
    use super::parse_du_kib;

    #[test]
    fn parses_du_output_per_path() {
        let sizes = parse_du_kib("120\t/app/data\n4\t/app/uploads\ngarbage\n");
        assert_eq!(sizes.get("/app/data"), Some(&120));
        assert_eq!(sizes.get("/app/uploads"), Some(&4));
        assert_eq!(sizes.len(), 2);
    }

    #[test]
    fn keeps_paths_with_spaces() {
        let sizes = parse_du_kib("8\t/srv/my files\n");
        assert_eq!(sizes.get("/srv/my files"), Some(&8));
    }
}
