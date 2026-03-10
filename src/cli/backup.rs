//! Backup and restore subcommand handlers for the Rivetr CLI.
//!
//! Handles:
//! - `backup` — Create a backup of the Rivetr instance
//! - `restore <backup_file>` — Restore from a backup file

use anyhow::{Context, Result};

use super::Cli;
use crate::backup;
use crate::config::Config;

/// Create a backup of the Rivetr instance
pub async fn cmd_backup(cli: &Cli, output: Option<&std::path::Path>) -> Result<()> {
    use sqlx::sqlite::SqlitePoolOptions;

    let config_path = &cli.config;
    let config = Config::load(config_path)?;

    // Connect to database directly
    let db_path = config.server.data_dir.join("rivetr.db");
    if !db_path.exists() {
        anyhow::bail!(
            "Database not found at {}. Is Rivetr initialized?",
            db_path.display()
        );
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    println!();
    println!("=== Rivetr Instance Backup ===");
    println!();
    println!("Data directory: {}", config.server.data_dir.display());
    println!("Config file:    {}", config_path.display());
    println!("ACME/SSL dir:   {}", config.proxy.acme_cache_dir.display());
    println!();

    let result = backup::create_backup(
        &pool,
        &config.server.data_dir,
        config_path,
        &config.proxy.acme_cache_dir,
        output,
    )
    .await
    .context("Failed to create backup")?;

    println!("[OK] Backup created successfully!");
    println!();
    println!("  File: {}", result.path.display());
    println!("  Size: {}", super::deploy::format_bytes(result.size));
    println!();

    Ok(())
}

/// Restore from a backup file
pub async fn cmd_restore(cli: &Cli, backup_file: &std::path::Path) -> Result<()> {
    if !backup_file.exists() {
        anyhow::bail!("Backup file not found: {}", backup_file.display());
    }

    let config_path = &cli.config;
    let config = Config::load(config_path)?;

    println!();
    println!("=== Rivetr Instance Restore ===");
    println!();
    println!("Backup file:    {}", backup_file.display());
    println!("Data directory: {}", config.server.data_dir.display());
    println!("Config file:    {}", config_path.display());
    println!();
    println!("[WARNING] This will replace the current database and configuration.");
    println!("          Make sure the Rivetr server is stopped before restoring.");
    println!();

    // Read the backup file
    let backup_data = std::fs::read(backup_file).context("Failed to read backup file")?;

    let result = backup::restore_from_backup(
        &backup_data,
        &config.server.data_dir,
        config_path,
        &config.proxy.acme_cache_dir,
    )
    .await
    .context("Failed to restore backup")?;

    println!("[OK] Restore completed!");
    println!();
    println!(
        "  Database restored: {}",
        if result.database_restored {
            "Yes"
        } else {
            "No"
        }
    );
    println!(
        "  Config restored:   {}",
        if result.config_restored { "Yes" } else { "No" }
    );
    println!(
        "  SSL certs restored: {}",
        if result.certs_restored { "Yes" } else { "No" }
    );

    if !result.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &result.warnings {
            println!("  [!] {}", warning);
        }
    }

    println!();
    println!("Please restart the Rivetr server to apply the restored data.");
    println!();

    Ok(())
}
