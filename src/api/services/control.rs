//! Start, stop, and log streaming handlers for Docker Compose services.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::db::{
    actions, resource_types, Service, ServiceGeneratedVar, ServiceResponse, ServiceStatus, User,
};
use crate::AppState;

use super::super::audit::{audit_log, ClientIp};
use super::compose::{
    get_compose_dir, get_service_compose_dir, inject_public_ports, run_compose_command,
    run_compose_command_streaming, substitute_magic_vars, write_compose_file_with_options,
};

/// Service log entry
#[derive(serde::Serialize)]
pub struct ServiceLogEntry {
    pub timestamp: String,
    pub service: String,
    pub message: String,
}

/// Query parameters for logs endpoint
#[derive(Deserialize, Default)]
pub struct LogsQuery {
    /// Number of lines to return (default: 100)
    #[serde(default = "default_lines")]
    pub lines: u32,
}

fn default_lines() -> u32 {
    100
}

/// Start a Docker Compose service
pub async fn start_service(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<Json<ServiceResponse>, StatusCode> {
    // Reset and seed the live start-log stream so the dashboard side panel
    // shows progress for this start cycle.
    let resource_key = format!("service:{}", id);
    state.start_log_streams.clear(&resource_key);
    state
        .start_log_streams
        .info(&resource_key, "info", "Starting service…");

    // Get the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            state
                .start_log_streams
                .error(&resource_key, "failed", format!("DB error: {}", e));
            state
                .start_log_streams
                .end(&resource_key, "failed", "Start aborted");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            state
                .start_log_streams
                .error(&resource_key, "failed", "Service not found");
            state
                .start_log_streams
                .end(&resource_key, "failed", "Start aborted");
            StatusCode::NOT_FOUND
        })?;

    let data_dir = &state.config.server.data_dir;
    let compose_dir = get_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Fetch generated vars for this service to pass as existing_vars context
    let gen_vars_rows: Vec<crate::db::ServiceGeneratedVar> =
        sqlx::query_as("SELECT * FROM service_generated_vars WHERE service_id = ?")
            .bind(&service.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    let existing_vars: std::collections::HashMap<String, String> = gen_vars_rows
        .into_iter()
        .map(|r| (r.key, r.value))
        .collect();

    // Substitute magic variables (SERVICE_PASSWORD_*, SERVICE_BASE64_*, required vars, FQDN/URL)
    let substituted_compose = match substitute_magic_vars(
        &service.compose_content,
        &service.id,
        service.domain.as_deref(),
        &existing_vars,
        &state.db,
        false,
    )
    .await
    {
        Ok(content) => content,
        Err(e) => {
            tracing::error!(
                "Failed to substitute magic vars in compose for service {}: {}",
                service.name,
                e
            );
            // Update status to failed with error message
            let _ = sqlx::query(
                "UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(crate::db::ServiceStatus::Failed.to_string())
            .bind(&e)
            .bind(&id)
            .execute(&state.db)
            .await;
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // Always write/overwrite compose file from database content (in case it was updated).
    // Inject isolated network if enabled (default: yes), unless raw mode is active.
    let raw_mode = service.raw_compose_mode != 0;
    let isolated_id = if !raw_mode && service.isolated_network != 0 {
        Some(service.id.as_str())
    } else {
        None
    };

    // If public_access is enabled (and not in raw mode), inject the host port binding before
    // writing the compose file to disk so Docker picks it up on the next `up`.
    let compose_to_write = if !raw_mode
        && service.public_access != 0
        && service.external_port != 0
        && service.expose_container_port != 0
    {
        match inject_public_ports(
            &substituted_compose,
            service.external_port,
            service.expose_container_port,
        ) {
            Ok(injected) => injected,
            Err(e) => {
                tracing::warn!(
                    "Failed to inject public ports for service {}: {}. Using compose without port injection.",
                    service.name,
                    e
                );
                substituted_compose
            }
        }
    } else {
        substituted_compose
    };

    if let Err(e) = write_compose_file_with_options(
        data_dir,
        &service.name,
        &compose_to_write,
        isolated_id,
        raw_mode,
    )
    .await
    {
        tracing::error!("Failed to write compose file: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clean up any orphaned containers from previous failed deployments
    // This prevents "container name already in use" errors
    tracing::debug!(
        "Cleaning up orphaned containers for project: {}",
        project_name
    );
    if let Err(e) =
        run_compose_command(&compose_dir, &project_name, &["down", "--remove-orphans"]).await
    {
        // Log but don't fail - the containers might not exist yet
        tracing::debug!("Compose down (cleanup) result: {}", e);
    }

    // Run docker compose up -d, streaming each line of output to the live
    // start-log channel so the deploy side panel can show pull/start progress.
    let stream_handle = state.start_log_streams.clone();
    let stream_key_for_up = resource_key.clone();
    state
        .start_log_streams
        .info(&resource_key, "pulling", "Running docker compose up -d");
    let up_result = run_compose_command_streaming(
        &compose_dir,
        &project_name,
        &["up", "-d"],
        |line, is_stderr| {
            // Heuristic: docker streams pull progress on stderr, but it's not
            // really an error — show it as info with a "pulling" phase.
            let phase = classify_compose_line(line);
            stream_handle.emit(
                &stream_key_for_up,
                if is_stderr && line.to_lowercase().contains("error") {
                    "error"
                } else {
                    "info"
                },
                phase,
                line,
            );
        },
    )
    .await;

    match up_result {
        Ok(_) => {
            // Update status to running
            sqlx::query("UPDATE services SET status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Running.to_string())
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Register proxy route if domain is configured
            if let Some(ref domain) = service.domain {
                if !domain.is_empty() {
                    let backend = crate::proxy::Backend::new(
                        format!("rivetr-svc-{}", service.name),
                        "127.0.0.1".to_string(),
                        service.port as u16,
                    );
                    state.routes.load().add_route(domain.clone(), backend);
                    tracing::info!(
                        "Registered proxy route: {} -> port {}",
                        domain,
                        service.port
                    );
                }
            }

            tracing::info!("Started Docker Compose service: {}", service.name);
            state.start_log_streams.info(
                &resource_key,
                "running",
                format!("Service {} is running", service.name),
            );
            state
                .start_log_streams
                .end(&resource_key, "running", "Service started successfully");
        }
        Err(e) => {
            // Update status to failed with error message
            sqlx::query("UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Failed.to_string())
                .bind(&e)
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            tracing::error!(
                "Failed to start Docker Compose service {}: {}",
                service.name,
                e
            );
            state.start_log_streams.error(
                &resource_key,
                "failed",
                format!("docker compose up failed: {}", e),
            );
            state
                .start_log_streams
                .end(&resource_key, "failed", "Start failed");
        }
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    audit_log(
        &state,
        actions::SERVICE_START,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(service.into()))
}

/// Stop a Docker Compose service
pub async fn stop_service(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<Json<ServiceResponse>, StatusCode> {
    // Get the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let data_dir = &state.config.server.data_dir;
    // Use get_service_compose_dir to check both data dir and temp dir
    // (template-deployed services write their compose file to the temp dir)
    let compose_dir = get_service_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Run docker compose down
    match run_compose_command(&compose_dir, &project_name, &["down"]).await {
        Ok(_) => {
            // Update status to stopped
            sqlx::query("UPDATE services SET status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Stopped.to_string())
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Remove proxy route if domain was configured
            if let Some(ref domain) = service.domain {
                if !domain.is_empty() {
                    state.routes.load().remove_route(domain);
                    tracing::info!("Removed proxy route for stopped service: {}", domain);
                }
            }

            tracing::info!("Stopped Docker Compose service: {}", service.name);
        }
        Err(e) => {
            // Update status to failed with error message
            sqlx::query("UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Failed.to_string())
                .bind(&e)
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            tracing::error!(
                "Failed to stop Docker Compose service {}: {}",
                service.name,
                e
            );
        }
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    audit_log(
        &state,
        actions::SERVICE_STOP,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(service.into()))
}

/// Restart a Docker Compose service (stop then start)
pub async fn restart_service(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<Json<ServiceResponse>, StatusCode> {
    let resource_key = format!("service:{}", id);
    state.start_log_streams.clear(&resource_key);
    state
        .start_log_streams
        .info(&resource_key, "info", "Restarting service…");

    // Get the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            state
                .start_log_streams
                .error(&resource_key, "failed", format!("DB error: {}", e));
            state
                .start_log_streams
                .end(&resource_key, "failed", "Restart aborted");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            state
                .start_log_streams
                .error(&resource_key, "failed", "Service not found");
            state
                .start_log_streams
                .end(&resource_key, "failed", "Restart aborted");
            StatusCode::NOT_FOUND
        })?;

    let data_dir = &state.config.server.data_dir;
    let compose_dir = get_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Fetch generated vars for this service to pass as existing_vars context
    let gen_vars_rows: Vec<crate::db::ServiceGeneratedVar> =
        sqlx::query_as("SELECT * FROM service_generated_vars WHERE service_id = ?")
            .bind(&service.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    let existing_vars: std::collections::HashMap<String, String> = gen_vars_rows
        .into_iter()
        .map(|r| (r.key, r.value))
        .collect();

    // Substitute magic variables
    let substituted_compose = match substitute_magic_vars(
        &service.compose_content,
        &service.id,
        service.domain.as_deref(),
        &existing_vars,
        &state.db,
        false,
    )
    .await
    {
        Ok(content) => content,
        Err(e) => {
            tracing::error!(
                "Failed to substitute magic vars in compose for service {}: {}",
                service.name,
                e
            );
            let _ = sqlx::query(
                "UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(crate::db::ServiceStatus::Failed.to_string())
            .bind(&e)
            .bind(&id)
            .execute(&state.db)
            .await;
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // Always write/overwrite compose file from database content.
    // Inject isolated network if enabled (default: yes), unless raw mode is active.
    let raw_mode = service.raw_compose_mode != 0;
    let isolated_id = if !raw_mode && service.isolated_network != 0 {
        Some(service.id.as_str())
    } else {
        None
    };

    // Inject public port binding if enabled (and not raw mode)
    let compose_to_write = if !raw_mode
        && service.public_access != 0
        && service.external_port != 0
        && service.expose_container_port != 0
    {
        match inject_public_ports(
            &substituted_compose,
            service.external_port,
            service.expose_container_port,
        ) {
            Ok(injected) => injected,
            Err(e) => {
                tracing::warn!(
                    "Failed to inject public ports for service {}: {}. Using compose without port injection.",
                    service.name,
                    e
                );
                substituted_compose
            }
        }
    } else {
        substituted_compose
    };

    if let Err(e) = write_compose_file_with_options(
        data_dir,
        &service.name,
        &compose_to_write,
        isolated_id,
        raw_mode,
    )
    .await
    {
        tracing::error!("Failed to write compose file: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Remove proxy route before restarting
    if let Some(ref domain) = service.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
        }
    }

    // Run docker compose restart, streaming each line to the live panel.
    let stream_handle = state.start_log_streams.clone();
    let stream_key_for_restart = resource_key.clone();
    state
        .start_log_streams
        .info(&resource_key, "starting", "Running docker compose restart");
    let restart_result = run_compose_command_streaming(
        &compose_dir,
        &project_name,
        &["restart"],
        |line, is_stderr| {
            let phase = classify_compose_line(line);
            stream_handle.emit(
                &stream_key_for_restart,
                if is_stderr && line.to_lowercase().contains("error") {
                    "error"
                } else {
                    "info"
                },
                phase,
                line,
            );
        },
    )
    .await;

    match restart_result {
        Ok(_) => {
            // Update status to running
            sqlx::query("UPDATE services SET status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Running.to_string())
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Re-register proxy route if domain is configured
            if let Some(ref domain) = service.domain {
                if !domain.is_empty() {
                    let backend = crate::proxy::Backend::new(
                        format!("rivetr-svc-{}", service.name),
                        "127.0.0.1".to_string(),
                        service.port as u16,
                    );
                    state.routes.load().add_route(domain.clone(), backend);
                    tracing::info!(
                        "Re-registered proxy route after restart: {} -> port {}",
                        domain,
                        service.port
                    );
                }
            }

            tracing::info!("Restarted Docker Compose service: {}", service.name);
            state
                .start_log_streams
                .end(&resource_key, "running", "Service restarted successfully");
        }
        Err(e) => {
            // Update status to failed with error message
            sqlx::query("UPDATE services SET status = ?, error_message = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(ServiceStatus::Failed.to_string())
                .bind(&e)
                .bind(&id)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update service status: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            tracing::error!(
                "Failed to restart Docker Compose service {}: {}",
                service.name,
                e
            );
            state.start_log_streams.error(
                &resource_key,
                "failed",
                format!("docker compose restart failed: {}", e),
            );
            state
                .start_log_streams
                .end(&resource_key, "failed", "Restart failed");
        }
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    audit_log(
        &state,
        actions::SERVICE_START,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(service.into()))
}

/// Internal helper: perform a full stop → start cycle for a running service.
///
/// Used by the update handler when `public_access` changes on a running service
/// so that Docker picks up the new port binding without requiring manual restart.
pub async fn restart_service_internal(state: &Arc<AppState>, id: &str) -> Result<(), String> {
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Service {} not found", id))?;

    let data_dir = &state.config.server.data_dir;
    let compose_dir = get_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Fetch generated vars
    let gen_vars_rows: Vec<ServiceGeneratedVar> =
        sqlx::query_as("SELECT * FROM service_generated_vars WHERE service_id = ?")
            .bind(&service.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    let existing_vars: std::collections::HashMap<String, String> = gen_vars_rows
        .into_iter()
        .map(|r| (r.key, r.value))
        .collect();

    // Substitute magic vars
    let substituted = substitute_magic_vars(
        &service.compose_content,
        &service.id,
        service.domain.as_deref(),
        &existing_vars,
        &state.db,
        false,
    )
    .await
    .map_err(|e| format!("Magic var substitution failed: {}", e))?;

    let raw_mode = service.raw_compose_mode != 0;
    let isolated_id = if !raw_mode && service.isolated_network != 0 {
        Some(service.id.as_str())
    } else {
        None
    };

    // Inject public ports if needed
    let compose_to_write = if !raw_mode
        && service.public_access != 0
        && service.external_port != 0
        && service.expose_container_port != 0
    {
        match inject_public_ports(
            &substituted,
            service.external_port,
            service.expose_container_port,
        ) {
            Ok(injected) => injected,
            Err(e) => {
                tracing::warn!(
                    "Failed to inject public ports for service {} during internal restart: {}",
                    service.name,
                    e
                );
                substituted
            }
        }
    } else {
        substituted
    };

    write_compose_file_with_options(
        data_dir,
        &service.name,
        &compose_to_write,
        isolated_id,
        raw_mode,
    )
    .await
    .map_err(|e| format!("Failed to write compose file: {}", e))?;

    // Stop (ignore errors — containers may already be down)
    let _ = run_compose_command(&compose_dir, &project_name, &["down", "--remove-orphans"]).await;

    // Start
    run_compose_command(&compose_dir, &project_name, &["up", "-d"])
        .await
        .map_err(|e| format!("docker compose up failed: {}", e))?;

    sqlx::query(
        "UPDATE services SET status = ?, error_message = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(ServiceStatus::Running.to_string())
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Failed to update service status: {}", e))?;

    // Re-register proxy route
    if let Some(ref domain) = service.domain {
        if !domain.is_empty() {
            let backend = crate::proxy::Backend::new(
                format!("rivetr-svc-{}", service.name),
                "127.0.0.1".to_string(),
                service.port as u16,
            );
            state.routes.load().add_route(domain.clone(), backend);
        }
    }

    tracing::info!(
        "restart_service_internal: service '{}' restarted successfully",
        service.name
    );
    Ok(())
}

/// Parse docker compose logs output into structured entries
fn parse_compose_logs(output: &str) -> Vec<ServiceLogEntry> {
    output
        .lines()
        .filter_map(|line| {
            // Docker compose logs format: "service-name  | 2024-01-01T12:00:00.000Z message"
            // Or with timestamps: "service-name  | 2024-01-01T12:00:00.000000000Z message"
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // Try to split by " | " which separates service name from log content
            if let Some(pipe_pos) = line.find(" | ") {
                let service_name = line[..pipe_pos].trim().to_string();
                let rest = &line[pipe_pos + 3..];

                // Try to extract timestamp from the rest
                // Format: "2024-01-01T12:00:00.000Z message" or "2024-01-01T12:00:00.000000000Z message"
                if rest.len() > 20 && rest.chars().nth(4) == Some('-') {
                    // Find where the timestamp ends (after the Z or timezone)
                    if let Some(space_after_ts) = rest.find(' ') {
                        let timestamp = rest[..space_after_ts].to_string();
                        let message = rest[space_after_ts + 1..].to_string();
                        return Some(ServiceLogEntry {
                            timestamp,
                            service: service_name,
                            message,
                        });
                    }
                }

                // No timestamp, use current time
                Some(ServiceLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    service: service_name,
                    message: rest.to_string(),
                })
            } else {
                // Fallback: no pipe separator, just use the whole line as message
                Some(ServiceLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    service: "unknown".to_string(),
                    message: line.to_string(),
                })
            }
        })
        .collect()
}

/// Parse a single log line into a ServiceLogEntry
fn parse_log_line(line: &str) -> Option<ServiceLogEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Parse the log line
    if let Some(pipe_pos) = line.find(" | ") {
        let service_name = line[..pipe_pos].trim().to_string();
        let rest = &line[pipe_pos + 3..];

        // Try to extract timestamp
        if rest.len() > 20 && rest.chars().nth(4) == Some('-') {
            if let Some(space_after_ts) = rest.find(' ') {
                let timestamp = rest[..space_after_ts].to_string();
                let message = rest[space_after_ts + 1..].to_string();
                return Some(ServiceLogEntry {
                    timestamp,
                    service: service_name,
                    message,
                });
            }
        }

        Some(ServiceLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            service: service_name,
            message: rest.to_string(),
        })
    } else {
        Some(ServiceLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            service: "unknown".to_string(),
            message: line.to_string(),
        })
    }
}

/// Get recent logs for a Docker Compose service
/// GET /api/services/:id/logs
pub async fn get_service_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Vec<ServiceLogEntry>>, (StatusCode, Json<serde_json::Value>)> {
    // Get the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get service"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Service not found"})),
            )
        })?;

    // Check if the service is running
    let status = service.get_status();
    if status != ServiceStatus::Running {
        tracing::info!(
            "Service {} is not running (status: {})",
            service.name,
            service.status
        );
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Container is stopped",
                "message": "The service is not running. Start the service to view logs.",
                "status": service.status
            })),
        ));
    }

    let data_dir = &state.config.server.data_dir;
    let compose_dir = get_service_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Get logs using docker compose logs command
    let lines = query.lines.min(1000); // Cap at 1000 lines

    // Build command - if compose_dir exists, use it; otherwise run without current_dir
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("-p")
        .arg(&project_name)
        .arg("logs")
        .arg("--tail")
        .arg(lines.to_string())
        .arg("--timestamps");

    if compose_dir.exists() {
        cmd.current_dir(&compose_dir);
    }

    let output = cmd.output().await;

    let logs = match output {
        Ok(output) => {
            if output.status.success() {
                parse_compose_logs(&String::from_utf8_lossy(&output.stdout))
            } else {
                // Try legacy docker-compose command
                let mut legacy_cmd = Command::new("docker-compose");
                legacy_cmd
                    .arg("-p")
                    .arg(&project_name)
                    .arg("logs")
                    .arg("--tail")
                    .arg(lines.to_string())
                    .arg("--timestamps");

                if compose_dir.exists() {
                    legacy_cmd.current_dir(&compose_dir);
                }

                let legacy_output = legacy_cmd.output().await;

                match legacy_output {
                    Ok(output) if output.status.success() => {
                        parse_compose_logs(&String::from_utf8_lossy(&output.stdout))
                    }
                    _ => {
                        tracing::warn!("Failed to get compose logs for service {}", service.name);
                        Vec::new()
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to execute docker compose logs: {}", e);
            Vec::new()
        }
    };

    Ok(Json(logs))
}

/// Stream logs from a Docker Compose service using SSE
/// GET /api/services/:id/logs/stream
pub async fn stream_service_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)>
{
    // Get the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get service"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Service not found"})),
            )
        })?;

    // Check if the service is running
    let status = service.get_status();
    if status != ServiceStatus::Running {
        tracing::info!(
            "Service {} is not running (status: {}), cannot stream logs",
            service.name,
            service.status
        );
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Container is stopped",
                "message": "The service is not running. Start the service to view logs.",
                "status": service.status
            })),
        ));
    }

    let data_dir = state.config.server.data_dir.clone();
    let compose_dir = get_service_compose_dir(&data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Start docker compose logs with --follow
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("-p")
        .arg(&project_name)
        .arg("logs")
        .arg("--follow")
        .arg("--timestamps")
        .arg("--tail")
        .arg("50") // Start with last 50 lines
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if compose_dir.exists() {
        cmd.current_dir(&compose_dir);
    }

    let mut child = cmd.spawn().map_err(|e| {
        tracing::error!("Failed to spawn docker compose logs: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to start log stream: {}", e)})),
        )
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        tracing::error!("Failed to get stdout from docker compose logs");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to get log output stream"})),
        )
    })?;

    let reader = BufReader::new(stdout);

    // Create the SSE stream using async_stream
    let stream = async_stream::stream! {
        let mut lines = reader.lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if let Some(entry) = parse_log_line(&line) {
                        if let Ok(json) = serde_json::to_string(&entry) {
                            yield Ok(Event::default().data(json));
                        }
                    }
                }
                Ok(None) => {
                    // Stream ended
                    yield Ok(Event::default().data(r#"{"type":"end","message":"Log stream ended"}"#));
                    break;
                }
                Err(e) => {
                    tracing::warn!("Error reading log line: {}", e);
                    yield Ok(Event::default().data(format!(r#"{{"type":"error","message":"{}"}}"#, e)));
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

/// List auto-generated magic variables for a service (SERVICE_PASSWORD_*, SERVICE_BASE64_*, etc.)
///
/// GET /api/services/:id/generated-vars
///
/// Returns all persisted generated variables. Values are shown in plain text
/// (they are not secret per se — compose templates embed them as env vars).
pub async fn get_service_generated_vars(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<crate::db::ServiceGeneratedVarResponse>>, (StatusCode, Json<serde_json::Value>)>
{
    // Verify service exists
    let service_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;

    if service_exists == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Service not found"})),
        ));
    }

    let rows: Vec<ServiceGeneratedVar> = sqlx::query_as(
        "SELECT * FROM service_generated_vars WHERE service_id = ? ORDER BY key ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch generated vars: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to fetch generated vars"})),
        )
    })?;

    let response: Vec<crate::db::ServiceGeneratedVarResponse> =
        rows.into_iter().map(Into::into).collect();
    Ok(Json(response))
}

