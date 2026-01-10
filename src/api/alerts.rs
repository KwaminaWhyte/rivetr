//! Alert configuration API endpoints.
//!
//! Provides endpoints for managing per-app alert configurations
//! and global alert defaults.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::db::{
    AlertConfig, AlertConfigResponse, CreateAlertConfigRequest, GlobalAlertDefault,
    GlobalAlertDefaultsResponse, UpdateAlertConfigRequest, UpdateGlobalAlertDefaultsRequest,
};
use crate::AppState;

/// Validate metric type
fn is_valid_metric_type(metric_type: &str) -> bool {
    matches!(
        metric_type.to_lowercase().as_str(),
        "cpu" | "memory" | "disk"
    )
}

/// Validate threshold percent
fn is_valid_threshold(threshold: f64) -> bool {
    threshold > 0.0 && threshold <= 100.0
}

/// List all alert configurations for an app
///
/// GET /api/apps/:id/alerts
pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<AlertConfigResponse>>, StatusCode> {
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

    let alerts = AlertConfig::list_for_app(&state.db, &app_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list alerts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let responses: Vec<AlertConfigResponse> = alerts.into_iter().map(|a| a.into()).collect();

    Ok(Json(responses))
}

/// Get a specific alert configuration
///
/// GET /api/apps/:id/alerts/:alert_id
pub async fn get_alert(
    State(state): State<Arc<AppState>>,
    Path((app_id, alert_id)): Path<(String, String)>,
) -> Result<Json<AlertConfigResponse>, StatusCode> {
    let alert = AlertConfig::get_by_id(&state.db, &alert_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify the alert belongs to this app
    if alert.app_id.as_deref() != Some(app_id.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(alert.into()))
}

/// Create a new alert configuration for an app
///
/// POST /api/apps/:id/alerts
pub async fn create_alert(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateAlertConfigRequest>,
) -> Result<(StatusCode, Json<AlertConfigResponse>), StatusCode> {
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

    // Validate metric type
    let metric_type = req.metric_type.to_lowercase();
    if !is_valid_metric_type(&metric_type) {
        tracing::warn!("Invalid metric type: {}", req.metric_type);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate threshold
    if !is_valid_threshold(req.threshold_percent) {
        tracing::warn!("Invalid threshold: {}", req.threshold_percent);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if alert already exists for this app and metric type
    let existing = AlertConfig::get_for_app_metric(&state.db, &app_id, &metric_type)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check existing alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if existing.is_some() {
        tracing::warn!(
            "Alert already exists for app {} and metric {}",
            app_id,
            metric_type
        );
        return Err(StatusCode::CONFLICT);
    }

    let alert = AlertConfig::create(
        &state.db,
        &app_id,
        &metric_type,
        req.threshold_percent,
        req.enabled,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create alert: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!(
        app_id = %app_id,
        metric_type = %metric_type,
        threshold = %req.threshold_percent,
        "Created alert configuration"
    );

    Ok((StatusCode::CREATED, Json(alert.into())))
}

/// Update an alert configuration
///
/// PUT /api/apps/:id/alerts/:alert_id
pub async fn update_alert(
    State(state): State<Arc<AppState>>,
    Path((app_id, alert_id)): Path<(String, String)>,
    Json(req): Json<UpdateAlertConfigRequest>,
) -> Result<Json<AlertConfigResponse>, StatusCode> {
    // Get existing alert
    let existing = AlertConfig::get_by_id(&state.db, &alert_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify the alert belongs to this app
    if existing.app_id.as_deref() != Some(app_id.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Validate threshold if provided
    if let Some(threshold) = req.threshold_percent {
        if !is_valid_threshold(threshold) {
            tracing::warn!("Invalid threshold: {}", threshold);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let updated = AlertConfig::update(&state.db, &alert_id, req.threshold_percent, req.enabled)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        app_id = %app_id,
        alert_id = %alert_id,
        "Updated alert configuration"
    );

    Ok(Json(updated.into()))
}

/// Delete an alert configuration
///
/// DELETE /api/apps/:id/alerts/:alert_id
pub async fn delete_alert(
    State(state): State<Arc<AppState>>,
    Path((app_id, alert_id)): Path<(String, String)>,
) -> Result<StatusCode, StatusCode> {
    // Get existing alert
    let existing = AlertConfig::get_by_id(&state.db, &alert_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify the alert belongs to this app
    if existing.app_id.as_deref() != Some(app_id.as_str()) {
        return Err(StatusCode::NOT_FOUND);
    }

    let deleted = AlertConfig::delete(&state.db, &alert_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!(
        app_id = %app_id,
        alert_id = %alert_id,
        "Deleted alert configuration"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Get global alert defaults
///
/// GET /api/settings/alert-defaults
pub async fn get_alert_defaults(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GlobalAlertDefaultsResponse>, StatusCode> {
    let defaults = GlobalAlertDefault::get_all_as_response(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alert defaults: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(defaults))
}

/// Update global alert defaults
///
/// PUT /api/settings/alert-defaults
pub async fn update_alert_defaults(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateGlobalAlertDefaultsRequest>,
) -> Result<Json<GlobalAlertDefaultsResponse>, StatusCode> {
    // Validate thresholds if provided
    if let Some(cpu) = &req.cpu {
        if let Some(threshold) = cpu.threshold_percent {
            if !is_valid_threshold(threshold) {
                tracing::warn!("Invalid CPU threshold: {}", threshold);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    if let Some(memory) = &req.memory {
        if let Some(threshold) = memory.threshold_percent {
            if !is_valid_threshold(threshold) {
                tracing::warn!("Invalid memory threshold: {}", threshold);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    if let Some(disk) = &req.disk {
        if let Some(threshold) = disk.threshold_percent {
            if !is_valid_threshold(threshold) {
                tracing::warn!("Invalid disk threshold: {}", threshold);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    let defaults = GlobalAlertDefault::update_all(&state.db, &req)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update alert defaults: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Updated global alert defaults");

    Ok(Json(defaults))
}
