use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

use crate::db::{App, SshKey};
use crate::runtime::{BuildContext, ContainerRuntime, PortMapping, RunConfig};
use crate::DbPool;

use super::{add_deployment_log, update_deployment_status, BuildLimits};

/// Execute deployment commands (pre or post) in a container
/// For pre-deploy: runs in a temporary container using the built image
/// For post-deploy: runs in the running container using docker exec
async fn execute_deployment_commands(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    container_id: &str,
    commands: &[String],
    phase: &str,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("Executing {} deployment commands ({} total)", phase, commands.len()),
    )
    .await?;

    for (i, command) in commands.iter().enumerate() {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("[{}/{}] Running: {}", i + 1, commands.len(), command),
        )
        .await?;

        // Execute command in container using shell
        let cmd = vec!["/bin/sh".to_string(), "-c".to_string(), command.clone()];
        let result = runtime.run_command(container_id, cmd).await?;

        // Log command output
        if !result.stdout.is_empty() {
            // Truncate very long output
            let stdout = if result.stdout.len() > 4000 {
                format!("{}... (truncated)", &result.stdout[..4000])
            } else {
                result.stdout.clone()
            };
            add_deployment_log(db, deployment_id, "info", &format!("Output: {}", stdout.trim()))
                .await?;
        }

        if !result.stderr.is_empty() {
            let stderr = if result.stderr.len() > 4000 {
                format!("{}... (truncated)", &result.stderr[..4000])
            } else {
                result.stderr.clone()
            };
            add_deployment_log(db, deployment_id, "warn", &format!("Stderr: {}", stderr.trim()))
                .await?;
        }

        // Check exit code
        if result.exit_code != 0 {
            add_deployment_log(
                db,
                deployment_id,
                "error",
                &format!(
                    "Command failed with exit code {}: {}",
                    result.exit_code, command
                ),
            )
            .await?;
            anyhow::bail!(
                "{} command failed with exit code {}: {}",
                phase,
                result.exit_code,
                command
            );
        }

        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("[{}/{}] Command completed successfully", i + 1, commands.len()),
        )
        .await?;
    }

    add_deployment_log(
        db,
        deployment_id,
        "info",
        &format!("All {} deployment commands completed", phase),
    )
    .await?;

    Ok(())
}

/// Information about a successfully deployed container
pub struct DeploymentResult {
    pub container_id: String,
    pub image_tag: String,
    pub port: Option<u16>,
}

