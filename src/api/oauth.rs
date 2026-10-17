use axum::{
    extract::{Path, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::{generate_session_token, hash_password};
use crate::db::{
    oauth_actions, oauth_resource_types, resource_types, CreateOAuthProviderRequest, OAuthProvider,
    OAuthProviderPublic, OAuthProviderResponse, User, UserOAuthConnection,
    UserOAuthConnectionResponse,
};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::state_cookie::{state_matches_cookie, StateCookie};

/// Query params for OAuth callback
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: Option<String>,
}

/// Response with authorization URL
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizeResponse {
    pub authorization_url: String,
    pub state: String,
}

/// User info fetched from OAuth provider
struct OAuthUserInfo {
    provider_user_id: String,
    email: Option<String>,
    /// Whether the provider asserts that `email` is verified (SEC-M3).
    /// Only verified emails may be used to link to an existing local account.
    email_verified: bool,
    name: Option<String>,
}

// ============================================================================
// Public endpoints
// ============================================================================

/// Cookie that binds the OAuth `state` to the browser that started the login (SEC-M3).
const STATE_COOKIE: StateCookie = StateCookie {
    name: "rivetr_oauth_state",
    path: "/api/auth/oauth-login",
};

/// Supported login OAuth providers (social login — not git providers)
fn is_supported_login_provider(p: &str) -> bool {
    matches!(p, "github" | "google" | "gitlab" | "azure" | "bitbucket")
}

/// GET /api/auth/oauth/providers - List enabled OAuth providers (public)
pub async fn list_enabled_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<OAuthProviderPublic>>, (StatusCode, String)> {
    let providers: Vec<OAuthProvider> =
        sqlx::query_as("SELECT * FROM oauth_providers WHERE enabled = 1")
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let public_providers: Vec<OAuthProviderPublic> = providers
        .into_iter()
        .map(|p| OAuthProviderPublic {
            provider: p.provider,
            enabled: p.enabled != 0,
        })
        .collect();

    Ok(Json(public_providers))
}

/// GET /api/auth/oauth-login/:provider/authorize - Get OAuth authorization URL for login
pub async fn oauth_login_authorize(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate provider
    if !is_supported_login_provider(&provider) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported OAuth provider: {}", provider),
        ));
    }

    // Look up the provider configuration from the database
    let oauth_provider: OAuthProvider =
        sqlx::query_as("SELECT * FROM oauth_providers WHERE provider = ? AND enabled = 1")
            .bind(&provider)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::NOT_FOUND,
                format!("{} OAuth login is not configured", provider),
            ))?;

    // Generate a random state parameter for CSRF protection
    let state_param = uuid::Uuid::new_v4().to_string();

    // Determine the base URL for callbacks
    let base_url = state
        .config
        .server
        .external_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}", state.config.server.api_port));

    let redirect_uri = format!("{}/api/auth/oauth-login/{}/callback", base_url, provider);

    let authorization_url = match provider.as_str() {
        "github" => {
            format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
                url_encode(&oauth_provider.client_id),
                url_encode(&redirect_uri),
                url_encode(&state_param),
            )
        }
        "google" => {
            format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&scope=email+profile&response_type=code&state={}",
                url_encode(&oauth_provider.client_id),
                url_encode(&redirect_uri),
                url_encode(&state_param),
            )
        }
        "gitlab" => {
            format!(
                "https://gitlab.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=read_user&state={}",
                url_encode(&oauth_provider.client_id),
                url_encode(&redirect_uri),
                url_encode(&state_param),
            )
        }
        "azure" => {
            let tenant_id = get_azure_tenant_id(&oauth_provider);
            format!(
                "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&state={}",
                tenant_id,
                url_encode(&oauth_provider.client_id),
                url_encode(&redirect_uri),
                url_encode(&state_param),
            )
        }
        "bitbucket" => format!(
            "https://bitbucket.org/site/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&state={}",
            url_encode(&oauth_provider.client_id),
            url_encode(&redirect_uri),
            url_encode(&state_param),
        ),
        _ => unreachable!(),
    };

    // Drop expired CSRF states (best effort) so abandoned logins do not pile up
    let _ = sqlx::query("DELETE FROM oauth_login_states WHERE expires_at < ?")
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&state.db)
        .await;

    // Store state in database for CSRF verification (expires in 10 minutes)
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(10))
        .unwrap()
        .to_rfc3339();

    sqlx::query("INSERT INTO oauth_login_states (state, expires_at) VALUES (?, ?)")
        .bind(&state_param)
        .bind(&expires_at)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Bind the state to this browser; the callback must present the same value.
    let cookie = STATE_COOKIE.header(&state_param, super::csp::is_https_request(&headers));

    Ok((
        [(SET_COOKIE, cookie)],
        Json(OAuthAuthorizeResponse {
            authorization_url,
            state: state_param,
        }),
    ))
}

