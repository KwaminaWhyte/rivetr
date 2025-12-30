use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::db::{LoginRequest, LoginResponse, Session, User, UserResponse};
use crate::AppState;
use serde::{Deserialize, Serialize};

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
        "password123!", "Password123!", "Admin123!@#", "Welcome123!",
        "Qwerty123!@#", "Changeme123!", "Letmein123!@", "123456789Ab!",
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
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(&token_hash)
    .bind(&expires_at)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user: UserResponse::from(user),
    }))
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
        if header.starts_with("Bearer ") {
            header[7..].to_string()
        } else {
            header.to_string()
        }
    } else if let Some(api_key) = request.headers().get("X-API-Key").and_then(|h| h.to_str().ok()) {
        // Try X-API-Key header
        api_key.to_string()
    } else {
        // Try query parameter (for SSE/EventSource which doesn't support custom headers)
        request
            .uri()
            .query()
            .and_then(|q| {
                // Simple query string parsing: find token=value
                q.split('&')
                    .find_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        let key = parts.next()?;
                        let value = parts.next()?;
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
    if admin_token.len() == provided_token.len()
        && admin_token.ct_eq(provided_token).into()
    {
        return Ok(next.run(request).await);
    }

    // Otherwise, check for a valid session
    let token_hash = hash_token(token);
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
pub async fn setup_status(
    State(state): State<Arc<AppState>>,
) -> Json<SetupStatusResponse> {
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
    let password_hash = hash_password(&request.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash password: {}", e)))?;

    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, role) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&request.email)
    .bind(&password_hash)
    .bind(&request.name)
    .bind("admin")
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("Created admin user during setup: {}", request.email);

    // Auto-login the new user
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();

    let session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
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
        },
    }))
}

/// Extract the token from request headers
fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        if auth_header.starts_with("Bearer ") {
            return Some(auth_header[7..].to_string());
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
    // For admin token, return a system user
    if token == config.auth.admin_token {
        // Return a synthetic admin user for API token auth
        return Ok(User {
            id: "system".to_string(),
            email: "system@rivetr.local".to_string(),
            password_hash: String::new(),
            name: "System Admin".to_string(),
            role: "admin".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
    }

    // Look up session and user
    let token_hash = hash_token(token);
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
