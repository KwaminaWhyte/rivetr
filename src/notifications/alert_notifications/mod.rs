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

mod channels;
mod discord;
mod email;
mod slack;

pub use channels::{
    build_alert_webhook_payload, build_custom_alert_payload, build_json_alert_payload,
    send_alert_webhook_with_retry,
};
pub use discord::build_discord_alert_payload;
pub use email::send_alert_email;
pub use slack::build_slack_alert_payload;

use crate::db::{AlertEvent, App, EmailConfig, NotificationChannel, WebhookConfig};
use crate::DbPool;
use anyhow::Result;
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
            webhook_secret: None,
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
            webhook_secret: None,
        };

        let result = build_custom_alert_payload(&config, &payload);

        // Should fall back to JSON format
        assert_eq!(result["app_name"], "MyApp");
        assert_eq!(result["metric_type"], "cpu");
    }
}