/// GET /api/auth/oauth-login/:provider/callback - OAuth callback handler for login
pub async fn oauth_login_callback(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    axum::extract::Query(params): axum::extract::Query<OAuthCallbackParams>,
    headers: HeaderMap,
    client_ip: ClientIp,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Validate provider
    if !is_supported_login_provider(&provider) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported OAuth provider: {}", provider),
        ));
    }

    // SEC-M3: enforce the CSRF state strictly. It must exist, be unexpired, and is
    // consumed atomically so it can only be used once.
    let state_param = params.state.as_deref().filter(|s| !s.is_empty()).ok_or((
        StatusCode::BAD_REQUEST,
        "Missing OAuth state parameter. Please start the sign-in again.".to_string(),
    ))?;

    // The state must also match the cookie set when this browser started the login,
    // so a state minted by an attacker cannot be replayed into a victim's browser.
    if !state_matches_cookie(state_param, STATE_COOKIE.read(&headers)) {
        return Err((
            StatusCode::BAD_REQUEST,
            "OAuth state does not match this browser. Please start the sign-in again from the same address you use to open the dashboard.".to_string(),
        ));
    }

    let consumed: Option<(String,)> =
        sqlx::query_as("DELETE FROM oauth_login_states WHERE state = ? RETURNING expires_at")
            .bind(state_param)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let state_valid = consumed
        .and_then(|(expires_at,)| chrono::DateTime::parse_from_rfc3339(&expires_at).ok())
        .is_some_and(|expires_at| expires_at > chrono::Utc::now());
    if !state_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid or expired OAuth state. Please start the sign-in again.".to_string(),
        ));
    }

    // Look up the provider configuration
    let oauth_provider: OAuthProvider =
        sqlx::query_as("SELECT * FROM oauth_providers WHERE provider = ? AND enabled = 1")
            .bind(&provider)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::NOT_FOUND,
                format!("{} OAuth login is not configured", provider),
            ))?;

    let base_url = state
        .config
        .server
        .external_url
        .clone()
        .unwrap_or_else(|| format!("http://localhost:{}", state.config.server.api_port));

    let redirect_uri = format!("{}/api/auth/oauth-login/{}/callback", base_url, provider);

    // Decrypt client_secret if encrypted
    let client_secret = if let Some(ref encryption_key) = state.config.auth.encryption_key {
        let key = crate::crypto::derive_key(encryption_key);
        crate::crypto::decrypt_if_encrypted(&oauth_provider.client_secret, Some(&key)).map_err(
            |e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Decryption failed: {}", e),
                )
            },
        )?
    } else {
        oauth_provider.client_secret.clone()
    };

    // Exchange code for access token
    let (access_token, refresh_token) = exchange_code_for_token(
        &provider,
        &params.code,
        &oauth_provider.client_id,
        &client_secret,
        &redirect_uri,
    )
    .await?;

    // Fetch user info from the provider
    let user_info = fetch_user_info(&provider, &access_token).await?;

    // For GitHub and Bitbucket, email may be null in user profile, so fetch from emails endpoint.
    // `email_verified` records whether the provider vouches for ownership of the address.
    let mut email_verified = user_info.email_verified;
    let email = match (&user_info.email, provider.as_str()) {
        (None, "github") => match fetch_github_primary_email(&access_token).await {
            Ok(e) => {
                email_verified = true;
                Some(e)
            }
            Err(_) => None,
        },
        (Some(e), "github") => {
            email_verified = github_email_is_verified(&access_token, e).await;
            Some(e.clone())
        }
        (_, "bitbucket") => match fetch_bitbucket_primary_email(&access_token).await {
            Ok((e, confirmed)) => {
                email_verified = confirmed;
                Some(e)
            }
            Err(_) => None,
        },
        (email, _) => email.clone(),
    };

    // Azure AD: Graph /me does not say whether `mail` is verified. In a single-tenant app
    // the tenant admin controls it; in multi-tenant mode any tenant can assert any address.
    if provider == "azure" {
        email_verified = !matches!(
            get_azure_tenant_id(&oauth_provider).as_str(),
            "common" | "organizations" | "consumers"
        );
    }

    let email = email.ok_or((
        StatusCode::BAD_REQUEST,
        "Could not determine email from OAuth provider. Please ensure your email is public or grant the email scope.".to_string(),
    ))?;

    // Now determine what to do: find or create user
    // 1. Check if there's already an OAuth connection for this provider_user_id
    let existing_connection: Option<UserOAuthConnection> = sqlx::query_as(
        "SELECT * FROM user_oauth_connections WHERE provider = ? AND provider_user_id = ?",
    )
    .bind(&provider)
    .bind(&user_info.provider_user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_id = if let Some(connection) = existing_connection {
        // User has connected this OAuth account before - log them in
        connection.user_id
    } else {
        // 2. Check if a user exists with the same email
        let existing_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(user) = existing_user {
            // SEC-M3: only auto-link to a local account when the provider vouches for the email
            if !email_verified {
                return Err((
                    StatusCode::FORBIDDEN,
                    format!(
                        "An account with this email already exists, but {} did not confirm that you own it. Verify the email with {} or sign in with your password.",
                        provider, provider
                    ),
                ));
            }

            // Link OAuth connection to existing user
            let conn_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            sqlx::query(
                r#"INSERT INTO user_oauth_connections (id, user_id, provider, provider_user_id, provider_email, provider_name, access_token, refresh_token, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&conn_id)
            .bind(&user.id)
            .bind(&provider)
            .bind(&user_info.provider_user_id)
            .bind(&email)
            .bind(&user_info.name)
            .bind(&access_token)
            .bind(&refresh_token)
            .bind(&now)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            user.id
        } else {
            // 3. Create a new user
            let new_user_id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();

            // Generate a random password hash (user logs in via OAuth, not password)
            let random_password = uuid::Uuid::new_v4().to_string();
            let password_hash = hash_password(&random_password).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to hash password: {}", e),
                )
            })?;

            let name = user_info
                .name
                .clone()
                .unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());

            // Determine role: first user is admin, others are member
            let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&state.db)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let role = if user_count.0 == 0 { "admin" } else { "member" };

            sqlx::query(
                "INSERT INTO users (id, email, password_hash, name, role, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&new_user_id)
            .bind(&email)
            .bind(&password_hash)
            .bind(&name)
            .bind(role)
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

            // Link OAuth connection
            let conn_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO user_oauth_connections (id, user_id, provider, provider_user_id, provider_email, provider_name, access_token, refresh_token, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&conn_id)
            .bind(&new_user_id)
            .bind(&provider)
            .bind(&user_info.provider_user_id)
            .bind(&email)
            .bind(&user_info.name)
            .bind(&access_token)
            .bind(&refresh_token)
            .bind(&now)
            .execute(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            tracing::info!("Created new user via OAuth login: {} ({})", email, provider);

            new_user_id
        }
    };

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
        oauth_actions::OAUTH_LOGIN,
        resource_types::USER,
        Some(&user_id),
        Some(&email),
        Some(&user_id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "provider": provider })),
    )
    .await;

    // Redirect to the frontend with the token as a query parameter
    // The frontend will pick this up and store it
    Ok((
        [(
            SET_COOKIE,
            STATE_COOKIE.clear_header(super::csp::is_https_request(&headers)),
        )],
        Redirect::to(&format!("/login?oauth_token={}", token)),
    ))
}

