use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StatsOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use bollard::auth::DockerCredentials;
use bollard::image::{CreateImageOptions, PruneImagesOptions, RemoveImageOptions};
use bollard::Docker;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::{BuildContext, CommandResult, ContainerInfo, ContainerRuntime, ContainerStats, ExecConfig, ExecHandle, LogLine, LogStream, RegistryAuth, RunConfig, TtySize};

pub struct DockerRuntime {
    client: Docker,
}

impl DockerRuntime {
    pub fn new(socket: &str) -> Result<Self> {
        // On Windows, always use local defaults (named pipe)
        // On Unix, use socket path if specified
        let client = if cfg!(windows) || socket.starts_with("npipe://") || socket.starts_with("tcp://") {
            Docker::connect_with_local_defaults()?
        } else {
            Docker::connect_with_socket(socket, 120, bollard::API_DEFAULT_VERSION)?
        };

        Ok(Self { client })
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn name(&self) -> &'static str {
        "Docker"
    }

    async fn build(&self, ctx: &BuildContext) -> Result<String> {
        use bollard::image::BuildImageOptions;
        use bytes::Bytes;

        // Create a tar archive of the build context
        let tar_path = format!("{}.tar", ctx.path);
        let tar_file = std::fs::File::create(&tar_path)?;
        let mut tar_builder = tar::Builder::new(tar_file);
        tar_builder.append_dir_all(".", &ctx.path)?;
        tar_builder.finish()?;

        let tar_data = std::fs::read(&tar_path)?;
        std::fs::remove_file(&tar_path)?;

        // Parse custom options for build args
        let extra_build_args = parse_custom_build_args(ctx.custom_options.as_deref());

        // Parse build resource limits
        let memory = ctx.memory_limit.as_ref().and_then(|m| parse_build_memory(m));
        let (cpuperiod, cpuquota) = parse_cpu_limits(ctx.cpu_limit.as_deref());

        let target = ctx.build_target.as_deref().unwrap_or("");
        // Bollard's BuildImageOptions expects u64 for memory, cpuperiod, cpuquota
        // but i64 for memswap. We use i64 internally and cast as needed.
        let options = BuildImageOptions {
            dockerfile: ctx.dockerfile.trim_start_matches("./"),
            t: &ctx.tag,
            rm: true,
            target,
            extrahosts: extra_build_args.extra_hosts.as_deref(),
            nocache: extra_build_args.no_cache,
            memory: memory.map(|m| m as u64),
            memswap: memory, // Set memswap equal to memory to disable swap
            cpuperiod: cpuperiod.map(|p| p as u64),
            cpuquota: cpuquota.map(|q| q as u64),
            ..Default::default()
        };

        if memory.is_some() || cpuquota.is_some() {
            tracing::info!(
                memory = ?memory,
                cpuperiod = ?cpuperiod,
                cpuquota = ?cpuquota,
                "Building image with resource limits"
            );
        }

        let mut stream = self.client.build_image(options, None, Some(Bytes::from(tar_data)));

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    if let Some(stream) = output.stream {
                        tracing::debug!("{}", stream.trim());
                    }
                    if let Some(error) = output.error {
                        anyhow::bail!("Build error: {}", error);
                    }
                }
                Err(e) => anyhow::bail!("Build failed: {}", e),
            }
        }

        Ok(ctx.tag.clone())
    }

    async fn run(&self, config: &RunConfig) -> Result<String> {
        let env: Vec<String> = config
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let mut port_bindings: HashMap<String, Option<Vec<bollard::service::PortBinding>>> = HashMap::new();
        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();

        // Add primary port (legacy)
        let primary_port_binding = format!("{}/tcp", config.port);
        port_bindings.insert(
            primary_port_binding.clone(),
            Some(vec![bollard::service::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: None, // Let Docker assign a random port
            }]),
        );
        exposed_ports.insert(primary_port_binding, HashMap::new());

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
            exposed_ports.insert(port_key, HashMap::new());
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

        // Set restart policy to ensure containers restart after server reboot
        let restart_policy = bollard::service::RestartPolicy {
            name: Some(bollard::service::RestartPolicyNameEnum::UNLESS_STOPPED),
            maximum_retry_count: None,
        };

        let host_config = bollard::service::HostConfig {
            port_bindings: Some(port_bindings),
            memory: config.memory_limit.as_ref().and_then(|m| parse_memory(m)),
            nano_cpus: config.cpu_limit.as_ref().and_then(|c| parse_cpu(c)),
            extra_hosts,
            binds,
            restart_policy: Some(restart_policy),
            ..Default::default()
        };

        // Set up network aliases if provided
        // Note: Network aliases require connecting to a custom network
        // For now, we'll add them as environment variables for container discovery
        // Full network alias support requires creating/joining a Docker network
        let mut final_env = env;
        if !config.network_aliases.is_empty() {
            final_env.push(format!("RIVETR_NETWORK_ALIASES={}", config.network_aliases.join(",")));
        }

        // Set up container labels
        let labels: Option<HashMap<String, String>> = if config.labels.is_empty() {
            None
        } else {
            Some(config.labels.clone())
        };

        let container_config = Config {
            image: Some(config.image.clone()),
            env: Some(final_env),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            labels,
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: &config.name,
            platform: None,
        };

        let response = self
            .client
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create container: {}", e))?;

        self.client
            .start_container::<String>(&response.id, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start container: {}", e))?;

        Ok(response.id)
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        self.client
            .start_container::<String>(container_id, None)
            .await
            .context("Failed to start container")?;
        Ok(())
    }

    async fn stop(&self, container_id: &str) -> Result<()> {
        let options = StopContainerOptions { t: 10 };
        self.client
            .stop_container(container_id, Some(options))
            .await
            .context("Failed to stop container")?;
        Ok(())
    }

    async fn remove(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        self.client
            .remove_container(container_id, Some(options))
            .await
            .context("Failed to remove container")?;
        Ok(())
    }

    async fn logs(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false, // Don't follow - just fetch existing logs
            timestamps: true, // Include Docker timestamps
            tail: "1000".to_string(), // Get last 1000 lines
            ..Default::default()
        };

        let stream = self.client.logs(container_id, Some(options));

        let mapped = stream.filter_map(|result| async move {
            match result {
                Ok(output) => {
                    let (stream_type, message) = match output {
                        LogOutput::StdOut { message } => (LogStream::Stdout, message),
                        LogOutput::StdErr { message } => (LogStream::Stderr, message),
                        _ => return None,
                    };
                    let message_str = String::from_utf8_lossy(&message).to_string();
                    // Parse Docker timestamp from the beginning of the message
                    // Format: "2024-01-01T00:00:00.000000000Z message"
                    let (timestamp, msg) = if message_str.len() > 30 && message_str.chars().nth(4) == Some('-') {
                        // Has timestamp prefix
                        let parts: Vec<&str> = message_str.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            (chrono::Utc::now().to_rfc3339(), message_str)
                        }
                    } else {
                        (chrono::Utc::now().to_rfc3339(), message_str)
                    };
                    Some(LogLine {
                        timestamp,
                        message: msg.trim_end().to_string(),
                        stream: stream_type,
                    })
                }
                Err(e) => {
                    tracing::warn!("Error reading container log: {}", e);
                    None
                }
            }
        });

        Ok(Box::pin(mapped))
    }

    async fn logs_stream(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: true, // Follow logs in real-time
            timestamps: true,
            tail: "100".to_string(), // Get last 100 lines then continue streaming
            ..Default::default()
        };

        let stream = self.client.logs(container_id, Some(options));

        let mapped = stream.filter_map(|result| async move {
            match result {
                Ok(output) => {
                    let (stream_type, message) = match output {
                        LogOutput::StdOut { message } => (LogStream::Stdout, message),
                        LogOutput::StdErr { message } => (LogStream::Stderr, message),
                        _ => return None,
                    };
                    let message_str = String::from_utf8_lossy(&message).to_string();
                    // Parse Docker timestamp from the beginning of the message
                    let (timestamp, msg) = if message_str.len() > 30 && message_str.chars().nth(4) == Some('-') {
                        let parts: Vec<&str> = message_str.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            (chrono::Utc::now().to_rfc3339(), message_str)
                        }
                    } else {
                        (chrono::Utc::now().to_rfc3339(), message_str)
                    };
                    Some(LogLine {
                        timestamp,
                        message: msg.trim_end().to_string(),
                        stream: stream_type,
                    })
                }
                Err(e) => {
                    tracing::warn!("Error reading container log: {}", e);
                    None
                }
            }
        });

        Ok(Box::pin(mapped))
    }

    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo> {
        let info = self
            .client
            .inspect_container(container_id, None)
            .await
            .context("Failed to inspect container")?;

        let host_port = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ports.as_ref())
            .and_then(|ports| {
                ports.values().next().and_then(|bindings| {
                    bindings.as_ref().and_then(|b| {
                        b.first()
                            .and_then(|p| p.host_port.as_ref())
                            .and_then(|p| p.parse().ok())
                    })
                })
            });

        let (running, status) = info
            .state
            .as_ref()
            .map(|s| {
                let is_running = s.running.unwrap_or(false);
                let status_str = s.status
                    .as_ref()
                    .map(|st| format!("{:?}", st))
                    .unwrap_or_default();
                (is_running, status_str)
            })
            .unwrap_or((false, String::new()));

        Ok(ContainerInfo {
            id: info.id.unwrap_or_default(),
            name: info.name.unwrap_or_default(),
            status,
            port: host_port,
            running,
            host_port,
        })
    }

    async fn is_available(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    async fn list_containers(&self, name_prefix: &str) -> Result<Vec<ContainerInfo>> {
        let mut filters = HashMap::new();
        filters.insert("status".to_string(), vec!["running".to_string()]);
        filters.insert("name".to_string(), vec![name_prefix.to_string()]);

        let options = ListContainersOptions {
            all: false,
            filters,
            ..Default::default()
        };

        let containers = self
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
                    .and_then(|p| p.public_port.map(|port| port as u16))
            });

            // Check if running based on status
            let status = container.state.clone().unwrap_or_default();
            let is_running = status.to_lowercase() == "running";

            result.push(ContainerInfo {
                id: container.id.unwrap_or_default(),
                name,
                status,
                port,
                running: is_running,
                host_port: port,
            });
        }

        Ok(result)
    }

    async fn list_compose_containers(&self, project_name: &str) -> Result<Vec<ContainerInfo>> {
        let mut filters = HashMap::new();
        filters.insert("status".to_string(), vec!["running".to_string()]);
        // Filter by Docker Compose project label
        filters.insert(
            "label".to_string(),
            vec![format!("com.docker.compose.project={}", project_name)],
        );

        let options = ListContainersOptions {
            all: false,
            filters,
            ..Default::default()
        };

        let containers = self
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
                    .and_then(|p| p.public_port.map(|port| port as u16))
            });

            let status = container.state.clone().unwrap_or_default();
            let is_running = status.to_lowercase() == "running";

            result.push(ContainerInfo {
                id: container.id.unwrap_or_default(),
                name,
                status,
                port,
                running: is_running,
                host_port: port,
            });
        }

        Ok(result)
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats> {
        // Use stream mode and take two samples to calculate CPU delta properly.
        // The one_shot mode doesn't provide valid precpu_stats for delta calculation.
        let options = StatsOptions {
            stream: true,
            one_shot: false,
        };

        let mut stream = self.client.stats(container_id, Some(options));

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
        let cpu_delta = second_stats.cpu_stats.cpu_usage.total_usage as f64
            - first_stats.cpu_stats.cpu_usage.total_usage as f64;

        let system_delta = second_stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
            - first_stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64;

        let num_cpus = second_stats
            .cpu_stats
            .online_cpus
            .or(second_stats
                .cpu_stats
                .cpu_usage
                .percpu_usage
                .as_ref()
                .map(|v: &Vec<u64>| v.len() as u64))
            .unwrap_or(1) as f64;

        let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 {
            (cpu_delta / system_delta) * num_cpus * 100.0
        } else {
            0.0
        };

        // Get memory stats from the latest sample
        let memory_stats = &second_stats.memory_stats;
        let memory_usage = memory_stats.usage.unwrap_or(0);
        let memory_limit = memory_stats.limit.unwrap_or(0);

        // Get network stats from the latest sample
        let (network_rx, network_tx) = if let Some(networks) = &second_stats.networks {
            let mut rx: u64 = 0;
            let mut tx: u64 = 0;
            for (_name, net_stats) in networks {
                rx += net_stats.rx_bytes;
                tx += net_stats.tx_bytes;
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
        })
    }

    async fn remove_image(&self, image: &str) -> Result<()> {
        let options = RemoveImageOptions {
            force: true,
            noprune: false,
        };

        self.client
            .remove_image(image, Some(options), None)
            .await
            .context("Failed to remove image")?;

        Ok(())
    }

    async fn prune_images(&self) -> Result<u64> {
        let options = PruneImagesOptions::<String> {
            filters: HashMap::new(),
        };

        let result = self
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

    async fn exec(&self, config: &ExecConfig) -> Result<ExecHandle> {
        // Create exec instance
        let exec_options = CreateExecOptions {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(config.tty),
            cmd: Some(config.cmd.clone()),
            ..Default::default()
        };

        let exec_instance = self
            .client
            .create_exec(&config.container_id, exec_options)
            .await
            .context("Failed to create exec instance")?;

        let exec_id = exec_instance.id.clone();

        // Start exec and get streams
        let start_result = self
            .client
            .start_exec(&exec_id, None)
            .await
            .context("Failed to start exec")?;

        // Create channels for bidirectional communication
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Bytes>(32);
        let (stdout_tx, stdout_rx) = mpsc::channel::<Bytes>(32);
        let (resize_tx, mut resize_rx) = mpsc::channel::<TtySize>(8);

        // Clone client for resize task
        let resize_client = self.client.clone();
        let resize_exec_id = exec_id.clone();

        // Spawn resize handler
        tokio::spawn(async move {
            while let Some(size) = resize_rx.recv().await {
                let options = ResizeExecOptions {
                    height: size.rows,
                    width: size.cols,
                };
                if let Err(e) = resize_client.resize_exec(&resize_exec_id, options).await {
                    tracing::warn!("Failed to resize exec: {}", e);
                }
            }
        });

        match start_result {
            StartExecResults::Attached { mut output, mut input } => {
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

    async fn run_command(&self, container_id: &str, cmd: Vec<String>) -> Result<CommandResult> {
        // Create exec instance
        let exec_options = CreateExecOptions {
            attach_stdin: Some(false),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            cmd: Some(cmd),
            ..Default::default()
        };

        let exec_instance = self
            .client
            .create_exec(container_id, exec_options)
            .await
            .context("Failed to create exec instance")?;

        let exec_id = exec_instance.id.clone();

        // Start exec and collect output
        let start_result = self
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
        let inspect = self.client.inspect_exec(&exec_id).await?;
        let exit_code = inspect.exit_code.unwrap_or(-1) as i32;

        Ok(CommandResult {
            exit_code,
            stdout,
            stderr,
        })
    }

    async fn pull_image(&self, image: &str, auth: Option<&RegistryAuth>) -> Result<()> {
        tracing::info!(image = %image, "Pulling image from registry");

        // Parse image reference to extract image name and tag
        // Format: [registry/]name[:tag]
        // Examples:
        //   postgres:16 -> name=postgres, tag=16
        //   nginx -> name=nginx, tag=latest
        //   ghcr.io/user/image:v1 -> name=ghcr.io/user/image, tag=v1
        //   registry:5000/image:tag -> name=registry:5000/image, tag=tag
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

        let options = CreateImageOptions {
            from_image: from_image.clone(),
            tag: tag.clone().unwrap_or_else(|| "latest".to_string()),
            ..Default::default()
        };

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

        let mut stream = self.client.create_image(Some(options), None, credentials);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    // Log progress
                    if let Some(status) = info.status {
                        if let Some(progress) = info.progress {
                            tracing::debug!("{}: {}", status, progress);
                        } else {
                            tracing::debug!("{}", status);
                        }
                    }
                    if let Some(error) = info.error {
                        anyhow::bail!("Failed to pull image: {}", error);
                    }
                }
                Err(e) => anyhow::bail!("Failed to pull image: {}", e),
            }
        }

        tracing::info!(image = %image, "Successfully pulled image");
        Ok(())
    }
}

