//! API token management endpoints and token resolution.
//!
//! Users create named API tokens for programmatic access. Tokens are stored as
//! SHA-256 hashes; the plaintext is shown once on creation and once per roll.
//!
//! A token carries scopes (`api/scopes.rs`), an optional restriction to one team
//! and/or to specific projects/resources, an optional expiry, and can be paused
//! (rejected until resumed) or rolled (new secret, the old one stays valid for a
//! short grace period).

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::audit::{audit_log, ClientIp};
use super::error::ApiError;
use super::optional_json::OptionalJson;
use super::scopes::{full_scopes, parse_scopes, scopes_to_csv, ScopeSet, TokenScope};
use crate::{
    db::{actions, resource_types, User},
    AppState,
};

/// Default and maximum grace period for the previous secret after a roll.
pub const DEFAULT_ROLL_GRACE_MINUTES: u32 = 60;
pub const MAX_ROLL_GRACE_MINUTES: u32 = 7 * 24 * 60;

/// Resource types a token can be restricted to.
pub const RESTRICTABLE_TYPES: [&str; 4] = ["project", "app", "database", "service"];

// ── Models ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiTokenRow {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub user_id: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub scopes: String,
    pub team_id: Option<String>,
    pub resources: Option<String>,
    pub disabled: i64,
    pub last_used_ip: Option<String>,
    pub token_hint: Option<String>,
    pub previous_token_hash: Option<String>,
    pub previous_expires_at: Option<String>,
    pub rolled_at: Option<String>,
}

/// A resource a token is restricted to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenResource {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
}

/// The resolved, validated identity of an API token for one request. Inserted
/// as a request extension by `auth_middleware`.
#[derive(Debug, Clone)]
pub struct TokenContext {
    pub token_id: String,
    pub user_id: String,
    pub scopes: ScopeSet,
    pub team_id: Option<String>,
    pub resources: Vec<TokenResource>,
}

impl TokenContext {
    /// Restricted to a team or to specific resources.
    pub fn is_restricted(&self) -> bool {
        self.team_id.is_some() || !self.resources.is_empty()
    }
}

/// Why a presented token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRejection {
    Disabled,
    Expired,
    /// The previous secret of a rolled token after its grace period.
    GraceExpired,
    /// The hash matches neither the current nor the previous secret.
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct ApiTokenResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub last_used_ip: Option<String>,
    pub expires_at: Option<String>,
    pub scopes: Vec<String>,
    pub team_id: Option<String>,
    pub resources: Vec<TokenResource>,
    pub disabled: bool,
    /// Last four characters of the current secret.
    pub token_hint: Option<String>,
    pub rolled_at: Option<String>,
    /// When the previous secret stops working after a roll (if still in grace).
    pub previous_expires_at: Option<String>,
    /// Only populated on creation and roll, never returned again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl From<ApiTokenRow> for ApiTokenResponse {
    fn from(t: ApiTokenRow) -> Self {
        let now = Utc::now();
        let previous_expires_at = t
            .previous_expires_at
            .filter(|e| parse_ts(e).is_some_and(|e| e > now));
        Self {
            id: t.id,
            name: t.name,
            created_at: t.created_at,
            last_used_at: t.last_used_at,
            last_used_ip: t.last_used_ip,
            expires_at: t.expires_at,
            scopes: parse_scopes(&t.scopes)
                .iter()
                .map(|s| s.as_str().to_string())
                .collect(),
            team_id: t.team_id,
            resources: parse_resources(t.resources.as_deref()),
            disabled: t.disabled != 0,
            token_hint: t.token_hint,
            rolled_at: t.rolled_at,
            previous_expires_at,
            token: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiTokenRequest {
    pub name: String,
    /// Optional ISO-8601 expiry date or timestamp.
    pub expires_at: Option<String>,
    /// Scopes; omitted means every scope (the pre-scopes behaviour).
    pub scopes: Option<Vec<String>>,
    /// Restrict the token to one team.
    pub team_id: Option<String>,
    /// Restrict the token to these projects/resources.
    pub resources: Option<Vec<TokenResource>>,
}

/// PATCH body. Absent fields are unchanged; `expires_at: ""` and
/// `team_id: ""` clear the value, `resources: []` lifts the restriction.
#[derive(Debug, Deserialize)]
pub struct UpdateApiTokenRequest {
    pub name: Option<String>,
    pub expires_at: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub team_id: Option<String>,
    pub resources: Option<Vec<TokenResource>>,
}

#[derive(Debug, Deserialize)]
pub struct RollApiTokenRequest {
    /// Minutes the previous secret keeps working (0 revokes it immediately).
    pub grace_minutes: Option<u32>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

fn generate_token() -> String {
    // 32 random bytes → 64 hex chars, prefixed with "rvt_" for easy identification
    let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
    format!("rvt_{}", hex::encode(bytes))
}

fn hint(token: &str) -> String {
    token[token.len().saturating_sub(4)..].to_string()
}

/// Parse the timestamp formats found in the table: RFC 3339, SQLite
/// `datetime('now')` (`YYYY-MM-DD HH:MM:SS`, UTC) and a bare date (midnight UTC).
pub fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M"] {
        if let Ok(t) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(t.and_utc());
        }
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|t| t.and_utc())
}