// ============================================================================
// Admin endpoints (protected)
// ============================================================================

/// GET /api/settings/oauth-providers - List all OAuth providers (admin)
pub async fn list_oauth_providers(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<OAuthProviderResponse>>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    let providers: Vec<OAuthProvider> =
        sqlx::query_as("SELECT * FROM oauth_providers ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<OAuthProviderResponse> = providers
        .into_iter()
        .map(OAuthProviderResponse::from)
        .collect();

    Ok(Json(responses))
}

/// POST /api/settings/oauth-providers - Create/update an OAuth provider (admin)
pub async fn create_oauth_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    client_ip: ClientIp,
    Json(req): Json<CreateOAuthProviderRequest>,
) -> Result<Json<OAuthProviderResponse>, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    // Validate provider type
    if !is_supported_login_provider(&req.provider) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Provider must be 'github', 'google', 'gitlab', 'azure', or 'bitbucket'".to_string(),
        ));
    }

    if req.client_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Client ID is required".to_string()));
    }

    // Check if this is an update (provider already exists)
    let existing: Option<OAuthProvider> =
        sqlx::query_as("SELECT * FROM oauth_providers WHERE provider = ?")
            .bind(&req.provider)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_update = existing.is_some();
    let secret_unchanged = req.client_secret.trim() == "unchanged";

    // For new providers, client_secret is required
    if !is_update && (req.client_secret.trim().is_empty() || secret_unchanged) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Client Secret is required".to_string(),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let enabled = req.enabled.unwrap_or(true);

    if is_update && secret_unchanged {
        // Update without changing the client_secret
        sqlx::query(
            r#"UPDATE oauth_providers SET
                client_id = ?,
                enabled = ?,
                extra_config = ?,
                updated_at = ?
            WHERE provider = ?"#,
        )
        .bind(&req.client_id)
        .bind(enabled as i32)
        .bind(&req.extra_config)
        .bind(&now)
        .bind(&req.provider)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
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
            r#"INSERT INTO oauth_providers (id, provider, client_id, client_secret, enabled, extra_config, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider) DO UPDATE SET
                client_id = excluded.client_id,
                client_secret = excluded.client_secret,
                enabled = excluded.enabled,
                extra_config = excluded.extra_config,
                updated_at = excluded.updated_at"#,
        )
        .bind(&id)
        .bind(&req.provider)
        .bind(&req.client_id)
        .bind(&client_secret)
        .bind(enabled as i32)
        .bind(&req.extra_config)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    audit_log(
        &state,
        oauth_actions::OAUTH_PROVIDER_CREATE,
        oauth_resource_types::OAUTH_PROVIDER,
        Some(&id),
        Some(&req.provider),
        Some(&user.id),
        client_ip.as_deref(),
        Some(serde_json::json!({ "provider": req.provider })),
    )
    .await;

    // Fetch the actual stored record (might be an update)
    let provider: OAuthProvider =
        sqlx::query_as("SELECT * FROM oauth_providers WHERE provider = ?")
            .bind(&req.provider)
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OAuthProviderResponse::from(provider)))
}

