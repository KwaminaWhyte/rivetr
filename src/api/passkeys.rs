//! Passkeys (E2): WebAuthn registration and passwordless sign in.
//!
//! Public (auth tier):
//! - `GET  /api/auth/passkeys/status`        is passkey sign in available
//! - `POST /api/auth/passkey/login/start`     discoverable login challenge
//! - `POST /api/auth/passkey/login/finish`    verify assertion, create a session
//!
//! Signed in:
//! - `GET    /api/passkeys`                   my passkeys
//! - `POST   /api/passkeys/register/start`    registration challenge
//! - `POST   /api/passkeys/register/finish`   store the new passkey
//! - `PUT    /api/passkeys/{id}`              rename
//! - `DELETE /api/passkeys/{id}`              remove
//!
//! The relying party ID and origin come from the instance domain (Settings >
//! General). Without one, the request host is used, which is what makes
//! `localhost` development work. Ceremony state lives server side in
//! `webauthn_challenges` for five minutes and is consumed on first use.
//!
//! A passkey is a phishing-resistant second factor on its own (user
//! verification is required), so a passkey login satisfies 2FA.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, Url, Uuid, Webauthn, WebauthnBuilder,
};

use crate::db::{actions, resource_types, InstanceSettings, LoginResponse, User, UserResponse};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::error::ApiError;

/// How long a registration or login ceremony may take.
pub const CHALLENGE_TTL_SECS: i64 = 300;
const PURPOSE_REGISTER: &str = "register";
const PURPOSE_LOGIN: &str = "login";
const MAX_NAME: usize = 64;

// ---------------------------------------------------------------------------
// Pure helpers (unit tested)
// ---------------------------------------------------------------------------

/// Outcome of comparing an authenticator's signature counter with the stored one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterCheck {
    /// Neither side counts (typical for synced passkeys): nothing to store.
    NotUsed,
    /// The counter moved forward: store the new value.
    Advanced(u32),
    /// The counter did not move forward: possible cloned authenticator.
    Regressed,
}

/// WebAuthn L3 6.1.1 step 21: when either counter is non-zero the new one must
/// be strictly greater than the stored one.
pub fn check_counter(stored: u32, received: u32) -> CounterCheck {
    if stored == 0 && received == 0 {
        CounterCheck::NotUsed
    } else if received > stored {
        CounterCheck::Advanced(received)
    } else {
        CounterCheck::Regressed
    }
}

/// Is a challenge that expires at `expires_at` still valid at `now` (unix seconds)?
pub fn challenge_valid(expires_at: i64, now: i64) -> bool {
    now < expires_at
}

