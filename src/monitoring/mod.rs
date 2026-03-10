//! Advanced monitoring subsystem.
//!
//! Provides background tasks for:
//! - Uptime checking (HTTP health checks every 60 seconds)
//! - Log cleanup based on retention policies
//! - Scheduled container restarts

pub mod log_cleaner;
pub mod uptime;

pub use log_cleaner::spawn_log_cleaner_task;
pub use uptime::spawn_uptime_checker_task;
