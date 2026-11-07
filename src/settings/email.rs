//! Transactional (system) email settings: SMTP, Resend or Sendry.
//!
//! Settings saved from the dashboard override `rivetr.toml [email]`. When the
//! dashboard config is enabled, any SMTP field left empty there falls back to
//! the toml value, so a password kept only in the toml keeps working. Secrets
//! (SMTP password, Resend API key) are encrypted at rest when an encryption key
//! is configured.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::{authentication::Credentials, extension::ClientId},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::parse_bool;
use crate::config::EmailConfig;

const KEY_ENABLED: &str = "email_enabled";
const KEY_PROVIDER: &str = "email_provider";
const KEY_FROM_NAME: &str = "email_from_name";
const KEY_FROM_ADDRESS: &str = "email_from_address";
const KEY_SMTP_HOST: &str = "email_smtp_host";
const KEY_SMTP_PORT: &str = "email_smtp_port";
const KEY_SMTP_ENCRYPTION: &str = "email_smtp_encryption";
const KEY_SMTP_USERNAME: &str = "email_smtp_username";
const KEY_SMTP_PASSWORD: &str = "email_smtp_password";
const KEY_SMTP_TIMEOUT: &str = "email_smtp_timeout_secs";
const KEY_SMTP_EHLO: &str = "email_smtp_ehlo_domain";
const KEY_RESEND_API_KEY: &str = "email_resend_api_key";
const KEY_SENDRY_API_KEY: &str = "email_sendry_api_key";

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailProvider {
    Smtp,
    Resend,
    Sendry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    Starttls,
    /// Implicit TLS (SMTPS), usually port 465.
    Tls,
}

impl SmtpEncryption {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "starttls" => Some(Self::Starttls),
            "tls" | "ssl" => Some(Self::Tls),
            _ => None,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Starttls => "starttls",
            Self::Tls => "tls",
        }
    }
}

/// SMTP connection settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    pub encryption: SmtpEncryption,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_secs: u64,
    pub ehlo_domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EmailTransport {
    Smtp(SmtpSettings),
    Resend { api_key: String },
    Sendry { api_key: String },
}

/// Where the active configuration comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailSource {
    Database,
    Config,
}

/// The fully resolved config used to send mail.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveEmail {
    pub source: EmailSource,
    pub from_name: String,
    pub from_address: String,
    pub transport: EmailTransport,
}

/// What the dashboard sees (no secrets).
#[derive(Debug, Clone, Serialize)]
pub struct EmailSettingsView {
    pub enabled: bool,
    pub provider: EmailProvider,
    pub from_name: Option<String>,
    pub from_address: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_encryption: SmtpEncryption,
    pub smtp_username: Option<String>,
    pub smtp_password_set: bool,
    pub smtp_timeout_secs: u64,
    pub smtp_ehlo_domain: Option<String>,
    pub resend_api_key_set: bool,
    pub sendry_api_key_set: bool,
    /// Which config is active right now ("database", "config") or null.
    pub active_source: Option<EmailSource>,
    /// Whether rivetr.toml [email] is configured (used as the fallback).
    pub config_file_configured: bool,
}

/// Partial update from the dashboard. For secrets: `None` keeps the stored
/// value, `Some("")` clears it, anything else replaces it.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateEmailSettings {
    pub enabled: Option<bool>,
    pub provider: Option<EmailProvider>,
    pub from_name: Option<String>,
    pub from_address: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_encryption: Option<SmtpEncryption>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_timeout_secs: Option<u64>,
    pub smtp_ehlo_domain: Option<String>,
    pub resend_api_key: Option<String>,
    pub sendry_api_key: Option<String>,
}

