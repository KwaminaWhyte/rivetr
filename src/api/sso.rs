use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::auth::{generate_session_token, hash_password};
use crate::db::{
    sso_actions, sso_resource_types, CreateOidcProviderRequest, OidcProvider, OidcProviderResponse,
    User,
};
use crate::AppState;

use super::audit::{audit_log, ClientIp};

/// OIDC discovery document structure
#[derive(Deserialize)]
struct OidcDiscovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
}

/// OIDC token exchange response
#[derive(Deserialize)]
struct OidcTokenResponse {
    access_token: String,
}

/// OIDC userinfo response
#[derive(Deserialize)]
struct OidcUserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
}

/// Query params for OIDC callback
#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

// ============================================================================
// Management endpoints (require auth)
// ============================================================================

/// GET /api/sso/providers - List all OIDC providers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<OidcProviderResponse>>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    let providers: Vec<OidcProvider> =
        sqlx::query_as("SELECT * FROM oidc_providers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<OidcProviderResponse> = providers
        .into_iter()
        .map(OidcProviderResponse::from)
        .collect();

    Ok(Json(responses))
}

/// POST /api/sso/providers - Create an OIDC provider
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateOidcProviderRequest>,
) -> Result<Json<OidcProviderResponse>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    if req.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Name is required".to_string()));
    }
    if req.client_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Client ID is required".to_string()));
    }
    if req.client_secret.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Client Secret is required".to_string(),
        ));
    }
    if req.discovery_url.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Discovery URL is required".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Compute redirect_uri if not provided
    let base_url = state
        .config
        .server
        .external_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}", state.config.server.api_port));
    let redirect_uri = req
        .redirect_uri
        .unwrap_or_else(|| format!("{}/auth/sso/{}/callback", base_url, id));

    let scopes = req
        .scopes
        .unwrap_or_else(|| "openid email profile".to_string());
    let enabled = req.enabled.unwrap_or(true);

    // Encrypt client_secret if encryption key is available
    let client_secret = if let Some(ref encryption_key) = state.config.auth.encryption_key {
        let key = crate::crypto::derive_key(encryption_key);
        crate::crypto::encrypt(&req.client_secret, &key).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Encryption failed: {}", e),
            )
        })?
    } else {
        req.client_secret.clone()
    };

    sqlx::query(
        r#"INSERT INTO oidc_providers (id, name, client_id, client_secret, discovery_url, redirect_uri, scopes, enabled, team_id, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(req.client_id.trim())
    .bind(&client_secret)
    .bind(req.discovery_url.trim())
    .bind(&redirect_uri)
    .bind(&scopes)
    .bind(enabled as i32)
    .bind(&req.team_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    audit_log(
        &state,
        sso_actions::SSO_PROVIDER_CREATE,
        sso_resource_types::SSO_PROVIDER,
        Some(&id),
        Some(req.name.trim()),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    let provider: OidcProvider = sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OidcProviderResponse::from(provider)))
}

/// GET /api/sso/providers/:id - Get an OIDC provider
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
) -> Result<Json<OidcProviderResponse>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    let provider: Option<OidcProvider> =
        sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match provider {
        Some(p) => Ok(Json(OidcProviderResponse::from(p))),
        None => Err((StatusCode::NOT_FOUND, "OIDC provider not found".to_string())),
    }
}

