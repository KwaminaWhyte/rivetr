//! Cost estimation API endpoints.
//!
//! Provides endpoints for retrieving cost data for apps, projects, and teams
//! based on resource metrics and configured rates.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{AppCostBreakdown, CostSnapshot, CostSummary};
use crate::AppState;

/// Query parameters for cost endpoints
#[derive(Debug, Deserialize)]
pub struct CostQueryParams {
    /// Period for cost calculation: 7d, 30d, or 90d (default: 30d)
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "30d".to_string()
}

/// Parse period string to days
fn parse_period(period: &str) -> Result<i64, StatusCode> {
    match period {
        "7d" => Ok(7),
        "30d" => Ok(30),
        "90d" => Ok(90),
        _ => {
            tracing::warn!("Invalid period: {}", period);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

/// Cost response with summary and optional breakdown
#[derive(Debug, Serialize)]
pub struct CostResponse {
    /// Cost summary with totals and projections
    pub summary: CostSummary,
    /// Optional breakdown by app (for project and team endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<AppCostBreakdown>>,
    /// Period used for the query
    pub period: String,
    /// Number of days in the period
    pub period_days: i64,
}

/// Get cost data for a specific app
///
/// GET /api/apps/:id/costs?period=7d|30d|90d
pub async fn get_app_costs(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Query(params): Query<CostQueryParams>,
) -> Result<Json<CostResponse>, StatusCode> {
    let days = parse_period(&params.period)?;

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

    let summary = CostSnapshot::get_summary_for_app(&state.db, &app_id, days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get app costs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_else(|| CostSummary {
            cpu_cost: 0.0,
            memory_cost: 0.0,
            disk_cost: 0.0,
            total_cost: 0.0,
            avg_cpu_cores: 0.0,
            avg_memory_gb: 0.0,
            avg_disk_gb: 0.0,
            days_in_period: 0,
            projected_monthly_cost: 0.0,
        });

    Ok(Json(CostResponse {
        summary,
        breakdown: None,
        period: params.period,
        period_days: days,
    }))
}

/// Get cost data for a project (all apps in the project)
///
/// GET /api/projects/:id/costs?period=7d|30d|90d
pub async fn get_project_costs(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Query(params): Query<CostQueryParams>,
) -> Result<Json<CostResponse>, StatusCode> {
    let days = parse_period(&params.period)?;

    // Verify project exists
    let project_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check project: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if project_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let summary = CostSnapshot::get_summary_for_project(&state.db, &project_id, days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get project costs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_else(|| CostSummary {
            cpu_cost: 0.0,
            memory_cost: 0.0,
            disk_cost: 0.0,
            total_cost: 0.0,
            avg_cpu_cores: 0.0,
            avg_memory_gb: 0.0,
            avg_disk_gb: 0.0,
            days_in_period: 0,
            projected_monthly_cost: 0.0,
        });

    let breakdown = CostSnapshot::get_breakdown_for_project(&state.db, &project_id, days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get project cost breakdown: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CostResponse {
        summary,
        breakdown: Some(breakdown),
        period: params.period,
        period_days: days,
    }))
}

/// Get cost data for a team (all apps owned by the team)
///
/// GET /api/teams/:id/costs?period=7d|30d|90d
pub async fn get_team_costs(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Query(params): Query<CostQueryParams>,
) -> Result<Json<CostResponse>, StatusCode> {
    let days = parse_period(&params.period)?;

    // Verify team exists
    let team_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check team: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if team_exists == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let summary = CostSnapshot::get_summary_for_team(&state.db, &team_id, days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get team costs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_else(|| CostSummary {
            cpu_cost: 0.0,
            memory_cost: 0.0,
            disk_cost: 0.0,
            total_cost: 0.0,
            avg_cpu_cores: 0.0,
            avg_memory_gb: 0.0,
            avg_disk_gb: 0.0,
            days_in_period: 0,
            projected_monthly_cost: 0.0,
        });

    let breakdown = CostSnapshot::get_breakdown_for_team(&state.db, &team_id, days)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get team cost breakdown: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CostResponse {
        summary,
        breakdown: Some(breakdown),
        period: params.period,
        period_days: days,
    }))
}
