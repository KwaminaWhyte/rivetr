//! Slack webhook alert payload builder.

use serde_json::json;

use super::AlertNotificationPayload;

/// Build Slack-compatible webhook payload for alerts
pub fn build_slack_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    let status_emoji = if payload.status == "resolved" {
        ":white_check_mark:"
    } else {
        match payload.severity() {
            "critical" => ":rotating_light:",
            "warning" => ":warning:",
            _ => ":information_source:",
        }
    };

    let status_text = if payload.status == "resolved" {
        format!(
            "The {} alert for *{}* has been resolved.",
            payload.metric_label(),
            payload.app_name
        )
    } else {
        format!(
            "The {} for *{}* has exceeded the configured threshold.",
            payload.metric_label(),
            payload.app_name
        )
    };

    let mut fields = vec![
        json!({
            "title": "Application",
            "value": &payload.app_name,
            "short": true
        }),
        json!({
            "title": "Metric",
            "value": payload.metric_label(),
            "short": true
        }),
        json!({
            "title": "Current Value",
            "value": format!("{:.1}%", payload.current_value),
            "short": true
        }),
        json!({
            "title": "Threshold",
            "value": format!("{:.1}%", payload.threshold),
            "short": true
        }),
    ];

    if payload.status != "resolved" {
        fields.push(json!({
            "title": "Severity",
            "value": payload.severity().to_uppercase(),
            "short": true
        }));
    }

    // Add dashboard link if available
    if let Some(ref url) = payload.dashboard_url {
        fields.push(json!({
            "title": "Dashboard",
            "value": format!("<{}/apps/{}|View App>", url.trim_end_matches('/'), payload.app_id),
            "short": true
        }));
    }

    json!({
        "attachments": [{
            "color": payload.color(),
            "title": format!("{} {}", status_emoji, payload.title()),
            "text": status_text,
            "fields": fields,
            "footer": "Rivetr Resource Monitoring",
            "ts": chrono::Utc::now().timestamp()
        }]
    })
}
