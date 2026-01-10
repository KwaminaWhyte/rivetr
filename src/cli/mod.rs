//! CLI module for Rivetr command-line interface.
//!
//! Provides subcommands for interacting with a running Rivetr server:
//! - `status` - Show server health, version, and uptime
//! - `apps list` - List all applications
//! - `deploy <app>` - Trigger deployment for an app
//! - `logs <app>` - Stream application logs
//! - `config check` - Validate configuration file

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
// CLI Command Handlers
// ============================================================================

/// Create an HTTP client with the given token
fn create_client(token: Option<&str>) -> Result<Client> {
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

/// Run a CLI command
pub async fn run_command(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Status) => cmd_status(cli).await,
        Some(Commands::Apps(AppsCommands::List)) => cmd_apps_list(cli).await,
        Some(Commands::Apps(AppsCommands::Show { app })) => cmd_apps_show(cli, app).await,
        Some(Commands::Deploy { app }) => cmd_deploy(cli, app).await,
        Some(Commands::Logs { app, lines, follow }) => cmd_logs(cli, app, *lines, *follow).await,
        Some(Commands::Config(ConfigCommands::Check)) => cmd_config_check(cli).await,
        Some(Commands::Db(DbCommands::MigrateTeams { execute })) => {
            cmd_migrate_teams(cli, *execute).await
        }
        None => {
            // No subcommand means start the server - this is handled in main.rs
            Ok(())
        }
    }
}

/// Display server status
async fn cmd_status(cli: &Cli) -> Result<()> {
    let client = create_client(cli.token.as_deref())?;
    let base_url = &cli.api_url;

    // Fetch health status
    println!("Connecting to {}...", base_url);

    let health_url = format!("{}/api/system/health", base_url);
    let health_response = client
        .get(&health_url)
        .send()
        .await
        .context("Failed to connect to server. Is Rivetr running?")?;

    if !health_response.status().is_success() {
        let status = health_response.status();
        let body = health_response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned error {}: {}", status, body);
    }

    let health: SystemHealthStatus = health_response
        .json()
        .await
        .context("Failed to parse health response")?;

    // Fetch stats
    let stats_url = format!("{}/api/system/stats", base_url);
    let stats: Option<SystemStats> = match client.get(&stats_url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.ok(),
        _ => None,
    };

    // Display status
    println!();
    println!("=== Rivetr Server Status ===");
    println!();

    // Version and health
    let health_icon = if health.healthy { "[OK]" } else { "[!!]" };
    println!("Version:    v{}", health.version);
    println!(
        "Status:     {} {}",
        health_icon,
        if health.healthy {
            "Healthy"
        } else {
            "Unhealthy"
        }
    );

    // Component health
    println!();
    println!("Components:");
    print_component("Database", health.database_healthy);
    print_component("Container Runtime", health.runtime_healthy);
    print_component("Disk Space", health.disk_healthy);

    // Stats if available
    if let Some(stats) = stats {
        println!();
        println!("Resources:");
        println!(
            "  Apps:       {}/{} running",
            stats.running_apps_count, stats.total_apps_count
        );
        println!(
            "  Databases:  {}/{} running",
            stats.running_databases_count, stats.total_databases_count
        );
        println!(
            "  Services:   {}/{} running",
            stats.running_services_count, stats.total_services_count
        );
        println!();
        println!("  CPU:        {:.1}%", stats.total_cpu_percent);
        println!(
            "  Memory:     {} / {} ({:.1}%)",
            format_bytes(stats.memory_used_bytes),
            format_bytes(stats.memory_total_bytes),
            (stats.memory_used_bytes as f64 / stats.memory_total_bytes as f64) * 100.0
        );
        println!("  Uptime:     {}", format_duration(stats.uptime_seconds));
    }

    // Individual checks if not all passed
    if !health.checks.iter().all(|c| c.passed) {
        println!();
        println!("Check Details:");
        for check in &health.checks {
            if !check.passed {
                let severity = if check.critical {
                    "CRITICAL"
                } else {
                    "WARNING"
                };
                println!("  [{}] {}: {}", severity, check.name, check.message);
                if let Some(details) = &check.details {
                    println!("           {}", details);
                }
            }
        }
    }

    println!();
    Ok(())
}

fn print_component(name: &str, healthy: bool) {
    let icon = if healthy { "[OK]" } else { "[!!]" };
    let status = if healthy { "OK" } else { "FAILED" };
    println!("  {} {:18} {}", icon, name, status);
}

