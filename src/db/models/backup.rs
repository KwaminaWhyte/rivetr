//! Database backup models.

use serde::{Deserialize, Serialize};

/// Database backup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupStatus::Pending => write!(f, "pending"),
            BackupStatus::Running => write!(f, "running"),
            BackupStatus::Completed => write!(f, "completed"),
            BackupStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Database backup type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Manual,
    Scheduled,
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupType::Manual => write!(f, "manual"),
            BackupType::Scheduled => write!(f, "scheduled"),
        }
    }
}

/// Schedule type for backups
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Hourly,
    Daily,
    Weekly,
}

impl std::fmt::Display for ScheduleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleType::Hourly => write!(f, "hourly"),
            ScheduleType::Daily => write!(f, "daily"),
            ScheduleType::Weekly => write!(f, "weekly"),
        }
    }
}

/// Database backup record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseBackup {
    pub id: String,
    pub database_id: String,
    pub backup_type: String,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub backup_format: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseBackup {
    pub fn new(database_id: &str, backup_type: BackupType) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            database_id: database_id.to_string(),
            backup_type: backup_type.to_string(),
            status: BackupStatus::Pending.to_string(),
            file_path: None,
            file_size: None,
            backup_format: None,
            started_at: None,
            completed_at: None,
            error_message: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn get_status(&self) -> BackupStatus {
        match self.status.as_str() {
            "pending" => BackupStatus::Pending,
            "running" => BackupStatus::Running,
            "completed" => BackupStatus::Completed,
            "failed" => BackupStatus::Failed,
            _ => BackupStatus::Pending,
        }
    }
}

/// Database backup response for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupResponse {
    pub id: String,
    pub database_id: String,
    pub backup_type: String,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub file_size_human: Option<String>,
    pub backup_format: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_seconds: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: String,
}

impl From<DatabaseBackup> for DatabaseBackupResponse {
    fn from(backup: DatabaseBackup) -> Self {
        let duration = match (&backup.started_at, &backup.completed_at) {
            (Some(start), Some(end)) => {
                let start_dt = chrono::DateTime::parse_from_rfc3339(start).ok();
                let end_dt = chrono::DateTime::parse_from_rfc3339(end).ok();
                match (start_dt, end_dt) {
                    (Some(s), Some(e)) => Some((e - s).num_seconds()),
                    _ => None,
                }
            }
            _ => None,
        };

        let file_size_human = backup.file_size.map(|size| {
            if size >= 1_073_741_824 {
                format!("{:.2} GB", size as f64 / 1_073_741_824.0)
            } else if size >= 1_048_576 {
                format!("{:.2} MB", size as f64 / 1_048_576.0)
            } else if size >= 1024 {
                format!("{:.2} KB", size as f64 / 1024.0)
            } else {
                format!("{} B", size)
            }
        });

        Self {
            id: backup.id,
            database_id: backup.database_id,
            backup_type: backup.backup_type,
            status: backup.status,
            file_path: backup.file_path,
            file_size: backup.file_size,
            file_size_human,
            backup_format: backup.backup_format,
            started_at: backup.started_at,
            completed_at: backup.completed_at,
            duration_seconds: duration,
            error_message: backup.error_message,
            created_at: backup.created_at,
        }
    }
}