/// Get aggregated container resource stats for a running Docker Compose service.
///
/// GET /api/services/:id/stats
///
/// Iterates all containers in the compose project and sums CPU, memory, and network
/// stats. Returns zeroed stats (rather than a 404) when the service is stopped so
/// that the dashboard can poll without getting console errors.
pub async fn get_service_stats(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<crate::runtime::ContainerStats>, (StatusCode, Json<serde_json::Value>)> {
    // Fetch the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get service"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Service not found"})),
            )
        })?;

    // Return zeroed stats for non-running services
    if service.get_status() != ServiceStatus::Running {
        return Ok(Json(crate::runtime::ContainerStats {
            cpu_percent: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            network_rx: 0,
            network_tx: 0,
        }));
    }

    let project_name = service.compose_project_name();

    // List all containers for this compose project
    let containers = state
        .runtime
        .list_compose_containers(&project_name)
        .await
        .map_err(|e| {
            tracing::warn!(
                "Failed to list compose containers for service {}: {}",
                service.name,
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to list containers: {}", e)})),
            )
        })?;

    if containers.is_empty() {
        return Ok(Json(crate::runtime::ContainerStats {
            cpu_percent: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            network_rx: 0,
            network_tx: 0,
        }));
    }

    // Aggregate stats across all containers
    let mut total_cpu: f64 = 0.0;
    let mut total_memory_usage: u64 = 0;
    let mut total_memory_limit: u64 = 0;
    let mut total_network_rx: u64 = 0;
    let mut total_network_tx: u64 = 0;

    for container in &containers {
        match state.runtime.stats(&container.id).await {
            Ok(stats) => {
                total_cpu += stats.cpu_percent;
                total_memory_usage += stats.memory_usage;
                total_memory_limit += stats.memory_limit;
                total_network_rx += stats.network_rx;
                total_network_tx += stats.network_tx;
            }
            Err(e) => {
                tracing::debug!(
                    "Could not get stats for container {} in service {}: {}",
                    container.name,
                    service.name,
                    e
                );
            }
        }
    }

    Ok(Json(crate::runtime::ContainerStats {
        cpu_percent: total_cpu,
        memory_usage: total_memory_usage,
        memory_limit: total_memory_limit,
        network_rx: total_network_rx,
        network_tx: total_network_tx,
    }))
}

