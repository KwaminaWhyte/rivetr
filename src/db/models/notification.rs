//! Notification channel and subscription models.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannelType {
    Slack,
    Discord,
    Email,
}

impl std::fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Discord => write!(f, "discord"),
            Self::Email => write!(f, "email"),
        }
    }
}

impl std::str::FromStr for NotificationChannelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slack" => Ok(Self::Slack),
            "discord" => Ok(Self::Discord),
            "email" => Ok(Self::Email),
            _ => Err(format!("Unknown channel type: {}", s)),
        }
    }
}

impl From<String> for NotificationChannelType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Slack)
    }
}

/// Notification event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    DeploymentStarted,
    DeploymentSuccess,
    DeploymentFailed,
    AppStopped,
    AppStarted,
}

impl std::fmt::Display for NotificationEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeploymentStarted => write!(f, "deployment_started"),
            Self::DeploymentSuccess => write!(f, "deployment_success"),
            Self::DeploymentFailed => write!(f, "deployment_failed"),
            Self::AppStopped => write!(f, "app_stopped"),
            Self::AppStarted => write!(f, "app_started"),
        }
    }
}

impl std::str::FromStr for NotificationEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deployment_started" => Ok(Self::DeploymentStarted),
            "deployment_success" => Ok(Self::DeploymentSuccess),
            "deployment_failed" => Ok(Self::DeploymentFailed),
            "app_stopped" => Ok(Self::AppStopped),
            "app_started" => Ok(Self::AppStarted),
            _ => Err(format!("Unknown event type: {}", s)),
        }
    }
}

impl From<String> for NotificationEventType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::DeploymentStarted)
    }
}

/// Slack webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

/// Discord webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

/// Email (SMTP) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_tls: bool,
    pub from_address: String,
    pub to_addresses: Vec<String>,
}

/// Notification channel stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub config: String, // JSON serialized config
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NotificationChannel {
    /// Get the channel type enum
    pub fn get_channel_type(&self) -> NotificationChannelType {
        self.channel_type
            .parse()
            .unwrap_or(NotificationChannelType::Slack)
    }

    /// Check if the channel is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }

    /// Parse the config as SlackConfig
    pub fn get_slack_config(&self) -> Option<SlackConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as DiscordConfig
    pub fn get_discord_config(&self) -> Option<DiscordConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as EmailConfig
    pub fn get_email_config(&self) -> Option<EmailConfig> {
        serde_json::from_str(&self.config).ok()
    }
}

/// Response DTO for NotificationChannel (masks sensitive config data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannelResponse {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value, // Sanitized config
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NotificationChannel> for NotificationChannelResponse {
    fn from(channel: NotificationChannel) -> Self {
        // Parse and sanitize the config (mask passwords)
        let config: serde_json::Value =
            serde_json::from_str(&channel.config).unwrap_or(serde_json::Value::Null);

        // For email config, mask the password
        let sanitized_config = if channel.channel_type == "email" {
            if let serde_json::Value::Object(mut obj) = config {
                if obj.contains_key("smtp_password") {
                    obj.insert("smtp_password".to_string(), serde_json::json!("********"));
                }
                serde_json::Value::Object(obj)
            } else {
                config
            }
        } else {
            config
        };

        Self {
            id: channel.id,
            name: channel.name,
            channel_type: channel.channel_type,
            config: sanitized_config,
            enabled: channel.enabled != 0,
            created_at: channel.created_at,
            updated_at: channel.updated_at,
        }
    }
}

/// Notification subscription stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationSubscription {
    pub id: String,
    pub channel_id: String,
    pub event_type: String,
    pub app_id: Option<String>,
    pub created_at: String,
}

impl NotificationSubscription {
    /// Get the event type enum
    pub fn get_event_type(&self) -> NotificationEventType {
        self.event_type
            .parse()
            .unwrap_or(NotificationEventType::DeploymentStarted)
    }
}

/// Response DTO for NotificationSubscription with app name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubscriptionResponse {
    pub id: String,
    pub channel_id: String,
    pub event_type: String,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub created_at: String,
}

/// Request to create a notification channel
#[derive(Debug, Deserialize)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub config: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a notification channel
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationChannelRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Request to create a notification subscription
#[derive(Debug, Deserialize)]
pub struct CreateNotificationSubscriptionRequest {
    pub event_type: NotificationEventType,
    pub app_id: Option<String>,
}

/// Test notification request
#[derive(Debug, Deserialize)]
pub struct TestNotificationRequest {
    pub message: Option<String>,
}
