//! HashiCorp Vault and OpenBao (same HTTP API): KV v2 reads, token or AppRole auth,
//! optional namespace.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use super::{
    base_url, http_client, json_object_to_map, non_empty, required, status_error, ConnectionConfig,
    ConnectionCredentials, Listing, ListingEntry, ProviderError, SecretData, SecretProvider,
};
use crate::secret_managers::sigv4::uri_encode;

enum Auth {
    Token(String),
    AppRole {
        mount: String,
        role_id: String,
        secret_id: String,
    },
}

pub struct VaultProvider {
    client: reqwest::Client,
    address: String,
    mount: String,
    namespace: Option<String>,
    auth: Auth,
    /// Token from an AppRole login, reused for the life of this client (one deploy).
    login_token: Mutex<Option<String>>,
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(uri_encode)
        .collect::<Vec<_>>()
        .join("/")
}

impl VaultProvider {
    pub fn new(
        config: &ConnectionConfig,
        creds: &ConnectionCredentials,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        let address = base_url(required(&config.address, "Vault address")?);
        let auth = match non_empty(&config.auth_method).unwrap_or("token") {
            "token" => Auth::Token(required(&creds.token, "Vault token")?.to_string()),
            "approle" => Auth::AppRole {
                mount: non_empty(&config.approle_mount)
                    .unwrap_or("approle")
                    .trim_matches('/')
                    .to_string(),
                role_id: required(&config.role_id, "AppRole role_id")?.to_string(),
                secret_id: required(&creds.secret_id, "AppRole secret_id")?.to_string(),
            },
            other => {
                return Err(ProviderError::Config(format!(
                    "unsupported Vault auth method '{}' (use token or approle)",
                    other
                )))
            }
        };
        Ok(Self {
            client: http_client(timeout),
            address,
            mount: non_empty(&config.mount)
                .unwrap_or("secret")
                .trim_matches('/')
                .to_string(),
            namespace: non_empty(&config.namespace).map(str::to_string),
            auth,
            login_token: Mutex::new(None),
        })
    }

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.request(method, url);
        if let Some(ns) = &self.namespace {
            req = req.header("X-Vault-Namespace", ns);
        }
        req
    }

    async fn token(&self) -> Result<String, ProviderError> {
        match &self.auth {
            Auth::Token(t) => Ok(t.clone()),
            Auth::AppRole {
                mount,
                role_id,
                secret_id,
            } => {
                let mut cached = self.login_token.lock().await;
                if let Some(t) = cached.as_ref() {
                    return Ok(t.clone());
                }
                let url = format!("{}/v1/auth/{}/login", self.address, encode_path(mount));
                let resp = self
                    .request(reqwest::Method::POST, &url)
                    .json(&serde_json::json!({ "role_id": role_id, "secret_id": secret_id }))
                    .send()
                    .await
                    .map_err(ProviderError::from_reqwest)?;
                if !resp.status().is_success() {
                    let e = status_error(resp, "AppRole login").await;
                    return Err(match e {
                        ProviderError::Http(m) | ProviderError::NotFound(m) => {
                            ProviderError::Auth(m)
                        }
                        other => other,
                    });
                }
                let body: Value = resp
                    .json()
                    .await
                    .map_err(|e| ProviderError::Invalid(e.without_url().to_string()))?;
                let token = body["auth"]["client_token"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .ok_or_else(|| {
                        ProviderError::Invalid("AppRole login returned no client_token".into())
                    })?
                    .to_string();
                *cached = Some(token.clone());
                Ok(token)
            }
        }
    }

    async fn get_json(
        &self,
        method: reqwest::Method,
        url: &str,
        what: &str,
    ) -> Result<Value, ProviderError> {
        let token = self.token().await?;
        let resp = self
            .request(method, url)
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;
        if !resp.status().is_success() {
            return Err(status_error(resp, what).await);
        }
        resp.json()
            .await
            .map_err(|e| ProviderError::Invalid(e.without_url().to_string()))
    }
}

#[async_trait]
impl SecretProvider for VaultProvider {
    async fn read(&self, path: &str) -> Result<SecretData, ProviderError> {
        let path = encode_path(path);
        if path.is_empty() {
            return Err(ProviderError::Config(
                "a Vault reference needs a secret path".into(),
            ));
        }
        let url = format!(
            "{}/v1/{}/data/{}",
            self.address,
            encode_path(&self.mount),
            path
        );
        let body = self
            .get_json(reqwest::Method::GET, &url, "Vault KV read")
            .await?;
        match body["data"]["data"].as_object() {
            Some(obj) => Ok(SecretData {
                values: json_object_to_map(obj),
                raw: None,
            }),
            // A deleted or destroyed latest version comes back with `data: null`.
            None => Err(ProviderError::NotFound(format!(
                "secret '{}' has no current version",
                path
            ))),
        }
    }

