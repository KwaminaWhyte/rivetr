//! Webhook channel alert implementations: generic webhook, custom template, and JSON fallback.

use anyhow::Result;
use serde_json::json;
use std::time::Duration;

use crate::db::WebhookConfig;

use super::discord::build_discord_alert_payload;
use super::slack::build_slack_alert_payload;
use super::AlertNotificationPayload;

/// Build the webhook payload based on the template type
pub fn build_alert_webhook_payload(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> serde_json::Value {
    match config.payload_template.as_str() {
        "slack" => build_slack_alert_payload(payload),
        "discord" => build_discord_alert_payload(payload),
        "custom" => build_custom_alert_payload(config, payload),
        _ => build_json_alert_payload(payload), // Default JSON format
    }
}

/// Build custom template payload with variable substitution
pub fn build_custom_alert_payload(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> serde_json::Value {
    if let Some(ref template) = config.custom_template {
        let result = template
            .replace("{{app_name}}", &payload.app_name)
            .replace("{{app_id}}", &payload.app_id)
            .replace("{{metric_type}}", &payload.metric_type)
            .replace("{{value}}", &format!("{:.1}", payload.current_value))
            .replace("{{threshold}}", &format!("{:.1}", payload.threshold))
            .replace("{{timestamp}}", &payload.timestamp)
            .replace("{{severity}}", payload.severity())
            .replace("{{status}}", &payload.status)
            .replace(
                "{{dashboard_url}}",
                payload.dashboard_link().as_deref().unwrap_or(""),
            );

        // Try to parse as JSON; if it fails, wrap in a message object
        serde_json::from_str(&result).unwrap_or_else(|_| json!({"message": result}))
    } else {
        // Fall back to default JSON format if no custom template
        build_json_alert_payload(payload)
    }
}

/// Build default JSON payload for alerts
pub fn build_json_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    json!({
        "app_name": &payload.app_name,
        "app_id": &payload.app_id,
        "metric_type": &payload.metric_type,
        "current_value": payload.current_value,
        "threshold": payload.threshold,
        "status": &payload.status,
        "severity": payload.severity(),
        "timestamp": &payload.timestamp,
        "dashboard_url": payload.dashboard_link()
    })
}

/// Send an alert webhook with retry logic
///
/// Implements exponential backoff retry: 1s, 2s, 4s delays between attempts.
/// Returns the HTTP status code on success, or the last error on failure.
pub async fn send_alert_webhook_with_retry(
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
    max_attempts: u32,
) -> Result<u16> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 1..=max_attempts {
        match send_alert_webhook_once(&client, config, payload).await {
            Ok(status_code) => {
                if attempt > 1 {
                    tracing::debug!(
                        url = %config.url,
                        attempt = attempt,
                        "Webhook delivered after retry"
                    );
                }
                return Ok(status_code);
            }
            Err(e) => {
                last_error = Some(e);

                if attempt < max_attempts {
                    // Exponential backoff: 1s, 2s, 4s
                    let delay = Duration::from_secs(1 << (attempt - 1));
                    tracing::warn!(
                        url = %config.url,
                        attempt = attempt,
                        max_attempts = max_attempts,
                        delay_secs = delay.as_secs(),
                        error = %last_error.as_ref().unwrap(),
                        "Webhook delivery failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Webhook delivery failed")))
}

/// Send a single webhook request
async fn send_alert_webhook_once(
    client: &reqwest::Client,
    config: &WebhookConfig,
    payload: &AlertNotificationPayload,
) -> Result<u16> {
    let body = build_alert_webhook_payload(config, payload);

    // Build the request with custom headers
    let mut request = client.post(&config.url).json(&body);

    // Add custom headers from config
    for (key, value) in &config.headers {
        request = request.header(key, value);
    }

    // Send the request
    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        tracing::warn!(
            url = %config.url,
            status = %status,
            response_body = %body_text,
            "Webhook request failed"
        );
        anyhow::bail!(
            "Webhook request failed with status {}: {}",
            status,
            body_text
        );
    }

    tracing::debug!(
        url = %config.url,
        status = status.as_u16(),
        "Webhook request successful"
    );

    Ok(status.as_u16())
}
