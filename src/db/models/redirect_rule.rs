//! App redirect rule models and DTOs.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A URL redirect rule associated with an app, enforced at the proxy level.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppRedirectRule {
    pub id: String,
    pub app_id: String,
    /// Regex pattern matched against the request path (e.g., `^/old-path(.*)`)
    pub source_pattern: String,
    /// Redirect destination; may reference capture groups as `$1`, `$2`, etc.
    pub destination: String,
    /// 1 = 301 Moved Permanently, 0 = 302 Found
    pub is_permanent: i32,
    /// 1 = enabled, 0 = disabled
    pub is_enabled: i32,
    /// Evaluation priority; lower value = evaluated first
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl AppRedirectRule {
    /// Returns true if the rule is currently enabled.
    pub fn enabled(&self) -> bool {
        self.is_enabled != 0
    }

    /// Returns true if the redirect should be 301 Permanent.
    pub fn permanent(&self) -> bool {
        self.is_permanent != 0
    }
}

// ---- DTOs ----

/// Request to create a new redirect rule.
#[derive(Debug, Deserialize)]
pub struct CreateRedirectRuleRequest {
    /// Regex pattern matched against the request path.
    pub source_pattern: String,
    /// Redirect destination (supports `$1`, `$2` capture-group substitution).
    pub destination: String,
    /// Whether the redirect should be 301 (permanent) instead of 302.
    #[serde(default)]
    pub is_permanent: bool,
    /// Whether the rule is active. Defaults to true.
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,
    /// Sort order (lower = evaluated first). Defaults to 0.
    #[serde(default)]
    pub sort_order: i32,
}

/// Request to update an existing redirect rule.
#[derive(Debug, Deserialize)]
pub struct UpdateRedirectRuleRequest {
    pub source_pattern: Option<String>,
    pub destination: Option<String>,
    pub is_permanent: Option<bool>,
    pub is_enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

fn default_enabled() -> bool {
    true
}

/// A lightweight version of the rule used in the proxy route table.
#[derive(Debug, Clone)]
pub struct RedirectRule {
    /// Compiled regex pattern.
    pub source_pattern: String,
    /// Redirect destination template.
    pub destination: String,
    /// True for 301, false for 302.
    pub is_permanent: bool,
}
