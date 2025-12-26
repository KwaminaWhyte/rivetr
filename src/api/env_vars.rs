use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{CreateEnvVarRequest, EnvVar, EnvVarResponse, UpdateEnvVarRequest};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListEnvVarsQuery {
    /// If true, reveal secret values (default: false)
    #[serde(default)]
    pub reveal: bool,
}

/// List all environment variables for an app
/// Secret values are masked unless reveal=true query param is passed
pub async fn list_env_vars(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(query): Query<ListEnvVarsQuery>,
) -> Result<Json<Vec<EnvVarResponse>>, StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let env_vars =
        sqlx::query_as::<_, EnvVar>("SELECT * FROM env_vars WHERE app_id = ? ORDER BY key ASC")
            .bind(&app_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list env vars: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let responses: Vec<EnvVarResponse> = env_vars
        .into_iter()
        .map(|v| v.to_response(query.reveal))
        .collect();

    Ok(Json(responses))
}

/// Create a new environment variable for an app
pub async fn create_env_var(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateEnvVarRequest>,
) -> Result<(StatusCode, Json<EnvVarResponse>), StatusCode> {
    // Verify app exists
    let app_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check app: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if app_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Validate key format (alphanumeric + underscore, starts with letter or underscore)
    if req.key.is_empty() || !is_valid_env_key(&req.key) {
        tracing::warn!("Invalid env var key format: {}", req.key);
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO env_vars (id, app_id, key, value, is_secret, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.key)
    .bind(&req.value)
    .bind(if req.is_secret { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create env var: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let env_var = sqlx::query_as::<_, EnvVar>("SELECT * FROM env_vars WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Return with value visible since user just created it
    Ok((StatusCode::CREATED, Json(env_var.to_response(true))))
}

/// Update an existing environment variable
pub async fn update_env_var(
    State(state): State<Arc<AppState>>,
    Path((app_id, key)): Path<(String, String)>,
    Json(req): Json<UpdateEnvVarRequest>,
) -> Result<Json<EnvVarResponse>, StatusCode> {
    // Check if env var exists for this app
    let existing = sqlx::query_as::<_, EnvVar>(
        "SELECT * FROM env_vars WHERE app_id = ? AND key = ?",
    )
    .bind(&app_id)
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    // Build update query dynamically
    let new_value = req.value.unwrap_or(existing.value);
    let new_is_secret = req.is_secret.map(|b| if b { 1 } else { 0 }).unwrap_or(existing.is_secret);

    sqlx::query(
        r#"
        UPDATE env_vars SET
            value = ?,
            is_secret = ?,
            updated_at = ?
        WHERE app_id = ? AND key = ?
        "#,
    )
    .bind(&new_value)
    .bind(new_is_secret)
    .bind(&now)
    .bind(&app_id)
    .bind(&key)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update env var: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let env_var = sqlx::query_as::<_, EnvVar>(
        "SELECT * FROM env_vars WHERE app_id = ? AND key = ?",
    )
    .bind(&app_id)
    .bind(&key)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Return with value visible since user just updated it
    Ok(Json(env_var.to_response(true)))
}

/// Delete an environment variable
pub async fn delete_env_var(
    State(state): State<Arc<AppState>>,
    Path((app_id, key)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM env_vars WHERE app_id = ? AND key = ?")
        .bind(&app_id)
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete env var: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Get a single environment variable by key
pub async fn get_env_var(
    State(state): State<Arc<AppState>>,
    Path((app_id, key)): Path<(String, String)>,
    Query(query): Query<ListEnvVarsQuery>,
) -> Result<Json<EnvVarResponse>, StatusCode> {
    let env_var = sqlx::query_as::<_, EnvVar>(
        "SELECT * FROM env_vars WHERE app_id = ? AND key = ?",
    )
    .bind(&app_id)
    .bind(&key)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get env var: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(env_var.to_response(query.reveal)))
}

/// Validate environment variable key format
/// Must start with letter or underscore, contain only alphanumeric and underscore
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_env_keys() {
        assert!(is_valid_env_key("DATABASE_URL"));
        assert!(is_valid_env_key("_PRIVATE_VAR"));
        assert!(is_valid_env_key("API_KEY_123"));
        assert!(is_valid_env_key("MY_VAR"));
    }

    #[test]
    fn test_invalid_env_keys() {
        assert!(!is_valid_env_key(""));
        assert!(!is_valid_env_key("123_VAR"));  // starts with number
        assert!(!is_valid_env_key("MY-VAR"));   // contains hyphen
        assert!(!is_valid_env_key("MY VAR"));   // contains space
        assert!(!is_valid_env_key("MY.VAR"));   // contains period
    }
}
