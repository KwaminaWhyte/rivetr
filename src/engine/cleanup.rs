//! Deployment cleanup module
//!
//! This module handles automatic cleanup of old deployments to prevent
//! disk space exhaustion. It runs as a background task that periodically:
//! - Removes old deployment records from the database
//! - Stops and removes associated containers
//! - Optionally prunes unused container images

use crate::config::CleanupConfig;
use crate::db::Deployment;
use crate::runtime::ContainerRuntime;
use crate::DbPool;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Handles cleanup of old deployments
pub struct DeploymentCleanup {
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: CleanupConfig,
}

impl DeploymentCleanup {
    pub fn new(db: DbPool, runtime: Arc<dyn ContainerRuntime>, config: CleanupConfig) -> Self {
        Self { db, runtime, config }
    }

    /// Run a single cleanup cycle
    pub async fn run_cleanup(&self) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();

        if !self.config.enabled {
            tracing::debug!("Cleanup is disabled, skipping");
            return Ok(stats);
        }

        tracing::info!("Starting deployment cleanup cycle");

        // Get all apps
        let apps: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, name FROM apps"
        )
        .fetch_all(&self.db)
        .await?;

        for (app_id, app_name) in apps {
            match self.cleanup_app_deployments(&app_id, &app_name).await {
                Ok(app_stats) => {
                    stats.deployments_removed += app_stats.deployments_removed;
                    stats.containers_removed += app_stats.containers_removed;
                    stats.images_removed += app_stats.images_removed;
                }
                Err(e) => {
                    tracing::warn!(
                        app = %app_name,
                        error = %e,
                        "Failed to cleanup deployments for app"
                    );
                }
            }
        }

        // Prune unused images if enabled
        if self.config.prune_images {
            match self.runtime.prune_images().await {
                Ok(bytes_reclaimed) => {
                    stats.bytes_reclaimed = bytes_reclaimed;
                    if bytes_reclaimed > 0 {
                        tracing::info!(
                            bytes = bytes_reclaimed,
                            "Pruned unused images, reclaimed {} bytes",
                            format_bytes(bytes_reclaimed)
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to prune images");
                }
            }
        }

        tracing::info!(
            deployments = stats.deployments_removed,
            containers = stats.containers_removed,
            images = stats.images_removed,
            bytes_reclaimed = stats.bytes_reclaimed,
            "Cleanup cycle completed"
        );

        Ok(stats)
    }

    /// Cleanup old deployments for a specific app
    async fn cleanup_app_deployments(&self, app_id: &str, app_name: &str) -> Result<CleanupStats> {
        let mut stats = CleanupStats::default();
        let max_deployments = self.config.max_deployments_per_app as i64;

        // Get deployments ordered by started_at descending, skip the first N (most recent)
        // We only clean up deployments that are not "running" status
        let old_deployments: Vec<Deployment> = sqlx::query_as(
            r#"
            SELECT id, app_id, commit_sha, commit_message, status, container_id, image_tag, error_message, started_at, finished_at
            FROM deployments
            WHERE app_id = ?
              AND status NOT IN ('running', 'pending', 'cloning', 'building', 'starting', 'checking')
            ORDER BY started_at DESC
            LIMIT -1 OFFSET ?
            "#,
        )
        .bind(app_id)
        .bind(max_deployments)
        .fetch_all(&self.db)
        .await?;

        if old_deployments.is_empty() {
            return Ok(stats);
        }

        tracing::debug!(
            app = %app_name,
            count = old_deployments.len(),
            "Found old deployments to clean up"
        );

        for deployment in old_deployments {
            // Stop and remove container if it exists
            if let Some(container_id) = &deployment.container_id {
                if !container_id.is_empty() {
                    // Try to stop the container first (ignore errors if already stopped)
                    if let Err(e) = self.runtime.stop(container_id).await {
                        tracing::debug!(
                            container = %container_id,
                            error = %e,
                            "Container may already be stopped"
                        );
                    }

                    // Remove the container
                    match self.runtime.remove(container_id).await {
                        Ok(_) => {
                            stats.containers_removed += 1;
                            tracing::debug!(
                                container = %container_id,
                                deployment = %deployment.id,
                                "Removed container"
                            );
                        }
                        Err(e) => {
                            tracing::debug!(
                                container = %container_id,
                                error = %e,
                                "Failed to remove container (may not exist)"
                            );
                        }
                    }
                }
            }

            // Remove the image if it exists
            if let Some(image_tag) = &deployment.image_tag {
                if !image_tag.is_empty() {
                    match self.runtime.remove_image(image_tag).await {
                        Ok(_) => {
                            stats.images_removed += 1;
                            tracing::debug!(
                                image = %image_tag,
                                deployment = %deployment.id,
                                "Removed image"
                            );
                        }
                        Err(e) => {
                            tracing::debug!(
                                image = %image_tag,
                                error = %e,
                                "Failed to remove image (may be in use or not exist)"
                            );
                        }
                    }
                }
            }

            // Delete deployment logs first (foreign key constraint)
            sqlx::query("DELETE FROM deployment_logs WHERE deployment_id = ?")
                .bind(&deployment.id)
                .execute(&self.db)
                .await?;

            // Delete the deployment record
            sqlx::query("DELETE FROM deployments WHERE id = ?")
                .bind(&deployment.id)
                .execute(&self.db)
                .await?;

            stats.deployments_removed += 1;
            tracing::debug!(
                deployment = %deployment.id,
                app = %app_name,
                "Removed old deployment"
            );
        }

        Ok(stats)
    }
}

/// Statistics from a cleanup run
#[derive(Debug, Default)]
pub struct CleanupStats {
    pub deployments_removed: u64,
    pub containers_removed: u64,
    pub images_removed: u64,
    pub bytes_reclaimed: u64,
}

/// Spawn the background cleanup task
pub fn spawn_cleanup_task(
    db: DbPool,
    runtime: Arc<dyn ContainerRuntime>,
    config: CleanupConfig,
) {
    if !config.enabled {
        tracing::info!("Deployment cleanup is disabled");
        return;
    }

    let interval_secs = config.cleanup_interval_seconds;
    tracing::info!(
        interval_secs = interval_secs,
        max_deployments = config.max_deployments_per_app,
        prune_images = config.prune_images,
        "Starting deployment cleanup task"
    );

    let cleanup = DeploymentCleanup::new(db, runtime, config);

    tokio::spawn(async move {
        // Wait a bit before the first cleanup to let the system stabilize
        tokio::time::sleep(Duration::from_secs(60)).await;

        let mut tick = interval(Duration::from_secs(interval_secs));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tick.tick().await;
            if let Err(e) = cleanup.run_cleanup().await {
                tracing::error!(error = %e, "Cleanup cycle failed");
            }
        }
    });
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
}
