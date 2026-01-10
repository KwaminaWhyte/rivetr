//! Alert evaluation engine module
//!
//! This module evaluates collected resource metrics against configured thresholds
//! and triggers alerts when thresholds are exceeded. It implements hysteresis
//! to prevent flapping alerts (requires 2 consecutive threshold breaches).
//!
//! Key features:
//! - Evaluates CPU, memory, and disk metrics against thresholds
//! - Hysteresis: Only triggers alerts after 2 consecutive threshold breaches
//! - Duplicate prevention: 15-minute window between notifications for same alert
//! - Automatically resolves alerts when metrics return to normal
//! - Sends email notifications when alerts fire or resolve

use crate::db::{AlertBreachCount, AlertConfig, AlertEvent, ResourceMetric};
use crate::notifications::AlertNotificationService;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Minimum consecutive breaches required before triggering an alert
const HYSTERESIS_THRESHOLD: i64 = 2;

/// Result of evaluating alerts for a single app
#[derive(Debug, Default)]
pub struct AlertEvaluationResult {
    /// Number of metrics evaluated
    pub metrics_evaluated: usize,
    /// Number of new alerts triggered
    pub alerts_triggered: usize,
    /// Number of alerts resolved
    pub alerts_resolved: usize,
    /// Number of alerts updated (still firing)
    pub alerts_updated: usize,
}

/// Alert evaluation service
pub struct AlertEvaluator {
    db: SqlitePool,
    /// Optional notification service for sending email alerts
    notification_service: Option<Arc<AlertNotificationService>>,
    /// Optional dashboard URL for building links in notifications
    dashboard_url: Option<String>,
}

