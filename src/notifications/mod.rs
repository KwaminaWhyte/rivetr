//! Notification system for sending alerts via Slack, Discord, and Email.
//!
//! This module provides a unified interface for sending notifications
//! on deployment events and app state changes.

pub mod alert_notifications;
pub mod email;
pub mod gotify;
pub mod lark;
pub mod mattermost;
pub mod ntfy;
pub mod pushover;
pub mod resend;
pub mod sendry;
pub mod teams;
pub mod telegram;

pub use alert_notifications::{
    spawn_alert_notification_worker, AlertNotificationPayload, AlertNotificationService,
};
pub use email::SystemEmailService;

use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde_json::json;

use crate::db::{
    DiscordConfig, EmailConfig, NotificationChannel, NotificationChannelType,
    NotificationEventType, NotificationSubscription, SlackConfig, WebhookConfig,
};
use crate::DbPool;

/// Notification payload with event details
#[derive(Debug, Clone)]
pub struct NotificationPayload {
    pub event_type: NotificationEventType,
    pub app_id: String,
    pub app_name: String,
    pub deployment_id: Option<String>,
    pub status: String,
    pub message: String,
    pub error_message: Option<String>,
    pub timestamp: String,
    /// Resource scope and extra detail lines (backup events use these)
    pub extras: NotificationExtras,
}

/// Extra context for events that are not about a deployment, such as backups.
#[derive(Debug, Clone, Default)]
pub struct NotificationExtras {
    /// Backup resource kind: database, volume or instance
    pub resource_type: Option<String>,
    /// Backup resource id (database or volume id); matched against subscriptions
    pub resource_id: Option<String>,
    /// Ordered label/value pairs rendered by every channel (size, duration, ...)
    pub fields: Vec<(String, String)>,
}

impl NotificationPayload {
    /// Create a new notification payload for a deployment event
    pub fn deployment_event(
        event_type: NotificationEventType,
        app_id: String,
        app_name: String,
        deployment_id: String,
        status: String,
        message: String,
        error_message: Option<String>,
    ) -> Self {
        Self {
            event_type,
            app_id,
            app_name,
            deployment_id: Some(deployment_id),
            status,
            message,
            error_message,
            timestamp: chrono::Utc::now().to_rfc3339(),
            extras: Default::default(),
        }
    }

    /// Create a new notification payload for an app event
    pub fn app_event(
        event_type: NotificationEventType,
        app_id: String,
        app_name: String,
        message: String,
    ) -> Self {
        let status = event_type.to_string();
        Self {
            event_type,
            app_id,
            app_name,
            deployment_id: None,
            status,
            message,
            error_message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            extras: Default::default(),
        }
    }

    /// Label for the `app_name` field: "Application", or the backup resource kind.
    pub fn subject_label(&self) -> String {
        if self.event_type.is_backup_event() {
            match self.extras.resource_type.as_deref() {
                Some("database") => "Database".to_string(),
                Some("volume") => "Volume".to_string(),
                Some("instance") => "Instance".to_string(),
                _ => "Resource".to_string(),
            }
        } else {
            "Application".to_string()
        }
    }

    /// Extra label/value lines to render after the standard fields.
    pub fn detail_fields(&self) -> &[(String, String)] {
        &self.extras.fields
    }

    /// Detail fields as plain text lines, each starting with a newline.
    pub fn detail_text_lines(&self) -> String {
        self.extras
            .fields
            .iter()
            .map(|(label, value)| format!("\n{}: {}", label, value))
            .collect()
    }

    /// Detail fields as HTML rows for the email templates (values escaped).
    pub fn detail_html_rows(&self) -> String {
        self.extras
            .fields
            .iter()
            .map(|(label, value)| {
                format!(
                    r#"<div class="field"><span class="field-label">{}:</span> <span class="field-value">{}</span></div>"#,
                    escape_html(label),
                    escape_html(value)
                )
            })
            .collect()
    }

