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
        GitProviderType::Gitlab => validate_gitlab_token(&req.token).await?,
        GitProviderType::Bitbucket => {
            let username = req.username.ok_or((
                StatusCode::BAD_REQUEST,
                "Username is required for Bitbucket App Password".to_string(),
            ))?;
            validate_bitbucket_app_password(&username, &req.token).await?
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

/// Validate a GitLab Personal Access Token and get user info
async fn validate_gitlab_token(token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitLabUser {
        id: i64,
        username: String,
        name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
    }

    let response = client
        .get("https://gitlab.com/api/v4/user")
        .header("PRIVATE-TOKEN", token)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to validate token: {}", e),
            )
        })?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid GitLab Personal Access Token".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("GitLab API error: {}", response.status()),
        ));
    }

    let user: GitLabUser = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse user info: {}", e),
        )
    })?;

    Ok(ProviderUserInfo {
        provider_user_id: user.id.to_string(),
        username: user.username,
        display_name: user.name,
        email: user.email,
        avatar_url: user.avatar_url,
        scopes: Some("api read_repository".to_string()),
    })
}

/// Validate a Bitbucket App Password and get user info
async fn validate_bitbucket_app_password(
    username: &str,
    app_password: &str,
) -> Result<ProviderUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct BitbucketUser {
        uuid: String,
        username: String,
        display_name: Option<String>,
        links: BitbucketUserLinks,
    }

    #[derive(Deserialize)]
    struct BitbucketUserLinks {
        avatar: Option<BitbucketLink>,
    }

    #[derive(Deserialize)]
    struct BitbucketLink {
        href: String,
    }

    let response = client
        .get("https://api.bitbucket.org/2.0/user")
        .basic_auth(username, Some(app_password))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to validate credentials: {}", e),
            )
        })?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid Bitbucket username or App Password".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Bitbucket API error: {}", response.status()),
        ));
    }

    let user: BitbucketUser = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse user info: {}", e),
        )
    })?;

    Ok(ProviderUserInfo {
        provider_user_id: user.uuid,
        username: user.username,
        display_name: user.display_name,
        email: None,
        avatar_url: user.links.avatar.map(|a| a.href),
        scopes: Some("repository account".to_string()),
    })
}

/// Fetch GitLab repos using Personal Access Token
pub async fn fetch_gitlab_repos_with_pat(
    access_token: &str,
    page: u32,
    per_page: u32,
) -> Result<Vec<GitRepository>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitLabProject {
        id: i64,
        name: String,
        path_with_namespace: String,
        description: Option<String>,
        web_url: String,
        http_url_to_repo: String,
        ssh_url_to_repo: String,
        default_branch: Option<String>,
        visibility: String,
        namespace: GitLabNamespace,
    }

    #[derive(Deserialize)]
    struct GitLabNamespace {
        name: String,
    }

    let response = client
        .get(format!("https://gitlab.com/api/v4/projects?membership=true&page={}&per_page={}&order_by=updated_at", page, per_page))
        .header("PRIVATE-TOKEN", access_token)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to fetch repos: {}", e)))?;

    let repos: Vec<GitLabProject> = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse repos: {}", e),
        )
    })?;

    Ok(repos
        .into_iter()
        .map(|r| GitRepository {
            id: r.id.to_string(),
            name: r.name,
            full_name: r.path_with_namespace,
            description: r.description,
            html_url: r.web_url,
            clone_url: r.http_url_to_repo,
            ssh_url: r.ssh_url_to_repo,
            default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
            private: r.visibility != "public",
            owner: r.namespace.name,
        })
        .collect())
}