pub fn parse_resources(json: Option<&str>) -> Vec<TokenResource> {
    json.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// Validate a presented token against its row. Pure so it can be unit tested.
pub fn evaluate_token(
    row: &ApiTokenRow,
    presented_hash: &str,
    now: DateTime<Utc>,
) -> Result<TokenContext, TokenRejection> {
    if row.token_hash != presented_hash {
        match (&row.previous_token_hash, &row.previous_expires_at) {
            (Some(prev), Some(until)) if prev == presented_hash => {
                if parse_ts(until).is_none_or(|u| u <= now) {
                    return Err(TokenRejection::GraceExpired);
                }
            }
            (Some(prev), None) if prev == presented_hash => {
                return Err(TokenRejection::GraceExpired)
            }
            _ => return Err(TokenRejection::Unknown),
        }
    }
    if row.disabled != 0 {
        return Err(TokenRejection::Disabled);
    }
    if let Some(exp) = row.expires_at.as_deref().filter(|e| !e.trim().is_empty()) {
        // An unparseable expiry fails closed.
        if parse_ts(exp).is_none_or(|e| e <= now) {
            return Err(TokenRejection::Expired);
        }
    }
    Ok(TokenContext {
        token_id: row.id.clone(),
        user_id: row.user_id.clone(),
        scopes: parse_scopes(&row.scopes),
        team_id: row.team_id.clone().filter(|t| !t.is_empty()),
        resources: parse_resources(row.resources.as_deref()),
    })
}

/// Resolve a plaintext `rvt_` token to its context. `Ok(None)` for unknown,
/// paused, expired and out-of-grace tokens.
pub async fn resolve_api_token(
    db: &SqlitePool,
    token: &str,
) -> Result<Option<TokenContext>, sqlx::Error> {
    let hash = hash_token(token);
    let row: Option<ApiTokenRow> = sqlx::query_as(
        "SELECT * FROM api_tokens WHERE token_hash = ? OR previous_token_hash = ? LIMIT 1",
    )
    .bind(&hash)
    .bind(&hash)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    match evaluate_token(&row, &hash, Utc::now()) {
        Ok(ctx) => Ok(Some(ctx)),
        Err(reason) => {
            tracing::debug!(token_id = %row.id, ?reason, "Rejected API token");
            Ok(None)
        }
    }
}

/// Record last use (time and client IP). Fire and forget.
pub fn record_token_use(db: &SqlitePool, token_id: &str, ip: Option<String>) {
    let db = db.clone();
    let id = token_id.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "UPDATE api_tokens SET last_used_at = datetime('now'), \
             last_used_ip = COALESCE(?, last_used_ip) WHERE id = ?",
        )
        .bind(ip)
        .bind(&id)
        .execute(&db)
        .await;
    });
}