/// Preview the final resolved compose YAML for a service without deploying.
///
/// GET /api/services/:id/preview-compose
///
/// Applies magic variable substitution (dry-run — nothing is written to DB)
/// and injects the rivetr network, returning the rendered YAML.
pub async fn preview_compose(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Fetch the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get service"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Service not found"})),
            )
        })?;

    // Load existing generated vars so preview shows stable values for already-generated vars
    let gen_vars_rows: Vec<ServiceGeneratedVar> =
        sqlx::query_as("SELECT * FROM service_generated_vars WHERE service_id = ?")
            .bind(&service.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
    let existing_vars: std::collections::HashMap<String, String> = gen_vars_rows
        .into_iter()
        .map(|r| (r.key, r.value))
        .collect();

    // Dry-run substitution — generates placeholder values but does NOT persist to DB
    let substituted = substitute_magic_vars(
        &service.compose_content,
        &service.id,
        service.domain.as_deref(),
        &existing_vars,
        &state.db,
        true,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"error": e})),
        )
    })?;

    // Inject the rivetr network (same as the real deploy path)
    use super::compose::inject_rivetr_network;
    let final_yaml = inject_rivetr_network(&substituted).unwrap_or(substituted);

    Ok(Json(serde_json::json!({ "compose_yaml": final_yaml })))
}

/// Classify a `docker compose up` output line into a coarse phase so the
/// dashboard can render an appropriate status badge.
fn classify_compose_line(line: &str) -> &'static str {
    let lower = line.to_lowercase();
    if lower.contains("pull") || lower.contains("download") || lower.contains("extract") {
        "pulling"
    } else if lower.contains("creating") || lower.contains("created") || lower.contains("starting")
    {
        "starting"
    } else if lower.contains("started") || lower.contains("running") || lower.contains("healthy") {
        "running"
    } else {
        "info"
    }
}
