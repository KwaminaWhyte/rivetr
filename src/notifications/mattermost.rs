//! Mattermost notification sender.
//!
//! Sends messages via Mattermost Incoming Webhook API.
//! Mattermost uses the same incoming webhook format as Slack.

use anyhow::Result;
use serde_json::json;

use crate::db::MattermostConfig;

use super::NotificationPayload;

/// Send a notification to Mattermost via an incoming webhook
pub async fn send_mattermost(
    http_client: &reqwest::Client,
    config: &MattermostConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let text = format_mattermost_message(payload);

    let body = json!({
        "text": text,
        "username": "Rivetr",
    });

    let response = http_client
        .post(&config.webhook_url)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Mattermost webhook request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Format a notification payload as a Mattermost markdown message
fn format_mattermost_message(payload: &NotificationPayload) -> String {
    let icon = match payload.event_type {
        crate::db::NotificationEventType::DeploymentStarted => ":rocket:",
        crate::db::NotificationEventType::DeploymentSuccess => ":white_check_mark:",
        crate::db::NotificationEventType::DeploymentFailed => ":x:",
        crate::db::NotificationEventType::AppStopped => ":octagonal_sign:",
        crate::db::NotificationEventType::AppStarted => ":arrow_forward:",
    };

    let mut msg = format!(
        "{icon} **{title}**\n\n{message}\n\n**Application:** {app}\n**Status:** {status}",
        icon = icon,
        title = payload.title(),
        message = payload.message,
        app = payload.app_name,
        status = payload.status,
    );

    if let Some(ref deployment_id) = payload.deployment_id {
        msg.push_str(&format!("\n**Deployment ID:** `{}`", deployment_id));
    }

    if let Some(ref error) = payload.error_message {
        msg.push_str(&format!("\n\n**Error:**\n```\n{}\n```", error));
    }

    msg.push_str("\n\n_Rivetr Deployment Engine_");

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_format_mattermost_message_deployment_success() {
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

        let msg = format_mattermost_message(&payload);
        assert!(msg.contains("**Deployment Successful: my-app**"));
        assert!(msg.contains("**Application:** my-app"));
        assert!(msg.contains("`dep-456`"));
        assert!(msg.contains("Rivetr Deployment Engine"));
        assert!(!msg.contains("**Error:**"));
    }

    #[test]
    fn test_format_mattermost_message_with_error() {
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

        let msg = format_mattermost_message(&payload);
        assert!(msg.contains("**Error:**"));
        assert!(msg.contains("Build error: missing Dockerfile"));
    }
}