fn validate_scopes(input: &[String]) -> Result<ScopeSet, ApiError> {
    let mut set = ScopeSet::new();
    for s in input {
        let scope = TokenScope::parse(s).ok_or_else(|| {
            ApiError::validation_field(
                "scopes",
                format!("Unknown scope `{s}`; use read, deploy, write, delete, secrets or admin"),
            )
        })?;
        set.insert(scope);
    }
    if set.is_empty() {
        return Err(ApiError::validation_field(
            "scopes",
            "A token needs at least one scope",
        ));
    }
    Ok(set)
}

fn validate_expiry(expires_at: &str) -> Result<(), ApiError> {
    match parse_ts(expires_at) {
        Some(t) if t > Utc::now() => Ok(()),
        Some(_) => Err(ApiError::validation_field(
            "expires_at",
            "Expiry must be in the future",
        )),
        None => Err(ApiError::validation_field(
            "expires_at",
            "Expiry must be an ISO-8601 date or timestamp",
        )),
    }
}

/// The caller may restrict a token to a team they belong to and to resources
/// they can access.
async fn validate_restrictions(
    state: &Arc<AppState>,
    user: &User,
    team_id: Option<&str>,
    resources: &[TokenResource],
) -> Result<(), ApiError> {
    if let Some(tid) = team_id {
        super::authz::authorize_token_team(state, user, tid).await?;
    }
    if resources.len() > 100 {
        return Err(ApiError::validation_field(
            "resources",
            "A token can be restricted to at most 100 resources",
        ));
    }
    for r in resources {
        match r.kind.as_str() {
            "project" => super::authz::authorize_project(state, user, &r.id)
                .await
                .map(|_| ())?,
            "app" => super::authz::authorize_app(state, user, &r.id)
                .await
                .map(|_| ())?,
            "database" => super::authz::authorize_database(state, user, &r.id)
                .await
                .map(|_| ())?,
            "service" => super::authz::authorize_service(state, user, &r.id)
                .await
                .map(|_| ())?,
            other => {
                return Err(ApiError::validation_field(
                    "resources",
                    format!(
                        "Unknown resource type `{other}`; use project, app, database or service"
                    ),
                ))
            }
        }
    }
    Ok(())
}

fn resources_json(resources: &[TokenResource]) -> Option<String> {
    if resources.is_empty() {
        None
    } else {
        serde_json::to_string(resources).ok()
    }
}

async fn load_own_token(
    state: &Arc<AppState>,
    user: &User,
    id: &str,
) -> Result<ApiTokenRow, ApiError> {
    sqlx::query_as("SELECT * FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::not_found("Token not found"))
}

fn reject_system(user: &User) -> Result<(), ApiError> {
    if user.id == "system" {
        return Err(ApiError::forbidden(
            "The admin API token from the config has no token list; sign in to manage tokens",
        ));
    }
    Ok(())
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// List all API tokens for the current user.
///
/// GET /api/tokens
pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<ApiTokenResponse>>, ApiError> {
    // System user (admin API token) has no DB record — return empty list
    if user.id == "system" {
        return Ok(Json(vec![]));
    }

    let tokens: Vec<ApiTokenRow> =
        sqlx::query_as("SELECT * FROM api_tokens WHERE user_id = ? ORDER BY created_at DESC")
            .bind(&user.id)
            .fetch_all(&state.db)
            .await?;

    Ok(Json(tokens.into_iter().map(Into::into).collect()))
}

/// Create a new API token.
///
/// POST /api/tokens
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    reject_system(&user)?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::validation_field(
            "name",
            "Token name cannot be empty",
        ));
    }
    let scopes = match &req.scopes {
        Some(s) => validate_scopes(s)?,
        None => full_scopes(),
    };
    let expires_at = req.expires_at.filter(|e| !e.trim().is_empty());
    if let Some(e) = &expires_at {
        validate_expiry(e)?;
    }
    let team_id = req.team_id.filter(|t| !t.is_empty());
    let resources = req.resources.unwrap_or_default();
    validate_restrictions(&state, &user, team_id.as_deref(), &resources).await?;

    let plaintext = generate_token();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO api_tokens (id, name, token_hash, user_id, created_at, expires_at, \
         scopes, team_id, resources, token_hint) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(hash_token(&plaintext))
    .bind(&user.id)
    .bind(&now)
    .bind(&expires_at)
    .bind(scopes_to_csv(&scopes))
    .bind(&team_id)
    .bind(resources_json(&resources))
    .bind(hint(&plaintext))
    .execute(&state.db)
    .await?;

    audit_log(
        &state,
        actions::TOKEN_CREATE,
        resource_types::TOKEN,
        Some(&id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({
            "scopes": scopes_to_csv(&scopes),
            "team_id": team_id,
            "resources": resources.len(),
        })),
    )
    .await;

    let row = load_own_token(&state, &user, &id).await?;
    let mut resp = ApiTokenResponse::from(row);
    resp.token = Some(plaintext);
    Ok(Json(resp))
}