/// List all applications
async fn cmd_apps_list(cli: &Cli) -> Result<()> {
    let client = create_client(cli.token.as_deref())?;
    let base_url = &cli.api_url;

    let url = format!("{}/api/apps", base_url);
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to server")?;

    if !response.status().is_success() {
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!(
                "Authentication required. Use --token or set RIVETR_TOKEN environment variable."
            );
        }
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned error {}: {}", status, body);
    }

    let apps: Vec<App> = response
        .json()
        .await
        .context("Failed to parse apps response")?;

    if apps.is_empty() {
        println!("No applications found.");
        return Ok(());
    }

    // Print header
    println!();
    println!(
        "{:<36}  {:<20}  {:<12}  {:<30}  {:<10}",
        "ID", "NAME", "ENVIRONMENT", "DOMAIN", "PORT"
    );
    println!("{}", "-".repeat(120));

    // Print apps
    for app in apps {
        let domain = app.domain.as_deref().unwrap_or("-");
        println!(
            "{:<36}  {:<20}  {:<12}  {:<30}  {:<10}",
            app.id,
            truncate(&app.name, 20),
            app.environment,
            truncate(domain, 30),
            app.port
        );
    }

    println!();
    Ok(())
}

/// Show details for a specific app
async fn cmd_apps_show(cli: &Cli, app_identifier: &str) -> Result<()> {
    let client = create_client(cli.token.as_deref())?;
    let base_url = &cli.api_url;

    // Try to find app by name or ID
    let app = find_app(&client, base_url, app_identifier).await?;

    println!();
    println!("=== Application: {} ===", app.name);
    println!();
    println!("ID:          {}", app.id);
    println!("Name:        {}", app.name);
    println!("Environment: {}", app.environment);
    println!("Domain:      {}", app.domain.as_deref().unwrap_or("-"));
    println!("Port:        {}", app.port);
    println!("Branch:      {}", app.branch);

    if !app.git_url.is_empty() {
        println!("Git URL:     {}", app.git_url);
    }
    if let Some(image) = &app.docker_image {
        println!("Docker Image: {}", image);
    }

    println!("Created:     {}", app.created_at);
    println!("Updated:     {}", app.updated_at);
    println!();

    Ok(())
}

/// Trigger deployment for an app
async fn cmd_deploy(cli: &Cli, app_identifier: &str) -> Result<()> {
    let client = create_client(cli.token.as_deref())?;
    let base_url = &cli.api_url;

    // Find app by name or ID
    let app = find_app(&client, base_url, app_identifier).await?;

    println!("Triggering deployment for app: {}", app.name);

    let url = format!("{}/api/apps/{}/deploy", base_url, app.id);
    let response = client
        .post(&url)
        .send()
        .await
        .context("Failed to trigger deployment")?;

    if !response.status().is_success() {
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!(
                "Authentication required. Use --token or set RIVETR_TOKEN environment variable."
            );
        }
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to trigger deployment: {} - {}", status, body);
    }

    let deployment: DeploymentResponse = response
        .json()
        .await
        .context("Failed to parse deployment response")?;

    println!();
    println!("[OK] Deployment triggered successfully!");
    println!();
    println!("Deployment ID: {}", deployment.id);
    println!("Status:        {}", deployment.status);
    println!("Started:       {}", deployment.started_at);
    println!();
    println!(
        "Use 'rivetr logs {}' to view deployment progress.",
        app.name
    );
    println!();

    Ok(())
}

