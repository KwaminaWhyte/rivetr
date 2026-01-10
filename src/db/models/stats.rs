//! Stats history models for metrics storage and aggregation.
//!
//! This module provides database models and queries for:
//! - Raw stats history (5-minute intervals, 7-day retention)
//! - Hourly aggregated stats (30-day retention)
//! - Daily aggregated stats (365-day retention)

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Raw stats history record (5-minute intervals)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StatsHistory {
    pub id: i64,
    pub timestamp: String,
    pub cpu_percent: f64,
    pub memory_used_bytes: i64,
    pub memory_total_bytes: i64,
    pub running_apps: i64,
    pub running_databases: i64,
    pub running_services: i64,
}

/// Hourly aggregated stats
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StatsHourly {
    pub id: i64,
    pub hour_timestamp: String,
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub min_cpu_percent: f64,
    pub avg_memory_used_bytes: i64,
    pub max_memory_used_bytes: i64,
    pub avg_memory_total_bytes: i64,
    pub avg_running_apps: f64,
    pub avg_running_databases: f64,
    pub avg_running_services: f64,
    pub sample_count: i64,
    pub created_at: String,
}

/// Daily aggregated stats
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StatsDaily {
    pub id: i64,
    pub day_timestamp: String,
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub min_cpu_percent: f64,
    pub avg_memory_used_bytes: i64,
    pub max_memory_used_bytes: i64,
    pub avg_memory_total_bytes: i64,
    pub avg_running_apps: f64,
    pub max_running_apps: i64,
    pub avg_running_databases: f64,
    pub max_running_databases: i64,
    pub avg_running_services: f64,
    pub max_running_services: i64,
    pub sample_count: i64,
    pub created_at: String,
}

/// Aggregated stats point for API responses
#[derive(Debug, Clone, Serialize)]
pub struct AggregatedStatsPoint {
    pub timestamp: String,
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub min_cpu_percent: f64,
    pub avg_memory_used_bytes: i64,
    pub max_memory_used_bytes: i64,
    pub avg_memory_total_bytes: i64,
    pub avg_running_apps: f64,
    pub avg_running_databases: f64,
    pub avg_running_services: f64,
    pub sample_count: i64,
}

/// Retention configuration for stats
#[derive(Debug, Clone)]
pub struct StatsRetentionConfig {
    /// Raw stats retention in days (default: 7)
    pub raw_retention_days: i64,
    /// Hourly aggregated stats retention in days (default: 30)
    pub hourly_retention_days: i64,
    /// Daily aggregated stats retention in days (default: 365)
    pub daily_retention_days: i64,
}

impl Default for StatsRetentionConfig {
    fn default() -> Self {
        Self {
            raw_retention_days: 7,
            hourly_retention_days: 30,
            daily_retention_days: 365,
        }
    }
}

impl StatsHistory {
    /// Get raw stats history for a time range
    pub async fn get_history(
        db: &SqlitePool,
        hours: i64,
        limit: Option<i64>,
    ) -> Result<Vec<StatsHistory>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
        let limit = limit.unwrap_or(hours * 12); // 12 samples per hour at 5-min intervals

        sqlx::query_as(
            r#"
            SELECT id, timestamp, cpu_percent, memory_used_bytes, memory_total_bytes,
                   running_apps, running_databases, running_services
            FROM stats_history
            WHERE timestamp >= ?
            ORDER BY timestamp ASC
            LIMIT ?
            "#,
        )
        .bind(&cutoff_str)
        .bind(limit)
        .fetch_all(db)
        .await
    }

    /// Get the latest stats record
    pub async fn get_latest(db: &SqlitePool) -> Result<Option<StatsHistory>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, timestamp, cpu_percent, memory_used_bytes, memory_total_bytes,
                   running_apps, running_databases, running_services
            FROM stats_history
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(db)
        .await
    }
}

impl StatsHourly {
    /// Get hourly aggregated stats for a time range
    pub async fn get_history(db: &SqlitePool, hours: i64) -> Result<Vec<StatsHourly>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:00:00").to_string();

