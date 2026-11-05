//! Infisical: universal auth (machine identity client id + secret), secrets of one
//! project + environment, the reference path is the folder.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use super::{
    base_url, http_client, non_empty, required, status_error, ConnectionConfig,
    ConnectionCredentials, Listing, ListingEntry, ProviderError, SecretData, SecretProvider,
};

const DEFAULT_SITE: &str = "https://app.infisical.com";

pub struct InfisicalProvider {
    client: reqwest::Client,
    site: String,
    project_id: String,
    environment: String,
    client_id: String,
    client_secret: String,
    access_token: Mutex<Option<String>>,
}

/// Infisical folder paths are absolute: "" and "a/b/" become "/" and "/a/b".
fn folder(path: &str) -> String {
    let p = path.trim_matches('/');
    if p.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", p)
    }
}

impl InfisicalProvider {
    pub fn new(
        config: &ConnectionConfig,
        creds: &ConnectionCredentials,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            client: http_client(timeout),
            site: base_url(non_empty(&config.site_url).unwrap_or(DEFAULT_SITE)),
            project_id: required(&config.project_id, "Infisical project ID")?.to_string(),
            environment: required(&config.environment, "Infisical environment slug")?.to_string(),
            client_id: required(&config.client_id, "Machine identity client ID")?.to_string(),
            client_secret: required(&creds.client_secret, "Machine identity client secret")?
                .to_string(),
            access_token: Mutex::new(None),
        })
    }

    async fn token(&self) -> Result<String, ProviderError> {
        let mut cached = self.access_token.lock().await;
        if let Some(t) = cached.as_ref() {
            return Ok(t.clone());
        }
        let resp = self
            .client
            .post(format!("{}/api/v1/auth/universal-auth/login", self.site))
            .json(&serde_json::json!({
                "clientId": self.client_id,
                "clientSecret": self.client_secret,
            }))
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;
        if !resp.status().is_success() {
            let e = status_error(resp, "Infisical login").await;
            return Err(match e {
                ProviderError::Http(m) => ProviderError::Auth(m),
                other => other,
            });
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Invalid(e.without_url().to_string()))?;
        let token = body["accessToken"]
            .as_str()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                ProviderError::Invalid("Infisical login returned no accessToken".into())
            })?
            .to_string();
        *cached = Some(token.clone());
        Ok(token)
    }

    async fn get(
        &self,
        url: &str,
        query: &[(&str, &str)],
        what: &str,
    ) -> Result<Value, ProviderError> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(url)
            .bearer_auth(token)
            .query(query)
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
impl SecretProvider for InfisicalProvider {
    async fn read(&self, path: &str) -> Result<SecretData, ProviderError> {
        let path = folder(path);
        let body = self
            .get(
                &format!("{}/api/v3/secrets/raw", self.site),
                &[
                    ("workspaceId", self.project_id.as_str()),
                    ("environment", self.environment.as_str()),
                    ("secretPath", path.as_str()),
                    ("expandSecretReferences", "true"),
                    ("include_imports", "true"),
                ],
                "Infisical secrets",
            )
            .await?;
        let mut data = SecretData::default();
        let mut add = |secrets: &Value, overwrite: bool| {
            for s in secrets.as_array().into_iter().flatten() {
                if let (Some(k), Some(v)) = (s["secretKey"].as_str(), s["secretValue"].as_str()) {
                    if overwrite || !data.values.contains_key(k) {
                        data.values.insert(k.to_string(), v.to_string());
                    }
                }
            }
        };
        add(&body["secrets"], true);
        // Imported secrets only fill keys the folder does not define itself.
        for import in body["imports"].as_array().into_iter().flatten() {
            add(&import["secrets"], false);
        }
        Ok(data)
    }

    async fn list(&self, path: &str) -> Result<Listing, ProviderError> {
        let path = folder(path);
        let body = self
            .get(
                &format!("{}/api/v1/folders", self.site),
                &[
                    ("workspaceId", self.project_id.as_str()),
                    ("environment", self.environment.as_str()),
                    ("path", path.as_str()),
                ],
                "Infisical folders",
            )
            .await?;
        let mut listing = Listing::default();
        for f in body["folders"].as_array().into_iter().flatten() {
            if let Some(name) = f["name"].as_str() {
                let full = format!("{}/{}", path.trim_end_matches('/'), name);
                listing.entries.push(ListingEntry {
                    name: name.to_string(),
                    path: full.trim_start_matches('/').to_string(),
                    kind: "folder",
                });
            }
        }
        listing.keys = self.read(&path).await?.values.into_keys().collect();
        Ok(listing)
    }

