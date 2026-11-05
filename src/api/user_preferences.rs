//! Per-user UI preferences (dashboard theme).
//!
//! GET /api/preferences/theme      - the caller's theme plus the instance default
//! PUT /api/preferences/theme      - save (or clear with `null`) the caller's theme
//! PUT /api/settings/default-theme - admin: set the theme used by users who never customised theirs
//!
//! The theme is an opaque JSON object owned by the dashboard. The server only
//! checks that it is an object of a sane size; the dashboard validates every
//! field before applying it.

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::authz;
use crate::db::User;
use crate::AppState;

/// Largest accepted serialized theme. A full theme is a few hundred bytes.
const MAX_THEME_BYTES: usize = 4096;

/// instance_settings key holding the default theme JSON.
const DEFAULT_THEME_KEY: &str = "default_theme";

#[derive(Debug, Serialize)]
pub struct ThemeResponse {
    /// The caller's saved theme, `null` when they never customised it.
    pub theme: Option<Value>,
    /// Instance default chosen by an admin, `null` when unset.
    pub instance_default: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThemeRequest {
    /// New theme, or `null` to clear it and fall back to the instance default.
    pub theme: Option<Value>,
}

/// Validate and serialize a theme. `Ok(None)` clears it.
fn encode_theme(theme: Option<&Value>) -> Result<Option<String>, StatusCode> {
    let Some(value) = theme else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    if !value.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let encoded = serde_json::to_string(value).map_err(|_| StatusCode::BAD_REQUEST)?;
    if encoded.len() > MAX_THEME_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(Some(encoded))
}

/// Parse a stored theme; a corrupt row reads as "not customised".
fn decode_theme(raw: Option<String>) -> Option<Value> {
    raw.and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
}

pub(crate) async fn load_user_theme(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT theme FROM user_preferences WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(decode_theme(row.and_then(|(t,)| t)))
}

pub(crate) async fn save_user_theme(
    pool: &SqlitePool,
    user_id: &str,
    theme: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_preferences (user_id, theme, updated_at) VALUES (?, ?, datetime('now')) \
         ON CONFLICT(user_id) DO UPDATE SET theme = excluded.theme, updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(theme)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn load_default_theme(pool: &SqlitePool) -> Result<Option<Value>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT value FROM instance_settings WHERE key = ?")
            .bind(DEFAULT_THEME_KEY)
            .fetch_optional(pool)
            .await?;
    Ok(decode_theme(row.and_then(|(v,)| v)))
}

pub(crate) async fn save_default_theme(
    pool: &SqlitePool,
    theme: Option<&str>,
) -> Result<(), sqlx::Error> {
    match theme {
        Some(value) => {
            sqlx::query(
                "INSERT INTO instance_settings (key, value, updated_at) VALUES (?, ?, datetime('now')) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .bind(DEFAULT_THEME_KEY)
            .bind(value)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM instance_settings WHERE key = ?")
                .bind(DEFAULT_THEME_KEY)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

fn db_error(e: sqlx::Error) -> StatusCode {
    tracing::error!("Theme preference query failed: {}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn respond(pool: &SqlitePool, user_id: &str) -> Result<Json<ThemeResponse>, StatusCode> {
    Ok(Json(ThemeResponse {
        theme: load_user_theme(pool, user_id).await.map_err(db_error)?,
        instance_default: load_default_theme(pool).await.map_err(db_error)?,
    }))
}

/// GET /api/preferences/theme
pub async fn get_theme(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<ThemeResponse>, StatusCode> {
    respond(&state.db, &user.id).await
}

/// PUT /api/preferences/theme
pub async fn update_theme(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<UpdateThemeRequest>,
) -> Result<Json<ThemeResponse>, StatusCode> {
    let encoded = encode_theme(req.theme.as_ref())?;
    save_user_theme(&state.db, &user.id, encoded.as_deref())
        .await
        .map_err(db_error)?;
    respond(&state.db, &user.id).await
}

/// PUT /api/settings/default-theme (admin only)
pub async fn update_default_theme(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<UpdateThemeRequest>,
) -> Result<Json<ThemeResponse>, StatusCode> {
    if !authz::is_privileged_user(&user) {
        return Err(StatusCode::FORBIDDEN);
    }
    let encoded = encode_theme(req.theme.as_ref())?;
    save_default_theme(&state.db, encoded.as_deref())
        .await
        .map_err(db_error)?;
    respond(&state.db, &user.id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encode_rejects_non_objects_and_oversized_themes() {
        assert_eq!(encode_theme(None), Ok(None));
        assert_eq!(encode_theme(Some(&Value::Null)), Ok(None));
        assert_eq!(
            encode_theme(Some(&json!("dark"))),
            Err(StatusCode::BAD_REQUEST)
        );
        let huge = json!({ "x": "a".repeat(MAX_THEME_BYTES) });
        assert_eq!(
            encode_theme(Some(&huge)),
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        );
        assert!(encode_theme(Some(&json!({ "mode": "dark" })))
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn user_and_default_themes_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();

        assert_eq!(load_user_theme(&pool, "u1").await.unwrap(), None);
        assert_eq!(load_default_theme(&pool).await.unwrap(), None);

        save_user_theme(&pool, "u1", Some(r#"{"mode":"dark","accent":"blue"}"#))
            .await
            .unwrap();
        save_user_theme(&pool, "u1", Some(r#"{"mode":"light"}"#))
            .await
            .unwrap();
        assert_eq!(
            load_user_theme(&pool, "u1").await.unwrap(),
            Some(json!({ "mode": "light" }))
        );
        assert_eq!(load_user_theme(&pool, "u2").await.unwrap(), None);

        save_user_theme(&pool, "u1", None).await.unwrap();
        assert_eq!(load_user_theme(&pool, "u1").await.unwrap(), None);

        save_default_theme(&pool, Some(r#"{"accent":"teal"}"#))
            .await
            .unwrap();
        assert_eq!(
            load_default_theme(&pool).await.unwrap(),
            Some(json!({ "accent": "teal" }))
        );
        save_default_theme(&pool, None).await.unwrap();
        assert_eq!(load_default_theme(&pool).await.unwrap(), None);
    }
}