    /// Get the title for the notification
    pub fn title(&self) -> String {
        match self.event_type {
            NotificationEventType::DeploymentStarted => {
                format!("Deployment Started: {}", self.app_name)
            }
            NotificationEventType::DeploymentSuccess => {
                format!("Deployment Successful: {}", self.app_name)
            }
            NotificationEventType::DeploymentFailed => {
                format!("Deployment Failed: {}", self.app_name)
            }
            NotificationEventType::AppStopped => format!("App Stopped: {}", self.app_name),
            NotificationEventType::AppStarted => format!("App Started: {}", self.app_name),
            NotificationEventType::ContainerCrash => {
                format!("Container Crashed: {}", self.app_name)
            }
            NotificationEventType::ContainerRestarted => {
                format!("Container Restarted: {}", self.app_name)
            }
            NotificationEventType::BackupSucceeded => {
                format!("Backup Succeeded: {}", self.app_name)
            }
            NotificationEventType::BackupFailed => format!("Backup Failed: {}", self.app_name),
            NotificationEventType::BackupMissed => format!("Backup Missed: {}", self.app_name),
        }
    }

    /// Get the color for the notification (for Slack/Discord)
    pub fn color(&self) -> &'static str {
        match self.event_type {
            NotificationEventType::DeploymentStarted => "#3498db", // Blue
            NotificationEventType::DeploymentSuccess | NotificationEventType::AppStarted => {
                "#2ecc71"
            } // Green
            NotificationEventType::DeploymentFailed | NotificationEventType::ContainerCrash => {
                "#e74c3c"
            } // Red
            NotificationEventType::AppStopped => "#f39c12",        // Orange
            NotificationEventType::ContainerRestarted => "#f39c12", // Orange
            NotificationEventType::BackupSucceeded => "#2ecc71",   // Green
            NotificationEventType::BackupFailed => "#e74c3c",      // Red
            NotificationEventType::BackupMissed => "#f39c12",      // Orange
        }
    }

    /// Get the emoji for the notification
    pub fn emoji(&self) -> &'static str {
        match self.event_type {
            NotificationEventType::DeploymentStarted => ":rocket:",
            NotificationEventType::DeploymentSuccess => ":white_check_mark:",
            NotificationEventType::DeploymentFailed => ":x:",
            NotificationEventType::AppStopped => ":octagonal_sign:",
            NotificationEventType::AppStarted => ":arrow_forward:",
            NotificationEventType::ContainerCrash => ":boom:",
            NotificationEventType::ContainerRestarted => ":arrows_counterclockwise:",
            NotificationEventType::BackupSucceeded => ":floppy_disk:",
            NotificationEventType::BackupFailed => ":x:",
            NotificationEventType::BackupMissed => ":warning:",
        }
    }
}

/// Notification service for sending alerts via various channels
pub struct NotificationService {
    db: DbPool,
    http_client: reqwest::Client,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(db: DbPool) -> Self {
        Self {
            db,
            http_client: reqwest::Client::new(),
        }
    }

