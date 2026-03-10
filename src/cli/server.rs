//! Server configuration subcommand handlers for the Rivetr CLI.
//!
//! Handles:
//! - `config check` — Validate the configuration file

use anyhow::Result;

use super::Cli;

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
