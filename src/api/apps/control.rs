use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::crypto;
use crate::db::{
    actions, list_audit_logs, resource_types, App, AuditLogListResponse, AuditLogQuery, Deployment,
    User,
};
use crate::runtime::{PortMapping, RunConfig};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::AppStatusResponse;

/// Key length for AES-256 encryption (32 bytes)
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config, if configured.
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// Collect and decrypt all env vars for an app (app + environment + project + team layers).
/// Mirrors `src/engine/pipeline/start.rs::collect_env_vars`.
async fn collect_env_vars_for_restart(state: &AppState, app: &App) -> Vec<(String, String)> {
    let encryption_key = get_encryption_key(state);
    let enc_key_ref: Option<&[u8; KEY_LENGTH]> = encryption_key.as_ref();

    // App-level env vars
    let raw_env_vars =
        sqlx::query_as::<_, (String, String)>("SELECT key, value FROM env_vars WHERE app_id = ?")
            .bind(&app.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    let mut env_vars: Vec<(String, String)> = raw_env_vars
        .into_iter()
        .map(|(key, value)| {
            let decrypted = crypto::decrypt_if_encrypted(&value, enc_key_ref).unwrap_or_else(|e| {
                tracing::warn!("Failed to decrypt env var {}: {}", key, e);
                value
            });
            (key, decrypted)
        })
        .collect();

    // Inject PORT if not already set
    if !env_vars.iter().any(|(k, _)| k == "PORT") {
        env_vars.push(("PORT".to_string(), app.port.to_string()));
    }

    // Rivetr system variables
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

    // Environment-scoped env vars
    if let Some(ref environment_id) = app.environment_id {
        let env_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM environment_env_vars WHERE environment_id = ?",
        )
        .bind(environment_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for (key, value) in env_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted =
                    crypto::decrypt_if_encrypted(&value, enc_key_ref).unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt environment env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    // Project-level shared env vars
    if let Some(ref project_id) = app.project_id {
        let project_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM project_env_vars WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for (key, value) in project_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted =
                    crypto::decrypt_if_encrypted(&value, enc_key_ref).unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt project env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    // Team-level shared env vars (lowest priority)
    if let Some(ref team_id) = app.team_id {
        let team_env_vars = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM team_env_vars WHERE team_id = ?",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for (key, value) in team_env_vars {
            if !env_vars.iter().any(|(k, _)| k == &key) {
                let decrypted =
                    crypto::decrypt_if_encrypted(&value, enc_key_ref).unwrap_or_else(|e| {
                        tracing::warn!("Failed to decrypt team env var {}: {}", key, e);
                        value
                    });
                env_vars.push((key, decrypted));
            }
        }
    }

    env_vars
}

/// Get current running status of an app
pub async fn get_app_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest deployment (any status) for phase detection
    let latest_deployment: Option<(String, String, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT id, status, container_id, started_at FROM deployments WHERE app_id = ? ORDER BY started_at DESC LIMIT 1"
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await?;

    // Derive deployment phase and active deployment info from the latest deployment
    let (deployment_phase, active_deployment_id, uptime_seconds) =
        if let Some((dep_id, dep_status, _dep_container, dep_started_at)) = &latest_deployment {
            let phase = match dep_status.as_str() {
                "running" => "stable",
                "cloning" | "building" => "deploying",
                "starting" => "deploying",
                "checking" => "health_checking",
                "switching" => "switching",
                _ => "stable",
            };
            let uptime = if dep_status == "running" {
                dep_started_at.as_deref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| {
                        let now = chrono::Utc::now();
                        let started: chrono::DateTime<chrono::Utc> = dt.into();
                        (now - started).num_seconds()
                    })
                })
            } else {
                None
            };
            (phase.to_string(), Some(dep_id.clone()), uptime)
        } else {
            ("stable".to_string(), None, None)
        };

    // Get the latest running or stopped deployment's container
    let deployment: Option<(String, String)> = sqlx::query_as(
        "SELECT container_id, status FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let (container_id, running, status, host_port) = if let Some((cid, dep_status)) = deployment {
        if cid.is_empty() {
            (None, false, "no_container".to_string(), None)
        } else if dep_status == "stopped" {
            // Manually stopped — don't inspect the container, just report stopped
            (Some(cid), false, "stopped".to_string(), None)
        } else {
            // Check if container is running
            match state.runtime.inspect(&cid).await {
                Ok(info) => (
                    Some(cid),
                    info.running,
                    if info.running { "running" } else { "stopped" }.to_string(),
                    info.host_port,
                ),
                Err(_) => (Some(cid), false, "not_found".to_string(), None),
            }
        }
    } else {
        (None, false, "not_deployed".to_string(), None)
    };

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id,
        running,
        status,
        host_port,
        deployment_phase,
        active_deployment_id,
        uptime_seconds,
    }))
}

