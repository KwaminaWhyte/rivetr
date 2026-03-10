//! System-level API endpoints for dashboard statistics.
//!
//! Provides aggregate system stats, disk stats, recent events, and instance backup/restore.

mod backup;
mod health;
mod updates;

// Re-export everything callers need
pub use backup::{
    create_backup, create_backup_schedule, delete_backup, delete_backup_schedule, download_backup,
    list_backup_schedules, list_backups, restore_backup, toggle_backup_schedule,
    upload_backup_to_s3,
};
pub use health::{
    get_detailed_health, get_disk_stats, get_recent_events, get_stats_history, get_stats_summary,
    get_system_stats,
};
pub use updates::{apply_update, check_for_updates, download_update, get_version_info};