        sqlx::query_as(
            r#"
            SELECT id, hour_timestamp, avg_cpu_percent, max_cpu_percent, min_cpu_percent,
                   avg_memory_used_bytes, max_memory_used_bytes, avg_memory_total_bytes,
                   avg_running_apps, avg_running_databases, avg_running_services,
                   sample_count, created_at
            FROM stats_hourly
            WHERE hour_timestamp >= ?
            ORDER BY hour_timestamp ASC
            "#,
        )
        .bind(&cutoff_str)
        .fetch_all(db)
        .await
    }

    /// Aggregate raw stats for a specific hour and upsert
    pub async fn aggregate_hour(db: &SqlitePool, hour_timestamp: &str) -> Result<(), sqlx::Error> {
        // Calculate the end of the hour
        let hour_start = hour_timestamp;
        let hour_end = {
            let dt = chrono::NaiveDateTime::parse_from_str(hour_timestamp, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| chrono::Utc::now().naive_utc());
            let end = dt + chrono::Duration::hours(1);
            end.format("%Y-%m-%d %H:%M:%S").to_string()
        };

        sqlx::query(
            r#"
            INSERT INTO stats_hourly (
                hour_timestamp, avg_cpu_percent, max_cpu_percent, min_cpu_percent,
                avg_memory_used_bytes, max_memory_used_bytes, avg_memory_total_bytes,
                avg_running_apps, avg_running_databases, avg_running_services, sample_count
            )
            SELECT
                ?,
                AVG(cpu_percent),
                MAX(cpu_percent),
                MIN(cpu_percent),
                CAST(AVG(memory_used_bytes) AS INTEGER),
                MAX(memory_used_bytes),
                CAST(AVG(memory_total_bytes) AS INTEGER),
                AVG(running_apps),
                AVG(running_databases),
                AVG(running_services),
                COUNT(*)
            FROM stats_history
            WHERE timestamp >= ? AND timestamp < ?
            ON CONFLICT(hour_timestamp) DO UPDATE SET
                avg_cpu_percent = excluded.avg_cpu_percent,
                max_cpu_percent = excluded.max_cpu_percent,
                min_cpu_percent = excluded.min_cpu_percent,
                avg_memory_used_bytes = excluded.avg_memory_used_bytes,
                max_memory_used_bytes = excluded.max_memory_used_bytes,
                avg_memory_total_bytes = excluded.avg_memory_total_bytes,
                avg_running_apps = excluded.avg_running_apps,
                avg_running_databases = excluded.avg_running_databases,
                avg_running_services = excluded.avg_running_services,
                sample_count = excluded.sample_count
            "#,
        )
        .bind(hour_start)
        .bind(hour_start)
        .bind(&hour_end)
        .execute(db)
        .await?;

        Ok(())
    }
}

impl StatsDaily {
    /// Get daily aggregated stats for a time range
    pub async fn get_history(db: &SqlitePool, days: i64) -> Result<Vec<StatsDaily>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        sqlx::query_as(
            r#"
            SELECT id, day_timestamp, avg_cpu_percent, max_cpu_percent, min_cpu_percent,
                   avg_memory_used_bytes, max_memory_used_bytes, avg_memory_total_bytes,
                   avg_running_apps, max_running_apps, avg_running_databases, max_running_databases,
                   avg_running_services, max_running_services, sample_count, created_at
            FROM stats_daily
            WHERE day_timestamp >= ?
            ORDER BY day_timestamp ASC
            "#,
        )
        .bind(&cutoff_str)
        .fetch_all(db)
        .await
    }

    /// Aggregate hourly stats for a specific day and upsert
    pub async fn aggregate_day(db: &SqlitePool, day_timestamp: &str) -> Result<(), sqlx::Error> {
        // Calculate the next day
        let day_start = format!("{} 00:00:00", day_timestamp);
        let day_end = {
            let dt = chrono::NaiveDate::parse_from_str(day_timestamp, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::Utc::now().date_naive());
            let end = dt + chrono::Duration::days(1);
            format!("{} 00:00:00", end.format("%Y-%m-%d"))
        };

        sqlx::query(
            r#"
            INSERT INTO stats_daily (
                day_timestamp, avg_cpu_percent, max_cpu_percent, min_cpu_percent,
                avg_memory_used_bytes, max_memory_used_bytes, avg_memory_total_bytes,
                avg_running_apps, max_running_apps, avg_running_databases, max_running_databases,
                avg_running_services, max_running_services, sample_count
            )
            SELECT
                ?,
                AVG(avg_cpu_percent),
                MAX(max_cpu_percent),
                MIN(min_cpu_percent),
                CAST(AVG(avg_memory_used_bytes) AS INTEGER),
                MAX(max_memory_used_bytes),
                CAST(AVG(avg_memory_total_bytes) AS INTEGER),
                AVG(avg_running_apps),
                CAST(MAX(avg_running_apps) AS INTEGER),
                AVG(avg_running_databases),
                CAST(MAX(avg_running_databases) AS INTEGER),
                AVG(avg_running_services),
                CAST(MAX(avg_running_services) AS INTEGER),
                SUM(sample_count)
            FROM stats_hourly
            WHERE hour_timestamp >= ? AND hour_timestamp < ?
            ON CONFLICT(day_timestamp) DO UPDATE SET
                avg_cpu_percent = excluded.avg_cpu_percent,
                max_cpu_percent = excluded.max_cpu_percent,
                min_cpu_percent = excluded.min_cpu_percent,
                avg_memory_used_bytes = excluded.avg_memory_used_bytes,
                max_memory_used_bytes = excluded.max_memory_used_bytes,
                avg_memory_total_bytes = excluded.avg_memory_total_bytes,
                avg_running_apps = excluded.avg_running_apps,
                max_running_apps = excluded.max_running_apps,
                avg_running_databases = excluded.avg_running_databases,
                max_running_databases = excluded.max_running_databases,
                avg_running_services = excluded.avg_running_services,
                max_running_services = excluded.max_running_services,
                sample_count = excluded.sample_count
            "#,
        )
        .bind(day_timestamp)
        .bind(&day_start)
        .bind(&day_end)
        .execute(db)
        .await?;

        Ok(())
    }
}

