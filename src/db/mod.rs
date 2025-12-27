mod models;

pub use models::*;

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::Path;
use tracing::info;

pub type DbPool = SqlitePool;

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

    let migration_001 = include_str!("../../migrations/001_initial.sql");
    sqlx::query(migration_001).execute(pool).await?;

    let migration_002 = include_str!("../../migrations/002_users.sql");
    sqlx::query(migration_002).execute(pool).await?;

    // Migration 003: Add image_tag column for rollback support
    // Using a check to avoid "duplicate column" error on existing databases
    let has_image_tag: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('deployments') WHERE name = 'image_tag'"
    )
    .fetch_optional(pool)
    .await?;

    if has_image_tag.is_none() {
        let migration_003 = include_str!("../../migrations/003_rollback.sql");
        // Execute each statement separately since SQLite doesn't support multiple statements
        for statement in migration_003.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 004: Add SSH keys table for private repository authentication
    let has_ssh_keys_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='ssh_keys'"
    )
    .fetch_optional(pool)
    .await?;

    if has_ssh_keys_table.is_none() {
        let migration_004 = include_str!("../../migrations/004_ssh_keys.sql");
        // Execute each statement separately since SQLite doesn't support multiple statements
        for statement in migration_004.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 005: Add git_providers table for OAuth connections
    let has_git_providers_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='git_providers'"
    )
    .fetch_optional(pool)
    .await?;

    if has_git_providers_table.is_none() {
        let migration_005 = include_str!("../../migrations/005_git_providers.sql");
        // Execute each statement separately since SQLite doesn't support multiple statements
        for statement in migration_005.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 006: Add environment field to apps
    let has_environment: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'environment'"
    )
    .fetch_optional(pool)
    .await?;

    if has_environment.is_none() {
        let migration_006 = include_str!("../../migrations/006_environment.sql");
        for statement in migration_006.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 007: Add projects table and project_id to apps
    let has_projects_table: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='projects'"
    )
    .fetch_optional(pool)
    .await?;

    if has_projects_table.is_none() {
        let migration_007 = include_str!("../../migrations/007_projects.sql");
        // Execute each statement separately since SQLite doesn't support multiple statements
        for statement in migration_007.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 008: Add is_secret and updated_at columns to env_vars
    let has_is_secret: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('env_vars') WHERE name = 'is_secret'"
    )
    .fetch_optional(pool)
    .await?;

    if has_is_secret.is_none() {
        let migration_008 = include_str!("../../migrations/008_env_vars_update.sql");
        for statement in migration_008.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 009: Add advanced build options to apps
    let has_dockerfile_path: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'dockerfile_path'"
    )
    .fetch_optional(pool)
    .await?;

    if has_dockerfile_path.is_none() {
        let migration_009 = include_str!("../../migrations/009_build_options.sql");
        for statement in migration_009.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 010: Add domain management to apps
    let has_domains: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'domains'"
    )
    .fetch_optional(pool)
    .await?;

    if has_domains.is_none() {
        let migration_010 = include_str!("../../migrations/010_domains.sql");
        for statement in migration_010.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 011: Add network configuration to apps
    let has_port_mappings: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'port_mappings'"
    )
    .fetch_optional(pool)
    .await?;

    if has_port_mappings.is_none() {
        let migration_011 = include_str!("../../migrations/011_network_config.sql");
        for statement in migration_011.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 012: Add HTTP basic auth to apps
    let has_basic_auth: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'basic_auth_enabled'"
    )
    .fetch_optional(pool)
    .await?;

    if has_basic_auth.is_none() {
        let migration_012 = include_str!("../../migrations/012_basic_auth.sql");
        for statement in migration_012.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    // Migration 013: Add pre/post deployment commands to apps
    let has_pre_deploy: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pragma_table_info('apps') WHERE name = 'pre_deploy_commands'"
    )
    .fetch_optional(pool)
    .await?;

    if has_pre_deploy.is_none() {
        let migration_013 = include_str!("../../migrations/013_deployment_commands.sql");
        for statement in migration_013.split(';') {
            let trimmed = statement.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::query(trimmed).execute(pool).await?;
            }
        }
    }

    info!("Migrations completed");
    Ok(())
}
