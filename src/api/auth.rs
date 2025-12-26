use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db::{LoginRequest, LoginResponse, Session, User, UserResponse};
use crate::AppState;

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

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            // Also check for X-API-Key header
            request
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok())
                .ok_or(StatusCode::UNAUTHORIZED)?
        }
    };

    // First check if it matches the admin token from config
    if token == state.config.auth.admin_token {
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

/// Create a default admin user if none exists
pub async fn ensure_admin_user(db: &sqlx::SqlitePool, email: &str, password: &str) -> anyhow::Result<()> {
    // Check if any user exists
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?;

    if count.0 == 0 {
        let id = uuid::Uuid::new_v4().to_string();
        let password_hash = hash_password(password)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, name, role) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(&password_hash)
        .bind("Admin")
        .bind("admin")
        .execute(db)
        .await?;

        tracing::info!("Created default admin user: {}", email);
    }

    Ok(())
}
