//! Shared environment variables API endpoints.
//!
//! Provides team-level and project-level shared env vars that are
//! inherited by all apps in the team/project.
//!
//! Inheritance order (lowest → highest priority):
//!   team_env_vars → project_env_vars → environment_env_vars → env_vars (app)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::crypto;
use crate::db::{
    CreateProjectEnvVarRequest, CreateTeamEnvVarRequest, EnvVarSource, ProjectEnvVar,
    ProjectEnvVarResponse, ResolvedEnvVar, TeamEnvVar, TeamEnvVarResponse,
    UpdateProjectEnvVarRequest, UpdateTeamEnvVarRequest,
};
use crate::AppState;

use super::error::ApiError;
use super::validation::validate_uuid;

/// Key length for AES-256 encryption
const KEY_LENGTH: usize = 32;

/// Get the derived encryption key from the config if configured
fn get_encryption_key(state: &AppState) -> Option<[u8; KEY_LENGTH]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|secret| crypto::derive_key(secret))
}

/// Query params for listing env vars (optional reveal)
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub reveal: bool,
}

/// Validate environment variable key format.
/// Must start with letter or underscore; contain only alphanumeric and underscore.
fn is_valid_env_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let first_char = key.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }
    key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---------------------------------------------------------------------------
// Team Env Vars
// ---------------------------------------------------------------------------

/// GET /api/teams/:id/env-vars
/// List all team-level shared environment variables (secrets masked unless reveal=true)
pub async fn list_team_env_vars(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<TeamEnvVarResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    let team_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(&state.db)
        .await?;
    if team_exists == 0 {
        return Err(ApiError::not_found("Team not found"));
    }

    let vars = sqlx::query_as::<_, TeamEnvVar>(
        "SELECT id, team_id, key, value, is_secret, description, created_at, updated_at \
         FROM team_env_vars WHERE team_id = ? ORDER BY key ASC",
    )
    .bind(&team_id)
    .fetch_all(&state.db)
    .await?;

    let encryption_key = get_encryption_key(&state);

    let responses: Vec<TeamEnvVarResponse> = vars
        .into_iter()
        .map(|v| {
            let decrypted_value = crypto::decrypt_if_encrypted(&v.value, encryption_key.as_ref())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to decrypt team env var {}: {}", v.key, e);
                    v.value.clone()
                });
            let decrypted = TeamEnvVar {
                value: decrypted_value,
                ..v
            };
            decrypted.to_response(query.reveal)
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/teams/:id/env-vars
/// Create a new team-level shared environment variable
pub async fn create_team_env_var(
    State(state): State<Arc<AppState>>,
    Path(team_id): Path<String>,
    Json(req): Json<CreateTeamEnvVarRequest>,
) -> Result<(StatusCode, Json<TeamEnvVarResponse>), ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }

    let team_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teams WHERE id = ?")
        .bind(&team_id)
        .fetch_one(&state.db)
        .await?;
    if team_exists == 0 {
        return Err(ApiError::not_found("Team not found"));
    }

    if !is_valid_env_key(&req.key) {
        return Err(ApiError::validation_field(
            "key",
            "Invalid environment variable key format. Must start with a letter or underscore and contain only alphanumeric characters and underscores.",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let encryption_key = get_encryption_key(&state);
    let stored_value = crypto::encrypt_if_key_available(&req.value, encryption_key.as_ref())
        .map_err(|e| {
            tracing::error!("Failed to encrypt team env var value: {}", e);
            ApiError::internal("Failed to encrypt environment variable value")
        })?;

    sqlx::query(
        r#"
        INSERT INTO team_env_vars (id, team_id, key, value, is_secret, description, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&team_id)
    .bind(&req.key)
    .bind(&stored_value)
    .bind(if req.is_secret { 1 } else { 0 })
    .bind(&req.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A variable with this key already exists for this team")
        } else {
            tracing::error!("Failed to create team env var: {}", e);
            ApiError::database("Failed to create team environment variable")
        }
    })?;

    let var = sqlx::query_as::<_, TeamEnvVar>(
        "SELECT id, team_id, key, value, is_secret, description, created_at, updated_at \
         FROM team_env_vars WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    // Return plaintext value to the user who just created it
    let response_var = TeamEnvVar {
        value: req.value.clone(),
        ..var
    };

    Ok((StatusCode::CREATED, Json(response_var.to_response(true))))
}

/// PUT /api/teams/:id/env-vars/:var_id
/// Update a team-level shared environment variable
pub async fn update_team_env_var(
    State(state): State<Arc<AppState>>,
    Path((team_id, var_id)): Path<(String, String)>,
    Json(req): Json<UpdateTeamEnvVarRequest>,
) -> Result<Json<TeamEnvVarResponse>, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "var_id") {
        return Err(ApiError::validation_field("var_id", e));
    }

    let existing = sqlx::query_as::<_, TeamEnvVar>(
        "SELECT id, team_id, key, value, is_secret, description, created_at, updated_at \
         FROM team_env_vars WHERE id = ? AND team_id = ?",
    )
    .bind(&var_id)
    .bind(&team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Team environment variable not found"))?;

    let encryption_key = get_encryption_key(&state);

    let existing_decrypted = crypto::decrypt_if_encrypted(&existing.value, encryption_key.as_ref())
        .unwrap_or_else(|_| existing.value.clone());

    let new_plaintext_value = req.value.unwrap_or(existing_decrypted);
    let new_is_secret = req
        .is_secret
        .map(|b| if b { 1 } else { 0 })
        .unwrap_or(existing.is_secret);
    let new_description = req.description.or(existing.description.clone());

    let stored_value =
        crypto::encrypt_if_key_available(&new_plaintext_value, encryption_key.as_ref()).map_err(
            |e| {
                tracing::error!("Failed to encrypt team env var value: {}", e);
                ApiError::internal("Failed to encrypt environment variable value")
            },
        )?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE team_env_vars SET
            value = ?,
            is_secret = ?,
            description = ?,
            updated_at = ?
        WHERE id = ? AND team_id = ?
        "#,
    )
    .bind(&stored_value)
    .bind(new_is_secret)
    .bind(&new_description)
    .bind(&now)
    .bind(&var_id)
    .bind(&team_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update team env var: {}", e);
        ApiError::database("Failed to update team environment variable")
    })?;

    let var = sqlx::query_as::<_, TeamEnvVar>(
        "SELECT id, team_id, key, value, is_secret, description, created_at, updated_at \
         FROM team_env_vars WHERE id = ?",
    )
    .bind(&var_id)
    .fetch_one(&state.db)
    .await?;

    let response_var = TeamEnvVar {
        value: new_plaintext_value,
        ..var
    };

    Ok(Json(response_var.to_response(true)))
}

