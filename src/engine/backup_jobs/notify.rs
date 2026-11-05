//! Backup notifications: `backup_succeeded`, `backup_failed` and `backup_missed`.
//!
//! Every backup path (database, volume, instance; manual and scheduled) builds a
//! [`BackupEvent`] and hands it to [`notify_backup`], which renders it into the
//! shared [`NotificationPayload`] that all channel formatters understand.

use std::time::Duration;

use crate::db::NotificationEventType;
use crate::notifications::{NotificationExtras, NotificationPayload, NotificationService};
use crate::DbPool;

/// What was backed up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupResourceKind {
    Database,
    Volume,
    Instance,
}

impl BackupResourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Database => "database",
            Self::Volume => "volume",
            Self::Instance => "instance",
        }
    }
}

/// How the backup ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupOutcome {
    Succeeded,
    Failed,
    Missed,
}

impl BackupOutcome {
    fn event_type(&self) -> NotificationEventType {
        match self {
            Self::Succeeded => NotificationEventType::BackupSucceeded,
            Self::Failed => NotificationEventType::BackupFailed,
            Self::Missed => NotificationEventType::BackupMissed,
        }
    }

    fn status(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Missed => "missed",
        }
    }
}

/// One backup result to report.
#[derive(Debug, Clone)]
pub struct BackupEvent {
    pub outcome: BackupOutcome,
    pub kind: BackupResourceKind,
    /// Database or volume id (None for the instance)
    pub resource_id: Option<String>,
    pub resource_name: String,
    /// Owning app for volume backups, so per-app subscriptions also match
    pub app_id: Option<String>,
    /// manual or scheduled
    pub trigger: Option<String>,
    pub size_bytes: Option<i64>,
    pub duration: Option<Duration>,
    /// "local", or "S3 (<config name>)"
    pub destination: Option<String>,
    pub error: Option<String>,
    /// For missed runs: when the run was due
    pub expected_at: Option<String>,
}

impl BackupEvent {
    pub fn new(outcome: BackupOutcome, kind: BackupResourceKind, name: impl Into<String>) -> Self {
        Self {
            outcome,
            kind,
            resource_id: None,
            resource_name: name.into(),
            app_id: None,
            trigger: None,
            size_bytes: None,
            duration: None,
            destination: None,
            error: None,
            expected_at: None,
        }
    }

    pub fn resource_id(mut self, id: impl Into<String>) -> Self {
        self.resource_id = Some(id.into());
        self
    }

    pub fn app_id(mut self, id: Option<String>) -> Self {
        self.app_id = id;
        self
    }

    pub fn trigger(mut self, trigger: impl Into<String>) -> Self {
        self.trigger = Some(trigger.into());
        self
    }

    pub fn size(mut self, bytes: Option<i64>) -> Self {
        self.size_bytes = bytes;
        self
    }

    pub fn duration(mut self, duration: Option<Duration>) -> Self {
        self.duration = duration;
        self
    }

    pub fn destination(mut self, destination: impl Into<String>) -> Self {
        self.destination = Some(destination.into());
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn expected_at(mut self, at: impl Into<String>) -> Self {
        self.expected_at = Some(at.into());
        self
    }

    /// Human-readable message line for the notification.
    fn message(&self) -> String {
        let what = match self.kind {
            BackupResourceKind::Instance => "the Rivetr instance".to_string(),
            kind => format!("{} '{}'", kind.as_str(), self.resource_name),
        };
        let trigger = match self.trigger.as_deref() {
            Some("scheduled") => "Scheduled backup",
            Some("manual") => "Manual backup",
            _ => "Backup",
        };
        match self.outcome {
            BackupOutcome::Succeeded => format!("{trigger} of {what} completed."),
            BackupOutcome::Failed => format!("{trigger} of {what} failed."),
            BackupOutcome::Missed => match self.expected_at.as_deref() {
                Some(at) => format!("Scheduled backup of {what} did not run (due {at})."),
                None => format!("Scheduled backup of {what} did not run."),
            },
        }
    }

    /// Render into the payload every channel formatter takes.
    pub fn to_payload(&self) -> NotificationPayload {
        let mut fields: Vec<(String, String)> = Vec::new();
        if let Some(ref trigger) = self.trigger {
            fields.push(("Trigger".into(), trigger.clone()));
        }
        if let Some(bytes) = self.size_bytes {
            fields.push(("Size".into(), format_size(bytes)));
        }
        if let Some(duration) = self.duration {
            fields.push(("Duration".into(), format_duration(duration)));
        }
        if let Some(ref destination) = self.destination {
            fields.push(("Destination".into(), destination.clone()));
        }
        if let Some(ref at) = self.expected_at {
            fields.push(("Expected at".into(), at.clone()));
        }

        NotificationPayload {
            event_type: self.outcome.event_type(),
            app_id: self.app_id.clone().unwrap_or_default(),
            app_name: self.resource_name.clone(),
            deployment_id: None,
            status: self.outcome.status().to_string(),
            message: self.message(),
            error_message: self.error.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            extras: NotificationExtras {
                resource_type: Some(self.kind.as_str().to_string()),
                resource_id: self.resource_id.clone(),
                fields,
            },
        }
    }
}

/// `1.5 MB`-style size.
pub fn format_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{} B", bytes.max(0));
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", value, UNITS[unit])
}

