use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{CreateVolumeRequest, UpdateVolumeRequest, Volume, VolumeResponse};
use crate::AppState;

/// List all volumes for an app
pub async fn list_volumes(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<VolumeResponse>>, StatusCode> {
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

    let volumes = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE app_id = ? ORDER BY name ASC"
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list volumes: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let responses: Vec<VolumeResponse> = volumes.into_iter().map(VolumeResponse::from).collect();

    Ok(Json(responses))
}

/// Create a new volume for an app
pub async fn create_volume(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateVolumeRequest>,
) -> Result<(StatusCode, Json<VolumeResponse>), StatusCode> {
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

    // Validate inputs
    if req.name.is_empty() {
        tracing::warn!("Volume name is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    if req.host_path.is_empty() {
        tracing::warn!("Volume host_path is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    if req.container_path.is_empty() {
        tracing::warn!("Volume container_path is empty");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Container path must be absolute
    if !req.container_path.starts_with('/') {
        tracing::warn!("Volume container_path must be absolute: {}", req.container_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO volumes (id, app_id, name, host_path, container_path, read_only, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.name)
    .bind(&req.host_path)
    .bind(&req.container_path)
    .bind(if req.read_only { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create volume: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let volume = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE id = ?"
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(VolumeResponse::from(volume))))
}

/// Get a single volume by ID
pub async fn get_volume(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    let volume = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE id = ?"
    )
    .bind(&volume_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get volume: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(VolumeResponse::from(volume)))
}

/// Update an existing volume
pub async fn update_volume(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
    Json(req): Json<UpdateVolumeRequest>,
) -> Result<Json<VolumeResponse>, StatusCode> {
    // Check if volume exists
    let existing = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE id = ?"
    )
    .bind(&volume_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let now = chrono::Utc::now().to_rfc3339();

    // Apply updates
    let new_name = req.name.unwrap_or(existing.name);
    let new_host_path = req.host_path.unwrap_or(existing.host_path);
    let new_container_path = req.container_path.unwrap_or(existing.container_path);
    let new_read_only = req
        .read_only
        .map(|b| if b { 1 } else { 0 })
        .unwrap_or(existing.read_only);

    // Validate container_path is absolute
    if !new_container_path.starts_with('/') {
        tracing::warn!("Volume container_path must be absolute: {}", new_container_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query(
        r#"
        UPDATE volumes SET
            name = ?,
            host_path = ?,
            container_path = ?,
            read_only = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&new_name)
    .bind(&new_host_path)
    .bind(&new_container_path)
    .bind(new_read_only)
    .bind(&now)
    .bind(&volume_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update volume: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let volume = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE id = ?"
    )
    .bind(&volume_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(VolumeResponse::from(volume)))
}

/// Delete a volume
pub async fn delete_volume(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM volumes WHERE id = ?")
        .bind(&volume_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete volume: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Backup a volume to a tar.gz archive
/// Returns the backup file as a download
pub async fn backup_volume(
    State(state): State<Arc<AppState>>,
    Path(volume_id): Path<String>,
) -> Result<(StatusCode, [(axum::http::header::HeaderName, String); 2], Vec<u8>), StatusCode> {
    // Get the volume
    let volume = sqlx::query_as::<_, Volume>(
        "SELECT id, app_id, name, host_path, container_path, read_only, created_at, updated_at FROM volumes WHERE id = ?"
    )
    .bind(&volume_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get volume: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Check if the host_path exists
    let path = std::path::Path::new(&volume.host_path);
    if !path.exists() {
        tracing::warn!("Volume host path does not exist: {}", volume.host_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Create a tar.gz archive in memory
    let backup_data = create_tar_gz_backup(path).await.map_err(|e| {
        tracing::error!("Failed to create backup: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Generate filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}-{}.tar.gz", volume.name, timestamp);

    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/gzip".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        backup_data,
    ))
}

/// Create a tar.gz backup of a directory
async fn create_tar_gz_backup(path: &std::path::Path) -> anyhow::Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut tar_data = Vec::new();

    // Create the tar archive
    {
        let mut tar_builder = tar::Builder::new(&mut tar_data);

        if path.is_dir() {
            tar_builder.append_dir_all(".", path)?;
        } else {
            // Single file
            let mut file = std::fs::File::open(path)?;
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            tar_builder.append_file(&*file_name, &mut file)?;
        }

        tar_builder.finish()?;
    }

    // Compress the tar archive
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_data)?;
    let compressed = encoder.finish()?;

    Ok(compressed)
}
