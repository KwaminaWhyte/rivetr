//! OIDC/SSO provider models for enterprise single sign-on.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Configured OIDC provider for enterprise SSO login
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OidcProvider {
    pub id: String,
    pub name: String,
    pub client_id: String,
    pub client_secret: String, // encrypted at rest
    pub discovery_url: String,
    pub redirect_uri: String,
    pub scopes: String,
    pub enabled: i64,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to create an OIDC provider
#[derive(Debug, Deserialize)]
pub struct CreateOidcProviderRequest {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub discovery_url: String,
    pub redirect_uri: Option<String>,
    pub scopes: Option<String>,
    pub team_id: Option<String>,
    pub enabled: Option<bool>,
}

/// Public response for OIDC provider (excludes client_secret)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcProviderResponse {
    pub id: String,
    pub name: String,
    pub client_id: String,
    // client_secret NOT included
    pub discovery_url: String,
    pub redirect_uri: String,
    pub scopes: String,
    pub enabled: bool,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OidcProvider> for OidcProviderResponse {
    fn from(p: OidcProvider) -> Self {
        Self {
            id: p.id,
            name: p.name,
            client_id: p.client_id,
            discovery_url: p.discovery_url,
            redirect_uri: p.redirect_uri,
            scopes: p.scopes,
            enabled: p.enabled != 0,
            team_id: p.team_id,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// SSO state entry for CSRF protection during OIDC flow
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SsoState {
    pub state: String,
    pub provider_id: String,
    pub redirect_to: Option<String>,
    pub created_at: String,
}

/// Audit action constants for SSO/OIDC
pub mod sso_actions {
    pub const SSO_LOGIN: &str = "auth.sso_login";
    pub const SSO_PROVIDER_CREATE: &str = "sso_provider.create";
    pub const SSO_PROVIDER_UPDATE: &str = "sso_provider.update";
    pub const SSO_PROVIDER_DELETE: &str = "sso_provider.delete";
}

pub mod sso_resource_types {
    pub const SSO_PROVIDER: &str = "sso_provider";
}
