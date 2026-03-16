use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::db::{
    actions, resource_types, LoginRequest, LoginResponse, Session, TeamInvitation,
    TeamMemberWithUser, User, UserResponse,
};
use crate::AppState;
use serde::{Deserialize, Serialize};

use super::audit::{audit_log, extract_client_ip};

/// Response for setup status check
#[derive(Serialize)]
pub struct SetupStatusResponse {
    pub needs_setup: bool,
}

/// Request for initial setup
#[derive(Deserialize)]
pub struct SetupRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Generate a random token
fn generate_token() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    hex::encode(bytes)
}

/// Hash a token for storage
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a session token and return (raw_token, token_hash)
/// Public so it can be used by OAuth login flow
pub fn generate_session_token() -> (String, String) {
    let token = generate_token();
    let hash = hash_token(&token);
    (token, hash)
}

/// Validate password strength
/// Returns None if valid, or Some(error_message) if invalid
fn validate_password_strength(password: &str) -> Option<String> {
    if password.len() < 12 {
        return Some("Password must be at least 12 characters".to_string());
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_uppercase {
        return Some("Password must contain at least one uppercase letter".to_string());
    }
    if !has_lowercase {
        return Some("Password must contain at least one lowercase letter".to_string());
    }
    if !has_digit {
        return Some("Password must contain at least one digit".to_string());
    }
    if !has_special {
        return Some("Password must contain at least one special character".to_string());
    }

    // Check for common weak passwords
    let common_passwords = [
        "password123!",
        "Password123!",
        "Admin123!@#",
        "Welcome123!",
        "Qwerty123!@#",
        "Changeme123!",
        "Letmein123!@",
        "123456789Ab!",
    ];
    let lower = password.to_lowercase();
    for common in common_passwords {
        if lower.contains(&common.to_lowercase()) {
            return Some("Password is too common. Please choose a stronger password.".to_string());
        }
    }

    None
}

/// Login endpoint
pub async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    // Find user by email
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
        .bind(&request.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user.ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    // Verify password
    if !verify_password(&request.password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Check if 2FA is enabled for this user
    if user.totp_enabled {
        // Create a short-lived temporary session (5 minutes) for 2FA validation
        let temp_token = generate_token();
        let temp_token_hash = hash_token(&temp_token);
        let temp_expires_at = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(5))
            .unwrap()
            .to_rfc3339();

        let temp_session_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&temp_session_id)
        .bind(&user.id)
        .bind(&temp_token_hash)
        .bind(&temp_expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        return Ok(Json(LoginResponse {
            token: temp_token,
            user: UserResponse::from(user),
            requires_2fa: Some(true),
        }));
    }

    // Generate token
    let token = generate_token();
    let token_hash = hash_token(&token);

    // Calculate expiration (7 days from now)
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();

    // Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&session_id)
        .bind(&user.id)
        .bind(&token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::AUTH_LOGIN,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from(user),
        requires_2fa: None,
    }))
}

/// Logout endpoint — invalidates the current session token
pub async fn logout(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(header) = auth_header {
        let token = header.strip_prefix("Bearer ").unwrap_or(header);

        // Delete the session from the database
        let token_hash = hash_token(token);
        let _ = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(&token_hash)
            .execute(&state.db)
            .await;
    }

    StatusCode::OK
}

/// Validate token endpoint
pub async fn validate(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => return StatusCode::UNAUTHORIZED,
    };

    // First check if it matches the admin token from config (constant-time comparison)
    let admin_token = state.config.auth.admin_token.as_bytes();
    let provided_token = token.as_bytes();
    if admin_token.len() == provided_token.len() && admin_token.ct_eq(provided_token).into() {
        return StatusCode::OK;
    }

    let token_hash = hash_token(token);

    // Check if session exists and is not expired
    let session: Option<Session> = sqlx::query_as(
        "SELECT * FROM sessions WHERE token_hash = ? AND expires_at > datetime('now')",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    match session {
        Some(_) => StatusCode::OK,
        None => StatusCode::UNAUTHORIZED,
    }
}

/// Get current authenticated user details
pub async fn me(State(state): State<Arc<AppState>>, request: Request<Body>) -> impl IntoResponse {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => header[7..].to_string(),
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    match get_current_user(&state.db, &state.config, &token).await {
        Ok(user) => Json(UserResponse::from(user)).into_response(),
        Err(status) => status.into_response(),
    }
}