/// Start an app's container
pub async fn start_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running or stopped deployment with a container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| {
            ApiError::bad_request("No deployment with container found. Deploy the app first.")
        })?;

    // Start the container
    state.runtime.start(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to start container");
        ApiError::internal(format!("Failed to start container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container started");

    // Restore deployment status to 'running' so the container monitor resumes crash detection.
    if let Err(e) = sqlx::query(
        "UPDATE deployments SET status = 'running', finished_at = NULL \
         WHERE app_id = ? AND status = 'stopped' AND container_id = ?",
    )
    .bind(&app.id)
    .bind(&container_id)
    .execute(&state.db)
    .await
    {
        tracing::warn!(
            app = %app.name,
            container = %container_id,
            error = %e,
            "Failed to update deployment status to running after manual start"
        );
    }

    // Get container info for the port (used for both routing and response)
    let host_port = state
        .runtime
        .inspect(&container_id)
        .await
        .ok()
        .and_then(|info| info.host_port);

    // Re-register the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            if let Some(port) = host_port {
                let backend =
                    crate::proxy::Backend::new(container_id.clone(), "127.0.0.1".to_string(), port)
                        .with_healthcheck(app.healthcheck.clone());

                state.routes.load().add_route(domain.clone(), backend);
                tracing::info!(domain = %domain, "Route re-registered after start");
            }
        }
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_START,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: true,
        status: "running".to_string(),
        host_port,
        deployment_phase: "stable".to_string(),
        active_deployment_id: None,
        uptime_seconds: None,
    }))
}

/// Stop an app's container
pub async fn stop_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get the latest running deployment with a container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let container_id = deployment
        .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
        .ok_or_else(|| ApiError::bad_request("No running deployment found"))?;

    // Stop the container
    state.runtime.stop(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to stop container");
        ApiError::internal(format!("Failed to stop container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container stopped");

    // Mark the deployment as stopped so the container monitor does NOT restart it.
    // Without this update the deployment record keeps status = 'running', which causes
    // the monitor to treat the stopped container as a crash and restart it automatically.
    if let Err(e) = sqlx::query(
        "UPDATE deployments SET status = 'stopped', finished_at = datetime('now') \
         WHERE app_id = ? AND status = 'running' AND container_id = ?",
    )
    .bind(&app.id)
    .bind(&container_id)
    .execute(&state.db)
    .await
    {
        tracing::warn!(
            app = %app.name,
            container = %container_id,
            error = %e,
            "Failed to update deployment status to stopped after manual stop"
        );
    }

    // Remove the route if app has a domain
    if let Some(domain) = &app.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
            tracing::info!(domain = %domain, "Route removed after stop");
        }
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_STOP,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(container_id),
        running: false,
        status: "stopped".to_string(),
        host_port: None, // Container is stopped, no port exposed
        deployment_phase: "stable".to_string(),
        active_deployment_id: None,
        uptime_seconds: None,
    }))
}

