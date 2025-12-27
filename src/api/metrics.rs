//! Prometheus metrics endpoint and HTTP request tracking middleware.
//!
//! This module provides:
//! - A `/metrics` endpoint that returns Prometheus-formatted metrics
//! - Middleware for tracking HTTP request counts and durations
//! - Helper functions to record deployment and app metrics

use axum::{
    body::Body,
    extract::{MatchedPath, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use std::time::Instant;

use crate::AppState;

// Metric names as constants for consistency
pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";
pub const DEPLOYMENTS_TOTAL: &str = "deployments_total";
pub const APPS_TOTAL: &str = "apps_total";
pub const CONTAINERS_RUNNING: &str = "containers_running";

// Disk space metrics
pub const DISK_TOTAL_BYTES: &str = "rivetr_disk_total_bytes";
pub const DISK_USED_BYTES: &str = "rivetr_disk_used_bytes";
pub const DISK_FREE_BYTES: &str = "rivetr_disk_free_bytes";
pub const DISK_USAGE_PERCENT: &str = "rivetr_disk_usage_percent";

// Health check metrics
pub const HEALTH_CHECK_TOTAL: &str = "rivetr_health_check_total";
pub const HEALTH_CHECK_DURATION_SECONDS: &str = "rivetr_health_check_duration_seconds";
pub const BACKEND_HEALTHY: &str = "rivetr_backend_healthy";
pub const HEALTH_CHECK_CONSECUTIVE_FAILURES: &str = "rivetr_health_check_consecutive_failures";

// Container resource metrics
pub const CONTAINER_CPU_PERCENT: &str = "rivetr_container_cpu_percent";
pub const CONTAINER_MEMORY_BYTES: &str = "rivetr_container_memory_bytes";
pub const CONTAINER_MEMORY_LIMIT_BYTES: &str = "rivetr_container_memory_limit_bytes";
pub const CONTAINER_NETWORK_RX_BYTES: &str = "rivetr_container_network_rx_bytes";
pub const CONTAINER_NETWORK_TX_BYTES: &str = "rivetr_container_network_tx_bytes";

// Container restart metrics
pub const CONTAINER_RESTARTS_TOTAL: &str = "rivetr_container_restarts_total";
pub const CONTAINER_RESTART_BACKOFF_SECONDS: &str = "rivetr_container_restart_backoff_seconds";

/// Initialize the Prometheus metrics recorder and return a handle for rendering metrics.
///
/// This should be called once during application startup.
pub fn init_metrics() -> PrometheusHandle {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    // Register metric descriptions
    describe_counter!(
        HTTP_REQUESTS_TOTAL,
        "Total number of HTTP requests received"
    );
    describe_histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        "HTTP request duration in seconds"
    );
    describe_counter!(
        DEPLOYMENTS_TOTAL,
        "Total number of deployments by status (success/failed)"
    );
    describe_gauge!(APPS_TOTAL, "Total number of registered applications");
    describe_gauge!(CONTAINERS_RUNNING, "Number of currently running containers");

    // Disk space metrics
    describe_gauge!(DISK_TOTAL_BYTES, "Total disk space in bytes");
    describe_gauge!(DISK_USED_BYTES, "Used disk space in bytes");
    describe_gauge!(DISK_FREE_BYTES, "Free disk space in bytes");
    describe_gauge!(DISK_USAGE_PERCENT, "Disk usage percentage (0-100)");

    // Health check metrics
    describe_counter!(
        HEALTH_CHECK_TOTAL,
        "Total number of health checks by domain and result"
    );
    describe_histogram!(
        HEALTH_CHECK_DURATION_SECONDS,
        "Health check duration in seconds"
    );
    describe_gauge!(
        BACKEND_HEALTHY,
        "Backend health status (1 for healthy, 0 for unhealthy)"
    );
    describe_gauge!(
        HEALTH_CHECK_CONSECUTIVE_FAILURES,
        "Number of consecutive health check failures"
    );

    // Container resource metrics
    describe_gauge!(
        CONTAINER_CPU_PERCENT,
        "Container CPU usage percentage (labeled by app_name)"
    );
    describe_gauge!(
        CONTAINER_MEMORY_BYTES,
        "Container memory usage in bytes (labeled by app_name)"
    );
    describe_gauge!(
        CONTAINER_MEMORY_LIMIT_BYTES,
        "Container memory limit in bytes (labeled by app_name)"
    );
    describe_gauge!(
        CONTAINER_NETWORK_RX_BYTES,
        "Container network bytes received (labeled by app_name)"
    );
    describe_gauge!(
        CONTAINER_NETWORK_TX_BYTES,
        "Container network bytes transmitted (labeled by app_name)"
    );

    // Container restart metrics
    describe_counter!(
        CONTAINER_RESTARTS_TOTAL,
        "Total number of container restarts (labeled by app_name)"
    );
    describe_gauge!(
        CONTAINER_RESTART_BACKOFF_SECONDS,
        "Current restart backoff delay in seconds (labeled by app_name)"
    );

    handle
}

/// GET /metrics - Returns Prometheus-formatted metrics.
///
/// This endpoint is accessible without authentication.
pub async fn metrics_endpoint(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Update gauge metrics before rendering
    update_gauge_metrics(&state).await;

    // Render metrics in Prometheus text format
    let handle = state.metrics_handle.as_ref();
    match handle {
        Some(h) => (StatusCode::OK, h.render()),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Metrics not initialized".to_string(),
        ),
    }
}

