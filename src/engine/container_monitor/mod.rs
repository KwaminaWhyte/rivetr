//! Container monitor module
//!
//! This module monitors running containers and automatically restarts them
//! if they crash. It implements exponential backoff for restart attempts
//! to prevent rapid restart loops.
//!
//! Monitors:
//! - App deployments (from `deployments` table)
//! - Managed databases (from `databases` table)
//! - Docker Compose services (from `services` table)

mod health;
mod recovery;
mod stats;

pub use recovery::ContainerRestartState;

use crate::api::metrics::{set_active_apps_total, set_active_databases_total};
use crate::config::ContainerMonitorConfig;
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

/// Container monitor that detects crashed containers and restarts them
pub struct ContainerMonitor {
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: ContainerMonitorConfig,
    /// Data directory for Docker Compose services (reserved for future use)
    #[allow(dead_code)]
    data_dir: PathBuf,
    /// Tracks restart state per container (keyed by container name)
    restart_states: HashMap<String, ContainerRestartState>,
    /// Tracks containers that are known to be healthy (have been running for stable_duration)
    /// Value is the time when the container was first seen as running
    healthy_containers: HashMap<String, Instant>,
}

impl ContainerMonitor {
    pub fn new(
        db: DbPool,
        runtime: Arc<dyn ContainerRuntime>,
        config: ContainerMonitorConfig,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            runtime,
            config,
            data_dir,
            restart_states: HashMap::new(),
            healthy_containers: HashMap::new(),
        }
    }

    /// Run a single monitoring cycle
    pub async fn check_and_restart(&mut self) -> MonitorResult {
        health::check_and_restart(
            &self.db,
            &self.runtime,
            &self.config,
            &mut self.restart_states,
            &mut self.healthy_containers,
        )
        .await
    }
}

/// Result of a monitoring cycle
#[derive(Debug, Default)]
pub struct MonitorResult {
    /// Number of deployments checked
    pub deployments_checked: usize,
    /// Number of containers that are running
    pub containers_running: usize,
    /// Number of containers that were found crashed
    pub containers_crashed: usize,
    /// Number of containers that were successfully restarted
    pub containers_restarted: usize,
    /// Number of restart attempts that failed
    pub restart_failures: usize,
    /// Number of databases checked
    pub databases_checked: usize,
    /// Number of databases running
    pub databases_running: usize,
    /// Number of databases stopped (status updated)
    pub databases_stopped: usize,
    /// Number of services checked
    pub services_checked: usize,
    /// Number of services running
    pub services_running: usize,
    /// Number of services stopped (status updated)
    pub services_stopped: usize,
}

/// Spawn the background container monitoring task
pub fn spawn_container_monitor_task(
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: ContainerMonitorConfig,
    data_dir: PathBuf,
) {
    if !config.enabled {
        tracing::info!("Container monitoring is disabled");
        return;
    }

    let check_interval = config.check_interval_secs;
    tracing::info!(
        check_interval_secs = check_interval,
        max_restart_attempts = config.max_restart_attempts,
        initial_backoff_secs = config.initial_backoff_secs,
        max_backoff_secs = config.max_backoff_secs,
        stable_duration_secs = config.stable_duration_secs,
        "Starting container monitor task (monitoring deployments, databases, and services)"
    );

    let mut monitor = ContainerMonitor::new(db, runtime, config, data_dir);

    tokio::spawn(async move {
        // Wait a bit before the first check to let the system stabilize
        tokio::time::sleep(Duration::from_secs(10)).await;

        let mut tick = interval(Duration::from_secs(check_interval));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;

            let result = monitor.check_and_restart().await;

            // Update active apps/databases Prometheus gauges after each cycle
            set_active_apps_total(result.containers_running as f64);
            set_active_databases_total(result.databases_running as f64);

            let has_app_changes = result.containers_crashed > 0 || result.containers_restarted > 0;
            let has_db_changes = result.databases_stopped > 0;
            let has_svc_changes = result.services_stopped > 0;

            if has_app_changes || has_db_changes || has_svc_changes {
                tracing::info!(
                    apps_checked = result.deployments_checked,
                    apps_running = result.containers_running,
                    apps_crashed = result.containers_crashed,
                    apps_restarted = result.containers_restarted,
                    restart_failures = result.restart_failures,
                    databases_checked = result.databases_checked,
                    databases_running = result.databases_running,
                    databases_stopped = result.databases_stopped,
                    services_checked = result.services_checked,
                    services_running = result.services_running,
                    services_stopped = result.services_stopped,
                    "Container monitor cycle completed (status changes detected)"
                );
            } else {
                tracing::debug!(
                    apps_checked = result.deployments_checked,
                    apps_running = result.containers_running,
                    databases_checked = result.databases_checked,
                    databases_running = result.databases_running,
                    services_checked = result.services_checked,
                    services_running = result.services_running,
                    "Container monitor cycle completed (all healthy)"
                );
            }
        }
    });
}

/// Reconcile container status on startup
///
/// This function checks all "running" status records in the database
/// and updates them if the corresponding containers are not actually running.
/// Should be called during server startup.
pub async fn reconcile_container_status(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) {
    tracing::info!("Reconciling container status on startup...");

    let deployments_updated = recovery::reconcile_deployments(db, runtime).await;
    let databases_updated = recovery::reconcile_databases(db, runtime).await;
    let services_updated = recovery::reconcile_services(db, runtime).await;

    tracing::info!(
        deployments = deployments_updated,
        databases = databases_updated,
        services = services_updated,
        "Container status reconciliation completed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_result_default() {
        let result = MonitorResult::default();
        assert_eq!(result.deployments_checked, 0);
        assert_eq!(result.containers_running, 0);
        assert_eq!(result.containers_crashed, 0);
        assert_eq!(result.containers_restarted, 0);
        assert_eq!(result.restart_failures, 0);
        // New fields for databases and services
        assert_eq!(result.databases_checked, 0);
        assert_eq!(result.databases_running, 0);
        assert_eq!(result.databases_stopped, 0);
        assert_eq!(result.services_checked, 0);
        assert_eq!(result.services_running, 0);
        assert_eq!(result.services_stopped, 0);
    }
}
