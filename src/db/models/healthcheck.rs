//! Structured application healthcheck.
//!
//! Stored as JSON in `apps.healthcheck_config`. The legacy single path in
//! `apps.healthcheck` is still honoured: when no structured config exists it maps to
//! an HTTP GET on that path with the default timing (see [`HealthcheckConfig::from_legacy_path`]).

use serde::{Deserialize, Serialize};

use crate::runtime::ContainerHealthcheck;

/// How the health of a container is checked.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HealthcheckType {
    /// HTTP request against the app port.
    #[default]
    Http,
    /// Shell command run inside the container; exit code 0 means healthy.
    Command,
}

fn default_true() -> bool {
    true
}
fn default_method() -> String {
    "GET".to_string()
}
fn default_scheme() -> String {
    "http".to_string()
}
fn default_host() -> String {
    "localhost".to_string()
}
fn default_path() -> String {
    "/".to_string()
}
fn default_status() -> u16 {
    200
}
fn default_interval() -> u32 {
    5
}
fn default_timeout() -> u32 {
    5
}
fn default_retries() -> u32 {
    10
}
fn default_start_period() -> u32 {
    5
}

/// Structured healthcheck settings for an app.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthcheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub check_type: HealthcheckType,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_scheme")]
    pub scheme: String,
    /// Host used inside the container (and sent as the Host header from the pipeline).
    #[serde(default = "default_host")]
    pub host: String,
    /// Port inside the container. `None` uses the app port.
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default = "default_status")]
    pub expected_status: u16,
    /// Text the response body must contain. Empty or `None` skips the body check.
    #[serde(default)]
    pub expected_body: Option<String>,
    /// Command for `check_type = command`.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default = "default_interval")]
    pub interval_secs: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u32,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_start_period")]
    pub start_period_secs: u32,
}

impl Default for HealthcheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_type: HealthcheckType::Http,
            method: default_method(),
            scheme: default_scheme(),
            host: default_host(),
            port: None,
            path: default_path(),
            expected_status: default_status(),
            expected_body: None,
            command: None,
            interval_secs: default_interval(),
            timeout_secs: default_timeout(),
            retries: default_retries(),
            start_period_secs: default_start_period(),
        }
    }
}

const ALLOWED_METHODS: &[&str] = &["GET", "HEAD", "POST", "OPTIONS"];

/// Quote a value for a POSIX shell (single quotes, embedded quotes escaped).
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

impl HealthcheckConfig {
    /// The structured equivalent of the legacy single healthcheck path.
    pub fn from_legacy_path(path: &str) -> Self {
        Self {
            path: path.to_string(),
            ..Self::default()
        }
    }

    /// Parse the stored JSON, falling back to the legacy path. Returns `None` when
    /// the app has no healthcheck at all.
    pub fn resolve(config_json: Option<&str>, legacy_path: Option<&str>) -> Option<Self> {
        if let Some(cfg) = config_json
            .filter(|s| !s.trim().is_empty())
            .and_then(|s| serde_json::from_str::<Self>(s).ok())
        {
            return Some(cfg);
        }
        legacy_path
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(Self::from_legacy_path)
    }

    /// Normalise casing and trim free-text fields.
    pub fn normalized(mut self) -> Self {
        self.method = self.method.trim().to_uppercase();
        self.scheme = self.scheme.trim().to_lowercase();
        self.host = self.host.trim().to_string();
        if self.host.is_empty() {
            self.host = default_host();
        }
        self.path = self.path.trim().to_string();
        if self.path.is_empty() {
            self.path = default_path();
        }
        self.expected_body = self
            .expected_body
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self.command = self
            .command
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self
    }