fn non_empty(map: &HashMap<String, String>, key: &str) -> Option<String> {
    map.get(key)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn decrypt(value: Option<String>, key: Option<&[u8; 32]>) -> Option<String> {
    let v = value?;
    crate::crypto::decrypt_if_encrypted(&v, key)
        .map_err(|e| tracing::warn!(error = %e, "Failed to decrypt stored email secret"))
        .ok()
        .filter(|v| !v.is_empty())
}

/// toml `[email]` as an effective config, when it is configured.
pub fn from_toml(toml: &EmailConfig) -> Option<EffectiveEmail> {
    if !toml.is_configured() {
        return None;
    }
    Some(EffectiveEmail {
        source: EmailSource::Config,
        from_name: toml.from_name.clone(),
        from_address: toml.from_address.clone()?,
        transport: EmailTransport::Smtp(SmtpSettings {
            host: toml.smtp_host.clone()?,
            port: toml.smtp_port,
            encryption: toml_encryption(toml),
            username: toml.smtp_username.clone(),
            password: toml.smtp_password.clone(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            ehlo_domain: None,
        }),
    })
}

fn toml_encryption(toml: &EmailConfig) -> SmtpEncryption {
    match (toml.smtp_tls, toml.smtp_port) {
        (false, _) => SmtpEncryption::None,
        (true, 465) => SmtpEncryption::Tls,
        (true, _) => SmtpEncryption::Starttls,
    }
}

/// Resolve the config to send with. Precedence:
/// 1. dashboard settings when `email_enabled = true` (empty SMTP fields fall back to toml),
/// 2. `rivetr.toml [email]` when configured,
/// 3. nothing.
pub fn effective(
    map: &HashMap<String, String>,
    toml: &EmailConfig,
    key: Option<&[u8; 32]>,
) -> Option<EffectiveEmail> {
    let db_enabled = map.get(KEY_ENABLED).and_then(|v| parse_bool(v)) == Some(true);
    if !db_enabled {
        return from_toml(toml);
    }

    let from_address = non_empty(map, KEY_FROM_ADDRESS).or_else(|| toml.from_address.clone())?;
    let from_name = non_empty(map, KEY_FROM_NAME).unwrap_or_else(|| toml.from_name.clone());

    let transport = match provider_of(map) {
        EmailProvider::Resend => EmailTransport::Resend {
            api_key: decrypt(non_empty(map, KEY_RESEND_API_KEY), key)?,
        },
        EmailProvider::Sendry => EmailTransport::Sendry {
            api_key: decrypt(non_empty(map, KEY_SENDRY_API_KEY), key)?,
        },
        EmailProvider::Smtp => EmailTransport::Smtp(SmtpSettings {
            host: non_empty(map, KEY_SMTP_HOST).or_else(|| toml.smtp_host.clone())?,
            port: non_empty(map, KEY_SMTP_PORT)
                .and_then(|v| v.parse().ok())
                .unwrap_or(toml.smtp_port),
            encryption: non_empty(map, KEY_SMTP_ENCRYPTION)
                .and_then(|v| SmtpEncryption::parse(&v))
                .unwrap_or_else(|| toml_encryption(toml)),
            username: non_empty(map, KEY_SMTP_USERNAME).or_else(|| toml.smtp_username.clone()),
            password: decrypt(non_empty(map, KEY_SMTP_PASSWORD), key)
                .or_else(|| toml.smtp_password.clone()),
            timeout_secs: non_empty(map, KEY_SMTP_TIMEOUT)
                .and_then(|v| v.parse().ok())
                .filter(|v| *v > 0)
                .unwrap_or(DEFAULT_TIMEOUT_SECS),
            ehlo_domain: non_empty(map, KEY_SMTP_EHLO),
        }),
    };

    Some(EffectiveEmail {
        source: EmailSource::Database,
        from_name,
        from_address,
        transport,
    })
}

fn provider_of(map: &HashMap<String, String>) -> EmailProvider {
    match map.get(KEY_PROVIDER).map(String::as_str) {
        Some("resend") => EmailProvider::Resend,
        Some("sendry") => EmailProvider::Sendry,
        _ => EmailProvider::Smtp,
    }
}

/// Load the effective config from the DB + toml.
pub async fn load_effective(
    db: &SqlitePool,
    toml: &EmailConfig,
    key: Option<&[u8; 32]>,
) -> Option<EffectiveEmail> {
    let map = crate::db::load_setting_map(db).await.ok()?;
    effective(&map, toml, key)
}

/// Build the dashboard view.
pub fn view(
    map: &HashMap<String, String>,
    toml: &EmailConfig,
    key: Option<&[u8; 32]>,
) -> EmailSettingsView {
    let active = effective(map, toml, key).map(|e| e.source);
    EmailSettingsView {
        enabled: map.get(KEY_ENABLED).and_then(|v| parse_bool(v)) == Some(true),
        provider: provider_of(map),
        from_name: non_empty(map, KEY_FROM_NAME),
        from_address: non_empty(map, KEY_FROM_ADDRESS),
        smtp_host: non_empty(map, KEY_SMTP_HOST),
        smtp_port: non_empty(map, KEY_SMTP_PORT).and_then(|v| v.parse().ok()),
        smtp_encryption: non_empty(map, KEY_SMTP_ENCRYPTION)
            .and_then(|v| SmtpEncryption::parse(&v))
            .unwrap_or(SmtpEncryption::Starttls),
        smtp_username: non_empty(map, KEY_SMTP_USERNAME),
        smtp_password_set: non_empty(map, KEY_SMTP_PASSWORD).is_some(),
        smtp_timeout_secs: non_empty(map, KEY_SMTP_TIMEOUT)
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS),
        smtp_ehlo_domain: non_empty(map, KEY_SMTP_EHLO),
        resend_api_key_set: non_empty(map, KEY_RESEND_API_KEY).is_some(),
        sendry_api_key_set: non_empty(map, KEY_SENDRY_API_KEY).is_some(),
        active_source: active,
        config_file_configured: toml.is_configured(),
    }
}

/// Validate and persist an update.
pub async fn save(
    db: &SqlitePool,
    req: &UpdateEmailSettings,
    key: Option<&[u8; 32]>,
) -> Result<(), String> {
    if let Some(addr) = req.from_address.as_deref().map(str::trim) {
        if !addr.is_empty() && addr.parse::<lettre::Address>().is_err() {
            return Err(format!("'{addr}' is not a valid email address"));
        }
    }
    if let Some(t) = req.smtp_timeout_secs {
        if t == 0 || t > 300 {
            return Err("SMTP timeout must be between 1 and 300 seconds".into());
        }
    }

    let mut writes: Vec<(&str, Option<String>)> = Vec::new();
    let text = |v: &Option<String>| v.as_ref().map(|s| s.trim().to_string());
    if let Some(v) = req.enabled {
        writes.push((KEY_ENABLED, Some(v.to_string())));
    }
    if let Some(p) = req.provider {
        let p = match p {
            EmailProvider::Smtp => "smtp",
            EmailProvider::Resend => "resend",
            EmailProvider::Sendry => "sendry",
        };
        writes.push((KEY_PROVIDER, Some(p.to_string())));
    }
    for (k, v) in [
        (KEY_FROM_NAME, text(&req.from_name)),
        (KEY_FROM_ADDRESS, text(&req.from_address)),
        (KEY_SMTP_HOST, text(&req.smtp_host)),
        (KEY_SMTP_USERNAME, text(&req.smtp_username)),
        (KEY_SMTP_EHLO, text(&req.smtp_ehlo_domain)),
    ] {
        if let Some(v) = v {
            writes.push((k, Some(v)));
        }
    }
    if let Some(p) = req.smtp_port {
        writes.push((KEY_SMTP_PORT, Some(p.to_string())));
    }
    if let Some(e) = req.smtp_encryption {
        writes.push((KEY_SMTP_ENCRYPTION, Some(e.as_str().to_string())));
    }
    if let Some(t) = req.smtp_timeout_secs {
        writes.push((KEY_SMTP_TIMEOUT, Some(t.to_string())));
    }
    for (k, secret) in [
        (KEY_SMTP_PASSWORD, &req.smtp_password),
        (KEY_RESEND_API_KEY, &req.resend_api_key),
        (KEY_SENDRY_API_KEY, &req.sendry_api_key),
    ] {
        if let Some(secret) = secret {
            let stored = if secret.is_empty() {
                None
            } else {
                Some(
                    crate::crypto::encrypt_if_key_available(secret, key)
                        .map_err(|e| format!("Failed to encrypt secret: {e}"))?,
                )
            };
            writes.push((k, stored));
        }
    }

    for (k, v) in writes {
        crate::db::set_setting_value(db, k, v.as_deref())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Send a message through the given config.
pub async fn send(
    cfg: &EffectiveEmail,
    to: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> Result<()> {
    match &cfg.transport {
        EmailTransport::Smtp(smtp) => send_smtp(cfg, smtp, to, subject, html_body, text_body).await,
        EmailTransport::Resend { api_key } => {
            send_resend(cfg, api_key, to, subject, html_body, text_body).await
        }
        EmailTransport::Sendry { api_key } => {
            send_sendry(cfg, api_key, to, subject, html_body, text_body).await
        }
    }
}

async fn send_smtp(
    cfg: &EffectiveEmail,
    smtp: &SmtpSettings,
    to: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> Result<()> {
    let from: Mailbox = format!("{} <{}>", cfg.from_name, cfg.from_address)
        .parse()
        .context("Invalid from address")?;
    let to: Mailbox = to.parse().context("Invalid recipient address")?;

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

    let mut builder = match smtp.encryption {
        SmtpEncryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)?,
        SmtpEncryption::Starttls => {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)?
        }
        SmtpEncryption::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host),
    }
    .port(smtp.port)
    .timeout(Some(Duration::from_secs(smtp.timeout_secs)));

    if let Some(domain) = &smtp.ehlo_domain {
        builder = builder.hello_name(ClientId::Domain(domain.clone()));
    }
    if let (Some(user), Some(pass)) = (&smtp.username, &smtp.password) {
        builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
    }

    builder.build().send(email).await?;
    tracing::info!(subject = %subject, "Email sent via SMTP");
    Ok(())
}

async fn send_resend(
    cfg: &EffectiveEmail,
    api_key: &str,
    to: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let res = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": format!("{} <{}>", cfg.from_name, cfg.from_address),
            "to": [to],
            "subject": subject,
            "html": html_body,
            "text": text_body,
        }))
        .send()
        .await
        .context("Failed to reach Resend")?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(anyhow!("Resend returned {status}: {body}"));
    }
    tracing::info!(subject = %subject, "Email sent via Resend");
    Ok(())
}

