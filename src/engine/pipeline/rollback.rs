use anyhow::{Context, Result};
use std::sync::Arc;

use crate::db::App;
use crate::runtime::{ContainerRuntime, PortMapping, RunConfig};
use crate::DbPool;

use super::super::{add_deployment_log, update_deployment_status, KEY_LENGTH};
use super::start::collect_env_vars;
use super::{AutoRollbackTriggered, DeploymentResult};

/// Trim old successful deployments to keep only the last `retention` entries.
/// Also removes deployment logs for the trimmed deployments.
pub async fn trim_old_deployments(db: &DbPool, app_id: &str, retention: i64) -> Result<()> {
    // Get IDs of successful deployments older than the retention limit
    let old_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM deployments WHERE app_id = ? AND status = 'success' \
         ORDER BY started_at DESC LIMIT -1 OFFSET ?",
    )
    .bind(app_id)
    .bind(retention)
    .fetch_all(db)
    .await?;

    for id in old_ids {
        sqlx::query("DELETE FROM deployment_logs WHERE deployment_id = ?")
            .bind(&id)
            .execute(db)
            .await?;
        sqlx::query("DELETE FROM deployments WHERE id = ?")
            .bind(&id)
            .execute(db)
            .await?;
    }

    Ok(())
}

/// Rollback to a previous deployment by restarting with the old image.
/// This does NOT rebuild the image - it reuses the existing image from the target deployment.
pub async fn run_rollback(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    rollback_deployment_id: &str,
    target_deployment: &crate::db::Deployment,
    app: &App,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
) -> Result<DeploymentResult> {
    let image_tag = target_deployment
        .image_tag
        .as_ref()
        .context("Target deployment has no image tag - cannot rollback")?;

    add_deployment_log(
        db,
        rollback_deployment_id,
        "info",
        &format!(
            "Rolling back to deployment {} with image {}",
            target_deployment.id, image_tag
        ),
    )
    .await?;
    update_deployment_status(db, rollback_deployment_id, "starting", None).await?;

    // Rename the current container so the rollback container can claim the canonical name
    // while the old one keeps serving traffic until proxy routes are swapped.
    let container_name = format!("rivetr-{}", app.name);
    let old_container_prev_name = format!("{}-prev", container_name);
    let mut old_container_ids: Vec<String> = Vec::new();

    match runtime.inspect(&container_name).await {
        Ok(_) => {
            if let Err(e) = runtime
                .rename_container(&container_name, &old_container_prev_name)
                .await
            {
                tracing::warn!(
                    error = %e,
                    container = %container_name,
                    "Could not rename old container for zero-downtime rollback swap; stopping it now"
                );
                let _ = runtime.stop(&container_name).await;
                let _ = runtime.remove(&container_name).await;
            } else {
                old_container_ids.push(old_container_prev_name.clone());
            }
        }
        Err(_) => {
            // No existing container to rename.
        }
    }

    // Pass the rollback deployment ID so SOURCE_COMMIT is set from the rollback record
    let env_vars = collect_env_vars(db, app, encryption_key, Some(rollback_deployment_id)).await;

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

    let rollback_cap_add: Vec<String> = app
        .cap_add
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let rollback_cap_drop: Vec<String> = app
        .docker_cap_drop
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let rollback_devices: Vec<String> = app
        .devices
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let rollback_shm_size: Option<i64> = app
        .shm_size
        .as_ref()
        .and_then(|s| crate::runtime::parse_shm_size(s));
    let rollback_ulimits: Vec<String> = app
        .docker_ulimits
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let rollback_security_opt: Vec<String> = app
        .docker_security_opt
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

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
        labels: app.get_container_labels(),
        binds,
        restart_policy: app.restart_policy.clone(),
        privileged: app.privileged != 0,
        cap_add: rollback_cap_add,
        cap_drop: rollback_cap_drop,
        devices: rollback_devices,
        shm_size: rollback_shm_size,
        init: app.init_process != 0,
        app_id: Some(app.id.clone()),
        gpus: app.docker_gpus.clone(),
        ulimits: rollback_ulimits,
        security_opt: rollback_security_opt,
        cmd: None,
    };

    add_deployment_log(
        db,
        rollback_deployment_id,
        "info",
        "Starting rollback container...",
    )
    .await?;
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
        add_deployment_log(
            db,
            rollback_deployment_id,
            "info",
            "Running health check...",
        )
        .await?;
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

    add_deployment_log(
        db,
        rollback_deployment_id,
        "info",
        "Rollback completed successfully",
    )
    .await?;
    update_deployment_status(db, rollback_deployment_id, "running", None).await?;

    Ok(DeploymentResult {
        container_id,
        image_tag: image_tag.clone(),
        port: final_info.port,
        auto_rollback_from: None,
        old_container_ids,
    })
}

