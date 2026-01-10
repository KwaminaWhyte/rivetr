//! Cost calculation service module
//!
//! This module computes estimated costs from resource metrics using configured
//! cost rates. It calculates daily average resource usage and stores cost
//! snapshots for per-app, per-project, and per-team aggregation.
//!
//! Cost calculation runs daily at midnight (or on demand)

use crate::db::{CostRate, CostSnapshot, CreateCostSnapshot};
use chrono::{Datelike, Timelike};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tokio::time::{interval, Duration};

/// Bytes per GB for memory/disk conversion
const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Days per month for cost calculation (average)
const DAYS_PER_MONTH: f64 = 30.0;

/// Default retention period for cost snapshots (in days)
const DEFAULT_COST_SNAPSHOT_RETENTION_DAYS: i64 = 365;

/// Cost calculation service
pub struct CostCalculator {
    db: SqlitePool,
    /// Retention period for cost snapshots in days
    retention_days: i64,
}

/// Result of a cost calculation cycle
#[derive(Debug, Default)]
pub struct CostCalculationResult {
    /// Number of apps processed
    pub apps_processed: usize,
    /// Number of cost snapshots created/updated
    pub snapshots_created: usize,
    /// Number of errors encountered
    pub errors: usize,
}

/// Cost rates loaded from database
#[derive(Debug, Clone)]
pub struct CostRates {
    /// Cost per CPU core per month (USD)
    pub cpu_per_core_month: f64,
    /// Cost per GB RAM per month (USD)
    pub memory_per_gb_month: f64,
    /// Cost per GB disk per month (USD)
    pub disk_per_gb_month: f64,
}

impl Default for CostRates {
    fn default() -> Self {
        Self {
            cpu_per_core_month: 0.02,
            memory_per_gb_month: 0.05,
            disk_per_gb_month: 0.10,
        }
    }
}

