use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Domain configuration for an application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Domain {
    /// The domain name (e.g., "example.com")
    pub domain: String,
    /// Whether this is the primary domain for the app
    #[serde(default)]
    pub primary: bool,
    /// Whether to redirect www to non-www (or vice versa)
    #[serde(default)]
    pub redirect_www: bool,
}

impl Domain {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            primary: false,
            redirect_www: false,
        }
    }

    pub fn primary(domain: String) -> Self {
        Self {
            domain,
            primary: true,
            redirect_www: false,
        }
    }

    /// Get the www variant of this domain
    pub fn www_domain(&self) -> String {
        if self.domain.starts_with("www.") {
            self.domain.clone()
        } else {
            format!("www.{}", self.domain)
        }
    }

    /// Get the non-www variant of this domain
    pub fn non_www_domain(&self) -> String {
        if self.domain.starts_with("www.") {
            self.domain
                .strip_prefix("www.")
                .unwrap_or(&self.domain)
                .to_string()
        } else {
            self.domain.clone()
        }
    }
}

/// Helper to parse domains JSON from database
pub fn parse_domains(json: Option<&str>) -> Vec<Domain> {
    json.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

/// Helper to serialize domains to JSON for database
pub fn serialize_domains(domains: &[Domain]) -> Option<String> {
    if domains.is_empty() {
        None
    } else {
        serde_json::to_string(domains).ok()
    }
}

/// Port mapping configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortMapping {
    /// Host port to bind (0 for auto-assign)
    pub host_port: u16,
    /// Container port to expose
    pub container_port: u16,
    /// Protocol (tcp or udp)
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

impl PortMapping {
    pub fn new(host_port: u16, container_port: u16) -> Self {
        Self {
            host_port,
            container_port,
            protocol: default_protocol(),
        }
    }

    pub fn with_protocol(mut self, protocol: &str) -> Self {
        self.protocol = protocol.to_string();
        self
    }
}

/// Environment type for applications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Self::Development
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(format!("Unknown environment: {}", s)),
        }
    }
}

