//! Email alert notification implementation.

use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::db::EmailConfig;

use super::AlertNotificationPayload;

/// Send an alert email using the provided configuration
pub async fn send_alert_email(
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
