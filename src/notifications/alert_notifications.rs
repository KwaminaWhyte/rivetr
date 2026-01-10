//! Alert notification service for sending email and webhook alerts when resource thresholds are exceeded.
//!
//! This module provides specialized notification handling for resource alerts,
//! including:
//! - Alert-specific email templates
//! - Webhook notifications with configurable payload templates (Slack, Discord, custom)
//! - Retry logic for webhook delivery (3 attempts with exponential backoff)
//! - Asynchronous notification queuing to prevent blocking alert evaluation
//! - Delivery status logging
//!
//! The service integrates with the existing notification channel configuration
//! and uses the same SMTP infrastructure as deployment notifications.

use crate::db::{AlertEvent, App, EmailConfig, NotificationChannel, WebhookConfig};
use crate::DbPool;
use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;

/// Alert notification payload for email template rendering
#[derive(Debug, Clone)]
pub struct AlertNotificationPayload {
    /// The app name
    pub app_name: String,
    /// The app ID (for dashboard link)
    pub app_id: String,
    /// Metric type (cpu, memory, disk)
    pub metric_type: String,
    /// Current value (percentage)
    pub current_value: f64,
    /// Configured threshold (percentage)
    pub threshold: f64,
    /// Alert status (firing or resolved)
    pub status: String,
    /// Timestamp of the alert
    pub timestamp: String,
    /// Optional dashboard base URL for direct links
    pub dashboard_url: Option<String>,
}

impl AlertNotificationPayload {
    /// Create a payload from an AlertEvent
    pub fn from_alert_event(
        event: &AlertEvent,
        app_name: &str,
        dashboard_url: Option<&str>,
    ) -> Self {
        Self {
            app_name: app_name.to_string(),
            app_id: event.app_id.clone(),
            metric_type: event.metric_type.clone(),
            current_value: event.current_value,
            threshold: event.threshold_percent,
            status: event.status.clone(),
            timestamp: event.fired_at.clone(),
            dashboard_url: dashboard_url.map(|s| s.to_string()),
        }
    }

    /// Get a human-readable metric type label
    pub fn metric_label(&self) -> &'static str {
        match self.metric_type.as_str() {
            "cpu" => "CPU Usage",
            "memory" => "Memory Usage",
            "disk" => "Disk Usage",
            _ => "Resource Usage",
        }
    }

    /// Get the severity level based on how much the threshold is exceeded
    pub fn severity(&self) -> &'static str {
        let overage = self.current_value - self.threshold;
        if overage > 20.0 {
            "critical"
        } else if overage > 10.0 {
            "warning"
        } else {
            "info"
        }
    }

    /// Get the color for the alert (for email styling)
    pub fn color(&self) -> &'static str {
        if self.status == "resolved" {
            "#22c55e" // Green
        } else {
            match self.severity() {
                "critical" => "#ef4444", // Red
                "warning" => "#f59e0b",  // Orange/Amber
                _ => "#3b82f6",          // Blue
            }
        }
    }

    /// Get the title for the notification
    pub fn title(&self) -> String {
        if self.status == "resolved" {
            format!(
                "Resolved: {} Alert - {}",
                self.metric_label(),
                self.app_name
            )
        } else {
            format!(
                "{}: {} Alert - {}",
                self.severity().to_uppercase(),
                self.metric_label(),
                self.app_name
            )
        }
    }

    /// Build the dashboard link for the app
    pub fn dashboard_link(&self) -> Option<String> {
        self.dashboard_url
            .as_ref()
            .map(|url| format!("{}/apps/{}", url.trim_end_matches('/'), self.app_id))
    }
}

/// Command for the alert notification queue
#[derive(Debug)]
pub enum AlertNotificationCommand {
    /// Send an alert email notification
    SendEmail {
        payload: AlertNotificationPayload,
        email_config: EmailConfig,
    },
    /// Send an alert webhook notification
    SendWebhook {
        payload: AlertNotificationPayload,
        webhook_config: WebhookConfig,
        channel_id: String,
        channel_name: String,
    },
}