    /// Send a notification to all subscribed channels for the given event
    pub async fn send(&self, payload: &NotificationPayload) -> Result<()> {
        // Find all subscriptions for this event type (and optionally app)
        let subscriptions = self.get_matching_subscriptions(payload).await?;

        if subscriptions.is_empty() {
            tracing::debug!(
                event_type = %payload.event_type,
                app_id = %payload.app_id,
                "No subscriptions found for notification event"
            );
            return Ok(());
        }

        // Get unique channel IDs
        let channel_ids: Vec<&str> = subscriptions
            .iter()
            .map(|s| s.channel_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Get channels and send notifications
        for channel_id in channel_ids {
            match self.get_enabled_channel(channel_id).await? {
                Some(channel) => {
                    if let Err(e) = self.send_to_channel(&channel, payload).await {
                        tracing::error!(
                            channel_id = %channel_id,
                            channel_name = %channel.name,
                            error = %e,
                            "Failed to send notification to channel"
                        );
                    }
                }
                None => {
                    tracing::warn!(
                        channel_id = %channel_id,
                        "Channel not found or disabled"
                    );
                }
            }
        }

        Ok(())
    }

    /// Get subscriptions matching the event type and optionally app ID
    async fn get_matching_subscriptions(
        &self,
        payload: &NotificationPayload,
    ) -> Result<Vec<NotificationSubscription>> {
        let event_type = payload.event_type.to_string();

        // Get subscriptions for this event type whose filters all match:
        // app_id NULL (all apps) or the payload's app, resource_type NULL or the
        // payload's resource kind, resource_id NULL or the payload's resource.
        let subscriptions = sqlx::query_as::<_, NotificationSubscription>(
            r#"
            SELECT * FROM notification_subscriptions
            WHERE event_type = ?
              AND (app_id IS NULL OR app_id = ?)
              AND (resource_type IS NULL OR resource_type = ?)
              AND (resource_id IS NULL OR resource_id = ?)
            "#,
        )
        .bind(&event_type)
        .bind(&payload.app_id)
        .bind(&payload.extras.resource_type)
        .bind(&payload.extras.resource_id)
        .fetch_all(&self.db)
        .await?;

        Ok(subscriptions)
    }

    /// Get an enabled channel by ID
    async fn get_enabled_channel(&self, channel_id: &str) -> Result<Option<NotificationChannel>> {
        let channel = sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE id = ? AND enabled = 1",
        )
        .bind(channel_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(channel)
    }

    /// Send notification to a specific channel
    async fn send_to_channel(
        &self,
        channel: &NotificationChannel,
        payload: &NotificationPayload,
    ) -> Result<()> {
        match channel.get_channel_type() {
            NotificationChannelType::Slack => {
                if let Some(config) = channel.get_slack_config() {
                    self.send_slack(&config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Slack config"
                    );
                }
            }
            NotificationChannelType::Discord => {
                if let Some(config) = channel.get_discord_config() {
                    self.send_discord(&config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Discord config"
                    );
                }
            }
            NotificationChannelType::Email => {
                if let Some(config) = channel.get_email_config() {
                    self.send_email(&config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Email config"
                    );
                }
            }
            NotificationChannelType::Webhook => {
                if let Some(config) = channel.get_webhook_config() {
                    self.send_webhook(&config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Webhook config"
                    );
                }
            }
            NotificationChannelType::Telegram => {
                if let Some(config) = channel.get_telegram_config() {
                    telegram::send_telegram(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Telegram config"
                    );
                }
            }
            NotificationChannelType::Teams => {
                if let Some(config) = channel.get_teams_config() {
                    teams::send_teams(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Teams config"
                    );
                }
            }
            NotificationChannelType::Pushover => {
                if let Some(config) = channel.get_pushover_config() {
                    pushover::send_pushover(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Pushover config"
                    );
                }
            }
            NotificationChannelType::Ntfy => {
                if let Some(config) = channel.get_ntfy_config() {
                    ntfy::send_ntfy(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Ntfy config"
                    );
                }
            }
            NotificationChannelType::Mattermost => {
                if let Some(config) = channel.get_mattermost_config() {
                    mattermost::send_mattermost(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Mattermost config"
                    );
                }
            }
            NotificationChannelType::Lark => {
                if let Some(config) = channel.get_lark_config() {
                    lark::send_lark(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Lark config"
                    );
                }
            }
            NotificationChannelType::Gotify => {
                if let Some(config) = channel.get_gotify_config() {
                    gotify::send_gotify(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Gotify config"
                    );
                }
            }
            NotificationChannelType::Resend => {
                if let Some(config) = channel.get_resend_config() {
                    resend::send_resend(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Resend config"
                    );
                }
            }
            NotificationChannelType::Sendry => {
                if let Some(config) = channel.get_sendry_config() {
                    sendry::send_sendry(&self.http_client, &config, payload).await?;
                } else {
                    tracing::warn!(
                        channel_id = %channel.id,
                        "Invalid Sendry config"
                    );
                }
            }
        }

        tracing::info!(
            channel_id = %channel.id,
            channel_name = %channel.name,
            channel_type = %channel.channel_type,
            event_type = %payload.event_type,
            "Notification sent"
        );

        Ok(())
    }

    /// Send a Slack notification
    pub async fn send_slack(
        &self,
        config: &SlackConfig,
        payload: &NotificationPayload,
    ) -> Result<()> {
        self.http_client
            .post(&config.webhook_url)
            .json(&build_slack_message(payload))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send a Discord notification
    pub async fn send_discord(
        &self,
        config: &DiscordConfig,
        payload: &NotificationPayload,
    ) -> Result<()> {
        self.http_client
            .post(&config.webhook_url)
            .json(&build_discord_message(payload))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send an email notification
    pub async fn send_email(
        &self,
        config: &EmailConfig,
        payload: &NotificationPayload,
    ) -> Result<()> {
        let from: Mailbox = config.from_address.parse()?;
        let (html_body, text_body) = build_email_bodies(payload);

        for to_address in &config.to_addresses {
            let to: Mailbox = to_address.parse()?;

            let email = Message::builder()
                .from(from.clone())
                .to(to)
                .subject(payload.title())
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(text_body.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html_body.clone()),
                        ),
                )?;

            // Build SMTP transport
            let mailer = if config.smtp_tls {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
            }
            .port(config.smtp_port);

            let mailer = if let (Some(username), Some(password)) =
                (&config.smtp_username, &config.smtp_password)
            {
                mailer.credentials(Credentials::new(username.clone(), password.clone()))
            } else {
                mailer
            };

            mailer.build().send(email).await?;
        }

        Ok(())
    }

    /// Send a generic webhook notification
    pub async fn send_webhook(
        &self,
        config: &WebhookConfig,
        payload: &NotificationPayload,
    ) -> Result<()> {
        let body = build_webhook_body(config, payload);

        // Serialize body to bytes so we can optionally sign it
        let body_bytes = serde_json::to_vec(&body)?;

        // Build the request with custom headers
        let client = reqwest::Client::new();
        let mut request = client
            .post(&config.url)
            .header("Content-Type", "application/json")
            .body(body_bytes.clone());

        // Compute HMAC-SHA256 signature if a secret is configured
        if let Some(ref secret) = config.webhook_secret {
            use hmac::{Hmac, KeyInit, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;

            if let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) {
                mac.update(&body_bytes);
                let result = mac.finalize();
                let hex_sig = hex::encode(result.into_bytes());
                request = request.header("X-Rivetr-Signature", format!("sha256={}", hex_sig));
            }
        }

        // Add custom headers
        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        // Send the request
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(
                url = %config.url,
                status = %status,
                response_body = %body,
                "Webhook request failed"
            );
            anyhow::bail!("Webhook request failed with status {}: {}", status, body);
        }

        Ok(())
    }

