//! Microsoft Teams notification sender.
//!
//! Sends messages via Microsoft Teams Incoming Webhook using Adaptive Card format.

use anyhow::Result;
use serde_json::json;

use crate::db::TeamsConfig;

use super::NotificationPayload;

/// Send a notification to Microsoft Teams via Incoming Webhook
pub async fn send_teams(
    http_client: &reqwest::Client,
    config: &TeamsConfig,
    payload: &NotificationPayload,
) -> Result<()> {
    let card = build_adaptive_card(payload);

    let response = http_client
        .post(&config.webhook_url)
        .header("Content-Type", "application/json")
        .json(&card)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "Teams webhook request failed with status {}: {}",
            status,
            response_body
        );
    }

    Ok(())
}

/// Build a Microsoft Teams Adaptive Card payload
fn build_adaptive_card(payload: &NotificationPayload) -> serde_json::Value {
    let status_color = match payload.event_type {
        crate::db::NotificationEventType::DeploymentStarted => "accent",
        crate::db::NotificationEventType::DeploymentSuccess
        | crate::db::NotificationEventType::AppStarted
        | crate::db::NotificationEventType::ContainerRestarted => "good",
        crate::db::NotificationEventType::DeploymentFailed
        | crate::db::NotificationEventType::ContainerCrash => "attention",
        crate::db::NotificationEventType::AppStopped => "warning",
    };

    let mut body = vec![
        // Title
        json!({
            "type": "TextBlock",
            "text": payload.title(),
            "weight": "bolder",
            "size": "medium",
            "color": status_color,
        }),
        // Message
        json!({
            "type": "TextBlock",
            "text": payload.message,
            "wrap": true,
        }),
        // Facts (application info)
        json!({
            "type": "FactSet",
            "facts": build_facts(payload),
        }),
    ];

    // Add error block if present
    if let Some(ref error) = payload.error_message {
        body.push(json!({
            "type": "TextBlock",
            "text": format!("Error: {}", error),
            "color": "attention",
            "wrap": true,
            "spacing": "medium",
        }));
    }

    // Footer
    body.push(json!({
        "type": "TextBlock",
        "text": format!("Rivetr Deployment Engine | {}", payload.timestamp),
        "size": "small",
        "isSubtle": true,
        "spacing": "medium",
    }));

    json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.card.adaptive",
            "content": {
                "type": "AdaptiveCard",
                "version": "1.4",
                "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                "body": body,
            }
        }]
    })
}

/// Build the fact set for the Adaptive Card
fn build_facts(payload: &NotificationPayload) -> Vec<serde_json::Value> {
    let mut facts = vec![
        json!({ "title": "Application", "value": payload.app_name }),
        json!({ "title": "Status", "value": payload.status }),
    ];

    if let Some(ref deployment_id) = payload.deployment_id {
        facts.push(json!({ "title": "Deployment ID", "value": deployment_id }));
    }

    facts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NotificationEventType;

    #[test]
    fn test_build_adaptive_card_success() {
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

        let card = build_adaptive_card(&payload);

        // Verify the overall structure
        assert_eq!(card["type"], "message");
        assert_eq!(
            card["attachments"][0]["contentType"],
            "application/vnd.microsoft.card.adaptive"
        );
        assert_eq!(card["attachments"][0]["content"]["type"], "AdaptiveCard");
        assert_eq!(card["attachments"][0]["content"]["version"], "1.4");

        // Verify body content
        let body = card["attachments"][0]["content"]["body"]
            .as_array()
            .unwrap();
        assert!(body.len() >= 3); // title, message, facts, footer
    }

    #[test]
    fn test_build_adaptive_card_with_error() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentFailed,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: Some("dep-456".to_string()),
            status: "failed".to_string(),
            message: "Deployment failed".to_string(),
            error_message: Some("Build error".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let card = build_adaptive_card(&payload);
        let body = card["attachments"][0]["content"]["body"]
            .as_array()
            .unwrap();
        // Should include an error text block (title + message + facts + error + footer = 5)
        assert_eq!(body.len(), 5);
    }

    #[test]
    fn test_build_facts() {
        let payload = NotificationPayload {
            event_type: NotificationEventType::DeploymentSuccess,
            app_id: "app-123".to_string(),
            app_name: "my-app".to_string(),
            deployment_id: Some("dep-456".to_string()),
            status: "success".to_string(),
            message: "Done".to_string(),
            error_message: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let facts = build_facts(&payload);
        assert_eq!(facts.len(), 3); // Application, Status, Deployment ID
        assert_eq!(facts[0]["title"], "Application");
        assert_eq!(facts[0]["value"], "my-app");
    }
}
