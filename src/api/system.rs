//! System-level API endpoints for dashboard statistics.
//!
//! Provides aggregate system stats and recent events for the dashboard.

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::db::{App, Deployment};
use crate::AppState;

use super::error::ApiError;

/// System-wide statistics for the dashboard
#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    /// Number of apps with a running deployment
    pub running_apps_count: u32,
    /// Total number of apps
    pub total_apps_count: u32,
    /// Aggregate CPU usage percentage across all running containers
    pub total_cpu_percent: f64,
    /// Aggregate memory usage in bytes across all running containers
    pub memory_used_bytes: u64,
    /// Total memory limit in bytes (sum of all container limits, or system memory if unlimited)
    pub memory_total_bytes: u64,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Uptime percentage based on successful health checks (simplified: 99.99% default)
    pub uptime_percent: f64,
}

/// A recent event (deployment, failure, restart, etc.)
#[derive(Debug, Clone, Serialize)]
pub struct RecentEvent {
    /// Unique event ID (deployment ID)
    pub id: String,
    /// App name this event is associated with
    pub app_name: String,
    /// App ID
    pub app_id: String,
    /// Type of event: "deployed", "failed", "building", "pending", "stopped"
    pub event_type: String,
    /// Event status for display: "success", "error", "warning", "info"
    pub status: String,
    /// Human-readable message
    pub message: String,
    /// When the event occurred (ISO 8601 timestamp)
    pub timestamp: String,
}

/// Get system-wide statistics
/// GET /api/system/stats
pub async fn get_system_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemStats>, ApiError> {
    // Get total apps count
    let apps: Vec<App> = sqlx::query_as("SELECT * FROM apps")
        .fetch_all(&state.db)
        .await?;

    let total_apps_count = apps.len() as u32;

    // Get running deployments (one per app, most recent with status = 'running')
    let running_deployments: Vec<Deployment> = sqlx::query_as(
        r#"
        SELECT d.* FROM deployments d
        INNER JOIN (
            SELECT app_id, MAX(started_at) as max_started
            FROM deployments
            WHERE status = 'running'
            GROUP BY app_id
        ) latest ON d.app_id = latest.app_id AND d.started_at = latest.max_started
        WHERE d.status = 'running'
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let running_apps_count = running_deployments.len() as u32;

    // Aggregate container stats for running apps
    let mut total_cpu_percent = 0.0;
    let mut memory_used_bytes: u64 = 0;
    let mut memory_total_bytes: u64 = 0;

    for deployment in &running_deployments {
        if let Some(container_id) = &deployment.container_id {
            match state.runtime.stats(container_id).await {
                Ok(stats) => {
                    total_cpu_percent += stats.cpu_percent;
                    memory_used_bytes += stats.memory_usage;
                    memory_total_bytes += stats.memory_limit;
                }
                Err(e) => {
                    tracing::debug!(
                        "Could not get stats for container {}: {}",
                        container_id,
                        e
                    );
                }
            }
        }
    }

    // Calculate server uptime using std::time
    // For now, use a static uptime value based on process start
    // In a production system, this would be tracked from server boot time
    static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = START_TIME.get_or_init(std::time::Instant::now);
    let uptime_seconds = start.elapsed().as_secs();

    // Uptime percentage - in a real system, track health check success rate
    // For now, return a high value as placeholder
    let uptime_percent = 99.99;

    Ok(Json(SystemStats {
        running_apps_count,
        total_apps_count,
        total_cpu_percent,
        memory_used_bytes,
        memory_total_bytes,
        uptime_seconds,
        uptime_percent,
    }))
}

/// Row type for joining deployment with app name
#[derive(Debug, sqlx::FromRow)]
struct DeploymentWithApp {
    // Deployment fields
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    #[allow(dead_code)]
    pub container_id: Option<String>,
    pub error_message: Option<String>,
    #[allow(dead_code)]
    pub commit_sha: Option<String>,
    #[allow(dead_code)]
    pub commit_message: Option<String>,
    #[allow(dead_code)]
    pub image_tag: Option<String>,
    // App name
    pub app_name: String,
}

/// Get recent events across all apps
/// GET /api/events/recent
pub async fn get_recent_events(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<RecentEvent>>, ApiError> {
    // Get recent deployments with app names
    let deployments: Vec<DeploymentWithApp> = sqlx::query_as(
        r#"
        SELECT
            d.id, d.app_id, d.status, d.started_at, d.finished_at,
            d.container_id, d.error_message, d.commit_sha, d.commit_message,
            d.image_tag,
            a.name as app_name
        FROM deployments d
        INNER JOIN apps a ON d.app_id = a.id
        ORDER BY d.started_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let events: Vec<RecentEvent> = deployments
        .into_iter()
        .map(|d| {
            let (event_type, status, message) = match d.status.as_str() {
                "running" => (
                    "deployed".to_string(),
                    "success".to_string(),
                    format!("{} deployed successfully", d.app_name),
                ),
                "failed" => (
                    "failed".to_string(),
                    "error".to_string(),
                    format!(
                        "{} deployment failed: {}",
                        d.app_name,
                        d.error_message.as_deref().unwrap_or("Unknown error")
                    ),
                ),
                "building" => (
                    "building".to_string(),
                    "info".to_string(),
                    format!("{} is building", d.app_name),
                ),
                "pending" | "cloning" => (
                    "pending".to_string(),
                    "info".to_string(),
                    format!("{} deployment pending", d.app_name),
                ),
                "starting" | "checking" => (
                    "starting".to_string(),
                    "info".to_string(),
                    format!("{} is starting", d.app_name),
                ),
                "stopped" => (
                    "stopped".to_string(),
                    "warning".to_string(),
                    format!("{} was stopped", d.app_name),
                ),
                _ => (
                    "unknown".to_string(),
                    "info".to_string(),
                    format!("{} status: {}", d.app_name, d.status),
                ),
            };

            RecentEvent {
                id: d.id.clone(),
                app_name: d.app_name,
                app_id: d.app_id.clone(),
                event_type,
                status,
                message,
                timestamp: d.finished_at.unwrap_or(d.started_at),
            }
        })
        .collect();

    Ok(Json(events))
}
