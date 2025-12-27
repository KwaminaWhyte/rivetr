//! HTTP Basic Auth API endpoints for applications.
//!
//! This module provides endpoints to manage HTTP Basic Auth settings
//! for protecting applications behind the proxy.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::hash_password;
use crate::db::App;
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// Response for basic auth status
#[derive(Debug, Serialize)]
pub struct BasicAuthStatusResponse {
    pub enabled: bool,
    pub username: Option<String>,
}

/// Request to update basic auth settings
#[derive(Debug, Deserialize)]
pub struct UpdateBasicAuthRequest {
    pub enabled: bool,
    pub username: Option<String>,
    /// Password in plain text - will be hashed before storing
    pub password: Option<String>,
}

/// Get basic auth status for an app
pub async fn get_basic_auth(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<BasicAuthStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    Ok(Json(BasicAuthStatusResponse {
        enabled: app.basic_auth_enabled != 0,
        username: app.basic_auth_username,
    }))
}

/// Update basic auth settings for an app
pub async fn update_basic_auth(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateBasicAuthRequest>,
) -> Result<Json<BasicAuthStatusResponse>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Check if app exists
    let _existing = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // Validate request
    if req.enabled {
        // Username is required when enabling basic auth
        let username = req.username.as_deref().unwrap_or("");
        if username.is_empty() {
            return Err(ApiError::validation_field(
                "username",
                "Username is required when enabling basic auth".to_string(),
            ));
        }
        if username.len() < 3 {
            return Err(ApiError::validation_field(
                "username",
                "Username must be at least 3 characters".to_string(),
            ));
        }
        if username.len() > 64 {
            return Err(ApiError::validation_field(
                "username",
                "Username must be at most 64 characters".to_string(),
            ));
        }
        // Check for valid username characters (alphanumeric, underscore, dash)
        if !username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(ApiError::validation_field(
                "username",
                "Username can only contain letters, numbers, underscores, and dashes".to_string(),
            ));
        }

        // Password is required when enabling or changing username
        let password = req.password.as_deref().unwrap_or("");
        if password.is_empty() {
            return Err(ApiError::validation_field(
                "password",
                "Password is required when enabling basic auth".to_string(),
            ));
        }
        if password.len() < 8 {
            return Err(ApiError::validation_field(
                "password",
                "Password must be at least 8 characters".to_string(),
            ));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    if req.enabled {
        // Hash the password and update
        let password_hash = hash_password(req.password.as_deref().unwrap())
            .map_err(|e| ApiError::internal(format!("Failed to hash password: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE apps SET
                basic_auth_enabled = 1,
                basic_auth_username = ?,
                basic_auth_password_hash = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&req.username)
        .bind(&password_hash)
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await?;
    } else {
        // Disable basic auth - clear credentials
        sqlx::query(
            r#"
            UPDATE apps SET
                basic_auth_enabled = 0,
                basic_auth_username = NULL,
                basic_auth_password_hash = NULL,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await?;
    }

    tracing::info!(
        app_id = %id,
        enabled = req.enabled,
        "Updated basic auth settings"
    );

    Ok(Json(BasicAuthStatusResponse {
        enabled: req.enabled,
        username: if req.enabled { req.username } else { None },
    }))
}

/// Delete/disable basic auth for an app
pub async fn delete_basic_auth(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE apps SET
            basic_auth_enabled = 0,
            basic_auth_username = NULL,
            basic_auth_password_hash = NULL,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("App not found"));
    }

    tracing::info!(app_id = %id, "Disabled basic auth");

    Ok(StatusCode::NO_CONTENT)
}
