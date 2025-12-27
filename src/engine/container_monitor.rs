//! Container monitor module
//!
//! This module monitors running containers and automatically restarts them
//! if they crash. It implements exponential backoff for restart attempts
//! to prevent rapid restart loops.

use crate::api::metrics::{
    increment_container_restarts, set_container_restart_backoff_seconds,
};
use crate::config::ContainerMonitorConfig;
use crate::db::Deployment;
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use anyhow::Result;
use std::collections::HashMap;
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
    ) -> Self {
        Self {
            db,
            runtime,
            config,
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
                   image_tag, error_message, started_at, finished_at
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
            let app_name: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM apps WHERE id = ?"
            )
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
                    if let Err(e) = self.mark_deployment_failed(&deployment.id, "Container not found").await {
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

        result
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
                if let Err(e) = self.mark_deployment_failed(
                    &deployment.id,
                    &format!("Exceeded maximum restart attempts ({})", max_attempts),
                ).await {
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
                current_attempt,
                max_attempts
            ),
        ).await;

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
                        restart_count,
                        max_attempts,
                        new_backoff
                    ),
                ).await;
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
                ).await;
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
            "INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)"
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
}

/// Spawn the background container monitoring task
pub fn spawn_container_monitor_task(
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: ContainerMonitorConfig,
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
        "Starting container monitor task"
    );

    let mut monitor = ContainerMonitor::new(db, runtime, config);

    tokio::spawn(async move {
        // Wait a bit before the first check to let the system stabilize
        tokio::time::sleep(Duration::from_secs(10)).await;

        let mut tick = interval(Duration::from_secs(check_interval));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;

            let result = monitor.check_and_restart().await;

            if result.containers_crashed > 0 || result.containers_restarted > 0 {
                tracing::info!(
                    checked = result.deployments_checked,
                    running = result.containers_running,
                    crashed = result.containers_crashed,
                    restarted = result.containers_restarted,
                    failures = result.restart_failures,
                    "Container monitor cycle completed"
                );
            } else {
                tracing::debug!(
                    checked = result.deployments_checked,
                    running = result.containers_running,
                    "Container monitor cycle completed (all healthy)"
                );
            }
        }
    });
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
    }
}
