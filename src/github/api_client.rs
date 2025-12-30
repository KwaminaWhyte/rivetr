//! GitHub API client for repository and webhook operations.
//!
//! This client uses installation access tokens to interact with GitHub's API
//! for repository listing, branch listing, commenting on issues/PRs, and webhook management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// GitHub API client for installation-based operations.
pub struct GitHubClient {
    access_token: String,
    client: reqwest::Client,
}

impl GitHubClient {
    /// Create a new GitHub client with an installation access token.
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            client: reqwest::Client::new(),
        }
    }

    /// Make an authenticated GET request to the GitHub API.
    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Rivetr")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .context("Failed to make GitHub API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error: {} - {}", status, body);
        }

        response.json().await.context("Failed to parse GitHub API response")
    }

    /// Make an authenticated POST request to the GitHub API.
    async fn post<B: Serialize, T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Rivetr")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()
            .await
            .context("Failed to make GitHub API request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error: {} - {}", status, body);
        }

        response.json().await.context("Failed to parse GitHub API response")
    }

    /// List repositories accessible to an installation.
    ///
    /// Returns repositories that the installation has access to.
    pub async fn list_repos(&self, per_page: u32, page: u32) -> Result<Vec<Repository>> {
        let url = format!(
            "https://api.github.com/installation/repositories?per_page={}&page={}",
            per_page, page
        );

        let response: ListReposResponse = self.get(&url).await?;
        Ok(response.repositories)
    }

    /// List all repositories (paginated, fetching all pages).
    pub async fn list_all_repos(&self) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let repos = self.list_repos(per_page, page).await?;
            if repos.is_empty() {
                break;
            }
            all_repos.extend(repos);
            page += 1;
        }

        Ok(all_repos)
    }

    /// List branches for a repository.
    ///
    /// # Arguments
    /// * `owner` - Repository owner (user or org)
    /// * `repo` - Repository name
    pub async fn list_branches(&self, owner: &str, repo: &str) -> Result<Vec<Branch>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/branches?per_page=100",
            owner, repo
        );

        self.get(&url).await
    }

    /// Post a comment on an issue or pull request.
    ///
    /// # Arguments
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `issue_number` - Issue or PR number
    /// * `body` - Comment body (Markdown supported)
    ///
    /// # Returns
    /// The comment ID
    pub async fn post_comment(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<u64> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/comments",
            owner, repo, issue_number
        );

        let request_body = CreateCommentRequest {
            body: body.to_string(),
        };

        let response: CommentResponse = self.post(&url, &request_body).await?;
        Ok(response.id)
    }

    /// Create a webhook for a repository.
    ///
    /// # Arguments
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `config` - Webhook configuration
    ///
    /// # Returns
    /// The webhook ID
    pub async fn create_webhook(
        &self,
        owner: &str,
        repo: &str,
        config: WebhookConfig,
    ) -> Result<u64> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/hooks",
            owner, repo
        );

        let request_body = CreateWebhookRequest {
            name: "web".to_string(),
            active: true,
            events: config.events,
            config: WebhookConfigPayload {
                url: config.url,
                content_type: "json".to_string(),
                secret: config.secret,
                insecure_ssl: "0".to_string(),
            },
        };

        let response: WebhookResponse = self.post(&url, &request_body).await?;
        Ok(response.id)
    }

    /// Get the authenticated app's information.
    pub async fn get_app_info(&self) -> Result<AppInfo> {
        let url = "https://api.github.com/app";
        self.get(url).await
    }

    /// List installations for the authenticated app.
    /// Note: This requires app JWT authentication, not installation token.
    pub async fn list_installations(&self) -> Result<Vec<Installation>> {
        let url = "https://api.github.com/app/installations";
        self.get(url).await
    }
}

// Response types

#[derive(Debug, Deserialize)]
struct ListReposResponse {
    #[allow(dead_code)]
    total_count: u64,
    repositories: Vec<Repository>,
}

/// A GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub private: bool,
    pub owner: RepositoryOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOwner {
    pub login: String,
    pub id: u64,
    #[serde(rename = "type")]
    pub owner_type: String,
}

/// A Git branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    #[serde(rename = "protected")]
    pub is_protected: bool,
    pub commit: BranchCommit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCommit {
    pub sha: String,
    pub url: String,
}

// Comment types

#[derive(Debug, Serialize)]
struct CreateCommentRequest {
    body: String,
}

#[derive(Debug, Deserialize)]
struct CommentResponse {
    id: u64,
}

// Webhook types

/// Configuration for creating a webhook.
pub struct WebhookConfig {
    /// The URL to send webhook payloads to
    pub url: String,
    /// Events to subscribe to (e.g., ["push", "pull_request"])
    pub events: Vec<String>,
    /// Secret for signing webhook payloads
    pub secret: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateWebhookRequest {
    name: String,
    active: bool,
    events: Vec<String>,
    config: WebhookConfigPayload,
}

#[derive(Debug, Serialize)]
struct WebhookConfigPayload {
    url: String,
    content_type: String,
    secret: Option<String>,
    insecure_ssl: String,
}

#[derive(Debug, Deserialize)]
struct WebhookResponse {
    id: u64,
}

// App types

/// GitHub App information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub owner: AppOwner,
    pub description: Option<String>,
    pub external_url: String,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppOwner {
    pub login: String,
    pub id: u64,
    #[serde(rename = "type")]
    pub owner_type: String,
}

/// A GitHub App installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub id: u64,
    pub account: InstallationAccount,
    pub repository_selection: String,
    pub access_tokens_url: String,
    pub repositories_url: String,
    pub html_url: String,
    pub app_id: u64,
    pub target_id: u64,
    pub target_type: String,
    pub permissions: serde_json::Value,
    pub events: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub suspended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationAccount {
    pub login: String,
    pub id: u64,
    #[serde(rename = "type")]
    pub account_type: String,
    pub avatar_url: Option<String>,
}