impl From<String> for Environment {
    fn from(s: String) -> Self {
        s.parse().unwrap_or_default()
    }
}

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
    pub environment: String,
    pub project_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration (JSON stored as TEXT)
    /// JSON array of PortMapping objects
    pub port_mappings: Option<String>,
    /// JSON array of network alias strings
    pub network_aliases: Option<String>,
    /// JSON array of "hostname:ip" entries for extra hosts
    pub extra_hosts: Option<String>,
    // HTTP Basic Auth
    pub basic_auth_enabled: i32,
    #[serde(skip_serializing)]
    pub basic_auth_username: Option<String>,
    #[serde(skip_serializing)]
    pub basic_auth_password_hash: Option<String>,
    // Deployment commands (JSON stored as TEXT)
    /// JSON array of commands to run before container starts (after build)
    pub pre_deploy_commands: Option<String>,
    /// JSON array of commands to run after container is healthy
    pub post_deploy_commands: Option<String>,
    // Domain management (JSON stored as TEXT)
    /// JSON array of Domain objects for multiple domain support
    pub domains: Option<String>,
    /// Auto-generated subdomain (e.g., app-name.rivetr.example.com)
    pub auto_subdomain: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO for App that excludes sensitive fields (password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResponse {
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
    pub environment: String,
    pub project_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<String>,
    pub network_aliases: Option<String>,
    pub extra_hosts: Option<String>,
    // HTTP Basic Auth (password hash excluded)
    pub basic_auth_enabled: bool,
    pub basic_auth_username: Option<String>,
    // Deployment commands
    pub pre_deploy_commands: Option<String>,
    pub post_deploy_commands: Option<String>,
    // Domain management
    pub domains: Option<String>,
    pub auto_subdomain: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<App> for AppResponse {
    fn from(app: App) -> Self {
        Self {
            id: app.id,
            name: app.name,
            git_url: app.git_url,
            branch: app.branch,
            dockerfile: app.dockerfile,
            domain: app.domain,
            port: app.port,
            healthcheck: app.healthcheck,
            memory_limit: app.memory_limit,
            cpu_limit: app.cpu_limit,
            ssh_key_id: app.ssh_key_id,
            environment: app.environment,
            project_id: app.project_id,
            dockerfile_path: app.dockerfile_path,
            base_directory: app.base_directory,
            build_target: app.build_target,
            watch_paths: app.watch_paths,
            custom_docker_options: app.custom_docker_options,
            port_mappings: app.port_mappings,
            network_aliases: app.network_aliases,
            extra_hosts: app.extra_hosts,
            basic_auth_enabled: app.basic_auth_enabled != 0,
            basic_auth_username: app.basic_auth_username,
            pre_deploy_commands: app.pre_deploy_commands,
            post_deploy_commands: app.post_deploy_commands,
            domains: app.domains,
            auto_subdomain: app.auto_subdomain,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}

impl App {
    /// Parse domains from JSON string
    pub fn get_domains(&self) -> Vec<Domain> {
        parse_domains(self.domains.as_deref())
    }

    /// Get the primary domain (from domains list or legacy domain field)
    pub fn get_primary_domain(&self) -> Option<String> {
        // First check the domains array for a primary domain
        let domains = self.get_domains();
        if let Some(primary) = domains.iter().find(|d| d.primary) {
            return Some(primary.domain.clone());
        }
        // If no primary in domains array but there are domains, use the first one
        if let Some(first) = domains.first() {
            return Some(first.domain.clone());
        }
        // Fall back to legacy domain field
        self.domain.clone()
    }

    /// Get all domain names (including legacy domain and auto_subdomain)
    pub fn get_all_domain_names(&self) -> Vec<String> {
        let mut result = Vec::new();

        // Add domains from the domains array
        for d in self.get_domains() {
            result.push(d.domain.clone());
            // If redirect_www is enabled, add the www variant too
            if d.redirect_www {
                if d.domain.starts_with("www.") {
                    result.push(d.non_www_domain());
                } else {
                    result.push(d.www_domain());
                }
            }
        }

        // Add legacy domain if not already included
        if let Some(ref domain) = self.domain {
            if !result.contains(domain) {
                result.push(domain.clone());
            }
        }

        // Add auto_subdomain if present
        if let Some(ref subdomain) = self.auto_subdomain {
            if !result.contains(subdomain) {
                result.push(subdomain.clone());
            }
        }

        result
    }

    /// Parse port mappings from JSON string
    pub fn get_port_mappings(&self) -> Vec<PortMapping> {
        self.port_mappings
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse network aliases from JSON string
    pub fn get_network_aliases(&self) -> Vec<String> {
        self.network_aliases
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse extra hosts from JSON string
    pub fn get_extra_hosts(&self) -> Vec<String> {
        self.extra_hosts
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse pre_deploy_commands JSON into Vec<String>
    pub fn get_pre_deploy_commands(&self) -> Vec<String> {
        self.pre_deploy_commands
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    /// Parse post_deploy_commands JSON into Vec<String>
    pub fn get_post_deploy_commands(&self) -> Vec<String> {
        self.post_deploy_commands
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Response DTO that masks secret values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarResponse {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl EnvVar {
    pub fn to_response(&self, reveal_secret: bool) -> EnvVarResponse {
        let value = if self.is_secret != 0 && !reveal_secret {
            "********".to_string()
        } else {
            self.value.clone()
        };
        EnvVarResponse {
            id: self.id.clone(),
            app_id: self.app_id.clone(),
            key: self.key.clone(),
            value,
            is_secret: self.is_secret != 0,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvVarRequest {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvVarRequest {
    pub value: Option<String>,
    pub is_secret: Option<bool>,
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
    #[serde(default)]
    pub environment: Environment,
    pub project_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<Vec<PortMapping>>,
    pub network_aliases: Option<Vec<String>>,
    pub extra_hosts: Option<Vec<String>>,
    // Deployment commands (JSON arrays)
    pub pre_deploy_commands: Option<Vec<String>>,
    pub post_deploy_commands: Option<Vec<String>>,
    // Domain management
    pub domains: Option<Vec<Domain>>,
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
    pub environment: Option<Environment>,
    pub project_id: Option<String>,
    // Advanced build options
    pub dockerfile_path: Option<String>,
    pub base_directory: Option<String>,
    pub build_target: Option<String>,
    pub watch_paths: Option<String>,
    pub custom_docker_options: Option<String>,
    // Network configuration
    pub port_mappings: Option<Vec<PortMapping>>,
    pub network_aliases: Option<Vec<String>>,
    pub extra_hosts: Option<Vec<String>>,
    // HTTP Basic Auth
    pub basic_auth_enabled: Option<bool>,
    pub basic_auth_username: Option<String>,
    /// Password in plain text - will be hashed before storing
    pub basic_auth_password: Option<String>,
    // Deployment commands (JSON arrays)
    pub pre_deploy_commands: Option<Vec<String>>,
    pub post_deploy_commands: Option<Vec<String>>,
    // Domain management
    pub domains: Option<Vec<Domain>>,
}

/// Request specifically for updating domains
#[derive(Debug, Deserialize)]
pub struct UpdateDomainsRequest {
    pub domains: Vec<Domain>,
}

/// Request specifically for updating basic auth settings
#[derive(Debug, Deserialize)]
pub struct UpdateBasicAuthRequest {
    pub enabled: bool,
    pub username: Option<String>,
    /// Password in plain text - will be hashed before storing
    pub password: Option<String>,
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

// Project models

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Project with app count for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithAppCount {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub app_count: i64,
}

/// Project with its apps for detail view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWithApps {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub apps: Vec<App>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to assign/unassign an app to a project
#[derive(Debug, Deserialize)]
pub struct AssignAppProjectRequest {
    pub project_id: Option<String>,
}