/// DELETE /api/teams/:id/env-vars/:var_id
/// Delete a team-level shared environment variable
pub async fn delete_team_env_var(
    State(state): State<Arc<AppState>>,
    Path((team_id, var_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&team_id, "team_id") {
        return Err(ApiError::validation_field("team_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "var_id") {
        return Err(ApiError::validation_field("var_id", e));
    }

    let result = sqlx::query("DELETE FROM team_env_vars WHERE id = ? AND team_id = ?")
        .bind(&var_id)
        .bind(&team_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Team environment variable not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Project Env Vars
// ---------------------------------------------------------------------------

/// GET /api/projects/:id/env-vars
/// List all project-level shared environment variables (secrets masked unless reveal=true)
pub async fn list_project_env_vars(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ProjectEnvVarResponse>>, ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    let project_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await?;
    if project_exists == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    let vars = sqlx::query_as::<_, ProjectEnvVar>(
        "SELECT id, project_id, key, value, is_secret, description, created_at, updated_at \
         FROM project_env_vars WHERE project_id = ? ORDER BY key ASC",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await?;

    let encryption_key = get_encryption_key(&state);

    let responses: Vec<ProjectEnvVarResponse> = vars
        .into_iter()
        .map(|v| {
            let decrypted_value = crypto::decrypt_if_encrypted(&v.value, encryption_key.as_ref())
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to decrypt project env var {}: {}", v.key, e);
                    v.value.clone()
                });
            let decrypted = ProjectEnvVar {
                value: decrypted_value,
                ..v
            };
            decrypted.to_response(query.reveal)
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/projects/:id/env-vars
/// Create a new project-level shared environment variable
pub async fn create_project_env_var(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateProjectEnvVarRequest>,
) -> Result<(StatusCode, Json<ProjectEnvVarResponse>), ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }

    let project_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await?;
    if project_exists == 0 {
        return Err(ApiError::not_found("Project not found"));
    }

    if !is_valid_env_key(&req.key) {
        return Err(ApiError::validation_field(
            "key",
            "Invalid environment variable key format. Must start with a letter or underscore and contain only alphanumeric characters and underscores.",
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let encryption_key = get_encryption_key(&state);
    let stored_value = crypto::encrypt_if_key_available(&req.value, encryption_key.as_ref())
        .map_err(|e| {
            tracing::error!("Failed to encrypt project env var value: {}", e);
            ApiError::internal("Failed to encrypt environment variable value")
        })?;

    sqlx::query(
        r#"
        INSERT INTO project_env_vars (id, project_id, key, value, is_secret, description, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&project_id)
    .bind(&req.key)
    .bind(&stored_value)
    .bind(if req.is_secret { 1 } else { 0 })
    .bind(&req.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            ApiError::conflict("A variable with this key already exists for this project")
        } else {
            tracing::error!("Failed to create project env var: {}", e);
            ApiError::database("Failed to create project environment variable")
        }
    })?;

    let var = sqlx::query_as::<_, ProjectEnvVar>(
        "SELECT id, project_id, key, value, is_secret, description, created_at, updated_at \
         FROM project_env_vars WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    let response_var = ProjectEnvVar {
        value: req.value.clone(),
        ..var
    };

    Ok((StatusCode::CREATED, Json(response_var.to_response(true))))
}

/// PUT /api/projects/:id/env-vars/:var_id
/// Update a project-level shared environment variable
pub async fn update_project_env_var(
    State(state): State<Arc<AppState>>,
    Path((project_id, var_id)): Path<(String, String)>,
    Json(req): Json<UpdateProjectEnvVarRequest>,
) -> Result<Json<ProjectEnvVarResponse>, ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "var_id") {
        return Err(ApiError::validation_field("var_id", e));
    }

    let existing = sqlx::query_as::<_, ProjectEnvVar>(
        "SELECT id, project_id, key, value, is_secret, description, created_at, updated_at \
         FROM project_env_vars WHERE id = ? AND project_id = ?",
    )
    .bind(&var_id)
    .bind(&project_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("Project environment variable not found"))?;

    let encryption_key = get_encryption_key(&state);

    let existing_decrypted = crypto::decrypt_if_encrypted(&existing.value, encryption_key.as_ref())
        .unwrap_or_else(|_| existing.value.clone());

    let new_plaintext_value = req.value.unwrap_or(existing_decrypted);
    let new_is_secret = req
        .is_secret
        .map(|b| if b { 1 } else { 0 })
        .unwrap_or(existing.is_secret);
    let new_description = req.description.or(existing.description.clone());

    let stored_value =
        crypto::encrypt_if_key_available(&new_plaintext_value, encryption_key.as_ref()).map_err(
            |e| {
                tracing::error!("Failed to encrypt project env var value: {}", e);
                ApiError::internal("Failed to encrypt environment variable value")
            },
        )?;

    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE project_env_vars SET
            value = ?,
            is_secret = ?,
            description = ?,
            updated_at = ?
        WHERE id = ? AND project_id = ?
        "#,
    )
    .bind(&stored_value)
    .bind(new_is_secret)
    .bind(&new_description)
    .bind(&now)
    .bind(&var_id)
    .bind(&project_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to update project env var: {}", e);
        ApiError::database("Failed to update project environment variable")
    })?;

    let var = sqlx::query_as::<_, ProjectEnvVar>(
        "SELECT id, project_id, key, value, is_secret, description, created_at, updated_at \
         FROM project_env_vars WHERE id = ?",
    )
    .bind(&var_id)
    .fetch_one(&state.db)
    .await?;

    let response_var = ProjectEnvVar {
        value: new_plaintext_value,
        ..var
    };

    Ok(Json(response_var.to_response(true)))
}

/// DELETE /api/projects/:id/env-vars/:var_id
/// Delete a project-level shared environment variable
pub async fn delete_project_env_var(
    State(state): State<Arc<AppState>>,
    Path((project_id, var_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    if let Err(e) = validate_uuid(&project_id, "project_id") {
        return Err(ApiError::validation_field("project_id", e));
    }
    if let Err(e) = validate_uuid(&var_id, "var_id") {
        return Err(ApiError::validation_field("var_id", e));
    }

    let result = sqlx::query("DELETE FROM project_env_vars WHERE id = ? AND project_id = ?")
        .bind(&var_id)
        .bind(&project_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "Project environment variable not found",
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Resolved Env Vars (inheritance chain)
// ---------------------------------------------------------------------------

/// GET /api/apps/:id/env-vars/resolved
/// Get the effective environment variables for an app showing the full
/// inheritance chain: team → project → environment → app (highest priority wins).
///
/// Secrets are always masked as `***` in this endpoint.
pub async fn get_resolved_env_vars(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<String>,
) -> Result<Json<Vec<ResolvedEnvVar>>, ApiError> {
    if let Err(e) = validate_uuid(&app_id, "app_id") {
        return Err(ApiError::validation_field("app_id", e));
    }

    // Fetch the app to get project_id, environment_id, and team_id
    let app = sqlx::query_as::<_, crate::db::App>("SELECT * FROM apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("App not found"))?;

    // We build a map: key → ResolvedEnvVar.
    // We insert in priority order lowest first, then higher priority overwrites.
    let mut resolved: std::collections::HashMap<String, ResolvedEnvVar> =
        std::collections::HashMap::new();

    let encryption_key = get_encryption_key(&state);

    // 1. Team-level vars (lowest priority)
    if let Some(ref team_id) = app.team_id {
        let team_vars = sqlx::query_as::<_, TeamEnvVar>(
            "SELECT id, team_id, key, value, is_secret, description, created_at, updated_at \
             FROM team_env_vars WHERE team_id = ?",
        )
        .bind(team_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for v in team_vars {
            resolved.insert(
                v.key.clone(),
                ResolvedEnvVar {
                    key: v.key,
                    value: if v.is_secret != 0 {
                        "***".to_string()
                    } else {
                        crypto::decrypt_if_encrypted(&v.value, encryption_key.as_ref())
                            .unwrap_or_else(|_| v.value.clone())
                    },
                    is_secret: v.is_secret != 0,
                    source: EnvVarSource::Team,
                    description: v.description,
                },
            );
        }
    }

    // 2. Project-level vars (overrides team)
    if let Some(ref project_id) = app.project_id {
        let project_vars = sqlx::query_as::<_, ProjectEnvVar>(
            "SELECT id, project_id, key, value, is_secret, description, created_at, updated_at \
             FROM project_env_vars WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for v in project_vars {
            resolved.insert(
                v.key.clone(),
                ResolvedEnvVar {
                    key: v.key,
                    value: if v.is_secret != 0 {
                        "***".to_string()
                    } else {
                        crypto::decrypt_if_encrypted(&v.value, encryption_key.as_ref())
                            .unwrap_or_else(|_| v.value.clone())
                    },
                    is_secret: v.is_secret != 0,
                    source: EnvVarSource::Project,
                    description: v.description,
                },
            );
        }
    }

    // 3. Environment-level vars (overrides project)
    if let Some(ref environment_id) = app.environment_id {
        // EnvironmentEnvVar has no description column; use None
        let env_vars = sqlx::query_as::<_, (String, String, i32)>(
            "SELECT key, value, is_secret FROM environment_env_vars WHERE environment_id = ?",
        )
        .bind(environment_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for (key, value, is_secret) in env_vars {
            resolved.insert(
                key.clone(),
                ResolvedEnvVar {
                    key,
                    value: if is_secret != 0 {
                        "***".to_string()
                    } else {
                        crypto::decrypt_if_encrypted(&value, encryption_key.as_ref())
                            .unwrap_or(value)
                    },
                    is_secret: is_secret != 0,
                    source: EnvVarSource::Environment,
                    description: None,
                },
            );
        }
    }

    // 4. App-level vars (highest priority)
    let app_vars = sqlx::query_as::<_, (String, String, i32)>(
        "SELECT key, value, is_secret FROM env_vars WHERE app_id = ?",
    )
    .bind(&app_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for (key, value, is_secret) in app_vars {
        resolved.insert(
            key.clone(),
            ResolvedEnvVar {
                key,
                value: if is_secret != 0 {
                    "***".to_string()
                } else {
                    crypto::decrypt_if_encrypted(&value, encryption_key.as_ref()).unwrap_or(value)
                },
                is_secret: is_secret != 0,
                source: EnvVarSource::App,
                description: None,
            },
        );
    }

    // Sort alphabetically by key for consistent output
    let mut result: Vec<ResolvedEnvVar> = resolved.into_values().collect();
    result.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(Json(result))
}