/// Auth middleware that validates tokens
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    // First try Authorization header
    let token = if let Some(header) = auth_header {
        if let Some(stripped) = header.strip_prefix("Bearer ") {
            stripped.to_string()
        } else {
            header.to_string()
        }
    } else if let Some(api_key) = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
    {
        // Try X-API-Key header
        api_key.to_string()
    } else {
        // Try query parameter (for SSE/EventSource which doesn't support custom headers)
        request
            .uri()
            .query()
            .and_then(|q| {
                // Simple query string parsing: find token=value
                q.split('&').find_map(|pair| {
                    let (key, value) = pair.split_once('=')?;

                    if key == "token" {
                        Some(value.to_string())
                    } else {
                        None
                    }
                })
            })
            .ok_or(StatusCode::UNAUTHORIZED)?
    };

    let token = token.as_str();

    // First check if it matches the admin token from config
    // Use constant-time comparison to prevent timing attacks
    let admin_token = state.config.auth.admin_token.as_bytes();
    let provided_token = token.as_bytes();

    // Only compare if lengths match (constant-time check)
    if admin_token.len() == provided_token.len() && admin_token.ct_eq(provided_token).into() {
        return Ok(next.run(request).await);
    }

    let token_hash = hash_token(token);

    // Check user-created API tokens (rvt_ prefix)
    if token.starts_with("rvt_") {
        let api_token_valid: Option<String> = sqlx::query_scalar(
            "SELECT id FROM api_tokens WHERE token_hash = ? AND (expires_at IS NULL OR expires_at > datetime('now'))",
        )
        .bind(&token_hash)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if api_token_valid.is_some() {
            // Update last_used_at asynchronously (fire-and-forget)
            let db = state.db.clone();
            let hash = token_hash.clone();
            tokio::spawn(async move {
                let _ = sqlx::query(
                    "UPDATE api_tokens SET last_used_at = datetime('now') WHERE token_hash = ?",
                )
                .bind(&hash)
                .execute(&db)
                .await;
            });
            return Ok(next.run(request).await);
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Otherwise, check for a valid session
    let session: Option<Session> = sqlx::query_as(
        "SELECT * FROM sessions WHERE token_hash = ? AND expires_at > datetime('now')",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match session {
        Some(_) => Ok(next.run(request).await),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Check if initial setup is needed (no users exist)
pub async fn setup_status(State(state): State<Arc<AppState>>) -> Json<SetupStatusResponse> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Json(SetupStatusResponse {
        needs_setup: count.0 == 0,
    })
}

/// Initial setup endpoint - creates the first admin user
pub async fn setup(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<SetupRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    // Check if any user already exists
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if count.0 > 0 {
        return Err((
            StatusCode::FORBIDDEN,
            "Setup has already been completed".to_string(),
        ));
    }

    // Validate input
    if request.email.is_empty() || !request.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "Invalid email address".to_string()));
    }
    if let Some(error) = validate_password_strength(&request.password) {
        return Err((StatusCode::BAD_REQUEST, error));
    }
    if request.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }

    // Create the admin user
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(&request.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to hash password: {}", e),
        )
    })?;

    sqlx::query("INSERT INTO users (id, email, password_hash, name, role) VALUES (?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(&request.name)
        .bind("admin")
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("Created admin user during setup: {}", request.email);

    // Create a default "Personal" team for the user
    let team_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO teams (id, name, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&team_id)
    .bind("Personal")
    .bind("personal")
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Add the user as owner of the default team
    let member_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO team_members (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&member_id)
    .bind(&team_id)
    .bind(&id)
    .bind("owner")
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("Created default Personal team for user: {}", request.email);

    // Log audit event
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::AUTH_SETUP,
        resource_types::USER,
        Some(&id),
        Some(&request.email),
        Some(&id),
        ip.as_deref(),
        None,
    )
    .await;

    // Auto-login the new user
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();

    let session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&session_id)
        .bind(&id)
        .bind(&token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse {
            id,
            email: request.email,
            name: request.name,
            role: "admin".to_string(),
            totp_enabled: false,
        },
        requires_2fa: None,
    }))
}

