//! Alert event models for tracking triggered alerts.
//!
//! This module provides database models and queries for managing alert events,
//! which are created when resource metrics exceed configured thresholds.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Alert status indicating whether the alert is active or resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Firing,
    Resolved,
}

impl AlertStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertStatus::Firing => "firing",
            AlertStatus::Resolved => "resolved",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "firing" => Some(AlertStatus::Firing),
            "resolved" => Some(AlertStatus::Resolved),
            _ => None,
        }
    }
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An alert event record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertEvent {
    pub id: String,
    pub app_id: String,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub current_value: f64,
    pub status: String,
    pub consecutive_breaches: i64,
    pub fired_at: String,
    pub resolved_at: Option<String>,
    pub last_notified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response format for alert events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEventResponse {
    pub id: String,
    pub app_id: String,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub current_value: f64,
    pub status: String,
    pub consecutive_breaches: i64,
    pub fired_at: String,
    pub resolved_at: Option<String>,
    pub last_notified_at: Option<String>,
}

impl From<AlertEvent> for AlertEventResponse {
    fn from(event: AlertEvent) -> Self {
        Self {
            id: event.id,
            app_id: event.app_id,
            metric_type: event.metric_type,
            threshold_percent: event.threshold_percent,
            current_value: event.current_value,
            status: event.status,
            consecutive_breaches: event.consecutive_breaches,
            fired_at: event.fired_at,
            resolved_at: event.resolved_at,
            last_notified_at: event.last_notified_at,
        }
    }
}

impl AlertEvent {
    /// Create a new alert event
    pub async fn create(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
        threshold_percent: f64,
        current_value: f64,
        consecutive_breaches: i64,
    ) -> Result<AlertEvent, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO alert_events (id, app_id, metric_type, threshold_percent, current_value, status, consecutive_breaches, fired_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 'firing', ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(app_id)
        .bind(metric_type)
        .bind(threshold_percent)
        .bind(current_value)
        .bind(consecutive_breaches)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        Self::get_by_id(db, &id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get an alert event by ID
    pub async fn get_by_id(db: &SqlitePool, id: &str) -> Result<Option<AlertEvent>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, current_value, status, consecutive_breaches, fired_at, resolved_at, last_notified_at, created_at, updated_at
            FROM alert_events
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(db)
        .await
    }

    /// Get all alert events for an app
    pub async fn list_for_app(
        db: &SqlitePool,
        app_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<AlertEvent>, sqlx::Error> {
        let limit = limit.unwrap_or(100);
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, current_value, status, consecutive_breaches, fired_at, resolved_at, last_notified_at, created_at, updated_at
            FROM alert_events
            WHERE app_id = ?
            ORDER BY fired_at DESC
            LIMIT ?
            "#,
        )
        .bind(app_id)
        .bind(limit)
        .fetch_all(db)
        .await
    }

    /// Get active (firing) alert for an app and metric type
    pub async fn get_active_for_app_metric(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<Option<AlertEvent>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, current_value, status, consecutive_breaches, fired_at, resolved_at, last_notified_at, created_at, updated_at
            FROM alert_events
            WHERE app_id = ? AND metric_type = ? AND status = 'firing'
            ORDER BY fired_at DESC
            LIMIT 1
            "#,
        )
        .bind(app_id)
        .bind(metric_type)
        .fetch_optional(db)
        .await
    }

    /// Get all active (firing) alerts
    pub async fn list_active(db: &SqlitePool) -> Result<Vec<AlertEvent>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, current_value, status, consecutive_breaches, fired_at, resolved_at, last_notified_at, created_at, updated_at
            FROM alert_events
            WHERE status = 'firing'
            ORDER BY fired_at DESC
            "#,
        )
        .fetch_all(db)
        .await
    }

    /// Update an alert event's current value and breach count
    pub async fn update_value(
        db: &SqlitePool,
        id: &str,
        current_value: f64,
        consecutive_breaches: i64,
    ) -> Result<AlertEvent, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE alert_events
            SET current_value = ?, consecutive_breaches = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(current_value)
        .bind(consecutive_breaches)
        .bind(&now)
        .bind(id)
        .execute(db)
        .await?;

        Self::get_by_id(db, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Resolve an alert event
    pub async fn resolve(db: &SqlitePool, id: &str) -> Result<AlertEvent, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE alert_events
            SET status = 'resolved', resolved_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(db)
        .await?;

        Self::get_by_id(db, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Update the last notified timestamp
    pub async fn set_notified(db: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE alert_events
            SET last_notified_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Check if notification should be sent (not within 15-minute window)
    pub fn should_notify(&self) -> bool {
        match &self.last_notified_at {
            None => true,
            Some(last_notified) => {
                let last = chrono::DateTime::parse_from_rfc3339(last_notified)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                match last {
                    None => true,
                    Some(last_time) => {
                        let now = chrono::Utc::now();
                        let diff = now.signed_duration_since(last_time);
                        diff.num_minutes() >= 15
                    }
                }
            }
        }
    }

    /// Delete old resolved alerts (older than retention period)
    pub async fn cleanup_old_events(
        db: &SqlitePool,
        retention_hours: i64,
    ) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(retention_hours);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

        let result =
            sqlx::query("DELETE FROM alert_events WHERE status = 'resolved' AND resolved_at < ?")
                .bind(&cutoff_str)
                .execute(db)
                .await?;

        Ok(result.rows_affected())
    }

    /// Delete all alerts for a specific app
    pub async fn delete_for_app(db: &SqlitePool, app_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM alert_events WHERE app_id = ?")
            .bind(app_id)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Breach count tracking for hysteresis
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AlertBreachCount {
    pub app_id: String,
    pub metric_type: String,
    pub consecutive_count: i64,
    pub last_checked_at: String,
}

impl AlertBreachCount {
    /// Get or initialize breach count for an app/metric
    pub async fn get_or_create(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<AlertBreachCount, sqlx::Error> {
        // Try to get existing
        let existing: Option<AlertBreachCount> = sqlx::query_as(
            "SELECT app_id, metric_type, consecutive_count, last_checked_at FROM alert_breach_counts WHERE app_id = ? AND metric_type = ?",
        )
        .bind(app_id)
        .bind(metric_type)
        .fetch_optional(db)
        .await?;

        if let Some(count) = existing {
            return Ok(count);
        }

        // Create new
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO alert_breach_counts (app_id, metric_type, consecutive_count, last_checked_at) VALUES (?, ?, 0, ?)",
        )
        .bind(app_id)
        .bind(metric_type)
        .bind(&now)
        .execute(db)
        .await?;

        Ok(AlertBreachCount {
            app_id: app_id.to_string(),
            metric_type: metric_type.to_string(),
            consecutive_count: 0,
            last_checked_at: now,
        })
    }

    /// Increment breach count (called when threshold exceeded)
    pub async fn increment(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<i64, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        // Upsert pattern
        sqlx::query(
            r#"
            INSERT INTO alert_breach_counts (app_id, metric_type, consecutive_count, last_checked_at)
            VALUES (?, ?, 1, ?)
            ON CONFLICT(app_id, metric_type) DO UPDATE SET
                consecutive_count = consecutive_count + 1,
                last_checked_at = ?
            "#,
        )
        .bind(app_id)
        .bind(metric_type)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        // Get updated count
        let count: (i64,) = sqlx::query_as(
            "SELECT consecutive_count FROM alert_breach_counts WHERE app_id = ? AND metric_type = ?",
        )
        .bind(app_id)
        .bind(metric_type)
        .fetch_one(db)
        .await?;

        Ok(count.0)
    }

    /// Reset breach count (called when threshold not exceeded)
    pub async fn reset(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO alert_breach_counts (app_id, metric_type, consecutive_count, last_checked_at)
            VALUES (?, ?, 0, ?)
            ON CONFLICT(app_id, metric_type) DO UPDATE SET
                consecutive_count = 0,
                last_checked_at = ?
            "#,
        )
        .bind(app_id)
        .bind(metric_type)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        Ok(())
    }

    /// Delete breach counts for a specific app
    pub async fn delete_for_app(db: &SqlitePool, app_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM alert_breach_counts WHERE app_id = ?")
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
    fn test_alert_status_roundtrip() {
        assert_eq!(AlertStatus::Firing.as_str(), "firing");
        assert_eq!(AlertStatus::Resolved.as_str(), "resolved");

        assert_eq!(AlertStatus::from_str("firing"), Some(AlertStatus::Firing));
        assert_eq!(
            AlertStatus::from_str("resolved"),
            Some(AlertStatus::Resolved)
        );
        assert_eq!(AlertStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_alert_event_response_conversion() {
        let event = AlertEvent {
            id: "test-id".to_string(),
            app_id: "app-1".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            current_value: 85.5,
            status: "firing".to_string(),
            consecutive_breaches: 2,
            fired_at: "2024-01-01T00:00:00Z".to_string(),
            resolved_at: None,
            last_notified_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let response: AlertEventResponse = event.into();
        assert_eq!(response.status, "firing");
        assert_eq!(response.current_value, 85.5);
        assert_eq!(response.consecutive_breaches, 2);
    }

    #[test]
    fn test_should_notify_no_previous() {
        let event = AlertEvent {
            id: "test-id".to_string(),
            app_id: "app-1".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            current_value: 85.5,
            status: "firing".to_string(),
            consecutive_breaches: 2,
            fired_at: "2024-01-01T00:00:00Z".to_string(),
            resolved_at: None,
            last_notified_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(event.should_notify());
    }

    #[test]
    fn test_should_notify_recent() {
        let now = chrono::Utc::now();
        let event = AlertEvent {
            id: "test-id".to_string(),
            app_id: "app-1".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            current_value: 85.5,
            status: "firing".to_string(),
            consecutive_breaches: 2,
            fired_at: now.to_rfc3339(),
            resolved_at: None,
            last_notified_at: Some(now.to_rfc3339()),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        };

        assert!(!event.should_notify());
    }

    #[test]
    fn test_should_notify_after_window() {
        let old_time = chrono::Utc::now() - chrono::Duration::minutes(20);
        let event = AlertEvent {
            id: "test-id".to_string(),
            app_id: "app-1".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            current_value: 85.5,
            status: "firing".to_string(),
            consecutive_breaches: 2,
            fired_at: old_time.to_rfc3339(),
            resolved_at: None,
            last_notified_at: Some(old_time.to_rfc3339()),
            created_at: old_time.to_rfc3339(),
            updated_at: old_time.to_rfc3339(),
        };

        assert!(event.should_notify());
    }
}
