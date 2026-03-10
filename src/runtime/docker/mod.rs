mod build;
mod container;
mod logs;

use anyhow::Result;
use async_trait::async_trait;
use bollard::Docker;
use futures::Stream;
use std::pin::Pin;

use super::{
    BuildContext, CommandResult, ContainerInfo, ContainerRuntime, ContainerStats, ExecConfig,
    ExecHandle, LogLine, RegistryAuth, RunConfig,
};

pub struct DockerRuntime {
    pub(super) client: Docker,
}

impl DockerRuntime {
    pub fn new(socket: &str) -> Result<Self> {
        // On Windows, always use local defaults (named pipe)
        // On Unix, use socket path if specified
        let client =
            if cfg!(windows) || socket.starts_with("npipe://") || socket.starts_with("tcp://") {
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
        build::build(self, ctx).await
    }

    async fn run(&self, config: &RunConfig) -> Result<String> {
        container::run(self, config).await
    }

    async fn start(&self, container_id: &str) -> Result<()> {
        container::start(self, container_id).await
    }

    async fn stop(&self, container_id: &str) -> Result<()> {
        container::stop(self, container_id).await
    }

    async fn remove(&self, container_id: &str) -> Result<()> {
        container::remove(self, container_id).await
    }

    async fn logs(
        &self,
        container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        logs::logs(self, container_id).await
    }

    async fn logs_stream(
        &self,
        container_id: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = LogLine> + Send>>> {
        logs::logs_stream(self, container_id).await
    }

    async fn inspect(&self, container_id: &str) -> Result<ContainerInfo> {
        container::inspect(self, container_id).await
    }

    async fn is_available(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    async fn list_containers(&self, name_prefix: &str) -> Result<Vec<ContainerInfo>> {
        container::list_containers(self, name_prefix).await
    }

    async fn list_compose_containers(&self, project_name: &str) -> Result<Vec<ContainerInfo>> {
        container::list_compose_containers(self, project_name).await
    }

    async fn stats(&self, container_id: &str) -> Result<ContainerStats> {
        container::stats(self, container_id).await
    }

    async fn remove_image(&self, image: &str) -> Result<()> {
        container::remove_image(self, image).await
    }

    async fn prune_images(&self) -> Result<u64> {
        container::prune_images(self).await
    }

    async fn exec(&self, config: &ExecConfig) -> Result<ExecHandle> {
        container::exec(self, config).await
    }

    async fn run_command(&self, container_id: &str, cmd: Vec<String>) -> Result<CommandResult> {
        container::run_command(self, container_id, cmd).await
    }

    async fn pull_image(&self, image: &str, auth: Option<&RegistryAuth>) -> Result<()> {
        container::pull_image(self, image, auth).await
    }
}
