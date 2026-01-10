// Email template module - for deployment notifications and alerts
// TODO: Implement email sending in Phase 2

/// Email template for deployment status notifications
pub struct DeploymentEmailTemplate {
    pub app_name: String,
    pub status: String,
    pub commit_sha: Option<String>,
    pub error_message: Option<String>,
}

impl DeploymentEmailTemplate {
    pub fn render(&self) -> String {
        let status_emoji = match self.status.as_str() {
            "running" => "✅",
            "failed" => "❌",
            "building" => "🔨",
            _ => "📦",
        };

        let commit_info = self
            .commit_sha
            .as_ref()
            .map(|sha| format!("\nCommit: {}", &sha[..7.min(sha.len())]))
            .unwrap_or_default();

        let error_info = self
            .error_message
            .as_ref()
            .map(|e| format!("\n\nError: {}", e))
            .unwrap_or_default();

        format!(
            "{} Deployment Update: {}\n\nStatus: {}{}{}",
            status_emoji, self.app_name, self.status, commit_info, error_info
        )
    }
}
