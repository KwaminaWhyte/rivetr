use axum::http::StatusCode;
use serde::Deserialize;

use crate::db::GitRepository;

use super::ProviderUserInfo;

// Helper struct for GitLab OAuth responses
#[derive(Debug, Deserialize)]
pub(super) struct GitLabTokenResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    #[allow(dead_code)]
    pub created_at: Option<i64>,
}

pub async fn exchange_token(
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

pub async fn get_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
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

/// Validate a GitLab Personal Access Token and get user info
pub async fn validate_token(token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
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

pub async fn fetch_repos(
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

/// Fetch GitLab repos using Personal Access Token (PAT-specific endpoint)
#[allow(dead_code)]
pub async fn fetch_repos_with_pat(
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