    /// Send a test notification
    pub async fn send_test(
        &self,
        channel: &NotificationChannel,
        message: Option<String>,
    ) -> Result<()> {
        let test_message =
            message.unwrap_or_else(|| "This is a test notification from Rivetr.".to_string());

        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentSuccess,
            app_id: "test-app-id".to_string(),
            app_name: "Test Application".to_string(),
            deployment_id: Some("test-deployment-id".to_string()),
            status: "success".to_string(),
            message: test_message,
            error_message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            extras: Default::default(),
        };

        self.send_to_channel(channel, &payload).await
    }
}

/// Slack attachment fields shared by the Slack and Slack-compatible webhook formats.
fn slack_fields(payload: &NotificationPayload) -> Vec<serde_json::Value> {
    let mut fields = vec![
        json!({ "title": payload.subject_label(), "value": &payload.app_name, "short": true }),
        json!({ "title": "Status", "value": &payload.status, "short": true }),
    ];
    if let Some(ref deployment_id) = payload.deployment_id {
        fields.push(json!({ "title": "Deployment ID", "value": deployment_id, "short": true }));
    }
    for (label, value) in payload.detail_fields() {
        fields.push(json!({ "title": label, "value": value, "short": true }));
    }
    if let Some(ref error) = payload.error_message {
        fields.push(json!({ "title": "Error", "value": error, "short": false }));
    }
    fields
}

