//! Instance settings model.
//!
//! Provides a simple key-value store for instance-level configuration
//! (instance domain, instance name, etc.) that the admin can change at runtime.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A single row in the `instance_settings` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InstanceSettingRow {
    pub key: String,
    pub value: Option<String>,
    pub updated_at: String,
}

/// API response / request body for instance settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSettings {
    /// The domain where this Rivetr instance is accessible (e.g. "rivetr.example.com").
    pub instance_domain: Option<String>,
    /// A human-readable name for this instance (e.g. "My Rivetr").
    pub instance_name: Option<String>,
    /// How many old deployments to keep per app before pruning (default: 5).
    pub max_deployments_per_app: Option<u32>,
    /// Whether to prune unused Docker images after each cleanup cycle (default: true).
    pub prune_images: Option<bool>,
    /// IANA timezone for this Rivetr instance (e.g. "UTC", "America/New_York").
    pub instance_timezone: Option<String>,
    /// AI provider: "claude" | "openai" | "gemini" | "moonshot"
    pub ai_provider: Option<String>,
    /// API key for the selected AI provider (stored in DB, set from dashboard).
    /// Returned as null in GET responses to avoid leaking the key.
    #[serde(skip_serializing)]
    pub ai_api_key: Option<String>,
    /// Whether an AI API key is currently configured (safe to expose).
    pub ai_configured: bool,
    /// Optional model override (defaults per provider if not set).
    pub ai_model: Option<String>,
    /// Max output tokens for AI requests (default: 2048).
    pub ai_max_tokens: Option<u32>,
}

/// Request body for updating instance settings (all fields optional).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateInstanceSettingsRequest {
    pub instance_domain: Option<String>,
    pub instance_name: Option<String>,
    pub max_deployments_per_app: Option<u32>,
    pub prune_images: Option<bool>,
    pub instance_timezone: Option<String>,
    /// AI provider: "claude" | "openai" | "gemini" | "moonshot"
    pub ai_provider: Option<String>,
    /// Set to Some("") to clear the key, Some("sk-...") to set it.
    pub ai_api_key: Option<String>,
    pub ai_model: Option<String>,
    pub ai_max_tokens: Option<u32>,
}

impl InstanceSettings {
    /// Load all instance settings from the database.
    pub async fn load(db: &SqlitePool) -> Result<Self, sqlx::Error> {
        let rows: Vec<InstanceSettingRow> =
            sqlx::query_as("SELECT key, value, updated_at FROM instance_settings")
                .fetch_all(db)
                .await?;

        let mut settings = Self {
            instance_domain: None,
            instance_name: None,
            max_deployments_per_app: None,
            prune_images: None,
            instance_timezone: None,
            ai_provider: None,
            ai_api_key: None,
            ai_configured: false,
            ai_model: None,
            ai_max_tokens: None,
        };

        for row in rows {
            match row.key.as_str() {
                "instance_domain" => settings.instance_domain = row.value,
                "instance_name" => settings.instance_name = row.value,
                "max_deployments_per_app" => {
                    settings.max_deployments_per_app =
                        row.value.as_deref().and_then(|v| v.parse().ok())
                }
                "prune_images" => {
                    settings.prune_images = row.value.as_deref().map(|v| v != "false" && v != "0")
                }
                "instance_timezone" => settings.instance_timezone = row.value,
                "ai_provider" => settings.ai_provider = row.value,
                "ai_api_key" => {
                    settings.ai_configured =
                        row.value.as_deref().map(|v| !v.is_empty()).unwrap_or(false);
                    settings.ai_api_key = row.value;
                }
                "ai_model" => settings.ai_model = row.value,
                "ai_max_tokens" => {
                    settings.ai_max_tokens = row.value.as_deref().and_then(|v| v.parse().ok())
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    /// Persist updated values.  Only the fields present in the request are written.
    pub async fn update(
        db: &SqlitePool,
        req: &UpdateInstanceSettingsRequest,
    ) -> Result<Self, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        if let Some(domain) = &req.instance_domain {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('instance_domain', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if domain.is_empty() { None } else { Some(domain.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(name) = &req.instance_name {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('instance_name', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if name.is_empty() { None } else { Some(name.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(max) = req.max_deployments_per_app {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('max_deployments_per_app', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(max.to_string())
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(prune) = req.prune_images {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('prune_images', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if prune { "true" } else { "false" })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(tz) = &req.instance_timezone {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('instance_timezone', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if tz.is_empty() { None } else { Some(tz.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(provider) = &req.ai_provider {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('ai_provider', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if provider.is_empty() { None } else { Some(provider.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(key) = &req.ai_api_key {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('ai_api_key', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if key.is_empty() { None } else { Some(key.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(model) = &req.ai_model {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('ai_model', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(if model.is_empty() { None } else { Some(model.as_str()) })
            .bind(&now)
            .execute(db)
            .await?;
        }

        if let Some(max_tokens) = req.ai_max_tokens {
            sqlx::query(
                r#"
                INSERT INTO instance_settings (key, value, updated_at)
                VALUES ('ai_max_tokens', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(max_tokens.to_string())
            .bind(&now)
            .execute(db)
            .await?;
        }

        Self::load(db).await
    }
}
