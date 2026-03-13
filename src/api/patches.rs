use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{AppPatch, CreateAppPatchRequest, UpdateAppPatchRequest};
use crate::AppState;

/// List all patches for an app
pub async fn list_patches(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<AppPatch>>, StatusCode> {
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

    let patches = sqlx::query_as::<_, AppPatch>(
        "SELECT id, app_id, file_path, content, operation, is_enabled, created_at, updated_at \
         FROM app_patches WHERE app_id = ? ORDER BY created_at ASC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list patches: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(patches))
}

/// Create a new patch for an app
pub async fn create_patch(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateAppPatchRequest>,
) -> Result<(StatusCode, Json<AppPatch>), StatusCode> {
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

    if req.file_path.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let valid_ops = ["create", "append", "delete"];
    if !valid_ops.contains(&req.operation.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let content = req.content.unwrap_or_default();
    let is_enabled = if req.is_enabled { 1i32 } else { 0i32 };

    sqlx::query(
        "INSERT INTO app_patches (id, app_id, file_path, content, operation, is_enabled, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.file_path)
    .bind(&content)
    .bind(&req.operation)
    .bind(is_enabled)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create patch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let patch = sqlx::query_as::<_, AppPatch>(
        "SELECT id, app_id, file_path, content, operation, is_enabled, created_at, updated_at \
         FROM app_patches WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch created patch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(patch)))
}

/// Update an existing patch
pub async fn update_patch(
    State(state): State<Arc<AppState>>,
    Path((app_id, patch_id)): Path<(String, String)>,
    Json(req): Json<UpdateAppPatchRequest>,
) -> Result<Json<AppPatch>, StatusCode> {
    let existing = sqlx::query_as::<_, AppPatch>(
        "SELECT id, app_id, file_path, content, operation, is_enabled, created_at, updated_at \
         FROM app_patches WHERE id = ? AND app_id = ?",
    )
    .bind(&patch_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch patch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    let file_path = req.file_path.unwrap_or(existing.file_path);
    let content = req.content.unwrap_or(existing.content);
    let operation = req.operation.unwrap_or(existing.operation);
    let is_enabled = req
        .is_enabled
        .map(|b| if b { 1i32 } else { 0i32 })
        .unwrap_or(existing.is_enabled);

    if file_path.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let valid_ops = ["create", "append", "delete"];
    if !valid_ops.contains(&operation.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE app_patches SET file_path = ?, content = ?, operation = ?, is_enabled = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(&file_path)
    .bind(&content)
    .bind(&operation)
    .bind(is_enabled)
    .bind(&now)
    .bind(&patch_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update patch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let patch = sqlx::query_as::<_, AppPatch>(
        "SELECT id, app_id, file_path, content, operation, is_enabled, created_at, updated_at \
         FROM app_patches WHERE id = ?",
    )
    .bind(&patch_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch updated patch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(patch))
}

/// Delete a patch
pub async fn delete_patch(
    State(state): State<Arc<AppState>>,
    Path((app_id, patch_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM app_patches WHERE id = ? AND app_id = ?")
        .bind(&patch_id)
        .bind(&app_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete patch: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
