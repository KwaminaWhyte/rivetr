//! Instance settings API endpoints.
//!
//! Provides GET and PUT endpoints for instance-level configuration such as
//! the instance domain and instance name.

use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db::{InstanceSettings, UpdateInstanceSettingsRequest};
use crate::AppState;

/// Get instance settings.
///
/// GET /api/settings/instance
pub async fn get_instance_settings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InstanceSettings>, StatusCode> {
    let settings = InstanceSettings::load(&state.db).await.map_err(|e| {
        tracing::error!("Failed to load instance settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(settings))
}

/// Update instance settings.
///
/// PUT /api/settings/instance
pub async fn update_instance_settings(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateInstanceSettingsRequest>,
) -> Result<Json<InstanceSettings>, StatusCode> {
    let settings = InstanceSettings::update(&state.db, &req)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update instance settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Updated instance settings");

    Ok(Json(settings))
}
