//! Common types and utilities shared across models.

use serde::{Deserialize, Serialize};

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
