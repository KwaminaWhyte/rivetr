//! Uptime monitoring background task.
//!
//! Runs every 60 seconds. For each app that has a health_check URL (the `healthcheck`
//! column in the `apps` table), sends an HTTP GET and records the result in the
//! `uptime_checks` table. Also cleans up old uptime data (>30 days by default).

use crate::DbPool;
use chrono::Utc;
use std::time::Duration;
use tokio::time::interval;

/// Spawn the uptime checker background task
pub fn spawn_uptime_checker_task(db: DbPool) {
    tracing::info!("Starting uptime checker (60s interval)");

    tokio::spawn(async move {
        // Wait a bit before the first check
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut tick = interval(Duration::from_secs(60));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        loop {
            tick.tick().await;
            uptime_cycle(&db, &client).await;
        }
    });
}

/// One cycle of uptime checks
async fn uptime_cycle(db: &DbPool, client: &reqwest::Client) {
    // Get all apps with a healthcheck URL
    let apps: Vec<(String, String, Option<String>)> = match sqlx::query_as(
        r#"
        SELECT id, name, healthcheck
        FROM apps
        WHERE healthcheck IS NOT NULL AND healthcheck != ''
        "#,
    )
    .fetch_all(db)
    .await
    {
        Ok(apps) => apps,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to fetch apps for uptime checks");
            return;
        }
    };

    if apps.is_empty() {
        return;
    }

    tracing::debug!(count = apps.len(), "Running uptime checks");

    for (app_id, app_name, healthcheck_url) in apps {
        let healthcheck_url = match healthcheck_url {
            Some(url) if !url.is_empty() => url,
            _ => continue,
        };

        // Determine the actual URL to check
        // If it starts with http, use as-is; otherwise, look up the running container's port
        let url =
            if healthcheck_url.starts_with("http://") || healthcheck_url.starts_with("https://") {
                healthcheck_url.clone()
            } else {
                // Build URL from custom domain, falling back to auto_subdomain (e.g. app.rivetr.site)
                let domain_row: Option<(Option<String>, Option<String>)> =
                    sqlx::query_as("SELECT domain, auto_subdomain FROM apps WHERE id = ?")
                        .bind(&app_id)
                        .fetch_optional(db)
                        .await
                        .unwrap_or(None);

                let resolved_domain = domain_row.and_then(|(domain, auto_subdomain)| {
                    if let Some(d) = domain {
                        if !d.is_empty() {
                            return Some(d);
                        }
                    }
                    auto_subdomain.filter(|d| !d.is_empty())
                });

                match resolved_domain {
                    Some(d) => {
                        let path = if healthcheck_url.starts_with('/') {
                            healthcheck_url.clone()
                        } else {
                            format!("/{}", healthcheck_url)
                        };
                        format!("https://{}{}", d, path)
                    }
                    None => {
                        // Can't construct a URL without a domain; skip this app
                        continue;
                    }
                }
            };

        let db = db.clone();
        let client = client.clone();
        let app_id = app_id.clone();

        tokio::spawn(async move {
            check_app_uptime(&db, &client, &app_id, &app_name, &url).await;
        });
    }

    // Cleanup old checks (keep 30 days)
    let cutoff = Utc::now() - chrono::Duration::days(30);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    if let Err(e) = sqlx::query("DELETE FROM uptime_checks WHERE checked_at < ?")
        .bind(&cutoff_str)
        .execute(db)
        .await
    {
        tracing::warn!(error = %e, "Failed to cleanup old uptime checks");
    }
}

/// Check uptime for a single app
async fn check_app_uptime(
    db: &DbPool,
    client: &reqwest::Client,
    app_id: &str,
    _app_name: &str,
    url: &str,
) {
    let start = std::time::Instant::now();

    let result = client.get(url).send().await;
    let elapsed_ms = start.elapsed().as_millis() as i32;

    let (status, status_code, error_message) = match result {
        Ok(response) => {
            let code = response.status().as_u16() as i32;
            if response.status().is_success() {
                if elapsed_ms > 5000 {
                    ("degraded".to_string(), Some(code), None)
                } else {
                    ("up".to_string(), Some(code), None)
                }
            } else {
                (
                    "down".to_string(),
                    Some(code),
                    Some(format!("HTTP {}", code)),
                )
            }
        }
        Err(e) => ("down".to_string(), None, Some(e.to_string())),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let checked_at = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO uptime_checks (id, app_id, status, response_time_ms, status_code, error_message, checked_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(app_id)
    .bind(&status)
    .bind(elapsed_ms)
    .bind(status_code)
    .bind(&error_message)
    .bind(&checked_at)
    .execute(db)
    .await
    {
        tracing::warn!(
            app_id = %app_id,
            error = %e,
            "Failed to insert uptime check record"
        );
    }
}