impl AlertEvaluator {
    /// Create a new alert evaluator
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            notification_service: None,
            dashboard_url: None,
        }
    }

    /// Create a new alert evaluator with notification service
    pub fn with_notifications(
        db: SqlitePool,
        notification_service: Arc<AlertNotificationService>,
        dashboard_url: Option<String>,
    ) -> Self {
        Self {
            db,
            notification_service: Some(notification_service),
            dashboard_url,
        }
    }

    /// Evaluate alerts for a single app based on the latest metric
    pub async fn evaluate_for_app(&self, app_id: &str) -> AlertEvaluationResult {
        let mut result = AlertEvaluationResult::default();

        // Get the latest metric for the app
        let metric = match ResourceMetric::get_latest_for_app(&self.db, app_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                tracing::trace!(app_id = %app_id, "No metrics found for app");
                return result;
            }
            Err(e) => {
                tracing::warn!(app_id = %app_id, error = %e, "Failed to get latest metric");
                return result;
            }
        };

        // Evaluate each metric type
        for metric_type in &["cpu", "memory", "disk"] {
            result.metrics_evaluated += 1;

            let current_value = match *metric_type {
                "cpu" => metric.cpu_percent,
                "memory" => {
                    if metric.memory_limit_bytes > 0 {
                        (metric.memory_bytes as f64 / metric.memory_limit_bytes as f64) * 100.0
                    } else {
                        0.0
                    }
                }
                "disk" => {
                    if metric.disk_limit_bytes > 0 {
                        (metric.disk_bytes as f64 / metric.disk_limit_bytes as f64) * 100.0
                    } else {
                        0.0
                    }
                }
                _ => continue,
            };

            // Get effective threshold (per-app or global default)
            let (threshold, enabled) =
                match AlertConfig::get_effective_threshold(&self.db, app_id, metric_type).await {
                    Ok(Some((t, e))) => (t, e),
                    Ok(None) => {
                        // No threshold configured, skip
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!(
                            app_id = %app_id,
                            metric_type = %metric_type,
                            error = %e,
                            "Failed to get threshold"
                        );
                        continue;
                    }
                };

            if !enabled {
                // Alerts disabled for this metric, but we should resolve any active alerts
                if let Err(e) = self.resolve_alert_if_active(app_id, metric_type).await {
                    tracing::warn!(error = %e, "Failed to resolve alert for disabled metric");
                }
                continue;
            }

            let is_breached = current_value > threshold;

            if is_breached {
                match self
                    .handle_threshold_breach(app_id, metric_type, threshold, current_value)
                    .await
                {
                    Ok(AlertAction::NewAlert) => result.alerts_triggered += 1,
                    Ok(AlertAction::Updated) => result.alerts_updated += 1,
                    Ok(AlertAction::None) => {}
                    Err(e) => {
                        tracing::warn!(
                            app_id = %app_id,
                            metric_type = %metric_type,
                            error = %e,
                            "Failed to handle threshold breach"
                        );
                    }
                }
            } else {
                // Not breached - reset counter and resolve any active alert
                if let Err(e) = AlertBreachCount::reset(&self.db, app_id, metric_type).await {
                    tracing::warn!(error = %e, "Failed to reset breach count");
                }

                match self.resolve_alert_if_active(app_id, metric_type).await {
                    Ok(true) => result.alerts_resolved += 1,
                    Ok(false) => {}
                    Err(e) => {
                        tracing::warn!(
                            app_id = %app_id,
                            metric_type = %metric_type,
                            error = %e,
                            "Failed to resolve alert"
                        );
                    }
                }
            }
        }

        result
    }

    /// Handle a threshold breach
    async fn handle_threshold_breach(
        &self,
        app_id: &str,
        metric_type: &str,
        threshold: f64,
        current_value: f64,
    ) -> Result<AlertAction, sqlx::Error> {
        // Increment consecutive breach count
        let consecutive_count = AlertBreachCount::increment(&self.db, app_id, metric_type).await?;

        tracing::debug!(
            app_id = %app_id,
            metric_type = %metric_type,
            threshold = threshold,
            current_value = current_value,
            consecutive_count = consecutive_count,
            "Threshold breach detected"
        );

        // Check if there's an active alert
        let active_alert =
            AlertEvent::get_active_for_app_metric(&self.db, app_id, metric_type).await?;

        match active_alert {
            Some(alert) => {
                // Update existing alert with new value
                let updated_alert =
                    AlertEvent::update_value(&self.db, &alert.id, current_value, consecutive_count)
                        .await?;

                // Check if we should send a re-notification (15-minute window)
                if updated_alert.should_notify() {
                    self.send_alert_notification(&updated_alert).await;
                    // Mark as notified
                    if let Err(e) = AlertEvent::set_notified(&self.db, &updated_alert.id).await {
                        tracing::warn!(error = %e, "Failed to update notification timestamp");
                    }
                }

                Ok(AlertAction::Updated)
            }
            None => {
                // Only trigger new alert if hysteresis threshold is met
                if consecutive_count >= HYSTERESIS_THRESHOLD {
                    let alert = AlertEvent::create(
                        &self.db,
                        app_id,
                        metric_type,
                        threshold,
                        current_value,
                        consecutive_count,
                    )
                    .await?;

                    tracing::info!(
                        app_id = %app_id,
                        metric_type = %metric_type,
                        threshold = threshold,
                        current_value = current_value,
                        "Alert triggered after {} consecutive breaches",
                        consecutive_count
                    );

                    // Send notification for new alert
                    self.send_alert_notification(&alert).await;
                    // Mark as notified
                    if let Err(e) = AlertEvent::set_notified(&self.db, &alert.id).await {
                        tracing::warn!(error = %e, "Failed to update notification timestamp");
                    }

                    Ok(AlertAction::NewAlert)
                } else {
                    tracing::debug!(
                        app_id = %app_id,
                        metric_type = %metric_type,
                        consecutive_count = consecutive_count,
                        hysteresis_threshold = HYSTERESIS_THRESHOLD,
                        "Breach count below hysteresis threshold, not triggering alert"
                    );
                    Ok(AlertAction::None)
                }
            }
        }
    }

    /// Send notification for an alert (fires or resolved)
    async fn send_alert_notification(&self, alert: &AlertEvent) {
        if let Some(ref notification_service) = self.notification_service {
            let dashboard_url = self.dashboard_url.as_deref();
            let result = notification_service
                .notify_alert_triggered(alert, dashboard_url)
                .await;
            match result {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!(
                            alert_id = %alert.id,
                            channels = count,
                            "Queued alert notifications"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        alert_id = %alert.id,
                        error = %e,
                        "Failed to queue alert notifications"
                    );
                }
            }
        }
    }

    /// Resolve an active alert if one exists
    async fn resolve_alert_if_active(
        &self,
        app_id: &str,
        metric_type: &str,
    ) -> Result<bool, sqlx::Error> {
        if let Some(alert) =
            AlertEvent::get_active_for_app_metric(&self.db, app_id, metric_type).await?
        {
            let resolved_alert = AlertEvent::resolve(&self.db, &alert.id).await?;

            tracing::info!(
                app_id = %app_id,
                metric_type = %metric_type,
                alert_id = %alert.id,
                "Alert resolved"
            );

            // Send resolution notification
            self.send_alert_notification(&resolved_alert).await;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Evaluate alerts for all apps with recent metrics
    pub async fn evaluate_all(&self) -> AlertEvaluationSummary {
        let mut summary = AlertEvaluationSummary::default();

        // Get distinct app_ids from recent metrics
        let app_ids: Vec<(String,)> = match sqlx::query_as(
            r#"
            SELECT DISTINCT app_id
            FROM resource_metrics
            WHERE timestamp > datetime('now', '-5 minutes')
            "#,
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get app IDs for alert evaluation");
                return summary;
            }
        };

        summary.apps_checked = app_ids.len();

        for (app_id,) in app_ids {
            let result = self.evaluate_for_app(&app_id).await;
            summary.metrics_evaluated += result.metrics_evaluated;
            summary.alerts_triggered += result.alerts_triggered;
            summary.alerts_resolved += result.alerts_resolved;
            summary.alerts_updated += result.alerts_updated;
        }

        if summary.alerts_triggered > 0 || summary.alerts_resolved > 0 {
            tracing::info!(
                apps = summary.apps_checked,
                triggered = summary.alerts_triggered,
                resolved = summary.alerts_resolved,
                "Alert evaluation completed"
            );
        }

        summary
    }
}

