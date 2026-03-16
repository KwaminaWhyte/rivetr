//! Crash detection, exponential backoff recovery state, and DB reconciliation helpers.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::{Deployment, ManagedDatabase, Service};
use crate::runtime::ContainerRuntime;
use crate::DbPool;

/// Tracks restart state for a single container
#[derive(Debug, Clone)]
pub struct ContainerRestartState {
    /// Number of restart attempts for this container
    pub(super) restart_count: u32,
    /// Last restart attempt time
    pub(super) last_restart: Option<Instant>,
    /// Current backoff delay in seconds
    pub(super) current_backoff_secs: u64,
    /// Whether the container has been marked as failed (exceeded max restarts)
    pub(super) failed: bool,
}

impl ContainerRestartState {
    pub(super) fn new(initial_backoff_secs: u64) -> Self {
        Self {
            restart_count: 0,
            last_restart: None,
            current_backoff_secs: initial_backoff_secs,
            failed: false,
        }
    }

    /// Check if we should attempt a restart based on backoff timing
    pub(super) fn should_restart(&self) -> bool {
        if self.failed {
            return false;
        }

        match self.last_restart {
            Some(last) => last.elapsed() >= Duration::from_secs(self.current_backoff_secs),
            None => true,
        }
    }

    /// Record a restart attempt and update backoff
    pub(super) fn record_restart(&mut self, max_backoff_secs: u64) {
        self.restart_count += 1;
        self.last_restart = Some(Instant::now());
        // Exponential backoff: double the delay each time, up to max
        self.current_backoff_secs = (self.current_backoff_secs * 2).min(max_backoff_secs);
    }

    /// Reset state after successful restart (container has been running for a while)
    pub(super) fn reset(&mut self, initial_backoff_secs: u64) {
        self.restart_count = 0;
        self.last_restart = None;
        self.current_backoff_secs = initial_backoff_secs;
        self.failed = false;
    }

    /// Mark as failed (exceeded max restarts)
    pub(super) fn mark_failed(&mut self) {
        self.failed = true;
    }
}

/// Clean up restart states for containers that are no longer being tracked.
/// For now we keep all states as they are small and useful for debugging.
pub(super) fn cleanup_stale_states(
    _restart_states: &mut HashMap<String, ContainerRestartState>,
    _running_deployments: &[Deployment],
) {
    // Future: add TTL-based cleanup for containers not seen recently
}

/// Mark a deployment as failed in the database
pub(super) async fn mark_deployment_failed(
    db: &DbPool,
    deployment_id: &str,
    error: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE deployments SET status = 'failed', error_message = ?, finished_at = ? WHERE id = ?",
    )
    .bind(error)
    .bind(&now)
    .bind(deployment_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Add a log entry for a deployment
pub(super) async fn add_deployment_log(
    db: &DbPool,
    deployment_id: &str,
    level: &str,
    message: &str,
) {
    if let Err(e) =
        sqlx::query("INSERT INTO deployment_logs (deployment_id, level, message) VALUES (?, ?, ?)")
            .bind(deployment_id)
            .bind(level)
            .bind(message)
            .execute(db)
            .await
    {
        tracing::warn!(
            deployment = %deployment_id,
            error = %e,
            "Failed to add deployment log"
        );
    }
}

/// Mark a database as stopped in the database
pub(super) async fn mark_database_stopped(db: &DbPool, database_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE databases SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
    )
    .bind(database_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Mark a database as failed in the database
pub(super) async fn mark_database_failed(
    db: &DbPool,
    database_id: &str,
    error: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE databases SET status = 'failed', error_message = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(error)
    .bind(database_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Mark a service as stopped in the database
pub(super) async fn mark_service_stopped(db: &DbPool, service_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE services SET status = 'stopped', updated_at = datetime('now') WHERE id = ?",
    )
    .bind(service_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Reconcile deployment container status on startup
pub(super) async fn reconcile_deployments(
    db: &DbPool,
    runtime: &Arc<dyn ContainerRuntime>,
) -> usize {
    // Reconcile both 'running' and 'starting' states — containers can be left in
    // 'starting' if Rivetr restarts mid-deployment or right after container creation.
    let deployments: Vec<Deployment> = match sqlx::query_as(
        "SELECT * FROM deployments WHERE status IN ('running', 'starting') AND container_id IS NOT NULL",
    )
    .fetch_all(db)
    .await
    {
        Ok(deployments) => deployments,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch deployments for reconciliation");
            return 0;
        }
    };

    let mut updated = 0;

    for deployment in deployments {
        let container_id = match &deployment.container_id {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        let is_running = match runtime.inspect(container_id).await {
            Ok(info) => info.running,
            Err(_) => false,
        };

        if is_running && deployment.status == "starting" {
            // Container is running but DB says starting — fix it
            if let Err(e) = sqlx::query("UPDATE deployments SET status = 'running' WHERE id = ?")
                .bind(&deployment.id)
                .execute(db)
                .await
            {
                tracing::warn!(deployment = %deployment.id, error = %e, "Failed to update deployment status during reconciliation");
            } else {
                tracing::info!(deployment = %deployment.id, container = %container_id, "Deployment status reconciled: starting -> running");
                updated += 1;
            }
        } else if !is_running {
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
                    "Deployment status reconciled: {} -> stopped", deployment.status
                );
                updated += 1;
            }
        }
    }

    updated
}

/// Reconcile database container status on startup
pub(super) async fn reconcile_databases(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) -> usize {
    let running_databases: Vec<ManagedDatabase> = match sqlx::query_as(
        "SELECT * FROM databases WHERE status IN ('running', 'starting') AND container_id IS NOT NULL",
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

        let is_running = match runtime.inspect(container_id).await {
            Ok(info) => info.running,
            Err(_) => false,
        };

        if is_running && database.status == "starting" {
            if let Err(e) = sqlx::query(
                "UPDATE databases SET status = 'running', updated_at = datetime('now') WHERE id = ?"
            )
            .bind(&database.id)
            .execute(db)
            .await
            {
                tracing::warn!(database = %database.id, error = %e, "Failed to update database status during reconciliation");
            } else {
                tracing::info!(database = %database.name, container = %container_id, "Database status reconciled: starting -> running");
                updated += 1;
            }
        } else if !is_running {
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
                    "Database status reconciled: {} -> stopped", database.status
                );
                updated += 1;
            }
        }
    }

    updated
}

/// Reconcile Docker Compose service status on startup
pub(super) async fn reconcile_services(db: &DbPool, runtime: &Arc<dyn ContainerRuntime>) -> usize {
    use super::stats::check_compose_running;

    let running_services: Vec<Service> = match sqlx::query_as(
        "SELECT * FROM services WHERE status = 'running'",
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

        let is_running = check_compose_running(&project_name, runtime).await;

        if !is_running {
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
}
