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
    // Docker Registry support
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app") - when set, skip build and pull from registry
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password (encrypted)
    #[serde(skip_serializing)]
    pub registry_password: Option<String>,
    /// Container labels (JSON object stored as TEXT)
    pub container_labels: Option<String>,
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
    // Docker Registry support (password excluded)
    pub docker_image: Option<String>,
    pub docker_image_tag: Option<String>,
    pub registry_url: Option<String>,
    pub registry_username: Option<String>,
    // Container labels
    pub container_labels: Option<String>,
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
            docker_image: app.docker_image,
            docker_image_tag: app.docker_image_tag,
            registry_url: app.registry_url,
            registry_username: app.registry_username,
            container_labels: app.container_labels,
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

    /// Check if this app uses a Docker registry image instead of building from git
    pub fn uses_registry_image(&self) -> bool {
        self.docker_image
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    }

    /// Get the full image reference including registry URL and tag
    /// Format: [registry/]image[:tag]
    pub fn get_full_image_reference(&self) -> Option<String> {
        let image = self.docker_image.as_ref()?;
        if image.is_empty() {
            return None;
        }

        let tag = self
            .docker_image_tag
            .as_ref()
            .filter(|t| !t.is_empty())
            .map(|t| t.as_str())
            .unwrap_or("latest");

        // If there's a custom registry URL, prepend it
        let full_image = if let Some(ref registry) = self.registry_url {
            if !registry.is_empty() {
                // Remove trailing slash from registry and leading slash from image
                let registry = registry.trim_end_matches('/');
                let image = image.trim_start_matches('/');
                format!("{}/{}", registry, image)
            } else {
                image.clone()
            }
        } else {
            image.clone()
        };

        Some(format!("{}:{}", full_image, tag))
    }

    /// Parse container_labels JSON into HashMap<String, String>
    pub fn get_container_labels(&self) -> std::collections::HashMap<String, String> {
        self.container_labels
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
    /// Git URL for source-based deployments (required if docker_image is not set)
    #[serde(default)]
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
    // Docker Registry support (alternative to git-based deployments)
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app")
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    #[serde(default = "default_image_tag")]
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password
    pub registry_password: Option<String>,
    /// Container labels (key-value pairs)
    pub container_labels: Option<std::collections::HashMap<String, String>>,
}

fn default_image_tag() -> Option<String> {
    Some("latest".to_string())
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
    // Docker Registry support
    /// Docker image name (e.g., "nginx", "ghcr.io/user/app") - set to empty string to clear
    pub docker_image: Option<String>,
    /// Docker image tag (default: "latest")
    pub docker_image_tag: Option<String>,
    /// Custom registry URL (null = Docker Hub)
    pub registry_url: Option<String>,
    /// Registry authentication username
    pub registry_username: Option<String>,
    /// Registry authentication password
    pub registry_password: Option<String>,
    /// Container labels (key-value pairs)
    pub container_labels: Option<std::collections::HashMap<String, String>>,
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
    #[serde(default)]
    pub databases: Vec<ManagedDatabaseResponse>,
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

// Team models for multi-user support with role-based access control

/// Team roles with hierarchical permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    /// Full access, can delete team, manage all members
    Owner,
    /// Manage apps, projects, members (except owners), deploy
    Admin,
    /// Create/edit apps, deploy, view logs
    Developer,
    /// Read-only access to apps, deployments, logs
    Viewer,
}

impl TeamRole {
    /// Check if this role has at least the specified permission level
    pub fn has_at_least(&self, required: TeamRole) -> bool {
        self.level() >= required.level()
    }

    /// Get the permission level (higher = more permissions)
    pub fn level(&self) -> u8 {
        match self {
            TeamRole::Owner => 4,
            TeamRole::Admin => 3,
            TeamRole::Developer => 2,
            TeamRole::Viewer => 1,
        }
    }

