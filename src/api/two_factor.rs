//! Two-Factor Authentication (2FA) API endpoints.
//!
//! Provides TOTP-based 2FA setup, verification, validation, and disabling.
//! Compatible with Google Authenticator, Authy, and other TOTP apps.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::crypto;
use crate::db::User;
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};
use crate::db::{actions, resource_types};

use super::auth::verify_password;

// ── Request / Response types ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct VerifyCodeRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub session_token: String,
    pub code: String,
}

#[derive(Deserialize)]
pub struct DisableRequest {
    /// Either a current TOTP code or the user's password
    pub code: String,
}

#[derive(Serialize)]
pub struct SetupResponse {
    pub secret: String,
    pub qr_code_url: String,
    pub qr_code_svg: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub recovery_codes: Vec<String>,
}

#[derive(Serialize)]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Get the encryption key from the app state config.
fn get_encryption_key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_ref()
        .map(|k| crypto::derive_key(k))
}

/// Build a TOTP instance from a base32 secret.
fn build_totp(secret_base32: &str, email: &str) -> Result<TOTP, (StatusCode, String)> {
    let secret_bytes = Secret::Encoded(secret_base32.to_string())
        .to_bytes()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid TOTP secret: {}", e),
            )
        })?;

    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("Rivetr".to_string()),
        email.to_string(),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create TOTP: {}", e),
        )
    })
}

/// Generate 10 random recovery codes (8-char alphanumeric).
fn generate_recovery_codes() -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::rng();
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..10)
        .map(|_| {
            (0..8)
                .map(|_| chars[rng.random_range(0..chars.len())] as char)
                .collect()
        })
        .collect()
}

/// Hash a recovery code for storage (SHA-256).
fn hash_recovery_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.to_uppercase().as_bytes());
    hex::encode(hasher.finalize())
}

/// Hash a token for session lookup.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// ── Endpoints ─────────────────────────────────────────────────────────────

/// POST /api/auth/2fa/setup
/// Start 2FA setup for the authenticated user. Generates a TOTP secret and QR code.
pub async fn setup_2fa(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    // Don't allow setup if 2FA is already enabled
    if user.totp_enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            "Two-factor authentication is already enabled".to_string(),
        ));
    }

    // Generate a random TOTP secret
    let secret = Secret::generate_secret();
    let secret_base32 = secret.to_encoded().to_string();

    // Build the TOTP to generate QR code URL
    let totp = build_totp(&secret_base32, &user.email)?;
    let qr_code_url = totp.get_url();
    let qr_code_svg = totp.get_qr_base64().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate QR code: {}", e),
        )
    })?;

    // Encrypt and store the secret temporarily (totp_enabled stays false until verified)
    let key = get_encryption_key(&state);
    let stored_secret =
        crypto::encrypt_if_key_available(&secret_base32, key.as_ref()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encrypt TOTP secret: {}", e),
            )
        })?;

    sqlx::query("UPDATE users SET totp_secret = ? WHERE id = ?")
        .bind(&stored_secret)
        .bind(&user.id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SetupResponse {
        secret: secret_base32,
        qr_code_url,
        qr_code_svg,
    }))
}

/// POST /api/auth/2fa/verify
/// Verify a TOTP code during setup to confirm the user has configured their authenticator app.
/// On success, enables 2FA and returns one-time recovery codes.
pub async fn verify_2fa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    user: User,
    Json(request): Json<VerifyCodeRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    if user.totp_enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            "Two-factor authentication is already enabled".to_string(),
        ));
    }

    // Retrieve and decrypt the stored secret
    let stored_secret = user.totp_secret.as_deref().ok_or((
        StatusCode::BAD_REQUEST,
        "No 2FA setup in progress. Call /api/auth/2fa/setup first.".to_string(),
    ))?;

    let key = get_encryption_key(&state);
    let secret_base32 = crypto::decrypt_if_encrypted(stored_secret, key.as_ref()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to decrypt TOTP secret: {}", e),
        )
    })?;

    // Validate the code
    let totp = build_totp(&secret_base32, &user.email)?;
    let is_valid = totp.check_current(&request.code).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("TOTP check failed: {}", e),
        )
    })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid verification code".to_string(),
        ));
    }

    // Generate recovery codes
    let recovery_codes = generate_recovery_codes();
    let hashed_codes: Vec<String> = recovery_codes
        .iter()
        .map(|c| hash_recovery_code(c))
        .collect();
    let recovery_codes_json = serde_json::to_string(&hashed_codes).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize recovery codes: {}", e),
        )
    })?;

    // Enable 2FA on the user
    sqlx::query("UPDATE users SET totp_enabled = 1, recovery_codes = ? WHERE id = ?")
        .bind(&recovery_codes_json)
        .bind(&user.id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Audit log
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::AUTH_2FA_ENABLE,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(VerifyResponse { recovery_codes }))
}

