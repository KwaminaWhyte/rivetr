//! Database backup API endpoints

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::db::{
    BackupType, CreateBackupScheduleRequest, DatabaseBackup, DatabaseBackupResponse,
    DatabaseBackupSchedule, DatabaseBackupScheduleResponse, ManagedDatabase, ScheduleType,
};
use crate::engine::database_backups::DatabaseBackupTask;
use crate::AppState;

use super::error::{ApiError, ErrorCode};

#[derive(Debug, Deserialize)]
pub struct ListBackupsQuery {
    #[serde(default)]
    pub limit: Option<i32>,
}

/// List backups for a database
pub async fn list_backups(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
    Query(query): Query<ListBackupsQuery>,
) -> Result<Json<Vec<DatabaseBackupResponse>>, ApiError> {
    // Verify database exists
    let _database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Database not found"))?;

    let limit = query.limit.unwrap_or(50).min(100);

    let backups: Vec<DatabaseBackup> = sqlx::query_as(
        r#"
        SELECT id, database_id, backup_type, status, file_path, file_size,
               backup_format, started_at, completed_at, error_message, created_at, updated_at
        FROM database_backups
        WHERE database_id = ?
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(&database_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<DatabaseBackupResponse> = backups.into_iter().map(Into::into).collect();

    Ok(Json(responses))
}

/// Get a specific backup
pub async fn get_backup(
    State(state): State<Arc<AppState>>,
    Path((database_id, backup_id)): Path<(String, String)>,
) -> Result<Json<DatabaseBackupResponse>, ApiError> {
    let backup: DatabaseBackup = sqlx::query_as(
        r#"
        SELECT id, database_id, backup_type, status, file_path, file_size,
               backup_format, started_at, completed_at, error_message, created_at, updated_at
        FROM database_backups
        WHERE id = ? AND database_id = ?
        "#,
    )
    .bind(&backup_id)
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Backup not found"))?;

    Ok(Json(backup.into()))
}

/// Trigger a manual backup
pub async fn create_backup(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
) -> Result<Json<DatabaseBackupResponse>, ApiError> {
    // Verify database exists and is running
    let database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Database not found"))?;

    if database.status != "running" {
        return Err(ApiError::new(
            ErrorCode::BadRequest,
            "Database must be running to create a backup",
        ));
    }

    // Create the backup task
    let backup_task = DatabaseBackupTask::new(
        state.db.clone(),
        state.runtime.clone(),
        state.config.database_backup.clone(),
        state.config.server.data_dir.clone(),
    );

    // Run the backup
    let backup = backup_task
        .backup_database(&database, BackupType::Manual)
        .await
        .map_err(|e| ApiError::new(ErrorCode::InternalError, format!("Backup failed: {}", e)))?;

    Ok(Json(backup.into()))
}

/// Delete a backup
pub async fn delete_backup(
    State(state): State<Arc<AppState>>,
    Path((database_id, backup_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get the backup
    let backup: DatabaseBackup = sqlx::query_as(
        r#"
        SELECT id, database_id, backup_type, status, file_path, file_size,
               backup_format, started_at, completed_at, error_message, created_at, updated_at
        FROM database_backups
        WHERE id = ? AND database_id = ?
        "#,
    )
    .bind(&backup_id)
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Backup not found"))?;

    // Delete the backup file if it exists
    if let Some(file_path) = &backup.file_path {
        if let Err(e) = tokio::fs::remove_file(file_path).await {
            tracing::warn!(
                backup_id = %backup_id,
                file_path = %file_path,
                error = %e,
                "Failed to delete backup file"
            );
        }
    }

    // Delete the backup record
    sqlx::query("DELETE FROM database_backups WHERE id = ?")
        .bind(&backup_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// Download a backup file
pub async fn download_backup(
    State(state): State<Arc<AppState>>,
    Path((database_id, backup_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    // Get the backup
    let backup: DatabaseBackup = sqlx::query_as(
        r#"
        SELECT id, database_id, backup_type, status, file_path, file_size,
               backup_format, started_at, completed_at, error_message, created_at, updated_at
        FROM database_backups
        WHERE id = ? AND database_id = ?
        "#,
    )
    .bind(&backup_id)
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Backup not found"))?;

    // Check backup status
    if backup.status != "completed" {
        return Err(ApiError::new(
            ErrorCode::BadRequest,
            "Backup is not completed",
        ));
    }

    // Get file path
    let file_path = backup
        .file_path
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Backup file path not found"))?;

    // Open the file
    let file = tokio::fs::File::open(&file_path).await.map_err(|e| {
        tracing::error!(error = %e, file_path = %file_path, "Failed to open backup file");
        ApiError::new(ErrorCode::NotFound, "Backup file not found on disk")
    })?;

    // Get file metadata for content-length
    let metadata = file.metadata().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get file metadata");
        ApiError::new(ErrorCode::InternalError, "Failed to read backup file")
    })?;

    // Extract filename from path
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("backup.sql");

    // Create stream from file
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // Build response with appropriate headers
    let response = axum::response::Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .header(header::CONTENT_LENGTH, metadata.len())
        .body(body)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            ApiError::new(ErrorCode::InternalError, "Failed to build response")
        })?;

    Ok(response)
}

/// Get backup schedule for a database
pub async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
) -> Result<Json<Option<DatabaseBackupScheduleResponse>>, ApiError> {
    // Verify database exists
    let _database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Database not found"))?;

    let schedule: Option<DatabaseBackupSchedule> = sqlx::query_as(
        "SELECT * FROM database_backup_schedules WHERE database_id = ?",
    )
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(schedule.map(Into::into)))
}

/// Create or update backup schedule
pub async fn upsert_schedule(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
    Json(req): Json<CreateBackupScheduleRequest>,
) -> Result<Json<DatabaseBackupScheduleResponse>, ApiError> {
    // Verify database exists
    let _database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Database not found"))?;

    // Check if schedule exists
    let existing: Option<DatabaseBackupSchedule> = sqlx::query_as(
        "SELECT * FROM database_backup_schedules WHERE database_id = ?",
    )
    .bind(&database_id)
    .fetch_optional(&state.db)
    .await?;

    let schedule_type = req
        .schedule_type
        .as_ref()
        .map(|s| match s.as_str() {
            "hourly" => ScheduleType::Hourly,
            "weekly" => ScheduleType::Weekly,
            _ => ScheduleType::Daily,
        })
        .unwrap_or(ScheduleType::Daily);

    let now = chrono::Utc::now();

    if let Some(existing) = existing {
        // Update existing schedule
        let enabled = req.enabled.map(|e| if e { 1 } else { 0 }).unwrap_or(existing.enabled);
        let schedule_hour = req.schedule_hour.unwrap_or(existing.schedule_hour);
        let schedule_day = req.schedule_day.or(existing.schedule_day);
        let retention_count = req.retention_count.unwrap_or(existing.retention_count);
        let next_run = DatabaseBackupSchedule::calculate_next_run(
            &schedule_type,
            schedule_hour,
            schedule_day,
            &now,
        );

        sqlx::query(
            r#"
            UPDATE database_backup_schedules
            SET enabled = ?, schedule_type = ?, schedule_hour = ?, schedule_day = ?,
                retention_count = ?, next_run_at = ?, updated_at = ?
            WHERE database_id = ?
            "#,
        )
        .bind(enabled)
        .bind(schedule_type.to_string())
        .bind(schedule_hour)
        .bind(schedule_day)
        .bind(retention_count)
        .bind(next_run.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(&database_id)
        .execute(&state.db)
        .await?;

        // Fetch updated schedule
        let schedule: DatabaseBackupSchedule = sqlx::query_as(
            "SELECT * FROM database_backup_schedules WHERE database_id = ?",
        )
        .bind(&database_id)
        .fetch_one(&state.db)
        .await?;

        Ok(Json(schedule.into()))
    } else {
        // Create new schedule
        let schedule = DatabaseBackupSchedule::new(&database_id, schedule_type.clone());
        let enabled = req.enabled.map(|e| if e { 1 } else { 0 }).unwrap_or(1);
        let schedule_hour = req.schedule_hour.unwrap_or(2);
        let schedule_day = req.schedule_day;
        let retention_count = req.retention_count.unwrap_or(5);
        let next_run = DatabaseBackupSchedule::calculate_next_run(
            &schedule_type,
            schedule_hour,
            schedule_day,
            &now,
        );

        sqlx::query(
            r#"
            INSERT INTO database_backup_schedules
            (id, database_id, enabled, schedule_type, schedule_hour, schedule_day,
             retention_count, next_run_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&schedule.id)
        .bind(&database_id)
        .bind(enabled)
        .bind(schedule_type.to_string())
        .bind(schedule_hour)
        .bind(schedule_day)
        .bind(retention_count)
        .bind(next_run.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&state.db)
        .await?;

        // Fetch created schedule
        let schedule: DatabaseBackupSchedule = sqlx::query_as(
            "SELECT * FROM database_backup_schedules WHERE database_id = ?",
        )
        .bind(&database_id)
        .fetch_one(&state.db)
        .await?;

        Ok(Json(schedule.into()))
    }
}

/// Delete backup schedule
pub async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(database_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify database exists
    let _database: ManagedDatabase = sqlx::query_as("SELECT * FROM databases WHERE id = ?")
        .bind(&database_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NotFound, "Database not found"))?;

    sqlx::query("DELETE FROM database_backup_schedules WHERE database_id = ?")
        .bind(&database_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}