    /// Check if the role can manage team members
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can manage members of the given role
    pub fn can_manage_member_role(&self, target_role: TeamRole) -> bool {
        match self {
            TeamRole::Owner => true,
            TeamRole::Admin => !matches!(target_role, TeamRole::Owner),
            _ => false,
        }
    }

    /// Check if the role can deploy apps
    pub fn can_deploy(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
    }

    /// Check if the role can create/edit apps
    pub fn can_manage_apps(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
    }

    /// Check if the role can delete apps
    pub fn can_delete_apps(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can manage projects
    pub fn can_manage_projects(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin | TeamRole::Developer)
    }

    /// Check if the role can delete projects
    pub fn can_delete_projects(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Admin)
    }

    /// Check if the role can delete the team
    pub fn can_delete_team(&self) -> bool {
        matches!(self, TeamRole::Owner)
    }

    /// Check if the role can view resources (all roles can view)
    pub fn can_view(&self) -> bool {
        true
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamRole::Owner => write!(f, "owner"),
            TeamRole::Admin => write!(f, "admin"),
            TeamRole::Developer => write!(f, "developer"),
            TeamRole::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for TeamRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(TeamRole::Owner),
            "admin" => Ok(TeamRole::Admin),
            "developer" => Ok(TeamRole::Developer),
            "viewer" => Ok(TeamRole::Viewer),
            _ => Err(format!("Unknown team role: {}", s)),
        }
    }
}

impl From<String> for TeamRole {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(TeamRole::Viewer)
    }
}

/// Team entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Team member entity linking users to teams with roles
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMember {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: String,
}

impl TeamMember {
    /// Get the role as a TeamRole enum
    pub fn role_enum(&self) -> TeamRole {
        TeamRole::from(self.role.clone())
    }
}

/// Team with member count for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWithMemberCount {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub member_count: i64,
    /// Current user's role in this team (if applicable)
    pub user_role: Option<String>,
}

/// Team member with user details
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMemberWithUser {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: String,
    /// User's name
    pub user_name: String,
    /// User's email
    pub user_email: String,
}

/// Team detail response with members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDetail {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
    pub members: Vec<TeamMemberWithUser>,
}

/// Request to create a new team
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    /// Optional slug (auto-generated from name if not provided)
    pub slug: Option<String>,
}

/// Request to update a team
#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

/// Request to invite/add a member to a team
#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    /// User ID or email to invite
    pub user_identifier: String,
    /// Role to assign
    pub role: String,
}

/// Request to update a member's role
#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

// -------------------------------------------------------------------------
// Notification models
// -------------------------------------------------------------------------

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannelType {
    Slack,
    Discord,
    Email,
}

impl std::fmt::Display for NotificationChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Discord => write!(f, "discord"),
            Self::Email => write!(f, "email"),
        }
    }
}

impl std::str::FromStr for NotificationChannelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slack" => Ok(Self::Slack),
            "discord" => Ok(Self::Discord),
            "email" => Ok(Self::Email),
            _ => Err(format!("Unknown channel type: {}", s)),
        }
    }
}

impl From<String> for NotificationChannelType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Slack)
    }
}

/// Notification event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    DeploymentStarted,
    DeploymentSuccess,
    DeploymentFailed,
    AppStopped,
    AppStarted,
}

impl std::fmt::Display for NotificationEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeploymentStarted => write!(f, "deployment_started"),
            Self::DeploymentSuccess => write!(f, "deployment_success"),
            Self::DeploymentFailed => write!(f, "deployment_failed"),
            Self::AppStopped => write!(f, "app_stopped"),
            Self::AppStarted => write!(f, "app_started"),
        }
    }
}

impl std::str::FromStr for NotificationEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "deployment_started" => Ok(Self::DeploymentStarted),
            "deployment_success" => Ok(Self::DeploymentSuccess),
            "deployment_failed" => Ok(Self::DeploymentFailed),
            "app_stopped" => Ok(Self::AppStopped),
            "app_started" => Ok(Self::AppStarted),
            _ => Err(format!("Unknown event type: {}", s)),
        }
    }
}