/// Change a token's name, scopes, expiry or restrictions.
///
/// PATCH /api/tokens/:id
pub async fn update_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<UpdateApiTokenRequest>,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    reject_system(&user)?;
    let row = load_own_token(&state, &user, &id).await?;

    let name = match req.name {
        Some(n) if n.trim().is_empty() => {
            return Err(ApiError::validation_field(
                "name",
                "Token name cannot be empty",
            ))
        }
        Some(n) => n.trim().to_string(),
        None => row.name.clone(),
    };
    let scopes = match &req.scopes {
        Some(s) => scopes_to_csv(&validate_scopes(s)?),
        None => row.scopes.clone(),
    };
    let expires_at = match req.expires_at {
        Some(e) if e.trim().is_empty() => None,
        Some(e) => {
            validate_expiry(&e)?;
            Some(e)
        }
        None => row.expires_at.clone(),
    };
    let team_id = match req.team_id {
        Some(t) if t.is_empty() => None,
        Some(t) => Some(t),
        None => row.team_id.clone(),
    };
    let resources = match req.resources {
        Some(r) => r,
        None => parse_resources(row.resources.as_deref()),
    };
    validate_restrictions(&state, &user, team_id.as_deref(), &resources).await?;

    sqlx::query(
        "UPDATE api_tokens SET name = ?, scopes = ?, expires_at = ?, team_id = ?, resources = ? \
         WHERE id = ? AND user_id = ?",
    )
    .bind(&name)
    .bind(&scopes)
    .bind(&expires_at)
    .bind(&team_id)
    .bind(resources_json(&resources))
    .bind(&id)
    .bind(&user.id)
    .execute(&state.db)
    .await?;

    audit_log(
        &state,
        actions::TOKEN_UPDATE,
        resource_types::TOKEN,
        Some(&id),
        Some(&name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "scopes": scopes, "team_id": team_id })),
    )
    .await;

    Ok(Json(load_own_token(&state, &user, &id).await?.into()))
}

async fn set_disabled(
    state: Arc<AppState>,
    id: String,
    user: User,
    client_ip: ClientIp,
    disabled: bool,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    reject_system(&user)?;
    let row = load_own_token(&state, &user, &id).await?;
    sqlx::query("UPDATE api_tokens SET disabled = ? WHERE id = ? AND user_id = ?")
        .bind(disabled as i64)
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;
    audit_log(
        &state,
        if disabled {
            actions::TOKEN_PAUSE
        } else {
            actions::TOKEN_RESUME
        },
        resource_types::TOKEN,
        Some(&id),
        Some(&row.name),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;
    Ok(Json(load_own_token(&state, &user, &id).await?.into()))
}

/// Pause a token: requests with it are rejected until it is resumed.
///
/// POST /api/tokens/:id/pause
pub async fn pause_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    set_disabled(state, id, user, client_ip, true).await
}

/// Resume a paused token.
///
/// POST /api/tokens/:id/resume
pub async fn resume_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    set_disabled(state, id, user, client_ip, false).await
}

