//! Resource metrics models for per-app resource usage tracking.
//!
//! This module provides database models and queries for storing and retrieving
//! per-app CPU, memory, and disk metrics with 24-hour default retention.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Per-app resource metric record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResourceMetric {
    pub id: i64,
    pub app_id: String,
    pub timestamp: String,
    pub cpu_percent: f64,
    pub memory_bytes: i64,
    pub memory_limit_bytes: i64,
    pub disk_bytes: i64,
    pub disk_limit_bytes: i64,
}

/// Input for creating a new resource metric
#[derive(Debug, Clone)]
pub struct CreateResourceMetric {
    pub app_id: String,
    pub cpu_percent: f64,
    pub memory_bytes: i64,
    pub memory_limit_bytes: i64,
    pub disk_bytes: i64,
    pub disk_limit_bytes: i64,
}

/// Retention configuration for resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetricsRetentionConfig {
    /// Retention period in hours (default: 24)
    pub retention_hours: i64,
}

impl Default for ResourceMetricsRetentionConfig {
    fn default() -> Self {
        Self {
            retention_hours: 24,
        }
    }
}

impl ResourceMetric {
    /// Insert a new resource metric record
    pub async fn insert(
        db: &SqlitePool,
        metric: &CreateResourceMetric,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO resource_metrics (app_id, cpu_percent, memory_bytes, memory_limit_bytes, disk_bytes, disk_limit_bytes)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&metric.app_id)
        .bind(metric.cpu_percent)
        .bind(metric.memory_bytes)
        .bind(metric.memory_limit_bytes)
        .bind(metric.disk_bytes)
        .bind(metric.disk_limit_bytes)
        .execute(db)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Insert multiple resource metrics in a batch
    pub async fn insert_batch(
        db: &SqlitePool,
        metrics: &[CreateResourceMetric],
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut count = 0;
        for metric in metrics {
            if Self::insert(db, metric).await.is_ok() {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get resource metrics for an app within a time range
    pub async fn get_for_app(
        db: &SqlitePool,
        app_id: &str,
        hours: i64,
        limit: Option<i64>,
    ) -> Result<Vec<ResourceMetric>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
        let limit = limit.unwrap_or(hours * 60); // 1 sample per minute

        sqlx::query_as(
            r#"
            SELECT id, app_id, timestamp, cpu_percent, memory_bytes, memory_limit_bytes, disk_bytes, disk_limit_bytes
            FROM resource_metrics
            WHERE app_id = ? AND timestamp >= ?
            ORDER BY timestamp ASC
            LIMIT ?
            "#,
        )
        .bind(app_id)
        .bind(&cutoff_str)
        .bind(limit)
        .fetch_all(db)
        .await
    }

    /// Get the latest resource metric for an app
    pub async fn get_latest_for_app(
        db: &SqlitePool,
        app_id: &str,
    ) -> Result<Option<ResourceMetric>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, timestamp, cpu_percent, memory_bytes, memory_limit_bytes, disk_bytes, disk_limit_bytes
            FROM resource_metrics
            WHERE app_id = ?
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(app_id)
        .fetch_optional(db)
        .await
    }

    /// Get aggregated resource metrics for an app
    pub async fn get_aggregated_for_app(
        db: &SqlitePool,
        app_id: &str,
        hours: i64,
    ) -> Result<Option<AggregatedResourceMetrics>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result: Option<(f64, f64, i64, i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                AVG(cpu_percent),
                MAX(cpu_percent),
                CAST(AVG(memory_bytes) AS INTEGER),
                MAX(memory_bytes),
                CAST(AVG(memory_limit_bytes) AS INTEGER),
                CAST(AVG(disk_bytes) AS INTEGER),
                COUNT(*)
            FROM resource_metrics
            WHERE app_id = ? AND timestamp >= ?
            "#,
        )
        .bind(app_id)
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(result.and_then(
            |(avg_cpu, max_cpu, avg_mem, max_mem, avg_mem_limit, avg_disk, count)| {
                if count > 0 {
                    Some(AggregatedResourceMetrics {
                        avg_cpu_percent: avg_cpu,
                        max_cpu_percent: max_cpu,
                        avg_memory_bytes: avg_mem,
                        max_memory_bytes: max_mem,
                        avg_memory_limit_bytes: avg_mem_limit,
                        avg_disk_bytes: avg_disk,
                        sample_count: count,
                    })
                } else {
                    None
                }
            },
        ))
    }

    /// Delete resource metrics older than the retention period
    pub async fn cleanup_old_metrics(
        db: &SqlitePool,
        retention_hours: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(retention_hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query("DELETE FROM resource_metrics WHERE timestamp < ?")
            .bind(&cutoff_str)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all metrics for a specific app
    pub async fn delete_for_app(db: &SqlitePool, app_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM resource_metrics WHERE app_id = ?")
            .bind(app_id)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Aggregated resource metrics for an app
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedResourceMetrics {
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub avg_memory_bytes: i64,
    pub max_memory_bytes: i64,
    pub avg_memory_limit_bytes: i64,
    pub avg_disk_bytes: i64,
    pub sample_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_config_default() {
        let config = ResourceMetricsRetentionConfig::default();
        assert_eq!(config.retention_hours, 24);
    }
}
