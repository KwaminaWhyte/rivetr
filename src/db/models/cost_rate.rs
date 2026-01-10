//! Cost rate models for resource cost estimation.
//!
//! This module provides database models and queries for managing cost rates
//! used to estimate infrastructure costs based on resource usage.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Resource types that can have cost rates configured
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Cpu => "cpu",
            ResourceType::Memory => "memory",
            ResourceType::Disk => "disk",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cpu" => Some(ResourceType::Cpu),
            "memory" => Some(ResourceType::Memory),
            "disk" => Some(ResourceType::Disk),
            _ => None,
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Cost rate configuration for a resource type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CostRate {
    pub id: String,
    pub resource_type: String,
    pub rate_per_unit: f64,
    pub unit_description: String,
    pub is_default: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Response format for cost rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRateResponse {
    pub id: String,
    pub resource_type: String,
    pub rate_per_unit: f64,
    pub unit_description: String,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CostRate> for CostRateResponse {
    fn from(rate: CostRate) -> Self {
        Self {
            id: rate.id,
            resource_type: rate.resource_type,
            rate_per_unit: rate.rate_per_unit,
            unit_description: rate.unit_description,
            is_default: rate.is_default != 0,
            created_at: rate.created_at,
            updated_at: rate.updated_at,
        }
    }
}

/// Update for a single resource type's cost rate
#[derive(Debug, Clone, Deserialize)]
pub struct CostRateUpdate {
    pub rate_per_unit: Option<f64>,
    pub unit_description: Option<String>,
}

/// Request to update cost rates
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCostRatesRequest {
    pub cpu: Option<CostRateUpdate>,
    pub memory: Option<CostRateUpdate>,
    pub disk: Option<CostRateUpdate>,
}

/// Response containing all cost rates
#[derive(Debug, Clone, Serialize)]
pub struct CostRatesResponse {
    pub cpu: Option<CostRateResponse>,
    pub memory: Option<CostRateResponse>,
    pub disk: Option<CostRateResponse>,
}

impl CostRate {
    /// Get all cost rates
    pub async fn list_all(db: &SqlitePool) -> Result<Vec<CostRate>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, resource_type, rate_per_unit, unit_description, is_default, created_at, updated_at
            FROM cost_rates
            ORDER BY resource_type ASC
            "#,
        )
        .fetch_all(db)
        .await
    }

    /// Get all cost rates as a structured response
    pub async fn get_all_as_response(db: &SqlitePool) -> Result<CostRatesResponse, sqlx::Error> {
        let rates = Self::list_all(db).await?;

        let mut response = CostRatesResponse {
            cpu: None,
            memory: None,
            disk: None,
        };

        for rate in rates {
            let resp = CostRateResponse::from(rate.clone());
            match rate.resource_type.as_str() {
                "cpu" => response.cpu = Some(resp),
                "memory" => response.memory = Some(resp),
                "disk" => response.disk = Some(resp),
                _ => {}
            }
        }

        Ok(response)
    }

    /// Get cost rate by resource type
    pub async fn get_by_resource_type(
        db: &SqlitePool,
        resource_type: &str,
    ) -> Result<Option<CostRate>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, resource_type, rate_per_unit, unit_description, is_default, created_at, updated_at
            FROM cost_rates
            WHERE resource_type = ?
            "#,
        )
        .bind(resource_type)
        .fetch_optional(db)
        .await
    }

    /// Update a cost rate
    pub async fn update(
        db: &SqlitePool,
        resource_type: &str,
        rate_per_unit: Option<f64>,
        unit_description: Option<String>,
    ) -> Result<CostRate, sqlx::Error> {
        let existing = Self::get_by_resource_type(db, resource_type)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // When admin customizes a rate, mark it as no longer default
        let is_default = if rate_per_unit.is_some() || unit_description.is_some() {
            0i64
        } else {
            existing.is_default
        };

        let new_rate = rate_per_unit.unwrap_or(existing.rate_per_unit);
        let new_unit_description = unit_description.unwrap_or(existing.unit_description);
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE cost_rates
            SET rate_per_unit = ?, unit_description = ?, is_default = ?, updated_at = ?
            WHERE resource_type = ?
            "#,
        )
        .bind(new_rate)
        .bind(&new_unit_description)
        .bind(is_default)
        .bind(&now)
        .bind(resource_type)
        .execute(db)
        .await?;

        Self::get_by_resource_type(db, resource_type)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Update all cost rates from a request
    pub async fn update_all(
        db: &SqlitePool,
        request: &UpdateCostRatesRequest,
    ) -> Result<CostRatesResponse, sqlx::Error> {
        if let Some(cpu) = &request.cpu {
            Self::update(db, "cpu", cpu.rate_per_unit, cpu.unit_description.clone()).await?;
        }
        if let Some(memory) = &request.memory {
            Self::update(
                db,
                "memory",
                memory.rate_per_unit,
                memory.unit_description.clone(),
            )
            .await?;
        }
        if let Some(disk) = &request.disk {
            Self::update(
                db,
                "disk",
                disk.rate_per_unit,
                disk.unit_description.clone(),
            )
            .await?;
        }

        Self::get_all_as_response(db).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_roundtrip() {
        assert_eq!(ResourceType::Cpu.as_str(), "cpu");
        assert_eq!(ResourceType::Memory.as_str(), "memory");
        assert_eq!(ResourceType::Disk.as_str(), "disk");

        assert_eq!(ResourceType::from_str("cpu"), Some(ResourceType::Cpu));
        assert_eq!(ResourceType::from_str("CPU"), Some(ResourceType::Cpu));
        assert_eq!(ResourceType::from_str("memory"), Some(ResourceType::Memory));
        assert_eq!(ResourceType::from_str("disk"), Some(ResourceType::Disk));
        assert_eq!(ResourceType::from_str("invalid"), None);
    }

    #[test]
    fn test_cost_rate_response_conversion() {
        let rate = CostRate {
            id: "default-cpu".to_string(),
            resource_type: "cpu".to_string(),
            rate_per_unit: 0.02,
            unit_description: "USD per CPU core per month".to_string(),
            is_default: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let response: CostRateResponse = rate.into();
        assert!(response.is_default);
        assert_eq!(response.rate_per_unit, 0.02);
        assert_eq!(response.resource_type, "cpu");
    }

    #[test]
    fn test_cost_rate_custom_conversion() {
        let rate = CostRate {
            id: "default-memory".to_string(),
            resource_type: "memory".to_string(),
            rate_per_unit: 0.10,
            unit_description: "Custom rate".to_string(),
            is_default: 0,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let response: CostRateResponse = rate.into();
        assert!(!response.is_default);
        assert_eq!(response.rate_per_unit, 0.10);
    }
}