/// Fetch Bitbucket repos using App Password
pub async fn fetch_bitbucket_repos_with_app_password(
    username: &str,
    app_password: &str,
    page: u32,
    per_page: u32,
) -> Result<Vec<GitRepository>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct BitbucketResponse {
        values: Vec<BitbucketRepo>,
    }

    #[derive(Deserialize)]
    struct BitbucketRepo {
        uuid: String,
        name: String,
        full_name: String,
        description: Option<String>,
        is_private: bool,
        links: BitbucketRepoLinks,
        mainbranch: Option<BitbucketBranch>,
        owner: BitbucketOwner,
    }

    #[derive(Deserialize)]
    struct BitbucketRepoLinks {
        html: BitbucketRepoLink,
        clone: Vec<BitbucketCloneLink>,
    }

    #[derive(Deserialize)]
    struct BitbucketRepoLink {
        href: String,
    }

    #[derive(Deserialize)]
    struct BitbucketCloneLink {
        name: String,
        href: String,
    }

    #[derive(Deserialize)]
    struct BitbucketBranch {
        name: String,
    }

    #[derive(Deserialize)]
    struct BitbucketOwner {
        username: String,
    }

    let response = client
        .get(format!(
            "https://api.bitbucket.org/2.0/repositories/{}?page={}&pagelen={}",
            username, page, per_page
        ))
        .basic_auth(username, Some(app_password))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch repos: {}", e),
            )
        })?;

    let repos: BitbucketResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse repos: {}", e),
        )
    })?;

    Ok(repos
        .values
        .into_iter()
        .map(|r| {
            let https_url = r
                .links
                .clone
                .iter()
                .find(|c| c.name == "https")
                .map(|c| c.href.clone())
                .unwrap_or_default();
            let ssh_url = r
                .links
                .clone
                .iter()
                .find(|c| c.name == "ssh")
                .map(|c| c.href.clone())
                .unwrap_or_default();

            GitRepository {
                id: r.uuid,
                name: r.name,
                full_name: r.full_name,
                description: r.description,
                html_url: r.links.html.href,
                clone_url: https_url,
                ssh_url,
                default_branch: r
                    .mainbranch
                    .map(|b| b.name)
                    .unwrap_or_else(|| "main".to_string()),
                private: r.is_private,
                owner: r.owner.username,
            }
        })
        .collect())
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
            exchange_github_token(&oauth.client_id, &oauth.client_secret, &params.code).await?
        }
        GitProviderType::Gitlab => {
            exchange_gitlab_token(
                &oauth.client_id,
                &oauth.client_secret,
                &params.code,
                oauth.redirect_uri.as_deref(),
            )
            .await?
        }
        GitProviderType::Bitbucket => {
            exchange_bitbucket_token(&oauth.client_id, &oauth.client_secret, &params.code).await?
        }
    };

    // Get user info from provider
    let user_info = match provider_type {
        GitProviderType::Github => get_github_user(&access_token).await?,
        GitProviderType::Gitlab => get_gitlab_user(&access_token).await?,
        GitProviderType::Bitbucket => get_bitbucket_user(&access_token).await?,
    };

    // Look up the first admin user for the OAuth callback context
    let admin_user: (String,) = sqlx::query_as("SELECT id FROM users WHERE role = 'admin' ORDER BY created_at ASC LIMIT 1")
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "No admin user found. Please complete setup first.".to_string()))?;
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
            fetch_github_repos(&provider.access_token, params.page, params.per_page).await?
        }
        GitProviderType::Gitlab => {
            fetch_gitlab_repos(&provider.access_token, params.page, params.per_page).await?
        }
        GitProviderType::Bitbucket => {
            fetch_bitbucket_repos(&provider.access_token, params.page, params.per_page).await?
        }
    };

    Ok(Json(repos))
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

// Helper structs for OAuth responses
#[derive(Debug, Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    token_type: String,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabTokenResponse {
    access_token: String,
    token_type: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    created_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BitbucketTokenResponse {
    access_token: String,
    token_type: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

struct ProviderUserInfo {
    provider_user_id: String,
    username: String,
    display_name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
    scopes: Option<String>,
}

// GitHub OAuth functions
async fn exchange_github_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<(String, Option<String>, Option<String>), (StatusCode, String)> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
        ])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to exchange token: {}", e),
            )
        })?;

    let token_response: GitHubTokenResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse token response: {}", e),
        )
    })?;

    Ok((token_response.access_token, None, None))
}

