use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub webhooks: WebhookConfig,
    #[serde(default)]
    pub oauth: OAuthConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_proxy_https_port")]
    pub proxy_https_port: u16,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            api_port: default_api_port(),
            proxy_port: default_proxy_port(),
            proxy_https_port: default_proxy_https_port(),
            data_dir: default_data_dir(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_api_port() -> u16 {
    8080
}

fn default_proxy_port() -> u16 {
    80
}

fn default_proxy_https_port() -> u16 {
    443
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./data")
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_admin_token")]
    pub admin_token: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            admin_token: default_admin_token(),
        }
    }
}

fn default_admin_token() -> String {
    // Generate a random token if not provided
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_runtime_type")]
    pub runtime_type: RuntimeType,
    #[serde(default = "default_docker_socket")]
    pub docker_socket: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_type: default_runtime_type(),
            docker_socket: default_docker_socket(),
        }
    }
}

fn default_runtime_type() -> RuntimeType {
    RuntimeType::Auto
}

fn default_docker_socket() -> String {
    if cfg!(windows) {
        "npipe:////./pipe/docker_engine".to_string()
    } else {
        "/var/run/docker.sock".to_string()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    Auto,
    Docker,
    Podman,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    /// Enable HTTPS with automatic Let's Encrypt certificates
    #[serde(default)]
    pub acme_enabled: bool,
    /// Email for Let's Encrypt account registration and notifications
    pub acme_email: Option<String>,
    /// Use Let's Encrypt staging environment for testing (avoids rate limits)
    #[serde(default)]
    pub acme_staging: bool,
    /// Directory to store ACME account and certificates (default: ./data/acme)
    #[serde(default = "default_acme_cache_dir")]
    pub acme_cache_dir: PathBuf,
    /// Interval between health checks in seconds (default: 30)
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: u64,
    /// Timeout for health check requests in seconds (default: 5)
    #[serde(default = "default_health_check_timeout")]
    pub health_check_timeout: u64,
    /// Number of consecutive failures before marking backend as unhealthy (default: 3)
    #[serde(default = "default_health_check_threshold")]
    pub health_check_threshold: u32,
    /// Base domain for auto-generated subdomains (e.g., "rivetr.example.com")
    /// Apps will get subdomains like "my-app.rivetr.example.com"
    pub base_domain: Option<String>,
    /// Enable automatic subdomain generation for new apps (default: true if base_domain is set)
    #[serde(default)]
    pub auto_subdomain_enabled: bool,
}

fn default_acme_cache_dir() -> PathBuf {
    PathBuf::from("./data/acme")
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_health_check_timeout() -> u64 {
    5
}

fn default_health_check_threshold() -> u32 {
    3
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            acme_enabled: false,
            acme_email: None,
            acme_staging: false,
            acme_cache_dir: default_acme_cache_dir(),
            health_check_interval: default_health_check_interval(),
            health_check_timeout: default_health_check_timeout(),
            health_check_threshold: default_health_check_threshold(),
            base_domain: None,
            auto_subdomain_enabled: false,
        }
    }
}

impl ProxyConfig {
    /// Generate a subdomain for an app name
    pub fn generate_subdomain(&self, app_name: &str) -> Option<String> {
        if self.auto_subdomain_enabled {
            self.base_domain
                .as_ref()
                .map(|base| format!("{}.{}", app_name, base))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfig {
    /// Secret for verifying GitHub webhook signatures (HMAC-SHA256)
    pub github_secret: Option<String>,
    /// Secret token for GitLab webhook verification (X-Gitlab-Token header)
    pub gitlab_token: Option<String>,
    /// Secret for verifying Gitea webhook signatures (HMAC-SHA256)
    pub gitea_secret: Option<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            github_secret: None,
            gitlab_token: None,
            gitea_secret: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthConfig {
    #[serde(default)]
    pub github: Option<OAuthProviderConfig>,
    #[serde(default)]
    pub gitlab: Option<OAuthProviderConfig>,
    #[serde(default)]
    pub bitbucket: Option<OAuthProviderConfig>,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            github: None,
            gitlab: None,
            bitbucket: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthProviderConfig {
    /// OAuth client ID
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// OAuth redirect URI (callback URL)
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting (default: true)
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,
    /// Requests per window for general API endpoints (default: 100)
    #[serde(default = "default_api_requests_per_window")]
    pub api_requests_per_window: u32,
    /// Requests per window for webhook endpoints (default: 500)
    #[serde(default = "default_webhook_requests_per_window")]
    pub webhook_requests_per_window: u32,
    /// Requests per window for auth endpoints (default: 20)
    #[serde(default = "default_auth_requests_per_window")]
    pub auth_requests_per_window: u32,
    /// Window duration in seconds (default: 60)
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
    /// Cleanup interval for expired entries in seconds (default: 300)
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,
}

fn default_rate_limit_enabled() -> bool {
    true
}

fn default_api_requests_per_window() -> u32 {
    100
}

fn default_webhook_requests_per_window() -> u32 {
    500
}

fn default_auth_requests_per_window() -> u32 {
    20
}

fn default_window_seconds() -> u64 {
    60
}

fn default_cleanup_interval() -> u64 {
    300
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_rate_limit_enabled(),
            api_requests_per_window: default_api_requests_per_window(),
            webhook_requests_per_window: default_webhook_requests_per_window(),
            auth_requests_per_window: default_auth_requests_per_window(),
            window_seconds: default_window_seconds(),
            cleanup_interval: default_cleanup_interval(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CleanupConfig {
    /// Enable automatic cleanup of old deployments (default: true)
    #[serde(default = "default_cleanup_enabled")]
    pub enabled: bool,
    /// Maximum number of deployments to keep per app (default: 10)
    #[serde(default = "default_max_deployments_per_app")]
    pub max_deployments_per_app: u32,
    /// Interval between cleanup runs in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_cleanup_interval_seconds")]
    pub cleanup_interval_seconds: u64,
    /// Prune unused Docker/Podman images after cleanup (default: true)
    #[serde(default = "default_prune_images")]
    pub prune_images: bool,
}

fn default_cleanup_enabled() -> bool {
    true
}

fn default_max_deployments_per_app() -> u32 {
    10
}

fn default_cleanup_interval_seconds() -> u64 {
    3600
}

fn default_prune_images() -> bool {
    true
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            enabled: default_cleanup_enabled(),
            max_deployments_per_app: default_max_deployments_per_app(),
            cleanup_interval_seconds: default_cleanup_interval_seconds(),
            prune_images: default_prune_images(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            info!("Loading configuration from {}", path.display());
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| "Failed to parse configuration file")?;
            Ok(config)
        } else {
            info!("No config file found, using defaults");
            Ok(Config::default())
        }
    }

    pub fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            runtime: RuntimeConfig::default(),
            proxy: ProxyConfig::default(),
            logging: LoggingConfig::default(),
            webhooks: WebhookConfig::default(),
            oauth: OAuthConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cleanup: CleanupConfig::default(),
        }
    }
}
