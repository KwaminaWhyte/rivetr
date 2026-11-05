//! Structured app healthcheck settings.
//!
//! GET/PUT /api/apps/:id/healthcheck. Saving also keeps the legacy `apps.healthcheck`
//! path in sync: it holds the HTTP path while an HTTP check is enabled and is cleared
//! otherwise, so the proxy health checker follows the same settings.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::db::{HealthcheckConfig, User};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz;
use super::error::ApiError;

/// Response: the effective config, or a disabled default when none is configured.
async fn effective(state: &AppState, id: &str) -> Result<HealthcheckConfig, ApiError> {
    let row: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT healthcheck_config, healthcheck FROM apps WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let (cfg, legacy) = row.ok_or_else(|| ApiError::not_found("App not found"))?;
    Ok(
        HealthcheckConfig::resolve(cfg.as_deref(), legacy.as_deref()).unwrap_or(
            HealthcheckConfig {
                enabled: false,
                ..Default::default()
            },
        ),
    )
}

/// GET /api/apps/:id/healthcheck
pub async fn get_healthcheck(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<HealthcheckConfig>, ApiError> {
    authz::authorize_app(&state, &user, &id).await?;
    Ok(Json(effective(&state, &id).await?))
}

/// PUT /api/apps/:id/healthcheck
pub async fn update_healthcheck(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
    Json(body): Json<HealthcheckConfig>,
) -> Result<Json<HealthcheckConfig>, ApiError> {
    let app = authz::authorize_app_write(&state, &user, &id).await?;
    let cfg = body.normalized();
    cfg.validate()
        .map_err(|e| ApiError::validation_field("healthcheck", e))?;
    let json = serde_json::to_string(&cfg).map_err(|e| ApiError::internal(e.to_string()))?;
    sqlx::query(
        "UPDATE apps SET healthcheck_config = ?, healthcheck = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&json)
    .bind(cfg.proxy_path())
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&id)
    .execute(&state.db)
    .await?;

    audit_log(
        &state,
        "app.healthcheck.update",
        "app",
        Some(&id),
        Some(&app.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "enabled": cfg.enabled, "type": cfg.check_type })),
    )
    .await;

    Ok(Json(cfg))
}

/// Keep the structured config in line when an API client changes only the legacy
/// `healthcheck` path through `PUT /api/apps/:id`. An empty path disables the check.
pub async fn sync_legacy_path(db: &crate::DbPool, app_id: &str, path: &str) {
    let stored: Option<(Option<String>,)> =
        sqlx::query_as("SELECT healthcheck_config FROM apps WHERE id = ?")
            .bind(app_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
    let Some(cfg) = stored
        .and_then(|(c,)| c)
        .and_then(|c| serde_json::from_str::<HealthcheckConfig>(&c).ok())
    else {
        // No structured config yet: the legacy path is resolved on read.
        return;
    };
    let path = path.trim();
    let updated = if path.is_empty() {
        HealthcheckConfig {
            enabled: false,
            ..cfg
        }
    } else {
        HealthcheckConfig {
            enabled: true,
            check_type: crate::db::HealthcheckType::Http,
            path: path.to_string(),
            ..cfg
        }
    };
    if let Ok(json) = serde_json::to_string(&updated) {
        let _ = sqlx::query("UPDATE apps SET healthcheck_config = ? WHERE id = ?")
            .bind(json)
            .bind(app_id)
            .execute(db)
            .await;
    }
}
