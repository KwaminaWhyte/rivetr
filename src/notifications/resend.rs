//! Resend email API notification sender.
//!
//! Sends transactional emails via the Resend API (https://resend.com).
//! This is an alternative to SMTP-based email that uses an HTTP API.

use anyhow::Result;
use serde_json::json;

use crate::db::ResendConfig;

use super::NotificationPayload;

/// Resend API endpoint
const RESEND_API_URL: &str = "https://api.resend.com/emails";

/// Send an email notification via the Resend API
pub async fn send_resend(
    http_client: &reqwest::Client,
    config: &ResendConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let subject = format!("Rivetr: {}", payload.title());
    let html_body = build_html_body(payload);
    let text_body = build_text_body(payload);

    let to_addresses: Vec<&str> = config.to_addresses.iter().map(|s| s.as_str()).collect();

    let body = json!({
        "from": config.from_address,
        "to": to_addresses,
        "subject": subject,
        "html": html_body,
        "text": text_body,
    });

    let response = http_client
        .post(RESEND_API_URL)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Resend API request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Build an HTML email body for the notification
fn build_html_body(payload: &NotificationPayload) -> String {
    format!(
        r#"<!DOCTYPE html>
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
            {}{}
        </div>
        <div class="footer">
            Rivetr Deployment Engine
        </div>
    </div>
</body>
</html>"#,
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
    )
}

/// Build a plain text email body for the notification
fn build_text_body(payload: &NotificationPayload) -> String {
    let mut body = format!(
        "{}\n\n{}\n\nApplication: {}\nStatus: {}",
        payload.title(),
        payload.message,
        payload.app_name,
        payload.status,
    );

    if let Some(ref deployment_id) = payload.deployment_id {
        body.push_str(&format!("\nDeployment ID: {}", deployment_id));
    }

    if let Some(ref error) = payload.error_message {
        body.push_str(&format!("\n\nError: {}", error));
    }

    body.push_str("\n\n---\nRivetr Deployment Engine");

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_build_text_body_deployment_success() {
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

        let body = build_text_body(&payload);
        assert!(body.contains("Deployment Successful: my-app"));
        assert!(body.contains("Application: my-app"));
        assert!(body.contains("Deployment ID: dep-456"));
        assert!(body.contains("Rivetr Deployment Engine"));
        assert!(!body.contains("Error:"));
    }

    #[test]
    fn test_build_text_body_with_error() {
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

        let body = build_text_body(&payload);
        assert!(body.contains("Error: Build error: missing Dockerfile"));
        assert!(!body.contains("Deployment ID:"));
    }

    #[test]
    fn test_build_html_body_contains_title() {
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

        let html = build_html_body(&payload);
        assert!(html.contains("App Stopped: my-app"));
        assert!(html.contains("Rivetr Deployment Engine"));
    }
}
