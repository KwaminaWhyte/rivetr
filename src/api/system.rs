//! System-level API endpoints for dashboard statistics.
//!
//! Provides aggregate system stats, disk stats, and recent events for the dashboard.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{App, Deployment, ManagedDatabase, Service};
use crate::engine::get_current_disk_stats;
use crate::startup::{get_system_health, SystemHealthStatus};
use crate::AppState;

use super::error::ApiError;

/// System-wide statistics for the dashboard
#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    /// Number of apps with a running deployment
    pub running_apps_count: u32,
    /// Total number of apps
    pub total_apps_count: u32,
    /// Number of running databases
    pub running_databases_count: u32,
    /// Total number of databases
    pub total_databases_count: u32,
    /// Number of running services (Docker Compose)
    pub running_services_count: u32,
    /// Total number of services
    pub total_services_count: u32,
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

/// Query parameters for system stats
#[derive(Debug, Clone, Deserialize)]
pub struct SystemStatsQuery {
    /// Optional team ID to filter stats by team
    pub team_id: Option<String>,
}

/// Get system-wide statistics
/// GET /api/system/stats
///
/// Query parameters:
/// - team_id: Optional team ID to filter stats by team scope
pub async fn get_system_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SystemStatsQuery>,
) -> Result<Json<SystemStats>, ApiError> {
    // Get apps - optionally filtered by team
    let apps: Vec<App> = match &query.team_id {
        Some(team_id) if !team_id.is_empty() => {
            sqlx::query_as("SELECT * FROM apps WHERE team_id = ?")
                .bind(team_id)
                .fetch_all(&state.db)
                .await?
        }
        _ => {
            sqlx::query_as("SELECT * FROM apps")
                .fetch_all(&state.db)
                .await?
        }
    };

    let total_apps_count = apps.len() as u32;
    let app_ids: Vec<&str> = apps.iter().map(|a| a.id.as_str()).collect();

    // Get running deployments (one per app, most recent with status = 'running')
    // Filter by app_ids when team scoping is active
    let running_deployments: Vec<Deployment> = if app_ids.is_empty() && query.team_id.is_some() {
        // No apps in this team, so no running deployments
        vec![]
    } else if let Some(team_id) = &query.team_id {
        if team_id.is_empty() {
            // System-wide stats
            sqlx::query_as(
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
            .await?
        } else {
            // Team-scoped stats - get deployments for team's apps
            sqlx::query_as(
                r#"
                SELECT d.* FROM deployments d
                INNER JOIN apps a ON d.app_id = a.id
                INNER JOIN (
                    SELECT app_id, MAX(started_at) as max_started
                    FROM deployments
                    WHERE status = 'running'
                    GROUP BY app_id
                ) latest ON d.app_id = latest.app_id AND d.started_at = latest.max_started
                WHERE d.status = 'running' AND a.team_id = ?
                "#,
            )
            .bind(team_id)
            .fetch_all(&state.db)
            .await?
        }
    } else {
        // No team filter - system-wide stats
        sqlx::query_as(
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
        .await?
    };

    let running_apps_count = running_deployments.len() as u32;

    // Get databases count - optionally filtered by team
    let databases: Vec<ManagedDatabase> = match &query.team_id {
        Some(team_id) if !team_id.is_empty() => {
            sqlx::query_as("SELECT * FROM databases WHERE team_id = ?")
                .bind(team_id)
                .fetch_all(&state.db)
                .await?
        }
        _ => {
            sqlx::query_as("SELECT * FROM databases")
                .fetch_all(&state.db)
                .await?
        }
    };

    let total_databases_count = databases.len() as u32;
    let running_databases: Vec<&ManagedDatabase> = databases
        .iter()
        .filter(|db| db.status == "running")
        .collect();
    let running_databases_count = running_databases.len() as u32;

    // Get services count - optionally filtered by team
    let services: Vec<Service> = match &query.team_id {
        Some(team_id) if !team_id.is_empty() => {
            sqlx::query_as("SELECT * FROM services WHERE team_id = ?")
                .bind(team_id)
                .fetch_all(&state.db)
                .await?
        }
        _ => {
            sqlx::query_as("SELECT * FROM services")
                .fetch_all(&state.db)
                .await?
        }
    };

    let total_services_count = services.len() as u32;

    // Aggregate container stats for running apps and databases
    let mut total_cpu_percent = 0.0;
    let mut memory_used_bytes: u64 = 0;
    let mut memory_total_bytes: u64 = 0;
    let mut has_unlimited_container = false;

    // Stats from running app deployments
    for deployment in &running_deployments {
        if let Some(container_id) = &deployment.container_id {
            match state.runtime.stats(container_id).await {
                Ok(stats) => {
                    total_cpu_percent += stats.cpu_percent;
                    memory_used_bytes += stats.memory_usage;
                    // memory_limit of 0 means unlimited (use system memory)
                    if stats.memory_limit == 0 {
                        has_unlimited_container = true;
                    } else {
                        memory_total_bytes += stats.memory_limit;
                    }
                }
                Err(e) => {
                    tracing::debug!("Could not get stats for container {}: {}", container_id, e);
                }
            }
        }
    }

    // Stats from running databases
    for database in &running_databases {
        if let Some(container_id) = &database.container_id {
            match state.runtime.stats(container_id).await {
                Ok(stats) => {
                    total_cpu_percent += stats.cpu_percent;
                    memory_used_bytes += stats.memory_usage;
                    if stats.memory_limit == 0 {
                        has_unlimited_container = true;
                    } else {
                        memory_total_bytes += stats.memory_limit;
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "Could not get stats for database container {}: {}",
                        container_id,
                        e
                    );
                }
            }
        }
    }

    // Stats from running services (Docker Compose)
    let running_services: Vec<&Service> =
        services.iter().filter(|s| s.status == "running").collect();

    for service in &running_services {
        // Get compose project name for this service (e.g., "rivetr-svc-myservice")
        let project_name = service.compose_project_name();

        // List all containers for this compose project by label
        match state.runtime.list_compose_containers(&project_name).await {
            Ok(containers) => {
                for container in containers {
                    match state.runtime.stats(&container.id).await {
                        Ok(stats) => {
                            total_cpu_percent += stats.cpu_percent;
                            memory_used_bytes += stats.memory_usage;
                            if stats.memory_limit == 0 {
                                has_unlimited_container = true;
                            } else {
                                memory_total_bytes += stats.memory_limit;
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Could not get stats for service container {}: {}",
                                container.name,
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    "Could not list containers for service {}: {}",
                    service.name,
                    e
                );
            }
        }
    }

    // If any container has no memory limit, use system memory as total
    if has_unlimited_container || memory_total_bytes == 0 {
        memory_total_bytes = get_system_memory();
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
        running_databases_count,
        total_databases_count,
        running_services_count: running_services.len() as u32,
        total_services_count,
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

/// Disk space statistics
#[derive(Debug, Clone, Serialize)]
pub struct DiskStatsResponse {
    /// Total disk space in bytes
    pub total_bytes: u64,
    /// Used disk space in bytes
    pub used_bytes: u64,
    /// Free disk space in bytes
    pub free_bytes: u64,
    /// Percentage of disk space used (0-100)
    pub usage_percent: f64,
    /// Path being monitored
    pub path: String,
    /// Human-readable total (e.g., "100 GB")
    pub total_human: String,
    /// Human-readable used (e.g., "80 GB")
    pub used_human: String,
    /// Human-readable free (e.g., "20 GB")
    pub free_human: String,
}

/// Get current disk space statistics
/// GET /api/system/disk
pub async fn get_disk_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DiskStatsResponse>, ApiError> {
    let data_dir = &state.config.server.data_dir;

    let stats = get_current_disk_stats(data_dir).map_err(|e| {
        tracing::error!(error = %e, "Failed to get disk stats");
        ApiError::internal(format!("Failed to get disk stats: {}", e))
    })?;

    Ok(Json(DiskStatsResponse {
        total_bytes: stats.total_bytes,
        used_bytes: stats.used_bytes,
        free_bytes: stats.free_bytes,
        usage_percent: stats.usage_percent,
        path: data_dir.display().to_string(),
        total_human: format_bytes(stats.total_bytes),
        used_human: format_bytes(stats.used_bytes),
        free_human: format_bytes(stats.free_bytes),
    }))
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get total system memory in bytes
fn get_system_memory() -> u64 {
    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;

        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
        }

        let mut status: MaybeUninit<MemoryStatusEx> = MaybeUninit::uninit();
        unsafe {
            let ptr = status.as_mut_ptr();
            (*ptr).dw_length = std::mem::size_of::<MemoryStatusEx>() as u32;
            if GlobalMemoryStatusEx(ptr) != 0 {
                return (*ptr).ull_total_phys;
            }
        }
        0
    }

    #[cfg(unix)]
    {
        use std::fs;
        // Read from /proc/meminfo on Linux
        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
        0
    }

    #[cfg(not(any(windows, unix)))]
    {
        0
    }
}

/// Get detailed system health status
/// GET /api/system/health
///
/// Returns comprehensive health information including:
/// - Database connectivity status
/// - Container runtime availability
/// - Disk space status
/// - Directory writability
/// - Individual check results
pub async fn get_detailed_health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemHealthStatus>, ApiError> {
    let health = get_system_health(&state.config, &state.db).await;
    Ok(Json(health))
}

/// Stats history data point
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct StatsHistoryPoint {
    pub timestamp: String,
    pub cpu_percent: f64,
    pub memory_used_bytes: i64,
    pub memory_total_bytes: i64,
    pub running_apps: i64,
    pub running_databases: i64,
    pub running_services: i64,
}

/// Stats history response
#[derive(Debug, Clone, Serialize)]
pub struct StatsHistoryResponse {
    /// Historical data points
    pub history: Vec<StatsHistoryPoint>,
    /// Number of data points returned
    pub count: usize,
}

/// Query parameters for stats history
#[derive(Debug, Clone, Deserialize)]
pub struct StatsHistoryQuery {
    /// Time range in hours (default: 24)
    /// Valid values: 1, 6, 24, 168 (7 days), 720 (30 days)
    #[serde(default = "default_hours")]
    pub hours: i64,
}

fn default_hours() -> i64 {
    24
}

/// Get stats history for dashboard charts
/// GET /api/system/stats/history
///
/// Returns historical system stats for the specified time range.
/// For short time ranges (<=24h), returns raw 5-minute data.
/// For longer ranges (>24h), returns aggregated hourly data for efficiency.
///
/// Query parameters:
/// - hours: Time range in hours (default: 24, valid: 1, 6, 24, 168, 720)
pub async fn get_stats_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsHistoryQuery>,
) -> Result<Json<StatsHistoryResponse>, ApiError> {
    // Validate and clamp hours to supported values
    let hours = match query.hours {
        h if h <= 1 => 1,
        h if h <= 6 => 6,
        h if h <= 24 => 24,
        h if h <= 168 => 168, // 7 days
        _ => 720,             // 30 days
    };

    // For time ranges > 24 hours, use aggregated hourly data for efficiency
    // This returns fewer data points but covers longer periods
    let history = if hours > 24 {
        get_aggregated_history(&state.db, hours).await
    } else {
        get_raw_history(&state.db, hours).await
    };

    let count = history.len();

    Ok(Json(StatsHistoryResponse { history, count }))
}

/// Get raw stats history (5-minute intervals) for short time ranges
async fn get_raw_history(db: &sqlx::SqlitePool, hours: i64) -> Vec<StatsHistoryPoint> {
    let limit = hours * 12; // 12 samples per hour at 5-min intervals
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let history: Vec<StatsHistoryPoint> = sqlx::query_as(
        r#"
        SELECT timestamp, cpu_percent, memory_used_bytes, memory_total_bytes,
               running_apps, running_databases, running_services
        FROM stats_history
        WHERE timestamp >= ?
        ORDER BY timestamp DESC
        LIMIT ?
        "#,
    )
    .bind(&cutoff_str)
    .bind(limit)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Reverse to get chronological order
    let mut history = history;
    history.reverse();
    history
}

/// Get aggregated hourly stats for longer time ranges
async fn get_aggregated_history(db: &sqlx::SqlitePool, hours: i64) -> Vec<StatsHistoryPoint> {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:00:00").to_string();

    // Try to get from hourly aggregates first
    #[derive(Debug, sqlx::FromRow)]
    struct HourlyRow {
        hour_timestamp: String,
        avg_cpu_percent: f64,
        avg_memory_used_bytes: i64,
        avg_memory_total_bytes: i64,
        avg_running_apps: f64,
        avg_running_databases: f64,
        avg_running_services: f64,
    }

    let hourly: Vec<HourlyRow> = sqlx::query_as(
        r#"
        SELECT hour_timestamp, avg_cpu_percent, avg_memory_used_bytes, avg_memory_total_bytes,
               avg_running_apps, avg_running_databases, avg_running_services
        FROM stats_hourly
        WHERE hour_timestamp >= ?
        ORDER BY hour_timestamp ASC
        "#,
    )
    .bind(&cutoff_str)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // If we have hourly data, use it
    if !hourly.is_empty() {
        return hourly
            .into_iter()
            .map(|h| StatsHistoryPoint {
                timestamp: h.hour_timestamp,
                cpu_percent: h.avg_cpu_percent,
                memory_used_bytes: h.avg_memory_used_bytes,
                memory_total_bytes: h.avg_memory_total_bytes,
                running_apps: h.avg_running_apps.round() as i64,
                running_databases: h.avg_running_databases.round() as i64,
                running_services: h.avg_running_services.round() as i64,
            })
            .collect();
    }

    // Fall back to raw data if no hourly aggregates exist
    // This can happen if the aggregation task hasn't run yet
    let raw_cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    let limit = hours * 12;

    let history: Vec<StatsHistoryPoint> = sqlx::query_as(
        r#"
        SELECT timestamp, cpu_percent, memory_used_bytes, memory_total_bytes,
               running_apps, running_databases, running_services
        FROM stats_history
        WHERE timestamp >= ?
        ORDER BY timestamp DESC
        LIMIT ?
        "#,
    )
    .bind(&raw_cutoff_str)
    .bind(limit)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut history = history;
    history.reverse();
    history
}

/// System-wide aggregated stats summary
/// GET /api/system/stats/summary
///
/// Returns a summary of system stats with different time period aggregations:
/// - Current: Most recent stats
/// - Last 24 hours: Average and max from raw data
/// - Last 7 days: Average and max from hourly aggregates
/// - Last 30 days: Average and max from daily aggregates
pub async fn get_stats_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<crate::db::SystemStatsSummary>, ApiError> {
    use crate::db::SystemStatsSummary;

    let summary = SystemStatsSummary::get(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get stats summary: {}", e)))?;

    Ok(Json(summary))
}

/// Get system version and update status
/// GET /api/system/version
///
/// Returns the current version, latest available version, and update status.
pub async fn get_version_info(
    State(state): State<Arc<AppState>>,
) -> Json<crate::engine::updater::UpdateStatus> {
    let status = state.update_checker.get_status();
    Json(status)
}

/// Check for updates
/// POST /api/system/update/check
///
/// Triggers an immediate update check and returns the result.
pub async fn check_for_updates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<crate::engine::updater::UpdateStatus>, ApiError> {
    state.update_checker.run_check().await;
    let status = state.update_checker.get_status();
    Ok(Json(status))
}

/// Download update response
#[derive(Debug, Clone, Serialize)]
pub struct DownloadUpdateResponse {
    pub success: bool,
    pub message: String,
    pub version: Option<String>,
    pub download_path: Option<String>,
}

/// Download update binary
/// POST /api/system/update/download
///
/// Downloads the latest update binary to a temporary location.
/// Does not apply the update - use /api/system/update/apply for that.
pub async fn download_update(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DownloadUpdateResponse>, ApiError> {
    // Check if update is available
    let status = state.update_checker.get_status();
    if !status.update_available {
        return Ok(Json(DownloadUpdateResponse {
            success: false,
            message: "No update available".to_string(),
            version: None,
            download_path: None,
        }));
    }

    let version = status.latest_version.clone();

    match state.update_checker.download_update().await {
        Ok(path) => Ok(Json(DownloadUpdateResponse {
            success: true,
            message: format!("Update downloaded to {}", path.display()),
            version,
            download_path: Some(path.display().to_string()),
        })),
        Err(e) => Err(ApiError::internal(format!("Failed to download update: {}", e))),
    }
}

/// Apply update response
#[derive(Debug, Clone, Serialize)]
pub struct ApplyUpdateResponse {
    pub success: bool,
    pub message: String,
    pub backup_path: Option<String>,
    pub restart_required: bool,
}

/// Apply downloaded update
/// POST /api/system/update/apply
///
/// Applies a previously downloaded update by replacing the binary.
/// Requires service restart to take effect.
pub async fn apply_update(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApplyUpdateResponse>, ApiError> {
    let temp_path = std::env::temp_dir().join("rivetr-update");

    if !temp_path.exists() {
        return Ok(Json(ApplyUpdateResponse {
            success: false,
            message: "No downloaded update found. Run download first.".to_string(),
            backup_path: None,
            restart_required: false,
        }));
    }

    match state.update_checker.apply_update(&temp_path).await {
        Ok(backup_path) => Ok(Json(ApplyUpdateResponse {
            success: true,
            message: "Update applied successfully. Service restart required.".to_string(),
            backup_path: Some(backup_path.display().to_string()),
            restart_required: true,
        })),
        Err(e) => Err(ApiError::internal(format!("Failed to apply update: {}", e))),
    }
}