/// Host part of a domain setting or Host header: no scheme, path or port, lowercased.
pub fn host_only(raw: &str) -> Option<String> {
    let s = raw.trim();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let s = s.split('/').next().unwrap_or("");
    // IPv6 literals cannot be a WebAuthn RP ID, so a bracketed host is rejected.
    if s.starts_with('[') {
        return None;
    }
    let host = s.split(':').next().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// Pick the relying party ID and origin.
///
/// The instance domain wins. Without one the request host is used. The origin
/// keeps the browser's `Origin` header when it belongs to the RP ID (so the
/// dashboard port and scheme match), otherwise `https://{rp_id}`.
pub fn relying_party(
    instance_domain: Option<&str>,
    request_host: Option<&str>,
    origin_header: Option<&str>,
) -> Option<(String, String)> {
    let rp_id = instance_domain
        .and_then(host_only)
        .or_else(|| request_host.and_then(host_only))?;
    if rp_id.parse::<std::net::IpAddr>().is_ok() {
        // Browsers refuse IP addresses as an RP ID.
        return None;
    }
    let origin = origin_header
        .and_then(|o| {
            let host = host_only(o)?;
            (host == rp_id || host.ends_with(&format!(".{rp_id}"))).then(|| o.trim().to_string())
        })
        .unwrap_or_else(|| {
            if rp_id == "localhost" {
                "http://localhost".to_string()
            } else {
                format!("https://{rp_id}")
            }
        });
    Some((rp_id, origin))
}

fn encode_cred_id(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn normalize_name(raw: Option<&str>) -> Result<String, ApiError> {
    let name = raw.unwrap_or("").trim();
    let name = if name.is_empty() { "Passkey" } else { name };
    if name.chars().count() > MAX_NAME {
        return Err(ApiError::validation_field(
            "name",
            format!("Name must be at most {MAX_NAME} characters"),
        ));
    }
    Ok(name.to_string())
}

// ---------------------------------------------------------------------------
// Challenge store
// ---------------------------------------------------------------------------

/// Why a stored challenge could not be used.
#[derive(Debug, PartialEq, Eq)]
pub enum ChallengeError {
    Missing,
    Expired,
}

/// Save ceremony state and return its id. Expired rows are swept on the way.
pub async fn store_challenge(
    db: &SqlitePool,
    user_id: Option<&str>,
    purpose: &str,
    state: &str,
    now: i64,
) -> Result<String, sqlx::Error> {
    sqlx::query("DELETE FROM webauthn_challenges WHERE expires_at <= ?")
        .bind(now)
        .execute(db)
        .await?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO webauthn_challenges (id, user_id, purpose, state, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id)
    .bind(purpose)
    .bind(state)
    .bind(now + CHALLENGE_TTL_SECS)
    .execute(db)
    .await?;
    Ok(id)
}

/// Remove and return ceremony state `(user_id, state)`. Single use: a second
/// call with the same id finds nothing.
pub async fn take_challenge(
    db: &SqlitePool,
    id: &str,
    purpose: &str,
    now: i64,
) -> Result<Result<(Option<String>, String), ChallengeError>, sqlx::Error> {
    let row: Option<(Option<String>, String, i64)> = sqlx::query_as(
        "DELETE FROM webauthn_challenges WHERE id = ? AND purpose = ? \
         RETURNING user_id, state, expires_at",
    )
    .bind(id)
    .bind(purpose)
    .fetch_optional(db)
    .await?;
    Ok(match row {
        None => Err(ChallengeError::Missing),
        Some((_, _, expires_at)) if !challenge_valid(expires_at, now) => {
            Err(ChallengeError::Expired)
        }
        Some((user_id, state, _)) => Ok((user_id, state)),
    })
}

fn challenge_api_error(e: ChallengeError) -> ApiError {
    match e {
        ChallengeError::Missing => {
            ApiError::bad_request("This passkey request is no longer valid. Please try again.")
        }
        ChallengeError::Expired => {
            ApiError::bad_request("This passkey request expired. Please try again.")
        }
    }
}

// ---------------------------------------------------------------------------
// Relying party
// ---------------------------------------------------------------------------

fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

fn passkeys_enabled() -> bool {
    crate::settings::advanced::current()
        .settings
        .passkeys_enabled
}

fn require_enabled() -> Result<(), ApiError> {
    if passkeys_enabled() {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "Passkeys are disabled on this instance. An admin can turn them on in Settings > Advanced.",
        ))
    }
}

async fn webauthn_for(state: &AppState, headers: &HeaderMap) -> Result<Webauthn, ApiError> {
    let domain = InstanceSettings::load(&state.db)
        .await
        .ok()
        .and_then(|s| s.instance_domain)
        .filter(|d| !d.trim().is_empty());
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|h| h.to_str().ok());
    let origin = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());
    let (rp_id, origin) = relying_party(domain.as_deref(), host, origin).ok_or_else(|| {
        ApiError::bad_request(
            "Passkeys need a domain name. Set the instance domain in Settings or open the dashboard through its domain.",
        )
    })?;
    let origin = Url::parse(&origin)
        .map_err(|_| ApiError::bad_request("Could not work out the dashboard origin"))?;
    let rp_name = crate::db::WhiteLabel::load(&state.db)
        .await
        .ok()
        .map(|w| w.app_name)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Rivetr".to_string());
    WebauthnBuilder::new(&rp_id, &origin)
        .map_err(|e| ApiError::bad_request(format!("Invalid passkey configuration: {e}")))?
        .rp_name(&rp_name)
        .allow_any_port(true)
        .build()
        .map_err(|e| ApiError::bad_request(format!("Invalid passkey configuration: {e}")))
}

