//! GitHub App models for system-wide app registration and installations.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A GitHub App registered via the manifest flow.
/// System-wide apps are available to all teams; team-scoped apps belong to a specific team.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GitHubApp {
    /// Internal UUID
    pub id: String,
    /// App name on GitHub
    pub name: String,
    /// GitHub App ID (numeric)
    pub app_id: i64,
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret (encrypted)
    pub client_secret: String,
    /// PEM private key for JWT signing (encrypted)
    pub private_key: String,
    /// Webhook secret for signature verification (encrypted)
    pub webhook_secret: String,

    // Optional fields
    /// GitHub App slug (URL-friendly name)
    pub slug: Option<String>,
    /// Owner (user or org) that owns the app on GitHub
    pub owner: Option<String>,
    /// JSON-encoded granted permissions
    pub permissions: Option<String>,
    /// JSON-encoded subscribed events
    pub events: Option<String>,

    // Sharing settings
    /// If true, available to all teams
    pub is_system_wide: bool,
    /// Owner team (if not system-wide)
    pub team_id: Option<String>,

    // Metadata
    pub created_at: String,
    pub updated_at: String,
    /// User who registered the app
    pub created_by: String,
}

/// Response DTO that excludes sensitive fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAppResponse {
    pub id: String,
    pub name: String,
    pub app_id: i64,
    pub slug: Option<String>,
    pub owner: Option<String>,
    pub permissions: Option<serde_json::Value>,
    pub events: Option<serde_json::Value>,
    pub is_system_wide: bool,
    pub team_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
}

impl From<GitHubApp> for GitHubAppResponse {
    fn from(app: GitHubApp) -> Self {
        Self {
            id: app.id,
            name: app.name,
            app_id: app.app_id,
            slug: app.slug,
            owner: app.owner,
            permissions: app.permissions.and_then(|p| serde_json::from_str(&p).ok()),
            events: app.events.and_then(|e| serde_json::from_str(&e).ok()),
            is_system_wide: app.is_system_wide,
            team_id: app.team_id,
            created_at: app.created_at,
            updated_at: app.updated_at,
            created_by: app.created_by,
        }
    }
}

/// A GitHub App installation (per org/user account).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GitHubAppInstallation {
    /// Internal UUID
    pub id: String,
    /// Reference to the parent GitHub App
    pub github_app_id: String,
    /// GitHub installation ID (numeric)
    pub installation_id: i64,
    /// Account type: 'user' or 'organization'
    pub account_type: String,
    /// GitHub username or org name
    pub account_login: String,
    /// GitHub account ID (numeric)
    pub account_id: i64,

    // Token management
    /// Current installation access token (encrypted)
    pub access_token: Option<String>,
    /// Token expiration timestamp
    pub token_expires_at: Option<String>,

    // Permissions snapshot
    /// JSON-encoded granted permissions at install time
    pub permissions: Option<String>,
    /// Repository selection: 'all' or 'selected'
    pub repository_selection: Option<String>,

    // Metadata
    /// If installation is suspended
    pub suspended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO for installations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAppInstallationResponse {
    pub id: String,
    pub github_app_id: String,
    pub installation_id: i64,
    pub account_type: String,
    pub account_login: String,
    pub account_id: i64,
    pub permissions: Option<serde_json::Value>,
    pub repository_selection: Option<String>,
    pub suspended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GitHubAppInstallation> for GitHubAppInstallationResponse {
    fn from(inst: GitHubAppInstallation) -> Self {
        Self {
            id: inst.id,
            github_app_id: inst.github_app_id,
            installation_id: inst.installation_id,
            account_type: inst.account_type,
            account_login: inst.account_login,
            account_id: inst.account_id,
            permissions: inst.permissions.and_then(|p| serde_json::from_str(&p).ok()),
            repository_selection: inst.repository_selection,
            suspended_at: inst.suspended_at,
            created_at: inst.created_at,
            updated_at: inst.updated_at,
        }
    }
}

/// Request payload for initiating GitHub App manifest registration
#[derive(Debug, Deserialize)]
pub struct ManifestRequest {
    /// Whether this app should be system-wide (admin only)
    #[serde(default)]
    pub is_system_wide: bool,
    /// Team to associate the app with (if not system-wide)
    pub team_id: Option<String>,
}

/// Response from GitHub's manifest registration callback
#[derive(Debug, Deserialize)]
pub struct GitHubManifestCallbackResponse {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub pem: String,
    pub webhook_secret: String,
    pub owner: GitHubAppOwner,
    pub permissions: serde_json::Value,
    pub events: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAppOwner {
    pub login: String,
    #[serde(rename = "type")]
    pub owner_type: String,
}

/// Response containing the manifest for registration
#[derive(Debug, Serialize)]
pub struct ManifestStartResponse {
    /// URL to POST the manifest form to
    pub manifest_url: String,
    /// The manifest JSON to submit as form data
    pub manifest: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// Callback query parameters from GitHub after manifest registration
#[derive(Debug, Deserialize)]
pub struct ManifestCallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

/// Installation callback query parameters
#[derive(Debug, Deserialize)]
pub struct InstallationCallbackQuery {
    pub installation_id: i64,
    pub setup_action: Option<String>,
}
