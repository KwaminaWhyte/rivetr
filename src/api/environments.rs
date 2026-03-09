//! Project environments API endpoints.
//!
//! Manages environments (dev/staging/production) within projects,
//! and environment-scoped environment variables.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{
    CreateEnvironmentEnvVarRequest, CreateEnvironmentRequest, EnvironmentEnvVar,
    EnvironmentEnvVarResponse, EnvironmentResponse, ProjectEnvironment, UpdateEnvironmentEnvVarRequest,
    UpdateEnvironmentRequest,
};
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// Query params for listing environment env vars
#[derive(Debug, Deserialize)]
pub struct ListEnvVarsQuery {
    #[serde(default)]
    pub reveal: bool,
}

// ---------------------------------------------------------------------------
// Environment CRUD
// ---------------------------------------------------------------------------

/// List all environments for a project
pub async fn list_environments(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<EnvironmentResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Verify project exists
    let project_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await?;
    if project_exists == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    let environments = sqlx::query_as::<_, ProjectEnvironment>(
        "SELECT * FROM environments WHERE project_id = ? ORDER BY is_default DESC, name ASC",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<EnvironmentResponse> = environments.iter().map(|e| e.to_response()).collect();

    Ok(Json(responses))
}

/// Create a new environment for a project
pub async fn create_environment(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateEnvironmentRequest>,
) -> Result<(StatusCode, Json<EnvironmentResponse>), ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    // Validate name
    if req.name.is_empty() {
        return Err(ApiError::validation_field(
            "name",
            "Environment name is required",
        ));
    }
    if req.name.len() > 50 {
        return Err(ApiError::validation_field(
            "name",
            "Environment name is too long (max 50 characters)",
        ));
    }

    // Verify project exists
    let project_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await?;
    if project_exists == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO environments (id, project_id, name, description, is_default, created_at, updated_at)
        VALUES (?, ?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An environment with this name already exists in this project")
        } else {
            tracing::error!("Failed to create environment: {}", e);
            ApiError::database("Failed to create environment")
        }
    })?;

    let env = sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(env.to_response())))
}

/// Update an environment
pub async fn update_environment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateEnvironmentRequest>,
) -> Result<Json<EnvironmentResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.is_empty() {
            return Err(ApiError::validation_field(
                "name",
                "Environment name is required",
            ));
        }
        if name.len() > 50 {
            return Err(ApiError::validation_field(
                "name",
                "Environment name is too long (max 50 characters)",
            ));
        }
    }

    let existing =
        sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    // Prevent renaming default environments
    if existing.is_default != 0 && req.name.is_some() {
        return Err(ApiError::bad_request(
            "Cannot rename a default environment",
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE environments SET
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
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An environment with this name already exists in this project")
        } else {
            tracing::error!("Failed to update environment: {}", e);
            ApiError::database("Failed to update environment")
        }
    })?;

    let env = sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(env.to_response()))
}

/// Delete an environment (cannot delete default environments)
pub async fn delete_environment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }

    let existing =
        sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM environments WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    // Prevent deleting default environments
    if existing.is_default != 0 {
        return Err(ApiError::bad_request(
            "Cannot delete a default environment. Change the default first.",
        ));
    }

    sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Environment Env Vars CRUD
// ---------------------------------------------------------------------------

/// List environment variables for an environment
pub async fn list_env_vars(
    State(state): State<Arc<AppState>>,
    Path(env_id): Path<String>,
    Query(query): Query<ListEnvVarsQuery>,
) -> Result<Json<Vec<EnvironmentEnvVarResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&env_id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }

    // Verify environment exists
    let env_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM environments WHERE id = ?")
            .bind(&env_id)
            .fetch_one(&state.db)
            .await?;
    if env_exists == 0 {
        return Err(ApiError::not_found("Environment not found"));
    }

    let vars = sqlx::query_as::<_, EnvironmentEnvVar>(
        "SELECT * FROM environment_env_vars WHERE environment_id = ? ORDER BY key ASC",
    )
    .bind(&env_id)
    .fetch_all(&state.db)
    .await?;

    let encryption_key = get_encryption_key(&state);

    let responses: Vec<EnvironmentEnvVarResponse> = vars
        .into_iter()
        .map(|v| {
            let decrypted_value = crypto::decrypt_if_encrypted(&v.value, encryption_key.as_ref())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to decrypt env var {}: {}", v.key, e);
                    v.value.clone()
                });
            let decrypted_var = EnvironmentEnvVar {
                value: decrypted_value,
                ..v
            };
            decrypted_var.to_response(query.reveal)
        })
        .collect();

    Ok(Json(responses))
}

/// Validate environment variable key format
fn is_valid_env_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let first_char = key.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }
    key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Create an environment variable for an environment
