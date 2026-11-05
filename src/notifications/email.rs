//! System email service for sending invitation and notification emails.
//!
//! Transport precedence: the transactional email settings saved in the
//! dashboard (Settings > Email), then `rivetr.toml [email]` SMTP, then an
//! enabled Sendry notification channel.

use anyhow::Result;

use crate::config::EmailConfig;
use crate::db::{NotificationChannel, SendryConfig};
use crate::settings::email::EffectiveEmail;

/// Service for sending system emails.
pub struct SystemEmailService {
    config: EmailConfig,
    db: Option<sqlx::SqlitePool>,
    /// Derived key for decrypting secrets saved from the dashboard.
    encryption_key: Option<[u8; 32]>,
}

impl SystemEmailService {
    /// Create a new system email service (toml SMTP only; no DB lookups).
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            db: None,
            encryption_key: None,
        }
    }

    /// Create a service that also uses dashboard email settings and the Sendry
    /// fallback. Without an encryption key, encrypted dashboard secrets can not
    /// be read; prefer [`SystemEmailService::from_state`].
    pub fn with_db(config: EmailConfig, db: sqlx::SqlitePool) -> Self {
        Self {
            config,
            db: Some(db),
            encryption_key: None,
        }
    }

    /// Full service built from the app state (DB settings, toml, Sendry).
    pub fn from_state(state: &crate::AppState) -> Self {
        Self {
            config: state.config.email.clone(),
            db: Some(state.db.clone()),
            encryption_key: state
                .config
                .auth
                .encryption_key
                .as_deref()
                .map(crate::crypto::derive_key),
        }
    }

    /// Whether toml SMTP is configured.
    pub fn is_enabled(&self) -> bool {
        self.config.is_configured()
    }

    /// Whether email can be delivered by ANY route.
    pub async fn is_available(&self) -> bool {
        self.resolve().await.is_some() || self.find_sendry_channel().await.is_some()
    }

    /// Dashboard settings (if enabled) merged over toml, else toml alone.
    async fn resolve(&self) -> Option<EffectiveEmail> {
        match &self.db {
            Some(db) => {
                crate::settings::email::load_effective(
                    db,
                    &self.config,
                    self.encryption_key.as_ref(),
                )
                .await
            }
            None => crate::settings::email::from_toml(&self.config),
        }
    }

    /// Look up an enabled Sendry channel's config, preferring a global one.
    async fn find_sendry_channel(&self) -> Option<SendryConfig> {
        let db = self.db.as_ref()?;
        let channel: Option<NotificationChannel> = sqlx::query_as(
            "SELECT * FROM notification_channels \
             WHERE channel_type = 'sendry' AND enabled = 1 \
             ORDER BY (team_id IS NULL) DESC, created_at ASC LIMIT 1",
        )
        .fetch_optional(db)
        .await
        .ok()?;
        channel?.get_sendry_config()
    }

    /// Send a team invitation email
    pub async fn send_invitation_email(
        &self,
        to_email: &str,
        team_name: &str,
        role: &str,
        inviter_name: &str,
        accept_url: &str,
        expires_in_days: i64,
    ) -> Result<()> {
        let subject = format!("You've been invited to join {} on Rivetr", team_name);

        let html_body =
            render_invitation_html(team_name, role, inviter_name, accept_url, expires_in_days);

        let text_body =
            render_invitation_text(team_name, role, inviter_name, accept_url, expires_in_days);

        self.send_email(to_email, &subject, &html_body, &text_body)
            .await
    }

    /// Send a password-reset email
    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        user_name: &str,
        reset_url: &str,
        expires_in_minutes: i64,
    ) -> Result<()> {
        let subject = "Reset your Rivetr password".to_string();
        let html_body = render_password_reset_html(user_name, reset_url, expires_in_minutes);
        let text_body = render_password_reset_text(user_name, reset_url, expires_in_minutes);

        self.send_email(to_email, &subject, &html_body, &text_body)
            .await
    }

    /// Send an email through the first available transport. If none is
    /// configured, logs a warning and returns Ok so callers' flows aren't broken.
    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<()> {
        if let Some(cfg) = self.resolve().await {
            return crate::settings::email::send(&cfg, to_email, subject, html_body, text_body)
                .await;
        }

        if let Some(sendry) = self.find_sendry_channel().await {
            let client = reqwest::Client::new();
            super::sendry::send_sendry_raw(
                &client, &sendry, to_email, subject, html_body, text_body,
            )
            .await?;
            tracing::info!(to = %to_email, subject = %subject, "System email sent via Sendry channel");
            return Ok(());
        }

        tracing::warn!(
            to = %to_email,
            "No email transport configured (Settings > Email, [email] SMTP, or an enabled \
             Sendry notification channel); skipping email"
        );
        Ok(())
    }
}

