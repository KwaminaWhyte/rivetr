//! Secret manager clients behind one trait. Each is a small reqwest client for the few
//! endpoints Rivetr needs; no provider SDKs.

pub mod aws;
pub mod doppler;
pub mod infisical;
pub mod vault;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Default timeout for a provider HTTP call.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Supported provider kinds (the `provider` column).
pub const PROVIDERS: &[&str] = &[
    "vault",
    "openbao",
    "infisical",
    "doppler",
    "aws_secrets_manager",
];

/// Error from a provider call. Messages never contain secret values: response bodies are
/// truncated and only included for error statuses.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("{0}")]
    Config(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("request timed out")]
    Timeout,
    #[error("{0}")]
    Http(String),
    #[error("unexpected response: {0}")]
    Invalid(String),
}

impl ProviderError {
    pub(crate) fn from_reqwest(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else if e.is_connect() {
            Self::Http(format!("could not connect: {}", e.without_url()))
        } else {
            Self::Http(e.without_url().to_string())
        }
    }
}

/// All key/value pairs stored at one path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecretData {
    pub values: BTreeMap<String, String>,
    /// The raw secret string, for providers whose secrets are a single string
    /// (AWS Secrets Manager `SecretString`). Used when a reference has no `#key`.
    pub raw: Option<String>,
}

/// A browse result: child folders/secrets under a path and the key names at the path.
/// Never contains values.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Listing {
    /// Child paths that can be browsed further (folders, or secrets holding keys).
    pub entries: Vec<ListingEntry>,
    /// Key names stored at this path.
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListingEntry {
    pub name: String,
    /// Full path to pass back to browse.
    pub path: String,
    /// "folder" or "secret".
    pub kind: &'static str,
}

#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Read every key stored at `path`.
    async fn read(&self, path: &str) -> Result<SecretData, ProviderError>;

    /// Browse `path` (folders/secrets below it, key names at it).
    async fn list(&self, path: &str) -> Result<Listing, ProviderError>;

    /// Check the connection and credentials. Returns a short human-readable summary.
    async fn test(&self) -> Result<String, ProviderError>;
}

/// Non-secret connection settings (stored as JSON in `secret_connections.config`).
/// One flat struct for every provider; each uses the fields it needs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ConnectionConfig {
    /// "token" | "approle" (Vault/OpenBao), "access_key" | "instance_role" (AWS).
    pub auth_method: Option<String>,

    // Vault / OpenBao
    pub address: Option<String>,
    /// KV v2 mount (default "secret").
    pub mount: Option<String>,
    pub namespace: Option<String>,
    pub role_id: Option<String>,
    /// AppRole auth mount (default "approle").
    pub approle_mount: Option<String>,

    // Infisical
    /// Default "https://app.infisical.com".
    pub site_url: Option<String>,
    pub project_id: Option<String>,
    pub environment: Option<String>,
    pub client_id: Option<String>,

    // Doppler
    /// Default "https://api.doppler.com".
    pub api_url: Option<String>,
    pub project: Option<String>,
    pub config: Option<String>,

    // AWS Secrets Manager
    pub region: Option<String>,
    pub access_key_id: Option<String>,
    /// Endpoint override (VPC endpoint, LocalStack). Default is the regional endpoint.
    pub endpoint: Option<String>,
}

/// Secret connection fields (stored encrypted in `secret_connections.credentials`).
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ConnectionCredentials {
    /// Vault/OpenBao token, or Doppler service token.
    pub token: Option<String>,
    /// Vault/OpenBao AppRole secret_id.
    pub secret_id: Option<String>,
    /// Infisical universal auth client secret.
    pub client_secret: Option<String>,
    /// AWS secret access key.
    pub secret_access_key: Option<String>,
    /// AWS session token (temporary credentials).
    pub session_token: Option<String>,
}

impl std::fmt::Debug for ConnectionCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let set = |o: &Option<String>| o.as_ref().map(|_| "********");
        f.debug_struct("ConnectionCredentials")
            .field("token", &set(&self.token))
            .field("secret_id", &set(&self.secret_id))
            .field("client_secret", &set(&self.client_secret))
            .field("secret_access_key", &set(&self.secret_access_key))
            .field("session_token", &set(&self.session_token))
            .finish()
    }
}

