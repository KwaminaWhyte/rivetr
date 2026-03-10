//! Discord webhook alert payload builder.

use serde_json::json;

use super::AlertNotificationPayload;

/// Build Discord-compatible webhook payload for alerts
pub fn build_discord_alert_payload(payload: &AlertNotificationPayload) -> serde_json::Value {
    let status_emoji = if payload.status == "resolved" {
        "\u{2705}" // White check mark
    } else {
        match payload.severity() {
            "critical" => "\u{1F6A8}", // Rotating light
            "warning" => "\u{26A0}",   // Warning sign
            _ => "\u{2139}",           // Information
        }
    };

    let description = if payload.status == "resolved" {
        format!(
            "The {} alert for **{}** has been resolved.",
            payload.metric_label(),
            payload.app_name
        )
    } else {
        format!(
            "The {} for **{}** has exceeded the configured threshold.",
            payload.metric_label(),
            payload.app_name
        )
    };

    // Convert hex color to integer
    let color_hex = payload.color().trim_start_matches('#');
    let color = i32::from_str_radix(color_hex, 16).unwrap_or(0x3b82f6);

    let mut fields = vec![
        json!({
            "name": "Application",
            "value": &payload.app_name,
            "inline": true
        }),
        json!({
            "name": "Metric",
            "value": payload.metric_label(),
            "inline": true
        }),
        json!({
            "name": "\u{200B}", // Zero-width space for line break
            "value": "\u{200B}",
            "inline": false
        }),
        json!({
            "name": "Current Value",
            "value": format!("{:.1}%", payload.current_value),
            "inline": true
        }),
        json!({
            "name": "Threshold",
            "value": format!("{:.1}%", payload.threshold),
            "inline": true
        }),
    ];

    if payload.status != "resolved" {
        fields.push(json!({
            "name": "Severity",
            "value": payload.severity().to_uppercase(),
            "inline": true
        }));
    }

    // Add dashboard link if available
    if let Some(ref url) = payload.dashboard_url {
        fields.push(json!({
            "name": "Dashboard",
            "value": format!("[View App]({}/apps/{})", url.trim_end_matches('/'), payload.app_id),
            "inline": false
        }));
    }

    json!({
        "embeds": [{
            "title": format!("{} {}", status_emoji, payload.title()),
            "description": description,
            "color": color,
            "fields": fields,
            "footer": {
                "text": "Rivetr Resource Monitoring"
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }]
    })
}