/// Create a restart deployment record and return its ID.
async fn create_restart_deployment(
    state: &AppState,
    app: &App,
    triggered_by: &str,
) -> Result<String, ApiError> {
    let restart_deployment_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO deployments \
         (id, app_id, commit_message, status, started_at, trigger) \
         VALUES (?, ?, ?, 'pending', ?, 'restart')",
    )
    .bind(&restart_deployment_id)
    .bind(&app.id)
    .bind(format!("Manual restart triggered by {}", triggered_by))
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        ApiError::internal(format!("Failed to create restart deployment record: {}", e))
    })?;

    Ok(restart_deployment_id)
}

/// Add a log line to a deployment record (fire-and-forget — errors are only logged).
async fn log_restart_step(state: &AppState, deployment_id: &str, level: &str, message: &str) {
    if let Err(e) =
        sqlx::query("INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)")
            .bind(deployment_id)
            .bind(level)
            .bind(message)
            .execute(&state.db)
            .await
    {
        tracing::warn!(
            deployment_id = %deployment_id,
            error = %e,
            "Failed to write restart deployment log"
        );
    }
}

/// Finish a restart deployment record with the given status and optional error.
async fn finish_restart_deployment(
    state: &AppState,
    deployment_id: &str,
    status: &str,
    container_id: Option<&str>,
    image_tag: Option<&str>,
    error_message: Option<&str>,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE deployments SET status = ?, container_id = ?, image_tag = ?, \
         error_message = ?, finished_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(container_id)
    .bind(image_tag)
    .bind(error_message)
    .bind(&now)
    .bind(deployment_id)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        tracing::warn!(
            deployment_id = %deployment_id,
            error = %e,
            "Failed to update restart deployment status"
        );
    }
}