async fn get_github_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitHubUser {
        id: i64,
        login: String,
        name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
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

    Ok(ProviderUserInfo {
        provider_user_id: user.id.to_string(),
        username: user.login,
        display_name: user.name,
        email: user.email,
        avatar_url: user.avatar_url,
        scopes: Some("repo read:user user:email".to_string()),
    })
}

async fn fetch_github_repos(
    access_token: &str,
    page: u32,
    per_page: u32,
) -> Result<Vec<GitRepository>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitHubRepo {
        id: i64,
        name: String,
        full_name: String,
        description: Option<String>,
        html_url: String,
        clone_url: String,
        ssh_url: String,
        default_branch: String,
        private: bool,
        owner: GitHubOwner,
    }

    #[derive(Deserialize)]
    struct GitHubOwner {
        login: String,
    }

    let response = client
        .get(format!(
            "https://api.github.com/user/repos?page={}&per_page={}&sort=updated",
            page, per_page
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Rivetr")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch repos: {}", e),
            )
        })?;

    let repos: Vec<GitHubRepo> = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse repos: {}", e),
        )
    })?;

    Ok(repos
        .into_iter()
        .map(|r| GitRepository {
            id: r.id.to_string(),
            name: r.name,
            full_name: r.full_name,
            description: r.description,
            html_url: r.html_url,
            clone_url: r.clone_url,
            ssh_url: r.ssh_url,
            default_branch: r.default_branch,
            private: r.private,
            owner: r.owner.login,
        })
        .collect())
}

// GitLab OAuth functions
async fn exchange_gitlab_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: Option<&str>,
) -> Result<(String, Option<String>, Option<String>), (StatusCode, String)> {
    let client = reqwest::Client::new();
    let redirect = redirect_uri.unwrap_or("http://localhost:8080/api/auth/oauth/gitlab/callback");

    let response = client
        .post("https://gitlab.com/oauth/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect),
        ])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to exchange token: {}", e),
            )
        })?;

    let token_response: GitLabTokenResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse token response: {}", e),
        )
    })?;

    let expires_at = token_response.expires_in.map(|e| {
        chrono::Utc::now()
            .checked_add_signed(chrono::Duration::seconds(e))
            .map(|t| t.to_rfc3339())
            .unwrap_or_default()
    });

    Ok((
        token_response.access_token,
        token_response.refresh_token,
        expires_at,
    ))
}

async fn get_gitlab_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitLabUser {
        id: i64,
        username: String,
        name: Option<String>,
        email: Option<String>,
        avatar_url: Option<String>,
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

    Ok(ProviderUserInfo {
        provider_user_id: user.id.to_string(),
        username: user.username,
        display_name: user.name,
        email: user.email,
        avatar_url: user.avatar_url,
        scopes: Some("api read_user read_repository".to_string()),
    })
}

async fn fetch_gitlab_repos(
    access_token: &str,
    page: u32,
    per_page: u32,
) -> Result<Vec<GitRepository>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct GitLabProject {
        id: i64,
        name: String,
        path_with_namespace: String,
        description: Option<String>,
        web_url: String,
        http_url_to_repo: String,
        ssh_url_to_repo: String,
        default_branch: Option<String>,
        visibility: String,
        namespace: GitLabNamespace,
    }

    #[derive(Deserialize)]
    struct GitLabNamespace {
        name: String,
    }

    let response = client
        .get(format!("https://gitlab.com/api/v4/projects?membership=true&page={}&per_page={}&order_by=updated_at", page, per_page))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to fetch repos: {}", e)))?;

    let repos: Vec<GitLabProject> = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse repos: {}", e),
        )
    })?;

    Ok(repos
        .into_iter()
        .map(|r| GitRepository {
            id: r.id.to_string(),
            name: r.name,
            full_name: r.path_with_namespace,
            description: r.description,
            html_url: r.web_url,
            clone_url: r.http_url_to_repo,
            ssh_url: r.ssh_url_to_repo,
            default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
            private: r.visibility != "public",
            owner: r.namespace.name,
        })
        .collect())
}