/// Alert notification service with async email queue
pub struct AlertNotificationService {
    db: DbPool,
    tx: mpsc::Sender<AlertNotificationCommand>,
}

impl AlertNotificationService {
    /// Create a new alert notification service with the specified queue capacity
    pub fn new(
        db: DbPool,
        queue_capacity: usize,
    ) -> (Self, mpsc::Receiver<AlertNotificationCommand>) {
        let (tx, rx) = mpsc::channel(queue_capacity);
        (Self { db, tx }, rx)
    }

    /// Queue an alert email notification for sending
    pub async fn queue_email_alert(
        &self,
        payload: AlertNotificationPayload,
        email_config: EmailConfig,
    ) -> Result<()> {
        self.tx
            .send(AlertNotificationCommand::SendEmail {
                payload,
                email_config,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to queue email alert notification: {}", e))
    }

    /// Queue an alert webhook notification for sending
    pub async fn queue_webhook_alert(
        &self,
        payload: AlertNotificationPayload,
        webhook_config: WebhookConfig,
        channel_id: String,
        channel_name: String,
    ) -> Result<()> {
        self.tx
            .send(AlertNotificationCommand::SendWebhook {
                payload,
                webhook_config,
                channel_id,
                channel_name,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to queue webhook alert notification: {}", e))
    }

    /// Get all enabled email notification channels
    pub async fn get_email_channels(&self) -> Result<Vec<NotificationChannel>> {
        let channels: Vec<NotificationChannel> = sqlx::query_as(
            r#"
            SELECT * FROM notification_channels
            WHERE channel_type = 'email' AND enabled = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(channels)
    }

    /// Get all enabled webhook notification channels
    pub async fn get_webhook_channels(&self) -> Result<Vec<NotificationChannel>> {
        let channels: Vec<NotificationChannel> = sqlx::query_as(
            r#"
            SELECT * FROM notification_channels
            WHERE channel_type = 'webhook' AND enabled = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(channels)
    }

    /// Get app by ID for name lookup
    pub async fn get_app(&self, app_id: &str) -> Result<Option<App>> {
        let app: Option<App> = sqlx::query_as("SELECT * FROM apps WHERE id = ?")
            .bind(app_id)
            .fetch_optional(&self.db)
            .await?;

        Ok(app)
    }

    /// Send alert notifications for a triggered alert
    pub async fn notify_alert_triggered(
        &self,
        alert: &AlertEvent,
        dashboard_url: Option<&str>,
    ) -> Result<usize> {
        // Get app name
        let app = self.get_app(&alert.app_id).await?;
        let app_name = app.map(|a| a.name).unwrap_or_else(|| alert.app_id.clone());

        // Create payload
        let payload = AlertNotificationPayload::from_alert_event(alert, &app_name, dashboard_url);

        let mut sent = 0;

        // Get all enabled email channels and queue notifications
        let email_channels = self.get_email_channels().await?;
        for channel in email_channels {
            if let Some(email_config) = channel.get_email_config() {
                if let Err(e) = self.queue_email_alert(payload.clone(), email_config).await {
                    tracing::warn!(
                        channel_id = %channel.id,
                        channel_type = "email",
                        error = %e,
                        "Failed to queue email alert notification"
                    );
                } else {
                    sent += 1;
                }
            }
        }

        // Get all enabled webhook channels and queue notifications
        let webhook_channels = self.get_webhook_channels().await?;
        for channel in webhook_channels {
            if let Some(webhook_config) = channel.get_webhook_config() {
                if let Err(e) = self
                    .queue_webhook_alert(
                        payload.clone(),
                        webhook_config,
                        channel.id.clone(),
                        channel.name.clone(),
                    )
                    .await
                {
                    tracing::warn!(
                        channel_id = %channel.id,
                        channel_type = "webhook",
                        error = %e,
                        "Failed to queue webhook alert notification"
                    );
                } else {
                    sent += 1;
                }
            }
        }

        Ok(sent)
    }

    /// Send alert notifications for a resolved alert
    pub async fn notify_alert_resolved(
        &self,
        alert: &AlertEvent,
        dashboard_url: Option<&str>,
    ) -> Result<usize> {
        // Same as triggered, but with resolved status already set in alert
        self.notify_alert_triggered(alert, dashboard_url).await
    }
}

/// Spawn the alert notification worker that processes the notification queue
pub fn spawn_alert_notification_worker(
    mut rx: mpsc::Receiver<AlertNotificationCommand>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Alert notification worker started");

        while let Some(command) = rx.recv().await {
            match command {
                AlertNotificationCommand::SendEmail {
                    payload,
                    email_config,
                } => {
                    let result = send_alert_email(&email_config, &payload).await;
                    match result {
                        Ok(recipients) => {
                            tracing::info!(
                                app_id = %payload.app_id,
                                metric_type = %payload.metric_type,
                                status = %payload.status,
                                recipients = recipients,
                                "Alert email sent successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                app_id = %payload.app_id,
                                metric_type = %payload.metric_type,
                                error = %e,
                                "Failed to send alert email"
                            );
                        }
                    }
                }
                AlertNotificationCommand::SendWebhook {
                    payload,
                    webhook_config,
                    channel_id,
                    channel_name,
                } => {
                    let result = send_alert_webhook_with_retry(&webhook_config, &payload, 3).await;
                    match result {
                        Ok(status_code) => {
                            tracing::info!(
                                app_id = %payload.app_id,
                                metric_type = %payload.metric_type,
                                status = %payload.status,
                                channel_id = %channel_id,
                                channel_name = %channel_name,
                                response_status = status_code,
                                "Alert webhook sent successfully"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                app_id = %payload.app_id,
                                metric_type = %payload.metric_type,
                                channel_id = %channel_id,
                                channel_name = %channel_name,
                                error = %e,
                                "Failed to send alert webhook after retries"
                            );
                        }
                    }
                }
            }
        }

        tracing::info!("Alert notification worker stopped");
    })
}

/// Send an alert email using the provided configuration
async fn send_alert_email(
    config: &EmailConfig,
    payload: &AlertNotificationPayload,
) -> Result<usize> {
    let from: Mailbox = config.from_address.parse()?;

    // Build HTML content
    let dashboard_link = payload.dashboard_link();
    let dashboard_link_html = dashboard_link
        .as_ref()
        .map(|link| {
            format!(
                r#"<p style="margin-top: 20px;"><a href="{}" style="background-color: #3b82f6; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px;">View in Dashboard</a></p>"#,
                link
            )
        })
        .unwrap_or_default();

    let status_message = if payload.status == "resolved" {
        format!(
            "The {} alert for <strong>{}</strong> has been resolved. Resource usage is now back within normal thresholds.",
            payload.metric_label(),
            payload.app_name
        )
    } else {
        format!(
            "The {} for <strong>{}</strong> has exceeded the configured threshold and requires attention.",
            payload.metric_label(),
            payload.app_name
        )
    };

    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
                .container {{ max-width: 600px; margin: 0 auto; background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                .header {{ background-color: {}; color: white; padding: 20px; text-align: center; }}
                .header h1 {{ margin: 0; font-size: 20px; }}
                .content {{ padding: 20px; }}
                .metrics-box {{ background-color: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 15px; margin: 15px 0; }}
                .metric-row {{ display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #e2e8f0; }}
                .metric-row:last-child {{ border-bottom: none; }}
                .metric-label {{ color: #64748b; font-weight: 500; }}
                .metric-value {{ color: #1e293b; font-weight: 600; }}
                .current-value {{ color: {}; }}
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
                    <div class="metrics-box">
                        <div class="metric-row">
                            <span class="metric-label">Application</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Metric</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Current Value</span>
                            <span class="metric-value current-value">{:.1}%</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Threshold</span>
                            <span class="metric-value">{:.1}%</span>
                        </div>
                        <div class="metric-row">
                            <span class="metric-label">Timestamp</span>
                            <span class="metric-value">{}</span>
                        </div>
                    </div>
                    {}
                </div>
                <div class="footer">
                    Rivetr Resource Monitoring
                </div>
            </div>
        </body>
        </html>
        "#,
        payload.color(),
        payload.color(),
        payload.title(),
        status_message,
        payload.app_name,
        payload.metric_label(),
        payload.current_value,
        payload.threshold,
        payload.timestamp,
        dashboard_link_html,
    );

    // Build plain text version
    let text_body = format!(
        "{}\n\n{}\n\nApplication: {}\nMetric: {}\nCurrent Value: {:.1}%\nThreshold: {:.1}%\nTimestamp: {}\n{}\n---\nRivetr Resource Monitoring",
        payload.title(),
        if payload.status == "resolved" {
            format!(
                "The {} alert for {} has been resolved.",
                payload.metric_label(),
                payload.app_name
            )
        } else {
            format!(
                "The {} for {} has exceeded the threshold.",
                payload.metric_label(),
                payload.app_name
            )
        },
        payload.app_name,
        payload.metric_label(),
        payload.current_value,
        payload.threshold,
        payload.timestamp,
        dashboard_link
            .as_ref()
            .map(|link| format!("\nView in Dashboard: {}", link))
            .unwrap_or_default(),
    );

    let mut sent_count = 0;

    for to_address in &config.to_addresses {
        let to: Mailbox = match to_address.parse() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    address = %to_address,
                    error = %e,
                    "Invalid email address, skipping"
                );
                continue;
            }
        };

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

        match mailer.build().send(email).await {
            Ok(_) => {
                sent_count += 1;
                tracing::debug!(
                    to = %to_address,
                    "Alert email delivered successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    to = %to_address,
                    error = %e,
                    "Failed to deliver alert email"
                );
            }
        }
    }

    Ok(sent_count)
}

/// Send an alert webhook with retry logic
///
/// Implements exponential backoff retry: 1s, 2s, 4s delays between attempts.
/// Returns the HTTP status code on success, or the last error on failure.
async fn send_alert_webhook_with_retry(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
    max_attempts: u32,
) -> Result<u16> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 1..=max_attempts {
        match send_alert_webhook_once(&client, config, payload).await {
            Ok(status_code) => {
                if attempt > 1 {
                    tracing::debug!(
                        url = %config.url,
                        attempt = attempt,
                        "Webhook delivered after retry"
                    );
                }
                return Ok(status_code);
            }
            Err(e) => {
                last_error = Some(e);

                if attempt < max_attempts {
                    // Exponential backoff: 1s, 2s, 4s
                    let delay = Duration::from_secs(1 << (attempt - 1));
                    tracing::warn!(
                        url = %config.url,
                        attempt = attempt,
                        max_attempts = max_attempts,
                        delay_secs = delay.as_secs(),
                        error = %last_error.as_ref().unwrap(),
                        "Webhook delivery failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Webhook delivery failed")))
}

/// Send a single webhook request
async fn send_alert_webhook_once(
    client: &reqwest::Client,
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> Result<u16> {
    let body = build_alert_webhook_payload(config, payload);

    // Build the request with custom headers
    let mut request = client.post(&config.url).json(&body);

    // Add custom headers from config
    for (key, value) in &config.headers {
        request = request.header(key, value);
    }

    // Send the request
    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        tracing::warn!(
            url = %config.url,
            status = %status,
            response_body = %body_text,
            "Webhook request failed"
        );
        anyhow::bail!(
            "Webhook request failed with status {}: {}",
            status,
            body_text
        );
    }

    tracing::debug!(
        url = %config.url,
        status = status.as_u16(),
        "Webhook request successful"
    );

    Ok(status.as_u16())
}

/// Build the webhook payload based on the template type
fn build_alert_webhook_payload(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> serde_json::Value {
    match config.payload_template.as_str() {
        "slack" => build_slack_alert_payload(payload),
        "discord" => build_discord_alert_payload(payload),
        "custom" => build_custom_alert_payload(config, payload),
        _ => build_json_alert_payload(payload), // Default JSON format
    }
}

/// Build Slack-compatible webhook payload for alerts
fn build_slack_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    let status_emoji = if payload.status == "resolved" {
        ":white_check_mark:"
    } else {
        match payload.severity() {
            "critical" => ":rotating_light:",
            "warning" => ":warning:",
            _ => ":information_source:",
        }
    };

    let status_text = if payload.status == "resolved" {
        format!(
            "The {} alert for *{}* has been resolved.",
            payload.metric_label(),
            payload.app_name
        )
    } else {
        format!(
            "The {} for *{}* has exceeded the configured threshold.",
            payload.metric_label(),
            payload.app_name
        )
    };

    let mut fields = vec![
        json!({
            "title": "Application",
            "value": &payload.app_name,
            "short": true
        }),
        json!({
            "title": "Metric",
            "value": payload.metric_label(),
            "short": true
        }),
        json!({
            "title": "Current Value",
            "value": format!("{:.1}%", payload.current_value),
            "short": true
        }),
        json!({
            "title": "Threshold",
            "value": format!("{:.1}%", payload.threshold),
            "short": true
        }),
    ];

    if payload.status != "resolved" {
        fields.push(json!({
            "title": "Severity",
            "value": payload.severity().to_uppercase(),
            "short": true
        }));
    }

    // Add dashboard link if available
    if let Some(ref url) = payload.dashboard_url {
        fields.push(json!({
            "title": "Dashboard",
            "value": format!("<{}/apps/{}|View App>", url.trim_end_matches('/'), payload.app_id),
            "short": true
        }));
    }

    json!({
        "attachments": [{
            "color": payload.color(),
            "title": format!("{} {}", status_emoji, payload.title()),
            "text": status_text,
            "fields": fields,
            "footer": "Rivetr Resource Monitoring",
            "ts": chrono::Utc::now().timestamp()
        }]
    })
}

/// Build Discord-compatible webhook payload for alerts
fn build_discord_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    let status_emoji = if payload.status == "resolved" {
        "\u{2705}" // White check mark
    } else {
        match payload.severity() {
            "critical" => "\u{1F6A8}", // Rotating light
            "warning" => "\u{26A0}",   // Warning sign
            _ => "\u{2139}",           // Information
        }
    };

    let description = if payload.status == "resolved" {
        format!(
            "The {} alert for **{}** has been resolved.",
            payload.metric_label(),
            payload.app_name
        )
    } else {
        format!(
            "The {} for **{}** has exceeded the configured threshold.",
            payload.metric_label(),
            payload.app_name
        )
    };

    // Convert hex color to integer
    let color_hex = payload.color().trim_start_matches('#');
    let color = i32::from_str_radix(color_hex, 16).unwrap_or(0x3b82f6);

    let mut fields = vec![
        json!({
            "name": "Application",
            "value": &payload.app_name,
            "inline": true
        }),
        json!({
            "name": "Metric",
            "value": payload.metric_label(),
            "inline": true
        }),
        json!({
            "name": "\u{200B}", // Zero-width space for line break
            "value": "\u{200B}",
            "inline": false
        }),
        json!({
            "name": "Current Value",
            "value": format!("{:.1}%", payload.current_value),
            "inline": true
        }),
        json!({
            "name": "Threshold",
            "value": format!("{:.1}%", payload.threshold),
            "inline": true
        }),
    ];

    if payload.status != "resolved" {
        fields.push(json!({
            "name": "Severity",
            "value": payload.severity().to_uppercase(),
            "inline": true
        }));
    }

    // Add dashboard link if available
    if let Some(ref url) = payload.dashboard_url {
        fields.push(json!({
            "name": "Dashboard",
            "value": format!("[View App]({}/apps/{})", url.trim_end_matches('/'), payload.app_id),
            "inline": false
        }));
    }

    json!({
        "embeds": [{
            "title": format!("{} {}", status_emoji, payload.title()),
            "description": description,
            "color": color,
            "fields": fields,
            "footer": {
                "text": "Rivetr Resource Monitoring"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }]
    })
}

/// Build custom template payload with variable substitution
fn build_custom_alert_payload(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> serde_json::Value {
    if let Some(ref template) = config.custom_template {
        let result = template
            .replace("{{app_name}}", &payload.app_name)
            .replace("{{app_id}}", &payload.app_id)
            .replace("{{metric_type}}", &payload.metric_type)
            .replace("{{value}}", &format!("{:.1}", payload.current_value))
            .replace("{{threshold}}", &format!("{:.1}", payload.threshold))
            .replace("{{timestamp}}", &payload.timestamp)
            .replace("{{severity}}", payload.severity())
            .replace("{{status}}", &payload.status)
            .replace(
                "{{dashboard_url}}",
                payload.dashboard_link().as_deref().unwrap_or(""),
            );

        // Try to parse as JSON; if it fails, wrap in a message object
        serde_json::from_str(&result).unwrap_or_else(|_| json!({"message": result}))
    } else {
        // Fall back to default JSON format if no custom template
        build_json_alert_payload(payload)
    }
}

/// Build default JSON payload for alerts
fn build_json_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    json!({
        "app_name": &payload.app_name,
        "app_id": &payload.app_id,
        "metric_type": &payload.metric_type,
        "current_value": payload.current_value,
        "threshold": payload.threshold,
        "status": &payload.status,
        "severity": payload.severity(),
        "timestamp": &payload.timestamp,
        "dashboard_url": payload.dashboard_link()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_payload_from_event() {
        let event = AlertEvent {
            id: "test-id".to_string(),
            app_id: "app-1".to_string(),
            metric_type: "cpu".to_string(),
            threshold_percent: 80.0,
            current_value: 95.0,
            status: "firing".to_string(),
            consecutive_breaches: 2,
            fired_at: "2024-01-01T00:00:00Z".to_string(),
            resolved_at: None,
            last_notified_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let payload = AlertNotificationPayload::from_alert_event(
            &event,
            "My App",
            Some("https://example.com"),
        );

        assert_eq!(payload.app_name, "My App");
        assert_eq!(payload.metric_type, "cpu");
        assert_eq!(payload.current_value, 95.0);
        assert_eq!(payload.threshold, 80.0);
        assert_eq!(payload.status, "firing");
    }

    #[test]
    fn test_metric_label() {
        let payload = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "1".to_string(),
            metric_type: "memory".to_string(),
            current_value: 90.0,
            threshold: 85.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };

        assert_eq!(payload.metric_label(), "Memory Usage");
    }

    #[test]
    fn test_severity_levels() {
        // Critical: more than 20% over threshold
        let critical = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "1".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 105.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(critical.severity(), "critical");

        // Warning: 10-20% over threshold
        let warning = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "1".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(warning.severity(), "warning");

        // Info: less than 10% over threshold
        let info = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "1".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 85.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(info.severity(), "info");
    }

    #[test]
    fn test_dashboard_link() {
        let with_url = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 85.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: Some("https://example.com/".to_string()),
        };
        assert_eq!(
            with_url.dashboard_link(),
            Some("https://example.com/apps/app-123".to_string())
        );

        let without_url = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 85.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(without_url.dashboard_link(), None);
    }

    #[test]
    fn test_title_firing() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "1".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(payload.title(), "WARNING: CPU Usage Alert - MyApp");
    }

    #[test]
    fn test_title_resolved() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "1".to_string(),
            metric_type: "memory".to_string(),
            current_value: 70.0,
            threshold: 80.0,
            status: "resolved".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(payload.title(), "Resolved: Memory Usage Alert - MyApp");
    }

    #[test]
    fn test_color_resolved() {
        let payload = AlertNotificationPayload {
            app_name: "Test".to_string(),
            app_id: "1".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 70.0,
            threshold: 80.0,
            status: "resolved".to_string(),
            timestamp: "now".to_string(),
            dashboard_url: None,
        };
        assert_eq!(payload.color(), "#22c55e"); // Green for resolved
    }

    #[test]
    fn test_build_slack_alert_payload() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: Some("https://example.com".to_string()),
        };

        let result = build_slack_alert_payload(&payload);

        // Check structure
        assert!(result["attachments"].is_array());
        let attachment = &result["attachments"][0];
        assert!(attachment["title"]
            .as_str()
            .unwrap()
            .contains("CPU Usage Alert"));
        assert!(attachment["text"]
            .as_str()
            .unwrap()
            .contains("exceeded the configured threshold"));
        assert!(attachment["fields"].is_array());
        assert_eq!(attachment["footer"], "Rivetr Resource Monitoring");
    }

    #[test]
    fn test_build_discord_alert_payload() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "memory".to_string(),
            current_value: 90.0,
            threshold: 85.0,
            status: "firing".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: None,
        };

        let result = build_discord_alert_payload(&payload);

        // Check structure
        assert!(result["embeds"].is_array());
        let embed = &result["embeds"][0];
        assert!(embed["title"]
            .as_str()
            .unwrap()
            .contains("Memory Usage Alert"));
        assert!(embed["description"]
            .as_str()
            .unwrap()
            .contains("exceeded the configured threshold"));
        assert!(embed["fields"].is_array());
        assert!(embed["color"].is_number());
        assert_eq!(embed["footer"]["text"], "Rivetr Resource Monitoring");
    }

    #[test]
    fn test_build_discord_alert_payload_resolved() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "disk".to_string(),
            current_value: 70.0,
            threshold: 80.0,
            status: "resolved".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: None,
        };

        let result = build_discord_alert_payload(&payload);

        let embed = &result["embeds"][0];
        assert!(embed["title"].as_str().unwrap().contains("Resolved"));
        assert!(embed["description"]
            .as_str()
            .unwrap()
            .contains("has been resolved"));
    }

    #[test]
    fn test_build_json_alert_payload() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: Some("https://example.com".to_string()),
        };

        let result = build_json_alert_payload(&payload);

        assert_eq!(result["app_name"], "MyApp");
        assert_eq!(result["app_id"], "app-123");
        assert_eq!(result["metric_type"], "cpu");
        assert_eq!(result["current_value"], 95.0);
        assert_eq!(result["threshold"], 80.0);
        assert_eq!(result["status"], "firing");
        assert_eq!(result["severity"], "warning"); // 15% over threshold
        assert_eq!(result["dashboard_url"], "https://example.com/apps/app-123");
    }