/// `2m 05s`-style duration.
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 1 {
        return format!("{} ms", d.as_millis());
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    if secs < 3600 {
        return format!("{}m {:02}s", secs / 60, secs % 60);
    }
    format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
}

/// Destination label for notifications and history.
pub fn s3_destination(config_name: &str) -> String {
    format!("S3 ({config_name})")
}

/// Send the notification now (errors are logged, never returned).
pub async fn send_backup_notification(db: &DbPool, event: &BackupEvent) {
    let payload = event.to_payload();
    if let Err(e) = NotificationService::new(db.clone()).send(&payload).await {
        tracing::warn!(error = %e, event = %payload.event_type, "Failed to send backup notification");
    }
}

/// Send the notification in the background.
pub fn notify_backup(db: &DbPool, event: BackupEvent) {
    let db = db.clone();
    tokio::spawn(async move {
        send_backup_notification(&db, &event).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn succeeded() -> BackupEvent {
        BackupEvent::new(
            BackupOutcome::Succeeded,
            BackupResourceKind::Database,
            "orders-db",
        )
        .resource_id("db-1")
        .trigger("scheduled")
        .size(Some(5 * 1024 * 1024))
        .duration(Some(Duration::from_secs(75)))
        .destination(s3_destination("offsite"))
    }

    #[test]
    fn succeeded_event_payload() {
        let p = succeeded().to_payload();
        assert_eq!(p.event_type, NotificationEventType::BackupSucceeded);
        assert_eq!(p.title(), "Backup Succeeded: orders-db");
        assert_eq!(p.subject_label(), "Database");
        assert_eq!(p.status, "succeeded");
        assert_eq!(
            p.message,
            "Scheduled backup of database 'orders-db' completed."
        );
        assert_eq!(p.extras.resource_type.as_deref(), Some("database"));
        assert_eq!(p.extras.resource_id.as_deref(), Some("db-1"));
        let fields: Vec<(&str, &str)> = p
            .detail_fields()
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        assert_eq!(
            fields,
            vec![
                ("Trigger", "scheduled"),
                ("Size", "5.0 MB"),
                ("Duration", "1m 15s"),
                ("Destination", "S3 (offsite)"),
            ]
        );
        assert!(p.error_message.is_none());
    }

    #[test]
    fn failed_event_carries_error() {
        let p = BackupEvent::new(BackupOutcome::Failed, BackupResourceKind::Volume, "uploads")
            .resource_id("vol-1")
            .app_id(Some("app-1".into()))
            .trigger("manual")
            .destination("local")
            .error("tar exited 2")
            .to_payload();
        assert_eq!(p.event_type, NotificationEventType::BackupFailed);
        assert_eq!(p.title(), "Backup Failed: uploads");
        assert_eq!(p.subject_label(), "Volume");
        assert_eq!(p.app_id, "app-1");
        assert_eq!(p.error_message.as_deref(), Some("tar exited 2"));
        assert_eq!(p.message, "Manual backup of volume 'uploads' failed.");
    }

    #[test]
    fn missed_event_mentions_due_time() {
        let p = BackupEvent::new(
            BackupOutcome::Missed,
            BackupResourceKind::Instance,
            "Rivetr",
        )
        .expected_at("2026-11-05T02:00:00Z")
        .to_payload();
        assert_eq!(p.event_type, NotificationEventType::BackupMissed);
        assert_eq!(p.title(), "Backup Missed: Rivetr");
        assert_eq!(p.subject_label(), "Instance");
        assert_eq!(
            p.message,
            "Scheduled backup of the Rivetr instance did not run (due 2026-11-05T02:00:00Z)."
        );
        assert!(p.extras.resource_id.is_none());
        assert_eq!(p.color(), "#f39c12");
    }

    #[test]
    fn size_and_duration_formatting() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert_eq!(format_duration(Duration::from_millis(250)), "250 ms");
        assert_eq!(format_duration(Duration::from_secs(42)), "42s");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1h 02m");
    }
}