/// Sendry exposes the same request shape as Resend at its own host.
async fn send_sendry(
    cfg: &EffectiveEmail,
    api_key: &str,
    to: &str,
    subject: &str,
    html_body: &str,
    text_body: &str,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let res = client
        .post("https://api.sendry.online/v1/emails")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": format!("{} <{}>", cfg.from_name, cfg.from_address),
            "to": [to],
            "subject": subject,
            "html": html_body,
            "text": text_body,
        }))
        .send()
        .await
        .context("Failed to reach Sendry")?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(anyhow!("Sendry returned {status}: {body}"));
    }
    tracing::info!(subject = %subject, "Email sent via Sendry");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn toml() -> EmailConfig {
        EmailConfig {
            enabled: true,
            smtp_host: Some("smtp.toml.test".into()),
            smtp_port: 587,
            smtp_username: Some("toml-user".into()),
            smtp_password: Some("toml-pass".into()),
            smtp_tls: true,
            from_address: Some("toml@example.com".into()),
            from_name: "Toml".into(),
        }
    }

    #[test]
    fn toml_used_when_dashboard_config_not_enabled() {
        let e = effective(&HashMap::new(), &toml(), None).unwrap();
        assert_eq!(e.source, EmailSource::Config);
        assert_eq!(e.from_address, "toml@example.com");

        let e = effective(
            &map(&[("email_enabled", "false"), ("email_smtp_host", "db.test")]),
            &toml(),
            None,
        )
        .unwrap();
        assert_eq!(e.source, EmailSource::Config);
    }

    #[test]
    fn nothing_configured_means_none() {
        assert!(effective(&HashMap::new(), &EmailConfig::default(), None).is_none());
    }

    #[test]
    fn dashboard_overrides_toml_field_by_field() {
        let e = effective(
            &map(&[
                ("email_enabled", "true"),
                ("email_smtp_host", "smtp.db.test"),
                ("email_smtp_port", "465"),
                ("email_smtp_encryption", "tls"),
                ("email_from_name", "Db"),
            ]),
            &toml(),
            None,
        )
        .unwrap();
        assert_eq!(e.source, EmailSource::Database);
        assert_eq!(e.from_name, "Db");
        // Not set in the dashboard: falls back to toml.
        assert_eq!(e.from_address, "toml@example.com");
        match e.transport {
            EmailTransport::Smtp(s) => {
                assert_eq!(s.host, "smtp.db.test");
                assert_eq!(s.port, 465);
                assert_eq!(s.encryption, SmtpEncryption::Tls);
                assert_eq!(s.password.as_deref(), Some("toml-pass"));
            }
            _ => panic!("expected SMTP"),
        }
    }

    #[test]
    fn resend_needs_an_api_key() {
        let base = [
            ("email_enabled", "true"),
            ("email_provider", "resend"),
            ("email_from_address", "a@example.com"),
        ];
        assert!(effective(&map(&base), &EmailConfig::default(), None).is_none());
        let mut with_key = base.to_vec();
        with_key.push(("email_resend_api_key", "re_123"));
        let e = effective(&map(&with_key), &EmailConfig::default(), None).unwrap();
        assert_eq!(
            e.transport,
            EmailTransport::Resend {
                api_key: "re_123".into()
            }
        );
    }

    #[test]
    fn sendry_needs_an_api_key() {
        let base = [
            ("email_enabled", "true"),
            ("email_provider", "sendry"),
            ("email_from_address", "a@example.com"),
        ];
        assert!(effective(&map(&base), &EmailConfig::default(), None).is_none());
        let mut with_key = base.to_vec();
        with_key.push(("email_sendry_api_key", "sn_live_123"));
        let e = effective(&map(&with_key), &EmailConfig::default(), None).unwrap();
        assert_eq!(
            e.transport,
            EmailTransport::Sendry {
                api_key: "sn_live_123".into()
            }
        );
    }

    #[test]
    fn encrypted_secrets_are_decrypted() {
        let key = crate::crypto::derive_key("test-secret");
        let enc = crate::crypto::encrypt("s3cret", &key).unwrap();
        let e = effective(
            &map(&[
                ("email_enabled", "true"),
                ("email_smtp_host", "h"),
                ("email_from_address", "a@example.com"),
                ("email_smtp_password", &enc),
            ]),
            &EmailConfig::default(),
            Some(&key),
        )
        .unwrap();
        match e.transport {
            EmailTransport::Smtp(s) => assert_eq!(s.password.as_deref(), Some("s3cret")),
            _ => panic!("expected SMTP"),
        }
    }

    #[test]
    fn toml_tls_maps_to_starttls_except_port_465() {
        let mut t = toml();
        assert_eq!(toml_encryption(&t), SmtpEncryption::Starttls);
        t.smtp_port = 465;
        assert_eq!(toml_encryption(&t), SmtpEncryption::Tls);
        t.smtp_tls = false;
        assert_eq!(toml_encryption(&t), SmtpEncryption::None);
    }
}
