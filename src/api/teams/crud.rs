//! Team CRUD handlers: list, get, create, update, delete.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::{
    CreateTeamRequest, Team, TeamAuditAction, TeamAuditResourceType, TeamDetail, TeamMemberWithUser,
    TeamRole, TeamWithMemberCount, UpdateTeamRequest, User,
};
use crate::AppState;

use super::super::error::ApiError;
use super::super::validation::validate_uuid;
use super::{
    generate_slug, get_user_team_membership, require_team_role, validate_create_request,
    validate_team_slug, validate_update_request,
};
use super::audit::log_team_audit;

/// List teams for the current user
pub async fn list_teams(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<TeamWithMemberCount>>, ApiError> {
    // Get all teams the user is a member of
    let mut teams = sqlx::query_as::<_, Team>(
        r#"
        SELECT t.* FROM teams t
        INNER JOIN team_members tm ON t.id = tm.team_id
        WHERE tm.user_id = ?
        ORDER BY t.created_at DESC
        "#,
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;

    // If user has no teams, create a default "Personal" team for them
    if teams.is_empty() {
        let team_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO teams (id, name, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&team_id)
        .bind("Personal")
        .bind(format!(
            "personal-{}",
            &user.id.chars().take(8).collect::<String>()
        ))
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;

        let member_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO team_members (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&member_id)
        .bind(&team_id)
        .bind(&user.id)
        .bind("owner")
        .bind(&now)
        .execute(&state.db)
        .await?;

        tracing::info!("Created default Personal team for user: {}", user.email);

        // Fetch the newly created team
        if let Some(team) = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
            .bind(&team_id)
            .fetch_optional(&state.db)
            .await?
        {
            teams.push(team);
        }
    }

    // Get member counts and user roles for each team
    let mut results = Vec::new();
    for team in teams {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_members WHERE team_id = ?")
            .bind(&team.id)
            .fetch_one(&state.db)
            .await?;

        let membership = get_user_team_membership(&state.db, &team.id, &user.id).await?;

        results.push(TeamWithMemberCount {
            id: team.id,
            name: team.name,
            slug: team.slug,
            created_at: team.created_at,
            updated_at: team.updated_at,
            member_count: count.0,
            user_role: membership.map(|m| m.role),
        });
    }

    Ok(Json(results))
}

/// Get a specific team by ID
pub async fn get_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<TeamDetail>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is a member of the team and get their role
    let membership = require_team_role(&state.db, &id, &user.id, TeamRole::Viewer).await?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Get members with user details
    let members = sqlx::query_as::<_, TeamMemberWithUser>(
        r#"
        SELECT tm.id, tm.team_id, tm.user_id, tm.role, tm.created_at,
               u.name as user_name, u.email as user_email
        FROM team_members tm
        INNER JOIN users u ON tm.user_id = u.id
        WHERE tm.team_id = ?
        ORDER BY
            CASE tm.role
                WHEN 'owner' THEN 1
                WHEN 'admin' THEN 2
                WHEN 'developer' THEN 3
                WHEN 'viewer' THEN 4
            END,
            tm.created_at ASC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(TeamDetail {
        id: team.id,
        name: team.name,
        slug: team.slug,
        created_at: team.created_at,
        updated_at: team.updated_at,
        members,
        user_role: Some(membership.role),
    }))
}

/// Create a new team
pub async fn create_team(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<CreateTeamRequest>,
) -> Result<(StatusCode, Json<Team>), ApiError> {
    // Validate request
    validate_create_request(&req)?;

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let slug = req.slug.unwrap_or_else(|| generate_slug(&req.name));

    // Validate the generated/provided slug
    validate_team_slug(&slug).map_err(|e| ApiError::validation_field("slug", e))?;

    // Create team
    sqlx::query(
        r#"
        INSERT INTO teams (id, name, slug, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&slug)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create team: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A team with this slug already exists")
        } else {
            ApiError::database("Failed to create team")
        }
    })?;

    // Add the creator as owner
    let member_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO team_members (id, team_id, user_id, role, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&member_id)
    .bind(&id)
    .bind(&user.id)
    .bind("owner")
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add team owner: {}", e);
        ApiError::database("Failed to create team membership")
    })?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    // Log audit event for team creation
    if let Err(e) = log_team_audit(
        &state.db,
        &id,
        Some(&user.id),
        TeamAuditAction::TeamCreated,
        TeamAuditResourceType::Team,
        Some(&id),
        Some(serde_json::json!({
            "name": &team.name,
            "slug": &team.slug
        })),
    )
    .await
    {
        tracing::error!("Failed to log team creation audit: {}", e);
    }

    tracing::info!("Created team '{}' with owner {}", team.name, user.email);

    Ok((StatusCode::CREATED, Json(team)))
}

/// Update a team
pub async fn update_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    Json(req): Json<UpdateTeamRequest>,
) -> Result<Json<Team>, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Validate request
    validate_update_request(&req)?;

    // Check user has admin+ role
    require_team_role(&state.db, &id, &user.id, TeamRole::Admin).await?;

    // Check if team exists
    let _existing = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE teams SET
            name = COALESCE(?, name),
            slug = COALESCE(?, slug),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&req.name)
    .bind(&req.slug)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update team: {}", e);
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A team with this slug already exists")
        } else {
            ApiError::database("Failed to update team")
        }
    })?;

    let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(team))
}

/// Delete a team (owner only)
pub async fn delete_team(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<StatusCode, ApiError> {
    // Validate ID format
    if let Err(e) = validate_uuid(&id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    // Check user is owner
    require_team_role(&state.db, &id, &user.id, TeamRole::Owner).await?;

    // Delete the team (members will be cascade deleted)
    let result = sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team not found"));
    }

    tracing::info!("Deleted team {} by user {}", id, user.email);

    Ok(StatusCode::NO_CONTENT)
}
