//! Server configuration subcommand handlers for the Rivetr CLI.
//!
//! Handles:
//! - `config check` — Validate the configuration file

use anyhow::{Context, Result};

use super::Cli;

/// Reset a user's password directly in the local database (offline recovery).
pub async fn cmd_reset_password(cli: &Cli, email: &str, password: Option<&str>) -> Result<()> {
    use crate::api::auth::hash_password;
    use crate::config::Config;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::io::Write;

    let config = Config::load(&cli.config)?;

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

    let email_norm = email.trim().to_lowercase();
    let user: Option<(String, String)> =
        sqlx::query_as("SELECT id, name FROM users WHERE email = ?")
            .bind(&email_norm)
            .fetch_optional(&pool)
            .await?;

    let Some((user_id, name)) = user else {
        anyhow::bail!("No user found with email '{}'", email_norm);
    };

    let new_password = match password {
        Some(p) => p.to_string(),
        None => {
            print!("New password for {} ({}): ", name, email_norm);
            std::io::stdout().flush()?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            line.trim_end_matches(['\n', '\r']).to_string()
        }
    };

    if new_password.len() < 12 {
        anyhow::bail!("Password must be at least 12 characters");
    }

    let hash = hash_password(&new_password)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    sqlx::query("UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&hash)
        .bind(&user_id)
        .execute(&pool)
        .await?;

    // Revoke all of this user's sessions so old tokens can't be reused.
    let revoked = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(&user_id)
        .execute(&pool)
        .await?;

    println!();
    println!(
        "Password reset for {} ({}). Revoked {} active session(s).",
        name,
        email_norm,
        revoked.rows_affected()
    );

    Ok(())
}

/// Validate configuration file
pub async fn cmd_config_check(cli: &Cli) -> Result<()> {
    use crate::config::Config;

    let config_path = &cli.config;

    println!("Checking configuration file: {}", config_path.display());
    println!();

    if !config_path.exists() {
        println!(
            "[!!] Configuration file not found: {}",
            config_path.display()
        );
        println!();
        println!("A default configuration will be used when starting the server.");
        println!("To create a custom configuration, copy rivetr.example.toml to rivetr.toml");
        return Ok(());
    }

    // Try to load the configuration
    match Config::load(config_path) {
        Ok(config) => {
            println!("[OK] Configuration file is valid!");
            println!();
            println!("=== Configuration Summary ===");
            println!();
            println!("Server:");
            println!("  Host:         {}", config.server.host);
            println!("  API Port:     {}", config.server.api_port);
            println!("  Proxy Port:   {}", config.server.proxy_port);
            println!("  Data Dir:     {}", config.server.data_dir.display());
            println!();
            println!("Runtime:");
            println!("  Type:         {:?}", config.runtime.runtime_type);
            println!("  Docker Socket: {}", config.runtime.docker_socket);
            println!("  Build CPU:    {}", config.runtime.build_cpu_limit);
            println!("  Build Memory: {}", config.runtime.build_memory_limit);
            println!();
            println!("Security:");
            println!(
                "  Rate Limiting: {}",
                if config.rate_limit.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            println!(
                "  Encryption:   {}",
                if config.auth.encryption_key.is_some() {
                    "Enabled"
                } else {
                    "Disabled (env vars stored in plaintext)"
                }
            );
            println!();
            println!("Features:");
            println!(
                "  ACME (SSL):   {}",
                if config.proxy.acme_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            println!(
                "  Cleanup:      {}",
                if config.cleanup.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            println!(
                "  Disk Monitor: {}",
                if config.disk_monitor.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            println!(
                "  Backups:      {}",
                if config.database_backup.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
            println!();

            // Warnings
            let mut warnings = Vec::new();

            if config.auth.encryption_key.is_none() {
                warnings.push(
                    "No encryption key set - environment variables will be stored in plaintext",
                );
            }

            if config.webhooks.github_secret.is_none()
                && config.webhooks.gitlab_token.is_none()
                && config.webhooks.gitea_secret.is_none()
                && config.webhooks.bitbucket_secret.is_none()
            {
                warnings
                    .push("No webhook secrets configured - webhooks will accept unsigned requests");
            }

            if !warnings.is_empty() {
                println!("Warnings:");
                for warning in warnings {
                    println!("  [!] {}", warning);
                }
                println!();
            }

            Ok(())
        }
        Err(e) => {
            println!("[!!] Configuration file is invalid!");
            println!();
            println!("Error: {}", e);
            println!();
            println!("Please check the configuration file syntax and try again.");
            anyhow::bail!("Invalid configuration file");
        }
    }
}
