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
    Webhook,
    Telegram,
    Teams,
    Pushover,
    Ntfy,
    Mattermost,
    Lark,
    Gotify,
    Resend,
}

impl std::fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Discord => write!(f, "discord"),
            Self::Email => write!(f, "email"),
            Self::Webhook => write!(f, "webhook"),
            Self::Telegram => write!(f, "telegram"),
            Self::Teams => write!(f, "teams"),
            Self::Pushover => write!(f, "pushover"),
            Self::Ntfy => write!(f, "ntfy"),
            Self::Mattermost => write!(f, "mattermost"),
            Self::Lark => write!(f, "lark"),
            Self::Gotify => write!(f, "gotify"),
            Self::Resend => write!(f, "resend"),
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
            "webhook" => Ok(Self::Webhook),
            "telegram" => Ok(Self::Telegram),
            "teams" => Ok(Self::Teams),
            "pushover" => Ok(Self::Pushover),
            "ntfy" => Ok(Self::Ntfy),
            "mattermost" => Ok(Self::Mattermost),
            "lark" => Ok(Self::Lark),
            "gotify" => Ok(Self::Gotify),
            "resend" => Ok(Self::Resend),
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

/// Telegram Bot API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<i64>,
}

/// Microsoft Teams Incoming Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    pub webhook_url: String,
}

/// Pushover API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushoverConfig {
    pub user_key: String,
    pub app_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Priority: -2 (silent) to 2 (emergency), default 0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Ntfy notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtfyConfig {
    /// Server URL, defaults to "https://ntfy.sh" if not set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    pub topic: String,
    /// Priority: 1 (min) to 5 (max), default 3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// Comma-separated tags for the notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

/// Mattermost Incoming Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MattermostConfig {
    pub webhook_url: String,
}

/// Lark (Feishu) custom bot webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkConfig {
    pub webhook_url: String,
}

/// Gotify push notification server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotifyConfig {
    /// Base URL of the Gotify server (e.g., "https://gotify.example.com")
    pub server_url: String,
    /// Gotify application token
    pub app_token: String,
    /// Message priority (default: 5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// Resend email API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendConfig {
    /// Resend API key (starts with "re_")
    pub api_key: String,
    /// From address (must be a verified sender in Resend)
    pub from_address: String,
    /// List of recipient email addresses
    pub to_addresses: Vec<String>,
}

/// Generic webhook configuration with headers and payload template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL (must be HTTPS)
    pub url: String,
    /// Optional custom headers (key-value pairs)
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    /// Payload template type: "json", "slack", "discord", or "custom"
    #[serde(default = "default_payload_template")]
    pub payload_template: String,
    /// Custom payload template (used when payload_template is "custom")
    /// Supports variables: {{app_name}}, {{metric_type}}, {{value}}, {{threshold}}, {{timestamp}}, {{severity}}
    pub custom_template: Option<String>,
}

fn default_payload_template() -> String {
    "json".to_string()
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
    /// Optional team ID for team-scoped channels (NULL for global channels)
    pub team_id: Option<String>,
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

    /// Parse the config as WebhookConfig
    pub fn get_webhook_config(&self) -> Option<WebhookConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as TelegramConfig
    pub fn get_telegram_config(&self) -> Option<TelegramConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as TeamsConfig
    pub fn get_teams_config(&self) -> Option<TeamsConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as PushoverConfig
    pub fn get_pushover_config(&self) -> Option<PushoverConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as NtfyConfig
    pub fn get_ntfy_config(&self) -> Option<NtfyConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as MattermostConfig
    pub fn get_mattermost_config(&self) -> Option<MattermostConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as LarkConfig
    pub fn get_lark_config(&self) -> Option<LarkConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as GotifyConfig
    pub fn get_gotify_config(&self) -> Option<GotifyConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as ResendConfig
    pub fn get_resend_config(&self) -> Option<ResendConfig> {
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
    pub team_id: Option<String>,
}

impl From<NotificationChannel> for NotificationChannelResponse {
    fn from(channel: NotificationChannel) -> Self {
        // Parse and sanitize the config (mask passwords and sensitive headers)
        let config: serde_json::Value =
            serde_json::from_str(&channel.config).unwrap_or(serde_json::Value::Null);

        // Sanitize based on channel type
        let sanitized_config = match channel.channel_type.as_str() {
            "email" => {
                if let serde_json::Value::Object(mut obj) = config {
                    if obj.contains_key("smtp_password") {
                        obj.insert("smtp_password".to_string(), serde_json::json!("********"));
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            "webhook" => {
                if let serde_json::Value::Object(mut obj) = config {
                    // Mask any headers that might contain sensitive values (Authorization, etc.)
                    if let Some(serde_json::Value::Object(headers)) = obj.get("headers").cloned() {
                        let mut masked_headers = serde_json::Map::new();
                        for (key, value) in &headers {
                            let lower_key = key.to_lowercase();
                            if lower_key.contains("auth")
                                || lower_key.contains("secret")
                                || lower_key.contains("token")
                                || lower_key.contains("key")
                            {
                                masked_headers.insert(key.clone(), serde_json::json!("********"));
                            } else {
                                masked_headers.insert(key.clone(), value.clone());
                            }
                        }
                        obj.insert(
                            "headers".to_string(),
                            serde_json::Value::Object(masked_headers),
                        );
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            "telegram" => {
                if let serde_json::Value::Object(mut obj) = config {
                    if obj.contains_key("bot_token") {
                        obj.insert("bot_token".to_string(), serde_json::json!("********"));
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            "pushover" => {
                if let serde_json::Value::Object(mut obj) = config {
                    if obj.contains_key("app_token") {
                        obj.insert("app_token".to_string(), serde_json::json!("********"));
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            "gotify" => {
                if let serde_json::Value::Object(mut obj) = config {
                    if obj.contains_key("app_token") {
                        obj.insert("app_token".to_string(), serde_json::json!("********"));
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            "resend" => {
                if let serde_json::Value::Object(mut obj) = config {
                    if obj.contains_key("api_key") {
                        obj.insert("api_key".to_string(), serde_json::json!("********"));
                    }
                    serde_json::Value::Object(obj)
                } else {
                    config
                }
            }
            _ => config,
        };

        Self {
            id: channel.id,
            name: channel.name,
            channel_type: channel.channel_type,
            config: sanitized_config,
            enabled: channel.enabled != 0,
            created_at: channel.created_at,
            updated_at: channel.updated_at,
            team_id: channel.team_id,
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