/// PUT /api/sso/providers/:id - Update an OIDC provider
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateOidcProviderRequest>,
) -> Result<Json<OidcProviderResponse>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    // Verify exists
    let existing: Option<OidcProvider> =
        sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let existing =
        existing.ok_or((StatusCode::NOT_FOUND, "OIDC provider not found".to_string()))?;

    let now = chrono::Utc::now().to_rfc3339();
    let enabled = req.enabled.unwrap_or(existing.enabled != 0);
    let scopes = req.scopes.unwrap_or_else(|| existing.scopes.clone());
    let redirect_uri = req
        .redirect_uri
        .unwrap_or_else(|| existing.redirect_uri.clone());

    let secret_unchanged =
        req.client_secret.trim() == "unchanged" || req.client_secret.trim().is_empty();

    if secret_unchanged {
        sqlx::query(
            r#"UPDATE oidc_providers SET
                name = ?, client_id = ?, discovery_url = ?, redirect_uri = ?,
                scopes = ?, enabled = ?, team_id = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(req.name.trim())
        .bind(req.client_id.trim())
        .bind(req.discovery_url.trim())
        .bind(&redirect_uri)
        .bind(&scopes)
        .bind(enabled as i32)
        .bind(&req.team_id)
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        let client_secret = if let Some(ref encryption_key) = state.config.auth.encryption_key {
            let key = crate::crypto::derive_key(encryption_key);
            crate::crypto::encrypt(&req.client_secret, &key).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Encryption failed: {}", e),
                )
            })?
        } else {
            req.client_secret.clone()
        };

        sqlx::query(
            r#"UPDATE oidc_providers SET
                name = ?, client_id = ?, client_secret = ?, discovery_url = ?, redirect_uri = ?,
                scopes = ?, enabled = ?, team_id = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(req.name.trim())
        .bind(req.client_id.trim())
        .bind(&client_secret)
        .bind(req.discovery_url.trim())
        .bind(&redirect_uri)
        .bind(&scopes)
        .bind(enabled as i32)
        .bind(&req.team_id)
        .bind(&now)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    audit_log(
        &state,
        sso_actions::SSO_PROVIDER_UPDATE,
        sso_resource_types::SSO_PROVIDER,
        Some(&id),
        Some(req.name.trim()),
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    let provider: OidcProvider = sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OidcProviderResponse::from(provider)))
}

/// DELETE /api/sso/providers/:id - Delete an OIDC provider
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    let result = sqlx::query("DELETE FROM oidc_providers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "OIDC provider not found".to_string()));
    }

    audit_log(
        &state,
        sso_actions::SSO_PROVIDER_DELETE,
        sso_resource_types::SSO_PROVIDER,
        Some(&id),
        None,
        Some(&user.id),
        client_ip.as_deref(),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Auth flow endpoints (no auth required)
// ============================================================================

/// GET /auth/sso/:provider_id/login - Initiate OIDC login flow
pub async fn initiate_sso_login(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Look up provider
    let provider: Option<OidcProvider> =
        sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ? AND enabled = 1")
            .bind(&provider_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let provider = provider.ok_or((
        StatusCode::NOT_FOUND,
        "OIDC provider not found or disabled".to_string(),
    ))?;

    // Fetch discovery document
    let http_client = reqwest::Client::new();
    let discovery: OidcDiscovery = http_client
        .get(&provider.discovery_url)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch OIDC discovery document: {}", e),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse OIDC discovery document: {}", e),
            )
        })?;

    // Generate CSRF state
    let state_param = uuid::Uuid::new_v4().to_string();

    // Store state in sso_states table (expires in 10 minutes - we'll clean up old ones lazily)
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO sso_states (state, provider_id, redirect_to, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&state_param)
    .bind(&provider_id)
    .bind(Option::<String>::None)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build authorization URL
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&response_type=code&state={}",
        discovery.authorization_endpoint,
        url_encode(&provider.client_id),
        url_encode(&provider.redirect_uri),
        url_encode(&provider.scopes),
        url_encode(&state_param),
    );

    Ok(Redirect::to(&auth_url))
}

