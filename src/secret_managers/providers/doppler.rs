//! Doppler: a service token scoped to one project config. The reference path is empty
//! (use the connection's project/config) or `project/config`.

use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::{
    base_url, http_client, json_object_to_map, non_empty, required, status_error, ConnectionConfig,
    ConnectionCredentials, Listing, ProviderError, SecretData, SecretProvider,
};

const DEFAULT_API: &str = "https://api.doppler.com";

pub struct DopplerProvider {
    client: reqwest::Client,
    api: String,
    token: String,
    project: Option<String>,
    config: Option<String>,
}

impl DopplerProvider {
    pub fn new(
        config: &ConnectionConfig,
        creds: &ConnectionCredentials,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        Ok(Self {
            client: http_client(timeout),
            api: base_url(non_empty(&config.api_url).unwrap_or(DEFAULT_API)),
            token: required(&creds.token, "Doppler service token")?.to_string(),
            project: non_empty(&config.project).map(str::to_string),
            config: non_empty(&config.config).map(str::to_string),
        })
    }

    /// Project and config for a path: `project/config` or the connection defaults.
    fn target(&self, path: &str) -> Result<(Option<String>, Option<String>), ProviderError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok((self.project.clone(), self.config.clone()));
        }
        match path.split_once('/') {
            Some((p, c)) if !p.is_empty() && !c.is_empty() && !c.contains('/') => {
                Ok((Some(p.to_string()), Some(c.to_string())))
            }
            _ => Err(ProviderError::Config(format!(
                "Doppler paths are empty or 'project/config', got '{}'",
                path
            ))),
        }
    }
}

#[async_trait]
impl SecretProvider for DopplerProvider {
    async fn read(&self, path: &str) -> Result<SecretData, ProviderError> {
        let (project, config) = self.target(path)?;
        let mut query: Vec<(&str, String)> = vec![("format", "json".to_string())];
        if let Some(p) = project {
            query.push(("project", p));
        }
        if let Some(c) = config {
            query.push(("config", c));
        }
        let resp = self
            .client
            .get(format!("{}/v3/configs/config/secrets/download", self.api))
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;
        if !resp.status().is_success() {
            return Err(status_error(resp, "Doppler secrets download").await);
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Invalid(e.without_url().to_string()))?;
        let obj = body
            .as_object()
            .ok_or_else(|| ProviderError::Invalid("expected a JSON object".into()))?;
        Ok(SecretData {
            values: json_object_to_map(obj),
            raw: None,
        })
    }

    async fn list(&self, path: &str) -> Result<Listing, ProviderError> {
        Ok(Listing {
            entries: vec![],
            keys: self.read(path).await?.values.into_keys().collect(),
        })
    }

    async fn test(&self) -> Result<String, ProviderError> {
        let n = self.read("").await?.values.len();
        Ok(format!("Authenticated; {} secret(s) readable", n))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_server::serve;
    use super::*;
    use axum::{
        extract::Query,
        http::{HeaderMap, StatusCode},
        routing::get,
        Json, Router,
    };
    use serde_json::json;
    use std::collections::HashMap;

    const T: Duration = Duration::from_secs(5);

    fn router() -> Router {
        Router::new().route(
            "/v3/configs/config/secrets/download",
            get(
                |h: HeaderMap, Query(q): Query<HashMap<String, String>>| async move {
                    if h.get("authorization").and_then(|v| v.to_str().ok())
                        != Some("Bearer dp.st.prd.abc")
                    {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"messages": ["Invalid Auth token"], "success": false})),
                        );
                    }
                    assert_eq!(q["format"], "json");
                    let cfg = q.get("config").cloned().unwrap_or_else(|| "prd".into());
                    if cfg == "missing" {
                        return (
                            StatusCode::NOT_FOUND,
                            Json(json!({"messages": ["Could not find requested config"]})),
                        );
                    }
                    (
                        StatusCode::OK,
                        Json(json!({"API_KEY": format!("key-{cfg}"), "DOPPLER_CONFIG": cfg})),
                    )
                },
            ),
        )
    }

    fn cfg(api: &str) -> ConnectionConfig {
        ConnectionConfig {
            api_url: Some(api.into()),
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
    async fn service_token_download() {
        let api = serve(router()).await;
        let p = DopplerProvider::new(&cfg(&api), &token("dp.st.prd.abc"), T).unwrap();
        assert_eq!(p.read("").await.unwrap().values["API_KEY"], "key-prd");
        assert_eq!(
            p.read("web/stg").await.unwrap().values["API_KEY"],
            "key-stg"
        );
        assert_eq!(
            p.list("").await.unwrap().keys,
            vec!["API_KEY", "DOPPLER_CONFIG"]
        );
        assert!(p.test().await.unwrap().contains("2 secret"));
    }

    #[tokio::test]
    async fn errors() {
        let api = serve(router()).await;
        let p = DopplerProvider::new(&cfg(&api), &token("bad"), T).unwrap();
        assert!(matches!(p.test().await, Err(ProviderError::Auth(_))));
        let p = DopplerProvider::new(&cfg(&api), &token("dp.st.prd.abc"), T).unwrap();
        assert!(matches!(
            p.read("web/missing").await,
            Err(ProviderError::NotFound(_))
        ));
        assert!(matches!(
            p.read("a/b/c").await,
            Err(ProviderError::Config(_))
        ));
    }
}