pub async fn run_deployment(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    build_limits: &BuildLimits,
) -> Result<DeploymentResult> {
    let work_dir = std::env::temp_dir().join(format!("rivetr-{}", deployment_id));

    // Step 1: Clone
    add_deployment_log(db, deployment_id, "info", &format!("Cloning repository: {}", app.git_url)).await?;
    update_deployment_status(db, deployment_id, "cloning", None).await?;

    // Get SSH key if configured for this app
    let ssh_key = get_ssh_key_for_app(db, app).await?;

    clone_repository(&app.git_url, &app.branch, &work_dir, ssh_key.as_ref()).await?;
    add_deployment_log(db, deployment_id, "info", "Repository cloned successfully").await?;

    // Step 2: Build
    add_deployment_log(db, deployment_id, "info", "Building Docker image...").await?;
    update_deployment_status(db, deployment_id, "building", None).await?;

    // Determine the actual build path (consider base_directory)
    let build_path = if let Some(ref base_dir) = app.base_directory {
        if !base_dir.is_empty() {
            work_dir.join(base_dir)
        } else {
            work_dir.clone()
        }
    } else {
        work_dir.clone()
    };

    // Determine the dockerfile to use (dockerfile_path takes precedence over dockerfile)
    let dockerfile = app
        .dockerfile_path
        .as_ref()
        .filter(|p| !p.is_empty())
        .cloned()
        .unwrap_or_else(|| app.dockerfile.clone());

    let image_tag = format!("rivetr-{}:{}", app.name, deployment_id);
    let build_ctx = BuildContext {
        path: build_path.to_string_lossy().to_string(),
        dockerfile,
        tag: image_tag.clone(),
        build_args: vec![],
        build_target: app.build_target.clone(),
        custom_options: app.custom_docker_options.clone(),
        cpu_limit: build_limits.cpu_limit.clone(),
        memory_limit: build_limits.memory_limit.clone(),
    };

    // Log build resource limits if configured
    if build_limits.cpu_limit.is_some() || build_limits.memory_limit.is_some() {
        let mut limits = vec![];
        if let Some(ref cpu) = build_limits.cpu_limit {
            limits.push(format!("cpu={}", cpu));
        }
        if let Some(ref mem) = build_limits.memory_limit {
            limits.push(format!("memory={}", mem));
        }
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("Build resource limits: {}", limits.join(", ")),
        )
        .await?;
    }

    // Log build options if any are set
    if app.base_directory.is_some() || app.dockerfile_path.is_some() || app.build_target.is_some() {
        let mut opts = vec![];
        if let Some(ref base_dir) = app.base_directory {
            if !base_dir.is_empty() {
                opts.push(format!("base_directory={}", base_dir));
            }
        }
        if let Some(ref df_path) = app.dockerfile_path {
            if !df_path.is_empty() {
                opts.push(format!("dockerfile_path={}", df_path));
            }
        }
        if let Some(ref target) = app.build_target {
            if !target.is_empty() {
                opts.push(format!("target={}", target));
            }
        }
        if !opts.is_empty() {
            add_deployment_log(
                db,
                deployment_id,
                "info",
                &format!("Build options: {}", opts.join(", ")),
            )
            .await?;
        }
    }

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

    // Parse network configuration from app
    let port_mappings: Vec<PortMapping> = app
        .get_port_mappings()
        .into_iter()
        .map(|pm| PortMapping {
            host_port: pm.host_port,
            container_port: pm.container_port,
            protocol: pm.protocol,
        })
        .collect();

    let run_config = RunConfig {
        image: image_tag,
        name: container_name.clone(),
        port: app.port as u16,
        env: env_vars,
        memory_limit: app.memory_limit.clone(),
        cpu_limit: app.cpu_limit.clone(),
        port_mappings,
        network_aliases: app.get_network_aliases(),
        extra_hosts: app.get_extra_hosts(),
    };

    let container_id = runtime.run(&run_config).await.context("Failed to start container")?;

    // Update deployment with container ID and image tag
    sqlx::query("UPDATE deployments SET container_id = ?, image_tag = ? WHERE id = ?")
        .bind(&container_id)
        .bind(&run_config.image)
        .bind(deployment_id)
        .execute(db)
        .await?;

    // Step 5: Execute pre-deploy commands (before health check)
    let pre_deploy_commands = app.get_pre_deploy_commands();
    if !pre_deploy_commands.is_empty() {
        // Wait a brief moment for container to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Err(e) = execute_deployment_commands(
            db,
            runtime.clone(),
            deployment_id,
            &container_id,
            &pre_deploy_commands,
            "pre",
        )
        .await
        {
            // Rollback: stop the container if pre-deploy commands fail
            let _ = runtime.stop(&container_id).await;
            let _ = runtime.remove(&container_id).await;
            return Err(e);
        }
    }

    // Step 6: Health check
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

    // Step 7: Execute post-deploy commands (after health check)
    let post_deploy_commands = app.get_post_deploy_commands();
    if !post_deploy_commands.is_empty() {
        if let Err(e) = execute_deployment_commands(
            db,
            runtime.clone(),
            deployment_id,
            &container_id,
            &post_deploy_commands,
            "post",
        )
        .await
        {
            // Log the error but don't rollback - container is already healthy
            add_deployment_log(
                db,
                deployment_id,
                "error",
                &format!("Post-deploy command failed: {}. Container is running but commands did not complete.", e),
            )
            .await?;
            // We don't rollback here because the container is healthy and running
            // The user can fix the commands and redeploy
        }
    }

    // Step 8: Get final container info for route update
    let final_info = runtime.inspect(&container_id).await?;

    // Step 9: Done
    add_deployment_log(db, deployment_id, "info", "Deployment completed successfully").await?;
    update_deployment_status(db, deployment_id, "running", None).await?;

    // Cleanup work directory
    let _ = tokio::fs::remove_dir_all(&work_dir).await;

    Ok(DeploymentResult {
        container_id,
        image_tag: run_config.image,
        port: final_info.port,
    })
}

