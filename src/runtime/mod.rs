mod docker;
mod podman;

pub use docker::parse_shm_size;
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

#[derive(Clone)]
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
    /// Optional channel to stream build log lines to the deployment log store
    pub log_tx: Option<mpsc::UnboundedSender<String>>,
    /// Build-time secrets injected via BuildKit `--secret` (key, plaintext value pairs).
    /// Each secret is written to a tmpfile and passed as `--secret id=KEY,src=TMPFILE`.
    pub build_secrets: Vec<(String, String)>,
    /// Target Docker build platform(s), e.g. "linux/amd64" or "linux/arm64".
    /// When set, `docker buildx build --platform` is used instead of the default daemon build.
    pub build_platforms: Option<String>,
    /// Force a clean build by disabling layer cache (`--no-cache`).
    pub no_cache: bool,
    /// Inject `SOURCE_COMMIT` build arg with the current git SHA.
    /// Value is `None` when the feature is disabled or the SHA is unknown.
    pub source_commit: Option<String>,
}

impl std::fmt::Debug for BuildContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildContext")
            .field("path", &self.path)
            .field("dockerfile", &self.dockerfile)
            .field("tag", &self.tag)
            .field("build_args", &self.build_args)
            .field("build_target", &self.build_target)
            .field("cpu_limit", &self.cpu_limit)
            .field("memory_limit", &self.memory_limit)
            .field("log_tx", &self.log_tx.is_some())
            .field(
                "build_secrets",
                &self
                    .build_secrets
                    .iter()
                    .map(|(k, _)| k.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("build_platforms", &self.build_platforms)
            .field("no_cache", &self.no_cache)
            .field("source_commit", &self.source_commit)
            .finish()
    }
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
    /// Container labels (key-value pairs)
    pub labels: std::collections::HashMap<String, String>,
    /// Volume bind mounts (format: host_path:container_path[:ro])
    pub binds: Vec<String>,
    /// Container restart policy: "always", "unless-stopped", "on-failure", or "never"
    pub restart_policy: String,
    // Custom Docker run options
    /// Run container in privileged mode
    pub privileged: bool,
    /// Capabilities to add (e.g. ["NET_ADMIN", "SYS_PTRACE"])
    pub cap_add: Vec<String>,
    /// Device mappings (e.g. ["/dev/snd:/dev/snd"])
    pub devices: Vec<String>,
    /// Shared memory size in bytes
    pub shm_size: Option<i64>,
    /// Run tini as PID 1
    pub init: bool,
    /// App ID used to name the per-app Docker network (`rivetr-app-{app_id}`).
    /// When set, the container is also connected to a dedicated per-app network in
    /// addition to the shared `rivetr` bridge.
    pub app_id: Option<String>,
    /// Capabilities to drop
    pub cap_drop: Vec<String>,
    /// GPU access: "all" or "device=0,1" — None means no GPU
    pub gpus: Option<String>,
    /// Ulimits (e.g. ["nofile=1024:1024"])
    pub ulimits: Vec<String>,
    /// Security options (e.g. ["seccomp=unconfined"])
    pub security_opt: Vec<String>,
    /// Override the container CMD (command + args after the entrypoint).
    /// When `None` the image's default CMD is used.
    pub cmd: Option<Vec<String>>,
    /// Override the default "rivetr" network with a named destination network.
    /// When `None`, the shared "rivetr" bridge network is used.
    pub network: Option<String>,
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
    /// Number of times Docker/Podman has restarted this container
    pub restart_count: u32,
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

/// Registry credentials for pulling images from private registries
#[derive(Debug, Clone, Default)]
pub struct RegistryAuth {
    pub username: Option<String>,
    pub password: Option<String>,
    /// Optional server address (e.g., "ghcr.io", "registry.example.com")
    pub server: Option<String>,
}

impl RegistryAuth {
    pub fn new(username: Option<String>, password: Option<String>, server: Option<String>) -> Self {
        Self {
            username,
            password,
            server,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.username.is_none() && self.password.is_none()
    }
}

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Return the name of the runtime ("Docker" or "Podman")
    fn name(&self) -> &'static str;
    async fn build(&self, ctx: &BuildContext) -> Result<String>;
    async fn run(&self, config: &RunConfig) -> Result<String>;
    /// Start a stopped container
    async fn start(&self, container_id: &str) -> Result<()>;
    async fn stop(&self, container_id: &str) -> Result<()>;
    async fn remove(&self, container_id: &str) -> Result<()>;
    async fn logs(&self, container_id: &str)
        -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    /// Stream logs from a container in real-time (follow mode)
    async fn logs_stream(
        &self,
        container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>>;
    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo>;
    async fn is_available(&self) -> bool;
    /// List running containers with names matching the given prefix
    async fn list_containers(&self, name_prefix: &str) -> Result<Vec<ContainerInfo>>;
    /// List running containers belonging to a Docker Compose project
    async fn list_compose_containers(&self, project_name: &str) -> Result<Vec<ContainerInfo>>;
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
    /// Pull an image from a registry, optionally with authentication
    async fn pull_image(&self, image: &str, auth: Option<&RegistryAuth>) -> Result<()>;
    /// Ensure the shared container network exists and connect all existing
    /// Rivetr-managed containers to it (no-op for non-Docker runtimes).
    async fn setup_shared_network(&self) {}
    /// Rename a container (used for zero-downtime swaps).
    /// Default implementation is a no-op; runtimes that support renaming override this.
    async fn rename_container(&self, _container_id: &str, _new_name: &str) -> Result<()> {
        Ok(())
    }

    /// Update CPU/memory limits on a running container without restarting it.
    /// Default implementation returns an error (not all runtimes support live updates).
    async fn apply_resource_limits(
        &self,
        _container_id: &str,
        _memory_limit: Option<&str>,
        _cpu_limit: Option<&str>,
    ) -> Result<()> {
        anyhow::bail!("Live resource limit updates not supported by this runtime. Redeploy to apply new limits.")
    }
}

/// A no-op runtime used when no container runtime is available
pub struct NoopRuntime;

#[async_trait]
impl ContainerRuntime for NoopRuntime {
    fn name(&self) -> &'static str {
        "None"
    }
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
    async fn logs(
        &self,
        _container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        anyhow::bail!("No container runtime available")
    }
    async fn logs_stream(
        &self,
        _container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
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
    async fn list_compose_containers(&self, _project_name: &str) -> Result<Vec<ContainerInfo>> {
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
    async fn pull_image(&self, _image: &str, _auth: Option<&RegistryAuth>) -> Result<()> {
        anyhow::bail!("No container runtime available")
    }
}

pub async fn detect_runtime(
    config: &crate::config::RuntimeConfig,
) -> Result<Arc<dyn ContainerRuntime>> {
    match config.runtime_type {
        RuntimeType::Docker => match DockerRuntime::new(&config.docker_socket) {
            Ok(runtime) => Ok(Arc::new(runtime)),
            Err(e) => {
                tracing::warn!(
                    "Failed to connect to Docker: {}. Deployments will not work.",
                    e
                );
                Ok(Arc::new(NoopRuntime))
            }
        },
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
