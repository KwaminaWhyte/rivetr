//! OAuth provider and user connection models for social login.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Configured OAuth provider for social login (GitHub, Google)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthProvider {
    pub id: String,
    pub provider: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Public response for OAuth provider (excludes client_secret)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderPublic {
    pub provider: String,
    pub enabled: bool,
}

/// Full response for OAuth provider (admin view, still excludes full secret)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderResponse {
    pub id: String,
    pub provider: String,
    pub client_id: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OAuthProvider> for OAuthProviderResponse {
    fn from(p: OAuthProvider) -> Self {
        Self {
            id: p.id,
            provider: p.provider,
            client_id: p.client_id,
            enabled: p.enabled != 0,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Link between a Rivetr user and an OAuth provider account
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserOAuthConnection {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_name: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub created_at: String,
}

/// Public response for user OAuth connection (excludes tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOAuthConnectionResponse {
    pub id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_name: Option<String>,
    pub created_at: String,
}

impl From<UserOAuthConnection> for UserOAuthConnectionResponse {
    fn from(c: UserOAuthConnection) -> Self {
        Self {
            id: c.id,
            provider: c.provider,
            provider_user_id: c.provider_user_id,
            provider_email: c.provider_email,
            provider_name: c.provider_name,
            created_at: c.created_at,
        }
    }
}

/// Request to create/update an OAuth provider
#[derive(Debug, Deserialize)]
pub struct CreateOAuthProviderRequest {
    pub provider: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: Option<bool>,
}

/// Audit action constants for OAuth
pub mod oauth_actions {
    pub const OAUTH_LOGIN: &str = "auth.oauth_login";
    pub const OAUTH_PROVIDER_CREATE: &str = "oauth_provider.create";
    pub const OAUTH_PROVIDER_DELETE: &str = "oauth_provider.delete";
    pub const OAUTH_ACCOUNT_LINK: &str = "oauth_account.link";
    pub const OAUTH_ACCOUNT_UNLINK: &str = "oauth_account.unlink";
}

pub mod oauth_resource_types {
    pub const OAUTH_PROVIDER: &str = "oauth_provider";
}