/// Restart an app's container with zero downtime using a blue-green swap.
///
/// The algorithm:
/// 1. Find the current running deployment and its image tag.
/// 2. Create a new deployments row with trigger = 'restart'.
/// 3. Start a NEW container from the same image (with fresh env vars).
/// 4. Poll the new container's health endpoint for up to 60 seconds.
/// 5. Atomically update proxy routes to point at the new container.
/// 6. Stop and remove the OLD container (traffic has already been switched).
/// 7. Update the restart deployment record to 'running' with the new container ID.
///
/// If the new container fails to start or does not become healthy within the
/// timeout, the old container is left running so the app remains available.
pub async fn restart_app(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AppStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // 1. Get the latest running deployment (we need its image tag)
    let deployment: Option<Deployment> = sqlx::query_as::<_, Deployment>(
        "SELECT * FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    // Fall back to any deployment with a container if none is currently 'running'
    let deployment = match deployment {
        Some(d) => d,
        None => {
            // No running deployment — fall back to a simple start of the existing container
            let fallback: Option<(String,)> = sqlx::query_as(
                "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1"
            )
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;

            let container_id = fallback
                .and_then(|(cid,)| if cid.is_empty() { None } else { Some(cid) })
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "No deployment with container found. Deploy the app first.",
                    )
                })?;

            // Create a restart deployment record for the fallback path
            let triggered_by = user.email.as_str();
            let restart_dep_id = create_restart_deployment(&state, &app, triggered_by).await?;
            log_restart_step(&state, &restart_dep_id, "info", "Restart triggered (fallback: no running deployment found, restarting existing container)").await;
            log_restart_step(
                &state,
                &restart_dep_id,
                "info",
                &format!(
                    "Stopping container: {}",
                    &container_id[..container_id.len().min(12)]
                ),
            )
            .await;

            // Best-effort restart of the existing container
            let _ = state.runtime.stop(&container_id).await;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            log_restart_step(&state, &restart_dep_id, "info", "Starting container...").await;

            match state.runtime.start(&container_id).await {
                Ok(()) => {}
                Err(e) => {
                    log_restart_step(
                        &state,
                        &restart_dep_id,
                        "error",
                        &format!("Failed to restart container: {}", e),
                    )
                    .await;
                    finish_restart_deployment(
                        &state,
                        &restart_dep_id,
                        "failed",
                        Some(&container_id),
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
                    return Err(ApiError::internal(format!(
                        "Failed to restart container: {}",
                        e
                    )));
                }
            }

            let host_port = state
                .runtime
                .inspect(&container_id)
                .await
                .ok()
                .and_then(|info| info.host_port);

            if let Some(port) = host_port {
                let route_table = state.routes.load();
                for (domain, www_redirect_target) in app.get_all_domains_with_redirects() {
                    let mut backend = crate::proxy::Backend::new(
                        container_id.clone(),
                        "127.0.0.1".to_string(),
                        port,
                    )
                    .with_healthcheck(app.healthcheck.clone());
                    backend.www_redirect_target = www_redirect_target;
                    route_table.add_route(domain, backend);
                }
            }

            log_restart_step(
                &state,
                &restart_dep_id,
                "info",
                "Container restarted successfully",
            )
            .await;
            finish_restart_deployment(
                &state,
                &restart_dep_id,
                "running",
                Some(&container_id),
                None,
                None,
            )
            .await;

            let ip = extract_client_ip(&headers, None);
            audit_log(
                &state,
                actions::APP_RESTART,
                resource_types::APP,
                Some(&app.id),
                Some(&app.name),
                Some(&user.id),
                ip.as_deref(),
                None,
            )
            .await;

            return Ok(Json(AppStatusResponse {
                app_id: id,
                container_id: Some(container_id),
                running: true,
                status: "running".to_string(),
                host_port,
                deployment_phase: "stable".to_string(),
                active_deployment_id: Some(restart_dep_id),
                uptime_seconds: None,
            }));
        }
    };

    let old_container_id = deployment
        .container_id
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ApiError::bad_request("Running deployment has no container. Deploy the app first.")
        })?;

    let image_tag = deployment.image_tag.clone().ok_or_else(|| {
        ApiError::bad_request(
            "Running deployment has no image tag. Cannot perform zero-downtime restart.",
        )
    })?;

    // 2. Create the restart deployment record (tracks this restart in the Deployments tab)
    let triggered_by = user.email.as_str();
    let restart_dep_id = create_restart_deployment(&state, &app, triggered_by).await?;

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        &format!(
            "Zero-downtime restart initiated. Current container: {}, image: {}",
            &old_container_id[..old_container_id.len().min(12)],
            &image_tag
        ),
    )
    .await;

    // Mark the restart deployment as 'starting' (building step is skipped — reusing existing image)
    let _ = sqlx::query("UPDATE deployments SET status = 'starting' WHERE id = ?")
        .bind(&restart_dep_id)
        .execute(&state.db)
        .await;

    // 3. Build RunConfig for the new (blue) container
    let new_container_name = format!(
        "rivetr-{}-restart-{}",
        app.name,
        &uuid::Uuid::new_v4().to_string()[..8]
    );

    let env_vars = collect_env_vars_for_restart(&state, &app).await;

    let volumes = sqlx::query_as::<_, crate::db::Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE app_id = ?",
    )
    .bind(&app.id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let binds: Vec<String> = volumes.iter().map(|v| v.to_bind_mount()).collect();

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
    let cap_drop: Vec<String> = app
        .docker_cap_drop
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let restart_devices: Vec<String> = app
        .devices
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let shm_size: Option<i64> = app
        .shm_size
        .as_ref()
        .and_then(|s| crate::runtime::parse_shm_size(s));
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

    let new_run_config = RunConfig {
        image: image_tag.clone(),
        name: new_container_name.clone(),
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
        devices: restart_devices,
        shm_size,
        init: app.init_process != 0,
        app_id: Some(app.id.clone()),
        gpus: app.docker_gpus.clone(),
        ulimits,
        security_opt,
        cmd: None,
        network: None,
        custom_labels: vec![],
    };

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        &format!("Starting new container from image: {}", &image_tag),
    )
    .await;

    // 4. Start the new container (old one is still running — no downtime yet)
    let new_container_id = match state.runtime.run(&new_run_config).await {
        Ok(cid) => cid,
        Err(e) => {
            tracing::error!(
                app = %app.name,
                error = %e,
                "Zero-downtime restart: failed to start new container"
            );
            log_restart_step(
                &state,
                &restart_dep_id,
                "error",
                &format!("Failed to start new container: {}", e),
            )
            .await;
            finish_restart_deployment(
                &state,
                &restart_dep_id,
                "failed",
                None,
                Some(&image_tag),
                Some(&e.to_string()),
            )
            .await;
            return Err(ApiError::internal(format!(
                "Failed to start new container for restart: {}",
                e
            )));
        }
    };

    tracing::info!(
        app = %app.name,
        new_container = %new_container_id,
        old_container = %old_container_id,
        "Zero-downtime restart: new container started, waiting for it to become healthy"
    );

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        &format!(
            "New container started: {}. Waiting for health check...",
            &new_container_id[..new_container_id.len().min(12)]
        ),
    )
    .await;

    // 5. Inspect the new container to get its host port.
    // Docker assigns the ephemeral port synchronously, but occasionally the
    // inspect response is returned before the port binding is populated in the
    // daemon. Retry up to 10 times (5 seconds) before giving up.
    let new_port = {
        let mut port: Option<u16> = None;
        for attempt in 0..10u8 {
            match state.runtime.inspect(&new_container_id).await {
                Ok(info) => {
                    if let Some(p) = info.host_port {
                        port = Some(p);
                        break;
                    }
                    // Port not yet visible — check if container is still running
                    if !info.running {
                        break; // Container died; stop retrying
                    }
                }
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "inspect failed during port wait");
                }
            }
            if attempt < 9 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        match port {
            Some(p) => p,
            None => {
                // Clean up before returning error; old container still running
                let runtime = state.runtime.clone();
                let cid = new_container_id.clone();
                tokio::spawn(async move {
                    let _ = runtime.stop(&cid).await;
                    let _ = runtime.remove(&cid).await;
                });
                let err_msg = "New container started but did not expose a host port (may have crashed at startup — check the app logs)";
                log_restart_step(&state, &restart_dep_id, "error", err_msg).await;
                finish_restart_deployment(
                    &state,
                    &restart_dep_id,
                    "failed",
                    Some(&new_container_id),
                    Some(&image_tag),
                    Some(err_msg),
                )
                .await;
                return Err(ApiError::internal(err_msg));
            }
        }
    };

    // Update status to 'checking' while we poll the health endpoint
    let _ = sqlx::query("UPDATE deployments SET status = 'checking' WHERE id = ?")
        .bind(&restart_dep_id)
        .execute(&state.db)
        .await;

    // 6. Poll the new container's health endpoint (up to 60 seconds)
    let health_path = app.healthcheck.as_deref().unwrap_or("/");
    let health_url = format!("http://127.0.0.1:{}{}", new_port, health_path);
    let health_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        &format!(
            "Health checking new container at {} (up to 60s)...",
            health_url
        ),
    )
    .await;

    let mut healthy = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);

    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        match health_client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                healthy = true;
                break;
            }
            Ok(resp) => {
                tracing::debug!(
                    app = %app.name,
                    status = %resp.status(),
                    "Zero-downtime restart: health poll returned non-2xx/3xx"
                );
            }
            Err(e) => {
                tracing::debug!(
                    app = %app.name,
                    error = %e,
                    "Zero-downtime restart: health poll connection error"
                );
            }
        }
    }

    if !healthy {
        // New container never became healthy — stop it and leave old one running
        tracing::error!(
            app = %app.name,
            new_container = %new_container_id,
            "Zero-downtime restart: new container did not become healthy within 60s; keeping old container"
        );

        let runtime = state.runtime.clone();
        let cid = new_container_id.clone();
        tokio::spawn(async move {
            let _ = runtime.stop(&cid).await;
            let _ = runtime.remove(&cid).await;
        });

        let err_msg = "New container did not become healthy within 60 seconds. Old container is still running.";
        log_restart_step(&state, &restart_dep_id, "error", err_msg).await;
        finish_restart_deployment(
            &state,
            &restart_dep_id,
            "failed",
            Some(&new_container_id),
            Some(&image_tag),
            Some(err_msg),
        )
        .await;

        return Err(ApiError::internal(err_msg));
    }

    tracing::info!(
        app = %app.name,
        new_container = %new_container_id,
        port = new_port,
        "Zero-downtime restart: new container is healthy, switching proxy routes"
    );

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        &format!(
            "New container is healthy on port {}. Switching proxy routes...",
            new_port
        ),
    )
    .await;

    // 7. Atomically update proxy routes to point at the new container
    {
        let domain_entries = app.get_all_domains_with_redirects();
        let route_table = state.routes.load();
        for (domain, www_redirect_target) in &domain_entries {
            let mut backend = crate::proxy::Backend::new(
                new_container_id.clone(),
                "127.0.0.1".to_string(),
                new_port,
            )
            .with_healthcheck(app.healthcheck.clone());
            backend.www_redirect_target = www_redirect_target.clone();
            route_table.add_route(domain.clone(), backend);
            tracing::info!(
                domain = %domain,
                port = new_port,
                "Zero-downtime restart: route switched to new container"
            );
        }
    }

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        "Proxy routes updated. Stopping old container...",
    )
    .await;

    // 8. Stop and remove the OLD container (traffic already switched), then rename
    //    the NEW container to the canonical `rivetr-<app-name>` so `docker ps`
    //    reports a stable name across restarts.  We can't rename earlier because
    //    Docker forbids two containers sharing a name on the same daemon.
    let canonical_name = app
        .custom_container_name
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("rivetr-{}", app.name));
    {
        let app_name_log = app.name.clone();
        let old_cid = old_container_id.clone();
        let new_cid = new_container_id.clone();
        let runtime = state.runtime.clone();
        let canonical = canonical_name.clone();
        tokio::spawn(async move {
            if let Err(e) = runtime.stop(&old_cid).await {
                tracing::warn!(
                    app = %app_name_log,
                    container = %old_cid,
                    error = %e,
                    "Zero-downtime restart: failed to stop old container (it may linger until next Rivetr restart)"
                );
            }
            if let Err(e) = runtime.remove(&old_cid).await {
                tracing::warn!(
                    app = %app_name_log,
                    container = %old_cid,
                    error = %e,
                    "Zero-downtime restart: failed to remove old container"
                );
            }
            // Now that the old container is gone, rename the new one to the canonical
            // name.  Best-effort — if Docker rejects the rename for any reason the
            // app keeps working under its `rivetr-<app>-restart-<hash>` name, just
            // less prettily.
            if let Err(e) = runtime.rename_container(&new_cid, &canonical).await {
                tracing::warn!(
                    app = %app_name_log,
                    container = %new_cid,
                    target = %canonical,
                    error = %e,
                    "Zero-downtime restart: failed to rename new container to canonical name"
                );
            } else {
                tracing::info!(
                    app = %app_name_log,
                    container = %new_cid,
                    new_name = %canonical,
                    "Zero-downtime restart: renamed new container to canonical name"
                );
            }
        });
    }

    tracing::info!(
        app = %app.name,
        old_container = %old_container_id,
        "Zero-downtime restart: old container teardown initiated"
    );

    // 9. Mark the previous 'running' deployment as 'replaced' now that the restart is live.
    let _ = sqlx::query(
        "UPDATE deployments SET status = 'replaced', finished_at = ? \
         WHERE app_id = ? AND status = 'running' AND id != ?",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&app.id)
    .bind(&restart_dep_id)
    .execute(&state.db)
    .await;

    log_restart_step(
        &state,
        &restart_dep_id,
        "info",
        "Restart complete — container is running.",
    )
    .await;

    // 10. Finalize the restart deployment record as 'running'.
    finish_restart_deployment(
        &state,
        &restart_dep_id,
        "running",
        Some(&new_container_id),
        Some(&image_tag),
        None,
    )
    .await;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::APP_RESTART,
        resource_types::APP,
        Some(&app.id),
        Some(&app.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(AppStatusResponse {
        app_id: id,
        container_id: Some(new_container_id),
        running: true,
        status: "running".to_string(),
        host_port: Some(new_port),
        deployment_phase: "stable".to_string(),
        active_deployment_id: Some(restart_dep_id),
        uptime_seconds: None,
    }))
}