impl From<String> for NotificationEventType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::DeploymentStarted)
    }
}

/// Slack webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
}

/// Discord webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

/// Email (SMTP) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_tls: bool,
    pub from_address: String,
    pub to_addresses: Vec<String>,
}

/// Notification channel stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub config: String, // JSON serialized config
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl NotificationChannel {
    /// Get the channel type enum
    pub fn get_channel_type(&self) -> NotificationChannelType {
        self.channel_type.parse().unwrap_or(NotificationChannelType::Slack)
    }

    /// Check if the channel is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }

    /// Parse the config as SlackConfig
    pub fn get_slack_config(&self) -> Option<SlackConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as DiscordConfig
    pub fn get_discord_config(&self) -> Option<DiscordConfig> {
        serde_json::from_str(&self.config).ok()
    }

    /// Parse the config as EmailConfig
    pub fn get_email_config(&self) -> Option<EmailConfig> {
        serde_json::from_str(&self.config).ok()
    }
}

/// Response DTO for NotificationChannel (masks sensitive config data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannelResponse {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub config: serde_json::Value, // Sanitized config
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<NotificationChannel> for NotificationChannelResponse {
    fn from(channel: NotificationChannel) -> Self {
        // Parse and sanitize the config (mask passwords)
        let config: serde_json::Value = serde_json::from_str(&channel.config)
            .unwrap_or(serde_json::Value::Null);

        // For email config, mask the password
        let sanitized_config = if channel.channel_type == "email" {
            if let serde_json::Value::Object(mut obj) = config {
                if obj.contains_key("smtp_password") {
                    obj.insert("smtp_password".to_string(), serde_json::json!("********"));
                }
                serde_json::Value::Object(obj)
            } else {
                config
            }
        } else {
            config
        };

        Self {
            id: channel.id,
            name: channel.name,
            channel_type: channel.channel_type,
            config: sanitized_config,
            enabled: channel.enabled != 0,
            created_at: channel.created_at,
            updated_at: channel.updated_at,
        }
    }
}

/// Notification subscription stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationSubscription {
    pub id: String,
    pub channel_id: String,
    pub event_type: String,
    pub app_id: Option<String>,
    pub created_at: String,
}

impl NotificationSubscription {
    /// Get the event type enum
    pub fn get_event_type(&self) -> NotificationEventType {
        self.event_type.parse().unwrap_or(NotificationEventType::DeploymentStarted)
    }
}

/// Response DTO for NotificationSubscription with app name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubscriptionResponse {
    pub id: String,
    pub channel_id: String,
    pub event_type: String,
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub created_at: String,
}

/// Request to create a notification channel
#[derive(Debug, Deserialize)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub config: serde_json::Value,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Request to update a notification channel
#[derive(Debug, Deserialize)]
pub struct UpdateNotificationChannelRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

/// Request to create a notification subscription
#[derive(Debug, Deserialize)]
pub struct CreateNotificationSubscriptionRequest {
    pub event_type: NotificationEventType,
    pub app_id: Option<String>,
}

/// Test notification request
#[derive(Debug, Deserialize)]
pub struct TestNotificationRequest {
    pub message: Option<String>,
}

// -------------------------------------------------------------------------
// Volume models
// -------------------------------------------------------------------------

/// Volume mount for persistent storage
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Volume {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl Volume {
    /// Check if this volume is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only != 0
    }

    /// Get the Docker bind mount string (host_path:container_path[:ro])
    pub fn to_bind_mount(&self) -> String {
        if self.is_read_only() {
            format!("{}:{}:ro", self.host_path, self.container_path)
        } else {
            format!("{}:{}", self.host_path, self.container_path)
        }
    }
}

