use axum::http::StatusCode;
use serde::Deserialize;

use crate::db::GitRepository;

use super::ProviderUserInfo;

// Helper struct for Bitbucket OAuth responses
#[derive(Debug, Deserialize)]
pub(super) struct BitbucketTokenResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

pub async fn exchange_token(
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

pub async fn get_user(access_token: &str) -> Result<ProviderUserInfo, (StatusCode, String)> {
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

/// Validate a Bitbucket API Token and get user info
pub async fn validate_api_token(
    username: &str,
    api_token: &str,
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
        .basic_auth(username, Some(api_token))
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
            "Invalid Bitbucket username or API Token".to_string(),
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

pub async fn fetch_repos(
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

/// Fetch Bitbucket repos using API Token (basic auth variant)
#[allow(dead_code)]
pub async fn fetch_repos_with_api_token(
    username: &str,
    api_token: &str,
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
        .basic_auth(username, Some(api_token))
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
