//! Per-app resource metrics collector module
//!
//! This module periodically collects CPU, memory, and disk metrics for each
//! running app and stores them in the resource_metrics table for alerting
//! and cost estimation purposes.
//!
//! Collection interval: 60 seconds (configurable)
//! Retention: 24 hours by default

use crate::db::{CreateResourceMetric, Deployment, ResourceMetric};
use crate::runtime::ContainerRuntime;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Default interval for collecting per-app resource metrics (in seconds)
const DEFAULT_RESOURCE_METRICS_INTERVAL_SECS: u64 = 60;

/// Default retention period for resource metrics (in hours)
const DEFAULT_RESOURCE_METRICS_RETENTION_HOURS: i64 = 24;

/// Per-app resource metrics collector
pub struct ResourceMetricsCollector {
    db: SqlitePool,
    runtime: Arc<dyn ContainerRuntime>,
    /// Interval between collection cycles
    interval_secs: u64,
    /// Retention period in hours
    retention_hours: i64,
}

impl ResourceMetricsCollector {
    /// Create a new resource metrics collector with default settings
    pub fn new(db: SqlitePool, runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self {
            db,
            runtime,
            interval_secs: DEFAULT_RESOURCE_METRICS_INTERVAL_SECS,
            retention_hours: DEFAULT_RESOURCE_METRICS_RETENTION_HOURS,
        }
    }

    /// Create a new resource metrics collector with custom settings
    pub fn with_config(
        db: SqlitePool,
        runtime: Arc<dyn ContainerRuntime>,
        interval_secs: u64,
        retention_hours: i64,
    ) -> Self {
        Self {
            db,
            runtime,
            interval_secs,
            retention_hours,
        }
    }

    /// Run a single metrics collection cycle
    pub async fn collect(&self) -> ResourceMetricsCollectionResult {
        let mut result = ResourceMetricsCollectionResult::default();

        // Get all running deployments
        let running_deployments: Vec<Deployment> = match sqlx::query_as(
            "SELECT * FROM deployments WHERE status = 'running'",
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(deployments) => deployments,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch running deployments for resource metrics");
                return result;
            }
        };

        result.apps_checked = running_deployments.len();

        // Collect metrics for each running deployment
        let mut metrics_to_insert: Vec<CreateResourceMetric> = Vec::new();

        for deployment in &running_deployments {
            let Some(container_id) = &deployment.container_id else {
                continue;
            };

            match self.runtime.stats(container_id).await {
                Ok(stats) => {
                    let metric = CreateResourceMetric {
                        app_id: deployment.app_id.clone(),
                        cpu_percent: stats.cpu_percent,
                        memory_bytes: stats.memory_usage as i64,
                        memory_limit_bytes: stats.memory_limit as i64,
                        // Disk metrics will be added in a future enhancement
                        // when we implement volume usage tracking
                        disk_bytes: 0,
                        disk_limit_bytes: 0,
                    };
                    metrics_to_insert.push(metric);
                    result.successful += 1;

                    tracing::trace!(
                        app_id = %deployment.app_id,
                        container_id = %container_id,
                        cpu_percent = stats.cpu_percent,
                        memory_mb = stats.memory_usage / (1024 * 1024),
                        memory_limit_mb = stats.memory_limit / (1024 * 1024),
                        "Collected resource metrics for app"
                    );
                }
                Err(e) => {
                    result.failed += 1;
                    tracing::debug!(
                        app_id = %deployment.app_id,
                        container_id = %container_id,
                        error = %e,
                        "Failed to collect resource metrics for app"
                    );
                }
            }
        }

        // Batch insert metrics
        if !metrics_to_insert.is_empty() {
            match ResourceMetric::insert_batch(&self.db, &metrics_to_insert).await {
                Ok(inserted) => {
                    tracing::debug!(
                        inserted = inserted,
                        total = metrics_to_insert.len(),
                        "Inserted resource metrics"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to insert resource metrics batch");
                }
            }
        }

        result
    }

    /// Run cleanup of old metrics based on retention policy
    pub async fn cleanup(&self) -> u64 {
        match ResourceMetric::cleanup_old_metrics(&self.db, self.retention_hours).await {
            Ok(deleted) => {
                if deleted > 0 {
                    tracing::debug!(
                        deleted = deleted,
                        retention_hours = self.retention_hours,
                        "Cleaned up old resource metrics"
                    );
                }
                deleted
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to cleanup old resource metrics");
                0
            }
        }
    }
}

/// Result of a resource metrics collection cycle
#[derive(Debug, Default)]
pub struct ResourceMetricsCollectionResult {
    /// Number of apps checked
    pub apps_checked: usize,
    /// Number of successful metrics collections
    pub successful: usize,
    /// Number of failed metrics collections
    pub failed: usize,
}

/// Spawn the background resource metrics collection task
pub fn spawn_resource_metrics_collector_task(db: SqlitePool, runtime: Arc<dyn ContainerRuntime>) {
    spawn_resource_metrics_collector_task_with_config(
        db,
        runtime,
        DEFAULT_RESOURCE_METRICS_INTERVAL_SECS,
        DEFAULT_RESOURCE_METRICS_RETENTION_HOURS,
    );
}

/// Spawn the background resource metrics collection task with custom config
pub fn spawn_resource_metrics_collector_task_with_config(
    db: SqlitePool,
    runtime: Arc<dyn ContainerRuntime>,
    interval_secs: u64,
    retention_hours: i64,
) {
    tracing::info!(
        interval_secs = interval_secs,
        retention_hours = retention_hours,
        "Starting per-app resource metrics collection task"
    );

    let collector =
        ResourceMetricsCollector::with_config(db, runtime, interval_secs, retention_hours);

    tokio::spawn(async move {
        // Wait a short time before the first collection to let containers start
        tokio::time::sleep(Duration::from_secs(10)).await;

        let mut collect_tick = interval(Duration::from_secs(collector.interval_secs));
        collect_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Run cleanup every hour
        let mut cleanup_tick = interval(Duration::from_secs(3600));
        cleanup_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = collect_tick.tick() => {
                    let result = collector.collect().await;

                    if result.apps_checked > 0 {
                        tracing::debug!(
                            apps = result.apps_checked,
                            successful = result.successful,
                            failed = result.failed,
                            "Resource metrics collection cycle completed"
                        );
                    }
                }
                _ = cleanup_tick.tick() => {
                    collector.cleanup().await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_result_default() {
        let result = ResourceMetricsCollectionResult::default();
        assert_eq!(result.apps_checked, 0);
        assert_eq!(result.successful, 0);
        assert_eq!(result.failed, 0);
    }
}
