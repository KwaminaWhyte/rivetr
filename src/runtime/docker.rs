use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StopContainerOptions,
};
use bollard::Docker;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;

use super::{BuildContext, ContainerInfo, ContainerRuntime, LogLine, LogStream, RunConfig};

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
}

fn parse_memory(s: &str) -> Option<i64> {
    let s = s.to_lowercase();
    if s.ends_with("gb") {
        s.trim_end_matches("gb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024 * 1024)
    } else if s.ends_with("mb") {
        s.trim_end_matches("mb")
            .parse::<i64>()
            .ok()
            .map(|n| n * 1024 * 1024)
    } else {
        s.parse().ok()
    }
}

fn parse_cpu(s: &str) -> Option<i64> {
    s.parse::<f64>().ok().map(|n| (n * 1_000_000_000.0) as i64)
}