/// Discord embed fields shared by the Discord and Discord-compatible webhook formats.
fn discord_fields(payload: &NotificationPayload) -> Vec<serde_json::Value> {
    let mut fields = vec![
        json!({ "name": payload.subject_label(), "value": &payload.app_name, "inline": true }),
        json!({ "name": "Status", "value": &payload.status, "inline": true }),
    ];
    if let Some(ref deployment_id) = payload.deployment_id {
        fields.push(json!({ "name": "Deployment ID", "value": deployment_id, "inline": true }));
    }
    for (label, value) in payload.detail_fields() {
        fields.push(json!({ "name": label, "value": value, "inline": true }));
    }
    if let Some(ref error) = payload.error_message {
        fields.push(json!({ "name": "Error", "value": error, "inline": false }));
    }
    fields
}

/// Slack incoming-webhook message.
pub(crate) fn build_slack_message(payload: &NotificationPayload) -> serde_json::Value {
    json!({
        "attachments": [{
            "color": payload.color(),
            "title": payload.title(),
            "text": &payload.message,
            "fields": slack_fields(payload),
            "footer": "Rivetr Deployment Engine",
            "ts": chrono::Utc::now().timestamp()
        }]
    })
}

/// Discord webhook message.
pub(crate) fn build_discord_message(payload: &NotificationPayload) -> serde_json::Value {
    let color_hex = payload.color().trim_start_matches('#');
    let color = i32::from_str_radix(color_hex, 16).unwrap_or(0x3498db);
    json!({
        "embeds": [{
            "title": payload.title(),
            "description": &payload.message,
            "color": color,
            "fields": discord_fields(payload),
            "footer": { "text": "Rivetr Deployment Engine" },
            "timestamp": &payload.timestamp
        }]
    })
}

