//! Notification channels and subscriptions API endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CreateNotificationChannelRequest, CreateNotificationSubscriptionRequest, NotificationChannel,
    NotificationChannelResponse, NotificationSubscription, NotificationSubscriptionResponse,
    TestNotificationRequest, UpdateNotificationChannelRequest,
};
use crate::notifications::NotificationService;
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

// -------------------------------------------------------------------------
// Notification Channels
// -------------------------------------------------------------------------

/// List all notification channels
pub async fn list_channels(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<NotificationChannelResponse>>, ApiError> {
    let channels = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<NotificationChannelResponse> =
        channels.into_iter().map(|c| c.into()).collect();

    Ok(Json(responses))
}

/// Get a notification channel by ID
pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<NotificationChannelResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    Ok(Json(channel.into()))
}

/// Create a new notification channel
pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNotificationChannelRequest>,
) -> Result<(StatusCode, Json<NotificationChannelResponse>), ApiError> {
    // Validate the name
    if req.name.trim().is_empty() {
        return Err(ApiError::validation_field("name", "Name is required"));
    }

    if req.name.len() > 100 {
        return Err(ApiError::validation_field(
            "name",
            "Name must be 100 characters or less",
        ));
    }

    // Validate the config based on channel type
    validate_channel_config(&req.channel_type.to_string(), &req.config)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&req.config)
        .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?;

    sqlx::query(
        r#"
        INSERT INTO notification_channels (id, name, channel_type, config, enabled, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(req.channel_type.to_string())
    .bind(&config_json)
    .bind(if req.enabled { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create notification channel: {}", e);
        ApiError::database("Failed to create notification channel")
    })?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok((StatusCode::CREATED, Json(channel.into())))
}

/// Update a notification channel
pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNotificationChannelRequest>,
) -> Result<Json<NotificationChannelResponse>, ApiError> {
    if let Err(e) = validate_uuid(&id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Check if channel exists
    let existing = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::validation_field("name", "Name cannot be empty"));
        }
        if name.len() > 100 {
            return Err(ApiError::validation_field(
                "name",
                "Name must be 100 characters or less",
            ));
        }
    }

    // Validate config if provided
    if let Some(ref config) = req.config {
        validate_channel_config(&existing.channel_type, config)?;
    }

    let now = chrono::Utc::now().to_rfc3339();
    let name = req.name.unwrap_or(existing.name);
    let config_json = match req.config {
        Some(config) => serde_json::to_string(&config)
            .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?,
        None => existing.config,
    };
    let enabled = req
        .enabled
        .map(|e| if e { 1 } else { 0 })
        .unwrap_or(existing.enabled);

    sqlx::query(
        r#"
        UPDATE notification_channels
        SET name = ?, config = ?, enabled = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&name)
    .bind(&config_json)
    .bind(enabled)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update notification channel: {}", e);
        ApiError::database("Failed to update notification channel")
    })?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(channel.into()))
}

/// Delete a notification channel
pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    let result = sqlx::query("DELETE FROM notification_channels WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Notification channel not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Send a test notification
pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<TestNotificationRequest>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    let notification_service = NotificationService::new(state.db.clone());

    notification_service
        .send_test(&channel, req.message)
        .await
        .map_err(|e| {
            tracing::error!("Test notification failed: {}", e);
            ApiError::internal(&format!("Failed to send test notification: {}", e))
        })?;

    Ok(StatusCode::OK)
}

// -------------------------------------------------------------------------
// Notification Subscriptions
// -------------------------------------------------------------------------

/// List subscriptions for a channel
pub async fn list_subscriptions(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<NotificationSubscriptionResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Verify channel exists
    let _channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&channel_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    // Get subscriptions with app names
    let subscriptions = sqlx::query_as::<_, NotificationSubscription>(
        "SELECT * FROM notification_subscriptions WHERE channel_id = ? ORDER BY created_at DESC",
    )
    .bind(&channel_id)
    .fetch_all(&state.db)
    .await?;

    let mut responses = Vec::new();
    for sub in subscriptions {
        let app_name = if let Some(ref app_id) = sub.app_id {
            sqlx::query_scalar::<_, String>("SELECT name FROM apps WHERE id = ?")
                .bind(app_id)
                .fetch_optional(&state.db)
                .await?
        } else {
            None
        };

        responses.push(NotificationSubscriptionResponse {
            id: sub.id,
            channel_id: sub.channel_id,
            event_type: sub.event_type,
            app_id: sub.app_id,
            app_name,
            created_at: sub.created_at,
        });
    }

    Ok(Json(responses))
}

