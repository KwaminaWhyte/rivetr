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

/// Daily cost data point for trend display
#[derive(Debug, Serialize)]
pub struct DailyCostPoint {
    pub date: String,
    pub total_cost: f64,
}

/// Dashboard cost response with summary, top apps, and trend data
#[derive(Debug, Serialize)]
pub struct DashboardCostResponse {
    /// Total cost summary across all apps
    pub summary: CostSummary,
    /// Top 5 apps by cost
    pub top_apps: Vec<AppCostBreakdown>,
    /// Daily cost trend (last 30 days)
    pub trend: Vec<DailyCostPoint>,
    /// Period used for the query
    pub period: String,
    /// Number of days in the period
    pub period_days: i64,
}

/// Get dashboard cost summary (system-wide)
///
/// GET /api/system/costs?period=7d|30d|90d
///
/// Returns total cost across all apps, top 5 apps by cost, and daily trend.
pub async fn get_dashboard_costs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CostQueryParams>,
) -> Result<Json<DashboardCostResponse>, StatusCode> {
    let days = parse_period(&params.period)?;

    // Aggregate costs across all apps
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    // Get aggregated summary
    let result: Option<(f64, f64, f64, f64, f64, f64, f64, i64)> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(cpu_cost), 0),
            COALESCE(SUM(memory_cost), 0),
            COALESCE(SUM(disk_cost), 0),
            COALESCE(SUM(total_cost), 0),
            COALESCE(AVG(avg_cpu_cores), 0),
            COALESCE(AVG(avg_memory_gb), 0),
            COALESCE(AVG(avg_disk_gb), 0),
            COUNT(DISTINCT snapshot_date)
        FROM cost_snapshots
        WHERE snapshot_date >= ?
        "#,
    )
    .bind(&cutoff_str)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get cost summary: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let summary = result
        .map(
            |(cpu_cost, memory_cost, disk_cost, total_cost, avg_cpu, avg_mem, avg_disk, count)| {
                let daily_avg = if count > 0 {
                    total_cost / count as f64
                } else {
                    0.0
                };
                let projected_monthly = daily_avg * 30.0;

                CostSummary {
                    cpu_cost,
                    memory_cost,
                    disk_cost,
                    total_cost,
                    avg_cpu_cores: avg_cpu,
                    avg_memory_gb: avg_mem,
                    avg_disk_gb: avg_disk,
                    days_in_period: count,
                    projected_monthly_cost: projected_monthly,
                }
            },
        )
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

    // Get top 5 apps by cost
    let top_apps: Vec<AppCostBreakdown> = sqlx::query_as(
        r#"
        SELECT
            a.id as app_id,
            a.name as app_name,
            COALESCE(SUM(cs.cpu_cost), 0) as cpu_cost,
            COALESCE(SUM(cs.memory_cost), 0) as memory_cost,
            COALESCE(SUM(cs.disk_cost), 0) as disk_cost,
            COALESCE(SUM(cs.total_cost), 0) as total_cost
        FROM apps a
        LEFT JOIN cost_snapshots cs ON cs.app_id = a.id AND cs.snapshot_date >= ?
        GROUP BY a.id, a.name
        HAVING total_cost > 0
        ORDER BY total_cost DESC
        LIMIT 5
        "#,
    )
    .bind(&cutoff_str)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get top apps by cost: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get daily trend for last 30 days
    let trend_cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let trend_cutoff_str = trend_cutoff.format("%Y-%m-%d").to_string();

    let trend: Vec<DailyCostPoint> = sqlx::query_as::<_, (String, f64)>(
        r#"
        SELECT
            snapshot_date as date,
            COALESCE(SUM(total_cost), 0) as total_cost
        FROM cost_snapshots
        WHERE snapshot_date >= ?
        GROUP BY snapshot_date
        ORDER BY snapshot_date ASC
        "#,
    )
    .bind(&trend_cutoff_str)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get cost trend: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .into_iter()
    .map(|(date, total_cost)| DailyCostPoint { date, total_cost })
    .collect();

    Ok(Json(DashboardCostResponse {
        summary,
        top_apps,
        trend,
        period: params.period,
        period_days: days,
    }))
}
