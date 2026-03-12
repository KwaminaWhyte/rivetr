use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::db::{actions, list_audit_logs, resource_types, App, AuditLogListResponse, AuditLogQuery, User};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::AppStatusResponse;

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

    // Get the latest running deployment's container
    let deployment: Option<(String,)> = sqlx::query_as(
        "SELECT container_id FROM deployments WHERE app_id = ? AND status = 'running' ORDER BY started_at DESC LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    let (container_id, running, status, host_port) = if let Some((cid,)) = deployment {
        if cid.is_empty() {
            (None, false, "no_container".to_string(), None)
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

/// Restart an app's container.
/// This stops and starts the container, which picks up new environment variables
/// and other configuration changes without requiring a full rebuild.
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

    // First stop the container (ignore errors if already stopped)
    let _ = state.runtime.stop(&container_id).await;

    // Brief pause to ensure clean stop
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Start the container
    state.runtime.start(&container_id).await.map_err(|e| {
        tracing::error!(container = %container_id, error = %e, "Failed to restart container");
        ApiError::internal(format!("Failed to restart container: {}", e))
    })?;

    tracing::info!(app = %app.name, container = %container_id, "App container restarted");

    // Get container info for the port (used for both routing and response)
    let host_port = state
        .runtime
        .inspect(&container_id)
        .await
        .ok()
        .and_then(|info| info.host_port);

    // Re-register routes for all domains (custom + auto_subdomain) with the new port
    if let Some(port) = host_port {
        let all_domains = app.get_all_domain_names();
        let route_table = state.routes.load();
        for domain in &all_domains {
            let backend =
                crate::proxy::Backend::new(container_id.clone(), "127.0.0.1".to_string(), port)
                    .with_healthcheck(app.healthcheck.clone());
            route_table.add_route(domain.clone(), backend);
            tracing::info!(domain = %domain, port = port, "Route re-registered after restart");
        }
    }

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
        container_id: Some(container_id),
        running: true,
        status: "running".to_string(),
        host_port,
        deployment_phase: "stable".to_string(),
        active_deployment_id: None,
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