/// GET /auth/sso/:provider_id/callback - Handle OIDC callback
pub async fn handle_sso_callback(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Query(params): Query<OidcCallbackParams>,
    client_ip: ClientIp,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check for errors from provider
    if let Some(error) = &params.error {
        let desc = params
            .error_description
            .as_deref()
            .unwrap_or("Unknown error");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("OIDC error: {} - {}", error, desc),
        ));
    }

    let code = params.code.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing authorization code".to_string(),
    ))?;

    let state_param = params.state.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing state parameter".to_string(),
    ))?;

    // Verify CSRF state
    let valid_state: Option<(String,)> =
        sqlx::query_as("SELECT state FROM sso_states WHERE state = ? AND provider_id = ?")
            .bind(&state_param)
            .bind(&provider_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if valid_state.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or expired state parameter".to_string(),
        ));
    }

    // Clean up used state
    let _ = sqlx::query("DELETE FROM sso_states WHERE state = ?")
        .bind(&state_param)
        .execute(&state.db)
        .await;

    // Clean up old states older than 10 minutes
    let _ = sqlx::query("DELETE FROM sso_states WHERE created_at < datetime('now', '-10 minutes')")
        .execute(&state.db)
        .await;

    // Look up provider
    let provider: Option<OidcProvider> =
        sqlx::query_as("SELECT * FROM oidc_providers WHERE id = ? AND enabled = 1")
            .bind(&provider_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let provider = provider.ok_or((
        StatusCode::NOT_FOUND,
        "OIDC provider not found or disabled".to_string(),
    ))?;

    // Decrypt client_secret if encrypted
    let client_secret = if let Some(ref encryption_key) = state.config.auth.encryption_key {
        let key = crate::crypto::derive_key(encryption_key);
        crate::crypto::decrypt_if_encrypted(&provider.client_secret, Some(&key)).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Decryption failed: {}", e),
            )
        })?
    } else {
        provider.client_secret.clone()
    };

    // Fetch discovery document to get token and userinfo endpoints
    let http_client = reqwest::Client::new();
    let discovery: OidcDiscovery = http_client
        .get(&provider.discovery_url)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch OIDC discovery document: {}", e),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse OIDC discovery document: {}", e),
            )
        })?;

    // Exchange code for access token
    let token_response: OidcTokenResponse = http_client
        .post(&discovery.token_endpoint)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &provider.redirect_uri),
            ("client_id", &provider.client_id),
            ("client_secret", &client_secret),
        ])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to exchange code for token: {}", e),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse token response: {}", e),
            )
        })?;

    // Fetch user info
    let user_info: OidcUserInfo = http_client
        .get(&discovery.userinfo_endpoint)
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token),
        )
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch user info: {}", e),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse user info: {}", e),
            )
        })?;

    let email = user_info.email.ok_or((
        StatusCode::BAD_REQUEST,
        "Could not determine email from OIDC provider. Ensure the 'email' scope is granted."
            .to_string(),
    ))?;

    // Find or create user
    let user_id = find_or_create_sso_user(
        &state,
        &email,
        &user_info.sub,
        &provider_id,
        user_info.name.as_deref(),
    )
    .await?;

    // Create a session for the user
    let (token, token_hash) = generate_session_token();
    let session_id = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .to_rfc3339();

    sqlx::query("INSERT INTO sessions (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)")
        .bind(&session_id)
        .bind(&user_id)
        .bind(&token_hash)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Audit log
    audit_log(
        &state,
        sso_actions::SSO_LOGIN,
        "user",
        Some(&user_id),
        Some(&email),
        Some(&user_id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "provider_id": provider_id, "provider_name": provider.name })),
    )
    .await;

    // Redirect to frontend with token
    Ok(Redirect::to(&format!("/login?oauth_token={}", token)))
}

/// Find existing user by OIDC subject or email, or create a new one
async fn find_or_create_sso_user(
    state: &Arc<AppState>,
    email: &str,
    oidc_subject: &str,
    provider_id: &str,
    name: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    // Check if user already linked to this OIDC provider and subject
    let existing_by_oidc: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE oidc_subject = ? AND oidc_provider_id = ?")
            .bind(oidc_subject)
            .bind(provider_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some((user_id,)) = existing_by_oidc {
        return Ok(user_id);
    }

    // Check if a user with this email exists
    let existing_by_email: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some((user_id,)) = existing_by_email {
        // Link the OIDC identity to the existing user
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE users SET oidc_subject = ?, oidc_provider_id = ?, updated_at = ? WHERE id = ?",
        )
        .bind(oidc_subject)
        .bind(provider_id)
        .bind(&now)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        return Ok(user_id);
    }

    // Create a new user
    let new_user_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let display_name = name
        .map(|n| n.to_string())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

    // Generate a random password hash (user logs in via SSO, not password)
    let random_password = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(&random_password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to hash password: {}", e),
        )
    })?;

    // Determine role: first user is admin
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let role = if user_count.0 == 0 { "admin" } else { "member" };

    sqlx::query(
        r#"INSERT INTO users (id, email, password_hash, name, role, oidc_subject, oidc_provider_id, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&new_user_id)
    .bind(email)
    .bind(&password_hash)
    .bind(&display_name)
    .bind(role)
    .bind(oidc_subject)
    .bind(provider_id)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create a default "Personal" team for the new user
    let team_id = uuid::Uuid::new_v4().to_string();
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

    let member_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO team_members (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&member_id)
    .bind(&team_id)
    .bind(&new_user_id)
    .bind("owner")
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(
        "Created new user via SSO login: {} (provider: {})",
        email,
        provider_id
    );

    Ok(new_user_id)
}

// ============================================================================
// Helper functions
// ============================================================================

/// URL-encode a string for use in query parameters
fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}