/// Create a subscription for a channel
pub async fn create_subscription(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
    Json(req): Json<CreateNotificationSubscriptionRequest>,
) -> Result<(StatusCode, Json<NotificationSubscriptionResponse>), ApiError> {
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Verify channel exists
    let _channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&channel_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    // Verify app exists if specified
    let app_name = if let Some(ref app_id) = req.app_id {
        if let Err(e) = validate_uuid(app_id, "app_id") {
            return Err(ApiError::validation_field("app_id", e));
        }

        let app_name = sqlx::query_scalar::<_, String>("SELECT name FROM apps WHERE id = ?")
            .bind(app_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| ApiError::not_found("App not found"))?;

        Some(app_name)
    } else {
        None
    };

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO notification_subscriptions (id, channel_id, event_type, app_id, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&channel_id)
    .bind(req.event_type.to_string())
    .bind(&req.app_id)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create notification subscription: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("This subscription already exists")
        } else {
            ApiError::database("Failed to create notification subscription")
        }
    })?;

    let response = NotificationSubscriptionResponse {
        id,
        channel_id,
        event_type: req.event_type.to_string(),
        app_id: req.app_id,
        app_name,
        created_at: now,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete a subscription
pub async fn delete_subscription(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&id, "subscription_id") {
        return Err(ApiError::validation_field("subscription_id", e));
    }

    let result = sqlx::query("DELETE FROM notification_subscriptions WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Notification subscription not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Validate channel configuration based on type
fn validate_channel_config(channel_type: &str, config: &serde_json::Value) -> Result<(), ApiError> {
    match channel_type {
        "slack" => {
            let webhook_url = config
                .get("webhook_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApiError::validation_field("config.webhook_url", "Webhook URL is required")
                })?;

            if !webhook_url.starts_with("https://hooks.slack.com/") {
                return Err(ApiError::validation_field(
                    "config.webhook_url",
                    "Invalid Slack webhook URL",
                ));
            }
        }
        "discord" => {
            let webhook_url = config
                .get("webhook_url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ApiError::validation_field("config.webhook_url", "Webhook URL is required")
                })?;

            if !webhook_url.starts_with("https://discord.com/api/webhooks/")
                && !webhook_url.starts_with("https://discordapp.com/api/webhooks/")
            {
                return Err(ApiError::validation_field(
                    "config.webhook_url",
                    "Invalid Discord webhook URL",
                ));
            }
        }
        "email" => {
            let smtp_host = config.get("smtp_host").and_then(|v| v.as_str());
            if smtp_host.is_none() || smtp_host.unwrap().is_empty() {
                return Err(ApiError::validation_field(
                    "config.smtp_host",
                    "SMTP host is required",
                ));
            }

            let smtp_port = config.get("smtp_port").and_then(|v| v.as_u64());
            if smtp_port.is_none() || smtp_port.unwrap() == 0 || smtp_port.unwrap() > 65535 {
                return Err(ApiError::validation_field(
                    "config.smtp_port",
                    "Valid SMTP port is required",
                ));
            }

            let from_address = config.get("from_address").and_then(|v| v.as_str());
            if from_address.is_none() || from_address.unwrap().is_empty() {
                return Err(ApiError::validation_field(
                    "config.from_address",
                    "From address is required",
                ));
            }

            let to_addresses = config.get("to_addresses").and_then(|v| v.as_array());
            if to_addresses.is_none() || to_addresses.unwrap().is_empty() {
                return Err(ApiError::validation_field(
                    "config.to_addresses",
                    "At least one recipient address is required",
                ));
            }
        }
        "webhook" => {
            let url = config.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
                ApiError::validation_field("config.url", "Webhook URL is required")
            })?;

            // Validate that URL is HTTPS
            if !url.starts_with("https://") {
                return Err(ApiError::validation_field(
                    "config.url",
                    "Webhook URL must use HTTPS",
                ));
            }

            // Validate URL format using reqwest::Url (which is re-exported from url crate)
            if reqwest::Url::parse(url).is_err() {
                return Err(ApiError::validation_field(
                    "config.url",
                    "Invalid webhook URL format",
                ));
            }

            // Validate payload_template if provided
            if let Some(template) = config.get("payload_template").and_then(|v| v.as_str()) {
                match template {
                    "json" | "slack" | "discord" | "custom" => {}
                    _ => {
                        return Err(ApiError::validation_field(
                            "config.payload_template",
                            "Invalid payload template. Must be one of: json, slack, discord, custom",
                        ));
                    }
                }

                // If custom template, validate custom_template is provided
                if template == "custom" {
                    let custom_template = config.get("custom_template").and_then(|v| v.as_str());
                    if custom_template.is_none() || custom_template.unwrap().is_empty() {
                        return Err(ApiError::validation_field(
                            "config.custom_template",
                            "Custom template is required when payload_template is 'custom'",
                        ));
                    }
                }
            }
        }
        _ => {
            return Err(ApiError::validation_field(
                "channel_type",
                "Invalid channel type",
            ));
        }
    }

    Ok(())
}

