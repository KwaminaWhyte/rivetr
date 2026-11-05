//! `domain_certificates` rows: per-hostname certificate mode and issuance state.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use super::store::CertInfo;

/// How a hostname gets its certificate.
pub const MODE_AUTO: &str = "auto";
pub const MODE_DNS: &str = "dns";
pub const MODE_CUSTOM: &str = "custom";

#[derive(Debug, Clone, FromRow, Serialize, Default)]
pub struct DomainCertRow {
    pub domain: String,
    pub mode: String,
    pub dns_provider_id: Option<String>,
    #[serde(skip_serializing)]
    pub custom_cert_pem: Option<String>,
    #[serde(skip_serializing)]
    pub custom_key: Option<String>,
    pub status: String,
    pub issuer: Option<String>,
    pub names: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub last_error: Option<String>,
    pub failure_count: i64,
    pub last_attempt_at: Option<String>,
    pub next_attempt_at: Option<String>,
    pub expiry_notified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl DomainCertRow {
    pub fn next_attempt(&self) -> Option<DateTime<Utc>> {
        parse_ts(self.next_attempt_at.as_deref())
    }
}

pub fn parse_ts(s: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s?)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn ts(d: DateTime<Utc>) -> String {
    d.to_rfc3339()
}

pub async fn get(db: &SqlitePool, domain: &str) -> sqlx::Result<Option<DomainCertRow>> {
    sqlx::query_as("SELECT * FROM domain_certificates WHERE domain = ?")
        .bind(domain.to_ascii_lowercase())
        .fetch_optional(db)
        .await
}

pub async fn list(db: &SqlitePool) -> sqlx::Result<Vec<DomainCertRow>> {
    sqlx::query_as("SELECT * FROM domain_certificates ORDER BY domain")
        .fetch_all(db)
        .await
}

async fn ensure(db: &SqlitePool, domain: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT OR IGNORE INTO domain_certificates (domain) VALUES (?)")
        .bind(domain.to_ascii_lowercase())
        .execute(db)
        .await?;
    Ok(())
}

/// Switch a hostname between `auto` and `dns`. Clears backoff so the manager
/// retries right away with the new settings.
pub async fn set_mode(
    db: &SqlitePool,
    domain: &str,
    mode: &str,
    dns_provider_id: Option<&str>,
) -> sqlx::Result<()> {
    ensure(db, domain).await?;
    sqlx::query(
        "UPDATE domain_certificates SET mode = ?, dns_provider_id = ?, failure_count = 0, \
         next_attempt_at = NULL, last_error = NULL, updated_at = datetime('now') WHERE domain = ?",
    )
    .bind(mode)
    .bind(dns_provider_id)
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn record_issued(db: &SqlitePool, domain: &str, info: &CertInfo) -> sqlx::Result<()> {
    ensure(db, domain).await?;
    sqlx::query(
        "UPDATE domain_certificates SET status = 'issued', issuer = ?, names = ?, not_before = ?, \
         not_after = ?, last_error = NULL, failure_count = 0, last_attempt_at = ?, \
         next_attempt_at = NULL, updated_at = datetime('now') WHERE domain = ?",
    )
    .bind(&info.issuer)
    .bind(info.names.join(","))
    .bind(ts(info.not_before))
    .bind(ts(info.not_after))
    .bind(ts(Utc::now()))
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn record_failure(
    db: &SqlitePool,
    domain: &str,
    error: &str,
    failure_count: i64,
    next_attempt: DateTime<Utc>,
) -> sqlx::Result<()> {
    ensure(db, domain).await?;
    sqlx::query(
        "UPDATE domain_certificates SET status = 'failed', last_error = ?, failure_count = ?, \
         last_attempt_at = ?, next_attempt_at = ?, updated_at = datetime('now') WHERE domain = ?",
    )
    .bind(error)
    .bind(failure_count)
    .bind(ts(Utc::now()))
    .bind(ts(next_attempt))
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

/// Store an uploaded certificate. `key` is already encrypted when possible.
pub async fn set_custom(
    db: &SqlitePool,
    domain: &str,
    cert_pem: &str,
    key: &str,
    info: &CertInfo,
) -> sqlx::Result<()> {
    ensure(db, domain).await?;
    sqlx::query(
        "UPDATE domain_certificates SET mode = 'custom', custom_cert_pem = ?, custom_key = ?, \
         status = 'custom', issuer = ?, names = ?, not_before = ?, not_after = ?, \
         last_error = NULL, failure_count = 0, next_attempt_at = NULL, expiry_notified_at = NULL, \
         updated_at = datetime('now') WHERE domain = ?",
    )
    .bind(cert_pem)
    .bind(key)
    .bind(&info.issuer)
    .bind(info.names.join(","))
    .bind(ts(info.not_before))
    .bind(ts(info.not_after))
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

/// Drop an uploaded certificate and go back to automatic issuance.
pub async fn clear_custom(db: &SqlitePool, domain: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE domain_certificates SET mode = 'auto', custom_cert_pem = NULL, custom_key = NULL, \
         status = 'pending', issuer = NULL, names = NULL, not_before = NULL, not_after = NULL, \
         expiry_notified_at = NULL, updated_at = datetime('now') WHERE domain = ?",
    )
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn mark_expiry_notified(db: &SqlitePool, domain: &str) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE domain_certificates SET expiry_notified_at = ?, updated_at = datetime('now') \
         WHERE domain = ?",
    )
    .bind(ts(Utc::now()))
    .bind(domain.to_ascii_lowercase())
    .execute(db)
    .await?;
    Ok(())
}

/// Certificates issued in the last 7 days for names under `registered_domain`
/// (Let's Encrypt allows 50 per registered domain per week).
pub async fn issued_last_week_under(db: &SqlitePool, registered_domain: &str) -> sqlx::Result<i64> {
    let since = ts(Utc::now() - chrono::Duration::days(7));
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM domain_certificates WHERE mode != 'custom' AND not_before >= ? \
         AND (domain = ? OR domain LIKE ?)",
    )
    .bind(since)
    .bind(registered_domain)
    .bind(format!("%.{registered_domain}"))
    .fetch_one(db)
    .await
}