/// DELETE /api/settings/oauth-providers/:id - Delete an OAuth provider (admin)
pub async fn delete_oauth_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if user.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Admin access required".to_string()));
    }

    let result = sqlx::query("DELETE FROM oauth_providers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "OAuth provider not found".to_string(),
        ));
    }

    audit_log(
        &state,
        oauth_actions::OAUTH_PROVIDER_DELETE,
        oauth_resource_types::OAUTH_PROVIDER,
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
// Account linking endpoints (protected)
// ============================================================================

/// GET /api/settings/oauth-connections - List current user's OAuth connections
pub async fn list_user_connections(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<Vec<UserOAuthConnectionResponse>>, (StatusCode, String)> {
    let connections: Vec<UserOAuthConnection> = sqlx::query_as(
        "SELECT * FROM user_oauth_connections WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<UserOAuthConnectionResponse> = connections
        .into_iter()
        .map(UserOAuthConnectionResponse::from)
        .collect();

    Ok(Json(responses))
}

/// DELETE /api/settings/oauth-connections/:id - Unlink an OAuth connection
pub async fn delete_user_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    client_ip: ClientIp,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // User always has password (set during account creation), so it's safe to unlink any OAuth connection

    let result = sqlx::query("DELETE FROM user_oauth_connections WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "OAuth connection not found".to_string(),
        ));
    }

    audit_log(
        &state,
        oauth_actions::OAUTH_ACCOUNT_UNLINK,
        resource_types::USER,
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
// Helper functions
// ============================================================================

/// Extract the Azure AD tenant_id from the provider's extra_config JSON.
/// Falls back to "common" if not set (allows any Microsoft account to sign in).
fn get_azure_tenant_id(provider: &OAuthProvider) -> String {
    if let Some(ref extra) = provider.extra_config {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(extra) {
            if let Some(tid) = v.get("tenant_id").and_then(|t| t.as_str()) {
                if !tid.is_empty() {
                    return tid.to_string();
                }
            }
        }
    }
    "common".to_string()
}

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

/// Exchange authorization code for access token
async fn exchange_code_for_token(
    provider: &str,
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<(String, Option<String>), (StatusCode, String)> {
    let client = reqwest::Client::new();

    match provider {
        "github" => {
            #[derive(Deserialize)]
            struct GitHubTokenResponse {
                access_token: String,
            }

            let response = client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                ])
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to exchange code: {}", e),
                    )
                })?;

            let token_response: GitHubTokenResponse = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse token response: {}", e),
                )
            })?;

            Ok((token_response.access_token, None))
        }
        "google" => {
            #[derive(Deserialize)]
            struct GoogleTokenResponse {
                access_token: String,
                refresh_token: Option<String>,
            }

            let response = client
                .post("https://oauth2.googleapis.com/token")
                .form(&[
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                    ("grant_type", "authorization_code"),
                ])
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to exchange code: {}", e),
                    )
                })?;

            let token_response: GoogleTokenResponse = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse token response: {}", e),
                )
            })?;

            Ok((token_response.access_token, token_response.refresh_token))
        }
        "gitlab" => {
            #[derive(Deserialize)]
            struct GitLabTokenResponse {
                access_token: String,
                refresh_token: Option<String>,
            }

            let response = client
                .post("https://gitlab.com/oauth/token")
                .form(&[
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                    ("grant_type", "authorization_code"),
                ])
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to exchange code: {}", e),
                    )
                })?;

            let token_response: GitLabTokenResponse = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse token response: {}", e),
                )
            })?;

            Ok((token_response.access_token, token_response.refresh_token))
        }
        "azure" => {
            #[derive(Deserialize)]
            struct AzureTokenResponse {
                access_token: String,
                refresh_token: Option<String>,
            }

            let response = client
                .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
                .form(&[
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                    ("grant_type", "authorization_code"),
                    ("scope", "openid email profile"),
                ])
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to exchange code: {}", e),
                    )
                })?;

            let token_response: AzureTokenResponse = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse token response: {}", e),
                )
            })?;

            Ok((token_response.access_token, token_response.refresh_token))
        }
        "bitbucket" => {
            #[derive(Deserialize)]
            struct BitbucketTokenResponse {
                access_token: String,
                refresh_token: Option<String>,
            }

            let response = client
                .post("https://bitbucket.org/site/oauth2/access_token")
                .basic_auth(client_id, Some(client_secret))
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                ])
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to exchange code: {}", e),
                    )
                })?;

            let token_response: BitbucketTokenResponse = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse token response: {}", e),
                )
            })?;

            Ok((token_response.access_token, token_response.refresh_token))
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported provider: {}", provider),
        )),
    }
}

