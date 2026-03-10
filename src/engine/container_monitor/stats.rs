//! Docker Compose service status checking helpers.

use std::sync::Arc;

use crate::runtime::ContainerRuntime;

/// Check if a Docker Compose project has running containers (used during reconciliation)
pub(super) async fn check_compose_running(
    project_name: &str,
    runtime: &Arc<dyn ContainerRuntime>,
) -> bool {
    use tokio::process::Command;

    let output = Command::new("docker")
        .arg("compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("--format")
        .arg("json")
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("\"State\":\"running\"")
                    || stdout.contains("\"Status\":\"running\"")
                {
                    return true;
                }
                if stdout.contains("running") && !stdout.trim().is_empty() {
                    return true;
                }
                false
            } else {
                check_compose_running_legacy(project_name, runtime).await
            }
        }
        Err(_) => check_compose_running_legacy(project_name, runtime).await,
    }
}

/// Check if a Docker Compose project has running containers using legacy command
pub(super) async fn check_compose_running_legacy(
    project_name: &str,
    runtime: &Arc<dyn ContainerRuntime>,
) -> bool {
    use tokio::process::Command;

    let output = Command::new("docker-compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("-q")
        .output()
        .await;

    match output {
        Ok(output) => {
            let container_ids = String::from_utf8_lossy(&output.stdout);
            if container_ids.trim().is_empty() {
                return false;
            }

            for container_id in container_ids.lines() {
                let container_id = container_id.trim();
                if container_id.is_empty() {
                    continue;
                }

                if let Ok(info) = runtime.inspect(container_id).await {
                    if info.running {
                        return true;
                    }
                }
            }

            false
        }
        Err(_) => false,
    }
}

/// Check if a Docker Compose service is running (used by ContainerMonitor during monitoring cycles)
pub(super) async fn check_compose_service_running(
    project_name: &str,
    service_name: &str,
    runtime: &Arc<dyn ContainerRuntime>,
) -> bool {
    use tokio::process::Command;

    let output = Command::new("docker")
        .arg("compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("--format")
        .arg("json")
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("\"State\":\"running\"")
                    || stdout.contains("\"Status\":\"running\"")
                {
                    return true;
                }
                if stdout.contains("running") && !stdout.trim().is_empty() {
                    return true;
                }
                false
            } else {
                check_compose_service_running_legacy(project_name, service_name, runtime).await
            }
        }
        Err(_) => {
            check_compose_service_running_legacy(project_name, service_name, runtime).await
        }
    }
}

/// Check if a Docker Compose service is running using legacy docker-compose command
pub(super) async fn check_compose_service_running_legacy(
    project_name: &str,
    _service_name: &str,
    runtime: &Arc<dyn ContainerRuntime>,
) -> bool {
    use tokio::process::Command;

    let output = Command::new("docker-compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("-q")
        .output()
        .await;

    match output {
        Ok(output) => {
            let container_ids = String::from_utf8_lossy(&output.stdout);
            if container_ids.trim().is_empty() {
                return false;
            }

            for container_id in container_ids.lines() {
                let container_id = container_id.trim();
                if container_id.is_empty() {
                    continue;
                }

                if let Ok(info) = runtime.inspect(container_id).await {
                    if info.running {
                        return true;
                    }
                }
            }

            false
        }
        Err(_) => false,
    }
}
