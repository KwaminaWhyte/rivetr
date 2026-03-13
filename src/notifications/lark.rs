//! Lark (Feishu) notification sender.
//!
//! Sends messages via Lark/Feishu custom bot incoming webhook API.

use anyhow::Result;
use serde_json::json;

use crate::db::LarkConfig;

use super::NotificationPayload;

/// Send a notification to Lark/Feishu via a custom bot webhook
pub async fn send_lark(
    http_client: &reqwest::Client,
    config: &LarkConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let text = format_lark_message(payload);

    let body = json!({
        "msg_type": "text",
        "content": {
            "text": text
        }
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
            "Lark webhook request failed with status {}: {}",
            status,
            response_body
        );
    }

    // Lark returns a JSON body with a code field; 0 means success
    Ok(())
}

/// Format a notification payload as a plain text message for Lark
fn format_lark_message(payload: &NotificationPayload) -> String {
    let icon = match payload.event_type {
        crate::db::NotificationEventType::DeploymentStarted => "[DEPLOY]",
        crate::db::NotificationEventType::DeploymentSuccess => "[SUCCESS]",
        crate::db::NotificationEventType::DeploymentFailed => "[FAILED]",
        crate::db::NotificationEventType::AppStopped => "[STOPPED]",
        crate::db::NotificationEventType::AppStarted => "[STARTED]",
        crate::db::NotificationEventType::ContainerCrash => "[CRASH]",
        crate::db::NotificationEventType::ContainerRestarted => "[RESTARTED]",
    };

    let mut msg = format!(
        "{icon} {title}\n\n{message}\n\nApplication: {app}\nStatus: {status}",
        icon = icon,
        title = payload.title(),
        message = payload.message,
        app = payload.app_name,
        status = payload.status,
    );

    if let Some(ref deployment_id) = payload.deployment_id {
        msg.push_str(&format!("\nDeployment ID: {}", deployment_id));
    }

    if let Some(ref error) = payload.error_message {
        msg.push_str(&format!("\n\nError: {}", error));
    }

    msg.push_str("\n\nRivetr Deployment Engine");

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_format_lark_message_deployment_success() {
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

        let msg = format_lark_message(&payload);
        assert!(msg.contains("[SUCCESS]"));
        assert!(msg.contains("Deployment Successful: my-app"));
        assert!(msg.contains("Application: my-app"));
        assert!(msg.contains("dep-456"));
        assert!(msg.contains("Rivetr Deployment Engine"));
    }

    #[test]
    fn test_format_lark_message_with_error() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentFailed,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: None,
            status: "failed".to_string(),
            message: "Deployment failed".to_string(),
            error_message: Some("Build error: missing Dockerfile".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let msg = format_lark_message(&payload);
        assert!(msg.contains("[FAILED]"));
        assert!(msg.contains("Error: Build error: missing Dockerfile"));
    }
}