// -------------------------------------------------------------------------
// Team Notification Channels
// -------------------------------------------------------------------------

use crate::db::{TeamRole, User};

/// Get the current user's membership in a team
async fn get_user_team_membership(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<Option<crate::db::TeamMember>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM team_members WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

/// Require that the current user has at least the specified role in the team
async fn require_team_role(
    pool: &sqlx::SqlitePool,
    team_id: &str,
    user_id: &str,
    required_role: TeamRole,
) -> Result<crate::db::TeamMember, ApiError> {
    let membership = get_user_team_membership(pool, team_id, user_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("You are not a member of this team"))?;

    let user_role = membership.role_enum();
    if !user_role.has_at_least(required_role) {
        return Err(ApiError::forbidden(format!(
            "This action requires {} role or higher",
            required_role
        )));
    }

    Ok(membership)
}

/// List notification channels for a team
///
/// GET /api/teams/:id/notification-channels
pub async fn list_team_channels(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    user: User,
) -> Result<Json<Vec<NotificationChannelResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is a member of the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Viewer).await?;

    let channels = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(&team_id)
    .fetch_all(&state.db)
    .await?;

    let responses: Vec<NotificationChannelResponse> =
        channels.into_iter().map(|c| c.into()).collect();

    Ok(Json(responses))
}

/// Get a team notification channel by ID
///
/// GET /api/teams/:id/notification-channels/:channel_id
pub async fn get_team_channel(
    State(state): State<Arc<AppState>>,
    Path((team_id, channel_id)): Path<(String, String)>,
    user: User,
) -> Result<Json<NotificationChannelResponse>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Check user is a member of the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Viewer).await?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ? AND team_id = ?",
    )
    .bind(&channel_id)
    .bind(&team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    Ok(Json(channel.into()))
}

/// Create a notification channel for a team
///
/// POST /api/teams/:id/notification-channels
pub async fn create_team_channel(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    user: User,
    Json(req): Json<CreateNotificationChannelRequest>,
) -> Result<(StatusCode, Json<NotificationChannelResponse>), ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user has admin+ role in the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    // Verify team exists
    let team_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check team: {}", e);
            ApiError::internal("Database error")
        })?;

    if team_exists == 0 {
        return Err(ApiError::not_found("Team not found"));
    }

    // Validate the name
    if req.name.trim().is_empty() {
        return Err(ApiError::validation_field("name", "Name is required"));
    }

    if req.name.len() > 100 {
        return Err(ApiError::validation_field(
            "name",
            "Name must be 100 characters or less",
        ));
    }

    // Validate the config based on channel type
    validate_channel_config(&req.channel_type.to_string(), &req.config)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&req.config)
        .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?;

    sqlx::query(
        r#"
        INSERT INTO notification_channels (id, name, channel_type, config, enabled, created_at, updated_at, team_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(req.channel_type.to_string())
    .bind(&config_json)
    .bind(if req.enabled { 1 } else { 0 })
    .bind(&now)
    .bind(&now)
    .bind(&team_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create team notification channel: {}", e);
        ApiError::database("Failed to create notification channel")
    })?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(
        team_id = %team_id,
        channel_id = %id,
        channel_type = %req.channel_type,
        "Created team notification channel"
    );

    Ok((StatusCode::CREATED, Json(channel.into())))
}

