//! Audit log models for tracking user actions.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Audit log entry for tracking user actions
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub details: Option<String>,
    pub created_at: String,
}

/// Request to create an audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuditLogRequest {
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// Response for listing audit logs with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogListResponse {
    pub items: Vec<AuditLog>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

/// Query parameters for filtering audit logs
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditLogQuery {
    /// Filter by action (e.g., "app.create")
    pub action: Option<String>,
    /// Filter by resource type (e.g., "app", "database")
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Start date for filtering (ISO 8601)
    pub start_date: Option<String>,
    /// End date for filtering (ISO 8601)
    pub end_date: Option<String>,
    /// Page number (1-indexed, defaults to 1)
    pub page: Option<i64>,
    /// Items per page (defaults to 50, max 100)
    pub per_page: Option<i64>,
}

/// Common audit action types
pub mod actions {
    // App actions
    pub const APP_CREATE: &str = "app.create";
    pub const APP_UPDATE: &str = "app.update";
    pub const APP_DELETE: &str = "app.delete";
    pub const APP_START: &str = "app.start";
    pub const APP_STOP: &str = "app.stop";
    pub const APP_RESTART: &str = "app.restart";

    // Deployment actions
    pub const DEPLOYMENT_TRIGGER: &str = "deployment.trigger";
    pub const DEPLOYMENT_ROLLBACK: &str = "deployment.rollback";

    // Database actions
    pub const DATABASE_CREATE: &str = "database.create";
    pub const DATABASE_UPDATE: &str = "database.update";
    pub const DATABASE_DELETE: &str = "database.delete";
    pub const DATABASE_START: &str = "database.start";
    pub const DATABASE_STOP: &str = "database.stop";
    pub const DATABASE_BACKUP: &str = "database.backup";

    // Service actions
    pub const SERVICE_CREATE: &str = "service.create";
    pub const SERVICE_UPDATE: &str = "service.update";
    pub const SERVICE_DELETE: &str = "service.delete";
    pub const SERVICE_START: &str = "service.start";
    pub const SERVICE_STOP: &str = "service.stop";

    // Project actions
    pub const PROJECT_CREATE: &str = "project.create";
    pub const PROJECT_UPDATE: &str = "project.update";
    pub const PROJECT_DELETE: &str = "project.delete";

    // Team actions
    pub const TEAM_CREATE: &str = "team.create";
    pub const TEAM_UPDATE: &str = "team.update";
    pub const TEAM_DELETE: &str = "team.delete";
    pub const TEAM_MEMBER_ADD: &str = "team.member.add";
    pub const TEAM_MEMBER_REMOVE: &str = "team.member.remove";
    pub const TEAM_MEMBER_UPDATE: &str = "team.member.update";

    // Auth actions
    pub const AUTH_LOGIN: &str = "auth.login";
    pub const AUTH_LOGOUT: &str = "auth.logout";
    pub const AUTH_SETUP: &str = "auth.setup";

    // Git provider actions
    pub const GIT_PROVIDER_ADD: &str = "git_provider.add";
    pub const GIT_PROVIDER_DELETE: &str = "git_provider.delete";

    // SSH key actions
    pub const SSH_KEY_CREATE: &str = "ssh_key.create";
    pub const SSH_KEY_UPDATE: &str = "ssh_key.update";
    pub const SSH_KEY_DELETE: &str = "ssh_key.delete";

    // GitHub App actions
    pub const GITHUB_APP_CREATE: &str = "github_app.create";
    pub const GITHUB_APP_DELETE: &str = "github_app.delete";

    // Environment variable actions
    pub const ENV_VAR_SET: &str = "env_var.set";
    pub const ENV_VAR_DELETE: &str = "env_var.delete";

    // Notification actions
    pub const NOTIFICATION_CHANNEL_CREATE: &str = "notification_channel.create";
    pub const NOTIFICATION_CHANNEL_UPDATE: &str = "notification_channel.update";
    pub const NOTIFICATION_CHANNEL_DELETE: &str = "notification_channel.delete";
}

/// Common resource types
pub mod resource_types {
    pub const APP: &str = "app";
    pub const DATABASE: &str = "database";
    pub const SERVICE: &str = "service";
    pub const PROJECT: &str = "project";
    pub const TEAM: &str = "team";
    pub const USER: &str = "user";
    pub const DEPLOYMENT: &str = "deployment";
    pub const GIT_PROVIDER: &str = "git_provider";
    pub const SSH_KEY: &str = "ssh_key";
    pub const GITHUB_APP: &str = "github_app";
    pub const ENV_VAR: &str = "env_var";
    pub const NOTIFICATION_CHANNEL: &str = "notification_channel";
}

/// Log an audit event to the database
pub async fn log_audit(
    db: &SqlitePool,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    resource_name: Option<&str>,
    user_id: Option<&str>,
    ip_address: Option<&str>,
    details: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let details_json = details.map(|d| d.to_string());

    sqlx::query(
        r#"
        INSERT INTO audit_logs (id, action, resource_type, resource_id, resource_name, user_id, ip_address, details, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(resource_name)
    .bind(user_id)
    .bind(ip_address)
    .bind(&details_json)
    .bind(&now)
    .execute(db)
    .await?;

    tracing::debug!(
        action = action,
        resource_type = resource_type,
        resource_id = resource_id,
        user_id = user_id,
        "Audit log recorded"
    );

    Ok(())
}

/// List audit logs with filtering and pagination
pub async fn list_audit_logs(
    db: &SqlitePool,
    query: &AuditLogQuery,
) -> Result<AuditLogListResponse, sqlx::Error> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build dynamic WHERE clause
    let mut conditions = Vec::new();
    let mut bindings: Vec<String> = Vec::new();

    if let Some(action) = &query.action {
        conditions.push(format!("action = ?"));
        bindings.push(action.clone());
    }

    if let Some(resource_type) = &query.resource_type {
        conditions.push(format!("resource_type = ?"));
        bindings.push(resource_type.clone());
    }

    if let Some(resource_id) = &query.resource_id {
        conditions.push(format!("resource_id = ?"));
        bindings.push(resource_id.clone());
    }

    if let Some(user_id) = &query.user_id {
        conditions.push(format!("user_id = ?"));
        bindings.push(user_id.clone());
    }

    if let Some(start_date) = &query.start_date {
        conditions.push(format!("created_at >= ?"));
        bindings.push(start_date.clone());
    }

    if let Some(end_date) = &query.end_date {
        conditions.push(format!("created_at <= ?"));
        bindings.push(end_date.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Build and execute count query
    let count_sql = format!("SELECT COUNT(*) as count FROM audit_logs {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for binding in &bindings {
        count_query = count_query.bind(binding);
    }
    let total = count_query.fetch_one(db).await?;

    // Build and execute main query
    let sql = format!(
        "SELECT * FROM audit_logs {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut query_builder = sqlx::query_as::<_, AuditLog>(&sql);
    for binding in &bindings {
        query_builder = query_builder.bind(binding);
    }
    query_builder = query_builder.bind(per_page).bind(offset);

    let items = query_builder.fetch_all(db).await?;

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(AuditLogListResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
    })
}