/// Roll a token: issue a new secret (shown once). The previous secret keeps
/// working for `grace_minutes` (default 60, 0 revokes it immediately).
///
/// POST /api/tokens/:id/roll
pub async fn roll_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
    OptionalJson(req): OptionalJson<RollApiTokenRequest>,
) -> Result<Json<ApiTokenResponse>, ApiError> {
    reject_system(&user)?;
    let row = load_own_token(&state, &user, &id).await?;
    let grace = req
        .and_then(|r| r.grace_minutes)
        .unwrap_or(DEFAULT_ROLL_GRACE_MINUTES);
    if grace > MAX_ROLL_GRACE_MINUTES {
        return Err(ApiError::validation_field(
            "grace_minutes",
            format!("Grace period can be at most {MAX_ROLL_GRACE_MINUTES} minutes"),
        ));
    }
    let now = Utc::now();
    let (previous_hash, previous_until) = if grace == 0 {
        (None, None)
    } else {
        (
            Some(row.token_hash.clone()),
            Some((now + chrono::Duration::minutes(grace as i64)).to_rfc3339()),
        )
    };

    let plaintext = generate_token();
    sqlx::query(
        "UPDATE api_tokens SET token_hash = ?, token_hint = ?, previous_token_hash = ?, \
         previous_expires_at = ?, rolled_at = ? WHERE id = ? AND user_id = ?",
    )
    .bind(hash_token(&plaintext))
    .bind(hint(&plaintext))
    .bind(&previous_hash)
    .bind(&previous_until)
    .bind(now.to_rfc3339())
    .bind(&id)
    .bind(&user.id)
    .execute(&state.db)
    .await?;

    audit_log(
        &state,
        actions::TOKEN_ROLL,
        resource_types::TOKEN,
        Some(&id),
        Some(&row.name),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "grace_minutes": grace })),
    )
    .await;

    let mut resp = ApiTokenResponse::from(load_own_token(&state, &user, &id).await?);
    resp.token = Some(plaintext);
    Ok(Json(resp))
}