/// Action taken after evaluating a threshold
#[derive(Debug)]
enum AlertAction {
    /// A new alert was triggered
    NewAlert,
    /// An existing alert was updated
    Updated,
    /// No action taken (below hysteresis threshold)
    None,
}

/// Summary of alert evaluation across all apps
#[derive(Debug, Default)]
pub struct AlertEvaluationSummary {
    /// Number of apps checked
    pub apps_checked: usize,
    /// Total number of metrics evaluated
    pub metrics_evaluated: usize,
    /// Number of new alerts triggered
    pub alerts_triggered: usize,
    /// Number of alerts resolved
    pub alerts_resolved: usize,
    /// Number of alerts updated (still firing)
    pub alerts_updated: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_evaluation_result_default() {
        let result = AlertEvaluationResult::default();
        assert_eq!(result.metrics_evaluated, 0);
        assert_eq!(result.alerts_triggered, 0);
        assert_eq!(result.alerts_resolved, 0);
        assert_eq!(result.alerts_updated, 0);
    }

    #[test]
    fn test_alert_evaluation_summary_default() {
        let summary = AlertEvaluationSummary::default();
        assert_eq!(summary.apps_checked, 0);
        assert_eq!(summary.metrics_evaluated, 0);
        assert_eq!(summary.alerts_triggered, 0);
        assert_eq!(summary.alerts_resolved, 0);
        assert_eq!(summary.alerts_updated, 0);
    }
}
