//! Log drain model for forwarding container logs to external services.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Log drain provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogDrainProvider {
    Axiom,
    #[serde(rename = "newrelic")]
    NewRelic,
    Http,
    Datadog,
    Logtail,
}

impl std::fmt::Display for LogDrainProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Axiom => write!(f, "axiom"),
            Self::NewRelic => write!(f, "newrelic"),
            Self::Http => write!(f, "http"),
            Self::Datadog => write!(f, "datadog"),
            Self::Logtail => write!(f, "logtail"),
        }
    }
}

impl std::str::FromStr for LogDrainProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "axiom" => Ok(Self::Axiom),
            "newrelic" => Ok(Self::NewRelic),
            "http" => Ok(Self::Http),
            "datadog" => Ok(Self::Datadog),
            "logtail" => Ok(Self::Logtail),
            _ => Err(format!("Unknown log drain provider: {}", s)),
        }
    }
}

/// Axiom log drain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomConfig {
    pub dataset: String,
    pub api_token: String,
}

/// New Relic log drain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRelicConfig {
    pub api_key: String,
    /// Region: "us" or "eu"
    #[serde(default = "default_us_region")]
    pub region: String,
}

fn default_us_region() -> String {
    "us".to_string()
}

/// Datadog log drain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatadogConfig {
    pub api_key: String,
    /// Site: "datadoghq.com", "datadoghq.eu", etc.
    #[serde(default = "default_datadog_site")]
    pub site: String,
}

fn default_datadog_site() -> String {
    "datadoghq.com".to_string()
}

/// Logtail (Better Stack) log drain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogtailConfig {
    pub source_token: String,
}

/// Generic HTTP log drain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpDrainConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_header_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_header_value: Option<String>,
}

/// Log drain stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogDrain {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub provider: String,
    pub config: String,
    pub enabled: i32,
    pub last_sent_at: Option<String>,
    pub error_count: i32,
    pub last_error: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl LogDrain {
    /// Get the provider type enum
    pub fn get_provider(&self) -> LogDrainProvider {
        self.provider.parse().unwrap_or(LogDrainProvider::Http)
    }

    /// Check if the drain is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }

    /// Parse the config as AxiomConfig
    pub fn get_axiom_config(&self) -> Option<AxiomConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as NewRelicConfig
    pub fn get_newrelic_config(&self) -> Option<NewRelicConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as DatadogConfig
    pub fn get_datadog_config(&self) -> Option<DatadogConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as LogtailConfig
    pub fn get_logtail_config(&self) -> Option<LogtailConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as HttpDrainConfig
    pub fn get_http_config(&self) -> Option<HttpDrainConfig> {
        serde_json::from_str(&self.config).ok()
    }
}

/// Response DTO for LogDrain (masks sensitive config data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogDrainResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub provider: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub last_sent_at: Option<String>,
    pub error_count: i32,
    pub last_error: Option<String>,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<LogDrain> for LogDrainResponse {
    fn from(drain: LogDrain) -> Self {
        let config: serde_json::Value =
            serde_json::from_str(&drain.config).unwrap_or(serde_json::Value::Null);

        // Sanitize sensitive fields
        let sanitized_config = if let serde_json::Value::Object(mut obj) = config {
            // Mask API keys, tokens, and auth values
            for key in ["api_key", "api_token", "source_token", "auth_header_value"] {
                if obj.contains_key(key) {
                    obj.insert(key.to_string(), serde_json::json!("********"));
                }
            }
            serde_json::Value::Object(obj)
        } else {
            config
        };

        Self {
            id: drain.id,
            app_id: drain.app_id,
            name: drain.name,
            provider: drain.provider,
            config: sanitized_config,
            enabled: drain.enabled != 0,
            last_sent_at: drain.last_sent_at,
            error_count: drain.error_count,
            last_error: drain.last_error,
            team_id: drain.team_id,
            created_at: drain.created_at,
            updated_at: drain.updated_at,
        }
    }
}

/// Request to create a log drain
#[derive(Debug, Deserialize)]
pub struct CreateLogDrainRequest {
    pub name: String,
    pub provider: LogDrainProvider,
    pub config: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a log drain
#[derive(Debug, Deserialize)]
pub struct UpdateLogDrainRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}
