//! Alert notification service for sending email alerts when resource thresholds are exceeded.
//!
//! This module provides specialized email notification handling for resource alerts,
//! including:
//! - Alert-specific email templates
//! - Asynchronous email queuing to prevent blocking alert evaluation
//! - Delivery status logging
//!
//! The service integrates with the existing notification channel configuration
//! and uses the same SMTP infrastructure as deployment notifications.

use crate::db::{AlertEvent, App, EmailConfig, NotificationChannel};
use crate::DbPool;
use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
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
    SendAlert {
        payload: AlertNotificationPayload,
        email_config: EmailConfig,
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

    /// Queue an alert notification for sending
    pub async fn queue_alert(
        &self,
        payload: AlertNotificationPayload,
        email_config: EmailConfig,
    ) -> Result<()> {
        self.tx
            .send(AlertNotificationCommand::SendAlert {
                payload,
                email_config,
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to queue alert notification: {}", e))
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

        // Get all enabled email channels
        let channels = self.get_email_channels().await?;
        let mut sent = 0;

        for channel in channels {
            if let Some(email_config) = channel.get_email_config() {
                if let Err(e) = self.queue_alert(payload.clone(), email_config).await {
                    tracing::warn!(
                        channel_id = %channel.id,
                        error = %e,
                        "Failed to queue alert notification"
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

/// Spawn the alert notification worker that processes the email queue
pub fn spawn_alert_notification_worker(
    mut rx: mpsc::Receiver<AlertNotificationCommand>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Alert notification worker started");

        while let Some(command) = rx.recv().await {
            match command {
                AlertNotificationCommand::SendAlert {
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
}
