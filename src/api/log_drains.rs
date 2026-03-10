//! Log drain API endpoints for managing log forwarding to external services.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CreateLogDrainRequest, LogDrain, LogDrainProvider, LogDrainResponse, UpdateLogDrainRequest,
};
use crate::logging::LogDrainManager;
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// List all log drains for an app
pub async fn list_log_drains(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<LogDrainResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify app exists
    let app_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM apps WHERE id = ?")
            .bind(&app_id)
            .fetch_optional(&state.db)
            .await?;
    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let drains = sqlx::query_as::<_, LogDrain>(
        "SELECT * FROM log_drains WHERE app_id = ? ORDER BY created_at DESC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<LogDrainResponse> = drains.into_iter().map(|d| d.into()).collect();
    Ok(Json(responses))
}

/// Create a new log drain for an app
pub async fn create_log_drain(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateLogDrainRequest>,
) -> Result<(StatusCode, Json<LogDrainResponse>), ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Verify app exists
    let app_exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM apps WHERE id = ?")
            .bind(&app_id)
            .fetch_optional(&state.db)
            .await?;
    if app_exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    // Validate name
    if req.name.trim().is_empty() {
        return Err(ApiError::validation_field("name", "Name is required"));
    }
    if req.name.len() > 100 {
        return Err(ApiError::validation_field(
            "name",
            "Name must be 100 characters or less",
        ));
    }

    // Validate config based on provider
    validate_drain_config(&req.provider, &req.config)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&req.config)
        .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?;

    sqlx::query(
        r#"
        INSERT INTO log_drains (id, app_id, name, provider, config, enabled, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.name)
    .bind(req.provider.to_string())
    .bind(&config_json)
    .bind(if req.enabled { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let drain = sqlx::query_as::<_, LogDrain>("SELECT * FROM log_drains WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok((StatusCode::CREATED, Json(drain.into())))
}

/// Update an existing log drain
pub async fn update_log_drain(
    State(state): State<Arc<AppState>>,
    Path((app_id, drain_id)): Path<(String, String)>,
    Json(req): Json<UpdateLogDrainRequest>,
) -> Result<Json<LogDrainResponse>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&drain_id, "drain_id") {
        return Err(ApiError::validation_field("drain_id", e));
    }

    // Verify drain exists and belongs to the app
    let drain = sqlx::query_as::<_, LogDrain>(
        "SELECT * FROM log_drains WHERE id = ? AND app_id = ?",
    )
    .bind(&drain_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Log drain not found"))?;

    // Build update query
    let now = chrono::Utc::now().to_rfc3339();

    let name = req.name.unwrap_or(drain.name);
    if name.trim().is_empty() {
        return Err(ApiError::validation_field("name", "Name is required"));
    }

    let config_json = if let Some(config) = &req.config {
        // Validate config if being updated
        let provider: LogDrainProvider = drain
            .provider
            .parse()
            .map_err(|e: String| ApiError::validation_field("provider", e))?;
        validate_drain_config(&provider, config)?;
        serde_json::to_string(config)
            .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?
    } else {
        drain.config
    };

    let enabled = req.enabled.map(|e| if e { 1 } else { 0 }).unwrap_or(drain.enabled);

    sqlx::query(
        "UPDATE log_drains SET name = ?, config = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&config_json)
    .bind(enabled)
    .bind(&now)
    .bind(&drain_id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_as::<_, LogDrain>("SELECT * FROM log_drains WHERE id = ?")
        .bind(&drain_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(updated.into()))
}

/// Delete a log drain
pub async fn delete_log_drain(
    State(state): State<Arc<AppState>>,
    Path((app_id, drain_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&drain_id, "drain_id") {
        return Err(ApiError::validation_field("drain_id", e));
    }

    let result = sqlx::query("DELETE FROM log_drains WHERE id = ? AND app_id = ?")
        .bind(&drain_id)
        .bind(&app_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Log drain not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Test a log drain by sending a test log entry
pub async fn test_log_drain(
    State(state): State<Arc<AppState>>,
    Path((app_id, drain_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&drain_id, "drain_id") {
        return Err(ApiError::validation_field("drain_id", e));
    }

    let drain = sqlx::query_as::<_, LogDrain>(
        "SELECT * FROM log_drains WHERE id = ? AND app_id = ?",
    )
    .bind(&drain_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Log drain not found"))?;

    let manager = LogDrainManager::new(state.db.clone());

    match manager.send_test(&drain).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Test log entry sent successfully"
        }))),
        Err(e) => {
            // Update error info in database
            let now = chrono::Utc::now().to_rfc3339();
            let error_msg = e.to_string();
            let _ = sqlx::query(
                "UPDATE log_drains SET error_count = error_count + 1, last_error = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&error_msg)
            .bind(&now)
            .bind(&drain_id)
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({
                "success": false,
                "message": format!("Test failed: {}", error_msg)
            })))
        }
    }
}

/// Validate the configuration based on the provider type
fn validate_drain_config(
    provider: &LogDrainProvider,
    config: &serde_json::Value,
) -> Result<(), ApiError> {
    match provider {
        LogDrainProvider::Axiom => {
            let obj = config
                .as_object()
                .ok_or_else(|| ApiError::validation_field("config", "Config must be an object"))?;
            if !obj.contains_key("dataset") || !obj.contains_key("api_token") {
                return Err(ApiError::validation_field(
                    "config",
                    "Axiom config requires 'dataset' and 'api_token' fields",
                ));
            }
        }
        LogDrainProvider::NewRelic => {
            let obj = config
                .as_object()
                .ok_or_else(|| ApiError::validation_field("config", "Config must be an object"))?;
            if !obj.contains_key("api_key") {
                return Err(ApiError::validation_field(
                    "config",
                    "New Relic config requires 'api_key' field",
                ));
            }
        }
        LogDrainProvider::Datadog => {
            let obj = config
                .as_object()
                .ok_or_else(|| ApiError::validation_field("config", "Config must be an object"))?;
            if !obj.contains_key("api_key") {
                return Err(ApiError::validation_field(
                    "config",
                    "Datadog config requires 'api_key' field",
                ));
            }
        }
        LogDrainProvider::Logtail => {
            let obj = config
                .as_object()
                .ok_or_else(|| ApiError::validation_field("config", "Config must be an object"))?;
            if !obj.contains_key("source_token") {
                return Err(ApiError::validation_field(
                    "config",
                    "Logtail config requires 'source_token' field",
                ));
            }
        }
        LogDrainProvider::Http => {
            let obj = config
                .as_object()
                .ok_or_else(|| ApiError::validation_field("config", "Config must be an object"))?;
            if !obj.contains_key("url") {
                return Err(ApiError::validation_field(
                    "config",
                    "HTTP drain config requires 'url' field",
                ));
            }
        }
    }

    Ok(())
}
