//! White label configuration API endpoints.
//!
//! GET /api/white-label  — returns the config (public, no auth required for login page)
//! PUT /api/white-label  — updates the config (admin only, behind auth middleware)

use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db::{UpdateWhiteLabelRequest, WhiteLabel};
use crate::AppState;

/// GET /api/white-label
///
/// Returns the current white label configuration.
/// This endpoint is PUBLIC (no auth) so the login page can load branding.
pub async fn get_white_label(
    State(state): State<Arc<AppState>>,
) -> Result<Json<WhiteLabel>, StatusCode> {
    let config = WhiteLabel::load(&state.db).await.map_err(|e| {
        tracing::error!("Failed to load white label config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(config))
}

/// PUT /api/white-label
///
/// Updates the white label configuration.
/// Requires authentication (placed behind the auth middleware in routes).
pub async fn update_white_label(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateWhiteLabelRequest>,
) -> Result<Json<WhiteLabel>, StatusCode> {
    let config = WhiteLabel::update(&state.db, &req).await.map_err(|e| {
        tracing::error!("Failed to update white label config: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("White label configuration updated");
    Ok(Json(config))
}
