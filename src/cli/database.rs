//! Database management subcommand handlers for the Rivetr CLI.
//!
//! Handles:
//! - `db migrate-teams` — Migrate unassigned resources to their owner's first team

use anyhow::{Context, Result};

use super::Cli;
use crate::config::Config;

/// Migrate unassigned resources to their owner's first team
pub async fn cmd_migrate_teams(cli: &Cli, execute: bool) -> Result<()> {
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
