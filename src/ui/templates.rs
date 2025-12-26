// Askama template definitions

use askama::Template;
use sqlx::FromRow;

use crate::db::{App, Deployment};

/// Custom filters for Askama templates
mod filters {
    pub fn truncate(s: &str, len: usize) -> ::askama::Result<String> {
        if s.len() <= len {
            Ok(s.to_string())
        } else {
            Ok(format!("{}...", &s[..len]))
        }
    }
}

// Dashboard stats
pub struct DashboardStats {
    pub total_apps: u32,
    pub running_apps: u32,
    pub total_deployments: u32,
    pub failed_deployments: u32,
}

// Recent deployment for dashboard
#[derive(Debug, FromRow)]
pub struct RecentDeployment {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl RecentDeployment {
    pub fn duration(&self) -> String {
        if let Some(ref finished) = self.finished_at {
            if let (Ok(start), Ok(end)) = (
                chrono::DateTime::parse_from_rfc3339(&self.started_at),
                chrono::DateTime::parse_from_rfc3339(finished),
            ) {
                let duration = end.signed_duration_since(start);
                let secs = duration.num_seconds();
                if secs < 60 {
                    return format!("{}s", secs);
                } else {
                    return format!("{}m {}s", secs / 60, secs % 60);
                }
            }
        }
        "-".to_string()
    }
}

// App with status info for lists (using String instead of Option for templates)
pub struct AppWithStatus {
    pub id: String,
    pub name: String,
    pub git_url: String,
    pub domain: String, // Empty string if no domain
    pub status: String,
    pub last_deploy: String, // "Never" if no deploys
}

// Login template
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
    pub version: String,
}

// Dashboard template
#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub stats: DashboardStats,
    pub recent_deployments: Vec<RecentDeployment>,
    pub apps: Vec<AppWithStatus>,
    pub token: String,
}

// Apps list template
#[derive(Template)]
#[template(path = "apps.html")]
pub struct AppsTemplate {
    pub apps: Vec<AppWithStatus>,
    pub token: String,
}

// New app form template
#[derive(Template)]
#[template(path = "app_new.html")]
pub struct AppNewTemplate {
    pub error: Option<String>,
}

// App detail template
#[derive(Template)]
#[template(path = "app_detail.html")]
pub struct AppDetailTemplate {
    pub app: App,
    pub status: String,
    pub deployments: Vec<Deployment>,
    pub base_url: String,
    pub token: String,
}
