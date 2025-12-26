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

    info!("Migrations completed");
    Ok(())
}