/// Update a team notification channel
///
/// PUT /api/teams/:id/notification-channels/:channel_id
pub async fn update_team_channel(
    State(state): State<Arc<AppState>>,
    Path((team_id, channel_id)): Path<(String, String)>,
    user: User,
    Json(req): Json<UpdateNotificationChannelRequest>,
) -> Result<Json<NotificationChannelResponse>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Check user has admin+ role in the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    // Check if channel exists and belongs to the team
    let existing = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ? AND team_id = ?",
    )
    .bind(&channel_id)
    .bind(&team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.trim().is_empty() {
            return Err(ApiError::validation_field("name", "Name cannot be empty"));
        }
        if name.len() > 100 {
            return Err(ApiError::validation_field(
                "name",
                "Name must be 100 characters or less",
            ));
        }
    }

    // Validate config if provided
    if let Some(ref config) = req.config {
        validate_channel_config(&existing.channel_type, config)?;
    }

    let now = chrono::Utc::now().to_rfc3339();
    let name = req.name.unwrap_or(existing.name);
    let config_json = match req.config {
        Some(config) => serde_json::to_string(&config)
            .map_err(|_| ApiError::validation_field("config", "Invalid configuration format"))?,
        None => existing.config,
    };
    let enabled = req
        .enabled
        .map(|e| if e { 1 } else { 0 })
        .unwrap_or(existing.enabled);

    sqlx::query(
        r#"
        UPDATE notification_channels
        SET name = ?, config = ?, enabled = ?, updated_at = ?
        WHERE id = ? AND team_id = ?
        "#,
    )
    .bind(&name)
    .bind(&config_json)
    .bind(enabled)
    .bind(&now)
    .bind(&channel_id)
    .bind(&team_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update team notification channel: {}", e);
        ApiError::database("Failed to update notification channel")
    })?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ?",
    )
    .bind(&channel_id)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(
        team_id = %team_id,
        channel_id = %channel_id,
        "Updated team notification channel"
    );

    Ok(Json(channel.into()))
}

/// Delete a team notification channel
///
/// DELETE /api/teams/:id/notification-channels/:channel_id
pub async fn delete_team_channel(
    State(state): State<Arc<AppState>>,
    Path((team_id, channel_id)): Path<(String, String)>,
    user: User,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Check user has admin+ role in the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    let result = sqlx::query("DELETE FROM notification_channels WHERE id = ? AND team_id = ?")
        .bind(&channel_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Notification channel not found"));
    }

    tracing::info!(
        team_id = %team_id,
        channel_id = %channel_id,
        "Deleted team notification channel"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Test a team notification channel
///
/// POST /api/teams/:id/notification-channels/:channel_id/test
pub async fn test_team_channel(
    State(state): State<Arc<AppState>>,
    Path((team_id, channel_id)): Path<(String, String)>,
    user: User,
    Json(req): Json<TestNotificationRequest>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&channel_id, "channel_id") {
        return Err(ApiError::validation_field("channel_id", e));
    }

    // Check user has admin+ role in the team
    require_team_role(&state.db, &team_id, &user.id, TeamRole::Admin).await?;

    let channel = sqlx::query_as::<_, NotificationChannel>(
        "SELECT * FROM notification_channels WHERE id = ? AND team_id = ?",
    )
    .bind(&channel_id)
    .bind(&team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    let notification_service = NotificationService::new(state.db.clone());

    notification_service
        .send_test(&channel, req.message)
        .await
        .map_err(|e| {
            tracing::error!("Test notification failed for team channel: {}", e);
            ApiError::internal(&format!("Failed to send test notification: {}", e))
        })?;

    tracing::info!(
        team_id = %team_id,
        channel_id = %channel_id,
        "Tested team notification channel"
    );

    Ok(StatusCode::OK)
}
