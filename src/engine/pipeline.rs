use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::App;
use crate::runtime::{BuildContext, ContainerRuntime, RunConfig};
use crate::DbPool;

use super::{add_deployment_log, update_deployment_status};

/// Information about a successfully deployed container
pub struct DeploymentResult {
    pub container_id: String,
    pub port: Option<u16>,
}

pub async fn run_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
) -> Result<DeploymentResult> {
    let work_dir = std::env::temp_dir().join(format!("rivetr-{}", deployment_id));

    // Step 1: Clone
    add_deployment_log(db, deployment_id, "info", &format!("Cloning repository: {}", app.git_url)).await?;
    update_deployment_status(db, deployment_id, "cloning", None).await?;

    clone_repository(&app.git_url, &app.branch, &work_dir).await?;
    add_deployment_log(db, deployment_id, "info", "Repository cloned successfully").await?;

    // Step 2: Build
    add_deployment_log(db, deployment_id, "info", "Building Docker image...").await?;
    update_deployment_status(db, deployment_id, "building", None).await?;

    let image_tag = format!("rivetr-{}:{}", app.name, deployment_id);
    let build_ctx = BuildContext {
        path: work_dir.to_string_lossy().to_string(),
        dockerfile: app.dockerfile.clone(),
        tag: image_tag.clone(),
        build_args: vec![],
    };

    runtime.build(&build_ctx).await.context("Build failed")?;
    add_deployment_log(db, deployment_id, "info", "Image built successfully").await?;

    // Step 3: Stop old container (if exists)
    let container_name = format!("rivetr-{}", app.name);
    let _ = runtime.stop(&container_name).await;
    let _ = runtime.remove(&container_name).await;

    // Step 4: Start new container
    add_deployment_log(db, deployment_id, "info", "Starting container...").await?;
    update_deployment_status(db, deployment_id, "starting", None).await?;

    // Get env vars from database
    let env_vars = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM env_vars WHERE app_id = ?",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let run_config = RunConfig {
        image: image_tag,
        name: container_name.clone(),
        port: app.port as u16,
        env: env_vars,
        memory_limit: app.memory_limit.clone(),
        cpu_limit: app.cpu_limit.clone(),
    };

    let container_id = runtime.run(&run_config).await.context("Failed to start container")?;

    // Update deployment with container ID
    sqlx::query("UPDATE deployments SET container_id = ? WHERE id = ?")
        .bind(&container_id)
        .bind(deployment_id)
        .execute(db)
        .await?;

    // Step 5: Health check
    if let Some(healthcheck) = &app.healthcheck {
        add_deployment_log(db, deployment_id, "info", "Running health check...").await?;
        update_deployment_status(db, deployment_id, "checking", None).await?;

        // Get the assigned port
        let info = runtime.inspect(&container_id).await?;
        if let Some(port) = info.port {
            let health_url = format!("http://127.0.0.1:{}{}", port, healthcheck);

            // Retry health check a few times
            let mut healthy = false;
            for attempt in 1..=10 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                match reqwest::get(&health_url).await {
                    Ok(resp) if resp.status().is_success() => {
                        healthy = true;
                        break;
                    }
                    Ok(resp) => {
                        add_deployment_log(
                            db,
                            deployment_id,
                            "warn",
                            &format!("Health check attempt {}: status {}", attempt, resp.status()),
                        )
                        .await?;
                    }
                    Err(e) => {
                        add_deployment_log(
                            db,
                            deployment_id,
                            "warn",
                            &format!("Health check attempt {}: {}", attempt, e),
                        )
                        .await?;
                    }
                }
            }

            if !healthy {
                // Rollback: stop the new container
                let _ = runtime.stop(&container_id).await;
                let _ = runtime.remove(&container_id).await;
                anyhow::bail!("Health check failed after 10 attempts");
            }
        }

        add_deployment_log(db, deployment_id, "info", "Health check passed").await?;
    }

    // Step 6: Get final container info for route update
    let final_info = runtime.inspect(&container_id).await?;

    // Step 7: Done
    add_deployment_log(db, deployment_id, "info", "Deployment completed successfully").await?;
    update_deployment_status(db, deployment_id, "running", None).await?;

    // Cleanup work directory
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    Ok(DeploymentResult {
        container_id,
        port: final_info.port,
    })
}

async fn clone_repository(url: &str, branch: &str, dest: &PathBuf) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // Use git CLI for simplicity (git2 requires more setup for SSH)
    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            url,
            &dest.to_string_lossy(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone failed: {}", stderr);
    }

    Ok(())
}