/// Render the HTML version of the invitation email
fn render_invitation_html(
    team_name: &str,
    role: &str,
    inviter_name: &str,
    accept_url: &str,
    expires_in_days: i64,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Team Invitation</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
            -webkit-font-smoothing: antialiased;
        }}
        .container {{
            max-width: 560px;
            margin: 0 auto;
            padding: 40px 20px;
        }}
        .card {{
            background-color: #ffffff;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
            overflow: hidden;
        }}
        .header {{
            background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
            color: white;
            padding: 32px 24px;
            text-align: center;
        }}
        .header h1 {{
            margin: 0;
            font-size: 24px;
            font-weight: 600;
        }}
        .content {{
            padding: 32px 24px;
        }}
        .content p {{
            margin: 0 0 16px;
            color: #374151;
            line-height: 1.6;
        }}
        .highlight {{
            background-color: #f3f4f6;
            border-radius: 6px;
            padding: 16px;
            margin: 20px 0;
        }}
        .highlight-row {{
            display: flex;
            justify-content: space-between;
            padding: 8px 0;
            border-bottom: 1px solid #e5e7eb;
        }}
        .highlight-row:last-child {{
            border-bottom: none;
        }}
        .highlight-label {{
            color: #6b7280;
            font-size: 14px;
        }}
        .highlight-value {{
            color: #111827;
            font-weight: 500;
        }}
        .button-container {{
            text-align: center;
            margin: 32px 0;
        }}
        .button {{
            display: inline-block;
            background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
            color: white !important;
            text-decoration: none;
            padding: 14px 32px;
            border-radius: 6px;
            font-weight: 500;
            font-size: 16px;
        }}
        .button:hover {{
            background: linear-gradient(135deg, #2563eb 0%, #1d4ed8 100%);
        }}
        .note {{
            color: #6b7280;
            font-size: 13px;
            text-align: center;
            margin-top: 24px;
        }}
        .footer {{
            padding: 24px;
            text-align: center;
            color: #9ca3af;
            font-size: 12px;
            border-top: 1px solid #f3f4f6;
        }}
        .footer a {{
            color: #3b82f6;
            text-decoration: none;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="header">
                <h1>Team Invitation</h1>
            </div>
            <div class="content">
                <p>Hi there,</p>
                <p><strong>{inviter_name}</strong> has invited you to join <strong>{team_name}</strong> on Rivetr.</p>

                <div class="highlight">
                    <div class="highlight-row">
                        <span class="highlight-label">Team</span>
                        <span class="highlight-value">{team_name}</span>
                    </div>
                    <div class="highlight-row">
                        <span class="highlight-label">Role</span>
                        <span class="highlight-value">{role}</span>
                    </div>
                    <div class="highlight-row">
                        <span class="highlight-label">Invited by</span>
                        <span class="highlight-value">{inviter_name}</span>
                    </div>
                </div>

                <div class="button-container">
                    <a href="{accept_url}" class="button">Accept Invitation</a>
                </div>

                <p class="note">This invitation will expire in {expires_in_days} days. If you didn't expect this invitation, you can safely ignore this email.</p>
            </div>
            <div class="footer">
                <p>Sent by <a href="https://rivetr.io">Rivetr</a> - Deploy your apps with ease</p>
            </div>
        </div>
    </div>
</body>
</html>"#,
        inviter_name = html_escape(inviter_name),
        team_name = html_escape(team_name),
        role = html_escape(&capitalize_role(role)),
        accept_url = accept_url,
        expires_in_days = expires_in_days,
    )
}

/// Render the plain text version of the invitation email
fn render_invitation_text(
    team_name: &str,
    role: &str,
    inviter_name: &str,
    accept_url: &str,
    expires_in_days: i64,
) -> String {
    format!(
        r#"Team Invitation

Hi there,

{inviter_name} has invited you to join {team_name} on Rivetr.

Team: {team_name}
Role: {role}
Invited by: {inviter_name}

To accept this invitation, visit:
{accept_url}

This invitation will expire in {expires_in_days} days.

If you didn't expect this invitation, you can safely ignore this email.

---
Sent by Rivetr - Deploy your apps with ease
https://rivetr.io"#,
        inviter_name = inviter_name,
        team_name = team_name,
        role = capitalize_role(role),
        accept_url = accept_url,
        expires_in_days = expires_in_days,
    )
}

/// Render the HTML version of the password-reset email
fn render_password_reset_html(user_name: &str, reset_url: &str, expires_in_minutes: i64) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reset your password</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; margin: 0; padding: 0; background-color: #f5f5f5; -webkit-font-smoothing: antialiased; }}
        .container {{ max-width: 560px; margin: 0 auto; padding: 40px 20px; }}
        .card {{ background-color: #ffffff; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.06); overflow: hidden; }}
        .header {{ background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%); color: white; padding: 32px 24px; text-align: center; }}
        .header h1 {{ margin: 0; font-size: 24px; font-weight: 600; }}
        .content {{ padding: 32px 24px; }}
        .content p {{ margin: 0 0 16px; color: #374151; line-height: 1.6; }}
        .button-container {{ text-align: center; margin: 32px 0; }}
        .button {{ display: inline-block; background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%); color: white !important; text-decoration: none; padding: 14px 32px; border-radius: 6px; font-weight: 500; font-size: 16px; }}
        .link-fallback {{ word-break: break-all; color: #6b7280; font-size: 13px; }}
        .note {{ color: #6b7280; font-size: 13px; text-align: center; margin-top: 24px; }}
        .footer {{ padding: 24px; text-align: center; color: #9ca3af; font-size: 12px; border-top: 1px solid #f3f4f6; }}
        .footer a {{ color: #3b82f6; text-decoration: none; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="header">
                <h1>Reset your password</h1>
            </div>
            <div class="content">
                <p>Hi {user_name},</p>
                <p>We received a request to reset the password for your Rivetr account. Click the button below to choose a new password.</p>
                <div class="button-container">
                    <a href="{reset_url}" class="button">Reset Password</a>
                </div>
                <p class="link-fallback">Or paste this link into your browser:<br>{reset_url}</p>
                <p class="note">This link expires in {expires_in_minutes} minutes. If you didn't request a password reset, you can safely ignore this email — your password won't change.</p>
            </div>
            <div class="footer">
                <p>Sent by <a href="https://rivetr.io">Rivetr</a> - Deploy your apps with ease</p>
            </div>
        </div>
    </div>
</body>
</html>"#,
        user_name = html_escape(user_name),
        reset_url = html_escape(reset_url),
        expires_in_minutes = expires_in_minutes,
    )
}

/// Render the plain text version of the password-reset email
fn render_password_reset_text(user_name: &str, reset_url: &str, expires_in_minutes: i64) -> String {
    format!(
        r#"Reset your password

Hi {user_name},

We received a request to reset the password for your Rivetr account.
To choose a new password, visit:

{reset_url}

This link expires in {expires_in_minutes} minutes.

If you didn't request a password reset, you can safely ignore this email
— your password won't change.

---
Sent by Rivetr - Deploy your apps with ease
https://rivetr.io"#,
        user_name = user_name,
        reset_url = reset_url,
        expires_in_minutes = expires_in_minutes,
    )
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Capitalize role for display
fn capitalize_role(role: &str) -> String {
    let mut chars = role.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("Tom & Jerry"), "Tom &amp; Jerry");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_capitalize_role() {
        assert_eq!(capitalize_role("admin"), "Admin");
        assert_eq!(capitalize_role("developer"), "Developer");
        assert_eq!(capitalize_role("owner"), "Owner");
        assert_eq!(capitalize_role(""), "");
    }

    #[test]
    fn test_render_invitation_text() {
        let text = render_invitation_text(
            "My Team",
            "admin",
            "John Doe",
            "https://example.com/accept",
            7,
        );
        assert!(text.contains("John Doe"));
        assert!(text.contains("My Team"));
        assert!(text.contains("Admin"));
        assert!(text.contains("https://example.com/accept"));
        assert!(text.contains("7 days"));
    }

    #[test]
    fn test_render_invitation_html() {
        let html = render_invitation_html(
            "My Team",
            "admin",
            "John Doe",
            "https://example.com/accept",
            7,
        );
        assert!(html.contains("John Doe"));
        assert!(html.contains("My Team"));
        assert!(html.contains("Admin"));
        assert!(html.contains("https://example.com/accept"));
        assert!(html.contains("7 days"));
        assert!(html.contains("<!DOCTYPE html>"));
    }
}
