//! Telegram notification sender.
//!
//! Sends messages via the Telegram Bot API using HTML parse mode.

use anyhow::Result;
use serde_json::json;

use crate::db::TelegramConfig;

use super::NotificationPayload;

/// Send a notification to Telegram via the Bot API
pub async fn send_telegram(
    http_client: &reqwest::Client,
    config: &TelegramConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        config.bot_token
    );

    let text = format_telegram_message(payload);

    let mut body = json!({
        "chat_id": config.chat_id,
        "text": text,
        "parse_mode": "HTML",
    });

    // Include message_thread_id if a topic ID is specified (for forum/topic groups)
    if let Some(topic_id) = config.topic_id {
        body.as_object_mut()
            .unwrap()
            .insert("message_thread_id".to_string(), json!(topic_id));
    }

    let response = http_client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Telegram API request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Format a notification payload as an HTML message for Telegram
fn format_telegram_message(payload: &NotificationPayload) -> String {
    let emoji = match payload.event_type {
        crate::db::NotificationEventType::DeploymentStarted => "🚀",
        crate::db::NotificationEventType::DeploymentSuccess => "✅",
        crate::db::NotificationEventType::DeploymentFailed => "❌",
        crate::db::NotificationEventType::AppStopped => "🛑",
        crate::db::NotificationEventType::AppStarted => "▶️",
        crate::db::NotificationEventType::ContainerCrash => "💥",
        crate::db::NotificationEventType::ContainerRestarted => "🔄",
    };

    let mut msg = format!(
        "{emoji} <b>{title}</b>\n\n{message}\n\n<b>Application:</b> <code>{app}</code>\n<b>Status:</b> {status}",
        emoji = emoji,
        title = html_escape(&payload.title()),
        message = html_escape(&payload.message),
        app = html_escape(&payload.app_name),
        status = html_escape(&payload.status),
    );

    if let Some(ref deployment_id) = payload.deployment_id {
        msg.push_str(&format!(
            "\n<b>Deployment ID:</b> <code>{}</code>",
            html_escape(deployment_id)
        ));
    }

    if let Some(ref error) = payload.error_message {
        msg.push_str(&format!(
            "\n\n<b>Error:</b>\n<code>{}</code>",
            html_escape(error)
        ));
    }

    msg.push_str("\n\n<i>Rivetr Deployment Engine</i>");

    msg
}

/// Escape HTML special characters for Telegram HTML parse mode
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_format_telegram_message_deployment_success() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentSuccess,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: Some("dep-456".to_string()),
            status: "success".to_string(),
            message: "Deployment completed successfully".to_string(),
            error_message: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let msg = format_telegram_message(&payload);
        assert!(msg.contains("<b>Deployment Successful: my-app</b>"));
        assert!(msg.contains("<code>my-app</code>"));
        assert!(msg.contains("<code>dep-456</code>"));
        assert!(msg.contains("Rivetr Deployment Engine"));
    }

    #[test]
    fn test_format_telegram_message_with_error() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentFailed,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: Some("dep-456".to_string()),
            status: "failed".to_string(),
            message: "Deployment failed".to_string(),
            error_message: Some("Build error: missing Dockerfile".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let msg = format_telegram_message(&payload);
        assert!(msg.contains("<b>Error:</b>"));
        assert!(msg.contains("Build error: missing Dockerfile"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("Tom & Jerry"), "Tom &amp; Jerry");
    }
}
