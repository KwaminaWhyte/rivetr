//! HTTP API client for TUI — calls the Rivetr REST API.
//!
//! Uses `reqwest` async client with a dedicated single-threaded Tokio runtime
//! so the TUI event loop (which must be synchronous) can still make HTTP calls.

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::runtime::Runtime;

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct App {
    pub id: String,
    pub name: String,
    pub git_url: Option<String>,
    pub branch: Option<String>,
    pub status: Option<String>,
    pub domain: Option<String>,
    pub port: Option<i64>,
    pub environment: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub commit_sha: Option<String>,
    pub trigger: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub status: Option<String>,
    pub port: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogEntry {
    pub id: Option<i64>,
    pub message: String,
    pub timestamp: Option<String>,
    pub level: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeployResponse {
    pub id: String,
    pub status: String,
}

// ─── Client ──────────────────────────────────────────────────────────────────

/// Synchronous API client backed by a private async reqwest client + Tokio runtime.
pub struct ApiClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
    rt: Runtime,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        if !token.is_empty() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token)
                    .parse()
                    .context("Invalid token format")?,
            );
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to build HTTP client")?;

        // Single-threaded runtime dedicated to HTTP calls from the TUI loop.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to create Tokio runtime for TUI API client")?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client,
            rt,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn block<F, T>(&self, fut: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        self.rt.block_on(fut)
    }

    pub fn list_apps(&self) -> Result<Vec<App>> {
        self.block(async {
            let resp = self
                .client
                .get(self.url("/api/apps"))
                .send()
                .await
                .context("GET /api/apps failed")?;

            if !resp.status().is_success() {
                anyhow::bail!("GET /api/apps returned {}", resp.status());
            }

            let text = resp.text().await.context("reading /api/apps body")?;
            if let Ok(apps) = serde_json::from_str::<Vec<App>>(&text) {
                return Ok(apps);
            }
            #[derive(Deserialize)]
            struct Wrapper {
                apps: Vec<App>,
            }
            let w: Wrapper = serde_json::from_str(&text).context("parsing /api/apps")?;
            Ok(w.apps)
        })
    }

    pub fn list_deployments(&self, limit: u32) -> Result<Vec<Deployment>> {
        self.block(async {
            let url = self.url(&format!("/api/deployments?limit={}", limit));
            let resp = self
                .client
                .get(url)
                .send()
                .await
                .context("GET /api/deployments failed")?;

            if !resp.status().is_success() {
                anyhow::bail!("GET /api/deployments returned {}", resp.status());
            }

            let text = resp.text().await.context("reading /api/deployments body")?;
            if let Ok(deps) = serde_json::from_str::<Vec<Deployment>>(&text) {
                return Ok(deps);
            }
            #[derive(Deserialize)]
            struct Wrapper {
                deployments: Vec<Deployment>,
            }
            let w: Wrapper = serde_json::from_str(&text).context("parsing /api/deployments")?;
            Ok(w.deployments)
        })
    }

    pub fn list_servers(&self) -> Result<Vec<Server>> {
        self.block(async {
            let resp = self
                .client
                .get(self.url("/api/servers"))
                .send()
                .await
                .context("GET /api/servers failed")?;

            if !resp.status().is_success() {
                anyhow::bail!("GET /api/servers returned {}", resp.status());
            }

            let text = resp.text().await.context("reading /api/servers body")?;
            if let Ok(servers) = serde_json::from_str::<Vec<Server>>(&text) {
                return Ok(servers);
            }
            #[derive(Deserialize)]
            struct Wrapper {
                servers: Vec<Server>,
            }
            let w: Wrapper = serde_json::from_str(&text).context("parsing /api/servers")?;
            Ok(w.servers)
        })
    }

    pub fn deploy_app(&self, app_id: &str) -> Result<DeployResponse> {
        let app_id = app_id.to_string();
        self.block(async {
            let url = self.url(&format!("/api/apps/{}/deploy", app_id));
            let resp = self
                .client
                .post(url)
                .header(reqwest::header::CONTENT_LENGTH, 0)
                .send()
                .await
                .context("POST /api/apps/:id/deploy failed")?;

            if !resp.status().is_success() {
                anyhow::bail!("deploy returned {}", resp.status());
            }

            resp.json::<DeployResponse>()
                .await
                .context("parsing deploy response")
        })
    }

    pub fn stop_app(&self, app_id: &str) -> Result<()> {
        let app_id = app_id.to_string();
        self.block(async {
            let url = self.url(&format!("/api/apps/{}/stop", app_id));
            self.client
                .post(url)
                .header(reqwest::header::CONTENT_LENGTH, 0)
                .send()
                .await
                .context("POST /api/apps/:id/stop failed")?;
            Ok(())
        })
    }

    pub fn restart_app(&self, app_id: &str) -> Result<()> {
        let app_id = app_id.to_string();
        self.block(async {
            let url = self.url(&format!("/api/apps/{}/restart", app_id));
            self.client
                .post(url)
                .header(reqwest::header::CONTENT_LENGTH, 0)
                .send()
                .await
                .context("POST /api/apps/:id/restart failed")?;
            Ok(())
        })
    }

    pub fn fetch_logs(&self, app_id: &str, deployment_id: &str) -> Result<Vec<LogEntry>> {
        let app_id = app_id.to_string();
        let deployment_id = deployment_id.to_string();
        self.block(async {
            let url = self.url(&format!(
                "/api/apps/{}/deployments/{}/logs",
                app_id, deployment_id
            ));
            let resp = self
                .client
                .get(url)
                .send()
                .await
                .context("GET logs failed")?;

            if !resp.status().is_success() {
                anyhow::bail!("GET logs returned {}", resp.status());
            }

            let text = resp.text().await.context("reading logs body")?;
            if let Ok(entries) = serde_json::from_str::<Vec<LogEntry>>(&text) {
                return Ok(entries);
            }
            #[derive(Deserialize)]
            struct Wrapper {
                logs: Vec<LogEntry>,
            }
            let w: Wrapper = serde_json::from_str(&text).context("parsing logs")?;
            Ok(w.logs)
        })
    }

    /// Returns true if the server is reachable.
    pub fn ping(&self) -> bool {
        self.block(async {
            let result = self
                .client
                .get(self.url("/api/system/health"))
                .send()
                .await;
            Ok::<bool, anyhow::Error>(result.map(|r: reqwest::Response| r.status().is_success()).unwrap_or(false))
        })
        .unwrap_or(false)
    }

    pub fn token(&self) -> &str {
        &self.token
    }
}
