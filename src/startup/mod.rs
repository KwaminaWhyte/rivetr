//! Startup self-checks module
//!
//! This module performs system verification before the server starts accepting requests.
//! Checks include:
//! - Database connectivity and schema version
//! - Container runtime availability (Docker or Podman)
//! - Required directories exist and are writable
//! - Sufficient disk space available

use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tracing::{error, info, warn};

use crate::config::{Config, RuntimeType};
use crate::engine::DiskStats;
use crate::runtime::{ContainerRuntime, DockerRuntime, PodmanRuntime};
use crate::DbPool;

/// Minimum required disk space in bytes (default: 1GB)
const MIN_DISK_SPACE_BYTES: u64 = 1024 * 1024 * 1024;

/// Result of a single startup check
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Name of the check
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Whether this check is critical (failure should abort startup)
    pub critical: bool,
    /// Human-readable message describing the result
    pub message: String,
    /// Additional details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl CheckResult {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            critical: false,
            message: message.into(),
            details: None,
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>, critical: bool) -> Self {
        Self {
            name: name.into(),
            passed: false,
            critical,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Aggregated startup check results
#[derive(Debug, Clone, Serialize)]
pub struct StartupCheckReport {
    /// All check results
    pub checks: Vec<CheckResult>,
    /// Whether all critical checks passed
    pub all_critical_passed: bool,
    /// Whether all checks passed (including non-critical)
    pub all_passed: bool,
    /// Summary message
    pub summary: String,
}

impl StartupCheckReport {
    pub fn new(checks: Vec<CheckResult>) -> Self {
        let all_critical_passed = checks.iter().filter(|c| c.critical).all(|c| c.passed);
        let all_passed = checks.iter().all(|c| c.passed);

        let failed_critical = checks
            .iter()
            .filter(|c| c.critical && !c.passed)
            .count();
        let failed_non_critical = checks
            .iter()
            .filter(|c| !c.critical && !c.passed)
            .count();
        let total = checks.len();
        let passed = checks.iter().filter(|c| c.passed).count();

        let summary = if all_passed {
            format!("All {} startup checks passed", total)
        } else if all_critical_passed {
            format!(
                "{}/{} checks passed ({} non-critical warnings)",
                passed, total, failed_non_critical
            )
        } else {
            format!(
                "{}/{} checks passed ({} critical failures)",
                passed, total, failed_critical
            )
        };

        Self {
            checks,
            all_critical_passed,
            all_passed,
            summary,
        }
    }
}

/// Run all startup self-checks
pub async fn run_startup_checks(config: &Config, db: &DbPool) -> StartupCheckReport {
    info!("Running startup self-checks...");

    let mut checks = Vec::new();

    // 1. Database connectivity check
    checks.push(check_database_connectivity(db).await);

    // 2. Database schema version check
    checks.push(check_database_schema(db).await);

    // 3. Container runtime check
    checks.push(check_container_runtime(config).await);

    // 4. Required directories check
    checks.push(check_required_directories(config));

    // 5. Directory writability check
    checks.push(check_directory_writability(config));

    // 6. Disk space check
    checks.push(check_disk_space(&config.server.data_dir));

    let report = StartupCheckReport::new(checks);

    // Log results
    for check in &report.checks {
        if check.passed {
            info!(
                check = %check.name,
                message = %check.message,
                "Startup check PASSED"
            );
        } else if check.critical {
            error!(
                check = %check.name,
                message = %check.message,
                details = ?check.details,
                "Startup check FAILED (CRITICAL)"
            );
        } else {
            warn!(
                check = %check.name,
                message = %check.message,
                details = ?check.details,
                "Startup check FAILED (non-critical)"
            );
        }
    }

    info!(
        summary = %report.summary,
        all_passed = report.all_passed,
        all_critical_passed = report.all_critical_passed,
        "Startup checks completed"
    );

    report
}

/// Check database connectivity
async fn check_database_connectivity(db: &DbPool) -> CheckResult {
    match sqlx::query("SELECT 1").fetch_one(db).await {
        Ok(_) => CheckResult::pass(
            "database_connectivity",
            "Database connection successful",
        ),
        Err(e) => CheckResult::fail(
            "database_connectivity",
            "Failed to connect to database",
            true,
        )
        .with_details(e.to_string()),
    }
}

/// Check database schema version
async fn check_database_schema(db: &DbPool) -> CheckResult {
    // Count the number of tables to estimate schema version
    let result: Result<Vec<(String,)>, _> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    )
    .fetch_all(db)
    .await;

    match result {
        Ok(tables) => {
            let table_names: Vec<&str> = tables.iter().map(|(n,)| n.as_str()).collect();

            // Check for essential tables
            let essential_tables = ["apps", "deployments", "deployment_logs", "env_vars"];
            let missing: Vec<&str> = essential_tables
                .iter()
                .filter(|t| !table_names.contains(*t))
                .copied()
                .collect();

            if missing.is_empty() {
                CheckResult::pass(
                    "database_schema",
                    format!("Database schema valid ({} tables)", tables.len()),
                )
                .with_details(format!("Tables: {}", table_names.join(", ")))
            } else {
                CheckResult::fail(
                    "database_schema",
                    "Missing essential database tables",
                    true,
                )
                .with_details(format!("Missing: {}", missing.join(", ")))
            }
        }
        Err(e) => CheckResult::fail(
            "database_schema",
            "Failed to query database schema",
            true,
        )
        .with_details(e.to_string()),
    }
}

/// Check container runtime availability
async fn check_container_runtime(config: &Config) -> CheckResult {
    match config.runtime.runtime_type {
        RuntimeType::Docker => check_docker_runtime(&config.runtime.docker_socket).await,
        RuntimeType::Podman => check_podman_runtime().await,
        RuntimeType::Auto => {
            // Try Docker first, then Podman
            let docker_result = check_docker_runtime(&config.runtime.docker_socket).await;
            if docker_result.passed {
                return docker_result;
            }

            let podman_result = check_podman_runtime().await;
            if podman_result.passed {
                return podman_result;
            }

            // Neither available
            CheckResult::fail(
                "container_runtime",
                "No container runtime available (Docker or Podman)",
                false, // Non-critical: server can start but deployments won't work
            )
            .with_details("Install Docker or Podman to enable deployments")
        }
    }
}

async fn check_docker_runtime(docker_socket: &str) -> CheckResult {
    match DockerRuntime::new(docker_socket) {
        Ok(runtime) => {
            if runtime.is_available().await {
                CheckResult::pass("container_runtime", "Docker runtime available")
                    .with_details(format!("Socket: {}", docker_socket))
            } else {
                CheckResult::fail(
                    "container_runtime",
                    "Docker daemon not responding",
                    false,
                )
                .with_details(format!("Socket: {}", docker_socket))
            }
        }
        Err(e) => CheckResult::fail(
            "container_runtime",
            "Failed to connect to Docker",
            false,
        )
        .with_details(e.to_string()),
    }
}

async fn check_podman_runtime() -> CheckResult {
    let runtime = PodmanRuntime::new();
    if runtime.is_available().await {
        CheckResult::pass("container_runtime", "Podman runtime available")
    } else {
        CheckResult::fail(
            "container_runtime",
            "Podman not available",
            false,
        )
        .with_details("Podman command not found or not responding")
    }
}

/// Check that required directories exist
fn check_required_directories(config: &Config) -> CheckResult {
    let data_dir = &config.server.data_dir;
    let static_dir = Path::new("static");

    let mut missing = Vec::new();

    if !data_dir.exists() {
        missing.push(data_dir.display().to_string());
    }

    // static/ is optional (may not exist in development)
    let static_warning = if !static_dir.exists() {
        Some("static/ directory not found (frontend may not be served)".to_string())
    } else {
        None
    };

    if missing.is_empty() {
        let mut result = CheckResult::pass(
            "required_directories",
            "Required directories exist",
        )
        .with_details(format!("Data dir: {}", data_dir.display()));

        if let Some(warning) = static_warning {
            result.details = Some(format!(
                "{}; Note: {}",
                result.details.unwrap_or_default(),
                warning
            ));
        }

        result
    } else {
        CheckResult::fail(
            "required_directories",
            "Missing required directories",
            true,
        )
        .with_details(format!("Missing: {}", missing.join(", ")))
    }
}

/// Check that directories are writable
fn check_directory_writability(config: &Config) -> CheckResult {
    let data_dir = &config.server.data_dir;

    // Try to create a test file
    let test_file = data_dir.join(".rivetr_write_test");

    match std::fs::write(&test_file, "test") {
        Ok(_) => {
            // Clean up test file
            let _ = std::fs::remove_file(&test_file);
            CheckResult::pass(
                "directory_writability",
                "Data directory is writable",
            )
            .with_details(format!("Path: {}", data_dir.display()))
        }
        Err(e) => CheckResult::fail(
            "directory_writability",
            "Data directory is not writable",
            true,
        )
        .with_details(format!("{}: {}", data_dir.display(), e)),
    }
}

/// Check available disk space
fn check_disk_space(data_dir: &Path) -> CheckResult {
    match DiskStats::for_path(data_dir) {
        Ok(stats) => {
            let free_gb = stats.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let min_gb = MIN_DISK_SPACE_BYTES as f64 / (1024.0 * 1024.0 * 1024.0);

            if stats.free_bytes >= MIN_DISK_SPACE_BYTES {
                CheckResult::pass(
                    "disk_space",
                    format!("Sufficient disk space ({:.2} GB free)", free_gb),
                )
                .with_details(format!(
                    "Usage: {:.1}% ({:.2} GB used of {:.2} GB total)",
                    stats.usage_percent,
                    stats.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
                    stats.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                ))
            } else {
                CheckResult::fail(
                    "disk_space",
                    format!(
                        "Low disk space ({:.2} GB free, minimum {:.2} GB required)",
                        free_gb, min_gb
                    ),
                    false, // Non-critical: warn but allow startup
                )
                .with_details(format!("Usage: {:.1}%", stats.usage_percent))
            }
        }
        Err(e) => CheckResult::fail(
            "disk_space",
            "Failed to check disk space",
            false,
        )
        .with_details(e.to_string()),
    }
}

/// Get detailed system health status (for health API endpoint)
pub async fn get_system_health(config: &Config, db: &DbPool) -> SystemHealthStatus {
    let mut checks = Vec::new();

    // Database connectivity
    let db_check = check_database_connectivity(db).await;
    let database_healthy = db_check.passed;
    checks.push(db_check);

    // Container runtime
    let runtime_check = check_container_runtime(config).await;
    let runtime_healthy = runtime_check.passed;
    checks.push(runtime_check);

    // Disk space
    let disk_check = check_disk_space(&config.server.data_dir);
    let disk_healthy = disk_check.passed;
    checks.push(disk_check);

    // Directory writability
    let dir_check = check_directory_writability(config);
    checks.push(dir_check);

    let overall_healthy = database_healthy; // Only database is truly critical for health

    SystemHealthStatus {
        healthy: overall_healthy,
        database_healthy,
        runtime_healthy,
        disk_healthy,
        checks,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// System health status response
#[derive(Debug, Clone, Serialize)]
pub struct SystemHealthStatus {
    /// Overall system health
    pub healthy: bool,
    /// Database connectivity
    pub database_healthy: bool,
    /// Container runtime availability
    pub runtime_healthy: bool,
    /// Disk space status
    pub disk_healthy: bool,
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Rivetr version
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_pass() {
        let result = CheckResult::pass("test", "Test passed");
        assert!(result.passed);
        assert!(!result.critical);
        assert_eq!(result.name, "test");
    }

    #[test]
    fn test_check_result_fail() {
        let result = CheckResult::fail("test", "Test failed", true);
        assert!(!result.passed);
        assert!(result.critical);
    }

    #[test]
    fn test_startup_check_report_all_passed() {
        let checks = vec![
            CheckResult::pass("check1", "ok"),
            CheckResult::pass("check2", "ok"),
        ];
        let report = StartupCheckReport::new(checks);
        assert!(report.all_passed);
        assert!(report.all_critical_passed);
    }

    #[test]
    fn test_startup_check_report_critical_failure() {
        let checks = vec![
            CheckResult::pass("check1", "ok"),
            CheckResult::fail("check2", "fail", true),
        ];
        let report = StartupCheckReport::new(checks);
        assert!(!report.all_passed);
        assert!(!report.all_critical_passed);
    }

    #[test]
    fn test_startup_check_report_non_critical_failure() {
        let checks = vec![
            CheckResult::pass("check1", "ok"),
            CheckResult::fail("check2", "warn", false),
        ];
        let report = StartupCheckReport::new(checks);
        assert!(!report.all_passed);
        assert!(report.all_critical_passed); // Non-critical failures don't affect this
    }
}