// Bitbucket OAuth functions (simplified)
async fn exchange_bitbucket_token(
    client_id: &str,
    client_secret: &str,
    code: &str,
) -> Result<(String, Option<String>, Option<String>), (StatusCode, String)> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://bitbucket.org/site/oauth2/access_token")
        .basic_auth(client_id, Some(client_secret))
        .form(&[("grant_type", "authorization_code"), ("code", code)])
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to exchange token: {}", e),
            )
        })?;

    let token_response: BitbucketTokenResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse token response: {}", e),
        )
    })?;

    let expires_at = token_response.expires_in.map(|e| {
        chrono::Utc::now()
            .checked_add_signed(chrono::Duration::seconds(e))
            .map(|t| t.to_rfc3339())
            .unwrap_or_default()
    });

    Ok((
        token_response.access_token,
        token_response.refresh_token,
        expires_at,
    ))
}

async fn get_bitbucket_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct BitbucketUser {
        uuid: String,
        username: String,
        display_name: Option<String>,
        links: BitbucketLinks,
    }

    #[derive(Deserialize)]
    struct BitbucketLinks {
        avatar: Option<BitbucketLink>,
    }

    #[derive(Deserialize)]
    struct BitbucketLink {
        href: String,
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

    Ok(ProviderUserInfo {
        provider_user_id: user.uuid,
        username: user.username,
        display_name: user.display_name,
        email: None, // Would need separate API call
        avatar_url: user.links.avatar.map(|a| a.href),
        scopes: Some("repository account".to_string()),
    })
}

async fn fetch_bitbucket_repos(
    access_token: &str,
    page: u32,
    per_page: u32,
) -> Result<Vec<GitRepository>, (StatusCode, String)> {
    let client = reqwest::Client::new();

    #[derive(Deserialize)]
    struct BitbucketResponse {
        values: Vec<BitbucketRepo>,
    }

    #[derive(Deserialize)]
    struct BitbucketRepo {
        uuid: String,
        name: String,
        full_name: String,
        description: Option<String>,
        is_private: bool,
        links: BitbucketRepoLinks,
        mainbranch: Option<BitbucketBranch>,
        owner: BitbucketOwner,
    }

    #[derive(Deserialize)]
    struct BitbucketRepoLinks {
        html: BitbucketLink,
        clone: Vec<BitbucketCloneLink>,
    }

    #[derive(Deserialize)]
    struct BitbucketCloneLink {
        name: String,
        href: String,
    }

    #[derive(Deserialize)]
    struct BitbucketLink {
        href: String,
    }

    #[derive(Deserialize)]
    struct BitbucketBranch {
        name: String,
    }

    #[derive(Deserialize)]
    struct BitbucketOwner {
        username: String,
    }

    let response = client
        .get(format!(
            "https://api.bitbucket.org/2.0/repositories?role=member&page={}&pagelen={}",
            page, per_page
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to fetch repos: {}", e),
            )
        })?;

    let repos: BitbucketResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to parse repos: {}", e),
        )
    })?;

    Ok(repos
        .values
        .into_iter()
        .map(|r| {
            let https_url = r
                .links
                .clone
                .iter()
                .find(|c| c.name == "https")
                .map(|c| c.href.clone())
                .unwrap_or_default();
            let ssh_url = r
                .links
                .clone
                .iter()
                .find(|c| c.name == "ssh")
                .map(|c| c.href.clone())
                .unwrap_or_default();

            GitRepository {
                id: r.uuid,
                name: r.name,
                full_name: r.full_name,
                description: r.description,
                html_url: r.links.html.href,
                clone_url: https_url,
                ssh_url,
                default_branch: r
                    .mainbranch
                    .map(|b| b.name)
                    .unwrap_or_else(|| "main".to_string()),
                private: r.is_private,
                owner: r.owner.username,
            }
        })
        .collect())
}
