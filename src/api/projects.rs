//! Projects API endpoints for grouping related apps/services together.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    actions, resource_types, App, AssignAppProjectRequest, CreateProjectRequest, ManagedDatabase,
    Project, ProjectWithAppCount, ProjectWithApps, Service, UpdateProjectRequest, User,
};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::validate_uuid;

/// Validate a project name
fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Project name is required".to_string());
    }

    if name.len() > 100 {
        return Err("Project name is too long (max 100 characters)".to_string());
    }

    if name.len() < 2 {
        return Err("Project name is too short (min 2 characters)".to_string());
    }

    Ok(())
}

/// Validate a project description
fn validate_project_description(description: &Option<String>) -> Result<(), String> {
    if let Some(d) = description {
        if d.len() > 500 {
            return Err("Project description is too long (max 500 characters)".to_string());
        }
    }

    Ok(())
}

/// Validate a CreateProjectRequest
fn validate_create_request(req: &CreateProjectRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Err(e) = validate_project_name(&req.name) {
        errors.add("name", &e);
    }

    if let Err(e) = validate_project_description(&req.description) {
        errors.add("description", &e);
    }

    errors.finish()
}

/// Validate an UpdateProjectRequest
fn validate_update_request(req: &UpdateProjectRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Some(ref name) = req.name {
        if let Err(e) = validate_project_name(name) {
            errors.add("name", &e);
        }
    }

    if let Err(e) = validate_project_description(&req.description) {
        errors.add("description", &e);
    }

    errors.finish()
}

/// List all projects with app counts
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProjectWithAppCount>>, ApiError> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT * FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await?;

    // Get app counts for each project
    let mut results = Vec::new();
    for project in projects {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM apps WHERE project_id = ?"
        )
        .bind(&project.id)
        .fetch_one(&state.db)
        .await?;

        results.push(ProjectWithAppCount {
            id: project.id,
            name: project.name,
            description: project.description,
            created_at: project.created_at,
            updated_at: project.updated_at,
            app_count: count.0,
        });
    }

    Ok(Json(results))
}

/// Get a project with its apps and databases
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProjectWithApps>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE project_id = ? ORDER BY created_at DESC"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    let databases = sqlx::query_as::<_, ManagedDatabase>(
        "SELECT * FROM databases WHERE project_id = ? ORDER BY created_at DESC"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    let services = sqlx::query_as::<_, Service>(
        "SELECT * FROM services WHERE project_id = ? ORDER BY created_at DESC"
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    let host = Some(state.config.server.host.as_str());
    let database_responses = databases
        .into_iter()
        .map(|db| db.to_response(false, host))
        .collect();

    let service_responses = services
        .into_iter()
        .map(|s| s.to_response())
        .collect();

    Ok(Json(ProjectWithApps {
        id: project.id,
        name: project.name,
        description: project.description,
        created_at: project.created_at,
        updated_at: project.updated_at,
        apps,
        databases: database_responses,
        services: service_responses,
    }))
}

/// Create a new project
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), ApiError> {
    // Validate request
    validate_create_request(&req)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO projects (id, name, description, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create project: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A project with this name already exists")
        } else {
            ApiError::database("Failed to create project")
        }
    })?;

    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::PROJECT_CREATE,
        resource_types::PROJECT,
        Some(&project.id),
        Some(&project.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok((StatusCode::CREATED, Json(project)))
}

/// Update a project
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Validate request
    validate_update_request(&req)?;

    // Check if project exists
    let _existing = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE projects SET
            name = COALESCE(?, name),
            description = COALESCE(?, description),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update project: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A project with this name already exists")
        } else {
            ApiError::database("Failed to update project")
        }
    })?;

    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::PROJECT_UPDATE,
        resource_types::PROJECT,
        Some(&project.id),
        Some(&project.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(project))
}

/// Delete a project (apps become unassigned)
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Get project before deleting for audit log
    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    // Delete the project - apps will have project_id set to NULL due to ON DELETE SET NULL
    let result = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::PROJECT_DELETE,
        resource_types::PROJECT,
        Some(&project.id),
        Some(&project.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// Assign or unassign an app to a project
pub async fn assign_app_project(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<AssignAppProjectRequest>,
) -> Result<Json<App>, ApiError> {
    // Validate app ID format
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // If project_id is provided, validate it exists
    if let Some(ref project_id) = req.project_id {
        if let Err(e) = validate_uuid(project_id, "project_id") {
            return Err(ApiError::validation_field("project_id", e));
        }

        let project_exists = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_optional(&state.db)
            .await?;

        if project_exists.is_none() {
            return Err(ApiError::not_found("Project not found"));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE apps SET
            project_id = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.project_id)
    .bind(&now)
    .bind(&app_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to assign app to project: {}", e);
        ApiError::database("Failed to assign app to project")
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(app))
}
