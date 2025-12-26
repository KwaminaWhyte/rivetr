use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub branch: String,
    pub dockerfile: String,
    pub domain: Option<String>,
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Cloning,
    Building,
    Starting,
    Checking,
    Running,
    Failed,
    Stopped,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Cloning => write!(f, "cloning"),
            Self::Building => write!(f, "building"),
            Self::Starting => write!(f, "starting"),
            Self::Checking => write!(f, "checking"),
            Self::Running => write!(f, "running"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
        }
    }
}

impl From<String> for DeploymentStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => Self::Pending,
            "cloning" => Self::Cloning,
            "building" => Self::Building,
            "starting" => Self::Starting,
            "checking" => Self::Checking,
            "running" => Self::Running,
            "failed" => Self::Failed,
            "stopped" => Self::Stopped,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    pub status: String,
    pub container_id: Option<String>,
    pub image_tag: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl Deployment {
    pub fn status_enum(&self) -> DeploymentStatus {
        DeploymentStatus::from(self.status.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentLog {
    pub id: i64,
    pub deployment_id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

// DTOs for API

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    pub git_url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_dockerfile")]
    pub dockerfile: String,
    pub domain: Option<String>,
    #[serde(default = "default_port")]
    pub port: i32,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_dockerfile() -> String {
    "./Dockerfile".to_string()
}

fn default_port() -> i32 {
    3000
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppRequest {
    pub name: Option<String>,
    pub git_url: Option<String>,
    pub branch: Option<String>,
    pub dockerfile: Option<String>,
    pub domain: Option<String>,
    pub port: Option<i32>,
    pub healthcheck: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub ssh_key_id: Option<String>,
}

// User models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            role: user.role,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

// SSH Key models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SshKey {
    pub id: String,
    pub name: String,
    pub private_key: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that excludes the private key for security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: String,
    pub name: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SshKey> for SshKeyResponse {
    fn from(key: SshKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            public_key: key.public_key,
            app_id: key.app_id,
            is_global: key.is_global != 0,
            created_at: key.created_at,
            updated_at: key.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSshKeyRequest {
    pub name: String,
    pub private_key: String,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    #[serde(default)]
    pub is_global: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSshKeyRequest {
    pub name: Option<String>,
    pub private_key: Option<String>,
    pub public_key: Option<String>,
    pub app_id: Option<String>,
    pub is_global: Option<bool>,
}

// Git Provider models (OAuth connections to GitHub, GitLab, etc.)

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
