//! Deploy subcommand handlers for the Rivetr CLI.
//!
//! Handles:
//! - `deploy <app>` — Trigger deployment for an application
//! - `logs <app>` — Stream application logs
//! - `apps list` / `apps show` — Application listing and details
//! - Helper utilities: find_app, format_bytes, format_duration, truncate

use anyhow::{Context, Result};
use reqwest::Client;

use super::{App, Cli, DeploymentResponse, LogEvent, SystemHealthStatus, SystemStats};

// ============================================================================
// Status
// ============================================================================

/// Display server status
pub async fn cmd_status(cli: &Cli) -> Result<()> {
    let client = super::create_client(cli.token.as_deref())?;
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

// ============================================================================
// Apps
// ============================================================================

/// List all applications
pub async fn cmd_apps_list(cli: &Cli) -> Result<()> {
    let client = super::create_client(cli.token.as_deref())?;
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
pub async fn cmd_apps_show(cli: &Cli, app_identifier: &str) -> Result<()> {
    let client = super::create_client(cli.token.as_deref())?;
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

// ============================================================================
// Deploy
// ============================================================================

/// Trigger deployment for an app
pub async fn cmd_deploy(cli: &Cli, app_identifier: &str) -> Result<()> {
    let client = super::create_client(cli.token.as_deref())?;
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

// ============================================================================
// Logs
// ============================================================================

/// Stream logs for an app
pub async fn cmd_logs(cli: &Cli, app_identifier: &str, _lines: u32, follow: bool) -> Result<()> {
    let client = super::create_client(cli.token.as_deref())?;
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
                                                    let short_ts =
                                                        ts.get(11..19).unwrap_or(&ts);
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

// ============================================================================
// Shared Helpers
// ============================================================================

/// Find an app by name or ID
pub async fn find_app(client: &Client, base_url: &str, identifier: &str) -> Result<App> {
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
pub fn format_bytes(bytes: u64) -> String {
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
pub fn format_duration(seconds: u64) -> String {
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
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