/// Extract the token from request headers
fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
            return Some(stripped.to_string());
        }
    }

    // Fall back to X-API-Key header
    headers
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Get the current user from a token
pub async fn get_current_user(
    pool: &sqlx::SqlitePool,
    config: &crate::config::Config,
    token: &str,
) -> Result<User, StatusCode> {
    // For admin token, return the first admin user from DB (or synthetic fallback)
    if token == config.auth.admin_token {
        if let Ok(Some(admin_user)) = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE role = 'admin' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        {
            return Ok(admin_user);
        }
        // Fallback synthetic user if no admin in DB yet
        return Ok(User {
            id: "system".to_string(),
            email: "system@rivetr.local".to_string(),
            password_hash: String::new(),
            name: "System Admin".to_string(),
            role: "admin".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            totp_secret: None,
            totp_enabled: false,
            recovery_codes: None,
            oidc_subject: None,
            oidc_provider_id: None,
        });
    }

    let token_hash = hash_token(token);

    // Check DB-level API tokens first (tokens starting with "rvt_")
    if token.starts_with("rvt_") {
        let api_token_user_id: Option<String> = sqlx::query_scalar(
            "SELECT user_id FROM api_tokens WHERE token_hash = ? AND (expires_at IS NULL OR expires_at > datetime('now'))",
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(user_id) = api_token_user_id {
            // Update last_used_at
            let _ = sqlx::query(
                "UPDATE api_tokens SET last_used_at = datetime('now') WHERE token_hash = ?",
            )
            .bind(&token_hash)
            .execute(pool)
            .await;

            let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
                .bind(&user_id)
                .fetch_optional(pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return user.ok_or(StatusCode::UNAUTHORIZED);
        }
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Look up session and user
    let session: Option<Session> = sqlx::query_as(
        "SELECT * FROM sessions WHERE token_hash = ? AND expires_at > datetime('now')",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session = session.ok_or(StatusCode::UNAUTHORIZED)?;

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&session.user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    user.ok_or(StatusCode::UNAUTHORIZED)
}

/// Register a new account using a team invitation token.
///
/// POST /api/auth/register-with-invitation
///
/// Creates a new user, logs them in, and automatically accepts the invitation —
/// all in one step. Intended for invited users who don't yet have an account.
pub async fn register_with_invitation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterWithInvitationRequest>,
) -> Result<Json<RegisterWithInvitationResponse>, (StatusCode, String)> {
    // Validate invitation token
    let invitation: TeamInvitation =
        sqlx::query_as("SELECT * FROM team_invitations WHERE token = ?")
            .bind(&req.token)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::NOT_FOUND,
                "Invitation not found or already accepted".to_string(),
            ))?;

    if invitation.accepted_at.is_some() {
        return Err((
            StatusCode::GONE,
            "This invitation has already been accepted".to_string(),
        ));
    }
    if invitation.is_expired() {
        return Err((StatusCode::GONE, "This invitation has expired".to_string()));
    }

    // Validate inputs
    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }
    if let Some(err) = validate_password_strength(&req.password) {
        return Err((StatusCode::BAD_REQUEST, err));
    }

    // Make sure no account with that email exists yet
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
        .bind(&invitation.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "An account with this email already exists. Please log in instead.".to_string(),
        ));
    }

    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();

    // Create the user
    let user_id = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(&req.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to hash password: {}", e),
        )
    })?;

    sqlx::query("INSERT INTO users (id, email, password_hash, name, role) VALUES (?, ?, ?, ?, ?)")
        .bind(&user_id)
        .bind(&invitation.email)
        .bind(&password_hash)
        .bind(req.name.trim())
        .bind("member")
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Add user to the team
    let member_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO team_members (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&member_id)
    .bind(&invitation.team_id)
    .bind(&user_id)
    .bind(&invitation.role)
    .bind(&now_str)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Mark invitation as accepted
    sqlx::query("UPDATE team_invitations SET accepted_at = ? WHERE id = ?")
        .bind(&now_str)
        .bind(&invitation.id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create a session for the new user
    let session_token = generate_token();
    let token_hash = hash_token(&session_token);
    let expires_at = now
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();
    let session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&session_id)
        .bind(&user_id)
        .bind(&token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_response = UserResponse {
        id: user_id.clone(),
        email: invitation.email.clone(),
        name: req.name.trim().to_string(),
        role: "member".to_string(),
        totp_enabled: false,
    };

    let member = TeamMemberWithUser {
        id: member_id,
        team_id: invitation.team_id.clone(),
        user_id,
        role: invitation.role.clone(),
        created_at: now.to_rfc3339(),
        user_name: req.name.trim().to_string(),
        user_email: invitation.email.clone(),
    };

    tracing::info!(
        email = %invitation.email,
        team_id = %invitation.team_id,
        "New user registered via invitation"
    );

    Ok(Json(RegisterWithInvitationResponse {
        token: session_token,
        user: user_response,
        member,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RegisterWithInvitationRequest {
    pub token: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterWithInvitationResponse {
    pub token: String,
    pub user: UserResponse,
    pub member: TeamMemberWithUser,
}

/// Extractor for getting the current authenticated user from a request
#[async_trait]
impl FromRequestParts<Arc<AppState>> for User {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(&parts.headers).ok_or(StatusCode::UNAUTHORIZED)?;
        get_current_user(&state.db, &state.config, &token).await
    }
}