/// Update gauge metrics (apps_total, containers_running) from current state.
async fn update_gauge_metrics(state: &AppState) {
    // Count total apps
    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM apps")
        .fetch_one(&state.db)
        .await
    {
        gauge!(APPS_TOTAL).set(count as f64);
    }

    // Count running containers
    if let Ok(containers) = state.runtime.list_containers("rivetr-").await {
        let running_count = containers
            .iter()
            .filter(|c| c.status.to_lowercase().contains("running"))
            .count();
        gauge!(CONTAINERS_RUNNING).set(running_count as f64);
    }
}

/// Middleware to track HTTP request metrics.
///
/// Records:
/// - `http_requests_total` counter with method, path, and status labels
/// - `http_request_duration_seconds` histogram with method and path labels
pub async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    // Extract path pattern (use matched path for templates like /apps/:id)
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    let method = request.method().to_string();

    // Process the request
    let response = next.run(request).await;

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    counter!(HTTP_REQUESTS_TOTAL, "method" => method.clone(), "path" => path.clone(), "status" => status).increment(1);
    histogram!(HTTP_REQUEST_DURATION_SECONDS, "method" => method, "path" => path).record(duration);

    response
}

/// Record a successful deployment.
pub fn record_deployment_success() {
    counter!(DEPLOYMENTS_TOTAL, "status" => "success").increment(1);
}

/// Record a failed deployment.
pub fn record_deployment_failed() {
    counter!(DEPLOYMENTS_TOTAL, "status" => "failed").increment(1);
}

/// Record a successful health check.
pub fn record_health_check_success(domain: &str, duration_secs: f64) {
    counter!(HEALTH_CHECK_TOTAL, "domain" => domain.to_string(), "result" => "success").increment(1);
    histogram!(HEALTH_CHECK_DURATION_SECONDS, "domain" => domain.to_string()).record(duration_secs);
}

/// Record a failed health check.
pub fn record_health_check_failure(domain: &str, duration_secs: f64) {
    counter!(HEALTH_CHECK_TOTAL, "domain" => domain.to_string(), "result" => "failure").increment(1);
    histogram!(HEALTH_CHECK_DURATION_SECONDS, "domain" => domain.to_string()).record(duration_secs);
}

/// Update the backend healthy gauge.
pub fn set_backend_healthy(domain: &str, healthy: bool) {
    gauge!(BACKEND_HEALTHY, "domain" => domain.to_string()).set(if healthy { 1.0 } else { 0.0 });
}

/// Update the consecutive failures gauge.
pub fn set_health_check_consecutive_failures(domain: &str, failures: u32) {
    gauge!(HEALTH_CHECK_CONSECUTIVE_FAILURES, "domain" => domain.to_string()).set(failures as f64);
}

/// Update container CPU usage metric.
pub fn set_container_cpu_percent(app_name: &str, cpu_percent: f64) {
    gauge!(CONTAINER_CPU_PERCENT, "app_name" => app_name.to_string()).set(cpu_percent);
}

/// Update container memory usage metric.
pub fn set_container_memory_bytes(app_name: &str, memory_bytes: u64) {
    gauge!(CONTAINER_MEMORY_BYTES, "app_name" => app_name.to_string()).set(memory_bytes as f64);
}

/// Update container memory limit metric.
pub fn set_container_memory_limit_bytes(app_name: &str, memory_limit_bytes: u64) {
    gauge!(CONTAINER_MEMORY_LIMIT_BYTES, "app_name" => app_name.to_string()).set(memory_limit_bytes as f64);
}

/// Update container network RX bytes metric.
pub fn set_container_network_rx_bytes(app_name: &str, rx_bytes: u64) {
    gauge!(CONTAINER_NETWORK_RX_BYTES, "app_name" => app_name.to_string()).set(rx_bytes as f64);
}

/// Update container network TX bytes metric.
pub fn set_container_network_tx_bytes(app_name: &str, tx_bytes: u64) {
    gauge!(CONTAINER_NETWORK_TX_BYTES, "app_name" => app_name.to_string()).set(tx_bytes as f64);
}

/// Increment container restart counter.
pub fn increment_container_restarts(app_name: &str) {
    counter!(CONTAINER_RESTARTS_TOTAL, "app_name" => app_name.to_string()).increment(1);
}

/// Update container restart backoff delay metric.
pub fn set_container_restart_backoff_seconds(app_name: &str, backoff_secs: f64) {
    gauge!(CONTAINER_RESTART_BACKOFF_SECONDS, "app_name" => app_name.to_string()).set(backoff_secs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_names() {
        // Ensure metric names follow Prometheus naming conventions
        assert!(HTTP_REQUESTS_TOTAL.contains("_total"));
        assert!(DEPLOYMENTS_TOTAL.contains("_total"));
        assert!(HTTP_REQUEST_DURATION_SECONDS.contains("_seconds"));
    }
}
