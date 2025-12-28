//! API handlers for Docker Compose services

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use uuid::Uuid;

use crate::db::{CreateServiceRequest, Service, ServiceResponse, ServiceStatus, UpdateServiceRequest};
use crate::AppState;

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

/// Write compose content to file
async fn write_compose_file(data_dir: &PathBuf, service_name: &str, content: &str) -> Result<PathBuf, std::io::Error> {
    let dir = get_compose_dir(data_dir, service_name);
    tokio::fs::create_dir_all(&dir).await?;
    let compose_file = dir.join("docker-compose.yml");
    tokio::fs::write(&compose_file, content).await?;
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

    tracing::info!("Deleted Docker Compose service: {}", service.name);
    Ok(StatusCode::NO_CONTENT)
}

/// Start a Docker Compose service
pub async fn start_service(
    State(state): State<Arc<AppState>>,
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

    // Ensure compose file exists
    if !compose_dir.join("docker-compose.yml").exists() {
        // Write compose file from database content
        if let Err(e) = write_compose_file(data_dir, &service.name, &service.compose_content).await {
            tracing::error!("Failed to write compose file: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
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

    Ok(Json(service.into()))
}

/// Stop a Docker Compose service
pub async fn stop_service(
    State(state): State<Arc<AppState>>,
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

    Ok(Json(service.into()))
}