/// Stream logs for an app
async fn cmd_logs(cli: &Cli, app_identifier: &str, _lines: u32, follow: bool) -> Result<()> {
    let client = create_client(cli.token.as_deref())?;
    let base_url = &cli.api_url;

    // Find app by name or ID
    let app = find_app(&client, base_url, app_identifier).await?;

    if follow {
        // Stream logs via SSE
        println!(
            "Streaming logs for app: {} (press Ctrl+C to stop)",
            app.name
        );
        println!();

        let url = format!("{}/api/apps/{}/logs/stream", base_url, app.id);

        // Use reqwest with eventsource-client for SSE
        let response = client
            .get(&url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .context("Failed to connect to log stream")?;

        if !response.status().is_success() {
            let status = response.status();
            if status == reqwest::StatusCode::NOT_FOUND {
                anyhow::bail!("No running container found for this app. Is the app deployed?");
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                anyhow::bail!("Authentication required. Use --token or set RIVETR_TOKEN environment variable.");
            }
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to stream logs: {} - {}", status, body);
        }

        // Read the SSE stream
        use futures::StreamExt;

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    // Process complete events
                    while let Some(idx) = buffer.find("\n\n") {
                        let event_str = buffer[..idx].to_string();
                        buffer = buffer[idx + 2..].to_string();

                        // Parse SSE event
                        for line in event_str.lines() {
                            if let Some(data) = line.strip_prefix("data:") {
                                let data = data.trim();
                                if data == "keep-alive" {
                                    continue;
                                }

                                if let Ok(event) = serde_json::from_str::<LogEvent>(data) {
                                    match event.event_type.as_str() {
                                        "connected" => {
                                            if let Some(cid) = event.container_id {
                                                println!(
                                                    "--- Connected to container {} ---",
                                                    &cid[..12.min(cid.len())]
                                                );
                                            }
                                        }
                                        "log" => {
                                            if let Some(msg) = event.message {
                                                if let Some(ts) = event.timestamp {
                                                    // Parse and format timestamp
                                                    let short_ts = ts.get(11..19).unwrap_or(&ts);
                                                    println!("{} | {}", short_ts, msg);
                                                } else {
                                                    println!("{}", msg);
                                                }
                                            }
                                        }
                                        "end" => {
                                            println!();
                                            println!("--- Log stream ended ---");
                                            return Ok(());
                                        }
                                        "error" => {
                                            if let Some(msg) = event.message {
                                                eprintln!("Error: {}", msg);
                                            }
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Stream error: {}", e);
                    break;
                }
            }
        }
    } else {
        // Non-follow mode: just indicate how to use follow mode
        println!(
            "Use 'rivetr logs {} --follow' to stream live logs.",
            app.name
        );
        println!();
        println!("Alternatively, view logs in the Rivetr dashboard.");
    }

    Ok(())
}

/// Validate configuration file
async fn cmd_config_check(cli: &Cli) -> Result<()> {
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

/// Migrate unassigned resources to their owner's first team
async fn cmd_migrate_teams(cli: &Cli, execute: bool) -> Result<()> {
    use crate::config::Config;
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
    println!("=== Rivetr Team Migration Tool ===");
    println!();

    if !execute {
        println!("[DRY RUN] Showing what would be migrated. Use --execute to perform migration.");
        println!();
    }

    // Track migration statistics
    let mut apps_migrated = 0;
    let mut projects_migrated = 0;
    let mut databases_migrated = 0;
    let mut services_migrated = 0;
    let mut users_without_team = 0;

    // Get all users with unassigned apps
    let unassigned_apps: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT a.id, a.name, u.id as user_id
        FROM apps a
        JOIN users u ON a.created_by = u.id
        WHERE a.team_id IS NULL
        ORDER BY u.id, a.name
        "#,
    )
    .fetch_all(&pool)
    .await?;

    // Get all users with unassigned projects
    let unassigned_projects: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT p.id, p.name, p.created_by as user_id
        FROM projects p
        WHERE p.team_id IS NULL AND p.created_by IS NOT NULL
        ORDER BY p.created_by, p.name
        "#,
    )
    .fetch_all(&pool)
    .await?;

    // Get all users with unassigned databases
    let unassigned_databases: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT d.id, d.name, d.created_by as user_id
        FROM databases d
        WHERE d.team_id IS NULL AND d.created_by IS NOT NULL
        ORDER BY d.created_by, d.name
        "#,
    )
    .fetch_all(&pool)
    .await?;

    // Get all users with unassigned services
    let unassigned_services: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT s.id, s.name, s.created_by as user_id
        FROM services s
        WHERE s.team_id IS NULL AND s.created_by IS NOT NULL
        ORDER BY s.created_by, s.name
        "#,
    )
    .fetch_all(&pool)
    .await?;

    // Collect all unique users with unassigned resources
    let mut user_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_, _, user_id) in &unassigned_apps {
        user_ids.insert(user_id.clone());
    }
    for (_, _, user_id) in &unassigned_projects {
        user_ids.insert(user_id.clone());
    }
    for (_, _, user_id) in &unassigned_databases {
        user_ids.insert(user_id.clone());
    }
    for (_, _, user_id) in &unassigned_services {
        user_ids.insert(user_id.clone());
    }

    // For each user, find their first team and migrate resources
    for user_id in &user_ids {
        // Find user's first team (by membership created_at date)
        let first_team: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT t.id, t.name
            FROM team_members tm
            JOIN teams t ON tm.team_id = t.id
            WHERE tm.user_id = ?
            ORDER BY tm.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&pool)
        .await?;

        // Get user info for display
        let user_info: Option<(String, String)> =
            sqlx::query_as("SELECT name, email FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&pool)
                .await?;

        let user_display = match user_info {
            Some((name, email)) => format!("{} ({})", name, email),
            None => user_id.clone(),
        };

        if let Some((team_id, team_name)) = first_team {
            println!("User: {}", user_display);
            println!("  Target team: {} ({})", team_name, team_id);

            // Migrate apps
            let user_apps: Vec<&(String, String, String)> = unassigned_apps
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .collect();

            if !user_apps.is_empty() {
                println!("  Apps to migrate: {}", user_apps.len());
                for (app_id, app_name, _) in &user_apps {
                    println!("    - {} ({})", app_name, app_id);
                    if execute {
                        sqlx::query("UPDATE apps SET team_id = ? WHERE id = ?")
                            .bind(&team_id)
                            .bind(app_id)
                            .execute(&pool)
                            .await?;
                    }
                    apps_migrated += 1;
                }
            }

            // Migrate projects
            let user_projects: Vec<&(String, String, String)> = unassigned_projects
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .collect();

            if !user_projects.is_empty() {
                println!("  Projects to migrate: {}", user_projects.len());
                for (project_id, project_name, _) in &user_projects {
                    println!("    - {} ({})", project_name, project_id);
                    if execute {
                        sqlx::query("UPDATE projects SET team_id = ? WHERE id = ?")
                            .bind(&team_id)
                            .bind(project_id)
                            .execute(&pool)
                            .await?;
                    }
                    projects_migrated += 1;
                }
            }

            // Migrate databases
            let user_databases: Vec<&(String, String, String)> = unassigned_databases
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .collect();

            if !user_databases.is_empty() {
                println!("  Databases to migrate: {}", user_databases.len());
                for (db_id, db_name, _) in &user_databases {
                    println!("    - {} ({})", db_name, db_id);
                    if execute {
                        sqlx::query("UPDATE databases SET team_id = ? WHERE id = ?")
                            .bind(&team_id)
                            .bind(db_id)
                            .execute(&pool)
                            .await?;
                    }
                    databases_migrated += 1;
                }
            }

            // Migrate services
            let user_services: Vec<&(String, String, String)> = unassigned_services
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .collect();

            if !user_services.is_empty() {
                println!("  Services to migrate: {}", user_services.len());
                for (service_id, service_name, _) in &user_services {
                    println!("    - {} ({})", service_name, service_id);
                    if execute {
                        sqlx::query("UPDATE services SET team_id = ? WHERE id = ?")
                            .bind(&team_id)
                            .bind(service_id)
                            .execute(&pool)
                            .await?;
                    }
                    services_migrated += 1;
                }
            }

            println!();
        } else {
            println!("User: {}", user_display);
            println!("  [!] No team membership found - resources will remain unassigned");

            // Count resources that won't be migrated
            let user_apps_count = unassigned_apps
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .count();
            let user_projects_count = unassigned_projects
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .count();
            let user_databases_count = unassigned_databases
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .count();
            let user_services_count = unassigned_services
                .iter()
                .filter(|(_, _, uid)| uid == user_id)
                .count();

            println!(
                "  Unassignable: {} apps, {} projects, {} databases, {} services",
                user_apps_count, user_projects_count, user_databases_count, user_services_count
            );
            println!();

            users_without_team += 1;
        }
    }

    // Print summary
    println!("=== Migration Summary ===");
    println!();

    if execute {
        println!("[EXECUTED] Migration completed:");
    } else {
        println!("[DRY RUN] Would migrate:");
    }

    println!("  Apps:      {}", apps_migrated);
    println!("  Projects:  {}", projects_migrated);
    println!("  Databases: {}", databases_migrated);
    println!("  Services:  {}", services_migrated);
    println!();

    if users_without_team > 0 {
        println!(
            "[WARNING] {} user(s) have no team membership.",
            users_without_team
        );
        println!("          Their resources cannot be automatically migrated.");
        println!("          Create a team for these users first, then run migration again.");
        println!();
    }

    if !execute && (apps_migrated + projects_migrated + databases_migrated + services_migrated) > 0
    {
        println!("To perform the migration, run:");
        println!("  rivetr db migrate-teams --execute");
        println!();
    }

    if apps_migrated + projects_migrated + databases_migrated + services_migrated == 0
        && users_without_team == 0
    {
        println!("No unassigned resources found. Nothing to migrate.");
        println!();
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find an app by name or ID
async fn find_app(client: &Client, base_url: &str, identifier: &str) -> Result<App> {
    // First try by ID (if it looks like a UUID)
    if identifier.len() == 36 && identifier.contains('-') {
        let url = format!("{}/api/apps/{}", base_url, identifier);
        let response = client.get(&url).send().await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                if let Ok(app) = resp.json::<App>().await {
                    return Ok(app);
                }
            } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                anyhow::bail!("Authentication required. Use --token or set RIVETR_TOKEN environment variable.");
            }
        }
    }

    // Try to find by name in the list
    let url = format!("{}/api/apps", base_url);
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch apps list")?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        anyhow::bail!(
            "Authentication required. Use --token or set RIVETR_TOKEN environment variable."
        );
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to fetch apps: {} - {}", status, body);
    }

    let apps: Vec<App> = response
        .json()
        .await
        .context("Failed to parse apps response")?;

    // Find by name (case-insensitive)
    let identifier_lower = identifier.to_lowercase();
    for app in apps {
        if app.name.to_lowercase() == identifier_lower || app.id == identifier {
            return Ok(app);
        }
    }

    anyhow::bail!("App not found: {}", identifier);
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration to human-readable string
fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Truncate a string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
