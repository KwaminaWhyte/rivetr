//! Cost rates API endpoints.
//!
//! Provides endpoints for managing cost rate configurations
//! used for resource cost estimation.

use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::db::{CostRate, CostRatesResponse, UpdateCostRatesRequest};
use crate::AppState;

/// Validate rate per unit
fn is_valid_rate(rate: f64) -> bool {
    rate >= 0.0
}

/// Get all cost rates
///
/// GET /api/settings/cost-rates
pub async fn get_cost_rates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CostRatesResponse>, StatusCode> {
    let rates = CostRate::get_all_as_response(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get cost rates: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(rates))
}

/// Update cost rates
///
/// PUT /api/settings/cost-rates
pub async fn update_cost_rates(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateCostRatesRequest>,
) -> Result<Json<CostRatesResponse>, StatusCode> {
    // Validate rates if provided
    if let Some(cpu) = &req.cpu {
        if let Some(rate) = cpu.rate_per_unit {
            if !is_valid_rate(rate) {
                tracing::warn!("Invalid CPU rate: {}", rate);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    if let Some(memory) = &req.memory {
        if let Some(rate) = memory.rate_per_unit {
            if !is_valid_rate(rate) {
                tracing::warn!("Invalid memory rate: {}", rate);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    if let Some(disk) = &req.disk {
        if let Some(rate) = disk.rate_per_unit {
            if !is_valid_rate(rate) {
                tracing::warn!("Invalid disk rate: {}", rate);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    let rates = CostRate::update_all(&state.db, &req).await.map_err(|e| {
        tracing::error!("Failed to update cost rates: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Updated cost rates");

    Ok(Json(rates))
}
