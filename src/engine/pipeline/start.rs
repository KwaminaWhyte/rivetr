use anyhow::{Context, Result};
use std::sync::Arc;

use crate::crypto;
use crate::db::App;
use crate::runtime::{ContainerRuntime, PortMapping, RunConfig};
use crate::DbPool;

use super::super::{add_deployment_log, update_deployment_status, KEY_LENGTH};
use super::DeploymentResult;

/// Collect and decrypt all env vars for an app (app + environment + project + team layers).
/// `deployment_id` is used to look up the deployment's commit SHA for the SOURCE_COMMIT variable.
pub(super) async fn collect_env_vars(
    db: &DbPool,
    app: &App,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
    deployment_id: Option<&str>,
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
    // RIVETR_FQDN: bare hostname without protocol (convenient complement to RIVETR_URL)
    if !env_vars.iter().any(|(k, _)| k == "RIVETR_FQDN") {
        if let Some(domain) = app.get_primary_domain() {
            env_vars.push(("RIVETR_FQDN".to_string(), domain));
        }
    }
    // SOURCE_COMMIT: the git commit SHA for this deployment
    if !env_vars.iter().any(|(k, _)| k == "SOURCE_COMMIT") {
        if let Some(dep_id) = deployment_id {
            let commit_sha: Option<String> =
                sqlx::query_scalar("SELECT commit_sha FROM deployments WHERE id = ?")
                    .bind(dep_id)
                    .fetch_optional(db)
                    .await
                    .unwrap_or(None)
                    .flatten();
            if let Some(sha) = commit_sha {
                // Exclude upload-based "commit" paths
                if !sha.is_empty() && !sha.contains("rivetr-upload-") {
                    env_vars.push(("SOURCE_COMMIT".to_string(), sha));
                }
            }
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
    use super::rollback::{trigger_auto_rollback, trim_old_deployments};

    // Step 3: Capture and rename old containers for zero-downtime swap.
    //
    // The old primary container uses the canonical name "rivetr-<app>".  To allow
    // the new container to claim that same name (Docker forbids duplicate names) while
    // the old one is still alive and serving traffic, we rename it to
    // "rivetr-<app>-prev".  The old container continues running under the new name;
    // the proxy still routes to it by container ID until we swap routes after health
    // check.  After the proxy swap the caller stops the renamed old container.
    let container_name = format!("rivetr-{}", app.name);
    let old_container_prev_name = format!("{}-prev", container_name);

    // Collect IDs to stop after proxy swap.
    let mut old_container_ids: Vec<String> = Vec::new();

    // Try to rename the current primary container so the new one can use the canonical name.
    // If inspect fails (no prior container), the rename is a no-op.
    match runtime.inspect(&container_name).await {
        Ok(_) => {
            // A container with the canonical name exists — rename it to free up the name.
            if let Err(e) = runtime
                .rename_container(&container_name, &old_container_prev_name)
                .await
            {
                // If rename fails (e.g., container already renamed from a previous partial run),
                // fall back to stopping it immediately so the new container can start.
                tracing::warn!(
                    error = %e,
                    container = %container_name,
                    "Could not rename old container for zero-downtime swap; stopping it now"
                );
                let _ = runtime.stop(&container_name).await;
                let _ = runtime.remove(&container_name).await;
            } else {
                // Renamed successfully — schedule for cleanup after proxy swap.
                old_container_ids.push(old_container_prev_name.clone());
            }
        }
        Err(_) => {
            // No existing container — first deployment or already removed.
        }
    }

    // Also collect running replica container IDs for cleanup after proxy swap.
    let old_replicas = sqlx::query_as::<_, crate::db::AppReplica>(
        "SELECT * FROM app_replicas WHERE app_id = ? AND status = 'running'",
    )
    .bind(&app.id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for old_replica in &old_replicas {
        if let Some(ref cid) = old_replica.container_id {
            old_container_ids.push(cid.clone());
        }
    }

    // Clear old replica records (the containers themselves are stopped after route swap).
    let _ = sqlx::query("DELETE FROM app_replicas WHERE app_id = ?")
        .bind(&app.id)
        .execute(db)
        .await;

    // Step 4: Start new container
    add_deployment_log(db, deployment_id, "info", "Starting container...").await?;
    update_deployment_status(db, deployment_id, "starting", None).await?;

    let env_vars = collect_env_vars(db, app, encryption_key, Some(deployment_id)).await;

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

    // Parse custom Docker run options from app settings
    let cap_add: Vec<String> = app
        .cap_add
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let devices: Vec<String> = app
        .devices
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let shm_size: Option<i64> = app
        .shm_size
        .as_ref()
        .and_then(|s| crate::runtime::parse_shm_size(s));
    let cap_drop: Vec<String> = app
        .docker_cap_drop
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let ulimits: Vec<String> = app
        .docker_ulimits
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let security_opt: Vec<String> = app
        .docker_security_opt
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

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
        restart_policy: app.restart_policy.clone(),
        privileged: app.privileged != 0,
        cap_add,
        cap_drop,
        devices,
        shm_size,
        init: app.init_process != 0,
        app_id: Some(app.id.clone()),
        gpus: app.docker_gpus.clone(),
        ulimits,
        security_opt,
        cmd: None,
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

                    // Try to trigger auto-rollback, passing old container IDs so they get
                    // cleaned up after the rollback proxy swap.
                    match trigger_auto_rollback(
                        db,
                        runtime.clone(),
                        deployment_id,
                        app,
                        encryption_key,
                        old_container_ids.clone(),
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

    // Step 10: Trim old deployments according to retention policy
    let retention = app.rollback_retention_count.max(1);
    if let Err(e) = trim_old_deployments(db, &app.id, retention).await {
        tracing::warn!(
            app_id = %app.id,
            error = %e,
            "Failed to trim old deployments (non-fatal)"
        );
    }

    Ok(DeploymentResult {
        container_id,
        image_tag: run_config.image,
        port: final_info.port,
        auto_rollback_from: None,
        old_container_ids,
    })
}
