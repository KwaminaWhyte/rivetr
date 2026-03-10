mod bitbucket;
mod github;
mod gitlab;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{
    actions, resource_types, GitProvider, GitProviderResponse, GitProviderType, GitRepository,
    OAuthAuthorizationResponse, OAuthCallbackRequest, User,
};
use crate::AppState;

use super::audit::{audit_log, extract_client_ip};

/// Shared user info returned by provider validation/OAuth flows
pub(super) struct ProviderUserInfo {
    pub provider_user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub scopes: Option<String>,
}

/// Request to add a provider via Personal Access Token
#[derive(Debug, Deserialize)]
pub struct AddTokenProviderRequest {
    pub provider: String,
    pub token: String,
    /// For Bitbucket: username is required along with app password
    pub username: Option<String>,
}

/// Response after adding a token-based provider
#[derive(Debug, Serialize)]
pub struct TokenProviderResponse {
    pub id: String,
    pub provider: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListReposParams {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_per_page")]
    per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    30
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

/// List all connected Git providers for the current user
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = &user.id;

    let providers: Vec<GitProvider> =
        sqlx::query_as("SELECT * FROM git_providers WHERE user_id = ? ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<GitProviderResponse> = providers
        .into_iter()
        .map(GitProviderResponse::from)
        .collect();
    Ok(Json(responses))
}

/// Get a specific Git provider connection
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider: GitProvider = sqlx::query_as("SELECT * FROM git_providers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

    let response: GitProviderResponse = provider.into();
    Ok(Json(response))
}

/// Disconnect (delete) a Git provider connection
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    user: User,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM git_providers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Provider not found".to_string()));
    }

    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::GIT_PROVIDER_DELETE,
        resource_types::GIT_PROVIDER,
        Some(&id),
        None,
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// Add a Git provider via Personal Access Token (GitLab) or App Password (Bitbucket)
pub async fn add_token_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    headers: HeaderMap,
    Json(req): Json<AddTokenProviderRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider_type: GitProviderType = req
        .provider
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    // GitHub should use GitHub Apps, not PAT
    if matches!(provider_type, GitProviderType::Github) {
        return Err((
            StatusCode::BAD_REQUEST,
            "GitHub integration uses GitHub Apps. Please use the GitHub Apps flow instead."
                .to_string(),
        ));
    }

    // Validate and get user info using the token
    let user_info = match provider_type {
        GitProviderType::Github => unreachable!(),
        GitProviderType::Gitlab => gitlab::validate_token(&req.token).await?,
        GitProviderType::Bitbucket => {
            let username = req.username.ok_or((
                StatusCode::BAD_REQUEST,
                "Username is required for Bitbucket App Password".to_string(),
            ))?;
            bitbucket::validate_app_password(&username, &req.token).await?
        }
    };

    // Store the provider
    let user_id = &user.id;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO git_providers (id, user_id, provider, provider_user_id, username, display_name, email, avatar_url, access_token, refresh_token, token_expires_at, scopes, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, provider) DO UPDATE SET
            provider_user_id = excluded.provider_user_id,
            username = excluded.username,
            display_name = excluded.display_name,
            email = excluded.email,
            avatar_url = excluded.avatar_url,
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_expires_at = excluded.token_expires_at,
            scopes = excluded.scopes,
            updated_at = excluded.updated_at
        "#
    )
    .bind(&id)
    .bind(user_id)
    .bind(provider_type.to_string())
    .bind(&user_info.provider_user_id)
    .bind(&user_info.username)
    .bind(&user_info.display_name)
    .bind(&user_info.email)
    .bind(&user_info.avatar_url)
    .bind(&req.token)
    .bind::<Option<String>>(None) // No refresh token for PATs
    .bind::<Option<String>>(None) // No expiration for PATs (they're long-lived)
    .bind(&user_info.scopes)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ip = extract_client_ip(&headers, None);
    audit_log(
        &state,
        actions::GIT_PROVIDER_ADD,
        resource_types::GIT_PROVIDER,
        Some(&id),
        Some(&req.provider),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "provider": req.provider,
            "username": &user_info.username,
        })),
    )
    .await;

    Ok(Json(TokenProviderResponse {
        id,
        provider: provider_type.to_string(),
        username: user_info.username,
        display_name: user_info.display_name,
        avatar_url: user_info.avatar_url,
    }))
}