/// Response DTO for Volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Volume> for VolumeResponse {
    fn from(v: Volume) -> Self {
        Self {
            id: v.id,
            app_id: v.app_id,
            name: v.name,
            host_path: v.host_path,
            container_path: v.container_path,
            read_only: v.read_only != 0,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Request to create a volume
#[derive(Debug, Deserialize)]
pub struct CreateVolumeRequest {
    pub name: String,
    pub host_path: String,
    pub container_path: String,
    #[serde(default)]
    pub read_only: bool,
}

/// Request to update a volume
#[derive(Debug, Deserialize)]
pub struct UpdateVolumeRequest {
    pub name: Option<String>,
    pub host_path: Option<String>,
    pub container_path: Option<String>,
    pub read_only: Option<bool>,
}

// -------------------------------------------------------------------------
// Managed Database models
// -------------------------------------------------------------------------

/// Supported database types for managed databases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Mongodb,
    Redis,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Mysql => write!(f, "mysql"),
            Self::Mongodb => write!(f, "mongodb"),
            Self::Redis => write!(f, "redis"),
        }
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(Self::Postgres),
            "mysql" | "mariadb" => Ok(Self::Mysql),
            "mongodb" | "mongo" => Ok(Self::Mongodb),
            "redis" => Ok(Self::Redis),
            _ => Err(format!("Unknown database type: {}", s)),
        }
    }
}

impl From<String> for DatabaseType {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Postgres)
    }
}

/// Database deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseStatus {
    Pending,
    Pulling,
    Starting,
    Running,
    Stopped,
    Failed,
}

impl std::fmt::Display for DatabaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Pulling => write!(f, "pulling"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for DatabaseStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "pulling" => Ok(Self::Pulling),
            "starting" => Ok(Self::Starting),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

impl From<String> for DatabaseStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(Self::Pending)
    }
}

/// Database credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCredentials {
    pub username: String,
    pub password: String,
    /// Database name (not applicable for Redis)
    pub database: Option<String>,
    /// Root password for MySQL
    pub root_password: Option<String>,
}

/// Managed database entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ManagedDatabase {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub container_id: Option<String>,
    pub status: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub public_access: i32,
    /// JSON-encoded DatabaseCredentials
    pub credentials: String,
    pub volume_name: Option<String>,
    pub volume_path: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub error_message: Option<String>,
    /// Associated project ID
    pub project_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ManagedDatabase {
    /// Get the database type as enum
    pub fn get_db_type(&self) -> DatabaseType {
        DatabaseType::from(self.db_type.clone())
    }

    /// Get the status as enum
    pub fn get_status(&self) -> DatabaseStatus {
        DatabaseStatus::from(self.status.clone())
    }

    /// Parse credentials from JSON
    pub fn get_credentials(&self) -> Option<DatabaseCredentials> {
        serde_json::from_str(&self.credentials).ok()
    }

    /// Check if public access is enabled
    pub fn is_public(&self) -> bool {
        self.public_access != 0
    }

    /// Get container name
    pub fn container_name(&self) -> String {
        format!("rivetr-db-{}", self.name)
    }

    /// Generate internal connection string (for apps on same Docker network)
    pub fn internal_connection_string(&self) -> Option<String> {
        let creds = self.get_credentials()?;
        let container_name = self.container_name();

        match self.get_db_type() {
            DatabaseType::Postgres => Some(format!(
                "postgresql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mysql => Some(format!(
                "mysql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mongodb => Some(format!(
                "mongodb://{}:{}@{}:{}/{}?authSource=admin",
                creds.username,
                creds.password,
                container_name,
                self.internal_port,
                creds.database.unwrap_or_else(|| "admin".to_string())
            )),
            DatabaseType::Redis => {
                if creds.password.is_empty() {
                    Some(format!("redis://{}:{}", container_name, self.internal_port))
                } else {
                    Some(format!(
                        "redis://:{}@{}:{}",
                        creds.password, container_name, self.internal_port
                    ))
                }
            }
        }
    }

    /// Generate external connection string (for public access)
    pub fn external_connection_string(&self, host: &str) -> Option<String> {
        if !self.is_public() || self.external_port == 0 {
            return None;
        }

        let creds = self.get_credentials()?;

        match self.get_db_type() {
            DatabaseType::Postgres => Some(format!(
                "postgresql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mysql => Some(format!(
                "mysql://{}:{}@{}:{}/{}",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| creds.username.clone())
            )),
            DatabaseType::Mongodb => Some(format!(
                "mongodb://{}:{}@{}:{}/{}?authSource=admin",
                creds.username,
                creds.password,
                host,
                self.external_port,
                creds.database.unwrap_or_else(|| "admin".to_string())
            )),
            DatabaseType::Redis => {
                if creds.password.is_empty() {
                    Some(format!("redis://{}:{}", host, self.external_port))
                } else {
                    Some(format!(
                        "redis://:{}@{}:{}",
                        creds.password, host, self.external_port
                    ))
                }
            }
        }
    }
}

