//! Git provider models for OAuth integration.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitProviderType {
    Github,
    Gitlab,
    Bitbucket,
}

impl std::fmt::Display for GitProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Github => write!(f, "github"),
            Self::Gitlab => write!(f, "gitlab"),
            Self::Bitbucket => write!(f, "bitbucket"),
        }
    }
}

impl std::str::FromStr for GitProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Self::Github),
            "gitlab" => Ok(Self::Gitlab),
            "bitbucket" => Ok(Self::Bitbucket),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GitProvider {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<String>,
    pub scopes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that excludes tokens for security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitProviderResponse {
    pub id: String,
    pub provider: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub scopes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GitProvider> for GitProviderResponse {
    fn from(p: GitProvider) -> Self {
        Self {
            id: p.id,
            provider: p.provider,
            username: p.username,
            display_name: p.display_name,
            email: p.email,
            avatar_url: p.avatar_url,
            scopes: p.scopes,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Repository info from Git provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepository {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub private: bool,
    pub owner: String,
}

/// OAuth callback request
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

/// OAuth authorization URL response
#[derive(Debug, Serialize)]
pub struct OAuthAuthorizationResponse {
    pub authorization_url: String,
    pub state: String,
}
