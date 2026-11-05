//! Server models and DTOs for multi-server support.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub ssh_private_key: Option<String>, // encrypted with AES-256-GCM
    pub ssh_password: Option<String>,    // encrypted with AES-256-GCM
    pub status: String,
    pub last_seen_at: Option<String>,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
    pub disk_usage: Option<f64>,
    pub memory_total: Option<i64>,
    pub disk_total: Option<i64>,
    pub os_info: Option<String>,
    pub docker_version: Option<String>,
    pub team_id: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Hourly cost in USD (used for cost analysis). Defaults to $0.036/hr (DO s-2vcpu-4gb).
    #[serde(default = "default_hourly_rate")]
    pub hourly_rate: f64,
    pub created_at: String,
    pub updated_at: String,
    /// How Rivetr reaches the server: `public` (host) or `tailscale` (tailscale_host).
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
    /// Tailscale IP (100.x.y.z) or MagicDNS name, used when network_mode is `tailscale`.
    #[serde(default)]
    pub tailscale_host: Option<String>,
}

impl Server {
    /// Address used for SSH: the Tailscale address when the server is set to
    /// reach over Tailscale, otherwise the public host.
    pub fn ssh_host(&self) -> &str {
        match (self.network_mode.as_str(), self.tailscale_host.as_deref()) {
            ("tailscale", Some(ts)) if !ts.trim().is_empty() => ts.trim(),
            _ => &self.host,
        }
    }
}

/// Validate a network mode and Tailscale address pair from a request.
pub fn validate_server_network(mode: &str, tailscale_host: Option<&str>) -> Result<(), String> {
    match mode {
        "public" => Ok(()),
        "tailscale" => {
            let ts = tailscale_host.map(str::trim).unwrap_or("");
            if ts.is_empty() {
                return Err("A Tailscale IP or MagicDNS name is required".to_string());
            }
            if ts.len() > 253
                || ts.starts_with('-')
                || !ts
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
            {
                return Err("Tailscale address must be an IP or a hostname".to_string());
            }
            Ok(())
        }
        other => Err(format!(
            "Unknown network mode '{other}' (use public or tailscale)"
        )),
    }
}

fn default_network_mode() -> String {
    "public".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_hourly_rate() -> f64 {
    0.036
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub ssh_password: Option<String>,
    pub team_id: Option<String>,
    /// Hourly cost in USD for cost analysis (e.g. 0.036 for DO s-2vcpu-4gb)
    pub hourly_rate: Option<f64>,
    /// `public` (default) or `tailscale`
    pub network_mode: Option<String>,
    pub tailscale_host: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub ssh_private_key: Option<String>,
    pub ssh_password: Option<String>,
    pub timezone: Option<String>,
    /// Hourly cost in USD for cost analysis
    pub hourly_rate: Option<f64>,
    /// `public` or `tailscale`
    pub network_mode: Option<String>,
    /// Tailscale IP or MagicDNS name (empty string clears it)
    pub tailscale_host: Option<String>,
}

#[cfg(test)]
mod network_tests {
    use super::*;

    #[test]
    fn network_validation() {
        assert!(validate_server_network("public", None).is_ok());
        assert!(validate_server_network("tailscale", Some("100.101.102.103")).is_ok());
        assert!(validate_server_network("tailscale", Some("web-1.tail1234.ts.net")).is_ok());
        assert!(validate_server_network("tailscale", Some("fd7a:115c:a1e0::1")).is_ok());
        assert!(validate_server_network("tailscale", None).is_err());
        assert!(validate_server_network("tailscale", Some("  ")).is_err());
        assert!(validate_server_network("tailscale", Some("-oProxyCommand=x")).is_err());
        assert!(validate_server_network("tailscale", Some("a b")).is_err());
        assert!(validate_server_network("vpn", None).is_err());
    }
}