/// Stats retention cleanup functions
pub struct StatsRetention;

impl StatsRetention {
    /// Delete raw stats older than retention period
    pub async fn cleanup_raw_stats(
        db: &SqlitePool,
        retention_days: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query("DELETE FROM stats_history WHERE timestamp < ?")
            .bind(&cutoff_str)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete hourly stats older than retention period
    pub async fn cleanup_hourly_stats(
        db: &SqlitePool,
        retention_days: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:00:00").to_string();

        let result = sqlx::query("DELETE FROM stats_hourly WHERE hour_timestamp < ?")
            .bind(&cutoff_str)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete daily stats older than retention period
    pub async fn cleanup_daily_stats(
        db: &SqlitePool,
        retention_days: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let result = sqlx::query("DELETE FROM stats_daily WHERE day_timestamp < ?")
            .bind(&cutoff_str)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Run all retention cleanups with default config
    pub async fn run_cleanup(
        db: &SqlitePool,
        config: &StatsRetentionConfig,
    ) -> Result<(u64, u64, u64), sqlx::Error> {
        let raw_deleted = Self::cleanup_raw_stats(db, config.raw_retention_days).await?;
        let hourly_deleted = Self::cleanup_hourly_stats(db, config.hourly_retention_days).await?;
        let daily_deleted = Self::cleanup_daily_stats(db, config.daily_retention_days).await?;

        Ok((raw_deleted, hourly_deleted, daily_deleted))
    }

    /// Aggregate pending raw stats into hourly buckets
    pub async fn aggregate_to_hourly(db: &SqlitePool) -> Result<u64, sqlx::Error> {
        // Get distinct hours that have raw data but may need aggregation
        // Only aggregate hours that are complete (not the current hour)
        let current_hour = chrono::Utc::now().format("%Y-%m-%d %H:00:00").to_string();

        let hours: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT strftime('%Y-%m-%d %H:00:00', timestamp) as hour
            FROM stats_history
            WHERE strftime('%Y-%m-%d %H:00:00', timestamp) < ?
            ORDER BY hour
            "#,
        )
        .bind(&current_hour)
        .fetch_all(db)
        .await?;

        let mut aggregated = 0u64;
        for (hour,) in hours {
            if StatsHourly::aggregate_hour(db, &hour).await.is_ok() {
                aggregated += 1;
            }
        }

        Ok(aggregated)
    }

    /// Aggregate pending hourly stats into daily buckets
    pub async fn aggregate_to_daily(db: &SqlitePool) -> Result<u64, sqlx::Error> {
        // Get distinct days that have hourly data but may need aggregation
        // Only aggregate days that are complete (not the current day)
        let current_day = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let days: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT date(hour_timestamp) as day
            FROM stats_hourly
            WHERE date(hour_timestamp) < ?
            ORDER BY day
            "#,
        )
        .bind(&current_day)
        .fetch_all(db)
        .await?;

        let mut aggregated = 0u64;
        for (day,) in days {
            if StatsDaily::aggregate_day(db, &day).await.is_ok() {
                aggregated += 1;
            }
        }

        Ok(aggregated)
    }
}

/// Get system-wide aggregated stats summary
#[derive(Debug, Clone, Serialize)]
pub struct SystemStatsSummary {
    /// Current stats (most recent)
    pub current: Option<CurrentStats>,
    /// Stats for the last 24 hours
    pub last_24h: Option<PeriodStats>,
    /// Stats for the last 7 days
    pub last_7d: Option<PeriodStats>,
    /// Stats for the last 30 days
    pub last_30d: Option<PeriodStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentStats {
    pub timestamp: String,
    pub cpu_percent: f64,
    pub memory_used_bytes: i64,
    pub memory_total_bytes: i64,
    pub running_apps: i64,
    pub running_databases: i64,
    pub running_services: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeriodStats {
    pub avg_cpu_percent: f64,
    pub max_cpu_percent: f64,
    pub avg_memory_used_bytes: i64,
    pub max_memory_used_bytes: i64,
    pub avg_memory_total_bytes: i64,
    pub sample_count: i64,
}

impl SystemStatsSummary {
    pub async fn get(db: &SqlitePool) -> Result<Self, sqlx::Error> {
        // Get current stats
        let current = StatsHistory::get_latest(db).await?.map(|s| CurrentStats {
            timestamp: s.timestamp,
            cpu_percent: s.cpu_percent,
            memory_used_bytes: s.memory_used_bytes,
            memory_total_bytes: s.memory_total_bytes,
            running_apps: s.running_apps,
            running_databases: s.running_databases,
            running_services: s.running_services,
        });

        // Get 24h stats from raw history
        let last_24h = Self::get_period_stats_raw(db, 24).await?;

        // Get 7d stats from hourly aggregates
        let last_7d = Self::get_period_stats_hourly(db, 168).await?;

        // Get 30d stats from daily aggregates
        let last_30d = Self::get_period_stats_daily(db, 30).await?;

        Ok(Self {
            current,
            last_24h,
            last_7d,
            last_30d,
        })
    }

    async fn get_period_stats_raw(
        db: &SqlitePool,
        hours: i64,
    ) -> Result<Option<PeriodStats>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let stats: Option<(f64, f64, i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                AVG(cpu_percent),
                MAX(cpu_percent),
                CAST(AVG(memory_used_bytes) AS INTEGER),
                MAX(memory_used_bytes),
                CAST(AVG(memory_total_bytes) AS INTEGER),
                COUNT(*)
            FROM stats_history
            WHERE timestamp >= ?
            "#,
        )
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(
            stats.and_then(|(avg_cpu, max_cpu, avg_mem, max_mem, avg_total, count)| {
                if count > 0 {
                    Some(PeriodStats {
                        avg_cpu_percent: avg_cpu,
                        max_cpu_percent: max_cpu,
                        avg_memory_used_bytes: avg_mem,
                        max_memory_used_bytes: max_mem,
                        avg_memory_total_bytes: avg_total,
                        sample_count: count,
                    })
                } else {
                    None
                }
            }),
        )
    }

    async fn get_period_stats_hourly(
        db: &SqlitePool,
        hours: i64,
    ) -> Result<Option<PeriodStats>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:00:00").to_string();

        let stats: Option<(f64, f64, i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                AVG(avg_cpu_percent),
                MAX(max_cpu_percent),
                CAST(AVG(avg_memory_used_bytes) AS INTEGER),
                MAX(max_memory_used_bytes),
                CAST(AVG(avg_memory_total_bytes) AS INTEGER),
                SUM(sample_count)
            FROM stats_hourly
            WHERE hour_timestamp >= ?
            "#,
        )
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(
            stats.and_then(|(avg_cpu, max_cpu, avg_mem, max_mem, avg_total, count)| {
                if count > 0 {
                    Some(PeriodStats {
                        avg_cpu_percent: avg_cpu,
                        max_cpu_percent: max_cpu,
                        avg_memory_used_bytes: avg_mem,
                        max_memory_used_bytes: max_mem,
                        avg_memory_total_bytes: avg_total,
                        sample_count: count,
                    })
                } else {
                    None
                }
            }),
        )
    }

