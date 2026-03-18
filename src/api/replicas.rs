use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::{App, AppReplica};
use crate::proxy::Backend;
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

#[derive(Debug, Deserialize)]
pub struct SetReplicaCountRequest {
    pub count: i64,
}

/// GET /api/apps/:id/replicas — list replicas with status
pub async fn list_replicas(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<AppReplica>>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let replicas = sqlx::query_as::<_, AppReplica>(
        "SELECT * FROM app_replicas WHERE app_id = ? ORDER BY replica_index ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(replicas))
}

/// PUT /api/apps/:id/replicas/count — set replica count
pub async fn set_replica_count(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<SetReplicaCountRequest>,
) -> Result<Json<Vec<AppReplica>>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    if req.count < 1 || req.count > 10 {
        return Err(ApiError::bad_request(
            "Replica count must be between 1 and 10",
        ));
    }

    // Fetch app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let current_count = app.replica_count;

    // Update replica_count in apps table
    sqlx::query("UPDATE apps SET replica_count = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(req.count)
        .bind(&id)
        .execute(&state.db)
        .await?;

    // Check if app is currently running by looking for a running deployment
    let running_deployment: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id, container_id FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    if let Some((deployment_id, Some(primary_container_id))) = running_deployment {
        let _ = deployment_id; // may be used later

        // Get image tag from the running deployment
        let image_tag: Option<(Option<String>,)> =
            sqlx::query_as("SELECT image_tag FROM deployments WHERE container_id = ?")
                .bind(&primary_container_id)
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None);

        let image_tag = image_tag
            .and_then(|(tag,)| tag)
            .unwrap_or_else(|| format!("rivetr-{}:latest", app.name));

        if req.count > current_count {
            // Start additional replicas
            for i in current_count..req.count {
                let replica_name = format!("rivetr-{}-{}", app.name, i);

                // Get env vars for new replicas (reuse app's env vars)
                let env_vars: Vec<(String, String)> =
                    sqlx::query_as("SELECT key, value FROM env_vars WHERE app_id = ?")
                        .bind(&app.id)
                        .fetch_all(&state.db)
                        .await
                        .unwrap_or_default();

                let run_config = crate::runtime::RunConfig {
                    image: image_tag.clone(),
                    name: replica_name.clone(),
                    port: app.port as u16,
                    env: env_vars,
                    memory_limit: app.memory_limit.clone(),
                    cpu_limit: app.cpu_limit.clone(),
                    port_mappings: vec![],
                    network_aliases: app.get_network_aliases(),
                    extra_hosts: app.get_extra_hosts(),
                    labels: app.get_container_labels(),
                    binds: vec![],
                    restart_policy: app.restart_policy.clone(),
                    privileged: app.privileged != 0,
                    cap_add: app
                        .cap_add
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default(),
                    cap_drop: app
                        .docker_cap_drop
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default(),
                    devices: app
                        .devices
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default(),
                    shm_size: app
                        .shm_size
                        .as_ref()
                        .and_then(|s| crate::runtime::parse_shm_size(s)),
                    init: app.init_process != 0,
                    app_id: Some(app.id.clone()),
                    gpus: app.docker_gpus.clone(),
                    ulimits: app
                        .docker_ulimits
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default(),
                    security_opt: app
                        .docker_security_opt
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default(),
                    cmd: None,
                    network: None,
                    custom_labels: vec![],
                };

                match state.runtime.run(&run_config).await {
                    Ok(container_id) => {
                        let replica_id = uuid::Uuid::new_v4().to_string();
                        let _ = sqlx::query(
                            "INSERT INTO app_replicas (id, app_id, replica_index, container_id, status, started_at)
                             VALUES (?, ?, ?, ?, 'running', datetime('now'))
                             ON CONFLICT(id) DO NOTHING",
                        )
                        .bind(&replica_id)
                        .bind(&app.id)
                        .bind(i)
                        .bind(&container_id)
                        .execute(&state.db)
                        .await;

                        // Get the port and add to proxy routes
                        if let Ok(info) = state.runtime.inspect(&container_id).await {
                            if let Some(port) = info.port {
                                let domain_entries = app.get_all_domains_with_redirects();
                                let route_table = state.routes.load();
                                for (domain, www_redirect_target) in &domain_entries {
                                    let mut backend = Backend::new(
                                        container_id.clone(),
                                        "127.0.0.1".to_string(),
                                        port,
                                    )
                                    .with_healthcheck(app.healthcheck.clone());
                                    backend.www_redirect_target = www_redirect_target.clone();
                                    route_table.add_route(domain.clone(), backend);
                                }
                            }
                        }

                        tracing::info!(replica = i, container = %container_id, "Started additional replica");
                    }
                    Err(e) => {
                        tracing::error!(replica = i, error = %e, "Failed to start replica");
                        let replica_id = uuid::Uuid::new_v4().to_string();
                        let _ = sqlx::query(
                            "INSERT INTO app_replicas (id, app_id, replica_index, container_id, status)
                             VALUES (?, ?, ?, NULL, 'error')
                             ON CONFLICT(id) DO NOTHING",
                        )
                        .bind(&replica_id)
                        .bind(&app.id)
                        .bind(i)
                        .execute(&state.db)
                        .await;
                    }
                }
            }
        } else if req.count < current_count {
            // Stop excess replicas (stop replicas with index >= new count)
            let excess_replicas = sqlx::query_as::<_, AppReplica>(
                "SELECT * FROM app_replicas WHERE app_id = ? AND replica_index >= ? ORDER BY replica_index DESC",
            )
            .bind(&app.id)
            .bind(req.count)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            for replica in excess_replicas {
                if let Some(ref container_id) = replica.container_id {
                    let _ = state.runtime.stop(container_id).await;
                    let _ = state.runtime.remove(container_id).await;
                }

                let _ = sqlx::query(
                    "UPDATE app_replicas SET status = 'stopped', stopped_at = datetime('now') WHERE id = ?",
                )
                .bind(&replica.id)
                .execute(&state.db)
                .await;
            }
        }
    }

    // Return updated replica list
    let replicas = sqlx::query_as::<_, AppReplica>(
        "SELECT * FROM app_replicas WHERE app_id = ? ORDER BY replica_index ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(replicas))
}

/// POST /api/apps/:id/replicas/:index/restart — restart specific replica
pub async fn restart_replica(
    State(state): State<Arc<AppState>>,
    Path((id, index)): Path<(String, i64)>,
) -> Result<Json<AppReplica>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Fetch the app
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Find the replica
    let replica = sqlx::query_as::<_, AppReplica>(
        "SELECT * FROM app_replicas WHERE app_id = ? AND replica_index = ?",
    )
    .bind(&id)
    .bind(index)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Replica not found"))?;

    // Stop old container if running
    if let Some(ref old_container_id) = replica.container_id {
        let _ = state.runtime.stop(old_container_id).await;
        let _ = state.runtime.remove(old_container_id).await;
    }

    // Get image from running deployment
    let image_tag: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT image_tag FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&app.id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let image_tag = image_tag
        .and_then(|(tag,)| tag)
        .unwrap_or_else(|| format!("rivetr-{}:latest", app.name));

    let replica_name = format!("rivetr-{}-{}", app.name, index);

    let env_vars: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM env_vars WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let run_config = crate::runtime::RunConfig {
        image: image_tag,
        name: replica_name,
        port: app.port as u16,
        env: env_vars,
        memory_limit: app.memory_limit.clone(),
        cpu_limit: app.cpu_limit.clone(),
        port_mappings: vec![],
        network_aliases: app.get_network_aliases(),
        extra_hosts: app.get_extra_hosts(),
        labels: app.get_container_labels(),
        binds: vec![],
        restart_policy: app.restart_policy.clone(),
        privileged: app.privileged != 0,
        cap_add: app
            .cap_add
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        cap_drop: app
            .docker_cap_drop
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        devices: app
            .devices
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        shm_size: app
            .shm_size
            .as_ref()
            .and_then(|s| crate::runtime::parse_shm_size(s)),
        init: app.init_process != 0,
        app_id: Some(app.id.clone()),
        gpus: app.docker_gpus.clone(),
        ulimits: app
            .docker_ulimits
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        security_opt: app
            .docker_security_opt
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default(),
        cmd: None,
        network: None,
        custom_labels: vec![],
    };

    // Update status to starting
    sqlx::query("UPDATE app_replicas SET status = 'starting', container_id = NULL WHERE id = ?")
        .bind(&replica.id)
        .execute(&state.db)
        .await?;

    match state.runtime.run(&run_config).await {
        Ok(new_container_id) => {
            sqlx::query(
                "UPDATE app_replicas SET status = 'running', container_id = ?, started_at = datetime('now'), stopped_at = NULL WHERE id = ?",
            )
            .bind(&new_container_id)
            .bind(&replica.id)
            .execute(&state.db)
            .await?;

            tracing::info!(
                replica = index,
                container = %new_container_id,
                "Replica restarted"
            );
        }
        Err(e) => {
            sqlx::query("UPDATE app_replicas SET status = 'error' WHERE id = ?")
                .bind(&replica.id)
                .execute(&state.db)
                .await?;

            return Err(ApiError::internal(format!(
                "Failed to restart replica: {}",
                e
            )));
        }
    }

    let updated_replica =
        sqlx::query_as::<_, AppReplica>("SELECT * FROM app_replicas WHERE id = ?")
            .bind(&replica.id)
            .fetch_one(&state.db)
            .await?;

    Ok(Json(updated_replica))
}