/// Response DTO for ManagedDatabase (masks password in credentials)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedDatabaseResponse {
    pub id: String,
    pub name: String,
    pub db_type: String,
    pub version: String,
    pub container_id: Option<String>,
    pub status: String,
    pub internal_port: i32,
    pub external_port: i32,
    pub public_access: bool,
    /// Credentials with password masked unless reveal=true
    pub credentials: DatabaseCredentials,
    pub volume_name: Option<String>,
    pub volume_path: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_limit: Option<String>,
    pub internal_connection_string: Option<String>,
    pub external_connection_string: Option<String>,
    pub error_message: Option<String>,
    pub project_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ManagedDatabase {
    pub fn to_response(
        &self,
        reveal_password: bool,
        external_host: Option<&str>,
    ) -> ManagedDatabaseResponse {
        let mut creds = self.get_credentials().unwrap_or(DatabaseCredentials {
            username: String::new(),
            password: String::new(),
            database: None,
            root_password: None,
        });

        if !reveal_password {
            creds.password = "********".to_string();
            if creds.root_password.is_some() {
                creds.root_password = Some("********".to_string());
            }
        }

        ManagedDatabaseResponse {
            id: self.id.clone(),
            name: self.name.clone(),
            db_type: self.db_type.clone(),
            version: self.version.clone(),
            container_id: self.container_id.clone(),
            status: self.status.clone(),
            internal_port: self.internal_port,
            external_port: self.external_port,
            public_access: self.is_public(),
            credentials: creds,
            volume_name: self.volume_name.clone(),
            volume_path: self.volume_path.clone(),
            memory_limit: self.memory_limit.clone(),
            cpu_limit: self.cpu_limit.clone(),
            internal_connection_string: self.internal_connection_string(),
            external_connection_string: external_host.and_then(|h| self.external_connection_string(h)),
            error_message: self.error_message.clone(),
            project_id: self.project_id.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

/// Request to create a managed database
#[derive(Debug, Deserialize)]
pub struct CreateManagedDatabaseRequest {
    pub name: String,
    pub db_type: DatabaseType,
    /// Version/tag (e.g., "16", "8.0", "7", "7.2")
    #[serde(default = "default_db_version")]
    pub version: String,
    /// Enable public port exposure
    #[serde(default)]
    pub public_access: bool,
    /// Custom username (optional, auto-generated if not provided)
    pub username: Option<String>,
    /// Custom password (optional, auto-generated if not provided)
    pub password: Option<String>,
    /// Custom database name (optional)
    pub database: Option<String>,
    /// Custom root password for MySQL (optional, auto-generated if not provided)
    pub root_password: Option<String>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// CPU limit
    pub cpu_limit: Option<String>,
    /// Associated project ID
    pub project_id: Option<String>,
}

fn default_db_version() -> String {
    "latest".to_string()
}

/// Request to update a managed database
#[derive(Debug, Deserialize)]
pub struct UpdateManagedDatabaseRequest {
    /// Enable/disable public access
    pub public_access: Option<bool>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// CPU limit
    pub cpu_limit: Option<String>,
}
