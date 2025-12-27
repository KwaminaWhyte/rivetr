mod docker;
mod podman;

pub use docker::DockerRuntime;
pub use podman::PodmanRuntime;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::config::RuntimeType;

#[derive(Debug, Clone)]
pub struct BuildContext {
    pub path: String,
    pub dockerfile: String,
    pub tag: String,
    pub build_args: Vec<(String, String)>,
    /// Docker multi-stage build target (--target flag)
    pub build_target: Option<String>,
    /// Custom Docker build options (extra CLI args)
    pub custom_options: Option<String>,
    /// CPU limit for build (e.g., "2" for 2 CPUs)
    pub cpu_limit: Option<String>,
    /// Memory limit for build (e.g., "2g" for 2GB)
    pub memory_limit: Option<String>,
}

/// Port mapping for container networking
#[derive(Debug, Clone)]
pub struct PortMapping {
    /// Host port to bind (0 for auto-assign)
    pub host_port: u16,
    /// Container port to expose
    pub container_port: u16,
    /// Protocol (tcp or udp)
    pub protocol: String,
}

impl PortMapping {
    pub fn new(host_port: u16, container_port: u16) -> Self {
        Self {
            host_port,
            container_port,
            protocol: "tcp".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub image: String,
    pub name: String,
    /// Primary port (legacy, used if port_mappings is empty)
    pub port: u16,
    pub env: Vec<(String, String)>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    /// Additional port mappings
    pub port_mappings: Vec<PortMapping>,
    /// Network aliases for the container
    pub network_aliases: Vec<String>,
    /// Extra hosts entries (hostname:ip format)
    pub extra_hosts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub port: Option<u16>,
    /// Whether the container is currently running
    pub running: bool,
    /// Host port the container is listening on
    pub host_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: String,
    pub message: String,
    pub stream: LogStream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

/// Container resource statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContainerStats {
    /// CPU usage percentage (0-100, can exceed 100 on multi-core)
    pub cpu_percent: f64,
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Memory limit in bytes (0 if no limit)
    pub memory_limit: u64,
    /// Network bytes received
    pub network_rx: u64,
    /// Network bytes transmitted
    pub network_tx: u64,
}

/// Configuration for executing a command in a container
#[derive(Debug, Clone)]
pub struct ExecConfig {
    /// Container ID or name
    pub container_id: String,
    /// Command to execute (e.g., ["/bin/sh"] for shell access)
    pub cmd: Vec<String>,
    /// Whether to allocate a pseudo-TTY
    pub tty: bool,
}

/// TTY size for resize operations
#[derive(Debug, Clone, Copy)]
pub struct TtySize {
    /// Number of columns
    pub cols: u16,
    /// Number of rows
    pub rows: u16,
}

/// Handle for interacting with an exec session
pub struct ExecHandle {
    /// Send data to the container's stdin
    pub stdin_tx: mpsc::Sender<Bytes>,
    /// Receive data from the container's stdout/stderr
    pub stdout_rx: mpsc::Receiver<Bytes>,
    /// Send resize events to the exec session
    pub resize_tx: mpsc::Sender<TtySize>,
}

/// Result of running a command in a container
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit code of the command (0 = success)
    pub exit_code: i32,
    /// Combined stdout output
    pub stdout: String,
    /// Combined stderr output
    pub stderr: String,
}

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn build(&self, ctx: &BuildContext) -> Result<String>;
    async fn run(&self, config: &RunConfig) -> Result<String>;
    /// Start a stopped container
    async fn start(&self, container_id: &str) -> Result<()>;
    async fn stop(&self, container_id: &str) -> Result<()>;
    async fn remove(&self, container_id: &str) -> Result<()>;
    async fn logs(&self, container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo>;
    async fn is_available(&self) -> bool;
    /// List running containers with names matching the given prefix
    async fn list_containers(&self, name_prefix: &str) -> Result<Vec<ContainerInfo>>;
    /// Get container resource statistics (CPU, memory, network)
    async fn stats(&self, container_id: &str) -> Result<ContainerStats>;
    /// Remove a container image by tag or ID
    async fn remove_image(&self, image: &str) -> Result<()>;
    /// Prune unused/dangling images, returns bytes reclaimed
    async fn prune_images(&self) -> Result<u64>;
    /// Execute a command in a running container with bidirectional I/O
    async fn exec(&self, config: &ExecConfig) -> Result<ExecHandle>;
    /// Run a command in a container and wait for completion, returning the output
    async fn run_command(&self, container_id: &str, cmd: Vec<String>) -> Result<CommandResult>;
}

/// A no-op runtime used when no container runtime is available
pub struct NoopRuntime;

#[async_trait]
impl ContainerRuntime for NoopRuntime {
    async fn build(&self, _ctx: &BuildContext) -> Result<String> {
        anyhow::bail!("No container runtime available")
    }
    async fn run(&self, _config: &RunConfig) -> Result<String> {
        anyhow::bail!("No container runtime available")
    }
    async fn start(&self, _container_id: &str) -> Result<()> {
        anyhow::bail!("No container runtime available")
    }
    async fn stop(&self, _container_id: &str) -> Result<()> {
        anyhow::bail!("No container runtime available")
    }
    async fn remove(&self, _container_id: &str) -> Result<()> {
        anyhow::bail!("No container runtime available")
    }
    async fn logs(&self, _container_id: &str) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        anyhow::bail!("No container runtime available")
    }
    async fn inspect(&self, _container_id: &str) -> Result<ContainerInfo> {
        anyhow::bail!("No container runtime available")
    }
    async fn is_available(&self) -> bool {
        false
    }
    async fn list_containers(&self, _name_prefix: &str) -> Result<Vec<ContainerInfo>> {
        Ok(vec![])
    }
    async fn stats(&self, _container_id: &str) -> Result<ContainerStats> {
        anyhow::bail!("No container runtime available")
    }
    async fn remove_image(&self, _image: &str) -> Result<()> {
        anyhow::bail!("No container runtime available")
    }
    async fn prune_images(&self) -> Result<u64> {
        anyhow::bail!("No container runtime available")
    }
    async fn exec(&self, _config: &ExecConfig) -> Result<ExecHandle> {
        anyhow::bail!("No container runtime available")
    }
    async fn run_command(&self, _container_id: &str, _cmd: Vec<String>) -> Result<CommandResult> {
        anyhow::bail!("No container runtime available")
    }
}

pub async fn detect_runtime(config: &crate::config::RuntimeConfig) -> Result<Arc<dyn ContainerRuntime>> {
    match config.runtime_type {
        RuntimeType::Docker => {
            match DockerRuntime::new(&config.docker_socket) {
                Ok(runtime) => Ok(Arc::new(runtime)),
                Err(e) => {
                    tracing::warn!("Failed to connect to Docker: {}. Deployments will not work.", e);
                    Ok(Arc::new(NoopRuntime))
                }
            }
        }
        RuntimeType::Podman => {
            let runtime = PodmanRuntime::new();
            Ok(Arc::new(runtime))
        }
        RuntimeType::Auto => {
            // Try Docker first
            if let Ok(docker) = DockerRuntime::new(&config.docker_socket) {
                if docker.is_available().await {
                    tracing::info!("Auto-detected Docker runtime");
                    return Ok(Arc::new(docker));
                }
            }

            // Try Podman
            let podman = PodmanRuntime::new();
            if podman.is_available().await {
                tracing::info!("Auto-detected Podman runtime");
                return Ok(Arc::new(podman));
            }

            tracing::warn!("No container runtime available. Deployments will not work until Docker or Podman is installed.");
            Ok(Arc::new(NoopRuntime))
        }
    }
}