/// Get SSH key for an app - checks app-specific key first, then falls back to global key
async fn get_ssh_key_for_app(db: &DbPool, app: &App) -> Result<Option<SshKey>> {
    // First, check if app has a specific SSH key configured
    if let Some(ref ssh_key_id) = app.ssh_key_id {
        let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
            .bind(ssh_key_id)
            .fetch_optional(db)
            .await?;
        if key.is_some() {
            return Ok(key);
        }
    }

    // Check for an app-specific SSH key (linked via app_id)
    let app_key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE app_id = ?")
        .bind(&app.id)
        .fetch_optional(db)
        .await?;
    if app_key.is_some() {
        return Ok(app_key);
    }

    // Fall back to global SSH key
    let global_key = sqlx::query_as::<_, SshKey>(
        "SELECT * FROM ssh_keys WHERE is_global = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(db)
    .await?;

    Ok(global_key)
}

async fn clone_repository(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: Option<&SshKey>,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // If we have an SSH key and the URL is an SSH URL, set up SSH authentication
    if let Some(key) = ssh_key {
        if is_ssh_url(url) {
            return clone_with_ssh_key(url, branch, dest, key).await;
        }
    }

    // Use git CLI for public repos or HTTPS URLs
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

/// Check if a URL is an SSH URL (git@host:path or ssh://...)
fn is_ssh_url(url: &str) -> bool {
    url.starts_with("git@") || url.starts_with("ssh://")
}

/// Clone a repository using SSH key authentication
async fn clone_with_ssh_key(
    url: &str,
    branch: &str,
    dest: &PathBuf,
    ssh_key: &SshKey,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    // Create a temporary file for the SSH key
    let temp_dir = std::env::temp_dir();
    let key_file = temp_dir.join(format!("rivetr-ssh-{}", uuid::Uuid::new_v4()));

    // Write the private key to the temp file
    tokio::fs::write(&key_file, &ssh_key.private_key).await?;

    // Set proper permissions on the key file (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&key_file).await?.permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(&key_file, perms).await?;
    }

    // Build GIT_SSH_COMMAND to use our key file
    let git_ssh_command = format!(
        "ssh -i {} -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null",
        key_file.display()
    );

    let output = Command::new("git")
        .env("GIT_SSH_COMMAND", &git_ssh_command)
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
        .context("Failed to execute git clone with SSH key")?;

    // Clean up the temporary key file
    let _ = tokio::fs::remove_file(&key_file).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Git clone with SSH failed: {}", stderr);
    }

    Ok(())
}

/// Rollback to a previous deployment by restarting with the old image
/// This does NOT rebuild the image - it reuses the existing image from the target deployment
pub async fn run_rollback(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    rollback_deployment_id: &str,
    target_deployment: &crate::db::Deployment,
    app: &App,
) -> Result<DeploymentResult> {
    let image_tag = target_deployment
        .image_tag
        .as_ref()
        .context("Target deployment has no image tag - cannot rollback")?;

    add_deployment_log(
        db,
        rollback_deployment_id,
        "info",
        &format!("Rolling back to deployment {} with image {}", target_deployment.id, image_tag),
    )
    .await?;
    update_deployment_status(db, rollback_deployment_id, "starting", None).await?;

    // Stop current container
    let container_name = format!("rivetr-{}", app.name);
    let _ = runtime.stop(&container_name).await;
    let _ = runtime.remove(&container_name).await;

    // Get env vars from database
    let env_vars = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM env_vars WHERE app_id = ?",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Parse network configuration from app
    let port_mappings: Vec<PortMapping> = app
        .get_port_mappings()
        .into_iter()
        .map(|pm| PortMapping {
            host_port: pm.host_port,
            container_port: pm.container_port,
            protocol: pm.protocol,
        })
        .collect();

    let run_config = RunConfig {
        image: image_tag.clone(),
        name: container_name.clone(),
        port: app.port as u16,
        env: env_vars,
        memory_limit: app.memory_limit.clone(),
        cpu_limit: app.cpu_limit.clone(),
        port_mappings,
        network_aliases: app.get_network_aliases(),
        extra_hosts: app.get_extra_hosts(),
    };

    add_deployment_log(db, rollback_deployment_id, "info", "Starting rollback container...").await?;
    let container_id = runtime
        .run(&run_config)
        .await
        .context("Failed to start rollback container")?;

    // Update deployment with container ID and image tag
    sqlx::query("UPDATE deployments SET container_id = ?, image_tag = ? WHERE id = ?")
        .bind(&container_id)
        .bind(image_tag)
        .bind(rollback_deployment_id)
        .execute(db)
        .await?;

    // Health check
    if let Some(healthcheck) = &app.healthcheck {
        add_deployment_log(db, rollback_deployment_id, "info", "Running health check...").await?;
        update_deployment_status(db, rollback_deployment_id, "checking", None).await?;

        let info = runtime.inspect(&container_id).await?;
        if let Some(port) = info.port {
            let health_url = format!("http://127.0.0.1:{}{}", port, healthcheck);

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
                            rollback_deployment_id,
                            "warn",
                            &format!("Health check attempt {}: status {}", attempt, resp.status()),
                        )
                        .await?;
                    }
                    Err(e) => {
                        add_deployment_log(
                            db,
                            rollback_deployment_id,
                            "warn",
                            &format!("Health check attempt {}: {}", attempt, e),
                        )
                        .await?;
                    }
                }
            }

            if !healthy {
                let _ = runtime.stop(&container_id).await;
                let _ = runtime.remove(&container_id).await;
                anyhow::bail!("Health check failed after 10 attempts during rollback");
            }
        }

        add_deployment_log(db, rollback_deployment_id, "info", "Health check passed").await?;
    }

    // Get final container info
    let final_info = runtime.inspect(&container_id).await?;

    add_deployment_log(db, rollback_deployment_id, "info", "Rollback completed successfully").await?;
    update_deployment_status(db, rollback_deployment_id, "running", None).await?;

    Ok(DeploymentResult {
        container_id,
        image_tag: image_tag.clone(),
        port: final_info.port,
    })
}