fn parse_memory(s: &str) -> Option<i64> {
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

/// Parse memory limit for Docker build, returns i64 as required by Bollard BuildImageOptions.
fn parse_build_memory(s: &str) -> Option<i64> {
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

/// Parse CPU limits for Docker build.
/// Docker build uses cpu-period (default 100000) and cpu-quota to limit CPU.
/// If cpu-quota = cpu-period * num_cpus, then the container can use num_cpus worth of CPU.
/// For example, with cpu-period=100000 and cpu-quota=200000, the build can use 2 CPUs.
fn parse_cpu_limits(cpu_limit: Option<&str>) -> (Option<i64>, Option<i64>) {
    let Some(cpu_str) = cpu_limit else {
        return (None, None);
    };

    // Parse the CPU value (e.g., "2" for 2 CPUs, "0.5" for half a CPU)
    let cpu_count = cpu_str.parse::<f64>().ok();

    match cpu_count {
        Some(cpus) if cpus > 0.0 => {
            let period: i64 = 100_000; // Default Docker CPU period (100ms)
            let quota = (cpus * period as f64) as i64;
            (Some(period), Some(quota))
        }
        _ => (None, None),
    }
}

/// Parsed custom build arguments from custom_docker_options string
#[derive(Default)]
struct CustomBuildArgs {
    no_cache: bool,
    extra_hosts: Option<String>,
}

/// Parse custom docker options string into build arguments
/// Supports: --no-cache, --add-host
fn parse_custom_build_args(options: Option<&str>) -> CustomBuildArgs {
    let mut args = CustomBuildArgs::default();

    let Some(opts) = options else {
        return args;
    };

    // Parse --no-cache
    if opts.contains("--no-cache") {
        args.no_cache = true;
    }

    // Parse --add-host (format: --add-host=host:ip or --add-host host:ip)
    let mut extra_hosts = Vec::new();
    for part in opts.split_whitespace() {
        if part.starts_with("--add-host=") {
            if let Some(host) = part.strip_prefix("--add-host=") {
                extra_hosts.push(host.to_string());
            }
        }
    }
    if !extra_hosts.is_empty() {
        args.extra_hosts = Some(extra_hosts.join(","));
    }

    args
}
