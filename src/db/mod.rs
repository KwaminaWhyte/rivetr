mod models;
mod seeders;

pub use models::*;
pub use seeders::seed_service_templates;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub type DbPool = SqlitePool;

/// Execute a SQL migration file, properly handling comments
async fn execute_sql(pool: &SqlitePool, sql: &str) -> Result<()> {
    for statement in sql.split(';') {
        // Strip SQL comment lines (lines starting with --)
        let cleaned: String = statement
            .lines()
            .filter(|line| !line.trim().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let trimmed = cleaned.trim();
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

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

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
        "SELECT name FROM pragma_table_info('deployments') WHERE name = 'image_tag'"
    )
    .fetch_optional(pool)
    .await?;
    if has_image_tag.is_none() {
        execute_sql(pool, include_str!("../../migrations/003_rollback.sql")).await?;
    }

    // Migration 004: Add SSH keys table for private repository authentication
    let has_ssh_keys_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='ssh_keys'"
    )
    .fetch_optional(pool)
    .await?;
    if has_ssh_keys_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/004_ssh_keys.sql")).await?;
    }

    // Migration 005: Add git_providers table for OAuth connections
    let has_git_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='git_providers'"
    )
    .fetch_optional(pool)
    .await?;
    if has_git_providers_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/005_git_providers.sql")).await?;
    }

    // Migration 006: Add environment field to apps
    let has_environment: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'environment'"
    )
    .fetch_optional(pool)
    .await?;
    if has_environment.is_none() {
        execute_sql(pool, include_str!("../../migrations/006_environment.sql")).await?;
    }

    // Migration 007: Add projects table and project_id to apps
    let has_projects_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='projects'"
    )
    .fetch_optional(pool)
    .await?;
    if has_projects_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/007_projects.sql")).await?;
    }

    // Migration 008: Add is_secret and updated_at columns to env_vars
    let has_is_secret: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('env_vars') WHERE name = 'is_secret'"
    )
    .fetch_optional(pool)
    .await?;
    if has_is_secret.is_none() {
        execute_sql(pool, include_str!("../../migrations/008_env_vars_update.sql")).await?;
    }

    // Migration 009: Add advanced build options to apps
    let has_dockerfile_path: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'dockerfile_path'"
    )
    .fetch_optional(pool)
    .await?;
    if has_dockerfile_path.is_none() {
        execute_sql(pool, include_str!("../../migrations/009_build_options.sql")).await?;
    }

    // Migration 010: Add domain management to apps
    let has_domains: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'domains'"
    )
    .fetch_optional(pool)
    .await?;
    if has_domains.is_none() {
        execute_sql(pool, include_str!("../../migrations/010_domains.sql")).await?;
    }

    // Migration 011: Add network configuration to apps
    let has_port_mappings: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'port_mappings'"
    )
    .fetch_optional(pool)
    .await?;
    if has_port_mappings.is_none() {
        execute_sql(pool, include_str!("../../migrations/011_network_config.sql")).await?;
    }

    // Migration 012: Add HTTP basic auth to apps
    let has_basic_auth: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'basic_auth_enabled'"
    )
    .fetch_optional(pool)
    .await?;
    if has_basic_auth.is_none() {
        execute_sql(pool, include_str!("../../migrations/012_basic_auth.sql")).await?;
    }

    // Migration 013: Add pre/post deployment commands to apps
    let has_pre_deploy: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'pre_deploy_commands'"
    )
    .fetch_optional(pool)
    .await?;
    if has_pre_deploy.is_none() {
        execute_sql(pool, include_str!("../../migrations/013_deployment_commands.sql")).await?;
    }

    // Migration 014: Add docker registry support
    let has_docker_image: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'docker_image'"
    )
    .fetch_optional(pool)
    .await?;
    if has_docker_image.is_none() {
        execute_sql(pool, include_str!("../../migrations/014_docker_registry.sql")).await?;
    }

    // Migration 015: Add teams and team_members tables for multi-user support
    let has_teams_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='teams'"
    )
    .fetch_optional(pool)
    .await?;
    if has_teams_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/015_teams.sql")).await?;
    }

    // Migration 016: Add notification channels and subscriptions
    let has_notification_channels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='notification_channels'"
    )
    .fetch_optional(pool)
    .await?;
    if has_notification_channels.is_none() {
        execute_sql(pool, include_str!("../../migrations/016_notifications.sql")).await?;
    }

    // Migration 017: Add container_labels to apps
    let has_container_labels: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'container_labels'"
    )
    .fetch_optional(pool)
    .await?;
    if has_container_labels.is_none() {
        execute_sql(pool, include_str!("../../migrations/017_container_labels.sql")).await?;
    }

    // Migration 018: Add volumes table for persistent storage
    let has_volumes_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='volumes'"
    )
    .fetch_optional(pool)
    .await?;
    if has_volumes_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/018_volumes.sql")).await?;
    }

    // Migration 019: Add databases table for managed database deployments
    let has_databases_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='databases'"
    )
    .fetch_optional(pool)
    .await?;
    if has_databases_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/019_databases.sql")).await?;
    }

    // Migration 020: Add project_id to databases table
    let has_db_project_id: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('databases') WHERE name = 'project_id'"
    )
    .fetch_optional(pool)
    .await?;
    if has_db_project_id.is_none() {
        execute_sql(pool, include_str!("../../migrations/020_databases_project.sql")).await?;
    }

    // Migration 021: Add database backups and backup schedules tables
    let has_database_backups_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='database_backups'"
    )
    .fetch_optional(pool)
    .await?;
    if has_database_backups_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/021_database_backups.sql")).await?;
    }

    // Migration 022: Add services table for Docker Compose services
    let has_services_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='services'"
    )
    .fetch_optional(pool)
    .await?;
    if has_services_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/022_services.sql")).await?;
    }

    // Migration 023: Add service_templates table
    let has_service_templates_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='service_templates'"
    )
    .fetch_optional(pool)
    .await?;
    if has_service_templates_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/023_service_templates.sql")).await?;
    }

    // Migration 024: Add audit_logs table for tracking user actions
    let has_audit_logs_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='audit_logs'"
    )
    .fetch_optional(pool)
    .await?;
    if has_audit_logs_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/024_audit_logs.sql")).await?;
    }

    // Migration 025: Add stats_history table for dashboard charts
    let has_stats_history_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='stats_history'"
    )
    .fetch_optional(pool)
    .await?;
    if has_stats_history_table.is_none() {
        execute_sql(pool, include_str!("../../migrations/025_stats_history.sql")).await?;
    }

    // Seed/update built-in templates (runs on every startup to add new templates)
    seeders::seed_service_templates(pool).await?;

    info!("Migrations completed");
    Ok(())
}