/// Escape text for an HTML email body.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// HTML and plain text bodies for an SMTP notification email.
pub(crate) fn build_email_bodies(payload: &NotificationPayload) -> (String, String) {
    let mut html_fields = String::new();
    let mut text_fields = String::new();
    if let Some(ref id) = payload.deployment_id {
        html_fields.push_str(&format!(
            r#"<div class="field"><span class="field-label">Deployment ID:</span> <span class="field-value">{}</span></div>"#,
            escape_html(id)
        ));
        text_fields.push_str(&format!("\nDeployment ID: {}", id));
    }
    for (label, value) in payload.detail_fields() {
        html_fields.push_str(&format!(
            r#"<div class="field"><span class="field-label">{}:</span> <span class="field-value">{}</span></div>"#,
            escape_html(label),
            escape_html(value)
        ));
        text_fields.push_str(&format!("\n{}: {}", label, value));
    }
    let html_error = payload
        .error_message
        .as_ref()
        .map(|e| {
            format!(
                r#"<div class="error"><strong>Error:</strong> {}</div>"#,
                escape_html(e)
            )
        })
        .unwrap_or_default();
    let text_error = payload
        .error_message
        .as_ref()
        .map(|e| format!("\nError: {}", e))
        .unwrap_or_default();

    let html_body = format!(
        r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
                    .container {{ max-width: 600px; margin: 0 auto; background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                    .header {{ background-color: {color}; color: white; padding: 20px; text-align: center; }}
                    .content {{ padding: 20px; }}
                    .field {{ margin-bottom: 15px; }}
                    .field-label {{ font-weight: bold; color: #666; }}
                    .field-value {{ color: #333; }}
                    .error {{ background-color: #fee2e2; border-left: 4px solid #ef4444; padding: 10px; margin-top: 15px; }}
                    .footer {{ padding: 15px; text-align: center; color: #888; font-size: 12px; border-top: 1px solid #eee; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>{title}</h1>
                    </div>
                    <div class="content">
                        <p>{message}</p>
                        <div class="field">
                            <span class="field-label">{label}:</span>
                            <span class="field-value">{name}</span>
                        </div>
                        <div class="field">
                            <span class="field-label">Status:</span>
                            <span class="field-value">{status}</span>
                        </div>
                        {html_fields}
                        {html_error}
                    </div>
                    <div class="footer">
                        Rivetr Deployment Engine
                    </div>
                </div>
            </body>
            </html>
            "#,
        color = payload.color(),
        title = escape_html(&payload.title()),
        message = escape_html(&payload.message),
        label = payload.subject_label(),
        name = escape_html(&payload.app_name),
        status = escape_html(&payload.status),
    );

    let text_body = format!(
        "{}\n\n{}\n\n{}: {}\nStatus: {}{}{}\n\n---\nRivetr Deployment Engine",
        payload.title(),
        payload.message,
        payload.subject_label(),
        payload.app_name,
        payload.status,
        text_fields,
        text_error,
    );

    (html_body, text_body)
}

/// JSON body for a generic webhook channel, following its payload template.
pub(crate) fn build_webhook_body(
    config: &WebhookConfig,
    payload: &NotificationPayload,
) -> serde_json::Value {
    let details: serde_json::Map<String, serde_json::Value> = payload
        .detail_fields()
        .iter()
        .map(|(k, v)| (k.clone(), json!(v)))
        .collect();
    match config.payload_template.as_str() {
        "slack" => json!({
            "attachments": [{
                "color": payload.color(),
                "title": payload.title(),
                "text": &payload.message,
                "fields": slack_fields(payload),
                "footer": "Rivetr Deployment Engine",
                "ts": chrono::Utc::now().timestamp()
            }]
        }),
        "discord" => {
            let color_hex = payload.color().trim_start_matches('#');
            json!({
                "embeds": [{
                    "color": i32::from_str_radix(color_hex, 16).unwrap_or(0x6c757d),
                    "title": payload.title(),
                    "description": &payload.message,
                    "fields": discord_fields(payload),
                    "footer": { "text": "Rivetr Deployment Engine" },
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }]
            })
        }
        "custom" => {
            if let Some(ref template) = config.custom_template {
                let detail = |label: &str| {
                    payload
                        .detail_fields()
                        .iter()
                        .find(|(k, _)| k == label)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default()
                };
                let result = template
                    .replace("{{app_name}}", &payload.app_name)
                    .replace("{{metric_type}}", &payload.status)
                    .replace("{{value}}", &payload.status)
                    .replace("{{threshold}}", "")
                    .replace("{{timestamp}}", &payload.timestamp)
                    .replace("{{severity}}", &payload.status)
                    .replace("{{message}}", &payload.message)
                    .replace("{{event_type}}", &payload.event_type.to_string())
                    .replace(
                        "{{resource_type}}",
                        payload.extras.resource_type.as_deref().unwrap_or(""),
                    )
                    .replace(
                        "{{resource_id}}",
                        payload.extras.resource_id.as_deref().unwrap_or(""),
                    )
                    .replace("{{size}}", &detail("Size"))
                    .replace("{{duration}}", &detail("Duration"))
                    .replace("{{destination}}", &detail("Destination"))
                    .replace("{{error}}", payload.error_message.as_deref().unwrap_or(""));

                // Try to parse as JSON, if it fails, wrap in a message object
                serde_json::from_str(&result).unwrap_or_else(|_| json!({"message": result}))
            } else {
                json!({
                    "app_name": &payload.app_name,
                    "event_type": payload.event_type.to_string(),
                    "status": &payload.status,
                    "message": &payload.message,
                    "timestamp": &payload.timestamp
                })
            }
        }
        _ => {
            let mut body = json!({
                "app_name": &payload.app_name,
                "app_id": &payload.app_id,
                "deployment_id": &payload.deployment_id,
                "event_type": payload.event_type.to_string(),
                "status": &payload.status,
                "message": &payload.message,
                "error_message": &payload.error_message,
                "timestamp": &payload.timestamp
            });
            if payload.event_type.is_backup_event() {
                body["resource_type"] = json!(payload.extras.resource_type);
                body["resource_id"] = json!(payload.extras.resource_id);
                body["details"] = serde_json::Value::Object(details);
            }
            body
        }
    }
}

/// Backup payloads shared by the channel formatter tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::time::Duration;

    use super::NotificationPayload;
    use crate::engine::backup_jobs::notify::{BackupEvent, BackupOutcome, BackupResourceKind};

    /// One payload per backup event: succeeded (database to S3), failed (volume),
    /// missed (instance).
    pub fn backup_payloads() -> Vec<NotificationPayload> {
        vec![
            BackupEvent::new(
                BackupOutcome::Succeeded,
                BackupResourceKind::Database,
                "orders-db",
            )
            .resource_id("db-1")
            .trigger("scheduled")
            .size(Some(3 * 1024 * 1024))
            .duration(Some(Duration::from_secs(12)))
            .destination("S3 (offsite)")
            .to_payload(),
            BackupEvent::new(BackupOutcome::Failed, BackupResourceKind::Volume, "uploads")
                .resource_id("vol-1")
                .trigger("manual")
                .destination("local")
                .error("tar: write error")
                .to_payload(),
            BackupEvent::new(
                BackupOutcome::Missed,
                BackupResourceKind::Instance,
                "Rivetr instance",
            )
            .trigger("scheduled")
            .expected_at("2026-11-05T02:00:00Z")
            .to_payload(),
        ]
    }

    /// Assert a rendered message carries what the backup payload says.
    pub fn assert_backup_rendered(rendered: &str, payload: &NotificationPayload) {
        assert!(rendered.contains(&payload.app_name), "{rendered}");
        assert!(rendered.contains(&payload.subject_label()), "{rendered}");
        for (label, value) in payload.detail_fields() {
            assert!(
                rendered.contains(label.as_str()),
                "missing {label}: {rendered}"
            );
            assert!(
                rendered.contains(value.as_str()),
                "missing {value}: {rendered}"
            );
        }
        if let Some(ref error) = payload.error_message {
            assert!(
                rendered.contains(error.as_str()),
                "missing error: {rendered}"
            );
        }
        assert!(!rendered.contains("Application"), "{rendered}");
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{assert_backup_rendered, backup_payloads};
    use super::*;

    #[test]
    fn backup_titles_colors_and_labels() {
        let p = backup_payloads();
        assert_eq!(p[0].title(), "Backup Succeeded: orders-db");
        assert_eq!(p[0].color(), "#2ecc71");
        assert_eq!(p[0].emoji(), ":floppy_disk:");
        assert_eq!(p[1].title(), "Backup Failed: uploads");
        assert_eq!(p[1].color(), "#e74c3c");
        assert_eq!(p[2].title(), "Backup Missed: Rivetr instance");
        assert_eq!(p[2].subject_label(), "Instance");
        assert!(p[1].detail_text_lines().contains("\nTrigger: manual"));
        assert!(p[0].detail_html_rows().contains("S3 (offsite)"));
    }

    #[test]
    fn slack_and_discord_render_backup_events() {
        for payload in backup_payloads() {
            assert_backup_rendered(&build_slack_message(&payload).to_string(), &payload);
            assert_backup_rendered(&build_discord_message(&payload).to_string(), &payload);
        }
    }

    #[test]
    fn email_renders_backup_events() {
        for payload in backup_payloads() {
            let (html, text) = build_email_bodies(&payload);
            assert_backup_rendered(&html, &payload);
            assert_backup_rendered(&text, &payload);
            assert!(html.contains(&payload.title()));
        }
    }

    #[test]
    fn webhook_renders_backup_events_in_every_template() {
        for template in ["json", "slack", "discord"] {
            let config = WebhookConfig {
                url: "https://example.com/hook".into(),
                headers: Default::default(),
                payload_template: template.into(),
                custom_template: None,
                webhook_secret: None,
            };
            for payload in backup_payloads() {
                let body = build_webhook_body(&config, &payload).to_string();
                if template == "json" {
                    assert!(body.contains(&payload.event_type.to_string()));
                    assert!(body.contains("\"resource_type\""));
                    for (_, value) in payload.detail_fields() {
                        assert!(body.contains(value.as_str()), "{body}");
                    }
                } else {
                    assert_backup_rendered(&body, &payload);
                }
            }
        }
    }

    #[test]
    fn webhook_custom_template_has_backup_variables() {
        let config = WebhookConfig {
            url: "https://example.com/hook".into(),
            headers: Default::default(),
            payload_template: "custom".into(),
            custom_template: Some(
                r#"{"e":"{{event_type}}","r":"{{resource_type}}","s":"{{size}}","d":"{{destination}}","err":"{{error}}"}"#
                    .into(),
            ),
            webhook_secret: None,
        };
        let payloads = backup_payloads();
        let body = build_webhook_body(&config, &payloads[0]);
        assert_eq!(body["e"], "backup_succeeded");
        assert_eq!(body["r"], "database");
        assert_eq!(body["s"], "3.0 MB");
        assert_eq!(body["d"], "S3 (offsite)");
        let body = build_webhook_body(&config, &payloads[1]);
        assert_eq!(body["err"], "tar: write error");
    }

    #[test]
    fn deployment_payloads_keep_application_label() {
        let p = NotificationPayload::app_event(
            NotificationEventType::AppStarted,
            "a".into(),
            "web".into(),
            "started".into(),
        );
        assert_eq!(p.subject_label(), "Application");
        assert!(build_slack_message(&p).to_string().contains("Application"));
    }

    #[tokio::test]
    async fn backup_subscriptions_match_by_resource() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();
        // Migration 133 is idempotent: a second init keeps the rebuilt table.
        pool.close().await;
        let pool = crate::db::init(dir.path()).await.unwrap();
        sqlx::query(
            "INSERT INTO notification_channels (id, name, channel_type, config) \
             VALUES ('c1', 'ops', 'slack', '{\"webhook_url\":\"https://example.com\"}')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO notification_subscriptions (id, channel_id, event_type, resource_type, resource_id) VALUES \
             ('all', 'c1', 'backup_failed', NULL, NULL), \
             ('dbs', 'c1', 'backup_failed', 'database', NULL), \
             ('one', 'c1', 'backup_failed', 'database', 'db-1'), \
             ('other', 'c1', 'backup_failed', 'database', 'db-2'), \
             ('vols', 'c1', 'backup_failed', 'volume', NULL), \
             ('deploy', 'c1', 'deployment_failed', NULL, NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = NotificationService::new(pool.clone());
        let failed_db = test_support::backup_payloads()
            .into_iter()
            .find(|p| p.extras.resource_type.as_deref() == Some("database"))
            .map(|mut p| {
                p.event_type = NotificationEventType::BackupFailed;
                p
            })
            .unwrap();
        let mut ids: Vec<String> = service
            .get_matching_subscriptions(&failed_db)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["all", "dbs", "one"]);

        // Deployment events still match subscriptions without resource filters only.
        let deploy = NotificationPayload::app_event(
            NotificationEventType::DeploymentFailed,
            "app-1".into(),
            "web".into(),
            "failed".into(),
        );
        let ids: Vec<String> = service
            .get_matching_subscriptions(&deploy)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(ids, vec!["deploy"]);
    }
}
