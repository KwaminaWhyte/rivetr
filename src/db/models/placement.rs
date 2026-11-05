//! Multi-server placement: which servers an app runs on, how a deployment
//! rolls across them, and the per-target records of a deployment.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Server id of the Rivetr host itself in a placement list.
pub const LOCAL_SERVER_ID: &str = "localhost";

/// Most containers per server in a placement.
pub const MAX_REPLICAS_PER_SERVER: i64 = 10;

/// Most servers in one placement list.
pub const MAX_PLACEMENT_SERVERS: usize = 20;

/// One server an app is placed on.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppServerPlacement {
    pub id: String,
    pub app_id: String,
    /// A `servers.id`, or `localhost` for the Rivetr host.
    pub server_id: String,
    /// Containers on this server.
    pub replicas: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// How images reach the target servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImageDistribution {
    /// Registry when the app pushes to one, otherwise SSH streaming.
    #[default]
    Auto,
    /// Push once to the app's registry, pull on every target.
    Registry,
    /// `docker save | ssh docker load` to every remote target.
    Ssh,
}

impl ImageDistribution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Registry => "registry",
            Self::Ssh => "ssh",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "registry" => Some(Self::Registry),
            "ssh" => Some(Self::Ssh),
            _ => None,
        }
    }
}

/// Rollout and load balancing options of a multi-server app (row of
/// `app_multi_server_settings`, defaults when the app has none).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiServerSettings {
    /// Containers replaced per rolling batch (at least 1).
    pub max_unavailable: i64,
    /// Roll the whole deployment back when at least this percentage of
    /// targets fail (0 rolls back on any failure, 100 only when all fail).
    pub failure_threshold_pct: i64,
    /// `auto`, `registry` or `ssh`.
    pub distribution: String,
    /// `round_robin` or `least_conn`.
    pub lb_policy: String,
    pub sticky_sessions: bool,
    /// Publish remote ports on the server's Tailscale address when it has one.
    pub bind_private: bool,
}

impl Default for MultiServerSettings {
    fn default() -> Self {
        Self {
            max_unavailable: 1,
            failure_threshold_pct: 50,
            distribution: "auto".to_string(),
            lb_policy: "round_robin".to_string(),
            sticky_sessions: false,
            bind_private: true,
        }
    }
}

impl MultiServerSettings {
    /// Check ranges and enum values from a request.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=50).contains(&self.max_unavailable) {
            return Err("max_unavailable must be between 1 and 50".to_string());
        }
        if !(0..=100).contains(&self.failure_threshold_pct) {
            return Err("failure_threshold_pct must be between 0 and 100".to_string());
        }
        if ImageDistribution::parse(&self.distribution).is_none() {
            return Err("distribution must be auto, registry or ssh".to_string());
        }
        if !matches!(self.lb_policy.as_str(), "round_robin" | "least_conn") {
            return Err("lb_policy must be round_robin or least_conn".to_string());
        }
        Ok(())
    }

    pub fn distribution_mode(&self) -> ImageDistribution {
        ImageDistribution::parse(&self.distribution).unwrap_or_default()
    }
}

/// Database row of `app_multi_server_settings`.
#[derive(Debug, Clone, FromRow)]
pub struct MultiServerSettingsRow {
    pub app_id: String,
    pub max_unavailable: i64,
    pub failure_threshold_pct: i64,
    pub distribution: String,
    pub lb_policy: String,
    pub sticky_sessions: i64,
    pub bind_private: i64,
}

impl From<MultiServerSettingsRow> for MultiServerSettings {
    fn from(r: MultiServerSettingsRow) -> Self {
        Self {
            max_unavailable: r.max_unavailable,
            failure_threshold_pct: r.failure_threshold_pct,
            distribution: r.distribution,
            lb_policy: r.lb_policy,
            sticky_sessions: r.sticky_sessions != 0,
            bind_private: r.bind_private != 0,
        }
    }
}

/// One container slot of a multi-server deployment.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeploymentTarget {
    pub id: String,
    pub deployment_id: String,
    pub app_id: String,
    pub server_id: String,
    pub server_name: String,
    pub replica_index: i64,
    pub batch: i64,
    /// pending, distributing, starting, checking, healthy, failed, rolled_back, replaced
    pub status: String,
    pub container_name: Option<String>,
    pub container_id: Option<String>,
    /// "host:port" the proxy forwards to.
    pub upstream: Option<String>,
    pub error_message: Option<String>,
    pub log: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
}

/// A placement entry in a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementInput {
    pub server_id: String,
    #[serde(default = "default_replicas")]
    pub replicas: i64,
}

fn default_replicas() -> i64 {
    1
}

/// Body of `PUT /api/apps/:id/placement`.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlacementRequest {
    pub placements: Vec<PlacementInput>,
    #[serde(default)]
    pub settings: Option<MultiServerSettings>,
}

/// Check a placement list from a request: known shape, no duplicates, sane counts.
/// Server existence is checked by the handler.
pub fn validate_placements(placements: &[PlacementInput]) -> Result<(), String> {
    if placements.len() > MAX_PLACEMENT_SERVERS {
        return Err(format!(
            "At most {} servers can be in a placement",
            MAX_PLACEMENT_SERVERS
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for p in placements {
        if p.server_id.trim().is_empty() {
            return Err("server_id is required".to_string());
        }
        if !seen.insert(p.server_id.as_str()) {
            return Err(format!("Server {} is listed twice", p.server_id));
        }
        if !(1..=MAX_REPLICAS_PER_SERVER).contains(&p.replicas) {
            return Err(format!(
                "replicas must be between 1 and {} per server",
                MAX_REPLICAS_PER_SERVER
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, replicas: i64) -> PlacementInput {
        PlacementInput {
            server_id: id.to_string(),
            replicas,
        }
    }

    #[test]
    fn placement_validation() {
        assert!(validate_placements(&[]).is_ok());
        assert!(validate_placements(&[p("localhost", 1), p("s1", 3)]).is_ok());
        assert!(validate_placements(&[p("s1", 1), p("s1", 2)]).is_err());
        assert!(validate_placements(&[p("s1", 0)]).is_err());
        assert!(validate_placements(&[p("s1", 11)]).is_err());
        assert!(validate_placements(&[p(" ", 1)]).is_err());
        let many: Vec<_> = (0..21).map(|i| p(&format!("s{i}"), 1)).collect();
        assert!(validate_placements(&many).is_err());
    }

    #[test]
    fn settings_validation() {
        let ok = MultiServerSettings::default();
        assert!(ok.validate().is_ok());
        let bad = MultiServerSettings {
            max_unavailable: 0,
            ..ok.clone()
        };
        assert!(bad.validate().is_err());
        let bad = MultiServerSettings {
            failure_threshold_pct: 101,
            ..ok.clone()
        };
        assert!(bad.validate().is_err());
        let bad = MultiServerSettings {
            distribution: "ftp".into(),
            ..ok.clone()
        };
        assert!(bad.validate().is_err());
        let bad = MultiServerSettings {
            lb_policy: "random".into(),
            ..ok
        };
        assert!(bad.validate().is_err());
    }
}