impl CostCalculator {
    /// Create a new cost calculator with default settings
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            retention_days: DEFAULT_COST_SNAPSHOT_RETENTION_DAYS,
        }
    }

    /// Create a new cost calculator with custom retention
    pub fn with_retention(db: SqlitePool, retention_days: i64) -> Self {
        Self { db, retention_days }
    }

    /// Load current cost rates from database
    pub async fn load_cost_rates(&self) -> CostRates {
        let rates = match CostRate::list_all(&self.db).await {
            Ok(rates) => rates,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load cost rates, using defaults");
                return CostRates::default();
            }
        };

        let mut cost_rates = CostRates::default();
        for rate in rates {
            match rate.resource_type.as_str() {
                "cpu" => cost_rates.cpu_per_core_month = rate.rate_per_unit,
                "memory" => cost_rates.memory_per_gb_month = rate.rate_per_unit,
                "disk" => cost_rates.disk_per_gb_month = rate.rate_per_unit,
                _ => {}
            }
        }

        cost_rates
    }

    /// Calculate and store cost snapshot for a specific date
    ///
    /// This calculates the daily average resource usage from metrics
    /// and applies the configured cost rates.
    pub async fn calculate_for_date(&self, date: &str) -> CostCalculationResult {
        let mut result = CostCalculationResult::default();

        // Load cost rates
        let rates = self.load_cost_rates().await;

        // Get all distinct app_ids that have metrics for this date
        let app_ids: Vec<(String,)> = match sqlx::query_as(
            r#"
            SELECT DISTINCT app_id
            FROM resource_metrics
            WHERE DATE(timestamp) = DATE(?)
            "#,
        )
        .bind(date)
        .fetch_all(&self.db)
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!(error = %e, date = %date, "Failed to fetch app_ids for cost calculation");
                return result;
            }
        };

        result.apps_processed = app_ids.len();

        for (app_id,) in app_ids {
            match self
                .calculate_app_cost_for_date(&app_id, date, &rates)
                .await
            {
                Ok(()) => {
                    result.snapshots_created += 1;
                }
                Err(e) => {
                    result.errors += 1;
                    tracing::debug!(
                        error = %e,
                        app_id = %app_id,
                        date = %date,
                        "Failed to calculate cost for app"
                    );
                }
            }
        }

        result
    }

    /// Calculate cost for a single app on a specific date
    async fn calculate_app_cost_for_date(
        &self,
        app_id: &str,
        date: &str,
        rates: &CostRates,
    ) -> anyhow::Result<()> {
        // Get aggregated metrics for this app on this date
        let metrics: Option<(f64, i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                AVG(cpu_percent),
                CAST(AVG(memory_bytes) AS INTEGER),
                CAST(AVG(memory_limit_bytes) AS INTEGER),
                CAST(AVG(disk_bytes) AS INTEGER),
                COUNT(*)
            FROM resource_metrics
            WHERE app_id = ? AND DATE(timestamp) = DATE(?)
            "#,
        )
        .bind(app_id)
        .bind(date)
        .fetch_optional(&self.db)
        .await?;

        let Some((
            avg_cpu_percent,
            avg_memory_bytes,
            _avg_memory_limit,
            avg_disk_bytes,
            sample_count,
        )) = metrics
        else {
            return Ok(());
        };

        if sample_count == 0 {
            return Ok(());
        }

        // Convert CPU percent to cores (assuming 100% = 1 core)
        let avg_cpu_cores = avg_cpu_percent / 100.0;

        // Convert bytes to GB
        let avg_memory_gb = avg_memory_bytes as f64 / BYTES_PER_GB;
        let avg_disk_gb = avg_disk_bytes as f64 / BYTES_PER_GB;

        // Calculate daily costs (monthly rate / days_per_month)
        let daily_cpu_rate = rates.cpu_per_core_month / DAYS_PER_MONTH;
        let daily_memory_rate = rates.memory_per_gb_month / DAYS_PER_MONTH;
        let daily_disk_rate = rates.disk_per_gb_month / DAYS_PER_MONTH;

        let cpu_cost = avg_cpu_cores * daily_cpu_rate;
        let memory_cost = avg_memory_gb * daily_memory_rate;
        let disk_cost = avg_disk_gb * daily_disk_rate;
        let total_cost = cpu_cost + memory_cost + disk_cost;

        // Create cost snapshot
        let snapshot = CreateCostSnapshot {
            app_id: app_id.to_string(),
            snapshot_date: date.to_string(),
            avg_cpu_cores,
            avg_memory_gb,
            avg_disk_gb,
            cpu_cost,
            memory_cost,
            disk_cost,
            total_cost,
            sample_count,
        };

        CostSnapshot::upsert(&self.db, &snapshot).await?;

        tracing::trace!(
            app_id = %app_id,
            date = %date,
            cpu_cores = avg_cpu_cores,
            memory_gb = avg_memory_gb,
            total_cost = total_cost,
            "Created cost snapshot"
        );

        Ok(())
    }

    /// Calculate costs for yesterday (called daily)
    pub async fn calculate_for_yesterday(&self) -> CostCalculationResult {
        let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
        let date_str = yesterday.format("%Y-%m-%d").to_string();
        self.calculate_for_date(&date_str).await
    }

    /// Calculate costs for today (for real-time dashboard)
    pub async fn calculate_for_today(&self) -> CostCalculationResult {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        self.calculate_for_date(&today).await
    }

    /// Backfill cost calculations for a date range
    pub async fn backfill(&self, days_back: i64) -> HashMap<String, CostCalculationResult> {
        let mut results = HashMap::new();

        for i in 1..=days_back {
            let date = chrono::Utc::now() - chrono::Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();
            let result = self.calculate_for_date(&date_str).await;
            results.insert(date_str, result);
        }

        results
    }

    /// Run cleanup of old cost snapshots based on retention policy
    pub async fn cleanup(&self) -> u64 {
        match CostSnapshot::cleanup_old_snapshots(&self.db, self.retention_days).await {
            Ok(deleted) => {
                if deleted > 0 {
                    tracing::debug!(
                        deleted = deleted,
                        retention_days = self.retention_days,
                        "Cleaned up old cost snapshots"
                    );
                }
                deleted
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to cleanup old cost snapshots");
                0
            }
        }
    }
}

