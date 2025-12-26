use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{CreateSshKeyRequest, SshKey, SshKeyResponse, UpdateSshKeyRequest};
use crate::AppState;

/// List all SSH keys (returns public info only, not private keys)
pub async fn list_ssh_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SshKeyResponse>>, StatusCode> {
    let keys = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list SSH keys: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<SshKeyResponse> = keys.into_iter().map(SshKeyResponse::from).collect();
    Ok(Json(responses))
}

/// Get a single SSH key by ID (returns public info only)
pub async fn get_ssh_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SshKeyResponse>, StatusCode> {
    let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get SSH key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SshKeyResponse::from(key)))
}

/// Create a new SSH key
pub async fn create_ssh_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSshKeyRequest>,
) -> Result<(StatusCode, Json<SshKeyResponse>), StatusCode> {
    // Validate the private key format
    if !req.private_key.contains("PRIVATE KEY") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // If marking as global, ensure there's only one global key
    // (or we could allow multiple and just use the most recent one)
    if req.is_global {
        // Check if setting app_id with global flag
        if req.app_id.is_some() {
            tracing::warn!("Cannot set app_id when is_global is true");
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    sqlx::query(
        r#"
        INSERT INTO ssh_keys (id, name, private_key, public_key, app_id, is_global, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.private_key)
    .bind(&req.public_key)
    .bind(&req.app_id)
    .bind(if req.is_global { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create SSH key: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(SshKeyResponse::from(key))))
}

/// Update an existing SSH key
pub async fn update_ssh_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSshKeyRequest>,
) -> Result<Json<SshKeyResponse>, StatusCode> {
    // Check if key exists
    let _existing = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate private key if provided
    if let Some(ref pk) = req.private_key {
        if !pk.contains("PRIVATE KEY") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE ssh_keys SET
            name = COALESCE(?, name),
            private_key = COALESCE(?, private_key),
            public_key = COALESCE(?, public_key),
            app_id = COALESCE(?, app_id),
            is_global = COALESCE(?, is_global),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.private_key)
    .bind(&req.public_key)
    .bind(&req.app_id)
    .bind(req.is_global.map(|b| if b { 1 } else { 0 }))
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update SSH key: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let key = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SshKeyResponse::from(key)))
}

/// Delete an SSH key
pub async fn delete_ssh_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM ssh_keys WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete SSH key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Get SSH keys for a specific app
pub async fn get_app_ssh_keys(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<SshKeyResponse>>, StatusCode> {
    let keys = sqlx::query_as::<_, SshKey>("SELECT * FROM ssh_keys WHERE app_id = ?")
        .bind(&app_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get app SSH keys: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<SshKeyResponse> = keys.into_iter().map(SshKeyResponse::from).collect();
    Ok(Json(responses))
}
