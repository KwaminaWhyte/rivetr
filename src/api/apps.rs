use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{App, CreateAppRequest, UpdateAppRequest};
use crate::AppState;

use super::error::{ApiError, ValidationErrorBuilder};
use super::validation::{
    validate_app_name, validate_branch, validate_cpu_limit, validate_dockerfile, validate_domain,
    validate_environment, validate_git_url, validate_healthcheck, validate_memory_limit,
    validate_port, validate_uuid,
};

/// Validate a CreateAppRequest
fn validate_create_request(req: &CreateAppRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Err(e) = validate_app_name(&req.name) {
        errors.add("name", &e);
    }

    if let Err(e) = validate_git_url(&req.git_url) {
        errors.add("git_url", &e);
    }

    if let Err(e) = validate_branch(&req.branch) {
        errors.add("branch", &e);
    }

    if let Err(e) = validate_dockerfile(&req.dockerfile) {
        errors.add("dockerfile", &e);
    }

    if let Err(e) = validate_domain(&req.domain) {
        errors.add("domain", &e);
    }

    if let Err(e) = validate_port(req.port) {
        errors.add("port", &e);
    }

    if let Err(e) = validate_healthcheck(&req.healthcheck) {
        errors.add("healthcheck", &e);
    }

    if let Err(e) = validate_memory_limit(&req.memory_limit) {
        errors.add("memory_limit", &e);
    }

    if let Err(e) = validate_cpu_limit(&req.cpu_limit) {
        errors.add("cpu_limit", &e);
    }

    // Environment is validated through serde deserialization (enum), but we double-check here
    if let Err(e) = validate_environment(&req.environment.to_string()) {
        errors.add("environment", &e);
    }

    errors.finish()
}

/// Validate an UpdateAppRequest (only validates provided fields)
fn validate_update_request(req: &UpdateAppRequest) -> Result<(), ApiError> {
    let mut errors = ValidationErrorBuilder::new();

    if let Some(ref name) = req.name {
        if let Err(e) = validate_app_name(name) {
            errors.add("name", &e);
        }
    }

    if let Some(ref git_url) = req.git_url {
        if let Err(e) = validate_git_url(git_url) {
            errors.add("git_url", &e);
        }
    }

    if let Some(ref branch) = req.branch {
        if let Err(e) = validate_branch(branch) {
            errors.add("branch", &e);
        }
    }

    if let Some(ref dockerfile) = req.dockerfile {
        if let Err(e) = validate_dockerfile(dockerfile) {
            errors.add("dockerfile", &e);
        }
    }

    if let Err(e) = validate_domain(&req.domain) {
        errors.add("domain", &e);
    }

    if let Some(port) = req.port {
        if let Err(e) = validate_port(port) {
            errors.add("port", &e);
        }
    }

    if let Err(e) = validate_healthcheck(&req.healthcheck) {
        errors.add("healthcheck", &e);
    }

    if let Err(e) = validate_memory_limit(&req.memory_limit) {
        errors.add("memory_limit", &e);
    }

    if let Err(e) = validate_cpu_limit(&req.cpu_limit) {
        errors.add("cpu_limit", &e);
    }

    if let Some(ref environment) = req.environment {
        if let Err(e) = validate_environment(&environment.to_string()) {
            errors.add("environment", &e);
        }
    }

    errors.finish()
}

pub async fn list_apps(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<App>>, ApiError> {
    let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(apps))
}

pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<App>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    Ok(Json(app))
}

pub async fn create_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<App>), ApiError> {
    // Validate request
    validate_create_request(&req)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO apps (id, name, git_url, branch, dockerfile, domain, port, healthcheck, memory_limit, cpu_limit, ssh_key_id, environment, project_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.git_url)
    .bind(&req.branch)
    .bind(&req.dockerfile)
    .bind(&req.domain)
    .bind(req.port)
    .bind(&req.healthcheck)
    .bind(&req.memory_limit)
    .bind(&req.cpu_limit)
    .bind(&req.ssh_key_id)
    .bind(req.environment.to_string())
    .bind(&req.project_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create app: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to create app")
        }
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(app)))
}

pub async fn update_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAppRequest>,
) -> Result<Json<App>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Validate request
    validate_update_request(&req)?;

    // Check if app exists
    let _existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    // Convert environment enum to string for binding
    let environment_str = req.environment.as_ref().map(|e| e.to_string());

    sqlx::query(
        r#"
        UPDATE apps SET
            name = COALESCE(?, name),
            git_url = COALESCE(?, git_url),
            branch = COALESCE(?, branch),
            dockerfile = COALESCE(?, dockerfile),
            domain = COALESCE(?, domain),
            port = COALESCE(?, port),
            healthcheck = COALESCE(?, healthcheck),
            memory_limit = COALESCE(?, memory_limit),
            cpu_limit = COALESCE(?, cpu_limit),
            ssh_key_id = COALESCE(?, ssh_key_id),
            environment = COALESCE(?, environment),
            project_id = COALESCE(?, project_id),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.git_url)
    .bind(&req.branch)
    .bind(&req.dockerfile)
    .bind(&req.domain)
    .bind(req.port)
    .bind(&req.healthcheck)
    .bind(&req.memory_limit)
    .bind(&req.cpu_limit)
    .bind(&req.ssh_key_id)
    .bind(&environment_str)
    .bind(&req.project_id)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update app: {}", e);
        // Check for unique constraint violation
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An app with this name already exists")
        } else {
            ApiError::database("Failed to update app")
        }
    })?;

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(app))
}

pub async fn delete_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("App not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
