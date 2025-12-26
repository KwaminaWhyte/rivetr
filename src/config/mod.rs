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
    #[serde(default = "default_admin_email")]
    pub admin_email: String,
    #[serde(default = "default_admin_password")]
    pub admin_password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            admin_token: default_admin_token(),
            admin_email: default_admin_email(),
            admin_password: default_admin_password(),
        }
    }
}

fn default_admin_token() -> String {
    // Generate a random token if not provided
    uuid::Uuid::new_v4().to_string()
}

fn default_admin_email() -> String {
    "admin@localhost".to_string()
}

fn default_admin_password() -> String {
    "admin".to_string()
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
    pub acme_email: Option<String>,
    #[serde(default)]
    pub acme_staging: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            acme_email: None,
            acme_staging: false,
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
        }
    }
}
