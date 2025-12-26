use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StatsOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecResults};
use bollard::image::{PruneImagesOptions, RemoveImageOptions};
use bollard::Docker;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::{BuildContext, ContainerInfo, ContainerRuntime, ContainerStats, ExecConfig, ExecHandle, LogLine, LogStream, RunConfig, TtySize};

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

        let options = BuildImageOptions {
            dockerfile: ctx.dockerfile.trim_start_matches("./"),
            t: &ctx.tag,
            rm: true,
            ..Default::default()
        };

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

        let port_binding = format!("{}/tcp", config.port);
        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            port_binding.clone(),
            Some(vec![bollard::service::PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: None, // Let Docker assign a random port
            }]),
        );

        let host_config = bollard::service::HostConfig {
            port_bindings: Some(port_bindings),
            memory: config.memory_limit.as_ref().and_then(|m| parse_memory(m)),
            nano_cpus: config.cpu_limit.as_ref().and_then(|c| parse_cpu(c)),
            ..Default::default()
        };

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(port_binding, HashMap::new());

        let container_config = Config {
            image: Some(config.image.clone()),
            env: Some(env),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
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
            .context("Failed to create container")?;

        self.client
            .start_container::<String>(&response.id, None)
            .await
            .context("Failed to start container")?;

        Ok(response.id)
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
            follow: true,
            ..Default::default()
        };

        let stream = self.client.logs(container_id, Some(options));

        let mapped = stream.filter_map(|result| async move {
            match result {
                Ok(output) => {
                    let (stream, message) = match output {
                        LogOutput::StdOut { message } => (LogStream::Stdout, message),
                        LogOutput::StdErr { message } => (LogStream::Stderr, message),
                        _ => return None,
                    };
                    Some(LogLine {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        message: String::from_utf8_lossy(&message).to_string(),
                        stream,
                    })
                }
                Err(_) => None,
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

        let port = info
            .network_settings
            .and_then(|ns| ns.ports)
            .and_then(|ports| {
                ports.values().next().and_then(|bindings| {
                    bindings.as_ref().and_then(|b| {
                        b.first()
                            .and_then(|p| p.host_port.as_ref())
                            .and_then(|p| p.parse().ok())
                    })
                })
            });

        Ok(ContainerInfo {
            id: info.id.unwrap_or_default(),
            name: info.name.unwrap_or_default(),
            status: info
                .state
                .and_then(|s| s.status)
                .map(|s| format!("{:?}", s))
                .unwrap_or_default(),
            port,
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

            result.push(ContainerInfo {
                id: container.id.unwrap_or_default(),
                name,
                status: container.state.unwrap_or_default(),
                port,
            });
        }

        Ok(result)
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats> {
        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.client.stats(container_id, Some(options));

        if let Some(result) = stream.next().await {
            let stats = result.context("Failed to get container stats")?;

            // Calculate CPU percentage
            // CPU percentage = (container_delta / system_delta) * num_cpus * 100
            let cpu_stats = &stats.cpu_stats;
            let precpu_stats = &stats.precpu_stats;

            let cpu_delta = cpu_stats.cpu_usage.total_usage as f64
                - precpu_stats.cpu_usage.total_usage as f64;

            let system_delta = cpu_stats.system_cpu_usage.unwrap_or(0) as f64
                - precpu_stats.system_cpu_usage.unwrap_or(0) as f64;

            let num_cpus = cpu_stats
                .online_cpus
                .or(cpu_stats.cpu_usage.percpu_usage.as_ref().map(|v: &Vec<u64>| v.len() as u64))
                .unwrap_or(1) as f64;

            let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 {
                (cpu_delta / system_delta) * num_cpus * 100.0
            } else {
                0.0
            };

            // Get memory stats
            let memory_stats = &stats.memory_stats;
            let memory_usage = memory_stats.usage.unwrap_or(0);
            let memory_limit = memory_stats.limit.unwrap_or(0);

            // Get network stats
            let (network_rx, network_tx) = if let Some(networks) = &stats.networks {
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
        } else {
            anyhow::bail!("No stats received for container")
        }
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
