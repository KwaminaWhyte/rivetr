//! Pushover notification sender.
//!
//! Sends messages via the Pushover API using JSON payloads.

use anyhow::Result;
use serde_json::json;

use crate::db::PushoverConfig;

use super::NotificationPayload;

/// Send a notification to Pushover via the API
pub async fn send_pushover(
    http_client: &reqwest::Client,
    config: &PushoverConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let url = "https://api.pushover.net/1/messages.json";

    let message = format_pushover_message(payload);
    let title = format!("Rivetr: {}", payload.title());
    let priority = config.priority.unwrap_or(0);

    let mut body = json!({
        "token": config.app_token,
        "user": config.user_key,
        "title": title,
        "message": message,
        "priority": priority,
    });

    // Add optional device
    if let Some(ref device) = config.device {
        body.as_object_mut()
            .unwrap()
            .insert("device".to_string(), json!(device));
    }

    let response = http_client.post(url).json(&body).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Pushover API request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Format a notification payload as a plain text message for Pushover
fn format_pushover_message(payload: &NotificationPayload) -> String {
    let mut msg = format!(
        "{message}\n\nApplication: {app}\nStatus: {status}",
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

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_format_pushover_message_deployment_success() {
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

        let msg = format_pushover_message(&payload);
        assert!(msg.contains("Deployment completed successfully"));
        assert!(msg.contains("Application: my-app"));
        assert!(msg.contains("Deployment ID: dep-456"));
        assert!(!msg.contains("Error:"));
    }

    #[test]
    fn test_format_pushover_message_with_error() {
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

        let msg = format_pushover_message(&payload);
        assert!(msg.contains("Error: Build error: missing Dockerfile"));
    }

    #[test]
    fn test_format_pushover_message_no_deployment_id() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::AppStopped,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: None,
            status: "stopped".to_string(),
            message: "App has been stopped".to_string(),
            error_message: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let msg = format_pushover_message(&payload);
        assert!(!msg.contains("Deployment ID:"));
        assert!(msg.contains("Application: my-app"));
    }
}
