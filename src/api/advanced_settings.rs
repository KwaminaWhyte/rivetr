//! Instance-level settings pages: Advanced, Updates, Email, public IPs.
//!
//! Everything here is admin only except the destructive-confirmation mode,
//! which every dashboard user needs to render confirm dialogs.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::User;
use crate::settings::advanced::{AdvancedSettings, SaveError, UpdateAdvancedSettings};
use crate::settings::email::{EmailSettingsView, UpdateEmailSettings};
use crate::settings::updates::{UpdateSettings, UpdateUpdateSettings};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz::is_privileged_user;
use super::error::ApiError;

fn require_admin(user: &User) -> Result<(), ApiError> {
    if is_privileged_user(user) {
        Ok(())
    } else {
        Err(ApiError::forbidden("Admin access required"))
    }
}

fn encryption_key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_deref()
        .map(crate::crypto::derive_key)
}

async fn audit_settings_change(state: &AppState, user: &User, ip: &ClientIp, what: &str) {
    audit_log(
        state,
        "settings.update",
        "instance_settings",
        Some(what),
        Some(what),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
}

// ---------------------------------------------------------------------------
// Advanced
// ---------------------------------------------------------------------------

/// GET /api/settings/advanced
pub async fn get_advanced(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<AdvancedSettings>, ApiError> {
    require_admin(&user)?;
    let settings = crate::settings::advanced::reload(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(Json(settings))
}

/// PUT /api/settings/advanced
pub async fn update_advanced(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<UpdateAdvancedSettings>,
) -> Result<Json<AdvancedSettings>, ApiError> {
    require_admin(&user)?;
    let settings = AdvancedSettings::save(&state.db, &req)
        .await
        .map_err(|e| match e {
            SaveError::Invalid(msg) => ApiError::bad_request(msg),
            SaveError::Db(e) => ApiError::database(e.to_string()),
        })?;
    audit_settings_change(&state, &user, &client_ip, "advanced").await;
    Ok(Json(settings))
}

#[derive(Serialize)]
pub struct ConfirmationModeResponse {
    pub mode: crate::settings::advanced::ConfirmationMode,
}

/// GET /api/settings/destructive-confirmation (any signed-in user)
pub async fn get_confirmation_mode() -> Json<ConfirmationModeResponse> {
    Json(ConfirmationModeResponse {
        mode: crate::settings::advanced::current()
            .settings
            .destructive_confirmation,
    })
}

/// Gate for `/mcp`: answers 404 while the MCP server is turned off.
pub async fn mcp_gate(request: Request<Body>, next: Next) -> Response {
    if !crate::settings::advanced::current().settings.mcp_enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "code": "mcp_disabled",
                    "message": "The MCP server is disabled. An admin can turn it on in Settings > Advanced."
                }
            })),
        )
            .into_response();
    }
    next.run(request).await
}

// ---------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------

/// GET /api/settings/updates
pub async fn get_update_settings(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<UpdateSettings>, ApiError> {
    require_admin(&user)?;
    Ok(Json(state.update_checker.settings()))
}

/// PUT /api/settings/updates
pub async fn update_update_settings(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<UpdateUpdateSettings>,
) -> Result<Json<UpdateSettings>, ApiError> {
    require_admin(&user)?;
    let settings = UpdateSettings::save(&state.db, state.update_checker.toml_config(), &req)
        .await
        .map_err(ApiError::bad_request)?;
    state.update_checker.set_settings(settings.clone());
    audit_settings_change(&state, &user, &client_ip, "updates").await;
    Ok(Json(settings))
}

// ---------------------------------------------------------------------------
// Transactional email
// ---------------------------------------------------------------------------

async fn email_view(state: &AppState) -> Result<EmailSettingsView, ApiError> {
    let map = crate::db::load_setting_map(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    let key = encryption_key(state);
    Ok(crate::settings::email::view(
        &map,
        &state.config.email,
        key.as_ref(),
    ))
}

/// GET /api/settings/email
pub async fn get_email_settings(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<EmailSettingsView>, ApiError> {
    require_admin(&user)?;
    Ok(Json(email_view(&state).await?))
}

/// PUT /api/settings/email
pub async fn update_email_settings(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<UpdateEmailSettings>,
) -> Result<Json<EmailSettingsView>, ApiError> {
    require_admin(&user)?;
    let key = encryption_key(&state);
    crate::settings::email::save(&state.db, &req, key.as_ref())
        .await
        .map_err(ApiError::bad_request)?;
    audit_settings_change(&state, &user, &client_ip, "email").await;
    Ok(Json(email_view(&state).await?))
}

#[derive(Deserialize)]
pub struct TestEmailRequest {
    /// Recipient. Defaults to the signed-in user's email.
    pub to: Option<String>,
}

#[derive(Serialize)]
pub struct TestEmailResponse {
    pub sent_to: String,
}

/// POST /api/settings/email/test
pub async fn send_test_email(
    State(state): State<Arc<AppState>>,
    user: User,
    Json(req): Json<TestEmailRequest>,
) -> Result<Json<TestEmailResponse>, ApiError> {
    require_admin(&user)?;
    let to = req
        .to
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| user.email.clone());
    if to.parse::<lettre::Address>().is_err() {
        return Err(ApiError::bad_request(format!(
            "'{to}' is not a valid email address"
        )));
    }
    let key = encryption_key(&state);
    let cfg = crate::settings::email::load_effective(&state.db, &state.config.email, key.as_ref())
        .await
        .ok_or_else(|| {
            ApiError::bad_request("Email is not configured. Fill in and save the settings first.")
        })?;
    let text = "This is a test email from Rivetr. Your transactional email settings work.";
    let html = format!("<p>{text}</p>");
    crate::settings::email::send(&cfg, &to, "Rivetr test email", &html, text)
        .await
        .map_err(|e| ApiError::bad_request(format!("Sending failed: {e:#}")))?;
    Ok(Json(TestEmailResponse { sent_to: to }))
}

// ---------------------------------------------------------------------------
// Public IPs (General page)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct PublicIps {
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
}

/// GET /api/settings/public-ip
pub async fn get_public_ips(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<PublicIps>, ApiError> {
    require_admin(&user)?;
    let (ipv4, ipv6) = crate::settings::advanced::public_ips(&state.db).await;
    Ok(Json(PublicIps { ipv4, ipv6 }))
}

/// PUT /api/settings/public-ip (empty string clears a value, missing keeps it)
pub async fn update_public_ips(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<PublicIps>,
) -> Result<Json<PublicIps>, ApiError> {
    require_admin(&user)?;
    crate::settings::save_public_ips(&state.db, req.ipv4.as_deref(), req.ipv6.as_deref())
        .await
        .map_err(ApiError::bad_request)?;
    audit_settings_change(&state, &user, &client_ip, "public_ip").await;
    let (ipv4, ipv6) = crate::settings::advanced::public_ips(&state.db).await;
    Ok(Json(PublicIps { ipv4, ipv6 }))
}

/// POST /api/settings/public-ip/detect (does not save)
pub async fn detect_public_ips(user: User) -> Result<Json<PublicIps>, ApiError> {
    require_admin(&user)?;
    let (ipv4, ipv6) = crate::settings::detect_public_ips().await;
    Ok(Json(PublicIps { ipv4, ipv6 }))
}