    /// Validate the settings. Errors are user-facing.
    pub fn validate(&self) -> Result<(), String> {
        if !ALLOWED_METHODS.contains(&self.method.as_str()) {
            return Err(format!(
                "Method must be one of {}",
                ALLOWED_METHODS.join(", ")
            ));
        }
        if self.scheme != "http" && self.scheme != "https" {
            return Err("Scheme must be http or https".to_string());
        }
        if self.host.len() > 253
            || !self
                .host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            return Err("Host may only contain letters, digits, dots and dashes".to_string());
        }
        if !self.path.starts_with('/') {
            return Err("Path must start with /".to_string());
        }
        if self.path.len() > 512
            || self
                .path
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err("Path must be at most 512 characters without spaces".to_string());
        }
        if !(100..=599).contains(&self.expected_status) {
            return Err("Expected status code must be between 100 and 599".to_string());
        }
        if let Some(body) = &self.expected_body {
            if body.len() > 256 || body.chars().any(|c| c.is_control()) {
                return Err("Expected response text must be at most 256 characters".to_string());
            }
        }
        if self.port == Some(0) {
            return Err("Port must be between 1 and 65535".to_string());
        }
        if self.check_type == HealthcheckType::Command {
            match &self.command {
                None => return Err("A command is required for a command healthcheck".to_string()),
                Some(c) if c.len() > 1024 || c.contains('\0') => {
                    return Err("Command must be at most 1024 characters".to_string())
                }
                _ => {}
            }
        }
        if !(1..=3600).contains(&self.interval_secs) {
            return Err("Interval must be between 1 and 3600 seconds".to_string());
        }
        if !(1..=3600).contains(&self.timeout_secs) {
            return Err("Timeout must be between 1 and 3600 seconds".to_string());
        }
        if !(1..=100).contains(&self.retries) {
            return Err("Retries must be between 1 and 100".to_string());
        }
        if self.start_period_secs > 3600 {
            return Err("Start period must be at most 3600 seconds".to_string());
        }
        Ok(())
    }

    /// The container port the check targets.
    pub fn target_port(&self, app_port: u16) -> u16 {
        self.port.unwrap_or(app_port)
    }

    /// The URL checked from inside the container.
    pub fn container_url(&self, app_port: u16) -> String {
        format!(
            "{}://{}:{}{}",
            self.scheme,
            self.host,
            self.target_port(app_port),
            self.path
        )
    }

    /// The path the proxy health checker should use, if this is an enabled HTTP check.
    pub fn proxy_path(&self) -> Option<String> {
        (self.enabled && self.check_type == HealthcheckType::Http).then(|| self.path.clone())
    }

    /// Shell command implementing the HTTP check inside the container. Uses curl when
    /// present, then wget; images without either are treated as healthy so a missing
    /// tool never marks a working app unhealthy.
    fn http_shell_command(&self, app_port: u16) -> String {
        let url = sh_quote(&self.container_url(app_port));
        let timeout = self.timeout_secs;
        let status = self.expected_status;
        let insecure = self.scheme == "https";
        let body_check = self
            .expected_body
            .as_deref()
            .map(|b| format!(" && printf '%s' \"$out\" | grep -qF -- {}", sh_quote(b)))
            .unwrap_or_default();

        let curl_method = if self.method == "HEAD" {
            "-I".to_string()
        } else {
            format!("-X {}", self.method)
        };
        // 200 (the default) accepts any 2xx, matching the deploy-time check.
        let status_test = if status == 200 {
            "case \"$code\" in 2??) true;; *) false;; esac".to_string()
        } else {
            format!("[ \"$code\" = \"{}\" ]", status)
        };
        let curl = format!(
            "out=$(curl -s{k} {m} --max-time {t} -w '\\n%{{http_code}}' {url}) || exit 1; \
             code=$(printf '%s' \"$out\" | tail -n 1); {status_test}{body}",
            k = if insecure { "k" } else { "" },
            m = curl_method,
            t = timeout,
            url = url,
            status_test = status_test,
            body = body_check,
        );
        let wget = format!(
            "out=$(wget -q {k}-T {t} -O - {url}) || exit 1; true{body}",
            k = if insecure {
                "--no-check-certificate "
            } else {
                ""
            },
            t = timeout,
            url = url,
            body = body_check,
        );
        format!(
            "if command -v curl >/dev/null 2>&1; then {curl}; \
             elif command -v wget >/dev/null 2>&1; then {wget}; \
             else exit 0; fi"
        )
    }

    /// Map to a container HEALTHCHECK. `None` when disabled (the image's own
    /// HEALTHCHECK, if any, is left in place).
    pub fn to_container_healthcheck(&self, app_port: u16) -> Option<ContainerHealthcheck> {
        if !self.enabled {
            return None;
        }
        let shell = match self.check_type {
            HealthcheckType::Http => self.http_shell_command(app_port),
            HealthcheckType::Command => self.command.clone()?,
        };
        Some(ContainerHealthcheck {
            test: vec!["CMD-SHELL".to_string(), shell],
            interval_secs: self.interval_secs,
            timeout_secs: self.timeout_secs,
            retries: self.retries,
            start_period_secs: self.start_period_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_path_maps_to_http_get_with_defaults() {
        let cfg = HealthcheckConfig::resolve(None, Some("/health")).unwrap();
        assert!(cfg.enabled);
        assert_eq!(cfg.check_type, HealthcheckType::Http);
        assert_eq!(cfg.method, "GET");
        assert_eq!(cfg.path, "/health");
        assert_eq!(cfg.expected_status, 200);
        assert_eq!(cfg.container_url(3000), "http://localhost:3000/health");
        assert!(HealthcheckConfig::resolve(None, Some("  ")).is_none());
        assert!(HealthcheckConfig::resolve(None, None).is_none());
    }

    #[test]
    fn stored_config_wins_over_legacy_path() {
        let json = r#"{"enabled":true,"check_type":"http","path":"/ready","port":9000}"#;
        let cfg = HealthcheckConfig::resolve(Some(json), Some("/health")).unwrap();
        assert_eq!(cfg.path, "/ready");
        assert_eq!(cfg.target_port(3000), 9000);
        // Missing fields take defaults.
        assert_eq!(cfg.retries, 10);
        assert_eq!(cfg.scheme, "http");
    }

    #[test]
    fn http_check_maps_to_container_healthcheck() {
        let cfg = HealthcheckConfig {
            path: "/health".into(),
            expected_status: 204,
            expected_body: Some("ok".into()),
            interval_secs: 10,
            timeout_secs: 3,
            retries: 4,
            start_period_secs: 20,
            ..Default::default()
        };
        let hc = cfg.to_container_healthcheck(8080).unwrap();
        assert_eq!(hc.test[0], "CMD-SHELL");
        let cmd = &hc.test[1];
        assert!(cmd.contains("'http://localhost:8080/health'"));
        assert!(cmd.contains("--max-time 3"));
        assert!(cmd.contains("\"204\""));
        assert!(cmd.contains("grep -qF -- 'ok'"));
        assert!(cmd.contains("wget -q -T 3"));
        assert!(cmd.ends_with("else exit 0; fi"));
        assert_eq!(
            (
                hc.interval_secs,
                hc.timeout_secs,
                hc.retries,
                hc.start_period_secs
            ),
            (10, 3, 4, 20)
        );
    }

    #[test]
    fn command_check_maps_verbatim_and_disabled_maps_to_none() {
        let cfg = HealthcheckConfig {
            check_type: HealthcheckType::Command,
            command: Some("pg_isready -U postgres".into()),
            ..Default::default()
        };
        let hc = cfg.to_container_healthcheck(5432).unwrap();
        assert_eq!(hc.test, vec!["CMD-SHELL", "pg_isready -U postgres"]);
        assert!(cfg.proxy_path().is_none());

        let off = HealthcheckConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(off.to_container_healthcheck(3000).is_none());
        assert!(off.proxy_path().is_none());
    }

    #[test]
    fn https_and_head_use_matching_flags() {
        let cfg = HealthcheckConfig {
            scheme: "https".into(),
            method: "HEAD".into(),
            ..Default::default()
        };
        let cmd = cfg.to_container_healthcheck(443).unwrap().test[1].clone();
        assert!(cmd.contains("curl -sk -I"));
        assert!(cmd.contains("--no-check-certificate"));
    }

    #[test]
    fn quotes_in_values_cannot_break_out_of_the_shell_string() {
        let cfg = HealthcheckConfig {
            expected_body: Some("it's fine".into()),
            ..Default::default()
        };
        let cmd = cfg.to_container_healthcheck(3000).unwrap().test[1].clone();
        assert!(cmd.contains(r"'it'\''s fine'"));
    }

    #[test]
    fn validation_rejects_bad_values() {
        let ok = HealthcheckConfig::default();
        assert!(ok.validate().is_ok());
        let bad_host = HealthcheckConfig {
            host: "localhost;rm".into(),
            ..Default::default()
        };
        assert!(bad_host.validate().is_err());
        let bad_path = HealthcheckConfig {
            path: "health".into(),
            ..Default::default()
        };
        assert!(bad_path.validate().is_err());
        let bad_method = HealthcheckConfig {
            method: "DELETE".into(),
            ..Default::default()
        };
        assert!(bad_method.validate().is_err());
        let no_cmd = HealthcheckConfig {
            check_type: HealthcheckType::Command,
            ..Default::default()
        };
        assert!(no_cmd.validate().is_err());
        let zero_retries = HealthcheckConfig {
            retries: 0,
            ..Default::default()
        };
        assert!(zero_retries.validate().is_err());
    }

    #[test]
    fn normalized_uppercases_method_and_fills_blanks() {
        let cfg = HealthcheckConfig {
            method: " get ".into(),
            scheme: "HTTP".into(),
            host: "".into(),
            path: "".into(),
            expected_body: Some("  ".into()),
            ..Default::default()
        }
        .normalized();
        assert_eq!(cfg.method, "GET");
        assert_eq!(cfg.scheme, "http");
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.path, "/");
        assert!(cfg.expected_body.is_none());
    }
}
