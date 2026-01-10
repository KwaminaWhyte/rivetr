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

use crate::api::metrics::{increment_container_restarts, set_container_restart_backoff_seconds};
use crate::config::ContainerMonitorConfig;
use crate::db::{Deployment, ManagedDatabase, Service};
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

/// Tracks restart state for a single container
#[derive(Debug, Clone)]
struct ContainerRestartState {
    /// Number of restart attempts for this container
    restart_count: u32,
    /// Last restart attempt time
    last_restart: Option<Instant>,
    /// Current backoff delay in seconds
    current_backoff_secs: u64,
    /// Whether the container has been marked as failed (exceeded max restarts)
    failed: bool,
}

impl ContainerRestartState {
    fn new(initial_backoff_secs: u64) -> Self {
        Self {
            restart_count: 0,
            last_restart: None,
            current_backoff_secs: initial_backoff_secs,
            failed: false,
        }
    }

    /// Check if we should attempt a restart based on backoff timing
    fn should_restart(&self) -> bool {
        if self.failed {
            return false;
        }

        match self.last_restart {
            Some(last) => last.elapsed() >= Duration::from_secs(self.current_backoff_secs),
            None => true,
        }
    }

    /// Record a restart attempt and update backoff
    fn record_restart(&mut self, max_backoff_secs: u64) {
        self.restart_count += 1;
        self.last_restart = Some(Instant::now());
        // Exponential backoff: double the delay each time, up to max
        self.current_backoff_secs = (self.current_backoff_secs * 2).min(max_backoff_secs);
    }

    /// Reset state after successful restart (container has been running for a while)
    fn reset(&mut self, initial_backoff_secs: u64) {
        self.restart_count = 0;
        self.last_restart = None;
        self.current_backoff_secs = initial_backoff_secs;
        self.failed = false;
    }

