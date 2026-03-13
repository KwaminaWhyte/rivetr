//! CRUD handlers for Docker Compose services.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, CreateServiceRequest, Service, ServiceResponse, ServiceStatus,
    TeamAuditAction, TeamAuditResourceType, UpdateServiceRequest, User,
};
use crate::AppState;

use super::super::audit::{audit_log, extract_client_ip};
use super::super::teams::log_team_audit;
use super::compose::{
    get_compose_dir, run_compose_command, validate_compose_content, write_compose_file,
};

/// Query parameters for listing services
#[derive(Debug, serde::Deserialize, Default)]
pub struct ListServicesQuery {
    /// Filter by team ID (optional)
    pub team_id: Option<String>,
}

/// List all services
pub async fn list_services(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListServicesQuery>,
) -> Result<Json<Vec<ServiceResponse>>, StatusCode> {
    let services = match &query.team_id {
        Some(team_id) if team_id.is_empty() => {
            // Empty string means get legacy/unassigned services
            sqlx::query_as::<_, Service>(
                "SELECT * FROM services WHERE team_id IS NULL ORDER BY created_at DESC",
            )
            .fetch_all(&state.db)
            .await
        }
        Some(team_id) => {
            // Filter by specific team, include legacy services with no team_id
            sqlx::query_as::<_, Service>(
                "SELECT * FROM services WHERE team_id = ? OR team_id IS NULL ORDER BY created_at DESC",
            )
            .bind(team_id)
            .fetch_all(&state.db)
            .await
        }
        None => {
            // No filter, return all services (backward compatibility)
            sqlx::query_as::<_, Service>("SELECT * FROM services ORDER BY created_at DESC")
                .fetch_all(&state.db)
                .await
        }
    }
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
    if !req
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        tracing::warn!("Service name contains invalid characters: {}", req.name);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate compose content
    if let Err(e) = validate_compose_content(&req.compose_content) {
        tracing::warn!("Invalid compose content: {}", e);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for port conflict if a port is provided
    if let Some(port) = req.port {
        // Check if port is already used by another service
        let existing_service: Option<(String,)> =
            sqlx::query_as("SELECT name FROM services WHERE port = ?")
                .bind(port)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to check port conflict in services: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

        if let Some((name,)) = existing_service {
            tracing::warn!("Port {} already used by service '{}'", port, name);
            return Err(StatusCode::CONFLICT);
        }

        // Check if port is already used by a public database
        let existing_db: Option<(String,)> = sqlx::query_as(
            "SELECT name FROM databases WHERE external_port = ? AND public_access = 1",
        )
        .bind(port)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check port conflict in databases: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some((name,)) = existing_db {
            tracing::warn!("Port {} already used by database '{}'", port, name);
            return Err(StatusCode::CONFLICT);
        }
    }

    // Auto-generate domain if not provided
    let domain = match &req.domain {
        Some(d) if !d.is_empty() => req.domain.clone(),
        _ => state.config.proxy.generate_auto_domain(&req.name),
    };

    // Create service record
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO services (id, name, project_id, team_id, compose_content, domain, port, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.project_id)
    .bind(&req.team_id)
    .bind(&req.compose_content)
    .bind(&domain)
    .bind(req.port.unwrap_or(80))
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

    // Log team audit event if service belongs to a team
    if let Some(ref team_id) = service.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::ServiceCreated,
            TeamAuditResourceType::Service,
            Some(&service.id),
            Some(serde_json::json!({
                "service_name": service.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

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

    // Update domain if provided
    if let Some(ref domain) = req.domain {
        sqlx::query("UPDATE services SET domain = ?, updated_at = ? WHERE id = ?")
            .bind(domain)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update service domain: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Update port if provided
    if let Some(port) = req.port {
        sqlx::query("UPDATE services SET port = ?, updated_at = ? WHERE id = ?")
            .bind(port)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update service port: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Update isolated_network if provided
    if let Some(isolated) = req.isolated_network {
        sqlx::query("UPDATE services SET isolated_network = ?, updated_at = ? WHERE id = ?")
            .bind(isolated as i32)
            .bind(&now)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update service isolated_network: {}", e);
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
        if let Err(e) = run_compose_command(
            &compose_dir,
            &project_name,
            &["down", "--volumes", "--remove-orphans"],
        )
        .await
        {
            tracing::warn!("Failed to run compose down: {}", e);
        }

        // Remove compose directory
        if let Err(e) = tokio::fs::remove_dir_all(&compose_dir).await {
            tracing::warn!("Failed to remove compose directory: {}", e);
        }
    }

    // Remove proxy route if domain was configured
    if let Some(ref domain) = service.domain {
        if !domain.is_empty() {
            state.routes.load().remove_route(domain);
            tracing::info!("Removed proxy route for deleted service: {}", domain);
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

    // Log team audit event if service belonged to a team
    if let Some(ref team_id) = service.team_id {
        if let Err(e) = log_team_audit(
            &state.db,
            team_id,
            Some(&user.id),
            TeamAuditAction::ServiceDeleted,
            TeamAuditResourceType::Service,
            Some(&service.id),
            Some(serde_json::json!({
                "service_name": service.name,
            })),
        )
        .await
        {
            tracing::warn!("Failed to log team audit event: {}", e);
        }
    }

    tracing::info!("Deleted Docker Compose service: {}", service.name);
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Port availability check
// ---------------------------------------------------------------------------

/// Query parameters for the check-port endpoint
#[derive(Debug, serde::Deserialize)]
pub struct CheckPortQuery {
    pub port: i32,
}

/// Response body for the check-port endpoint
#[derive(Debug, Serialize)]
pub struct CheckPortResponse {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<String>,
}

/// GET /services/check-port?port=N
/// Returns whether the requested port is available for use.
pub async fn check_port(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CheckPortQuery>,
) -> Result<Json<CheckPortResponse>, StatusCode> {
    let port = query.port;

    // Check services table
    let existing_service: Option<(String,)> =
        sqlx::query_as("SELECT name FROM services WHERE port = ?")
            .bind(port)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check port in services: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    if let Some((name,)) = existing_service {
        return Ok(Json(CheckPortResponse {
            available: false,
            conflict: Some(format!(
                "Port {} is already used by service '{}'",
                port, name
            )),
        }));
    }

    // Check databases table (public databases only)
    let existing_db: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM databases WHERE external_port = ? AND public_access = 1",
    )
    .bind(port)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check port in databases: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some((name,)) = existing_db {
        return Ok(Json(CheckPortResponse {
            available: false,
            conflict: Some(format!(
                "Port {} is already used by database '{}'",
                port, name
            )),
        }));
    }

    Ok(Json(CheckPortResponse {
        available: true,
        conflict: None,
    }))
}