    #[test]
    fn test_build_custom_alert_payload() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: Some("https://example.com".to_string()),
        };

        let config = WebhookConfig {
            url: "https://webhook.example.com".to_string(),
            headers: std::collections::HashMap::new(),
            payload_template: "custom".to_string(),
            custom_template: Some(
                r#"{"text": "Alert for {{app_name}}: {{metric_type}} at {{value}}%"}"#.to_string(),
            ),
        };

        let result = build_custom_alert_payload(&config, &payload);

        assert_eq!(result["text"], "Alert for MyApp: cpu at 95.0%");
    }

    #[test]
    fn test_build_custom_alert_payload_fallback() {
        let payload = AlertNotificationPayload {
            app_name: "MyApp".to_string(),
            app_id: "app-123".to_string(),
            metric_type: "cpu".to_string(),
            current_value: 95.0,
            threshold: 80.0,
            status: "firing".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dashboard_url: None,
        };

        // Config with no custom template should fall back to JSON
        let config = WebhookConfig {
            url: "https://webhook.example.com".to_string(),
            headers: std::collections::HashMap::new(),
            payload_template: "custom".to_string(),
            custom_template: None,
        };

        let result = build_custom_alert_payload(&config, &payload);

        // Should fall back to JSON format
        assert_eq!(result["app_name"], "MyApp");
        assert_eq!(result["metric_type"], "cpu");
    }
}