/// Database backup schedule
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseBackupSchedule {
    pub id: String,
    pub database_id: String,
    pub enabled: i32,
    pub schedule_type: String,
    pub schedule_hour: i32,
    pub schedule_day: Option<i32>,
    pub retention_count: i32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseBackupSchedule {
    pub fn new(database_id: &str, schedule_type: ScheduleType) -> Self {
        let now = chrono::Utc::now();
        let next_run = Self::calculate_next_run(&schedule_type, 2, None, &now);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            database_id: database_id.to_string(),
            enabled: 1,
            schedule_type: schedule_type.to_string(),
            schedule_hour: 2, // Default 2 AM
            schedule_day: None,
            retention_count: 5,
            last_run_at: None,
            next_run_at: Some(next_run.to_rfc3339()),
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }

    pub fn get_schedule_type(&self) -> ScheduleType {
        match self.schedule_type.as_str() {
            "hourly" => ScheduleType::Hourly,
            "weekly" => ScheduleType::Weekly,
            _ => ScheduleType::Daily,
        }
    }

    pub fn is_due(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        match &self.next_run_at {
            Some(next) => {
                if let Ok(next_dt) = chrono::DateTime::parse_from_rfc3339(next) {
                    chrono::Utc::now() >= next_dt
                } else {
                    false
                }
            }
            None => false,
        }
    }

    pub fn calculate_next_run(
        schedule_type: &ScheduleType,
        hour: i32,
        day: Option<i32>,
        from: &chrono::DateTime<chrono::Utc>,
    ) -> chrono::DateTime<chrono::Utc> {
        use chrono::{Datelike, Duration, Timelike};

        match schedule_type {
            ScheduleType::Hourly => {
                // Next hour
                *from + Duration::hours(1)
            }
            ScheduleType::Daily => {
                // Next occurrence of the specified hour
                let mut next = from
                    .with_hour(hour as u32)
                    .unwrap_or(*from)
                    .with_minute(0)
                    .unwrap_or(*from)
                    .with_second(0)
                    .unwrap_or(*from);
                if next <= *from {
                    next = next + Duration::days(1);
                }
                next
            }
            ScheduleType::Weekly => {
                // Next occurrence of the specified day and hour
                let target_day = day.unwrap_or(0) as u32; // 0 = Sunday
                let current_weekday = from.weekday().num_days_from_sunday();
                let days_until = if current_weekday < target_day {
                    target_day - current_weekday
                } else if current_weekday > target_day {
                    7 - (current_weekday - target_day)
                } else {
                    // Same day, check if hour has passed
                    if from.hour() >= hour as u32 {
                        7
                    } else {
                        0
                    }
                };

                let mut next = *from + Duration::days(days_until as i64);
                next = next
                    .with_hour(hour as u32)
                    .unwrap_or(next)
                    .with_minute(0)
                    .unwrap_or(next)
                    .with_second(0)
                    .unwrap_or(next);
                next
            }
        }
    }

    pub fn update_next_run(&mut self) {
        let now = chrono::Utc::now();
        self.last_run_at = Some(now.to_rfc3339());
        self.next_run_at = Some(
            Self::calculate_next_run(
                &self.get_schedule_type(),
                self.schedule_hour,
                self.schedule_day,
                &now,
            )
            .to_rfc3339(),
        );
        self.updated_at = now.to_rfc3339();
    }
}

/// Response for backup schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupScheduleResponse {
    pub id: String,
    pub database_id: String,
    pub enabled: bool,
    pub schedule_type: String,
    pub schedule_hour: i32,
    pub schedule_day: Option<i32>,
    pub retention_count: i32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
}

impl From<DatabaseBackupSchedule> for DatabaseBackupScheduleResponse {
    fn from(schedule: DatabaseBackupSchedule) -> Self {
        let enabled = schedule.is_enabled();
        Self {
            id: schedule.id,
            database_id: schedule.database_id,
            enabled,
            schedule_type: schedule.schedule_type,
            schedule_hour: schedule.schedule_hour,
            schedule_day: schedule.schedule_day,
            retention_count: schedule.retention_count,
            last_run_at: schedule.last_run_at,
            next_run_at: schedule.next_run_at,
            created_at: schedule.created_at,
        }
    }
}

/// Request to create/update a backup schedule
#[derive(Debug, Deserialize)]
pub struct CreateBackupScheduleRequest {
    pub enabled: Option<bool>,
    pub schedule_type: Option<String>,
    pub schedule_hour: Option<i32>,
    pub schedule_day: Option<i32>,
    pub retention_count: Option<i32>,
}