    /// Mark as failed (exceeded max restarts)
    fn mark_failed(&mut self) {
        self.failed = true;
    }
}

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
        let mut result = MonitorResult::default();

        if !self.config.enabled {
            return result;
        }

        // Get all deployments that should be running
        let running_deployments: Vec<Deployment> = match sqlx::query_as(
            r#"
            SELECT id, app_id, commit_sha, commit_message, status, container_id,
                   error_message, started_at, finished_at, image_tag,
                   rollback_from_deployment_id, is_auto_rollback
            FROM deployments
            WHERE status = 'running' AND container_id IS NOT NULL
            "#,
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(deployments) => deployments,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch running deployments");
                return result;
            }
        };

        result.deployments_checked = running_deployments.len();

        for deployment in &running_deployments {
            let container_id = match &deployment.container_id {
                Some(id) if !id.is_empty() => id,
                _ => continue,
            };

            // Get app name for logging and metrics
            let app_name: Option<(String,)> = sqlx::query_as("SELECT name FROM apps WHERE id = ?")
                .bind(&deployment.app_id)
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten();

            let app_name = app_name
                .map(|(name,)| name)
                .unwrap_or_else(|| "unknown".to_string());

            let container_name = format!("rivetr-{}", app_name);

            // Check if container is running
            match self.runtime.inspect(container_id).await {
                Ok(info) => {
                    if info.running {
                        // Container is running, track it for health reset
                        self.handle_running_container(&container_name, &app_name);
                        result.containers_running += 1;
                    } else {
                        // Container exists but is not running - it crashed
                        result.containers_crashed += 1;
                        self.handle_crashed_container(
                            deployment,
                            container_id,
                            &container_name,
                            &app_name,
                            &mut result,
                        )
                        .await;
                    }
                }
                Err(e) => {
                    // Container doesn't exist or can't be inspected
                    tracing::debug!(
                        container = %container_id,
                        app = %app_name,
                        error = %e,
                        "Failed to inspect container"
                    );
                    // This might mean the container was removed - mark deployment as failed
                    if let Err(e) = self
                        .mark_deployment_failed(&deployment.id, "Container not found")
                        .await
                    {
                        tracing::warn!(
                            deployment = %deployment.id,
                            error = %e,
                            "Failed to update deployment status"
                        );
                    }
                }
            }
        }

        // Clean up old restart states for containers that no longer exist
        self.cleanup_stale_states(&running_deployments);

        // Check managed databases
        self.check_databases(&mut result).await;

        // Check Docker Compose services
        self.check_services(&mut result).await;

        result
    }

    /// Check managed databases for stopped/crashed containers
    async fn check_databases(&mut self, result: &mut MonitorResult) {
        // Get all databases that should be running
        let running_databases: Vec<ManagedDatabase> = match sqlx::query_as(
            r#"
            SELECT id, name, db_type, version, container_id, status, internal_port,
                   external_port, public_access, credentials, volume_name, volume_path,
                   memory_limit, cpu_limit, error_message, project_id, created_at, updated_at
            FROM databases
            WHERE status = 'running' AND container_id IS NOT NULL
            "#,
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(databases) => databases,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch running databases");
                return;
            }
        };

        result.databases_checked = running_databases.len();

        for database in &running_databases {
            let container_id = match &database.container_id {
                Some(id) if !id.is_empty() => id,
                _ => continue,
            };

            let container_name = database.container_name();

            // Check if container is running
            match self.runtime.inspect(container_id).await {
                Ok(info) => {
                    if info.running {
                        result.databases_running += 1;
                    } else {
                        // Container exists but is not running - it stopped/crashed
                        result.databases_stopped += 1;
                        tracing::warn!(
                            database = %database.name,
                            container = %container_name,
                            "Database container stopped"
                        );

                        // Update database status to stopped
                        if let Err(e) = self.mark_database_stopped(&database.id).await {
                            tracing::warn!(
                                database = %database.id,
                                error = %e,
                                "Failed to update database status"
                            );
                        }
                    }
                }
                Err(e) => {
                    // Container doesn't exist or can't be inspected
                    tracing::debug!(
                        container = %container_id,
                        database = %database.name,
                        error = %e,
                        "Failed to inspect database container"
                    );

                    result.databases_stopped += 1;

                    // Mark database as stopped (container not found)
                    if let Err(e) = self
                        .mark_database_failed(&database.id, "Container not found")
                        .await
                    {
                        tracing::warn!(
                            database = %database.id,
                            error = %e,
                            "Failed to update database status"
                        );
                    }
                }
            }
        }
    }

    /// Check Docker Compose services for stopped/crashed containers
    async fn check_services(&mut self, result: &mut MonitorResult) {
        // Get all services that should be running
        let running_services: Vec<Service> = match sqlx::query_as(
            r#"
            SELECT id, name, project_id, team_id, compose_content, status, error_message, created_at, updated_at
            FROM services
            WHERE status = 'running'
            "#,
        )
        .fetch_all(&self.db)
        .await
        {
            Ok(services) => services,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch running services");
                return;
            }
        };

        result.services_checked = running_services.len();

        for service in &running_services {
            let project_name = service.compose_project_name();

            // Check if any containers are running for this compose project
            // We use docker compose ps to check the status
            let is_running = self
                .check_compose_service_running(&project_name, &service.name)
                .await;

            if is_running {
                result.services_running += 1;
            } else {
                result.services_stopped += 1;
                tracing::warn!(
                    service = %service.name,
                    project = %project_name,
                    "Docker Compose service stopped"
                );

                // Update service status to stopped
                if let Err(e) = self.mark_service_stopped(&service.id).await {
                    tracing::warn!(
                        service = %service.id,
                        error = %e,
                        "Failed to update service status"
                    );
                }
            }
        }
    }

    /// Check if a Docker Compose service is running
    async fn check_compose_service_running(&self, project_name: &str, service_name: &str) -> bool {
        use tokio::process::Command;

        // Try docker compose ps first (modern), then docker-compose (legacy)
        let output = Command::new("docker")
            .arg("compose")
            .arg("-p")
            .arg(project_name)
            .arg("ps")
            .arg("--format")
            .arg("json")
            .output()
            .await;

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Check if any containers are running
                    // JSON format contains "State":"running" for running containers
                    if stdout.contains("\"State\":\"running\"")
                        || stdout.contains("\"Status\":\"running\"")
                    {
                        return true;
                    }
                    // Also check for older format
                    if stdout.contains("running") && !stdout.trim().is_empty() {
                        return true;
                    }
                    false
                } else {
                    // Try legacy docker-compose
                    self.check_compose_service_running_legacy(project_name, service_name)
                        .await
                }
            }
            Err(_) => {
                self.check_compose_service_running_legacy(project_name, service_name)
                    .await
            }
        }
    }

    /// Check if a Docker Compose service is running using legacy docker-compose command
    async fn check_compose_service_running_legacy(
        &self,
        project_name: &str,
        _service_name: &str,
    ) -> bool {
        use tokio::process::Command;

        let output = Command::new("docker-compose")
            .arg("-p")
            .arg(project_name)
            .arg("ps")
            .arg("-q")
            .output()
            .await;

        match output {
            Ok(output) => {
                // If we get any container IDs, service has containers
                // We still need to check if they're running
                let container_ids = String::from_utf8_lossy(&output.stdout);
                if container_ids.trim().is_empty() {
                    return false;
                }

                // Check each container ID to see if it's running
                for container_id in container_ids.lines() {
                    let container_id = container_id.trim();
                    if container_id.is_empty() {
                        continue;
                    }

                    if let Ok(info) = self.runtime.inspect(container_id).await {
                        if info.running {
                            return true;
                        }
                    }
                }

                false
            }
            Err(_) => false,
        }
    }

    /// Handle a container that is currently running
    fn handle_running_container(&mut self, container_name: &str, app_name: &str) {
        // Track when we first saw this container as running
        if !self.healthy_containers.contains_key(container_name) {
            self.healthy_containers
                .insert(container_name.to_string(), Instant::now());
        }

        // Check if the container has been running long enough to be considered stable
        if let Some(first_seen) = self.healthy_containers.get(container_name) {
            let stable_duration = Duration::from_secs(self.config.stable_duration_secs);
            if first_seen.elapsed() >= stable_duration {
                // Container is stable, reset its restart state
                if let Some(state) = self.restart_states.get_mut(container_name) {
                    if state.restart_count > 0 {
                        tracing::info!(
                            container = %container_name,
                            app = %app_name,
                            "Container stable, resetting restart counter"
                        );
                        state.reset(self.config.initial_backoff_secs);
                        // Update metric
                        set_container_restart_backoff_seconds(app_name, 0.0);
                    }
                }
            }
        }
    }

    /// Handle a crashed container
    async fn handle_crashed_container(
        &mut self,
        deployment: &Deployment,
        container_id: &str,
        container_name: &str,
        app_name: &str,
        result: &mut MonitorResult,
    ) {
        // Remove from healthy containers tracking
        self.healthy_containers.remove(container_name);

        let initial_backoff = self.config.initial_backoff_secs;
        let max_backoff = self.config.max_backoff_secs;
        let max_attempts = self.config.max_restart_attempts;

        // Get or create restart state and extract needed info
        let state = self
            .restart_states
            .entry(container_name.to_string())
            .or_insert_with(|| ContainerRestartState::new(initial_backoff));

        // Check if we've exceeded max restart attempts
        if state.restart_count >= max_attempts {
            if !state.failed {
                let restart_count = state.restart_count;
                state.mark_failed();

                tracing::error!(
                    container = %container_name,
                    app = %app_name,
                    attempts = restart_count,
                    "Container exceeded maximum restart attempts, marking as failed"
                );

                // Update deployment status to failed
                if let Err(e) = self
                    .mark_deployment_failed(
                        &deployment.id,
                        &format!("Exceeded maximum restart attempts ({})", max_attempts),
                    )
                    .await
                {
                    tracing::warn!(
                        deployment = %deployment.id,
                        error = %e,
                        "Failed to update deployment status"
                    );
                }

                // Log to deployment logs
                self.add_deployment_log(
                    &deployment.id,
                    "ERROR",
                    &format!(
                        "Container crashed and exceeded maximum restart attempts ({}). Manual intervention required.",
                        max_attempts
                    ),
                ).await;
            }
            return;
        }

        // Check if we should wait for backoff
        if !state.should_restart() {
            let remaining = state
                .last_restart
                .map(|last| {
                    let elapsed = last.elapsed().as_secs();
                    state.current_backoff_secs.saturating_sub(elapsed)
                })
                .unwrap_or(0);

            tracing::debug!(
                container = %container_name,
                app = %app_name,
                remaining_secs = remaining,
                "Waiting for backoff before restart"
            );
            return;
        }

        // Extract state info before async operations
        let current_attempt = state.restart_count + 1;

        // Attempt to restart the container
        tracing::info!(
            container = %container_name,
            app = %app_name,
            attempt = current_attempt,
            max_attempts = max_attempts,
            "Attempting to restart crashed container"
        );

        // Log restart attempt to deployment logs
        self.add_deployment_log(
            &deployment.id,
            "WARN",
            &format!(
                "Container crashed, attempting restart (attempt {}/{})",
                current_attempt, max_attempts
            ),
        )
        .await;

        let restart_result = self.runtime.start(container_id).await;

        // Update state after restart attempt
        let state = self
            .restart_states
            .get_mut(container_name)
            .expect("State should exist");

        match restart_result {
            Ok(_) => {
                state.record_restart(max_backoff);
                let new_backoff = state.current_backoff_secs;
                let restart_count = state.restart_count;
                result.containers_restarted += 1;

                // Update Prometheus metrics
                increment_container_restarts(app_name);
                set_container_restart_backoff_seconds(app_name, new_backoff as f64);

                tracing::info!(
                    container = %container_name,
                    app = %app_name,
                    next_backoff_secs = new_backoff,
                    "Container restarted successfully"
                );

                // Log success to deployment logs
                self.add_deployment_log(
                    &deployment.id,
                    "INFO",
                    &format!(
                        "Container restarted successfully (attempt {}/{}). Next backoff: {}s",
                        restart_count, max_attempts, new_backoff
                    ),
                )
                .await;
            }
            Err(e) => {
                state.record_restart(max_backoff);
                result.restart_failures += 1;

                tracing::error!(
                    container = %container_name,
                    app = %app_name,
                    error = %e,
                    "Failed to restart container"
                );

                // Log failure to deployment logs
                self.add_deployment_log(
                    &deployment.id,
                    "ERROR",
                    &format!("Failed to restart container: {}", e),
                )
                .await;
            }
        }
    }

    /// Mark a deployment as failed in the database
    async fn mark_deployment_failed(&self, deployment_id: &str, error: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE deployments SET status = 'failed', error_message = ?, finished_at = ? WHERE id = ?"
        )
        .bind(error)
        .bind(&now)
        .bind(deployment_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Add a log entry for a deployment
    async fn add_deployment_log(&self, deployment_id: &str, level: &str, message: &str) {
        if let Err(e) = sqlx::query(
            "INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)",
        )
        .bind(deployment_id)
        .bind(level)
        .bind(message)
        .execute(&self.db)
        .await
        {
            tracing::warn!(
                deployment = %deployment_id,
                error = %e,
                "Failed to add deployment log"
            );
        }
    }

    /// Clean up restart states for containers that are no longer being tracked
    fn cleanup_stale_states(&mut self, _running_deployments: &[Deployment]) {
        // For now, we'll keep all states as they might be useful for debugging
        // The states are small and don't accumulate significantly.
        // In the future, we could add a TTL-based cleanup where we remove
        // states for containers that haven't been seen in a while.
    }

    /// Mark a database as stopped in the database
    async fn mark_database_stopped(&self, database_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE databases SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
        )
        .bind(database_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Mark a database as failed in the database
    async fn mark_database_failed(&self, database_id: &str, error: &str) -> Result<()> {
        sqlx::query(
            "UPDATE databases SET status = 'failed', error_message = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(error)
        .bind(database_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Mark a service as stopped in the database
    async fn mark_service_stopped(&self, service_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE services SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
        )
        .bind(service_id)
        .execute(&self.db)
        .await?;

        Ok(())
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

            // Log summary of status changes
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

    // Reconcile deployment containers
    let deployments_updated = reconcile_deployments(db, runtime).await;

    // Reconcile database containers
    let databases_updated = reconcile_databases(db, runtime).await;

    // Reconcile Docker Compose services
    let services_updated = reconcile_services(db, runtime).await;

    tracing::info!(
        deployments = deployments_updated,
        databases = databases_updated,
        services = services_updated,
        "Container status reconciliation completed"
    );
}

/// Reconcile deployment container status
async fn reconcile_deployments(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) -> usize {
    let running_deployments: Vec<Deployment> = match sqlx::query_as(
        r#"
        SELECT id, app_id, commit_sha, commit_message, status, container_id,
               error_message, started_at, finished_at, image_tag,
               rollback_from_deployment_id, is_auto_rollback
        FROM deployments
        WHERE status = 'running' AND container_id IS NOT NULL
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(deployments) => deployments,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running deployments for reconciliation");
            return 0;
        }
    };

    let mut updated = 0;

    for deployment in running_deployments {
        let container_id = match &deployment.container_id {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        // Check if container is actually running
        let is_running = match runtime.inspect(container_id).await {
            Ok(info) => info.running,
            Err(_) => false,
        };

        if !is_running {
            // Update deployment status
            if let Err(e) = sqlx::query(
                "UPDATE deployments SET status = 'stopped', finished_at = datetime('now') WHERE id = ?"
            )
            .bind(&deployment.id)
            .execute(db)
            .await
            {
                tracing::warn!(
                    deployment = %deployment.id,
                    error = %e,
                    "Failed to update deployment status during reconciliation"
                );
            } else {
                tracing::info!(
                    deployment = %deployment.id,
                    container = %container_id,
                    "Deployment status reconciled: running -> stopped"
                );
                updated += 1;
            }
        }
    }

    updated
}

/// Reconcile database container status
async fn reconcile_databases(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) -> usize {
    let running_databases: Vec<ManagedDatabase> = match sqlx::query_as(
        r#"
        SELECT id, name, db_type, version, container_id, status, internal_port,
               external_port, public_access, credentials, volume_name, volume_path,
               memory_limit, cpu_limit, error_message, project_id, created_at, updated_at
        FROM databases
        WHERE status = 'running' AND container_id IS NOT NULL
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(databases) => databases,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running databases for reconciliation");
            return 0;
        }
    };

    let mut updated = 0;

    for database in running_databases {
        let container_id = match &database.container_id {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        // Check if container is actually running
        let is_running = match runtime.inspect(container_id).await {
            Ok(info) => info.running,
            Err(_) => false,
        };

        if !is_running {
            // Update database status
            if let Err(e) = sqlx::query(
                "UPDATE databases SET status = 'stopped', updated_at = datetime('now') WHERE id = ?"
            )
            .bind(&database.id)
            .execute(db)
            .await
            {
                tracing::warn!(
                    database = %database.id,
                    error = %e,
                    "Failed to update database status during reconciliation"
                );
            } else {
                tracing::info!(
                    database = %database.name,
                    container = %container_id,
                    "Database status reconciled: running -> stopped"
                );
                updated += 1;
            }
        }
    }

    updated
}

/// Reconcile Docker Compose service status
async fn reconcile_services(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) -> usize {
    let running_services: Vec<Service> = match sqlx::query_as(
        r#"
        SELECT id, name, project_id, team_id, compose_content, status, error_message, created_at, updated_at
        FROM services
        WHERE status = 'running'
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(services) => services,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch running services for reconciliation");
            return 0;
        }
    };

    let mut updated = 0;

    for service in running_services {
        let project_name = service.compose_project_name();

        // Check if any containers are running for this compose project
        let is_running = check_compose_running(&project_name, runtime).await;

        if !is_running {
            // Update service status
            if let Err(e) = sqlx::query(
                "UPDATE services SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
            )
            .bind(&service.id)
            .execute(db)
            .await
            {
                tracing::warn!(
                    service = %service.id,
                    error = %e,
                    "Failed to update service status during reconciliation"
                );
            } else {
                tracing::info!(
                    service = %service.name,
                    project = %project_name,
                    "Service status reconciled: running -> stopped"
                );
                updated += 1;
            }
        }
    }

    updated
}

/// Check if a Docker Compose project has running containers
async fn check_compose_running(project_name: &str, runtime: &Arc<dyn ContainerRuntime>) -> bool {
    use tokio::process::Command;

    // Try docker compose ps first
    let output = Command::new("docker")
        .arg("compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("--format")
        .arg("json")
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("\"State\":\"running\"")
                    || stdout.contains("\"Status\":\"running\"")
                {
                    return true;
                }
                if stdout.contains("running") && !stdout.trim().is_empty() {
                    return true;
                }
                false
            } else {
                // Try legacy docker-compose
                check_compose_running_legacy(project_name, runtime).await
            }
        }
        Err(_) => check_compose_running_legacy(project_name, runtime).await,
    }
}

/// Check if a Docker Compose project has running containers using legacy command
async fn check_compose_running_legacy(
    project_name: &str,
    runtime: &Arc<dyn ContainerRuntime>,
) -> bool {
    use tokio::process::Command;

    let output = Command::new("docker-compose")
        .arg("-p")
        .arg(project_name)
        .arg("ps")
        .arg("-q")
        .output()
        .await;

    match output {
        Ok(output) => {
            let container_ids = String::from_utf8_lossy(&output.stdout);
            if container_ids.trim().is_empty() {
                return false;
            }

            for container_id in container_ids.lines() {
                let container_id = container_id.trim();
                if container_id.is_empty() {
                    continue;
                }

                if let Ok(info) = runtime.inspect(container_id).await {
                    if info.running {
                        return true;
                    }
                }
            }

            false
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restart_state_should_restart() {
        let mut state = ContainerRestartState::new(5);

        // Should restart immediately on first crash
        assert!(state.should_restart());

        // After recording restart, should wait for backoff
        state.record_restart(300);
        assert!(!state.should_restart());
        assert_eq!(state.restart_count, 1);
        assert_eq!(state.current_backoff_secs, 10); // 5 * 2 = 10
    }

    #[test]
    fn test_restart_state_exponential_backoff() {
        let mut state = ContainerRestartState::new(5);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 10);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 20);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 40);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 80);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 160);

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 300); // Capped at max

        state.record_restart(300);
        assert_eq!(state.current_backoff_secs, 300); // Still capped
    }

    #[test]
    fn test_restart_state_reset() {
        let mut state = ContainerRestartState::new(5);

        state.record_restart(300);
        state.record_restart(300);
        assert_eq!(state.restart_count, 2);
        assert_eq!(state.current_backoff_secs, 20);

        state.reset(5);
        assert_eq!(state.restart_count, 0);
        assert_eq!(state.current_backoff_secs, 5);
        assert!(!state.failed);
    }

    #[test]
    fn test_restart_state_mark_failed() {
        let mut state = ContainerRestartState::new(5);

        state.mark_failed();
        assert!(state.failed);
        assert!(!state.should_restart());
    }

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