    async fn list(&self, path: &str) -> Result<Listing, ProviderError> {
        let clean = path.trim_matches('/');
        let url = format!(
            "{}/v1/{}/metadata/{}?list=true",
            self.address,
            encode_path(&self.mount),
            encode_path(clean)
        );
        let mut listing = Listing::default();
        match self
            .get_json(reqwest::Method::GET, &url, "Vault KV list")
            .await
        {
            Ok(body) => {
                for name in body["data"]["keys"].as_array().into_iter().flatten() {
                    let Some(name) = name.as_str() else { continue };
                    let is_folder = name.ends_with('/');
                    let short = name.trim_end_matches('/');
                    let full = if clean.is_empty() {
                        short.to_string()
                    } else {
                        format!("{}/{}", clean, short)
                    };
                    listing.entries.push(ListingEntry {
                        name: short.to_string(),
                        path: full,
                        kind: if is_folder { "folder" } else { "secret" },
                    });
                }
            }
            // No children (or not a folder) is a 404 on LIST.
            Err(ProviderError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
        if !clean.is_empty() {
            match self.read(clean).await {
                Ok(data) => listing.keys = data.values.into_keys().collect(),
                Err(ProviderError::NotFound(_)) => {}
                Err(e) if listing.entries.is_empty() => return Err(e),
                Err(_) => {}
            }
        }
        Ok(listing)
    }

    async fn test(&self) -> Result<String, ProviderError> {
        let url = format!("{}/v1/auth/token/lookup-self", self.address);
        let body = self
            .get_json(reqwest::Method::GET, &url, "Vault token lookup")
            .await?;
        let policies: Vec<&str> = body["data"]["policies"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        Ok(if policies.is_empty() {
            "Authenticated".to_string()
        } else {
            format!("Authenticated with policies: {}", policies.join(", "))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_server::serve;
    use super::*;
    use axum::{
        extract::{Path, Query},
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use std::collections::HashMap;

    fn authed(h: &HeaderMap) -> bool {
        h.get("x-vault-token").and_then(|v| v.to_str().ok()) == Some("good-token")
    }

    fn router() -> Router {
        Router::new()
            .route(
                "/v1/auth/approle/login",
                post(|Json(b): Json<Value>| async move {
                    if b["role_id"] == "role" && b["secret_id"] == "sid" {
                        (
                            StatusCode::OK,
                            Json(json!({"auth": {"client_token": "good-token"}})),
                        )
                    } else {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"errors": ["invalid role or secret ID"]})),
                        )
                    }
                }),
            )
            .route(
                "/v1/auth/token/lookup-self",
                get(|h: HeaderMap| async move {
                    if authed(&h) {
                        (
                            StatusCode::OK,
                            Json(json!({"data": {"policies": ["default", "app"]}})),
                        )
                    } else {
                        (
                            StatusCode::FORBIDDEN,
                            Json(json!({"errors": ["permission denied"]})),
                        )
                    }
                }),
            )
            .route(
                "/v1/kv/data/{*path}",
                get(|h: HeaderMap, Path(path): Path<String>| async move {
                    if !authed(&h) {
                        return (StatusCode::FORBIDDEN, Json(json!({"errors": []})));
                    }
                    // Namespaced requests must carry the header.
                    if path.starts_with("ns-only")
                        && h.get("x-vault-namespace").and_then(|v| v.to_str().ok())
                            != Some("team-a")
                    {
                        return (StatusCode::NOT_FOUND, Json(json!({"errors": []})));
                    }
                    match path.as_str() {
                        "app/config" | "ns-only/x" => (
                            StatusCode::OK,
                            Json(json!({"data": {"data": {
                                "DATABASE_URL": "postgres://db", "PORT": 5432
                            }, "metadata": {"version": 3}}})),
                        ),
                        "deleted" => (StatusCode::OK, Json(json!({"data": {"data": null}}))),
                        _ => (StatusCode::NOT_FOUND, Json(json!({"errors": []}))),
                    }
                }),
            )
            .route(
                "/v1/kv/metadata/",
                get(|q: Query<HashMap<String, String>>| async move {
                    assert_eq!(q.get("list").map(String::as_str), Some("true"));
                    Json(json!({"data": {"keys": ["app/", "top"]}}))
                }),
            )
            .route(
                "/v1/kv/metadata/{*path}",
                get(|Path(path): Path<String>| async move {
                    if path == "app" {
                        (StatusCode::OK, Json(json!({"data": {"keys": ["config"]}})))
                    } else {
                        (StatusCode::NOT_FOUND, Json(json!({"errors": []})))
                    }
                }),
            )
            .route(
                "/v1/slow/data/{*path}",
                get(|| async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Json(json!({}))
                }),
            )
    }

    fn cfg(addr: &str) -> ConnectionConfig {
        ConnectionConfig {
            address: Some(addr.to_string()),
            mount: Some("kv".into()),
            ..Default::default()
        }
    }

    fn token(t: &str) -> ConnectionCredentials {
        ConnectionCredentials {
            token: Some(t.into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn token_auth_reads_kv_v2() {
        let addr = serve(router()).await;
        let p = VaultProvider::new(&cfg(&addr), &token("good-token"), DEFAULT).unwrap();
        let data = p.read("/app/config/").await.unwrap();
        assert_eq!(data.values["DATABASE_URL"], "postgres://db");
        assert_eq!(data.values["PORT"], "5432");
        assert!(p.test().await.unwrap().contains("default, app"));
    }

    const DEFAULT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn bad_token_is_an_auth_error() {
        let addr = serve(router()).await;
        let p = VaultProvider::new(&cfg(&addr), &token("nope"), DEFAULT).unwrap();
        assert!(matches!(
            p.read("app/config").await,
            Err(ProviderError::Auth(_))
        ));
        assert!(matches!(p.test().await, Err(ProviderError::Auth(_))));
    }

    #[tokio::test]
    async fn approle_login_then_read() {
        let addr = serve(router()).await;
        let mut c = cfg(&addr);
        c.auth_method = Some("approle".into());
        c.role_id = Some("role".into());
        let creds = ConnectionCredentials {
            secret_id: Some("sid".into()),
            ..Default::default()
        };
        let p = VaultProvider::new(&c, &creds, DEFAULT).unwrap();
        assert_eq!(
            p.read("app/config").await.unwrap().values["DATABASE_URL"],
            "postgres://db"
        );

        let bad = ConnectionCredentials {
            secret_id: Some("wrong".into()),
            ..Default::default()
        };
        let p = VaultProvider::new(&c, &bad, DEFAULT).unwrap();
        assert!(matches!(p.test().await, Err(ProviderError::Auth(_))));
    }

    #[tokio::test]
    async fn namespace_header_is_sent() {
        let addr = serve(router()).await;
        let p = VaultProvider::new(&cfg(&addr), &token("good-token"), DEFAULT).unwrap();
        assert!(matches!(
            p.read("ns-only/x").await,
            Err(ProviderError::NotFound(_))
        ));
        let mut c = cfg(&addr);
        c.namespace = Some("team-a".into());
        let p = VaultProvider::new(&c, &token("good-token"), DEFAULT).unwrap();
        assert!(p.read("ns-only/x").await.is_ok());
    }

    #[tokio::test]
    async fn missing_and_deleted_secrets() {
        let addr = serve(router()).await;
        let p = VaultProvider::new(&cfg(&addr), &token("good-token"), DEFAULT).unwrap();
        assert!(matches!(
            p.read("nope").await,
            Err(ProviderError::NotFound(_))
        ));
        assert!(matches!(
            p.read("deleted").await,
            Err(ProviderError::NotFound(_))
        ));
        assert!(matches!(p.read("").await, Err(ProviderError::Config(_))));
    }

    #[tokio::test]
    async fn browse_lists_folders_secrets_and_keys() {
        let addr = serve(router()).await;
        let p = VaultProvider::new(&cfg(&addr), &token("good-token"), DEFAULT).unwrap();
        let root = p.list("").await.unwrap();
        assert_eq!(root.entries.len(), 2);
        assert_eq!(root.entries[0].kind, "folder");
        assert_eq!(root.entries[0].path, "app");
        assert_eq!(root.entries[1].kind, "secret");
        let app = p.list("app").await.unwrap();
        assert_eq!(app.entries[0].path, "app/config");
        let leaf = p.list("app/config").await.unwrap();
        assert!(leaf.entries.is_empty());
        assert_eq!(leaf.keys, vec!["DATABASE_URL", "PORT"]);
    }

    #[tokio::test]
    async fn slow_server_times_out() {
        let addr = serve(router()).await;
        let mut c = cfg(&addr);
        c.mount = Some("slow".into());
        let p = VaultProvider::new(&c, &token("good-token"), Duration::from_millis(300)).unwrap();
        assert!(matches!(p.read("x").await, Err(ProviderError::Timeout)));
    }

    #[test]
    fn config_validation() {
        assert!(VaultProvider::new(&ConnectionConfig::default(), &token("t"), DEFAULT).is_err());
        let addr = "http://127.0.0.1:1";
        assert!(
            VaultProvider::new(&cfg(addr), &ConnectionCredentials::default(), DEFAULT).is_err()
        );
        let mut c = cfg(addr);
        c.auth_method = Some("kubernetes".into());
        assert!(VaultProvider::new(&c, &token("t"), DEFAULT).is_err());
    }
}
