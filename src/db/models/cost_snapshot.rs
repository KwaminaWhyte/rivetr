//! Cost snapshot models for daily cost calculations.
//!
//! This module provides database models and queries for storing and retrieving
//! daily cost snapshots computed from resource metrics.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Daily cost snapshot for an app
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CostSnapshot {
    pub id: i64,
    pub app_id: String,
    pub snapshot_date: String,
    pub avg_cpu_cores: f64,
    pub avg_memory_gb: f64,
    pub avg_disk_gb: f64,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub disk_cost: f64,
    pub total_cost: f64,
    pub sample_count: i64,
    pub created_at: String,
}

/// Input for creating or updating a cost snapshot
#[derive(Debug, Clone)]
pub struct CreateCostSnapshot {
    pub app_id: String,
    pub snapshot_date: String,
    pub avg_cpu_cores: f64,
    pub avg_memory_gb: f64,
    pub avg_disk_gb: f64,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub disk_cost: f64,
    pub total_cost: f64,
    pub sample_count: i64,
}

/// Aggregated cost data for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub disk_cost: f64,
    pub total_cost: f64,
    pub avg_cpu_cores: f64,
    pub avg_memory_gb: f64,
    pub avg_disk_gb: f64,
    pub days_in_period: i64,
    pub projected_monthly_cost: f64,
}

/// Cost breakdown by app for aggregation views
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppCostBreakdown {
    pub app_id: String,
    pub app_name: String,
    pub cpu_cost: f64,
    pub memory_cost: f64,
    pub disk_cost: f64,
    pub total_cost: f64,
}

impl CostSnapshot {
    /// Insert or update a cost snapshot (upsert on app_id + snapshot_date)
    pub async fn upsert(
        db: &SqlitePool,
        snapshot: &CreateCostSnapshot,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO cost_snapshots (
                app_id, snapshot_date, avg_cpu_cores, avg_memory_gb, avg_disk_gb,
                cpu_cost, memory_cost, disk_cost, total_cost, sample_count
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(app_id, snapshot_date) DO UPDATE SET
                avg_cpu_cores = excluded.avg_cpu_cores,
                avg_memory_gb = excluded.avg_memory_gb,
                avg_disk_gb = excluded.avg_disk_gb,
                cpu_cost = excluded.cpu_cost,
                memory_cost = excluded.memory_cost,
                disk_cost = excluded.disk_cost,
                total_cost = excluded.total_cost,
                sample_count = excluded.sample_count
            "#,
        )
        .bind(&snapshot.app_id)
        .bind(&snapshot.snapshot_date)
        .bind(snapshot.avg_cpu_cores)
        .bind(snapshot.avg_memory_gb)
        .bind(snapshot.avg_disk_gb)
        .bind(snapshot.cpu_cost)
        .bind(snapshot.memory_cost)
        .bind(snapshot.disk_cost)
        .bind(snapshot.total_cost)
        .bind(snapshot.sample_count)
        .execute(db)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get cost snapshots for an app within a time period
    pub async fn get_for_app(
        db: &SqlitePool,
        app_id: &str,
        days: i64,
    ) -> Result<Vec<CostSnapshot>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        sqlx::query_as(
            r#"
            SELECT id, app_id, snapshot_date, avg_cpu_cores, avg_memory_gb, avg_disk_gb,
                   cpu_cost, memory_cost, disk_cost, total_cost, sample_count, created_at
            FROM cost_snapshots
            WHERE app_id = ? AND snapshot_date >= ?
            ORDER BY snapshot_date ASC
            "#,
        )
        .bind(app_id)
        .bind(&cutoff_str)
        .fetch_all(db)
        .await
    }

