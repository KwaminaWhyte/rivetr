//! Alert configuration models for threshold-based resource alerts.
//!
//! This module provides database models and queries for managing per-app
//! alert configurations and global default thresholds.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Metric types that can have alerts configured
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    Cpu,
    Memory,
    Disk,
}

impl MetricType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MetricType::Cpu => "cpu",
            MetricType::Memory => "memory",
            MetricType::Disk => "disk",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cpu" => Some(MetricType::Cpu),
            "memory" => Some(MetricType::Memory),
            "disk" => Some(MetricType::Disk),
            _ => None,
        }
    }
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Per-app alert configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertConfig {
    pub id: String,
    pub app_id: Option<String>,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub enabled: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Response format for alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfigResponse {
    pub id: String,
    pub app_id: Option<String>,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AlertConfig> for AlertConfigResponse {
    fn from(config: AlertConfig) -> Self {
        Self {
            id: config.id,
            app_id: config.app_id,
            metric_type: config.metric_type,
            threshold_percent: config.threshold_percent,
            enabled: config.enabled != 0,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Request to create a new alert configuration
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAlertConfigRequest {
    pub metric_type: String,
    pub threshold_percent: f64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update an alert configuration
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAlertConfigRequest {
    pub threshold_percent: Option<f64>,
    pub enabled: Option<bool>,
}

impl AlertConfig {
    /// Create a new alert configuration for an app
    pub async fn create(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
        threshold_percent: f64,
        enabled: bool,
    ) -> Result<AlertConfig, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let enabled_int = if enabled { 1i64 } else { 0i64 };

        sqlx::query(
            r#"
            INSERT INTO alert_configs (id, app_id, metric_type, threshold_percent, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(app_id)
        .bind(metric_type)
        .bind(threshold_percent)
        .bind(enabled_int)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;

        Self::get_by_id(db, &id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Get an alert configuration by ID
    pub async fn get_by_id(db: &SqlitePool, id: &str) -> Result<Option<AlertConfig>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, enabled, created_at, updated_at
            FROM alert_configs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(db)
        .await
    }

    /// Get all alert configurations for an app
    pub async fn list_for_app(
        db: &SqlitePool,
        app_id: &str,
    ) -> Result<Vec<AlertConfig>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, enabled, created_at, updated_at
            FROM alert_configs
            WHERE app_id = ?
            ORDER BY metric_type ASC
            "#,
        )
        .bind(app_id)
        .fetch_all(db)
        .await
    }

    /// Get alert configuration for an app and metric type
    pub async fn get_for_app_metric(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<Option<AlertConfig>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, app_id, metric_type, threshold_percent, enabled, created_at, updated_at
            FROM alert_configs
            WHERE app_id = ? AND metric_type = ?
            "#,
        )
        .bind(app_id)
        .bind(metric_type)
        .fetch_optional(db)
        .await
    }

    /// Update an alert configuration
    pub async fn update(
        db: &SqlitePool,
        id: &str,
        threshold_percent: Option<f64>,
        enabled: Option<bool>,
    ) -> Result<AlertConfig, sqlx::Error> {
        let existing = Self::get_by_id(db, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let new_threshold = threshold_percent.unwrap_or(existing.threshold_percent);
        let new_enabled = enabled
            .map(|e| if e { 1i64 } else { 0i64 })
            .unwrap_or(existing.enabled);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE alert_configs
            SET threshold_percent = ?, enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_threshold)
        .bind(new_enabled)
        .bind(&now)
        .bind(id)
        .execute(db)
        .await?;

        Self::get_by_id(db, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Delete an alert configuration
    pub async fn delete(db: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM alert_configs WHERE id = ?")
            .bind(id)
            .execute(db)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all alert configurations for an app
    pub async fn delete_for_app(db: &SqlitePool, app_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM alert_configs WHERE app_id = ?")
            .bind(app_id)
            .execute(db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get count of apps with custom alert configurations
    pub async fn count_apps_with_custom_configs(db: &SqlitePool) -> Result<i64, sqlx::Error> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT app_id) FROM alert_configs WHERE app_id IS NOT NULL
            "#,
        )
        .fetch_one(db)
        .await?;

        Ok(count)
    }

    /// Get effective threshold for an app and metric type
    /// Returns per-app config if exists, otherwise returns global default
    pub async fn get_effective_threshold(
        db: &SqlitePool,
        app_id: &str,
        metric_type: &str,
    ) -> Result<Option<(f64, bool)>, sqlx::Error> {
        // First check for per-app config
        if let Some(config) = Self::get_for_app_metric(db, app_id, metric_type).await? {
            return Ok(Some((config.threshold_percent, config.enabled != 0)));
        }

        // Fall back to global default
        if let Some(default) = GlobalAlertDefault::get_by_metric_type(db, metric_type).await? {
            return Ok(Some((default.threshold_percent, default.enabled != 0)));
        }

        Ok(None)
    }
}

/// Global alert default configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GlobalAlertDefault {
    pub id: String,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub enabled: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Response format for global alert default
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAlertDefaultResponse {
    pub id: String,
    pub metric_type: String,
    pub threshold_percent: f64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GlobalAlertDefault> for GlobalAlertDefaultResponse {
    fn from(default: GlobalAlertDefault) -> Self {
        Self {
            id: default.id,
            metric_type: default.metric_type,
            threshold_percent: default.threshold_percent,
            enabled: default.enabled != 0,
            created_at: default.created_at,
            updated_at: default.updated_at,
        }
    }
}

/// Request to update global alert defaults
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGlobalAlertDefaultsRequest {
    pub cpu: Option<GlobalAlertDefaultUpdate>,
    pub memory: Option<GlobalAlertDefaultUpdate>,
    pub disk: Option<GlobalAlertDefaultUpdate>,
}

/// Update for a single metric type's global default
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalAlertDefaultUpdate {
    pub threshold_percent: Option<f64>,
    pub enabled: Option<bool>,
}

/// Response containing all global alert defaults
#[derive(Debug, Clone, Serialize)]
pub struct GlobalAlertDefaultsResponse {
    pub cpu: Option<GlobalAlertDefaultResponse>,
    pub memory: Option<GlobalAlertDefaultResponse>,
    pub disk: Option<GlobalAlertDefaultResponse>,
}

impl GlobalAlertDefault {
    /// Get all global alert defaults
    pub async fn list_all(db: &SqlitePool) -> Result<Vec<GlobalAlertDefault>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, metric_type, threshold_percent, enabled, created_at, updated_at
            FROM global_alert_defaults
            ORDER BY metric_type ASC
            "#,
        )
        .fetch_all(db)
        .await
    }

    /// Get all defaults as a structured response
    pub async fn get_all_as_response(
        db: &SqlitePool,
    ) -> Result<GlobalAlertDefaultsResponse, sqlx::Error> {
        let defaults = Self::list_all(db).await?;

        let mut response = GlobalAlertDefaultsResponse {
            cpu: None,
            memory: None,
            disk: None,
        };

        for default in defaults {
            let resp = GlobalAlertDefaultResponse::from(default.clone());
            match default.metric_type.as_str() {
                "cpu" => response.cpu = Some(resp),
                "memory" => response.memory = Some(resp),
                "disk" => response.disk = Some(resp),
                _ => {}
            }
        }

        Ok(response)
    }

    /// Get global default by metric type
    pub async fn get_by_metric_type(
        db: &SqlitePool,
        metric_type: &str,
    ) -> Result<Option<GlobalAlertDefault>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, metric_type, threshold_percent, enabled, created_at, updated_at
            FROM global_alert_defaults
            WHERE metric_type = ?
            "#,
        )
        .bind(metric_type)
        .fetch_optional(db)
        .await
    }

    /// Update a global alert default
    pub async fn update(
        db: &SqlitePool,
        metric_type: &str,
        threshold_percent: Option<f64>,
        enabled: Option<bool>,
    ) -> Result<GlobalAlertDefault, sqlx::Error> {
        let existing = Self::get_by_metric_type(db, metric_type)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let new_threshold = threshold_percent.unwrap_or(existing.threshold_percent);
        let new_enabled = enabled
            .map(|e| if e { 1i64 } else { 0i64 })
            .unwrap_or(existing.enabled);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE global_alert_defaults
            SET threshold_percent = ?, enabled = ?, updated_at = ?
            WHERE metric_type = ?
            "#,
        )
        .bind(new_threshold)
        .bind(new_enabled)
        .bind(&now)
        .bind(metric_type)
        .execute(db)
        .await?;

        Self::get_by_metric_type(db, metric_type)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Update all global alert defaults from a request
    pub async fn update_all(
        db: &SqlitePool,
        request: &UpdateGlobalAlertDefaultsRequest,
    ) -> Result<GlobalAlertDefaultsResponse, sqlx::Error> {
        if let Some(cpu) = &request.cpu {
            Self::update(db, "cpu", cpu.threshold_percent, cpu.enabled).await?;
        }
        if let Some(memory) = &request.memory {
            Self::update(db, "memory", memory.threshold_percent, memory.enabled).await?;
        }
        if let Some(disk) = &request.disk {
            Self::update(db, "disk", disk.threshold_percent, disk.enabled).await?;
        }

        Self::get_all_as_response(db).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type_roundtrip() {
        assert_eq!(MetricType::Cpu.as_str(), "cpu");
        assert_eq!(MetricType::Memory.as_str(), "memory");
        assert_eq!(MetricType::Disk.as_str(), "disk");

        assert_eq!(MetricType::from_str("cpu"), Some(MetricType::Cpu));
        assert_eq!(MetricType::from_str("CPU"), Some(MetricType::Cpu));
        assert_eq!(MetricType::from_str("memory"), Some(MetricType::Memory));
        assert_eq!(MetricType::from_str("disk"), Some(MetricType::Disk));
        assert_eq!(MetricType::from_str("invalid"), None);
    }

    #[test]
    fn test_alert_config_response_conversion() {
        let config = AlertConfig {
            id: "test-id".to_string(),
            app_id: Some("app-1".to_string()),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            enabled: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let response: AlertConfigResponse = config.into();
        assert!(response.enabled);
        assert_eq!(response.threshold_percent, 80.0);
    }

    #[test]
    fn test_global_alert_default_response_conversion() {
        let default = GlobalAlertDefault {
            id: "default-cpu".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            enabled: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let response: GlobalAlertDefaultResponse = default.into();
        assert!(!response.enabled);
        assert_eq!(response.threshold_percent, 80.0);
    }
}
