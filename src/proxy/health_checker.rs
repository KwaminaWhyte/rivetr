// Health check service for proxy backends
//
// Periodically checks the health of all registered backends and updates
// their health status in the route table. Supports automatic recovery
// detection when previously unhealthy backends become healthy again.

use arc_swap::ArcSwap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::api::metrics::{
    record_health_check_failure, record_health_check_success, set_backend_healthy,
    set_health_check_consecutive_failures,
};

use super::RouteTable;
use crate::config::ProxyConfig;

/// Configuration for the health checker
#[derive(Debug, Clone)]
pub struct HealthCheckerConfig {
    /// Interval between health check rounds
    pub interval: Duration,
    /// Timeout for individual health check requests
    pub timeout: Duration,
    /// Number of consecutive failures before marking backend unhealthy
    pub failure_threshold: u32,
}

impl HealthCheckerConfig {
    pub fn from_proxy_config(config: &ProxyConfig) -> Self {
        Self {
            interval: Duration::from_secs(config.health_check_interval),
            timeout: Duration::from_secs(config.health_check_timeout),
            failure_threshold: config.health_check_threshold,
        }
    }
}

impl Default for HealthCheckerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
        }
    }
}

/// Health checker service that periodically checks backend health
pub struct HealthChecker {
    routes: Arc<ArcSwap<RouteTable>>,
    config: HealthCheckerConfig,
    client: reqwest::Client,
}

impl HealthChecker {
    pub fn new(routes: Arc<ArcSwap<RouteTable>>, config: HealthCheckerConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            routes,
            config,
            client,
        }
    }

    /// Start the health checker background task
    pub async fn run(self) {
        info!(
            interval_secs = self.config.interval.as_secs(),
            timeout_secs = self.config.timeout.as_secs(),
            threshold = self.config.failure_threshold,
            "Health checker started"
        );

        let mut ticker = interval(self.config.interval);

        loop {
            ticker.tick().await;
            self.check_all_backends().await;
        }
    }

    /// Check health of all registered backends
    async fn check_all_backends(&self) {
        let routes = self.routes.load();
        let backends = routes.all_backends();

        if backends.is_empty() {
            debug!("No backends to health check");
            return;
        }

        debug!(count = backends.len(), "Running health checks");

        // Run health checks concurrently for all backends
        let checks: Vec<_> = backends
            .into_iter()
            .map(|(domain, backend)| {
                let client = self.client.clone();
                let failure_threshold = self.config.failure_threshold;
                let routes = self.routes.clone();

                async move {
                    let health_url = backend.health_url();
                    let was_healthy = backend.healthy;

                    // Time the health check
                    let start = Instant::now();
                    let check_passed = match client.get(&health_url).send().await {
                        Ok(response) => {
                            let status = response.status();
                            if status.is_success() {
                                debug!(
                                    domain = %domain,
                                    url = %health_url,
                                    status = %status,
                                    "Health check passed"
                                );
                                true
                            } else {
                                debug!(
                                    domain = %domain,
                                    url = %health_url,
                                    status = %status,
                                    "Health check returned non-success status"
                                );
                                false
                            }
                        }
                        Err(e) => {
                            debug!(
                                domain = %domain,
                                url = %health_url,
                                error = %e,
                                "Health check failed"
                            );
                            false
                        }
                    };
                    let duration_secs = start.elapsed().as_secs_f64();

                    // Record health check metrics
                    if check_passed {
                        record_health_check_success(&domain, duration_secs);
                    } else {
                        record_health_check_failure(&domain, duration_secs);
                    }

                    // Update health status in route table
                    let routes_ref = routes.load();
                    let status_changed =
                        routes_ref.update_health(&domain, check_passed, failure_threshold);

                    // Get current failure count and update metrics
                    let current_failures = routes_ref
                        .get_backend(&domain)
                        .map(|b| b.failure_count)
                        .unwrap_or(0);
                    let is_healthy = routes_ref
                        .get_backend(&domain)
                        .map(|b| b.healthy)
                        .unwrap_or(false);

                    // Update gauge metrics
                    set_backend_healthy(&domain, is_healthy);
                    set_health_check_consecutive_failures(&domain, current_failures);

                    if status_changed {
                        if check_passed {
                            info!(
                                domain = %domain,
                                "Backend recovered - marked healthy"
                            );
                        } else {
                            warn!(
                                domain = %domain,
                                threshold = failure_threshold,
                                "Backend marked unhealthy after consecutive failures"
                            );
                        }
                    } else if !check_passed && was_healthy {
                        // Backend is failing but not yet unhealthy
                        debug!(
                            domain = %domain,
                            failures = current_failures,
                            threshold = failure_threshold,
                            "Backend health check failed, still within threshold"
                        );
                    }
                }
            })
            .collect();

        // Wait for all health checks to complete
        futures::future::join_all(checks).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::Backend;

    #[test]
    fn test_health_checker_config_default() {
        let config = HealthCheckerConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.failure_threshold, 3);
    }

    #[test]
    fn test_backend_health_url() {
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);
        assert_eq!(backend.health_url(), "http://127.0.0.1:3000/");

        let backend_with_path = Backend::new("container-123".into(), "127.0.0.1".into(), 3000)
            .with_healthcheck(Some("/health".to_string()));
        assert_eq!(backend_with_path.health_url(), "http://127.0.0.1:3000/health");
    }

    #[test]
    fn test_route_table_update_health() {
        let table = RouteTable::new();
        let backend = Backend::new("container-123".into(), "127.0.0.1".into(), 3000);
        table.add_route("example.com".into(), backend);

        // Initial state: healthy
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 0);

        // First failure: still healthy
        let changed = table.update_health("example.com", false, 3);
        assert!(!changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 1);

        // Second failure: still healthy
        let changed = table.update_health("example.com", false, 3);
        assert!(!changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 2);

        // Third failure: now unhealthy
        let changed = table.update_health("example.com", false, 3);
        assert!(changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(!b.healthy);
        assert_eq!(b.failure_count, 3);

        // Recovery: healthy again
        let changed = table.update_health("example.com", true, 3);
        assert!(changed);
        let b = table.get_backend("example.com").unwrap();
        assert!(b.healthy);
        assert_eq!(b.failure_count, 0);
    }
}