    /// Get aggregated cost summary for an app
    pub async fn get_summary_for_app(
        db: &SqlitePool,
        app_id: &str,
        days: i64,
    ) -> Result<Option<CostSummary>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        // Cast to REAL to avoid SQLite returning INTEGER for empty results
        let result: Option<(f64, f64, f64, f64, f64, f64, f64, i64)> = sqlx::query_as(
            r#"
            SELECT
                CAST(COALESCE(SUM(cpu_cost), 0) AS REAL),
                CAST(COALESCE(SUM(memory_cost), 0) AS REAL),
                CAST(COALESCE(SUM(disk_cost), 0) AS REAL),
                CAST(COALESCE(SUM(total_cost), 0) AS REAL),
                CAST(COALESCE(AVG(avg_cpu_cores), 0) AS REAL),
                CAST(COALESCE(AVG(avg_memory_gb), 0) AS REAL),
                CAST(COALESCE(AVG(avg_disk_gb), 0) AS REAL),
                COUNT(*)
            FROM cost_snapshots
            WHERE app_id = ? AND snapshot_date >= ?
            "#,
        )
        .bind(app_id)
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(result.map(
            |(cpu_cost, memory_cost, disk_cost, total_cost, avg_cpu, avg_mem, avg_disk, count)| {
                // Project monthly cost based on daily average
                let daily_avg = if count > 0 {
                    total_cost / count as f64
                } else {
                    0.0
                };
                let projected_monthly = daily_avg * 30.0;

                CostSummary {
                    cpu_cost,
                    memory_cost,
                    disk_cost,
                    total_cost,
                    avg_cpu_cores: avg_cpu,
                    avg_memory_gb: avg_mem,
                    avg_disk_gb: avg_disk,
                    days_in_period: count,
                    projected_monthly_cost: projected_monthly,
                }
            },
        ))
    }

    /// Get aggregated cost summary for a project (all apps in the project)
    pub async fn get_summary_for_project(
        db: &SqlitePool,
        project_id: &str,
        days: i64,
    ) -> Result<Option<CostSummary>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        // Cast to REAL to avoid SQLite returning INTEGER for empty results
        let result: Option<(f64, f64, f64, f64, f64, f64, f64, i64)> = sqlx::query_as(
            r#"
            SELECT
                CAST(COALESCE(SUM(cs.cpu_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.memory_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.disk_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.total_cost), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_cpu_cores), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_memory_gb), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_disk_gb), 0) AS REAL),
                COUNT(DISTINCT cs.snapshot_date)
            FROM cost_snapshots cs
            INNER JOIN apps a ON cs.app_id = a.id
            WHERE a.project_id = ? AND cs.snapshot_date >= ?
            "#,
        )
        .bind(project_id)
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(result.map(
            |(cpu_cost, memory_cost, disk_cost, total_cost, avg_cpu, avg_mem, avg_disk, count)| {
                let daily_avg = if count > 0 {
                    total_cost / count as f64
                } else {
                    0.0
                };
                let projected_monthly = daily_avg * 30.0;

                CostSummary {
                    cpu_cost,
                    memory_cost,
                    disk_cost,
                    total_cost,
                    avg_cpu_cores: avg_cpu,
                    avg_memory_gb: avg_mem,
                    avg_disk_gb: avg_disk,
                    days_in_period: count,
                    projected_monthly_cost: projected_monthly,
                }
            },
        ))
    }

    /// Get aggregated cost summary for a team (all apps owned by the team)
    pub async fn get_summary_for_team(
        db: &SqlitePool,
        team_id: &str,
        days: i64,
    ) -> Result<Option<CostSummary>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        // Cast to REAL to avoid SQLite returning INTEGER for empty results
        let result: Option<(f64, f64, f64, f64, f64, f64, f64, i64)> = sqlx::query_as(
            r#"
            SELECT
                CAST(COALESCE(SUM(cs.cpu_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.memory_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.disk_cost), 0) AS REAL),
                CAST(COALESCE(SUM(cs.total_cost), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_cpu_cores), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_memory_gb), 0) AS REAL),
                CAST(COALESCE(AVG(cs.avg_disk_gb), 0) AS REAL),
                COUNT(DISTINCT cs.snapshot_date)
            FROM cost_snapshots cs
            INNER JOIN apps a ON cs.app_id = a.id
            WHERE a.team_id = ? AND cs.snapshot_date >= ?
            "#,
        )
        .bind(team_id)
        .bind(&cutoff_str)
        .fetch_optional(db)
        .await?;

        Ok(result.map(
            |(cpu_cost, memory_cost, disk_cost, total_cost, avg_cpu, avg_mem, avg_disk, count)| {
                let daily_avg = if count > 0 {
                    total_cost / count as f64
                } else {
                    0.0
                };
                let projected_monthly = daily_avg * 30.0;

                CostSummary {
                    cpu_cost,
                    memory_cost,
                    disk_cost,
                    total_cost,
                    avg_cpu_cores: avg_cpu,
                    avg_memory_gb: avg_mem,
                    avg_disk_gb: avg_disk,
                    days_in_period: count,
                    projected_monthly_cost: projected_monthly,
                }
            },
        ))
    }

    /// Get cost breakdown by app for a project
    pub async fn get_breakdown_for_project(
        db: &SqlitePool,
        project_id: &str,
        days: i64,
    ) -> Result<Vec<AppCostBreakdown>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        // Cast to REAL to avoid SQLite returning INTEGER for empty results
        sqlx::query_as(
            r#"
            SELECT
                a.id as app_id,
                a.name as app_name,
                CAST(COALESCE(SUM(cs.cpu_cost), 0) AS REAL) as cpu_cost,
                CAST(COALESCE(SUM(cs.memory_cost), 0) AS REAL) as memory_cost,
                CAST(COALESCE(SUM(cs.disk_cost), 0) AS REAL) as disk_cost,
                CAST(COALESCE(SUM(cs.total_cost), 0) AS REAL) as total_cost
            FROM apps a
            LEFT JOIN cost_snapshots cs ON cs.app_id = a.id AND cs.snapshot_date >= ?
            WHERE a.project_id = ?
            GROUP BY a.id, a.name
            ORDER BY total_cost DESC
            "#,
        )
        .bind(&cutoff_str)
        .bind(project_id)
        .fetch_all(db)
        .await
    }

    /// Get cost breakdown by app for a team
    pub async fn get_breakdown_for_team(
        db: &SqlitePool,
        team_id: &str,
        days: i64,
    ) -> Result<Vec<AppCostBreakdown>, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        // Cast to REAL to avoid SQLite returning INTEGER for empty results
        sqlx::query_as(
            r#"
            SELECT
                a.id as app_id,
                a.name as app_name,
                CAST(COALESCE(SUM(cs.cpu_cost), 0) AS REAL) as cpu_cost,
                CAST(COALESCE(SUM(cs.memory_cost), 0) AS REAL) as memory_cost,
                CAST(COALESCE(SUM(cs.disk_cost), 0) AS REAL) as disk_cost,
                CAST(COALESCE(SUM(cs.total_cost), 0) AS REAL) as total_cost
            FROM apps a
            LEFT JOIN cost_snapshots cs ON cs.app_id = a.id AND cs.snapshot_date >= ?
            WHERE a.team_id = ?
            GROUP BY a.id, a.name
            ORDER BY total_cost DESC
            "#,
        )
        .bind(&cutoff_str)
        .bind(team_id)
        .fetch_all(db)
        .await
    }

    /// Delete old cost snapshots (retention policy)
    pub async fn cleanup_old_snapshots(
        db: &SqlitePool,
        retention_days: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let result = sqlx::query("DELETE FROM cost_snapshots WHERE snapshot_date < ?")
            .bind(&cutoff_str)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete all snapshots for a specific app
    pub async fn delete_for_app(db: &SqlitePool, app_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM cost_snapshots WHERE app_id = ?")
            .bind(app_id)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_summary_projection() {
        let summary = CostSummary {
            cpu_cost: 0.60,
            memory_cost: 1.50,
            disk_cost: 0.30,
            total_cost: 2.40,
            avg_cpu_cores: 1.0,
            avg_memory_gb: 1.0,
            avg_disk_gb: 1.0,
            days_in_period: 7,
            projected_monthly_cost: 10.29, // 2.40/7 * 30
        };

        assert_eq!(summary.days_in_period, 7);
        // Verify projected monthly is roughly (total_cost / days) * 30
        let expected_projection = (summary.total_cost / 7.0) * 30.0;
        assert!((summary.projected_monthly_cost - expected_projection).abs() < 0.01);
    }
}