/// Fetch user info from OAuth provider
async fn fetch_user_info(
    provider: &str,
    access_token: &str,
) -> Result<OAuthUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    match provider {
        "github" => {
            #[derive(Deserialize)]
            struct GitHubUser {
                id: i64,
                email: Option<String>,
                name: Option<String>,
            }

            let response = client
                .get("https://api.github.com/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "Rivetr")
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to get user info: {}", e),
                    )
                })?;

            let user: GitHubUser = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse user info: {}", e),
                )
            })?;

            Ok(OAuthUserInfo {
                provider_user_id: user.id.to_string(),
                email: user.email,
                // Checked against /user/emails by the caller
                email_verified: false,
                name: user.name,
            })
        }
        "google" => {
            #[derive(Deserialize)]
            struct GoogleUser {
                id: String,
                email: Option<String>,
                #[serde(default, alias = "email_verified")]
                verified_email: Option<bool>,
                name: Option<String>,
            }

            let response = client
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to get user info: {}", e),
                    )
                })?;

            let user: GoogleUser = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse user info: {}", e),
                )
            })?;

            Ok(OAuthUserInfo {
                provider_user_id: user.id,
                email: user.email,
                email_verified: user.verified_email.unwrap_or(false),
                name: user.name,
            })
        }
        "gitlab" => {
            #[derive(Deserialize)]
            struct GitLabUser {
                id: i64,
                email: Option<String>,
                confirmed_at: Option<String>,
                name: Option<String>,
            }

            let response = client
                .get("https://gitlab.com/api/v4/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to get user info: {}", e),
                    )
                })?;

            let user: GitLabUser = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse user info: {}", e),
                )
            })?;

            Ok(OAuthUserInfo {
                provider_user_id: user.id.to_string(),
                email: user.email,
                email_verified: user.confirmed_at.is_some(),
                name: user.name,
            })
        }
        "azure" => {
            #[derive(Deserialize)]
            struct AzureUser {
                id: String,
                #[serde(rename = "displayName")]
                display_name: Option<String>,
                mail: Option<String>,
                #[serde(rename = "userPrincipalName")]
                user_principal_name: Option<String>,
            }

            let response = client
                .get("https://graph.microsoft.com/v1.0/me")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to get user info: {}", e),
                    )
                })?;

            let user: AzureUser = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse user info: {}", e),
                )
            })?;

            let email = user.mail.or(user.user_principal_name);

            Ok(OAuthUserInfo {
                provider_user_id: user.id,
                email,
                // Decided by the caller based on tenant configuration
                email_verified: false,
                name: user.display_name,
            })
        }
        "bitbucket" => {
            #[derive(Deserialize)]
            struct BitbucketUser {
                #[serde(rename = "account_id")]
                account_id: String,
                #[serde(rename = "display_name")]
                display_name: Option<String>,
            }

            let response = client
                .get("https://api.bitbucket.org/2.0/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await
                .map_err(|e| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to get user info: {}", e),
                    )
                })?;

            let user: BitbucketUser = response.json().await.map_err(|e| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to parse user info: {}", e),
                )
            })?;

            Ok(OAuthUserInfo {
                provider_user_id: user.account_id,
                // Email is fetched separately via /user/emails
                email: None,
                email_verified: false,
                name: user.display_name,
            })
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported provider: {}", provider),
        )),
    }
}