    async fn get_period_stats_daily(
        db: &SqlitePool,
        days: i64,
    ) -> Result<Option<PeriodStats>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let stats: Option<(f64, f64, i64, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                AVG(avg_cpu_percent),
                MAX(max_cpu_percent),
                CAST(AVG(avg_memory_used_bytes) AS INTEGER),
                MAX(max_memory_used_bytes),
                CAST(AVG(avg_memory_total_bytes) AS INTEGER),
                SUM(sample_count)
            FROM stats_daily
            WHERE day_timestamp >= ?
            "#,
        )
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(
            stats.and_then(|(avg_cpu, max_cpu, avg_mem, max_mem, avg_total, count)| {
                if count > 0 {
                    Some(PeriodStats {
                        avg_cpu_percent: avg_cpu,
                        max_cpu_percent: max_cpu,
                        avg_memory_used_bytes: avg_mem,
                        max_memory_used_bytes: max_mem,
                        avg_memory_total_bytes: avg_total,
                        sample_count: count,
                    })
                } else {
                    None
                }
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_config_default() {
        let config = StatsRetentionConfig::default();
        assert_eq!(config.raw_retention_days, 7);
        assert_eq!(config.hourly_retention_days, 30);
        assert_eq!(config.daily_retention_days, 365);
    }
}
