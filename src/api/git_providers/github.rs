use axum::http::StatusCode;
use serde::Deserialize;

use crate::db::GitRepository;

use super::ProviderUserInfo;

// Helper structs for GitHub OAuth responses
#[derive(Debug, Deserialize)]
pub(super) struct GitHubTokenResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub scope: Option<String>,
}

pub async fn exchange_token(
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

pub async fn get_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
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

pub async fn fetch_repos(
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
