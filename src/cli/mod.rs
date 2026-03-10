//! CLI module for Rivetr command-line interface.
//!
//! Provides subcommands for interacting with a running Rivetr server:
//! - `status` - Show server health, version, and uptime
//! - `apps list` - List all applications
//! - `deploy <app>` - Trigger deployment for an app
//! - `logs <app>` - Stream application logs
//! - `config check` - Validate configuration file

pub mod backup;
pub mod database;
pub mod deploy;
pub mod server;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

/// CLI arguments structure
#[derive(Parser, Debug)]
#[command(name = "rivetr")]
#[command(author, version, about = "A fast, lightweight deployment engine", long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "rivetr.toml")]
    pub config: PathBuf,

    /// Override log level
    #[arg(short, long)]
    pub log_level: Option<String>,

    /// Skip startup self-checks (for development only)
    #[arg(long)]
    pub skip_checks: bool,

    /// API URL to connect to (default: http://localhost:8080)
    #[arg(long, env = "RIVETR_API_URL", default_value = "http://localhost:8080")]
    pub api_url: String,

    /// Authentication token (can also be set via RIVETR_TOKEN env var)
    #[arg(long, env = "RIVETR_TOKEN")]
    pub token: Option<String>,

    /// Subcommand to run (if none, starts the server)
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available CLI subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show server status (health, version, uptime)
    Status,

    /// Application management commands
    #[command(subcommand)]
    Apps(AppsCommands),

    /// Trigger deployment for an application
    Deploy {
        /// App name or ID
        app: String,
    },

    /// Stream logs for an application
    Logs {
        /// App name or ID
        app: String,
        /// Number of lines to show (default: 100)
        #[arg(short = 'n', long, default_value = "100")]
        lines: u32,
        /// Follow log output (stream new logs as they arrive)
        #[arg(short, long)]
        follow: bool,
    },

    /// Configuration management commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Database management commands
    #[command(subcommand)]
    Db(DbCommands),

    /// Create a backup of the Rivetr instance
    Backup {
        /// Output path for the backup file (default: data/backups/)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Restore from a backup file
    Restore {
        /// Path to the backup .tar.gz file
        backup_file: PathBuf,
    },
}

/// Apps subcommands
#[derive(Subcommand, Debug)]
pub enum AppsCommands {
    /// List all applications
    List,
    /// Show details for a specific app
    Show {
        /// App name or ID
        app: String,
    },
}

/// Config subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Validate configuration file
    Check,
}

/// Database subcommands
#[derive(Subcommand, Debug)]
pub enum DbCommands {
    /// Migrate unassigned resources to their owner's first team
    MigrateTeams {
        /// Perform the migration (without this flag, just shows what would be migrated)
        #[arg(long)]
        execute: bool,
    },
}

// ============================================================================
// API Response Types
// ============================================================================

/// System health status from /api/system/health
#[derive(Debug, Deserialize)]
pub struct SystemHealthStatus {
    pub healthy: bool,
    pub database_healthy: bool,
    pub runtime_healthy: bool,
    pub disk_healthy: bool,
    pub version: String,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub critical: bool,
    pub message: String,
    pub details: Option<String>,
}

/// System stats from /api/system/stats
#[derive(Debug, Deserialize)]
pub struct SystemStats {
    pub running_apps_count: u32,
    pub total_apps_count: u32,
    pub running_databases_count: u32,
    pub total_databases_count: u32,
    pub running_services_count: u32,
    pub total_services_count: u32,
    pub total_cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub uptime_seconds: u64,
    pub uptime_percent: f64,
}

/// App from /api/apps
#[derive(Debug, Deserialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub domain: Option<String>,
    pub port: i32,
    pub environment: String,
    pub created_at: String,
    pub updated_at: String,
    pub docker_image: Option<String>,
}

/// Deployment response
#[derive(Debug, Deserialize)]
pub struct DeploymentResponse {
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub started_at: String,
}

/// SSE log event
#[derive(Debug, Deserialize)]
pub struct LogEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: Option<String>,
    pub message: Option<String>,
    pub container_id: Option<String>,
    pub stream: Option<String>,
}

/// API error response
#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub details: Option<String>,
}

// ============================================================================
// Shared HTTP Client Helper
// ============================================================================

/// Create an HTTP client with the given token
pub fn create_client(token: Option<&str>) -> Result<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(token) = token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", token)
                .parse()
                .context("Invalid token format")?,
        );
    }

    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")
}

// ============================================================================
// CLI Dispatcher
// ============================================================================

/// Run a CLI command
pub async fn run_command(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Status) => deploy::cmd_status(cli).await,
        Some(Commands::Apps(AppsCommands::List)) => deploy::cmd_apps_list(cli).await,
        Some(Commands::Apps(AppsCommands::Show { app })) => deploy::cmd_apps_show(cli, app).await,
        Some(Commands::Deploy { app }) => deploy::cmd_deploy(cli, app).await,
        Some(Commands::Logs { app, lines, follow }) => {
            deploy::cmd_logs(cli, app, *lines, *follow).await
        }
        Some(Commands::Config(ConfigCommands::Check)) => server::cmd_config_check(cli).await,
        Some(Commands::Db(DbCommands::MigrateTeams { execute })) => {
            database::cmd_migrate_teams(cli, *execute).await
        }
        Some(Commands::Backup { output }) => backup::cmd_backup(cli, output.as_deref()).await,
        Some(Commands::Restore { backup_file }) => backup::cmd_restore(cli, backup_file).await,
        None => {
            // No subcommand means start the server - this is handled in main.rs
            Ok(())
        }
    }
}