pub async fn create_env_var(
    State(state): State<Arc<AppState>>,
    Path(env_id): Path<String>,
    Json(req): Json<CreateEnvironmentEnvVarRequest>,
) -> Result<(StatusCode, Json<EnvironmentEnvVarResponse>), ApiError> {
    if let Err(e) = validate_uuid(&env_id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }

    // Verify environment exists
    let env_exists =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM environments WHERE id = ?")
            .bind(&env_id)
            .fetch_one(&state.db)
            .await?;
    if env_exists == 0 {
        return Err(ApiError::not_found("Environment not found"));
    }

    // Validate key format
    if !is_valid_env_key(&req.key) {
        return Err(ApiError::validation_field(
            "key",
            "Invalid environment variable key format. Must start with a letter or underscore and contain only alphanumeric characters and underscores.",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Encrypt value if configured
    let encryption_key = get_encryption_key(&state);
    let stored_value = crypto::encrypt_if_key_available(&req.value, encryption_key.as_ref())
        .map_err(|e| {
            tracing::error!("Failed to encrypt env var value: {}", e);
            ApiError::internal("Failed to encrypt environment variable value")
        })?;

    sqlx::query(
        r#"
        INSERT INTO environment_env_vars (id, environment_id, key, value, is_secret, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&env_id)
    .bind(&req.key)
    .bind(&stored_value)
    .bind(if req.is_secret { 1 } else { 0 })
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("An environment variable with this key already exists in this environment")
        } else {
            tracing::error!("Failed to create env var: {}", e);
            ApiError::database("Failed to create environment variable")
        }
    })?;

    let var = sqlx::query_as::<_, EnvironmentEnvVar>(
        "SELECT * FROM environment_env_vars WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    // Return with original plaintext value
    let response_var = EnvironmentEnvVar {
        value: req.value.clone(),
        ..var
    };

    Ok((StatusCode::CREATED, Json(response_var.to_response(true))))
}

/// Update an environment variable
pub async fn update_env_var(
    State(state): State<Arc<AppState>>,
    Path((env_id, var_id)): Path<(String, String)>,
    Json(req): Json<UpdateEnvironmentEnvVarRequest>,
) -> Result<Json<EnvironmentEnvVarResponse>, ApiError> {
    if let Err(e) = validate_uuid(&env_id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "env_var_id") {
        return Err(ApiError::validation_field("env_var_id", e));
    }

    let existing = sqlx::query_as::<_, EnvironmentEnvVar>(
        "SELECT * FROM environment_env_vars WHERE id = ? AND environment_id = ?",
    )
    .bind(&var_id)
    .bind(&env_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Environment variable not found"))?;

    let encryption_key = get_encryption_key(&state);

    // Decrypt existing value
    let existing_decrypted = crypto::decrypt_if_encrypted(&existing.value, encryption_key.as_ref())
        .unwrap_or_else(|_| existing.value.clone());

    let new_plaintext_value = req.value.unwrap_or(existing_decrypted);
    let new_is_secret = req
        .is_secret
        .map(|b| if b { 1 } else { 0 })
        .unwrap_or(existing.is_secret);

    // Encrypt for storage
    let stored_value =
        crypto::encrypt_if_key_available(&new_plaintext_value, encryption_key.as_ref()).map_err(
            |e| {
                tracing::error!("Failed to encrypt env var value: {}", e);
                ApiError::internal("Failed to encrypt environment variable value")
            },
        )?;

    sqlx::query(
        r#"
        UPDATE environment_env_vars SET
            value = ?,
            is_secret = ?
        WHERE id = ? AND environment_id = ?
        "#,
    )
    .bind(&stored_value)
    .bind(new_is_secret)
    .bind(&var_id)
    .bind(&env_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update env var: {}", e);
        ApiError::database("Failed to update environment variable")
    })?;

    let var = sqlx::query_as::<_, EnvironmentEnvVar>(
        "SELECT * FROM environment_env_vars WHERE id = ?",
    )
    .bind(&var_id)
    .fetch_one(&state.db)
    .await?;

    // Return plaintext value
    let response_var = EnvironmentEnvVar {
        value: new_plaintext_value,
        ..var
    };

    Ok(Json(response_var.to_response(true)))
}

/// Delete an environment variable
pub async fn delete_env_var(
    State(state): State<Arc<AppState>>,
    Path((env_id, var_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&env_id, "environment_id") {
        return Err(ApiError::validation_field("environment_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "env_var_id") {
        return Err(ApiError::validation_field("env_var_id", e));
    }

    let result =
        sqlx::query("DELETE FROM environment_env_vars WHERE id = ? AND environment_id = ?")
            .bind(&var_id)
            .bind(&env_id)
            .execute(&state.db)
            .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Environment variable not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Helper: Auto-create default environments for a project
// ---------------------------------------------------------------------------

/// Create the default environments (production, staging, development) for a project.
/// Called when a new project is created.
pub async fn create_default_environments(
    db: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();

    let defaults = [
        ("production", "Production environment", true),
        ("staging", "Staging environment", false),
        ("development", "Development environment", false),
    ];

    for (name, description, is_default) in defaults {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO environments (id, project_id, name, description, is_default, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(description)
        .bind(if is_default { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;
    }

    Ok(())
}
