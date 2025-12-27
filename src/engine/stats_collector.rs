//! Container stats collector module
//!
//! This module periodically collects resource statistics (CPU, memory, network)
//! from all running containers and updates Prometheus metrics with labels for
//! each app.

use crate::api::metrics::{
    set_container_cpu_percent, set_container_memory_bytes, set_container_memory_limit_bytes,
    set_container_network_rx_bytes, set_container_network_tx_bytes,
};
use crate::runtime::ContainerRuntime;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Default interval for collecting container stats (in seconds)
const DEFAULT_STATS_INTERVAL_SECS: u64 = 15;

/// Container stats collector that periodically fetches stats from running containers
/// and updates Prometheus gauges.
pub struct StatsCollector {
    runtime: Arc<dyn ContainerRuntime>,
    /// Interval between stats collection cycles
    interval_secs: u64,
}

impl StatsCollector {
    /// Create a new stats collector
    pub fn new(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self {
            runtime,
            interval_secs: DEFAULT_STATS_INTERVAL_SECS,
        }
    }

    /// Create a new stats collector with a custom interval
    pub fn with_interval(runtime: Arc<dyn ContainerRuntime>, interval_secs: u64) -> Self {
        Self {
            runtime,
            interval_secs,
        }
    }

    /// Run a single stats collection cycle
    pub async fn collect(&self) -> CollectionResult {
        let mut result = CollectionResult::default();

        // List all running rivetr containers
        let containers = match self.runtime.list_containers("rivetr-").await {
            Ok(containers) => containers,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to list containers for stats collection");
                return result;
            }
        };

        // Filter to only running containers
        let running_containers: Vec<_> = containers
            .iter()
            .filter(|c| c.status.to_lowercase().contains("running"))
            .collect();

        result.containers_checked = running_containers.len();

        for container in running_containers {
            // Extract app name from container name (remove "rivetr-" prefix)
            let app_name = container
                .name
                .strip_prefix("rivetr-")
                .unwrap_or(&container.name);

            // Get stats for the container
            match self.runtime.stats(&container.id).await {
                Ok(stats) => {
                    // Update Prometheus gauges with app_name label
                    set_container_cpu_percent(app_name, stats.cpu_percent);
                    set_container_memory_bytes(app_name, stats.memory_usage);
                    set_container_memory_limit_bytes(app_name, stats.memory_limit);
                    set_container_network_rx_bytes(app_name, stats.network_rx);
                    set_container_network_tx_bytes(app_name, stats.network_tx);

                    result.successful += 1;

                    tracing::trace!(
                        app = %app_name,
                        cpu_percent = stats.cpu_percent,
                        memory_mb = stats.memory_usage / (1024 * 1024),
                        memory_limit_mb = stats.memory_limit / (1024 * 1024),
                        network_rx_kb = stats.network_rx / 1024,
                        network_tx_kb = stats.network_tx / 1024,
                        "Collected container stats"
                    );
                }
                Err(e) => {
                    result.failed += 1;
                    tracing::debug!(
                        container = %container.name,
                        error = %e,
                        "Failed to collect stats for container"
                    );
                }
            }
        }

        result
    }
}

/// Result of a stats collection cycle
#[derive(Debug, Default)]
pub struct CollectionResult {
    /// Number of containers checked
    pub containers_checked: usize,
    /// Number of successful stats collections
    pub successful: usize,
    /// Number of failed stats collections
    pub failed: usize,
}

/// Spawn the background stats collection task
pub fn spawn_stats_collector_task(runtime: Arc<dyn ContainerRuntime>) {
    spawn_stats_collector_task_with_interval(runtime, DEFAULT_STATS_INTERVAL_SECS);
}

/// Spawn the background stats collection task with a custom interval
pub fn spawn_stats_collector_task_with_interval(
    runtime: Arc<dyn ContainerRuntime>,
    interval_secs: u64,
) {
    tracing::info!(
        interval_secs = interval_secs,
        "Starting container stats collection task"
    );

    let collector = StatsCollector::with_interval(runtime, interval_secs);

    tokio::spawn(async move {
        // Wait a short time before the first collection to let containers start
        tokio::time::sleep(Duration::from_secs(5)).await;

        let mut tick = interval(Duration::from_secs(collector.interval_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;

            let result = collector.collect().await;

            if result.containers_checked > 0 {
                tracing::debug!(
                    containers = result.containers_checked,
                    successful = result.successful,
                    failed = result.failed,
                    "Stats collection cycle completed"
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_result_default() {
        let result = CollectionResult::default();
        assert_eq!(result.containers_checked, 0);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_stats_collector_interval() {
        use crate::runtime::NoopRuntime;

        let runtime: Arc<dyn ContainerRuntime> = Arc::new(NoopRuntime);
        let collector = StatsCollector::with_interval(runtime, 30);
        assert_eq!(collector.interval_secs, 30);
    }
}