fn webauthn_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::bad_request(format!("Passkey verification failed: {e}"))
}

fn json_err(e: serde_json::Error) -> ApiError {
    ApiError::internal(format!("passkey state: {e}"))
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PasskeyInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PasskeyStatus {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct RegisterStartResponse {
    pub challenge_id: String,
    /// `CreationChallengeResponse` as JSON, adjusted by [`require_discoverable`].
    pub options: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinishRequest {
    pub challenge_id: String,
    pub name: Option<String>,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Serialize)]
pub struct LoginStartResponse {
    pub challenge_id: String,
    pub options: RequestChallengeResponse,
}

#[derive(Debug, Deserialize)]
pub struct LoginFinishRequest {
    pub challenge_id: String,
    pub credential: PublicKeyCredential,
}

#[derive(Debug, Deserialize)]
pub struct RenamePasskeyRequest {
    pub name: String,
}

// ---------------------------------------------------------------------------
// Handlers: public
// ---------------------------------------------------------------------------

/// GET /api/auth/passkeys/status
pub async fn passkey_status() -> Json<PasskeyStatus> {
    Json(PasskeyStatus {
        enabled: passkeys_enabled(),
    })
}

/// POST /api/auth/passkey/login/start
pub async fn login_start(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<LoginStartResponse>, ApiError> {
    require_enabled()?;
    let webauthn = webauthn_for(&state, &headers).await?;
    let (mut options, auth_state) = webauthn
        .start_discoverable_authentication()
        .map_err(webauthn_error)?;
    // The library asks for conditional (autofill) mediation; the dashboard uses an
    // explicit "Sign in with a passkey" button, which needs the modal prompt.
    options.mediation = None;
    let state_json = serde_json::to_string(&auth_state).map_err(json_err)?;
    let challenge_id =
        store_challenge(&state.db, None, PURPOSE_LOGIN, &state_json, now_secs()).await?;
    Ok(Json(LoginStartResponse {
        challenge_id,
        options,
    }))
}

/// POST /api/auth/passkey/login/finish
pub async fn login_finish(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    client_ip: ClientIp,
    Json(req): Json<LoginFinishRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    require_enabled()?;
    let (_, state_json) = take_challenge(&state.db, &req.challenge_id, PURPOSE_LOGIN, now_secs())
        .await?
        .map_err(challenge_api_error)?;
    let auth_state: DiscoverableAuthentication =
        serde_json::from_str(&state_json).map_err(json_err)?;
    let webauthn = webauthn_for(&state, &headers).await?;

    let invalid = || ApiError::unauthorized("This passkey is not registered here");
    let (user_handle, cred_id) = webauthn
        .identify_discoverable_authentication(&req.credential)
        .map_err(|_| invalid())?;
    let cred_id = encode_cred_id(cred_id);

    let row: Option<(String, String, String, String, i64)> = sqlx::query_as(
        "SELECT id, user_id, user_handle, passkey, sign_count FROM user_passkeys WHERE credential_id = ?",
    )
    .bind(&cred_id)
    .fetch_optional(&state.db)
    .await?;
    let (passkey_id, user_id, stored_handle, passkey_json, sign_count) = row.ok_or_else(invalid)?;
    if stored_handle != user_handle.to_string() {
        return Err(invalid());
    }
    let mut passkey: Passkey = serde_json::from_str(&passkey_json).map_err(json_err)?;

    // `_v2` needs the `danger-credential-internals` feature; this is the same check.
    #[allow(deprecated)]
    let result = webauthn
        .finish_discoverable_authentication(
            &req.credential,
            auth_state,
            &[DiscoverableKey::from(&passkey)],
        )
        .map_err(|e| {
            tracing::warn!(user_id = %user_id, error = %e, "Passkey sign in rejected");
            ApiError::unauthorized("Passkey verification failed")
        })?;

    // The library already rejects a counter that went backwards; keep our own
    // check so the stored `sign_count` column can never move back either.
    match check_counter(sign_count.max(0) as u32, result.counter()) {
        CounterCheck::Regressed => {
            tracing::warn!(user_id = %user_id, passkey_id = %passkey_id, "Passkey counter regressed");
            return Err(ApiError::unauthorized(
                "This passkey may have been cloned and was rejected",
            ));
        }
        CounterCheck::Advanced(_) | CounterCheck::NotUsed => {}
    }
    passkey.update_credential(&result);
    let passkey_json = serde_json::to_string(&passkey).map_err(json_err)?;
    sqlx::query(
        "UPDATE user_passkeys SET passkey = ?, sign_count = MAX(sign_count, ?), last_used_at = ? WHERE id = ?",
    )
    .bind(&passkey_json)
    .bind(result.counter() as i64)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(&passkey_id)
    .execute(&state.db)
    .await?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(invalid)?;

    let (token, token_hash) = super::auth::generate_session_token();
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&user.id)
        .bind(&token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await?;

    audit_log(
        &state,
        actions::AUTH_LOGIN,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "method": "passkey", "passkey_id": passkey_id })),
    )
    .await;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from(user),
        requires_2fa: None,
    }))
}

