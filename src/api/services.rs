//! API handlers for Docker Compose services

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, CreateServiceRequest, Service, ServiceResponse, ServiceStatus,
    UpdateServiceRequest, User,
};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};

/// Validate docker-compose content
/// Checks that it's valid YAML with a 'services' key
fn validate_compose_content(content: &str) -> Result<(), String> {
    // Check it's not completely empty/whitespace
    if content.trim().is_empty() {
        return Err("Compose file is empty".to_string());
    }

    // Parse as YAML
    let yaml: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| format!("Invalid YAML: {}", e))?;

    // Check for 'services' key
    let mapping = yaml.as_mapping()
        .ok_or_else(|| "Compose file must be a YAML mapping".to_string())?;

    // Check for 'services' key (required in docker-compose)
    if !mapping.contains_key(&serde_yaml::Value::String("services".to_string())) {
        return Err("Compose file must contain a 'services' key".to_string());
    }

    Ok(())
}

/// Get the compose file path for a service
fn get_compose_dir(data_dir: &PathBuf, service_name: &str) -> PathBuf {
    data_dir.join("services").join(service_name)
}

/// Namespace container names in compose content to prevent global conflicts
/// Prefixes all container_name values with "rivetr-{service_name}-"
fn namespace_container_names(content: &str, service_name: &str) -> Result<String, String> {
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| format!("Invalid YAML: {}", e))?;

    let prefix = format!("rivetr-{}-", service_name);

    if let Some(mapping) = yaml.as_mapping_mut() {
        if let Some(services) = mapping.get_mut(&serde_yaml::Value::String("services".to_string())) {
            if let Some(services_map) = services.as_mapping_mut() {
                for (_service_key, service_config) in services_map.iter_mut() {
                    if let Some(config_map) = service_config.as_mapping_mut() {
                        let container_name_key = serde_yaml::Value::String("container_name".to_string());
                        if let Some(container_name_val) = config_map.get_mut(&container_name_key) {
                            if let Some(name) = container_name_val.as_str() {
                                // Only add prefix if not already prefixed
                                if !name.starts_with(&prefix) && !name.starts_with("rivetr-") {
                                    *container_name_val = serde_yaml::Value::String(format!("{}{}", prefix, name));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    serde_yaml::to_string(&yaml)
        .map_err(|e| format!("Failed to serialize YAML: {}", e))
}

/// Write compose content to file
/// Namespaces container names to prevent global conflicts
async fn write_compose_file(data_dir: &PathBuf, service_name: &str, content: &str) -> Result<PathBuf, std::io::Error> {
    let dir = get_compose_dir(data_dir, service_name);
    tokio::fs::create_dir_all(&dir).await?;
    let compose_file = dir.join("docker-compose.yml");

    // Namespace container names to prevent global conflicts
    let namespaced_content = namespace_container_names(content, service_name)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to namespace container names: {}. Using original content.", e);
            content.to_string()
        });

    tokio::fs::write(&compose_file, namespaced_content).await?;
    Ok(dir)
}

/// Run docker compose command
async fn run_compose_command(
    project_dir: &std::path::Path,
    project_name: &str,
    args: &[&str],
) -> Result<String, String> {
    // Try docker compose first (modern), then docker-compose (legacy)
    let result = Command::new("docker")
        .arg("compose")
        .arg("-p")
        .arg(project_name)
        .args(args)
        .current_dir(project_dir)
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(_) => {
            // Try legacy docker-compose command
            let output = Command::new("docker-compose")
                .arg("-p")
                .arg(project_name)
                .args(args)
                .current_dir(project_dir)
                .output()
                .await
                .map_err(|e| format!("Failed to execute docker-compose: {}", e))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
    }
}

/// List all services
pub async fn list_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ServiceResponse>>, StatusCode> {
    let services = sqlx::query_as::<_, Service>(
        "SELECT * FROM services ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list services: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<ServiceResponse> = services.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}

/// Get a single service by ID
pub async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServiceResponse>, StatusCode> {
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(service.into()))
}

/// Create a new Docker Compose service
pub async fn create_service(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<CreateServiceRequest>,
) -> Result<(StatusCode, Json<ServiceResponse>), StatusCode> {
    // Validate name
    if req.name.is_empty() {
        tracing::warn!("Service name is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate name format (alphanumeric and hyphens only)
    if !req.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        tracing::warn!("Service name contains invalid characters: {}", req.name);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate compose content
    if let Err(e) = validate_compose_content(&req.compose_content) {
        tracing::warn!("Invalid compose content: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create service record
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO services (id, name, project_id, compose_content, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.project_id)
    .bind(&req.compose_content)
    .bind(ServiceStatus::Pending.to_string())
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create service: {}", e);
        if e.to_string().contains("UNIQUE") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    // Write compose file to disk
    let data_dir = &state.config.server.data_dir;
    if let Err(e) = write_compose_file(data_dir, &req.name, &req.compose_content).await {
        tracing::error!("Failed to write compose file: {}", e);
        // Clean up the database record
        let _ = sqlx::query("DELETE FROM services WHERE id = ?")
            .bind(&id)
            .execute(&state.db)
            .await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Fetch and return the service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::SERVICE_CREATE,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    tracing::info!("Created Docker Compose service: {}", req.name);
    Ok((StatusCode::CREATED, Json(service.into())))
}

/// Update a Docker Compose service
pub async fn update_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateServiceRequest>,
) -> Result<Json<ServiceResponse>, StatusCode> {
    // Check if service exists
    let existing = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get service: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate new compose content if provided
    if let Some(ref content) = req.compose_content {
        if let Err(e) = validate_compose_content(content) {
            tracing::warn!("Invalid compose content: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Update compose content if provided
    if let Some(ref content) = req.compose_content {
        sqlx::query("UPDATE services SET compose_content = ?, updated_at = ? WHERE id = ?")
            .bind(content)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update service compose content: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Write updated compose file to disk
        let data_dir = &state.config.server.data_dir;
        if let Err(e) = write_compose_file(data_dir, &existing.name, content).await {
            tracing::error!("Failed to write compose file: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Update project_id if provided
    if let Some(ref project_id) = req.project_id {
        sqlx::query("UPDATE services SET project_id = ?, updated_at = ? WHERE id = ?")
            .bind(project_id)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update service project_id: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Updated Docker Compose service: {}", existing.name);
    Ok(Json(service.into()))
}

/// Delete a Docker Compose service
pub async fn delete_service(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
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

    // Stop and remove containers if running
    let data_dir = &state.config.server.data_dir;
    let compose_dir = get_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    if compose_dir.exists() {
        // Run docker compose down to clean up containers
        if let Err(e) = run_compose_command(&compose_dir, &project_name, &["down", "--volumes", "--remove-orphans"]).await {
            tracing::warn!("Failed to run compose down: {}", e);
        }

        // Remove compose directory
        if let Err(e) = tokio::fs::remove_dir_all(&compose_dir).await {
            tracing::warn!("Failed to remove compose directory: {}", e);
        }
    }

    // Delete the database record
    sqlx::query("DELETE FROM services WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete service: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::SERVICE_DELETE,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    tracing::info!("Deleted Docker Compose service: {}", service.name);
    Ok(StatusCode::NO_CONTENT)
}

/// Start a Docker Compose service
pub async fn start_service(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
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
    let compose_dir = get_compose_dir(data_dir, &service.name);
    let project_name = service.compose_project_name();

    // Always write/overwrite compose file from database content (in case it was updated)
    if let Err(e) = write_compose_file(data_dir, &service.name, &service.compose_content).await {
        tracing::error!("Failed to write compose file: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clean up any orphaned containers from previous failed deployments
    // This prevents "container name already in use" errors
    tracing::debug!("Cleaning up orphaned containers for project: {}", project_name);
    if let Err(e) = run_compose_command(&compose_dir, &project_name, &["down", "--remove-orphans"]).await {
        // Log but don't fail - the containers might not exist yet
        tracing::debug!("Compose down (cleanup) result: {}", e);
    }

    // Run docker compose up -d
    match run_compose_command(&compose_dir, &project_name, &["up", "-d"]).await {
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

            tracing::info!("Started Docker Compose service: {}", service.name);
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

            tracing::error!("Failed to start Docker Compose service {}: {}", service.name, e);
        }
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::SERVICE_START,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(service.into()))
}

/// Stop a Docker Compose service
pub async fn stop_service(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
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
    let compose_dir = get_compose_dir(data_dir, &service.name);
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

            tracing::error!("Failed to stop Docker Compose service {}: {}", service.name, e);
        }
    }

    // Fetch and return the updated service
    let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::SERVICE_STOP,
        resource_types::SERVICE,
        Some(&service.id),
        Some(&service.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(service.into()))
}

// -------------------------------------------------------------------------
// Service Logs
// -------------------------------------------------------------------------

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

/// Get the compose directory for a service, checking both data dir and temp dir
fn get_service_compose_dir(data_dir: &PathBuf, service_name: &str) -> PathBuf {
    // First try the standard data directory
    let data_compose_dir = get_compose_dir(data_dir, service_name);
    if data_compose_dir.join("docker-compose.yml").exists() {
        return data_compose_dir;
    }

    // Try the temp directory (used by template deployments)
    let temp_compose_dir = std::env::temp_dir().join(format!("rivetr-svc-{}", service_name));
    if temp_compose_dir.join("docker-compose.yml").exists() {
        return temp_compose_dir;
    }

    // Default to data directory even if it doesn't exist
    data_compose_dir
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

/// Stream logs from a Docker Compose service using SSE
/// GET /api/services/:id/logs/stream
pub async fn stream_service_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<serde_json::Value>)> {
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
