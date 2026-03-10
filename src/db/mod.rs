mod models;
mod seeders;

pub use models::*;
pub use seeders::seed_service_templates;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub type DbPool = SqlitePool;

/// Execute a SQL migration file, properly handling comments.
///
/// Strips `--` comment lines first, then splits on `;` to run individual statements.
/// This avoids the bug where a `;` inside a `--` comment would produce invalid SQL fragments.
async fn execute_sql(pool: &SqlitePool, sql: &str) -> Result<()> {
    // First pass: strip all comment lines (lines whose non-whitespace content starts with --)
    let stripped: String = sql
        .lines()
        .filter(|line| !line.trim().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");

    // Second pass: split on `;` and run each non-empty statement
    for statement in stripped.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed).execute(pool).await?;
        }
    }
    Ok(())
}

pub async fn init(data_dir: &Path) -> Result<DbPool> {
    let db_path = data_dir.join("rivetr.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    info!("Initializing database at {}", db_path.display());

    // Use a temporary single-connection pool for migrations to avoid
    // stale prepared-statement caches after ALTER TABLE migrations.
    {
        let migration_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await?;

        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&migration_pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&migration_pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&migration_pool)
            .await?;

        run_migrations(&migration_pool).await?;

        // Checkpoint the WAL and close the migration pool so the new pool
        // opens fresh connections with the fully-updated schema.
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&migration_pool)
            .await;
        migration_pool.close().await;
    }

    // Open the production pool with fresh connections (no stale statement cache).
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    info!("Database initialized successfully");
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running database migrations...");

    // Migration 001: Initial schema
    execute_sql(pool, include_str!("../../migrations/001_initial.sql")).await?;

    // Migration 002: Users table
    execute_sql(pool, include_str!("../../migrations/002_users.sql")).await?;

    // Migration 003: Add image_tag column for rollback support
    let has_image_tag: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('deployments') WHERE name = 'image_tag'",
    )
    .fetch_optional(pool)
    .await?;
    if has_image_tag.is_none() {
        execute_sql(pool, include_str!("../../migrations/003_rollback.sql")).await?;
    }

    // Migration 004: Add SSH keys table for private repository authentication
    let has_ssh_keys_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='ssh_keys'")
            .fetch_optional(pool)
            .await?;
    if has_ssh_keys_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/004_ssh_keys.sql")).await?;
    }

    // Migration 005: Add git_providers table for OAuth connections
    let has_git_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='git_providers'",
    )
    .fetch_optional(pool)
    .await?;
    if has_git_providers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/005_git_providers.sql")).await?;
    }

    // Migration 006: Add environment field to apps
    let has_environment: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'environment'")
            .fetch_optional(pool)
            .await?;
    if has_environment.is_none() {
        execute_sql(pool, include_str!("../../migrations/006_environment.sql")).await?;
    }

    // Migration 007: Add projects table and project_id to apps
    let has_projects_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='projects'")
            .fetch_optional(pool)
            .await?;
    if has_projects_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/007_projects.sql")).await?;
    }

    // Migration 008: Add is_secret and updated_at columns to env_vars
    let has_is_secret: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('env_vars') WHERE name = 'is_secret'")
            .fetch_optional(pool)
            .await?;
    if has_is_secret.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/008_env_vars_update.sql"),
        )
        .await?;
    }

    // Migration 009: Add advanced build options to apps
    let has_dockerfile_path: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'dockerfile_path'")
            .fetch_optional(pool)
            .await?;
    if has_dockerfile_path.is_none() {
        execute_sql(pool, include_str!("../../migrations/009_build_options.sql")).await?;
    }

    // Migration 010: Add domain management to apps
    let has_domains: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'domains'")
            .fetch_optional(pool)
            .await?;
    if has_domains.is_none() {
        execute_sql(pool, include_str!("../../migrations/010_domains.sql")).await?;
    }

    // Migration 011: Add network configuration to apps
    let has_port_mappings: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'port_mappings'")
            .fetch_optional(pool)
            .await?;
    if has_port_mappings.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/011_network_config.sql"),
        )
        .await?;
    }

    // Migration 012: Add HTTP basic auth to apps
    let has_basic_auth: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'basic_auth_enabled'",
    )
    .fetch_optional(pool)
    .await?;
    if has_basic_auth.is_none() {
        execute_sql(pool, include_str!("../../migrations/012_basic_auth.sql")).await?;
    }

    // Migration 013: Add pre/post deployment commands to apps
    let has_pre_deploy: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'pre_deploy_commands'",
    )
    .fetch_optional(pool)
    .await?;
    if has_pre_deploy.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/013_deployment_commands.sql"),
        )
        .await?;
    }

    // Migration 014: Add docker registry support
    let has_docker_image: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'docker_image'")
            .fetch_optional(pool)
            .await?;
    if has_docker_image.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/014_docker_registry.sql"),
        )
        .await?;
    }

    // Migration 015: Add teams and team_members tables for multi-user support
    let has_teams_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='teams'")
            .fetch_optional(pool)
            .await?;
    if has_teams_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/015_teams.sql")).await?;
    }

    // Migration 016: Add notification channels and subscriptions
    let has_notification_channels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='notification_channels'",
    )
    .fetch_optional(pool)
    .await?;
    if has_notification_channels.is_none() {
        execute_sql(pool, include_str!("../../migrations/016_notifications.sql")).await?;
    }

    // Migration 017: Add container_labels to apps
    let has_container_labels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'container_labels'",
    )
    .fetch_optional(pool)
    .await?;
    if has_container_labels.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/017_container_labels.sql"),
        )
        .await?;
    }

    // Migration 018: Add volumes table for persistent storage
    let has_volumes_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='volumes'")
            .fetch_optional(pool)
            .await?;
    if has_volumes_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/018_volumes.sql")).await?;
    }

    // Migration 019: Add databases table for managed database deployments
    let has_databases_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='databases'")
            .fetch_optional(pool)
            .await?;
    if has_databases_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/019_databases.sql")).await?;
    }

    // Migration 020: Add project_id to databases table
    let has_db_project_id: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('databases') WHERE name = 'project_id'")
            .fetch_optional(pool)
            .await?;
    if has_db_project_id.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/020_databases_project.sql"),
        )
        .await?;
    }

    // Migration 021: Add database backups and backup schedules tables
    let has_database_backups_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='database_backups'",
    )
    .fetch_optional(pool)
    .await?;
    if has_database_backups_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/021_database_backups.sql"),
        )
        .await?;
    }

    // Migration 022: Add services table for Docker Compose services
    let has_services_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='services'")
            .fetch_optional(pool)
            .await?;
    if has_services_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/022_services.sql")).await?;
    }

    // Migration 023: Add service_templates table
    let has_service_templates_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='service_templates'",
    )
    .fetch_optional(pool)
    .await?;
    if has_service_templates_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/023_service_templates.sql"),
        )
        .await?;
    }

    // Migration 024: Add audit_logs table for tracking user actions
    let has_audit_logs_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='audit_logs'")
            .fetch_optional(pool)
            .await?;
    if has_audit_logs_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/024_audit_logs.sql")).await?;
    }

    // Migration 025: Add stats_history table for dashboard charts
    let has_stats_history_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='stats_history'",
    )
    .fetch_optional(pool)
    .await?;
    if has_stats_history_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/025_stats_history.sql")).await?;
    }

    // Migration 026: Add build_type for Nixpacks support
    let has_build_type: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'build_type'")
            .fetch_optional(pool)
            .await?;
    if has_build_type.is_none() {
        execute_sql(pool, include_str!("../../migrations/026_build_type.sql")).await?;
    }

    // Migration 027: Add preview_deployments table for PR preview environments
    let has_preview_deployments_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='preview_deployments'",
    )
    .fetch_optional(pool)
    .await?;
    if has_preview_deployments_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/027_preview_deployments.sql"),
        )
        .await?;
    }

    // Migration 028: Add GitHub Apps tables for system-wide app registration
    let has_github_apps_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='github_apps'")
            .fetch_optional(pool)
            .await?;
    if has_github_apps_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/028_github_apps.sql")).await?;
    }

    // Migration 029: Add deployment_source field to apps
    let has_deployment_source: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'deployment_source'",
    )
    .fetch_optional(pool)
    .await?;
    if has_deployment_source.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/029_deployment_source.sql"),
        )
        .await?;
    }

    // Migration 030: Add automatic rollback settings to apps
    let has_auto_rollback_enabled: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'auto_rollback_enabled'",
    )
    .fetch_optional(pool)
    .await?;
    if has_auto_rollback_enabled.is_none() {
        execute_sql(pool, include_str!("../../migrations/030_auto_rollback.sql")).await?;
    }

    // Migration 031: Add stats aggregation tables (hourly and daily)
    let has_stats_hourly_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='stats_hourly'")
            .fetch_optional(pool)
            .await?;
    if has_stats_hourly_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/031_stats_aggregation.sql"),
        )
        .await?;
    }

    // Migration 032a: Add team_id to databases table
    let has_databases_team_id: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('databases') WHERE name = 'team_id'")
            .fetch_optional(pool)
            .await?;
    if has_databases_team_id.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/032_databases_team.sql"),
        )
        .await?;
    }

    // Migration 032b: Add resource_metrics table for per-app resource monitoring
    let has_resource_metrics_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='resource_metrics'",
    )
    .fetch_optional(pool)
    .await?;
    if has_resource_metrics_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/032_resource_metrics.sql"),
        )
        .await?;
    }

    // Migration 033a: Add team_id to services table
    let has_services_team_id: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('services') WHERE name = 'team_id'")
            .fetch_optional(pool)
            .await?;
    if has_services_team_id.is_none() {
        execute_sql(pool, include_str!("../../migrations/033_services_team.sql")).await?;
    }

    // Migration 033b: Add alert_configs and global_alert_defaults tables
    let has_alert_configs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='alert_configs'",
    )
    .fetch_optional(pool)
    .await?;
    if has_alert_configs_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/033_alert_configs.sql")).await?;
    }

    // Migration 034a: Add team_invitations table for email-based invitations
    let has_team_invitations_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='team_invitations'",
    )
    .fetch_optional(pool)
    .await?;
    if has_team_invitations_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/034_team_invitations.sql"),
        )
        .await?;
    }

    // Migration 034b: Add alert_events and alert_breach_counts tables
    let has_alert_events_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='alert_events'")
            .fetch_optional(pool)
            .await?;
    if has_alert_events_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/034_alert_events.sql")).await?;
    }

    // Migration 035a: Add team_audit_logs table for tracking team activities
    let has_team_audit_logs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='team_audit_logs'",
    )
    .fetch_optional(pool)
    .await?;
    if has_team_audit_logs_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/035_team_audit_logs.sql"),
        )
        .await?;
    }

    // Migration 035b: Add team_id to notification_channels for team-scoped notifications
    let has_team_id_in_notification_channels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('notification_channels') WHERE name = 'team_id'",
    )
    .fetch_optional(pool)
    .await?;
    if has_team_id_in_notification_channels.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/035_team_notification_channels.sql"),
        )
        .await?;
    }

    // Migration 036a: Add app_shares table for sharing apps between teams
    let has_app_shares_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='app_shares'")
            .fetch_optional(pool)
            .await?;
    if has_app_shares_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/036_app_shares.sql")).await?;
    }

    // Migration 036b: Add cost_rates table for resource cost estimation
    let has_cost_rates_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='cost_rates'")
            .fetch_optional(pool)
            .await?;
    if has_cost_rates_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/036_cost_rates.sql")).await?;
    }

    // Migration 037: Add cost_snapshots table for daily cost storage
    let has_cost_snapshots_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='cost_snapshots'",
    )
    .fetch_optional(pool)
    .await?;
    if has_cost_snapshots_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/037_cost_snapshots.sql"),
        )
        .await?;
    }

    // Migration 038: Add 'webhook' to notification_channels channel_type CHECK constraint
    let check_has_webhook: Option<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='notification_channels' AND sql LIKE '%webhook%'",
    )
    .fetch_optional(pool)
    .await?;
    if check_has_webhook.is_none() {
        // Disable foreign keys temporarily for table recreation
        sqlx::query("PRAGMA foreign_keys=OFF").execute(pool).await?;
        execute_sql(
            pool,
            include_str!("../../migrations/038_notification_webhook_type.sql"),
        )
        .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(pool).await?;
    }

    // Migration 039: Add 'telegram' and 'teams' to notification_channels channel_type CHECK constraint
    let check_has_telegram: Option<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='notification_channels' AND sql LIKE '%telegram%'",
    )
    .fetch_optional(pool)
    .await?;
    if check_has_telegram.is_none() {
        // Disable foreign keys temporarily for table recreation
        sqlx::query("PRAGMA foreign_keys=OFF").execute(pool).await?;
        execute_sql(
            pool,
            include_str!("../../migrations/039_notification_telegram_teams.sql"),
        )
        .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(pool).await?;
    }

    // Migration 040: Add OAuth login providers and user OAuth connections
    let has_oauth_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='oauth_providers'",
    )
    .fetch_optional(pool)
    .await?;
    if has_oauth_providers_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/040_oauth_providers.sql"),
        )
        .await?;
    }

    // Migration 041: Add 'pushover' and 'ntfy' to notification_channels channel_type CHECK constraint
    let check_has_pushover: Option<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='notification_channels' AND sql LIKE '%pushover%'",
    )
    .fetch_optional(pool)
    .await?;
    if check_has_pushover.is_none() {
        // Disable foreign keys temporarily for table recreation
        sqlx::query("PRAGMA foreign_keys=OFF").execute(pool).await?;
        execute_sql(
            pool,
            include_str!("../../migrations/041_notification_pushover_ntfy.sql"),
        )
        .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(pool).await?;
    }

    // Migration 042: Add project environments and environment-scoped env vars
    let has_environments_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='environments'")
            .fetch_optional(pool)
            .await?;
    if has_environments_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/042_project_environments.sql"),
        )
        .await?;
    }

    // Migration 043: Add two-factor authentication columns to users table
    let has_totp_enabled: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('users') WHERE name = 'totp_enabled'")
            .fetch_optional(pool)
            .await?;
    if has_totp_enabled.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/043_two_factor_auth.sql"),
        )
        .await?;
    }

    // Migration 044: Add scheduled_jobs and scheduled_job_runs tables
    let has_scheduled_jobs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='scheduled_jobs'",
    )
    .fetch_optional(pool)
    .await?;
    if has_scheduled_jobs_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/044_scheduled_jobs.sql"),
        )
        .await?;
    }

    // Migration 045: Add git_tag column to deployments for deploy-by-commit/tag
    let has_git_tag: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('deployments') WHERE name = 'git_tag'")
            .fetch_optional(pool)
            .await?;
    if has_git_tag.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/045_deploy_by_commit_tag.sql"),
        )
        .await?;
    }

    // Migration 046: Add s3_storage_configs and s3_backups tables
    let has_s3_storage_configs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='s3_storage_configs'",
    )
    .fetch_optional(pool)
    .await?;
    if has_s3_storage_configs_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/046_s3_storage.sql")).await?;
    }

    // Migration 047: Add advanced monitoring tables (log retention, uptime, scheduled restarts)
    let has_log_retention_policies_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='log_retention_policies'",
    )
    .fetch_optional(pool)
    .await?;
    if has_log_retention_policies_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/047_advanced_monitoring.sql"),
        )
        .await?;
    }

    // Migration 048: Add log_drains table for forwarding container logs
    let has_log_drains_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='log_drains'")
            .fetch_optional(pool)
            .await?;
    if has_log_drains_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/048_log_drains.sql")).await?;
    }

    // Migration 049: Add deployment enhancements (approval workflow, maintenance mode, freeze windows)
    let has_approval_status: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('deployments') WHERE name = 'approval_status'",
    )
    .fetch_optional(pool)
    .await?;
    if has_approval_status.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/049_deployment_enhancements.sql"),
        )
        .await?;
    }

    // Migration 050: Add config_snapshots table and maintenance mode columns
    let has_config_snapshots_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='config_snapshots'",
    )
    .fetch_optional(pool)
    .await?;
    if has_config_snapshots_table.is_none() {
        execute_sql(
            pool,
            r#"CREATE TABLE IF NOT EXISTS config_snapshots (
              id TEXT PRIMARY KEY,
              app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
              name TEXT NOT NULL,
              description TEXT,
              config_json TEXT NOT NULL,
              env_vars_json TEXT NOT NULL,
              created_by TEXT REFERENCES users(id),
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"#,
        )
        .await?;
    }

    // Add maintenance_mode column to apps if missing
    let has_maintenance_mode: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'maintenance_mode'",
    )
    .fetch_optional(pool)
    .await?;
    if has_maintenance_mode.is_none() {
        execute_sql(
            pool,
            "ALTER TABLE apps ADD COLUMN maintenance_mode INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        execute_sql(pool, "ALTER TABLE apps ADD COLUMN maintenance_message TEXT").await?;
    }

    // Migration 051: Add shared environment variables (team, project, environment level)
    let has_team_env_vars_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='team_env_vars'",
    )
    .fetch_optional(pool)
    .await?;
    if has_team_env_vars_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/051_shared_env_vars.sql")).await?;
    }

    // Migration 052: Add multi-server support (servers and app_server_assignments tables)
    let has_servers_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='servers'")
            .fetch_optional(pool)
            .await?;
    if has_servers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/052_multi_server.sql")).await?;
    }

    // Migration 053: Add OIDC/SSO provider support
    let has_oidc_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='oidc_providers'",
    )
    .fetch_optional(pool)
    .await?;
    if has_oidc_providers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/053_sso_oidc.sql")).await?;
    }

    // Add oidc_subject and oidc_provider_id columns to users if missing
    let has_oidc_subject: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('users') WHERE name = 'oidc_subject'")
            .fetch_optional(pool)
            .await?;
    if has_oidc_subject.is_none() {
        execute_sql(pool, "ALTER TABLE users ADD COLUMN oidc_subject TEXT").await?;
        execute_sql(pool, "ALTER TABLE users ADD COLUMN oidc_provider_id TEXT REFERENCES oidc_providers(id) ON DELETE SET NULL").await?;
    }

    // Migration 054: Add container replicas (replica_count on apps, app_replicas table)
    let has_replica_count: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'replica_count'")
            .fetch_optional(pool)
            .await?;
    if has_replica_count.is_none() {
        execute_sql(pool, include_str!("../../migrations/054_container_replicas.sql")).await?;
    }

    // Migration 055: Add backup_schedules table for scheduled backups
    let has_backup_schedules_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='backup_schedules'",
    )
    .fetch_optional(pool)
    .await?;
    if has_backup_schedules_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/055_scheduled_backups.sql")).await?;
    }

    // Migration 056: Add require_2fa column to teams for 2FA enforcement
    let has_require_2fa: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('teams') WHERE name = 'require_2fa'")
            .fetch_optional(pool)
            .await?;
    if has_require_2fa.is_none() {
        execute_sql(pool, include_str!("../../migrations/056_2fa_enforcement.sql")).await?;
    }

    // Migration 057: Add service_dependencies table for dependency graph visualization
    let has_service_dependencies_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='service_dependencies'",
    )
    .fetch_optional(pool)
    .await?;
    if has_service_dependencies_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/057_service_dependencies.sql"),
        )
        .await?;
    }

    // Migration 058: Add server_id column to apps for preferred-server deployments
    let has_apps_server_id: Option<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('apps') WHERE name = 'server_id'")
            .fetch_optional(pool)
            .await?;
    if has_apps_server_id.is_none() {
        execute_sql(pool, include_str!("../../migrations/058_server_deploy.sql")).await?;
    }

    // Migration 059: Add Docker Swarm tables (swarm_nodes, swarm_services, swarm_config)
    let has_swarm_nodes_table: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' AND name='swarm_nodes'")
            .fetch_optional(pool)
            .await?;
    if has_swarm_nodes_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/059_docker_swarm.sql")).await?;
    }

    // Migration 060: Add build_servers table for remote build servers
    let has_build_servers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='build_servers'",
    )
    .fetch_optional(pool)
    .await?;
    if has_build_servers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/060_build_servers.sql")).await?;
    }

    // Migration 061: Registry push pipeline - no-op, columns already added in earlier migrations

    // Migration 062: Add rollback_retention_count to apps
    let has_rollback_retention_count: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'rollback_retention_count'",
    )
    .fetch_optional(pool)
    .await?;
    if has_rollback_retention_count.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/062_rollback_retention.sql"),
        )
        .await?;
    }

    // Migration 063: Add template_suggestions table
    let has_template_suggestions_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='template_suggestions'",
    )
    .fetch_optional(pool)
    .await?;
    if has_template_suggestions_table.is_none() {
        execute_sql(
            pool,
            include_str!("../../migrations/063_template_suggestions.sql"),
        )
        .await?;
    }

    // Migration 064: Add autoscaling_rules table
    let has_autoscaling_rules_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='autoscaling_rules'",
    )
    .fetch_optional(pool)
    .await?;
    if has_autoscaling_rules_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/064_autoscaling.sql")).await?;
    }

    // Migration 065: Add webhook_events table for audit logging
    let has_webhook_events_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='webhook_events'",
    )
    .fetch_optional(pool)
    .await?;
    if has_webhook_events_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/065_webhook_audit.sql")).await?;
    }

    // Seed/update built-in templates (runs on every startup to add new templates)
    seeders::seed_service_templates(pool).await?;

    info!("Migrations completed");
    Ok(())
}
