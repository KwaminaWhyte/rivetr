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
    #[serde(default)]
    pub disk_monitor: DiskMonitorConfig,
    #[serde(default)]
    pub container_monitor: ContainerMonitorConfig,
    #[serde(default)]
    pub database_backup: DatabaseBackupConfig,
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
    /// External URL for callbacks (e.g., ngrok URL for development)
    /// If set, this is used for GitHub App callbacks and webhooks
    #[serde(default)]
    pub external_url: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            api_port: default_api_port(),
            proxy_port: default_proxy_port(),
            proxy_https_port: default_proxy_https_port(),
            data_dir: default_data_dir(),
            external_url: None,
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
    /// Secret key for encrypting environment variables in the database.
    /// If not set, environment variables are stored in plaintext (backwards compatible).
    /// Recommended: Use a strong, random 32+ character string.
    pub encryption_key: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            admin_token: default_admin_token(),
            encryption_key: None,
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
    /// CPU limit for builds (e.g., "2" for 2 CPUs). Default: 2
    #[serde(default = "default_build_cpu_limit")]
    pub build_cpu_limit: String,
    /// Memory limit for builds (e.g., "2g" for 2GB). Default: 2g
    #[serde(default = "default_build_memory_limit")]
    pub build_memory_limit: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_type: default_runtime_type(),
            docker_socket: default_docker_socket(),
            build_cpu_limit: default_build_cpu_limit(),
            build_memory_limit: default_build_memory_limit(),
        }
    }
}

fn default_build_cpu_limit() -> String {
    "2".to_string()
}

fn default_build_memory_limit() -> String {
    "2g".to_string()
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
    /// Server's public IP address for sslip.io domain generation
    /// If not set, will try to auto-detect
    pub server_ip: Option<String>,
    /// Enable sslip.io automatic domains (generates domains like abc123.192.168.1.1.sslip.io)
    #[serde(default)]
    pub sslip_enabled: bool,
    /// Base domain for PR preview deployments (e.g., "preview.example.com")
    /// Preview environments get subdomains like "pr-123.my-app.preview.example.com"
    pub preview_domain: Option<String>,
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
            server_ip: None,
            sslip_enabled: false,
            preview_domain: None,
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

    /// Generate a sslip.io domain for an app
    /// Format: <random-id>.<ip>.sslip.io
    /// Example: abc123def.192.168.1.1.sslip.io
    pub fn generate_sslip_domain(&self, server_ip: Option<&str>) -> Option<String> {
        use rand::Rng;
        let ip = server_ip.or(self.server_ip.as_deref())?;
        // Generate a random 12-character ID
        let mut rng = rand::rng();
        let id: String = (0..12)
            .map(|_| {
                let chars = b"abcdefghijklmnopqrstuvwxyz0123456789";
                chars[rng.random_range(0..chars.len())] as char
            })
            .collect();
        Some(format!("{}.{}.sslip.io", id, ip))
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

#[derive(Debug, Clone, Deserialize)]
pub struct DiskMonitorConfig {
    /// Enable disk space monitoring (default: true)
    #[serde(default = "default_disk_monitor_enabled")]
    pub enabled: bool,
    /// Interval between disk space checks in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_disk_check_interval")]
    pub check_interval_seconds: u64,
    /// Disk usage percentage threshold for warning logs (default: 80)
    #[serde(default = "default_disk_warning_threshold")]
    pub warning_threshold: u8,
    /// Disk usage percentage threshold for critical logs (default: 90)
    #[serde(default = "default_disk_critical_threshold")]
    pub critical_threshold: u8,
}

fn default_disk_monitor_enabled() -> bool {
    true
}

fn default_disk_check_interval() -> u64 {
    300 // 5 minutes
}

fn default_disk_warning_threshold() -> u8 {
    80
}

fn default_disk_critical_threshold() -> u8 {
    90
}

impl Default for DiskMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: default_disk_monitor_enabled(),
            check_interval_seconds: default_disk_check_interval(),
            warning_threshold: default_disk_warning_threshold(),
            critical_threshold: default_disk_critical_threshold(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContainerMonitorConfig {
    /// Enable container crash monitoring and auto-restart (default: true)
    #[serde(default = "default_container_monitor_enabled")]
    pub enabled: bool,
    /// Interval between container health checks in seconds (default: 30)
    #[serde(default = "default_container_check_interval")]
    pub check_interval_secs: u64,
    /// Maximum number of restart attempts before giving up (default: 5)
    #[serde(default = "default_max_restart_attempts")]
    pub max_restart_attempts: u32,
    /// Initial backoff delay in seconds after a crash (default: 5)
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_secs: u64,
    /// Maximum backoff delay in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_max_backoff")]
    pub max_backoff_secs: u64,
    /// Duration in seconds a container must run before being considered stable (default: 120)
    /// After this duration, the restart counter is reset
    #[serde(default = "default_stable_duration")]
    pub stable_duration_secs: u64,
}

fn default_container_monitor_enabled() -> bool {
    true
}

fn default_container_check_interval() -> u64 {
    30
}

fn default_max_restart_attempts() -> u32 {
    5
}

fn default_initial_backoff() -> u64 {
    5
}

fn default_max_backoff() -> u64 {
    300 // 5 minutes
}

fn default_stable_duration() -> u64 {
    120 // 2 minutes
}

impl Default for ContainerMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: default_container_monitor_enabled(),
            check_interval_secs: default_container_check_interval(),
            max_restart_attempts: default_max_restart_attempts(),
            initial_backoff_secs: default_initial_backoff(),
            max_backoff_secs: default_max_backoff(),
            stable_duration_secs: default_stable_duration(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseBackupConfig {
    /// Enable automatic database backup scheduling (default: true)
    #[serde(default = "default_db_backup_enabled")]
    pub enabled: bool,
    /// Interval between schedule checks in seconds (default: 60)
    #[serde(default = "default_db_backup_check_interval")]
    pub check_interval_seconds: u64,
    /// Directory to store backups (relative to data_dir, default: "backups")
    #[serde(default = "default_db_backup_dir")]
    pub backup_dir: String,
    /// Timeout for backup commands in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_db_backup_timeout")]
    pub timeout_seconds: u64,
}

fn default_db_backup_enabled() -> bool {
    true
}

fn default_db_backup_check_interval() -> u64 {
    60 // 1 minute
}

fn default_db_backup_dir() -> String {
    "backups".to_string()
}

fn default_db_backup_timeout() -> u64 {
    3600 // 1 hour
}

impl Default for DatabaseBackupConfig {
    fn default() -> Self {
        Self {
            enabled: default_db_backup_enabled(),
            check_interval_seconds: default_db_backup_check_interval(),
            backup_dir: default_db_backup_dir(),
            timeout_seconds: default_db_backup_timeout(),
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
            disk_monitor: DiskMonitorConfig::default(),
            container_monitor: ContainerMonitorConfig::default(),
            database_backup: DatabaseBackupConfig::default(),
        }
    }
}