impl ConnectionCredentials {
    /// Fill unset fields from `previous` (an update that leaves a secret blank keeps it).
    pub fn merged_over(mut self, previous: &ConnectionCredentials) -> Self {
        fn keep(new: &mut Option<String>, old: &Option<String>) {
            if new.as_deref().is_none_or(str::is_empty) {
                *new = old.clone();
            }
        }
        keep(&mut self.token, &previous.token);
        keep(&mut self.secret_id, &previous.secret_id);
        keep(&mut self.client_secret, &previous.client_secret);
        keep(&mut self.secret_access_key, &previous.secret_access_key);
        keep(&mut self.session_token, &previous.session_token);
        self
    }
}

pub(crate) fn non_empty(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

pub(crate) fn required<'a>(v: &'a Option<String>, what: &str) -> Result<&'a str, ProviderError> {
    non_empty(v).ok_or_else(|| ProviderError::Config(format!("{} is required", what)))
}

/// Build an HTTP client with the given timeout.
pub(crate) fn http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout.min(Duration::from_secs(10)))
        .user_agent(concat!("rivetr/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Turn an error status into a [`ProviderError`] with a short, truncated body.
pub(crate) async fn status_error(resp: reqwest::Response, what: &str) -> ProviderError {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let mut snippet: String = body.chars().take(200).collect();
    if body.chars().count() > 200 {
        snippet.push_str("...");
    }
    let msg = if snippet.trim().is_empty() {
        format!("{} returned HTTP {}", what, status.as_u16())
    } else {
        format!(
            "{} returned HTTP {}: {}",
            what,
            status.as_u16(),
            snippet.trim()
        )
    };
    match status.as_u16() {
        401 | 403 => ProviderError::Auth(msg),
        404 => ProviderError::NotFound(msg),
        _ => ProviderError::Http(msg),
    }
}

/// Trim a base URL's trailing slash.
pub(crate) fn base_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Build the provider for a connection.
pub fn build_provider(
    provider: &str,
    config: &ConnectionConfig,
    creds: &ConnectionCredentials,
    timeout: Duration,
) -> Result<Arc<dyn SecretProvider>, ProviderError> {
    match provider {
        "vault" | "openbao" => Ok(Arc::new(vault::VaultProvider::new(config, creds, timeout)?)),
        "infisical" => Ok(Arc::new(infisical::InfisicalProvider::new(
            config, creds, timeout,
        )?)),
        "doppler" => Ok(Arc::new(doppler::DopplerProvider::new(
            config, creds, timeout,
        )?)),
        "aws_secrets_manager" => Ok(Arc::new(aws::AwsSecretsManagerProvider::new(
            config, creds, timeout,
        )?)),
        other => Err(ProviderError::Config(format!(
            "unknown provider '{}'",
            other
        ))),
    }
}

/// Every URL a connection will call (for SSRF checks when saving).
pub fn outbound_urls(provider: &str, config: &ConnectionConfig) -> Vec<String> {
    match provider {
        "vault" | "openbao" => non_empty(&config.address)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        "infisical" => non_empty(&config.site_url)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        "doppler" => non_empty(&config.api_url)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        "aws_secrets_manager" => non_empty(&config.endpoint)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Flatten a JSON object into string values: strings as-is, other scalars and nested
/// values as their JSON text.
pub(crate) fn json_object_to_map(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, String> {
    obj.iter()
        .map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => String::new(),
                other => other.to_string(),
            };
            (k.clone(), s)
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod test_server {
    //! In-process HTTP mock used by the provider tests.
    use axum::Router;
    use std::net::SocketAddr;

    /// Serve `router` on a random local port and return its base URL.
    pub async fn serve(router: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        format!("http://{}", addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_merge_keeps_blank_fields() {
        let old = ConnectionCredentials {
            token: Some("old".into()),
            client_secret: Some("cs".into()),
            ..Default::default()
        };
        let new = ConnectionCredentials {
            token: Some("".into()),
            secret_id: Some("sid".into()),
            ..Default::default()
        }
        .merged_over(&old);
        assert_eq!(new.token.as_deref(), Some("old"));
        assert_eq!(new.secret_id.as_deref(), Some("sid"));
        assert_eq!(new.client_secret.as_deref(), Some("cs"));
    }

    #[test]
    fn credentials_debug_is_masked() {
        let c = ConnectionCredentials {
            token: Some("hvs.supersecret".into()),
            ..Default::default()
        };
        assert!(!format!("{:?}", c).contains("supersecret"));
    }

    #[test]
    fn unknown_provider_is_rejected() {
        assert!(build_provider(
            "nope",
            &ConnectionConfig::default(),
            &ConnectionCredentials::default(),
            DEFAULT_TIMEOUT
        )
        .is_err());
    }
}
