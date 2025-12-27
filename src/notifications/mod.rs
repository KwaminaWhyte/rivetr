//! Notification system for sending alerts via Slack, Discord, and Email.
//!
//! This module provides a unified interface for sending notifications
//! on deployment events and app state changes.

use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde_json::json;

use crate::db::{
    DiscordConfig, EmailConfig, NotificationChannel, NotificationChannelType,
    NotificationEventType, NotificationSubscription, SlackConfig,
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
        }
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
        }
    }

    /// Get the color for the notification (for Slack/Discord)
    pub fn color(&self) -> &'static str {
        match self.event_type {
            NotificationEventType::DeploymentStarted => "#3498db", // Blue
            NotificationEventType::DeploymentSuccess | NotificationEventType::AppStarted => {
                "#2ecc71"
            } // Green
            NotificationEventType::DeploymentFailed => "#e74c3c", // Red
            NotificationEventType::AppStopped => "#f39c12",       // Orange
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

        // Get subscriptions for this event type that either:
        // 1. Have no app_id (subscribe to all apps)
        // 2. Have the specific app_id
        let subscriptions = sqlx::query_as::<_, NotificationSubscription>(
            r#"
            SELECT * FROM notification_subscriptions
            WHERE event_type = ? AND (app_id IS NULL OR app_id = ?)
            "#,
        )
        .bind(&event_type)
        .bind(&payload.app_id)
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
    pub async fn send_slack(&self, config: &SlackConfig, payload: &NotificationPayload) -> Result<()> {
        let mut fields = vec![
            json!({
                "title": "Application",
                "value": &payload.app_name,
                "short": true
            }),
            json!({
                "title": "Status",
                "value": &payload.status,
                "short": true
            }),
        ];

        if let Some(ref deployment_id) = payload.deployment_id {
            fields.push(json!({
                "title": "Deployment ID",
                "value": deployment_id,
                "short": true
            }));
        }

        if let Some(ref error) = payload.error_message {
            fields.push(json!({
                "title": "Error",
                "value": error,
                "short": false
            }));
        }

        let message = json!({
            "attachments": [{
                "color": payload.color(),
                "title": payload.title(),
                "text": &payload.message,
                "fields": fields,
                "footer": "Rivetr Deployment Engine",
                "ts": chrono::Utc::now().timestamp()
            }]
        });

        self.http_client
            .post(&config.webhook_url)
            .json(&message)
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
        let mut fields = vec![
            json!({
                "name": "Application",
                "value": &payload.app_name,
                "inline": true
            }),
            json!({
                "name": "Status",
                "value": &payload.status,
                "inline": true
            }),
        ];

        if let Some(ref deployment_id) = payload.deployment_id {
            fields.push(json!({
                "name": "Deployment ID",
                "value": deployment_id,
                "inline": true
            }));
        }

        if let Some(ref error) = payload.error_message {
            fields.push(json!({
                "name": "Error",
                "value": error,
                "inline": false
            }));
        }

        // Convert hex color to integer
        let color_hex = payload.color().trim_start_matches('#');
        let color = i32::from_str_radix(color_hex, 16).unwrap_or(0x3498db);

        let message = json!({
            "embeds": [{
                "title": payload.title(),
                "description": &payload.message,
                "color": color,
                "fields": fields,
                "footer": {
                    "text": "Rivetr Deployment Engine"
                },
                "timestamp": &payload.timestamp
            }]
        });

        self.http_client
            .post(&config.webhook_url)
            .json(&message)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send an email notification
    pub async fn send_email(&self, config: &EmailConfig, payload: &NotificationPayload) -> Result<()> {
        let from: Mailbox = config.from_address.parse()?;

        // Build HTML content
        let html_body = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
                    .container {{ max-width: 600px; margin: 0 auto; background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                    .header {{ background-color: {}; color: white; padding: 20px; text-align: center; }}
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
                        <h1>{}</h1>
                    </div>
                    <div class="content">
                        <p>{}</p>
                        <div class="field">
                            <span class="field-label">Application:</span>
                            <span class="field-value">{}</span>
                        </div>
                        <div class="field">
                            <span class="field-label">Status:</span>
                            <span class="field-value">{}</span>
                        </div>
                        {}
                        {}
                    </div>
                    <div class="footer">
                        Rivetr Deployment Engine
                    </div>
                </div>
            </body>
            </html>
            "#,
            payload.color(),
            payload.title(),
            payload.message,
            payload.app_name,
            payload.status,
            payload
                .deployment_id
                .as_ref()
                .map(|id| format!(
                    r#"<div class="field"><span class="field-label">Deployment ID:</span> <span class="field-value">{}</span></div>"#,
                    id
                ))
                .unwrap_or_default(),
            payload
                .error_message
                .as_ref()
                .map(|e| format!(r#"<div class="error"><strong>Error:</strong> {}</div>"#, e))
                .unwrap_or_default(),
        );

        // Build plain text version
        let text_body = format!(
            "{}\n\n{}\n\nApplication: {}\nStatus: {}{}{}\n\n---\nRivetr Deployment Engine",
            payload.title(),
            payload.message,
            payload.app_name,
            payload.status,
            payload
                .deployment_id
                .as_ref()
                .map(|id| format!("\nDeployment ID: {}", id))
                .unwrap_or_default(),
            payload
                .error_message
                .as_ref()
                .map(|e| format!("\nError: {}", e))
                .unwrap_or_default(),
        );

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

    /// Send a test notification
    pub async fn send_test(&self, channel: &NotificationChannel, message: Option<String>) -> Result<()> {
        let test_message = message.unwrap_or_else(|| "This is a test notification from Rivetr.".to_string());

        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentSuccess,
            app_id: "test-app-id".to_string(),
            app_name: "Test Application".to_string(),
            deployment_id: Some("test-deployment-id".to_string()),
            status: "success".to_string(),
            message: test_message,
            error_message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        self.send_to_channel(channel, &payload).await
    }
}