// ---------------------------------------------------------------------------
// Handlers: signed in
// ---------------------------------------------------------------------------

/// GET /api/passkeys
pub async fn list_passkeys(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<PasskeyInfo>>, ApiError> {
    let rows = sqlx::query_as::<_, PasskeyInfo>(
        "SELECT id, name, created_at, last_used_at FROM user_passkeys WHERE user_id = ? ORDER BY created_at",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

fn require_real_user(user: &User) -> Result<(), ApiError> {
    if user.id == "system" {
        Err(ApiError::bad_request(
            "The admin API token cannot register passkeys; sign in as a user",
        ))
    } else {
        Ok(())
    }
}

/// POST /api/passkeys/register/start
pub async fn register_start(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
) -> Result<Json<RegisterStartResponse>, ApiError> {
    require_enabled()?;
    require_real_user(&user)?;
    let webauthn = webauthn_for(&state, &headers).await?;

    let existing: Vec<(String, String)> =
        sqlx::query_as("SELECT user_handle, passkey FROM user_passkeys WHERE user_id = ?")
            .bind(&user.id)
            .fetch_all(&state.db)
            .await?;
    let handle = existing
        .first()
        .and_then(|(h, _)| Uuid::parse_str(h).ok())
        .unwrap_or_else(Uuid::new_v4);
    let exclude: Vec<_> = existing
        .iter()
        .filter_map(|(_, p)| serde_json::from_str::<Passkey>(p).ok())
        .map(|p| p.cred_id().clone())
        .collect();

    let display = if user.name.trim().is_empty() {
        user.email.clone()
    } else {
        user.name.clone()
    };
    let (options, reg_state) = webauthn
        .start_passkey_registration(
            handle,
            &user.email,
            &display,
            (!exclude.is_empty()).then_some(exclude),
        )
        .map_err(webauthn_error)?;
    let mut options = serde_json::to_value(&options).map_err(json_err)?;
    require_discoverable(&mut options);
    let state_json = serde_json::to_string(&(handle.to_string(), reg_state)).map_err(json_err)?;
    let challenge_id = store_challenge(
        &state.db,
        Some(&user.id),
        PURPOSE_REGISTER,
        &state_json,
        now_secs(),
    )
    .await?;
    Ok(Json(RegisterStartResponse {
        challenge_id,
        options,
    }))
}

/// Sign in uses discoverable (usernameless) authentication, so the credential must
/// live on the authenticator. webauthn-rs defaults passkey registration to
/// "resident key discouraged", which security keys and some platforms honour by
/// creating a credential the login step can never find.
fn require_discoverable(options: &mut serde_json::Value) {
    let Some(public_key) = options.get_mut("publicKey").and_then(|v| v.as_object_mut()) else {
        return;
    };
    let sel = public_key
        .entry("authenticatorSelection")
        .or_insert_with(|| serde_json::json!({}));
    if let Some(sel) = sel.as_object_mut() {
        sel.insert("residentKey".into(), "required".into());
        sel.insert("requireResidentKey".into(), true.into());
    }
}

/// POST /api/passkeys/register/finish
pub async fn register_finish(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    client_ip: ClientIp,
    Json(req): Json<RegisterFinishRequest>,
) -> Result<(StatusCode, Json<PasskeyInfo>), ApiError> {
    require_enabled()?;
    require_real_user(&user)?;
    let name = normalize_name(req.name.as_deref())?;
    let (owner, state_json) =
        take_challenge(&state.db, &req.challenge_id, PURPOSE_REGISTER, now_secs())
            .await?
            .map_err(challenge_api_error)?;
    if owner.as_deref() != Some(user.id.as_str()) {
        return Err(challenge_api_error(ChallengeError::Missing));
    }
    let (handle, reg_state): (String, PasskeyRegistration) =
        serde_json::from_str(&state_json).map_err(json_err)?;
    let webauthn = webauthn_for(&state, &headers).await?;
    let passkey = webauthn
        .finish_passkey_registration(&req.credential, &reg_state)
        .map_err(webauthn_error)?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let passkey_json = serde_json::to_string(&passkey).map_err(json_err)?;
    let inserted = sqlx::query(
        "INSERT INTO user_passkeys (id, user_id, name, credential_id, user_handle, passkey, sign_count, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&name)
    .bind(encode_cred_id(passkey.cred_id().as_ref()))
    .bind(&handle)
    .bind(&passkey_json)
    .bind(&now)
    .execute(&state.db)
    .await;
    if let Err(sqlx::Error::Database(db)) = &inserted {
        if db.is_unique_violation() {
            return Err(ApiError::conflict("This passkey is already registered"));
        }
    }
    inserted?;

    audit_log(
        &state,
        actions::PASSKEY_ADD,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "passkey_id": id, "name": name })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(PasskeyInfo {
            id,
            name,
            created_at: now,
            last_used_at: None,
        }),
    ))
}

/// PUT /api/passkeys/{id}
pub async fn rename_passkey(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
    Json(req): Json<RenamePasskeyRequest>,
) -> Result<StatusCode, ApiError> {
    let name = normalize_name(Some(&req.name))?;
    let res = sqlx::query("UPDATE user_passkeys SET name = ? WHERE id = ? AND user_id = ?")
        .bind(&name)
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("Passkey not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/passkeys/{id}
pub async fn delete_passkey(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let res = sqlx::query("DELETE FROM user_passkeys WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("Passkey not found"));
    }
    audit_log(
        &state,
        actions::PASSKEY_REMOVE,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "passkey_id": id })),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Does the user have at least one passkey? (2FA enforcement accepts passkeys.)
pub async fn user_has_passkey(db: &SqlitePool, user_id: &str) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_passkeys WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(db)
        .await?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_must_strictly_increase_once_used() {
        assert_eq!(check_counter(0, 0), CounterCheck::NotUsed);
        assert_eq!(check_counter(0, 1), CounterCheck::Advanced(1));
        assert_eq!(check_counter(5, 6), CounterCheck::Advanced(6));
        assert_eq!(check_counter(5, 5), CounterCheck::Regressed);
        assert_eq!(check_counter(5, 4), CounterCheck::Regressed);
        // A counter that drops back to zero after being used is a regression too.
        assert_eq!(check_counter(5, 0), CounterCheck::Regressed);
    }

    #[test]
    fn challenge_expiry_boundary() {
        assert!(challenge_valid(100, 99));
        assert!(!challenge_valid(100, 100));
        assert!(!challenge_valid(100, 101));
    }

    #[test]
    fn relying_party_prefers_instance_domain() {
        let (id, origin) = relying_party(
            Some("https://Dash.Example.com/"),
            Some("10.0.0.5:8080"),
            Some("https://dash.example.com"),
        )
        .unwrap();
        assert_eq!(id, "dash.example.com");
        assert_eq!(origin, "https://dash.example.com");

        // Origin header from another site is ignored.
        let (_, origin) =
            relying_party(Some("dash.example.com"), None, Some("https://evil.test")).unwrap();
        assert_eq!(origin, "https://dash.example.com");
    }

    #[test]
    fn relying_party_falls_back_to_localhost_request_host() {
        let (id, origin) =
            relying_party(None, Some("localhost:5173"), Some("http://localhost:5173")).unwrap();
        assert_eq!(id, "localhost");
        assert_eq!(origin, "http://localhost:5173");
        assert!(relying_party(None, Some("127.0.0.1:8080"), None).is_none());
        assert!(relying_party(None, None, None).is_none());
    }

    async fn pool() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init(dir.path()).await.unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn challenges_are_single_use() {
        let (_dir, db) = pool().await;
        let id = store_challenge(&db, Some("u1"), PURPOSE_REGISTER, "{}", 1_000)
            .await
            .unwrap();
        // Wrong purpose does not consume it.
        assert_eq!(
            take_challenge(&db, &id, PURPOSE_LOGIN, 1_001)
                .await
                .unwrap(),
            Err(ChallengeError::Missing)
        );
        assert_eq!(
            take_challenge(&db, &id, PURPOSE_REGISTER, 1_001)
                .await
                .unwrap(),
            Ok((Some("u1".to_string()), "{}".to_string()))
        );
        assert_eq!(
            take_challenge(&db, &id, PURPOSE_REGISTER, 1_002)
                .await
                .unwrap(),
            Err(ChallengeError::Missing)
        );
    }

    #[tokio::test]
    async fn expired_challenges_are_rejected_and_swept() {
        let (_dir, db) = pool().await;
        let id = store_challenge(&db, None, PURPOSE_LOGIN, "{}", 1_000)
            .await
            .unwrap();
        assert_eq!(
            take_challenge(&db, &id, PURPOSE_LOGIN, 1_000 + CHALLENGE_TTL_SECS)
                .await
                .unwrap(),
            Err(ChallengeError::Expired)
        );

        // A stale row left behind is removed by the next store.
        store_challenge(&db, None, PURPOSE_LOGIN, "{}", 1_000)
            .await
            .unwrap();
        store_challenge(
            &db,
            None,
            PURPOSE_LOGIN,
            "{}",
            1_000 + CHALLENGE_TTL_SECS + 1,
        )
        .await
        .unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webauthn_challenges")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn registration_and_login_state_round_trip() {
        // Build a real ceremony to prove the state survives the JSON round trip.
        let origin = Url::parse("http://localhost:8080").unwrap();
        let webauthn = WebauthnBuilder::new("localhost", &origin)
            .unwrap()
            .allow_any_port(true)
            .build()
            .unwrap();
        let (ccr, reg) = webauthn
            .start_passkey_registration(Uuid::new_v4(), "a@b.c", "A", None)
            .unwrap();
        let mut options = serde_json::to_value(&ccr).unwrap();
        require_discoverable(&mut options);
        let sel = &options["publicKey"]["authenticatorSelection"];
        assert_eq!(sel["residentKey"], "required");
        assert_eq!(sel["requireResidentKey"], true);
        // The browser-facing JSON must still parse back into the webauthn-rs type.
        let _: webauthn_rs::prelude::CreationChallengeResponse =
            serde_json::from_value(options).unwrap();
        let json = serde_json::to_string(&("h".to_string(), reg)).unwrap();
        let _: (String, PasskeyRegistration) = serde_json::from_str(&json).unwrap();

        let (rcr, auth) = webauthn.start_discoverable_authentication().unwrap();
        assert!(rcr.public_key.allow_credentials.is_empty());
        let json = serde_json::to_string(&auth).unwrap();
        let _: DiscoverableAuthentication = serde_json::from_str(&json).unwrap();
    }
}
