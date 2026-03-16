//! URL Redirect Rules API endpoints for applications.
//!
//! Provides CRUD operations for per-app proxy-level redirect rules.
//! Rules are evaluated in sort_order before requests are forwarded to the container.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{AppRedirectRule, CreateRedirectRuleRequest, UpdateRedirectRuleRequest};
use crate::proxy::RedirectRule;
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// List all redirect rules for an app.
pub async fn list_redirect_rules(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<AppRedirectRule>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Ensure app exists
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let rules = sqlx::query_as::<_, AppRedirectRule>(
        "SELECT * FROM app_redirect_rules WHERE app_id = ? ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rules))
}

/// Create a new redirect rule for an app.
pub async fn create_redirect_rule(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
    Json(req): Json<CreateRedirectRuleRequest>,
) -> Result<(StatusCode, Json<AppRedirectRule>), ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Validate regex compiles
    validate_regex(&req.source_pattern)?;

    // Ensure app exists
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("App not found"));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO app_redirect_rules
            (id, app_id, source_pattern, destination, is_permanent, is_enabled, sort_order, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&app_id)
    .bind(&req.source_pattern)
    .bind(&req.destination)
    .bind(req.is_permanent)
    .bind(req.is_enabled)
    .bind(req.sort_order)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create redirect rule: {}", e);
        ApiError::database("Failed to create redirect rule")
    })?;

    let rule =
        sqlx::query_as::<_, AppRedirectRule>("SELECT * FROM app_redirect_rules WHERE id = ?")
            .bind(&id)
            .fetch_one(&state.db)
            .await?;

    // Refresh proxy routes for this app
    refresh_proxy_routes(&state, &app_id).await;

    Ok((StatusCode::CREATED, Json(rule)))
}

/// Update an existing redirect rule.
pub async fn update_redirect_rule(
    State(state): State<Arc<AppState>>,
    Path((app_id, rule_id)): Path<(String, String)>,
    Json(req): Json<UpdateRedirectRuleRequest>,
) -> Result<Json<AppRedirectRule>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&rule_id, "rule_id") {
        return Err(ApiError::validation_field("rule_id", e));
    }

    // Fetch existing rule
    let existing = sqlx::query_as::<_, AppRedirectRule>(
        "SELECT * FROM app_redirect_rules WHERE id = ? AND app_id = ?",
    )
    .bind(&rule_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Redirect rule not found"))?;

    // Validate new regex if provided
    let source_pattern = match &req.source_pattern {
        Some(p) => {
            validate_regex(p)?;
            p.clone()
        }
        None => existing.source_pattern.clone(),
    };

    let destination = req.destination.clone().unwrap_or(existing.destination);
    let is_permanent = req.is_permanent.unwrap_or(existing.is_permanent != 0);
    let is_enabled = req.is_enabled.unwrap_or(existing.is_enabled != 0);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE app_redirect_rules SET
            source_pattern = ?,
            destination = ?,
            is_permanent = ?,
            is_enabled = ?,
            sort_order = ?,
            updated_at = ?
        WHERE id = ? AND app_id = ?
        "#,
    )
    .bind(&source_pattern)
    .bind(&destination)
    .bind(is_permanent)
    .bind(is_enabled)
    .bind(sort_order)
    .bind(&now)
    .bind(&rule_id)
    .bind(&app_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update redirect rule: {}", e);
        ApiError::database("Failed to update redirect rule")
    })?;

    let updated =
        sqlx::query_as::<_, AppRedirectRule>("SELECT * FROM app_redirect_rules WHERE id = ?")
            .bind(&rule_id)
            .fetch_one(&state.db)
            .await?;

    // Refresh proxy routes for this app
    refresh_proxy_routes(&state, &app_id).await;

    Ok(Json(updated))
}

/// Delete a redirect rule.
pub async fn delete_redirect_rule(
    State(state): State<Arc<AppState>>,
    Path((app_id, rule_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }
    if let Err(e) = validate_uuid(&rule_id, "rule_id") {
        return Err(ApiError::validation_field("rule_id", e));
    }

    let result = sqlx::query("DELETE FROM app_redirect_rules WHERE id = ? AND app_id = ?")
        .bind(&rule_id)
        .bind(&app_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Redirect rule not found"));
    }

    // Refresh proxy routes for this app
    refresh_proxy_routes(&state, &app_id).await;

    Ok(StatusCode::NO_CONTENT)
}

// ---- Helpers ----

/// Validate that a string is a valid regex pattern.
fn validate_regex(pattern: &str) -> Result<(), ApiError> {
    if pattern.is_empty() {
        return Err(ApiError::validation_field(
            "source_pattern",
            "Source pattern cannot be empty".to_string(),
        ));
    }
    Regex::new(pattern).map_err(|e| {
        ApiError::validation_field("source_pattern", format!("Invalid regex pattern: {}", e))
    })?;
    Ok(())
}

/// Reload the redirect rules for an app into its live proxy backend entries.
///
/// Fetches all enabled rules from DB and updates every domain route for the app.
async fn refresh_proxy_routes(state: &Arc<AppState>, app_id: &str) {
    // Load enabled redirect rules
    let rules: Vec<AppRedirectRule> = match sqlx::query_as(
        "SELECT * FROM app_redirect_rules WHERE app_id = ? AND is_enabled = 1 ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(app_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(app_id = %app_id, error = %e, "Failed to load redirect rules for proxy refresh");
            return;
        }
    };

    let proxy_rules: Vec<RedirectRule> = rules
        .into_iter()
        .map(|r| RedirectRule {
            source_pattern: r.source_pattern,
            destination: r.destination,
            is_permanent: r.is_permanent != 0,
        })
        .collect();

    // Fetch the app's domain info to find which routes to update
    let app_info: Option<(
        Option<String>, // domain
        Option<String>, // domains (JSON)
        Option<String>, // auto_subdomain
    )> = sqlx::query_as("SELECT domain, domains, auto_subdomain FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let (legacy_domain, domains_json, auto_subdomain) = match app_info {
        Some(t) => t,
        None => return,
    };

    // Collect all domain names for this app
    let mut domain_names: Vec<String> = Vec::new();

    if let Some(ref json) = domains_json {
        if let Ok(arr) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(list) = arr.as_array() {
                for entry in list {
                    if let Some(d) = entry.get("domain").and_then(|v| v.as_str()) {
                        if !d.is_empty() && !domain_names.contains(&d.to_string()) {
                            domain_names.push(d.to_string());
                        }
                    }
                }
            }
        }
    }
    if let Some(ref d) = legacy_domain {
        if !d.is_empty() && !domain_names.contains(d) {
            domain_names.push(d.clone());
        }
    }
    if let Some(ref d) = auto_subdomain {
        if !d.is_empty() && !domain_names.contains(d) {
            domain_names.push(d.clone());
        }
    }

    let route_table = state.routes.load();

    for domain in &domain_names {
        route_table.update_redirect_rules(domain, proxy_rules.clone());
    }

    tracing::info!(
        app_id = %app_id,
        domains = ?domain_names,
        rules_count = proxy_rules.len(),
        "Redirect rules refreshed in proxy route table"
    );
}