/// Spawn the background cost calculation task
///
/// This runs daily to calculate cost snapshots for the previous day's metrics.
/// It also runs an initial calculation on startup for today's data.
pub fn spawn_cost_calculator_task(db: SqlitePool) {
    spawn_cost_calculator_task_with_config(db, DEFAULT_COST_SNAPSHOT_RETENTION_DAYS);
}

/// Spawn the background cost calculation task with custom retention
pub fn spawn_cost_calculator_task_with_config(db: SqlitePool, retention_days: i64) {
    tracing::info!(
        retention_days = retention_days,
        "Starting cost calculation background task"
    );

    let calculator = CostCalculator::with_retention(db, retention_days);

    tokio::spawn(async move {
        // Wait a bit for the system to stabilize
        tokio::time::sleep(Duration::from_secs(30)).await;

        // Initial calculation for today and yesterday
        let today_result = calculator.calculate_for_today().await;
        tracing::debug!(
            apps = today_result.apps_processed,
            snapshots = today_result.snapshots_created,
            "Initial cost calculation for today completed"
        );

        let yesterday_result = calculator.calculate_for_yesterday().await;
        tracing::debug!(
            apps = yesterday_result.apps_processed,
            snapshots = yesterday_result.snapshots_created,
            "Initial cost calculation for yesterday completed"
        );

        // Run daily at the start of each hour, calculating yesterday's costs
        // We use hourly checks to catch the midnight transition reliably
        let mut tick = interval(Duration::from_secs(3600)); // 1 hour
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut last_calculated_date = String::new();

        loop {
            tick.tick().await;

            let now = chrono::Utc::now();
            let yesterday = now - chrono::Duration::days(1);
            let yesterday_str = yesterday.format("%Y-%m-%d").to_string();

            // Only calculate if we haven't calculated for this day yet
            if yesterday_str != last_calculated_date {
                let result = calculator.calculate_for_date(&yesterday_str).await;
                tracing::info!(
                    date = %yesterday_str,
                    apps = result.apps_processed,
                    snapshots = result.snapshots_created,
                    errors = result.errors,
                    "Daily cost calculation completed"
                );
                last_calculated_date = yesterday_str;

                // Also update today's partial data
                let today_result = calculator.calculate_for_today().await;
                if today_result.apps_processed > 0 {
                    tracing::debug!(
                        apps = today_result.apps_processed,
                        snapshots = today_result.snapshots_created,
                        "Today's cost calculation updated"
                    );
                }
            }

            // Run cleanup weekly (on Sunday at the first hourly check)
            if now.weekday() == chrono::Weekday::Sun && now.hour() < 2 {
                calculator.cleanup().await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cost_rates() {
        let rates = CostRates::default();
        assert!((rates.cpu_per_core_month - 0.02).abs() < 0.001);
        assert!((rates.memory_per_gb_month - 0.05).abs() < 0.001);
        assert!((rates.disk_per_gb_month - 0.10).abs() < 0.001);
    }

    #[test]
    fn test_cost_calculation_result_default() {
        let result = CostCalculationResult::default();
        assert_eq!(result.apps_processed, 0);
        assert_eq!(result.snapshots_created, 0);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn test_daily_rate_calculation() {
        let rates = CostRates::default();
        let daily_cpu_rate = rates.cpu_per_core_month / DAYS_PER_MONTH;
        let daily_memory_rate = rates.memory_per_gb_month / DAYS_PER_MONTH;

        // CPU: $0.02/month / 30 days = $0.000667/day per core
        assert!((daily_cpu_rate - 0.02 / 30.0).abs() < 0.0001);
        // Memory: $0.05/month / 30 days = $0.00167/day per GB
        assert!((daily_memory_rate - 0.05 / 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_bytes_to_gb_conversion() {
        let one_gb_bytes: f64 = 1024.0 * 1024.0 * 1024.0;
        let converted = one_gb_bytes / BYTES_PER_GB;
        assert!((converted - 1.0).abs() < 0.001);

        let two_gb_bytes: f64 = 2.0 * 1024.0 * 1024.0 * 1024.0;
        let converted2 = two_gb_bytes / BYTES_PER_GB;
        assert!((converted2 - 2.0).abs() < 0.001);
    }
}