    async fn test(&self) -> Result<String, ProviderError> {
        let data = self.read("/").await?;
        Ok(format!(
            "Authenticated; {} secret(s) in {} at /",
            data.values.len(),
            self.environment
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_server::serve;
    use super::*;
    use axum::{
        extract::Query,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use std::collections::HashMap;

    const T: Duration = Duration::from_secs(5);

    fn ok(h: &HeaderMap) -> bool {
        h.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer at-123")
    }

    fn router() -> Router {
        Router::new()
            .route(
                "/api/v1/auth/universal-auth/login",
                post(|Json(b): Json<Value>| async move {
                    if b["clientId"] == "cid" && b["clientSecret"] == "csecret" {
                        (StatusCode::OK, Json(json!({"accessToken": "at-123", "expiresIn": 7200})))
                    } else {
                        (StatusCode::UNAUTHORIZED, Json(json!({"message": "Invalid credentials"})))
                    }
                }),
            )
            .route(
                "/api/v3/secrets/raw",
                get(|h: HeaderMap, Query(q): Query<HashMap<String, String>>| async move {
                    if !ok(&h) {
                        return (StatusCode::UNAUTHORIZED, Json(json!({})));
                    }
                    assert_eq!(q["workspaceId"], "proj");
                    assert_eq!(q["environment"], "prod");
                    match q["secretPath"].as_str() {
                        "/" => (StatusCode::OK, Json(json!({"secrets": [
                            {"secretKey": "ROOT_KEY", "secretValue": "r"}
                        ]}))),
                        "/backend" => (StatusCode::OK, Json(json!({
                            "secrets": [{"secretKey": "STRIPE_KEY", "secretValue": "sk_live"}],
                            "imports": [{"secrets": [
                                {"secretKey": "STRIPE_KEY", "secretValue": "imported"},
                                {"secretKey": "SHARED", "secretValue": "s"}
                            ]}]
                        }))),
                        _ => (StatusCode::NOT_FOUND, Json(json!({"message": "Folder not found"}))),
                    }
                }),
            )
            .route(
                "/api/v1/folders",
                get(|h: HeaderMap, Query(q): Query<HashMap<String, String>>| async move {
                    assert!(ok(&h));
                    if q["path"] == "/" {
                        Json(json!({"folders": [{"id": "1", "name": "backend"}]}))
                    } else {
                        Json(json!({"folders": []}))
                    }
                }),
            )
    }

    fn cfg(site: &str) -> ConnectionConfig {
        ConnectionConfig {
            site_url: Some(site.into()),
            project_id: Some("proj".into()),
            environment: Some("prod".into()),
            client_id: Some("cid".into()),
            ..Default::default()
        }
    }

    fn creds(s: &str) -> ConnectionCredentials {
        ConnectionCredentials {
            client_secret: Some(s.into()),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn universal_auth_and_folder_read() {
        let site = serve(router()).await;
        let p = InfisicalProvider::new(&cfg(&site), &creds("csecret"), T).unwrap();
        let d = p.read("backend/").await.unwrap();
        assert_eq!(d.values["STRIPE_KEY"], "sk_live");
        assert_eq!(d.values["SHARED"], "s");
        assert!(p.test().await.unwrap().contains("1 secret"));
    }

    #[tokio::test]
    async fn bad_secret_fails_auth() {
        let site = serve(router()).await;
        let p = InfisicalProvider::new(&cfg(&site), &creds("wrong"), T).unwrap();
        assert!(matches!(p.read("/").await, Err(ProviderError::Auth(_))));
    }

    #[tokio::test]
    async fn browse_and_missing_folder() {
        let site = serve(router()).await;
        let p = InfisicalProvider::new(&cfg(&site), &creds("csecret"), T).unwrap();
        let root = p.list("").await.unwrap();
        assert_eq!(root.entries[0].path, "backend");
        assert_eq!(root.keys, vec!["ROOT_KEY"]);
        assert!(matches!(
            p.read("nope").await,
            Err(ProviderError::NotFound(_))
        ));
    }

    #[test]
    fn requires_project_environment_and_identity() {
        let mut c = cfg("http://x");
        c.environment = None;
        assert!(InfisicalProvider::new(&c, &creds("s"), T).is_err());
        assert!(
            InfisicalProvider::new(&cfg("http://x"), &ConnectionCredentials::default(), T).is_err()
        );
        assert_eq!(folder(""), "/");
        assert_eq!(folder("/a/b/"), "/a/b");
    }
}
