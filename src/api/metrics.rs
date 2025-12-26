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