/// POST /api/auth/2fa/disable
/// Disable 2FA for the authenticated user. Requires a valid TOTP code or password.
pub async fn disable_2fa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    user: User,
    Json(request): Json<DisableRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !user.totp_enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            "Two-factor authentication is not enabled".to_string(),
        ));
    }

    // Try to verify as TOTP code first, then as password
    let mut authorized = false;

    // Try TOTP code (6 digits)
    if request.code.len() == 6 && request.code.chars().all(|c| c.is_ascii_digit()) {
        if let Some(stored_secret) = &user.totp_secret {
            let key = get_encryption_key(&state);
            if let Ok(secret_base32) = crypto::decrypt_if_encrypted(stored_secret, key.as_ref()) {
                if let Ok(totp) = build_totp(&secret_base32, &user.email) {
                    if let Ok(true) = totp.check_current(&request.code) {
                        authorized = true;
                    }
                }
            }
        }
    }

    // Try password
    if !authorized {
        authorized = verify_password(&request.code, &user.password_hash);
    }

    if !authorized {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid code or password".to_string(),
        ));
    }

    // Clear 2FA fields
    sqlx::query(
        "UPDATE users SET totp_secret = NULL, totp_enabled = 0, recovery_codes = NULL WHERE id = ?",
    )
    .bind(&user.id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Audit log
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::AUTH_2FA_DISABLE,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(StatusCode::OK)
}

/// POST /api/auth/2fa/validate
/// Validate a TOTP code during login (for users with 2FA enabled).
/// Accepts a temporary session token and a TOTP code or recovery code.
/// Returns a full auth token on success.
pub async fn validate_2fa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ValidateRequest>,
) -> Result<Json<crate::db::LoginResponse>, (StatusCode, String)> {
    // Look up the temporary session
    let token_hash = hash_token(&request.session_token);
    let session: Option<crate::db::Session> = sqlx::query_as(
        "SELECT * FROM sessions WHERE token_hash = ? AND expires_at > datetime('now')",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let session = session.ok_or((
        StatusCode::UNAUTHORIZED,
        "Invalid or expired session token".to_string(),
    ))?;

    // Get the user
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&session.user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user.ok_or((StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    if !user.totp_enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            "Two-factor authentication is not enabled for this user".to_string(),
        ));
    }

    let code = request.code.trim().to_uppercase();
    let mut validated = false;

    // Try as TOTP code (6 digits)
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        if let Some(stored_secret) = &user.totp_secret {
            let key = get_encryption_key(&state);
            let secret_base32 =
                crypto::decrypt_if_encrypted(stored_secret, key.as_ref()).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to decrypt TOTP secret: {}", e),
                    )
                })?;

            let totp = build_totp(&secret_base32, &user.email)?;
            validated = totp.check_current(&code).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("TOTP check failed: {}", e),
                )
            })?;
        }
    }

    // Try as recovery code (8-char alphanumeric)
    if !validated {
        if let Some(recovery_codes_json) = &user.recovery_codes {
            let hashed_codes: Vec<String> =
                serde_json::from_str(recovery_codes_json).unwrap_or_default();
            let code_hash = hash_recovery_code(&code);

            if let Some(pos) = hashed_codes.iter().position(|h| h == &code_hash) {
                // Remove the used recovery code
                let mut remaining = hashed_codes;
                remaining.remove(pos);
                let updated_json = serde_json::to_string(&remaining).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to update recovery codes: {}", e),
                    )
                })?;

                sqlx::query("UPDATE users SET recovery_codes = ? WHERE id = ?")
                    .bind(&updated_json)
                    .bind(&user.id)
                    .execute(&state.db)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

                validated = true;
            }
        }
    }

    if !validated {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid verification code".to_string(),
        ));
    }

    // Delete the temporary session
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(&session.id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create a full session
    let new_token = {
        use rand::Rng;
        let mut rng = rand::rng();
        let bytes: [u8; 32] = rng.random();
        hex::encode(bytes)
    };
    let new_token_hash = hash_token(&new_token);
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();

    let new_session_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&new_session_id)
        .bind(&user.id)
        .bind(&new_token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Audit log
    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::AUTH_2FA_VALIDATE,
        resource_types::USER,
        Some(&user.id),
        Some(&user.email),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(Json(crate::db::LoginResponse {
        token: new_token,
        user: crate::db::UserResponse::from(user),
        requires_2fa: None,
    }))
}

/// GET /api/auth/2fa/status
/// Get the 2FA status for the authenticated user.
pub async fn status_2fa(user: User) -> Json<TwoFactorStatusResponse> {
    Json(TwoFactorStatusResponse {
        enabled: user.totp_enabled,
    })
}