/// Fetch primary email from GitHub (when user profile email is null)
async fn fetch_github_primary_email(access_token: &str) -> Result<String, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitHubEmail {
        email: String,
        primary: bool,
        verified: bool,
    }

    let response = client
        .get("https://api.github.com/user/emails")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Rivetr")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch emails: {}", e),
            )
        })?;

    let emails: Vec<GitHubEmail> = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse emails: {}", e),
        )
    })?;

    // Find primary verified email
    emails
        .iter()
        .find(|e| e.primary && e.verified)
        .or_else(|| emails.iter().find(|e| e.verified))
        .map(|e| e.email.clone())
        .ok_or((
            StatusCode::BAD_REQUEST,
            "No verified email found on GitHub account".to_string(),
        ))
}

/// Whether GitHub lists `email` as a verified address on the account (SEC-M3).
/// Returns false on any failure (e.g. token lacks the `user:email` scope).
async fn github_email_is_verified(access_token: &str, email: &str) -> bool {
    #[derive(Deserialize)]
    struct GitHubEmail {
        email: String,
        verified: bool,
    }

    let response = match reqwest::Client::new()
        .get("https://api.github.com/user/emails")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Rivetr")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    match response.json::<Vec<GitHubEmail>>().await {
        Ok(emails) => emails
            .iter()
            .any(|e| e.verified && e.email.eq_ignore_ascii_case(email)),
        Err(_) => false,
    }
}

/// Fetch primary email from Bitbucket (when user profile email is null).
/// Returns the address and whether Bitbucket has confirmed it.
async fn fetch_bitbucket_primary_email(
    access_token: &str,
) -> Result<(String, bool), (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct BitbucketEmailValue {
        email: String,
        is_primary: bool,
        is_confirmed: bool,
    }

    #[derive(Deserialize)]
    struct BitbucketEmailsPage {
        values: Vec<BitbucketEmailValue>,
    }

    let response = client
        .get("https://api.bitbucket.org/2.0/user/emails")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch Bitbucket emails: {}", e),
            )
        })?;

    let page: BitbucketEmailsPage = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse Bitbucket emails: {}", e),
        )
    })?;

    // Find primary confirmed email first, then any confirmed email
    page.values
        .iter()
        .find(|e| e.is_primary && e.is_confirmed)
        .or_else(|| page.values.iter().find(|e| e.is_confirmed))
        .or_else(|| page.values.first())
        .map(|e| (e.email.clone(), e.is_confirmed))
        .ok_or((
            StatusCode::BAD_REQUEST,
            "No email found on Bitbucket account".to_string(),
        ))
}
