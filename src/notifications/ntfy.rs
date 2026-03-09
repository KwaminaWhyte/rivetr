//! Ntfy notification sender.
//!
//! Sends messages via ntfy.sh or self-hosted ntfy instances using HTTP headers.

use anyhow::Result;

use crate::db::NtfyConfig;

use super::NotificationPayload;

/// Default ntfy server URL
const DEFAULT_NTFY_SERVER: &str = "https://ntfy.sh";

/// Send a notification to ntfy
pub async fn send_ntfy(
    http_client: &reqwest::Client,
    config: &NtfyConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let server_url = config
        .server_url
        .as_deref()
        .unwrap_or(DEFAULT_NTFY_SERVER)
        .trim_end_matches('/');

    let url = format!("{}/{}", server_url, config.topic);

    let title = format!("Rivetr: {}", payload.title());
    let message = format_ntfy_message(payload);

    // Map priority: ntfy uses 1 (min) to 5 (max), default 3
    let priority = config.priority.unwrap_or(3).to_string();

    let mut request = http_client
        .post(&url)
        .header("Title", title)
        .header("Priority", priority)
        .body(message);

    // Add tags if configured
    if let Some(ref tags) = config.tags {
        request = request.header("Tags", tags.as_str());
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Ntfy request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Format a notification payload as plain text for ntfy
fn format_ntfy_message(payload: &NotificationPayload) -> String {
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

    msg.push_str("\n\nRivetr Deployment Engine");

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_format_ntfy_message_deployment_success() {
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

        let msg = format_ntfy_message(&payload);
        assert!(msg.contains("Deployment completed successfully"));
        assert!(msg.contains("Application: my-app"));
        assert!(msg.contains("Deployment ID: dep-456"));
        assert!(msg.contains("Rivetr Deployment Engine"));
        assert!(!msg.contains("Error:"));
    }

    #[test]
    fn test_format_ntfy_message_with_error() {
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

        let msg = format_ntfy_message(&payload);
        assert!(msg.contains("Error: Build error: missing Dockerfile"));
    }

    #[test]
    fn test_format_ntfy_message_app_event_no_deployment() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::AppStarted,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: None,
            status: "started".to_string(),
            message: "App has been started".to_string(),
            error_message: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let msg = format_ntfy_message(&payload);
        assert!(!msg.contains("Deployment ID:"));
        assert!(msg.contains("Application: my-app"));
        assert!(msg.contains("Status: started"));
    }
}