/// Get OAuth authorization URL for a provider
pub async fn get_auth_url(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider_type: GitProviderType = provider
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    let oauth_config = match provider_type {
        GitProviderType::Github => state.config.oauth.github.as_ref(),
        GitProviderType::Gitlab => state.config.oauth.gitlab.as_ref(),
        GitProviderType::Bitbucket => state.config.oauth.bitbucket.as_ref(),
    };

    let oauth = oauth_config.ok_or((
        StatusCode::NOT_FOUND,
        format!("{} OAuth is not configured", provider_type),
    ))?;

    // Generate a random state for CSRF protection
    let state_param = uuid::Uuid::new_v4().to_string();

    let authorization_url = match provider_type {
        GitProviderType::Github => {
            let redirect_uri = oauth.redirect_uri.clone().unwrap_or_else(|| {
                format!("{}/api/auth/oauth/github/callback", "http://localhost:8080")
            });
            format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
                oauth.client_id,
                url_encode(&redirect_uri),
                url_encode("repo read:user user:email"),
                state_param
            )
        }
        GitProviderType::Gitlab => {
            let redirect_uri = oauth.redirect_uri.clone().unwrap_or_else(|| {
                format!("{}/api/auth/oauth/gitlab/callback", "http://localhost:8080")
            });
            format!(
                "https://gitlab.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                oauth.client_id,
                url_encode(&redirect_uri),
                url_encode("api read_user read_repository"),
                state_param
            )
        }
        GitProviderType::Bitbucket => {
            let redirect_uri = oauth.redirect_uri.clone().unwrap_or_else(|| {
                format!(
                    "{}/api/auth/oauth/bitbucket/callback",
                    "http://localhost:8080"
                )
            });
            format!(
                "https://bitbucket.org/site/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                oauth.client_id,
                url_encode(&redirect_uri),
                url_encode("repository account"),
                state_param
            )
        }
    };

    Ok(Json(OAuthAuthorizationResponse {
        authorization_url,
        state: state_param,
    }))
}

/// Handle OAuth callback from provider
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(params): Query<OAuthCallbackRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let provider_type: GitProviderType = provider
        .parse()
        .map_err(|e: String| (StatusCode::BAD_REQUEST, e))?;

    let oauth_config = match provider_type {
        GitProviderType::Github => state.config.oauth.github.as_ref(),
        GitProviderType::Gitlab => state.config.oauth.gitlab.as_ref(),
        GitProviderType::Bitbucket => state.config.oauth.bitbucket.as_ref(),
    };

    let oauth = oauth_config.ok_or((
        StatusCode::NOT_FOUND,
        format!("{} OAuth is not configured", provider_type),
    ))?;

    // Exchange code for access token
    let (access_token, refresh_token, expires_at) = match provider_type {
        GitProviderType::Github => {
            github::exchange_token(&oauth.client_id, &oauth.client_secret, &params.code).await?
        }
        GitProviderType::Gitlab => {
            gitlab::exchange_token(
                &oauth.client_id,
                &oauth.client_secret,
                &params.code,
                oauth.redirect_uri.as_deref(),
            )
            .await?
        }
        GitProviderType::Bitbucket => {
            bitbucket::exchange_token(&oauth.client_id, &oauth.client_secret, &params.code).await?
        }
    };

    // Get user info from provider
    let user_info = match provider_type {
        GitProviderType::Github => github::get_user(&access_token).await?,
        GitProviderType::Gitlab => gitlab::get_user(&access_token).await?,
        GitProviderType::Bitbucket => bitbucket::get_user(&access_token).await?,
    };

    // Look up the first admin user for the OAuth callback context
    let admin_user: (String,) =
        sqlx::query_as("SELECT id FROM users WHERE role = 'admin' ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "No admin user found. Please complete setup first.".to_string(),
            ))?;
    let user_id = &admin_user.0;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Insert or update the provider connection
    sqlx::query(
        r#"
        INSERT INTO git_providers (id, user_id, provider, provider_user_id, username, display_name, email, avatar_url, access_token, refresh_token, token_expires_at, scopes, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, provider) DO UPDATE SET
            provider_user_id = excluded.provider_user_id,
            username = excluded.username,
            display_name = excluded.display_name,
            email = excluded.email,
            avatar_url = excluded.avatar_url,
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_expires_at = excluded.token_expires_at,
            scopes = excluded.scopes,
            updated_at = excluded.updated_at
        "#
    )
    .bind(&id)
    .bind(user_id)
    .bind(provider_type.to_string())
    .bind(&user_info.provider_user_id)
    .bind(&user_info.username)
    .bind(&user_info.display_name)
    .bind(&user_info.email)
    .bind(&user_info.avatar_url)
    .bind(&access_token)
    .bind(&refresh_token)
    .bind(&expires_at)
    .bind(&user_info.scopes)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Redirect to settings page with success message
    Ok(axum::response::Redirect::to(
        "/settings/git-providers?connected=true",
    ))
}

/// List repositories from a connected Git provider
pub async fn list_repos(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ListReposParams>,
) -> Result<Json<Vec<GitRepository>>, (StatusCode, String)> {
    let provider: GitProvider = sqlx::query_as("SELECT * FROM git_providers WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Provider not found".to_string()))?;

    let provider_type: GitProviderType = provider
        .provider
        .parse()
        .map_err(|e: String| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let repos: Vec<GitRepository> = match provider_type {
        GitProviderType::Github => {
            github::fetch_repos(&provider.access_token, params.page, params.per_page).await?
        }
        GitProviderType::Gitlab => {
            gitlab::fetch_repos(&provider.access_token, params.page, params.per_page).await?
        }
        GitProviderType::Bitbucket => {
            bitbucket::fetch_repos(&provider.access_token, params.page, params.per_page).await?
        }
    };

    Ok(Json(repos))
}