/// Delete an API token.
///
/// DELETE /api/tokens/:id
pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<Json<serde_json::Value>, ApiError> {
    if user.id == "system" {
        return Err(ApiError::forbidden(
            "Cannot delete tokens for the admin API token user",
        ));
    }

    // Capture name before delete for the audit log entry.
    let name: Option<String> =
        sqlx::query_scalar("SELECT name FROM api_tokens WHERE id = ? AND user_id = ?")
            .bind(&id)
            .bind(&user.id)
            .fetch_optional(&state.db)
            .await?;

    let result = sqlx::query("DELETE FROM api_tokens WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Token not found"));
    }

    audit_log(
        &state,
        actions::TOKEN_DELETE,
        resource_types::TOKEN,
        Some(&id),
        name.as_deref(),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "message": "Token deleted" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(hash: &str) -> ApiTokenRow {
        ApiTokenRow {
            id: "t1".into(),
            name: "ci".into(),
            token_hash: hash.into(),
            user_id: "u1".into(),
            created_at: "2026-01-01 00:00:00".into(),
            last_used_at: None,
            expires_at: None,
            scopes: "read,deploy".into(),
            team_id: None,
            resources: None,
            disabled: 0,
            last_used_ip: None,
            token_hint: None,
            previous_token_hash: None,
            previous_expires_at: None,
            rolled_at: None,
        }
    }

    fn now() -> DateTime<Utc> {
        parse_ts("2026-06-01T12:00:00Z").unwrap()
    }

    #[test]
    fn valid_token_resolves_scopes() {
        let ctx = evaluate_token(&row("h"), "h", now()).unwrap();
        assert_eq!(ctx.user_id, "u1");
        assert!(ctx.scopes.contains(&TokenScope::Deploy));
        assert!(!ctx.scopes.contains(&TokenScope::Write));
        assert!(!ctx.is_restricted());
        assert_eq!(
            evaluate_token(&row("h"), "other", now()).unwrap_err(),
            TokenRejection::Unknown
        );
    }

    #[test]
    fn paused_token_is_rejected_and_resumed_token_works() {
        let mut r = row("h");
        r.disabled = 1;
        assert_eq!(
            evaluate_token(&r, "h", now()).unwrap_err(),
            TokenRejection::Disabled
        );
        r.disabled = 0;
        assert!(evaluate_token(&r, "h", now()).is_ok());
    }

    #[test]
    fn expiry_in_any_stored_format() {
        let mut r = row("h");
        r.expires_at = Some("2026-05-31".into());
        assert_eq!(
            evaluate_token(&r, "h", now()).unwrap_err(),
            TokenRejection::Expired
        );
        r.expires_at = Some("2026-06-01 11:59:59".into());
        assert_eq!(
            evaluate_token(&r, "h", now()).unwrap_err(),
            TokenRejection::Expired
        );
        r.expires_at = Some("2026-06-02T00:00:00Z".into());
        assert!(evaluate_token(&r, "h", now()).is_ok());
        r.expires_at = Some("not a date".into());
        assert_eq!(
            evaluate_token(&r, "h", now()).unwrap_err(),
            TokenRejection::Expired
        );
    }

    #[test]
    fn rolled_token_old_secret_works_only_during_grace() {
        let mut r = row("new");
        r.previous_token_hash = Some("old".into());
        r.previous_expires_at = Some("2026-06-01T13:00:00Z".into());
        // Both secrets work inside the grace period.
        assert!(evaluate_token(&r, "new", now()).is_ok());
        assert!(evaluate_token(&r, "old", now()).is_ok());
        // After it, only the new one.
        let later = parse_ts("2026-06-01T13:00:01Z").unwrap();
        assert!(evaluate_token(&r, "new", later).is_ok());
        assert_eq!(
            evaluate_token(&r, "old", later).unwrap_err(),
            TokenRejection::GraceExpired
        );
        // Pausing blocks the old secret too.
        r.disabled = 1;
        assert_eq!(
            evaluate_token(&r, "old", now()).unwrap_err(),
            TokenRejection::Disabled
        );
        // Grace 0: previous hash kept without a deadline is never valid.
        let mut r = row("new");
        r.previous_token_hash = Some("old".into());
        assert_eq!(
            evaluate_token(&r, "old", now()).unwrap_err(),
            TokenRejection::GraceExpired
        );
    }

    #[test]
    fn restrictions_parse() {
        let mut r = row("h");
        r.team_id = Some("team-1".into());
        r.resources = Some(r#"[{"type":"project","id":"p1"}]"#.into());
        let ctx = evaluate_token(&r, "h", now()).unwrap();
        assert!(ctx.is_restricted());
        assert_eq!(
            ctx.resources,
            vec![TokenResource {
                kind: "project".into(),
                id: "p1".into()
            }]
        );
        // Corrupt JSON never widens access: it parses to "no list", and the
        // team restriction still applies.
        r.resources = Some("{".into());
        assert!(evaluate_token(&r, "h", now()).unwrap().resources.is_empty());
    }

    #[tokio::test]
    async fn resolve_against_a_real_table() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE api_tokens (id TEXT, name TEXT, token_hash TEXT, user_id TEXT, \
             created_at TEXT, last_used_at TEXT, expires_at TEXT, scopes TEXT NOT NULL DEFAULT \
             'read,deploy,write,delete,secrets,admin', team_id TEXT, resources TEXT, disabled \
             INTEGER NOT NULL DEFAULT 0, last_used_ip TEXT, token_hint TEXT, \
             previous_token_hash TEXT, previous_expires_at TEXT, rolled_at TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        // A pre-migration token: only the old columns are set, scopes defaults to full.
        sqlx::query(
            "INSERT INTO api_tokens (id, name, token_hash, user_id, created_at) \
             VALUES ('t1', 'old', ?, 'u1', datetime('now'))",
        )
        .bind(hash_token("rvt_abc"))
        .execute(&pool)
        .await
        .unwrap();
        let ctx = resolve_api_token(&pool, "rvt_abc").await.unwrap().unwrap();
        assert_eq!(ctx.scopes, full_scopes());
        assert!(resolve_api_token(&pool, "rvt_nope")
            .await
            .unwrap()
            .is_none());

        sqlx::query("UPDATE api_tokens SET disabled = 1")
            .execute(&pool)
            .await
            .unwrap();
        assert!(resolve_api_token(&pool, "rvt_abc").await.unwrap().is_none());
    }
}
