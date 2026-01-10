//! System email service for sending invitation and notification emails.
//!
//! This module provides a service for sending system emails like team invitations,
//! using the SMTP configuration from the main config file.

use anyhow::Result;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::config::EmailConfig;

/// Service for sending system emails
pub struct SystemEmailService {
    config: EmailConfig,
}

impl SystemEmailService {
    /// Create a new system email service
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    /// Check if email sending is configured and enabled
    pub fn is_enabled(&self) -> bool {
        self.config.is_configured()
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
        if !self.is_enabled() {
            tracing::warn!(
                "Email not configured, skipping invitation email to {}",
                to_email
            );
            return Ok(());
        }

        let subject = format!("You've been invited to join {} on Rivetr", team_name);

        let html_body =
            render_invitation_html(team_name, role, inviter_name, accept_url, expires_in_days);

        let text_body =
            render_invitation_text(team_name, role, inviter_name, accept_url, expires_in_days);

        self.send_email(to_email, &subject, &html_body, &text_body)
            .await
    }

    /// Send an email with HTML and plain text versions
    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<()> {
        let smtp_host = self
            .config
            .smtp_host
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP host not configured"))?;
        let from_address = self
            .config
            .from_address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("From address not configured"))?;

        // Build the from mailbox with name
        let from_mailbox = format!("{} <{}>", self.config.from_name, from_address);
        let from: Mailbox = from_mailbox.parse()?;
        let to: Mailbox = to_email.parse()?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text_body.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )?;

        // Build SMTP transport
        let mailer = if self.config.smtp_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
        }
        .port(self.config.smtp_port);

        let mailer = if let (Some(username), Some(password)) =
            (&self.config.smtp_username, &self.config.smtp_password)
        {
            mailer.credentials(Credentials::new(username.clone(), password.clone()))
        } else {
            mailer
        };

        mailer.build().send(email).await?;

        tracing::info!(
            to = %to_email,
            subject = %subject,
            "Email sent successfully"
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
