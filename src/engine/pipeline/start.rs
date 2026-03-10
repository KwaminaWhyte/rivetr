use anyhow::{Context, Result};
use std::sync::Arc;

use crate::crypto;
use crate::db::App;
use crate::runtime::{ContainerRuntime, PortMapping, RunConfig};
use crate::DbPool;

use super::super::{add_deployment_log, update_deployment_status, KEY_LENGTH};
use super::DeploymentResult;

/// Collect and decrypt all env vars for an app (app + environment + project + team layers)
pub(super) async fn collect_env_vars(
    db: &DbPool,
    app: &App,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Vec<(String, String)> {
    // Get env vars from database
    let raw_env_vars =
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM env_vars WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(db)
            .await
            .unwrap_or_default();

    // Decrypt env var values if encryption is enabled
    let mut env_vars: Vec<(String, String)> = raw_env_vars
        .into_iter()
        .map(|(key, value)| {
            let decrypted =
                crypto::decrypt_if_encrypted(&value, encryption_key).unwrap_or_else(|e| {
                    tracing::warn!("Failed to decrypt env var {}: {}", key, e);
                    value
                });
            (key, decrypted)
        })
        .collect();

    // Automatically set PORT environment variable if not already set
    if !env_vars.iter().any(|(k, _)| k == "PORT") {
        env_vars.push(("PORT".to_string(), app.port.to_string()));
    }

    // Inject predefined Rivetr system variables (if not already set by user)
    if !env_vars.iter().any(|(k, _)| k == "RIVETR_ENV") {
        env_vars.push(("RIVETR_ENV".to_string(), app.environment.clone()));
    }
    if !env_vars.iter().any(|(k, _)| k == "RIVETR_APP_NAME") {
        env_vars.push(("RIVETR_APP_NAME".to_string(), app.name.clone()));
    }
    if !env_vars.iter().any(|(k, _)| k == "RIVETR_URL") {
        if let Some(domain) = app.get_primary_domain() {
            env_vars.push(("RIVETR_URL".to_string(), format!("https://{}", domain)));
        }
    }

    // Merge environment-scoped env vars (from project environment)
    if let Some(ref environment_id) = app.environment_id {
        let env_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM environment_env_vars WHERE environment_id = ?",
        )
        .bind(environment_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        for (key, value) in env_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted = crypto::decrypt_if_encrypted(&value, encryption_key)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt environment env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    // Merge project-level shared env vars
    if let Some(ref project_id) = app.project_id {
        let project_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM project_env_vars WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        for (key, value) in project_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted = crypto::decrypt_if_encrypted(&value, encryption_key)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt project env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    // Merge team-level shared env vars (lowest priority)
    if let Some(ref team_id) = app.team_id {
        let team_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM team_env_vars WHERE team_id = ?",
        )
        .bind(team_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        for (key, value) in team_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted = crypto::decrypt_if_encrypted(&value, encryption_key)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt team env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    env_vars
}

/// Start the container, run replicas, execute deploy commands, health check, and finalize
pub(super) async fn start_container(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    deployment_id: &str,
    app: &App,
    image_tag: String,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<DeploymentResult> {
    use super::build::execute_deployment_commands;
    use super::rollback::trigger_auto_rollback;

    // Step 3: Stop old containers (primary and any replicas)
    let container_name = format!("rivetr-{}", app.name);
    let _ = runtime.stop(&container_name).await;
    let _ = runtime.remove(&container_name).await;

    // Also stop any running replicas from previous deployment
    let old_replicas = sqlx::query_as::<_, crate::db::AppReplica>(
        "SELECT * FROM app_replicas WHERE app_id = ? AND status = 'running'",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for old_replica in &old_replicas {
        if let Some(ref cid) = old_replica.container_id {
            let _ = runtime.stop(cid).await;
            let _ = runtime.remove(cid).await;
        }
    }
    // Clear old replica records
    let _ = sqlx::query("DELETE FROM app_replicas WHERE app_id = ?")
        .bind(&app.id)
        .execute(db)
        .await;

    // Step 4: Start new container
    add_deployment_log(db, deployment_id, "info", "Starting container...").await?;
    update_deployment_status(db, deployment_id, "starting", None).await?;

    let env_vars = collect_env_vars(db, app, encryption_key).await;

    // Get volumes from database
    let volumes = sqlx::query_as::<_, crate::db::Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE app_id = ?",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Convert volumes to bind mount strings
    let binds: Vec<String> = volumes.iter().map(|v| v.to_bind_mount()).collect();

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
        labels: app.get_container_labels(),
        binds,
    };

    let container_id = runtime
        .run(&run_config)
        .await
        .context("Failed to start container")?;

    // Update deployment with container ID and image tag
    sqlx::query("UPDATE deployments SET container_id = ?, image_tag = ? WHERE id = ?")
        .bind(&container_id)
        .bind(&run_config.image)
        .bind(deployment_id)
        .execute(db)
        .await?;

    // Record primary container as replica 0
    {
        let replica_id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO app_replicas (id, app_id, replica_index, container_id, status, started_at)
             VALUES (?, ?, 0, ?, 'running', datetime('now'))",
        )
        .bind(&replica_id)
        .bind(&app.id)
        .bind(&container_id)
        .execute(db)
        .await;
    }

    // Start additional replicas if replica_count > 1
    let replica_count = app.replica_count.max(1);
    if replica_count > 1 {
        add_deployment_log(
            db,
            deployment_id,
            "info",
            &format!("Starting {} additional replica(s)...", replica_count - 1),
        )
        .await?;

        for i in 1..replica_count {
            let replica_name = format!("rivetr-{}-{}", app.name, i);
            let mut replica_config = run_config.clone();
            replica_config.name = replica_name;
            // Additional replicas don't need explicit port mappings (ephemeral ports)
            replica_config.port_mappings = vec![];

            match runtime.run(&replica_config).await {
                Ok(replica_container_id) => {
                    let replica_id = uuid::Uuid::new_v4().to_string();
                    let _ = sqlx::query(
                        "INSERT INTO app_replicas (id, app_id, replica_index, container_id, status, started_at)
                         VALUES (?, ?, ?, ?, 'running', datetime('now'))",
                    )
                    .bind(&replica_id)
                    .bind(&app.id)
                    .bind(i)
                    .bind(&replica_container_id)
                    .execute(db)
                    .await;

                    add_deployment_log(
                        db,
                        deployment_id,
                        "info",
                        &format!("Replica {} started: {}", i, replica_container_id),
                    )
                    .await?;
                }
                Err(e) => {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        &format!("Failed to start replica {}: {}", i, e),
                    )
                    .await?;
                }
            }
        }
    }

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
                // Stop the unhealthy container
                let _ = runtime.stop(&container_id).await;
                let _ = runtime.remove(&container_id).await;

                // Check if auto-rollback is enabled
                if app.is_auto_rollback_enabled() {
                    add_deployment_log(
                        db,
                        deployment_id,
                        "warn",
                        "Health check failed. Auto-rollback is enabled, attempting to rollback to previous version...",
                    )
                    .await?;

                    // Try to trigger auto-rollback
                    match trigger_auto_rollback(
                        db,
                        runtime.clone(),
                        deployment_id,
                        app,
                        encryption_key,
                    )
                    .await
                    {
                        Ok(rollback_info) => {
                            add_deployment_log(
                                db,
                                deployment_id,
                                "info",
                                &format!(
                                    "Auto-rollback initiated to deployment {}. Rollback deployment ID: {}",
                                    rollback_info.target_deployment_id,
                                    rollback_info.rollback_deployment_id
                                ),
                            )
                            .await?;

                            // Return the auto-rollback error so the engine knows what happened
                            return Err(rollback_info.into());
                        }
                        Err(rollback_err) => {
                            add_deployment_log(
                                db,
                                deployment_id,
                                "error",
                                &format!("Auto-rollback failed: {}. No previous deployment available for rollback.", rollback_err),
                            )
                            .await?;
                            anyhow::bail!("Health check failed after 10 attempts. Auto-rollback also failed: {}", rollback_err);
                        }
                    }
                }

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
        }
    }

    // Step 8: Get final container info for route update
    let final_info = runtime.inspect(&container_id).await?;

    // Step 9: Done
    add_deployment_log(
        db,
        deployment_id,
        "info",
        "Deployment completed successfully",
    )
    .await?;
    update_deployment_status(db, deployment_id, "running", None).await?;

    Ok(DeploymentResult {
        container_id,
        image_tag: run_config.image,
        port: final_info.port,
        auto_rollback_from: None,
    })
}