/// Trigger an automatic rollback to the previous successful deployment.
/// Called when health check fails and auto_rollback_enabled is true.
///
/// `prev_old_container_ids`: container IDs that were already renamed/captured before the
/// failed deployment started (i.e. the containers that were serving traffic before this
/// deployment).  They should be stopped after the rollback's proxy routes are updated.
pub(super) async fn trigger_auto_rollback(
    db: &DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    failed_deployment_id: &str,
    app: &App,
    encryption_key: Option<&[u8; KEY_LENGTH]>,
    prev_old_container_ids: Vec<String>,
) -> Result<AutoRollbackTriggered> {
    use crate::db::Deployment;

    // Find the previous successful deployment with an image_tag (not the current one)
    let target_deployment: Option<Deployment> = sqlx::query_as(
        r#"
        SELECT * FROM deployments
        WHERE app_id = ?
          AND id != ?
          AND image_tag IS NOT NULL
          AND status IN ('running', 'replaced', 'stopped')
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .bind(&app.id)
    .bind(failed_deployment_id)
    .fetch_optional(db)
    .await?;

    let target = target_deployment.ok_or_else(|| {
        anyhow::anyhow!("No previous deployment with image found for auto-rollback")
    })?;

    // Create a new deployment record for the auto-rollback
    let rollback_deployment_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO deployments (id, app_id, status, started_at, image_tag, rollback_from_deployment_id, is_auto_rollback)
        VALUES (?, ?, 'pending', ?, ?, ?, 1)
        "#,
    )
    .bind(&rollback_deployment_id)
    .bind(&app.id)
    .bind(&now)
    .bind(&target.image_tag)
    .bind(failed_deployment_id)
    .execute(db)
    .await?;

    add_deployment_log(
        db,
        &rollback_deployment_id,
        "info",
        &format!(
            "Auto-rollback initiated due to health check failure in deployment {}",
            failed_deployment_id
        ),
    )
    .await?;

    // Execute the rollback
    match run_rollback(
        db,
        runtime,
        &rollback_deployment_id,
        &target,
        app,
        encryption_key,
    )
    .await
    {
        Ok(rollback_result) => {
            tracing::info!(
                rollback_deployment_id = %rollback_deployment_id,
                target_deployment_id = %target.id,
                failed_deployment_id = %failed_deployment_id,
                "Auto-rollback completed successfully"
            );

            // Merge: containers from the rollback itself + those from before the failed deploy
            let mut all_old = rollback_result.old_container_ids;
            for id in prev_old_container_ids {
                if !all_old.contains(&id) {
                    all_old.push(id);
                }
            }

            Ok(AutoRollbackTriggered {
                failed_deployment_id: failed_deployment_id.to_string(),
                rollback_deployment_id,
                target_deployment_id: target.id.clone(),
                old_container_ids: all_old,
            })
        }
        Err(e) => {
            // Update rollback deployment as failed
            let _ = update_deployment_status(
                db,
                &rollback_deployment_id,
                "failed",
                Some(&e.to_string()),
            )
            .await;
            Err(anyhow::anyhow!("Auto-rollback execution failed: {}", e))
        }
    }
}
