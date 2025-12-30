//! GitHub Apps API endpoints for system-wide app registration and management.
//!
//! These endpoints handle:
//! - Initiating GitHub App manifest registration
//! - Handling registration callbacks from GitHub
//! - Managing GitHub App installations
//! - Listing repositories from installations

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::crypto;
use crate::db::{
    GitHubApp, GitHubAppInstallation, GitHubAppInstallationResponse, GitHubAppResponse,
    GitHubManifestCallbackResponse, InstallationCallbackQuery, ManifestCallbackQuery,
    ManifestRequest, ManifestStartResponse,
};
use crate::github::{get_installation_token, GitHubClient};
use crate::AppState;

/// Default permissions requested for the GitHub App
const DEFAULT_PERMISSIONS: &str = r#"{
    "contents": "read",
    "metadata": "read",
    "pull_requests": "write",
    "issues": "write",
    "statuses": "write",
    "checks": "write"
}"#;

/// Default events to subscribe to
const DEFAULT_EVENTS: &[&str] = &["push", "pull_request", "create", "delete"];

/// POST /api/github-apps/manifest - Initiate GitHub App manifest registration
///
/// Returns the manifest JSON and form action URL for the frontend to POST to GitHub.
/// The manifest includes the app configuration (permissions, events, webhook URL, etc.)
pub async fn create_manifest(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ManifestRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Generate a state parameter for CSRF protection
    let state_param = uuid::Uuid::new_v4().to_string();

    // Build the callback URL (where GitHub will redirect after registration)
    // Use external_url if configured (for ngrok/tunnels), otherwise build from host:port
    let api_host = state
        .config
        .server
        .external_url
        .clone()
        .unwrap_or_else(|| {
            format!(
                "http://{}:{}",
                state.config.server.host, state.config.server.api_port
            )
        });
    let callback_url = format!("{}/api/auth/github-apps/callback", api_host);
    let setup_url = format!("{}/api/auth/github-apps/installation/callback", api_host);

    // Build the webhook URL for the app
    let webhook_url = format!("{}/webhooks/github", api_host);

    // Create the manifest - use a unique name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp() % 10000;
    let app_name = format!("rivetr-deploy-{}", timestamp);

    let manifest = GitHubAppManifest {
        name: app_name,
        url: api_host.clone(),
        hook_attributes: HookAttributes {
            url: webhook_url,
            active: true,
        },
        redirect_url: callback_url.clone(),
        callback_urls: vec![callback_url],
        setup_url,
        description: Some("Automated deployment platform".to_string()),
        public: false,
        default_permissions: serde_json::from_str(DEFAULT_PERMISSIONS)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        default_events: DEFAULT_EVENTS.iter().map(|s| s.to_string()).collect(),
    };

    // Encode manifest as JSON
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Store state and metadata in a temporary way (in production, use a proper cache/session store)
    // For now, we encode it in the state parameter
    let state_data = StateData {
        is_system_wide: request.is_system_wide,
        team_id: request.team_id,
    };
    let state_json = serde_json::to_string(&state_data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Return the manifest JSON for the frontend to POST via form
    // GitHub requires POST to https://github.com/settings/apps/new with manifest in form body
    Ok(Json(ManifestStartResponse {
        manifest_url: "https://github.com/settings/apps/new".to_string(),
        manifest: manifest_json,
        state: format!("{}:{}", state_param, base64_encode(&state_json)),
    }))
}

/// GET /api/github-apps/callback - Handle GitHub App manifest registration callback
///
/// GitHub redirects here after the user creates the app from the manifest.
/// We receive a temporary code that we exchange for the app credentials.
pub async fn manifest_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ManifestCallbackQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Exchange the code for app credentials
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "https://api.github.com/app-manifests/{}/conversions",
            params.code
        ))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Rivetr")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("GitHub API error: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("GitHub API error: {} - {}", status, body),
        ));
    }

    let github_response: GitHubManifestCallbackResponse = response
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to parse response: {}", e)))?;

    // Parse state to get metadata
    let (is_system_wide, team_id) = if let Some(state_str) = &params.state {
        if let Some(pos) = state_str.find(':') {
            let encoded = &state_str[pos + 1..];
            if let Ok(decoded) = base64_decode(encoded) {
                if let Ok(state_data) = serde_json::from_str::<StateData>(&decoded) {
                    (state_data.is_system_wide, state_data.team_id)
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    // Encrypt sensitive fields
    let encryption_key = state.config.auth.encryption_key.as_ref().map(|k| crypto::derive_key(k));
    let encrypted_client_secret = crypto::encrypt_if_key_available(&github_response.client_secret, encryption_key.as_ref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let encrypted_private_key = crypto::encrypt_if_key_available(&github_response.pem, encryption_key.as_ref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let encrypted_webhook_secret = crypto::encrypt_if_key_available(&github_response.webhook_secret, encryption_key.as_ref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create the GitHub App record
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let created_by = "admin".to_string(); // TODO: Get from auth context

    sqlx::query(
        r#"
        INSERT INTO github_apps (
            id, name, app_id, client_id, client_secret, private_key, webhook_secret,
            slug, owner, permissions, events, is_system_wide, team_id,
            created_at, updated_at, created_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&github_response.name)
    .bind(github_response.id)
    .bind(&github_response.client_id)
    .bind(&encrypted_client_secret)
    .bind(&encrypted_private_key)
    .bind(&encrypted_webhook_secret)
    .bind(&github_response.slug)
    .bind(&github_response.owner.login)
    .bind(serde_json::to_string(&github_response.permissions).ok())
    .bind(serde_json::to_string(&github_response.events).ok())
    .bind(is_system_wide)
    .bind(&team_id)
    .bind(&now)
    .bind(&now)
    .bind(&created_by)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Auto-redirect to GitHub to install the app immediately
    // This streamlines the flow: create app → install app → back to Rivetr
    let install_url = format!(
        "https://github.com/apps/{}/installations/new",
        github_response.slug
    );
    Ok(Redirect::to(&install_url))
}

/// GET /api/github-apps - List all accessible GitHub Apps
pub async fn list_apps(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // For now, return all apps (in production, filter by team membership)
    let apps: Vec<GitHubApp> = sqlx::query_as(
        "SELECT * FROM github_apps ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<GitHubAppResponse> = apps.into_iter().map(GitHubAppResponse::from).collect();
    Ok(Json(responses))
}

/// GET /api/github-apps/:id - Get a specific GitHub App
pub async fn get_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let app: GitHubApp = sqlx::query_as("SELECT * FROM github_apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "GitHub App not found".to_string()))?;

    Ok(Json(GitHubAppResponse::from(app)))
}

/// GET /api/github-apps/:id/install - Get the installation URL for an app
pub async fn get_install_url(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let app: GitHubApp = sqlx::query_as("SELECT * FROM github_apps WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "GitHub App not found".to_string()))?;

    // The installation URL uses the app slug
    let install_url = if let Some(slug) = &app.slug {
        format!("https://github.com/apps/{}/installations/new", slug)
    } else {
        format!("https://github.com/settings/apps/{}/installations", app.name)
    };

    Ok(Json(InstallUrlResponse { install_url }))
}

/// GET /api/github-apps/installation/callback - Handle installation callback from GitHub
pub async fn installation_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<InstallationCallbackQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // The installation_id tells us which app was installed
    // We need to look up which GitHub App this installation belongs to

    // First, we need to find the app by checking each registered app
    let apps: Vec<GitHubApp> = sqlx::query_as("SELECT * FROM github_apps")
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let encryption_key = state.config.auth.encryption_key.as_ref().map(|k| crypto::derive_key(k));

    // Try to find the app that owns this installation
    let mut found_app: Option<GitHubApp> = None;
    let mut installation_info: Option<crate::github::api_client::Installation> = None;

    for app in apps {
        // Decrypt the private key to make API calls
        let private_key = crypto::decrypt_if_encrypted(&app.private_key, encryption_key.as_ref())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Get an installation token using the app JWT
        let token_result = get_installation_token(app.app_id, &private_key, params.installation_id).await;

        if let Ok(token_response) = token_result {
            // This app owns the installation
            let client = GitHubClient::new(token_response.token.clone());

            // We found the right app, now get installation details
            // For now, we'll construct the installation info from what we have
            found_app = Some(app);

            // Store the access token (for future use in caching)
            let _encrypted_token = crypto::encrypt_if_key_available(&token_response.token, encryption_key.as_ref())
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            // We need to get installation details from GitHub
            // Using the app JWT to list installations
            if let Ok(repos) = client.list_repos(1, 1).await {
                // We got a valid response, the token works
                // Get account info from the first repo if available
                if let Some(repo) = repos.first() {
                    installation_info = Some(crate::github::api_client::Installation {
                        id: params.installation_id as u64,
                        account: crate::github::api_client::InstallationAccount {
                            login: repo.owner.login.clone(),
                            id: repo.owner.id,
                            account_type: repo.owner.owner_type.clone(),
                            avatar_url: None,
                        },
                        repository_selection: token_response.repository_selection.unwrap_or_else(|| "all".to_string()),
                        access_tokens_url: String::new(),
                        repositories_url: String::new(),
                        html_url: String::new(),
                        app_id: found_app.as_ref().unwrap().app_id as u64,
                        target_id: repo.owner.id,
                        target_type: repo.owner.owner_type.clone(),
                        permissions: token_response.permissions,
                        events: vec![],
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        suspended_at: None,
                    });
                }
            }
            break;
        }
    }

    let app = found_app.ok_or((StatusCode::NOT_FOUND, "No registered app owns this installation".to_string()))?;

    // Create the installation record
    let installation_id_uuid = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Use installation_info if available, otherwise use defaults
    let (account_type, account_login, account_id, repository_selection, permissions) =
        if let Some(info) = installation_info {
            (
                info.account.account_type,
                info.account.login,
                info.account.id as i64,
                Some(info.repository_selection),
                Some(serde_json::to_string(&info.permissions).unwrap_or_default()),
            )
        } else {
            ("unknown".to_string(), "unknown".to_string(), 0i64, None, None)
        };

    sqlx::query(
        r#"
        INSERT INTO github_app_installations (
            id, github_app_id, installation_id, account_type, account_login, account_id,
            permissions, repository_selection, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(github_app_id, installation_id) DO UPDATE SET
            account_type = excluded.account_type,
            account_login = excluded.account_login,
            permissions = excluded.permissions,
            repository_selection = excluded.repository_selection,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&installation_id_uuid)
    .bind(&app.id)
    .bind(params.installation_id)
    .bind(&account_type)
    .bind(&account_login)
    .bind(account_id)
    .bind(&permissions)
    .bind(&repository_selection)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Redirect to settings page with success
    Ok(Redirect::to(&format!(
        "/settings/github-apps?installed=true&installation_id={}",
        params.installation_id
    )))
}

/// GET /api/github-apps/:id/installations - List installations for a GitHub App
pub async fn list_installations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let installations: Vec<GitHubAppInstallation> = sqlx::query_as(
        "SELECT * FROM github_app_installations WHERE github_app_id = ? ORDER BY created_at DESC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<GitHubAppInstallationResponse> = installations
        .into_iter()
        .map(GitHubAppInstallationResponse::from)
        .collect();

    Ok(Json(responses))
}

/// GET /api/github-apps/installations - List ALL installations across all GitHub Apps
///
/// Returns all installations the user has access to, enabling a single dropdown
/// for repository selection in the app creation form.
pub async fn list_all_installations(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get all installations with their associated app info
    let installations: Vec<GitHubAppInstallation> = sqlx::query_as(
        r#"
        SELECT i.* FROM github_app_installations i
        JOIN github_apps a ON i.github_app_id = a.id
        ORDER BY i.account_login ASC, i.created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let responses: Vec<GitHubAppInstallationResponse> = installations
        .into_iter()
        .map(GitHubAppInstallationResponse::from)
        .collect();

    Ok(Json(responses))
}

/// GET /api/github-apps/installations/:installation_id/repos - List repos by installation ID
///
/// A simpler endpoint that fetches repositories using just the installation's internal ID.
/// This is more convenient for the frontend as it doesn't require knowing the app ID.
pub async fn list_repos_by_installation(
    State(state): State<Arc<AppState>>,
    Path(installation_id): Path<String>,
    Query(params): Query<ListReposParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get the installation
    let installation: GitHubAppInstallation = sqlx::query_as(
        "SELECT * FROM github_app_installations WHERE id = ?",
    )
    .bind(&installation_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Installation not found".to_string()))?;

    // Get the associated GitHub App
    let app: GitHubApp = sqlx::query_as("SELECT * FROM github_apps WHERE id = ?")
        .bind(&installation.github_app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "GitHub App not found".to_string()))?;

    // Decrypt the private key
    let encryption_key = state.config.auth.encryption_key.as_ref().map(|k| crypto::derive_key(k));
    let private_key = crypto::decrypt_if_encrypted(&app.private_key, encryption_key.as_ref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get an installation access token
    let token_response = get_installation_token(app.app_id, &private_key, installation.installation_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to get installation token: {}", e)))?;

    // List repositories
    let client = GitHubClient::new(token_response.token);
    let repos = client
        .list_repos(params.per_page, params.page)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to list repos: {}", e)))?;

    Ok(Json(repos))
}

/// GET /api/github-apps/:id/installations/:iid/repos - List repos for an installation
pub async fn list_installation_repos(
    State(state): State<Arc<AppState>>,
    Path((app_id, installation_id)): Path<(String, String)>,
    Query(params): Query<ListReposParams>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get the app
    let app: GitHubApp = sqlx::query_as("SELECT * FROM github_apps WHERE id = ?")
        .bind(&app_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "GitHub App not found".to_string()))?;

    // Get the installation
    let installation: GitHubAppInstallation = sqlx::query_as(
        "SELECT * FROM github_app_installations WHERE id = ? AND github_app_id = ?",
    )
    .bind(&installation_id)
    .bind(&app_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Installation not found".to_string()))?;

    // Decrypt the private key
    let encryption_key = state.config.auth.encryption_key.as_ref().map(|k| crypto::derive_key(k));
    let private_key = crypto::decrypt_if_encrypted(&app.private_key, encryption_key.as_ref())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get an installation access token
    let token_response = get_installation_token(app.app_id, &private_key, installation.installation_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to get installation token: {}", e)))?;

    // List repositories
    let client = GitHubClient::new(token_response.token);
    let repos = client
        .list_repos(params.per_page, params.page)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Failed to list repos: {}", e)))?;

    Ok(Json(repos))
}

// Helper types

#[derive(Debug, Serialize, Deserialize)]
struct StateData {
    is_system_wide: bool,
    team_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubAppManifest {
    name: String,
    url: String,
    hook_attributes: HookAttributes,
    redirect_url: String,
    callback_urls: Vec<String>,
    setup_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    public: bool,
    default_permissions: serde_json::Value,
    default_events: Vec<String>,
}

#[derive(Debug, Serialize)]
struct HookAttributes {
    url: String,
    active: bool,
}

#[derive(Debug, Serialize)]
struct InstallUrlResponse {
    install_url: String,
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

// Base64 helpers
fn base64_encode(s: &str) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.encode(s.as_bytes())
}

fn base64_decode(s: &str) -> Result<String, base64::DecodeError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bytes = STANDARD.decode(s)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}