/// Get recent activity (audit log events) for a specific app.
/// Returns up to 50 most recent audit log entries where resource_id = app id.
pub async fn get_app_activity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AuditLogListResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify the app exists
    let _app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let query = AuditLogQuery {
        resource_id: Some(id),
        per_page: Some(50),
        ..Default::default()
    };

    let result = list_audit_logs(&state.db, &query).await?;
    Ok(Json(result))
}

/// Apply resource limits (CPU/memory) to a running container live, without restarting.
/// Uses `docker update` to apply cgroup-level changes immediately.
/// The app's `cpu_limit` and `memory_limit` fields must be set first via PUT /apps/:id.
pub async fn apply_resource_limits(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Get container_id from the latest running deployment
    let container_id: Option<String> = sqlx::query_scalar(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status IN ('running', 'stopped') AND container_id IS NOT NULL ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let container_id =
        container_id.ok_or_else(|| ApiError::bad_request("App has no running container"))?;

    state
        .runtime
        .apply_resource_limits(
            &container_id,
            app.memory_limit.as_deref(),
            app.cpu_limit.as_deref(),
        )
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Resource limits applied to running container",
        "memory_limit": app.memory_limit,
        "cpu_limit": app.cpu_limit,
        "container_id": container_id
    })))
}

/// Generate a random domain for an app.
/// Returns a subdomain based on the server's wildcard domain, or a sslip.io/traefik.me domain.
/// POST /api/apps/:id/generate-domain
pub async fn generate_domain(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify app exists
    sqlx::query_scalar::<_, String>("SELECT id FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Generate a random 8-char lowercase alphanumeric string
    use rand::Rng;
    let mut rng = rand::rng();
    let random_prefix: String = (0..8)
        .map(|_| {
            let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
            chars[rng.random_range(0..chars.len())] as char
        })
        .collect();

    // Try each domain generation strategy in order of preference
    let domain = if let Some(ref base_domain) = state.config.proxy.base_domain {
        // Strategy 1: Use the configured base domain (e.g., "rivetr.example.com")
        format!("{}.{}", random_prefix, base_domain)
    } else if state.config.proxy.sslip_enabled {
        // Strategy 2: sslip.io domain
        if let Some(ref ip) = state.config.proxy.server_ip {
            format!("{}.{}.sslip.io", random_prefix, ip)
        } else {
            return Err(ApiError::bad_request(
                "No domain generation strategy configured. Set base_domain or server_ip in proxy config.",
            ));
        }
    } else if let Some(ref ip) = state.config.proxy.server_ip {
        // Strategy 3: traefik.me style domain
        let ip_dashes = ip.replace('.', "-");
        format!("{}-{}.traefik.me", random_prefix, ip_dashes)
    } else {
        return Err(ApiError::bad_request(
            "No domain generation strategy configured. Set base_domain or server_ip in proxy config.",
        ));
    };

    Ok(Json(serde_json::json!({ "domain": domain })))
}
