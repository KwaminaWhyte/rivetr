//! Per-domain TLS certificates.
//!
//! Every routed hostname gets its own certificate, served through an SNI
//! resolver ([`store::CertStore`]). The [`manager::CertManager`] background
//! task orders, renews and retries certificates one domain at a time, so one
//! failing domain never blocks the others. Certificates are persisted under
//! `<acme_cache_dir>/domains/<domain>/` and their state in the
//! `domain_certificates` table (migration 132).
//!
//! Sources, in priority order: custom uploads, ACME per-domain certificates,
//! and the legacy shared SAN certificate (`<acme_cache_dir>/certs/<domain>/`)
//! which keeps serving its names until per-domain certificates replace it.

#[cfg(test)]
mod acme_tests;
pub mod db;
pub mod dns;
pub mod launch;
pub mod manager;
pub mod store;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tracing::{info, warn};

pub use manager::CertManager;
pub use store::{CertInfo, CertSource, CertStore, LoadedCert};

use db::{DomainCertRow, MODE_CUSTOM, MODE_DNS};

/// Renew this long before expiry (plus a per-domain jitter).
pub const RENEW_BEFORE_DAYS: i64 = 30;
/// Spread renewals over this many hours so they do not all fire at once.
pub const RENEW_JITTER_HOURS: i64 = 72;
/// Let's Encrypt allows 300 new orders per account per 3 hours; stay below.
pub const ORDERS_PER_WINDOW: usize = 250;
pub const ORDER_WINDOW_HOURS: i64 = 3;
/// Let's Encrypt allows 50 certificates per registered domain per week.
pub const WEEKLY_CERTS_PER_DOMAIN: i64 = 45;
/// `instance_settings` keys for the instance wildcard certificate.
pub const KEY_WILDCARD_DOMAIN: &str = "wildcard_cert_domain";
pub const KEY_WILDCARD_PROVIDER: &str = "wildcard_cert_dns_provider_id";
/// Warn about uploaded certificates this many days before they expire.
pub const CUSTOM_EXPIRY_WARNING_DAYS: i64 = 14;

/// Shared handles for the API and the background manager.
pub struct CertRuntime {
    pub store: Arc<CertStore>,
    pub cache_dir: PathBuf,
    pub acme_enabled: bool,
    encryption_key: Option<[u8; 32]>,
    wake: Notify,
}

impl CertRuntime {
    pub fn new(
        store: Arc<CertStore>,
        cache_dir: PathBuf,
        acme_enabled: bool,
        encryption_secret: Option<&str>,
    ) -> Self {
        Self {
            store,
            cache_dir,
            acme_enabled,
            encryption_key: encryption_secret.map(crate::crypto::derive_key),
            wake: Notify::new(),
        }
    }

    /// Ask the manager to run now (a domain or setting changed).
    pub fn wake(&self) {
        self.wake.notify_one();
    }

    pub(crate) async fn woken(&self) {
        self.wake.notified().await;
    }

    pub fn encryption_key(&self) -> Option<&[u8; 32]> {
        self.encryption_key.as_ref()
    }
}

static RUNTIME: OnceLock<Arc<CertRuntime>> = OnceLock::new();

/// Register the process-wide runtime (first call wins).
pub fn install(runtime: Arc<CertRuntime>) -> Arc<CertRuntime> {
    RUNTIME.get_or_init(|| runtime).clone()
}

/// The runtime, when the HTTPS listener is set up.
pub fn runtime() -> Option<Arc<CertRuntime>> {
    RUNTIME.get().cloned()
}

/// Build the store and load every certificate already on disk or in the DB.
pub async fn bootstrap(
    db: &SqlitePool,
    cache_dir: &Path,
    instance_domain: Option<String>,
    acme_enabled: bool,
    encryption_secret: Option<&str>,
) -> Result<Arc<CertRuntime>> {
    let store = Arc::new(CertStore::new()?);
    store.set_default_name(instance_domain);
    let runtime = CertRuntime::new(
        store.clone(),
        cache_dir.to_path_buf(),
        acme_enabled,
        encryption_secret,
    );

    for (id, cert) in load_dir_certs(&cache_dir.join("certs"), |n| n.to_string()).await {
        info!(id = %id, names = cert.info.names.len(), "Loaded legacy shared certificate");
        store.insert(CertSource::Legacy, &id, cert);
    }
    let domains = load_dir_certs(&cache_dir.join("domains"), id_from_dir).await;
    let count = domains.len();
    for (id, cert) in domains {
        store.insert(CertSource::Acme, &id, cert);
    }
    for (domain, cert) in load_custom(db, runtime.encryption_key()).await {
        store.insert(CertSource::Custom, &domain, cert);
    }
    info!(
        acme = count,
        custom = store.ids(CertSource::Custom).len(),
        "Certificate store ready"
    );
    Ok(install(Arc::new(runtime)))
}

// ---------------------------------------------------------------------------
// Disk layout
// ---------------------------------------------------------------------------

/// Directory name for a certificate id (`*` is not portable in paths).
pub fn dir_name(id: &str) -> String {
    id.replace('*', "_wildcard")
}

fn id_from_dir(name: &str) -> String {
    name.replace("_wildcard", "*")
}

pub fn domain_dir(cache_dir: &Path, id: &str) -> PathBuf {
    cache_dir.join("domains").join(dir_name(id))
}

/// Persist an issued certificate; the key file is written owner-only.
pub async fn save_domain_cert(
    cache_dir: &Path,
    id: &str,
    chain_pem: &str,
    key_pem: &str,
) -> Result<()> {
    let dir = domain_dir(cache_dir, id);
    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("Failed to create {}", dir.display()))?;
    write_atomic(&dir.join("privkey.pem"), key_pem, true).await?;
    write_atomic(&dir.join("fullchain.pem"), chain_pem, false).await?;
    Ok(())
}

async fn write_atomic(path: &Path, contents: &str, private: bool) -> Result<()> {
    let tmp = path.with_extension("pem.tmp");
    tokio::fs::write(&tmp, contents)
        .await
        .with_context(|| format!("Failed to write {}", tmp.display()))?;
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600)).await?;
    }
    #[cfg(not(unix))]
    let _ = private;
    tokio::fs::rename(&tmp, path)
        .await
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Load `<dir>/<name>/{fullchain,privkey}.pem` for every subdirectory.
async fn load_dir_certs(dir: &Path, id_of: fn(&str) -> String) -> Vec<(String, LoadedCert)> {
    let mut out = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let chain = tokio::fs::read_to_string(path.join("fullchain.pem")).await;
        let key = tokio::fs::read_to_string(path.join("privkey.pem")).await;
        match (chain, key) {
            (Ok(chain), Ok(key)) => match store::load_pem(&chain, &key) {
                Ok(cert) => out.push((id_of(&name), cert)),
                Err(e) => {
                    warn!(dir = %path.display(), error = %e, "Skipping unreadable certificate")
                }
            },
            _ => continue,
        }
    }
    out
}

/// Uploaded certificates from the DB, keys decrypted.
pub async fn load_custom(db: &SqlitePool, key: Option<&[u8; 32]>) -> Vec<(String, LoadedCert)> {
    let rows = db::list(db).await.unwrap_or_default();
    let mut out = Vec::new();
    for row in rows.into_iter().filter(|r| r.mode == MODE_CUSTOM) {
        let (Some(cert), Some(enc_key)) = (row.custom_cert_pem, row.custom_key) else {
            continue;
        };
        let loaded = crate::crypto::decrypt_if_encrypted(&enc_key, key)
            .and_then(|k| store::load_pem(&cert, &k));
        match loaded {
            Ok(c) => out.push((row.domain, c)),
            Err(e) => warn!(domain = %row.domain, error = %e, "Skipping custom certificate"),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------

/// Stable per-id offset in `[0, RENEW_JITTER_HOURS)` so renewals spread out.
pub fn jitter(id: &str) -> chrono::Duration {
    let digest = Sha256::digest(id.as_bytes());
    let n = u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]));
    chrono::Duration::seconds((n % (RENEW_JITTER_HOURS as u64 * 3600)) as i64)
}

/// When a certificate valid until `not_after` should be renewed.
pub fn renewal_due_at(not_after: DateTime<Utc>, id: &str) -> DateTime<Utc> {
    not_after - chrono::Duration::days(RENEW_BEFORE_DAYS) - jitter(id)
}

/// Wait after the n-th consecutive failure: 15 min doubling, capped at 24 h.
/// Keeps us under Let's Encrypt's 5 failed validations per host per hour.
pub fn backoff(failures: i64) -> chrono::Duration {
    let n = failures.clamp(1, 16) as u32;
    let minutes = 15i64.saturating_mul(1i64 << (n - 1));
    chrono::Duration::minutes(minutes.min(24 * 60))
}

/// Sliding-window limit on new ACME orders.
#[derive(Debug)]
pub struct RateBudget {
    max: usize,
    window: chrono::Duration,
    used: VecDeque<DateTime<Utc>>,
}

impl RateBudget {
    pub fn new(max: usize, window: chrono::Duration) -> Self {
        Self {
            max,
            window,
            used: VecDeque::new(),
        }
    }

    pub fn lets_encrypt() -> Self {
        Self::new(
            ORDERS_PER_WINDOW,
            chrono::Duration::hours(ORDER_WINDOW_HOURS),
        )
    }

    /// Take one order slot if available.
    pub fn try_take(&mut self, now: DateTime<Utc>) -> bool {
        while self.used.front().is_some_and(|t| now - *t >= self.window) {
            self.used.pop_front();
        }
        if self.used.len() >= self.max {
            return false;
        }
        self.used.push_back(now);
        true
    }
}

/// Best-effort registered domain (what Let's Encrypt counts per-week limits
/// against): the last two labels, or three under common two-level suffixes
/// such as `co.uk`.
pub fn registered_domain(host: &str) -> String {
    let host = host.trim_start_matches("*.").trim_end_matches('.');
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() <= 2 {
        return host.to_string();
    }
    let sld = labels[labels.len() - 2];
    let tld = labels[labels.len() - 1];
    let take =
        if tld.len() == 2 && matches!(sld, "co" | "com" | "net" | "org" | "gov" | "ac" | "edu") {
            3
        } else {
            2
        };
    labels[labels.len() - take..].join(".")
}

/// A name we can get a public certificate for.
pub fn is_certifiable(host: &str) -> bool {
    let h = host.trim_start_matches("*.");
    !h.is_empty()
        && h.contains('.')
        && h.parse::<std::net::IpAddr>().is_err()
        && !h.ends_with(".sslip.io")
        && !h.ends_with(".traefik.me")
        && !h.ends_with(".nip.io")
        && !h.ends_with(".localhost")
        && !h.ends_with(".local")
        && h.split('.').all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

// ---------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Challenge {
    Http01,
    Dns01 { provider_id: String },
}

/// One certificate to keep issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderPlan {
    /// Storage key (row and directory); the hostname as configured.
    pub id: String,
    /// Names on the certificate.
    pub names: Vec<String>,
    pub challenge: Challenge,
}

/// Instance-wide wildcard (`*.apps.example.com`) issued through a DNS provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WildcardSetting {
    pub domain: String,
    pub provider_id: String,
}

/// Decide which certificates should exist for the routed `domains`.
///
/// - `custom` rows are skipped (served from the upload);
/// - `dns` rows with a provider get `name` plus `*.name` through DNS-01;
/// - everything else gets its own HTTP-01 certificate, unless a DNS-01
///   wildcard (per-domain or the instance wildcard) already covers it.
pub fn plan_orders(
    domains: &[String],
    rows: &HashMap<String, DomainCertRow>,
    wildcard: Option<&WildcardSetting>,
) -> Vec<OrderPlan> {
    let mut seen = std::collections::HashSet::new();
    let domains: Vec<String> = domains
        .iter()
        .map(|d| d.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|d| is_certifiable(d) && seen.insert(d.clone()))
        .collect();

    let dns_provider = |d: &str| {
        rows.get(d)
            .filter(|r| r.mode == MODE_DNS)
            .and_then(|r| r.dns_provider_id.clone())
    };
    let dns_names = |d: &str| -> Vec<String> {
        match d.strip_prefix("*.") {
            Some(base) => vec![d.to_string(), base.to_string()],
            None => vec![d.to_string(), format!("*.{d}")],
        }
    };

    // Wildcards that DNS-01 orders will provide.
    let mut wildcards: Vec<String> = domains
        .iter()
        .filter(|d| dns_provider(d).is_some())
        .flat_map(|d| dns_names(d))
        .filter(|n| n.starts_with("*."))
        .collect();

    let mut plans = Vec::new();
    if let Some(w) = wildcard.filter(|w| is_certifiable(&w.domain)) {
        wildcards.push(w.domain.clone());
        plans.push(OrderPlan {
            id: w.domain.clone(),
            names: vec![w.domain.clone()],
            challenge: Challenge::Dns01 {
                provider_id: w.provider_id.clone(),
            },
        });
    }

    for d in &domains {
        if rows.get(d).is_some_and(|r| r.mode == MODE_CUSTOM) {
            continue;
        }
        if let Some(provider_id) = dns_provider(d) {
            plans.push(OrderPlan {
                id: d.clone(),
                names: dns_names(d),
                challenge: Challenge::Dns01 { provider_id },
            });
            continue;
        }
        // HTTP-01 cannot prove a wildcard, and covered names need nothing.
        if d.starts_with("*.") || wildcards.iter().any(|w| store::name_matches(w, d)) {
            continue;
        }
        plans.push(OrderPlan {
            id: d.clone(),
            names: vec![d.clone()],
            challenge: Challenge::Http01,
        });
    }
    plans
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(domain: &str, mode: &str, provider: Option<&str>) -> (String, DomainCertRow) {
        (
            domain.to_string(),
            DomainCertRow {
                domain: domain.to_string(),
                mode: mode.to_string(),
                dns_provider_id: provider.map(str::to_string),
                ..Default::default()
            },
        )
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn plans_one_http01_order_per_domain() {
        let plans = plan_orders(
            &s(&[
                "A.example.com",
                "b.example.com",
                "a.example.com",
                "x.sslip.io",
                "10.0.0.1",
            ]),
            &HashMap::new(),
            None,
        );
        assert_eq!(
            plans,
            vec![
                OrderPlan {
                    id: "a.example.com".into(),
                    names: s(&["a.example.com"]),
                    challenge: Challenge::Http01
                },
                OrderPlan {
                    id: "b.example.com".into(),
                    names: s(&["b.example.com"]),
                    challenge: Challenge::Http01
                },
            ]
        );
    }

    #[test]
    fn dns_mode_and_wildcards_cover_subdomains() {
        let rows: HashMap<_, _> = [
            row("example.com", MODE_DNS, Some("p1")),
            row("custom.org", MODE_CUSTOM, None),
            row("orphan.org", MODE_DNS, None),
        ]
        .into_iter()
        .collect();
        let wildcard = WildcardSetting {
            domain: "*.apps.host.net".into(),
            provider_id: "p2".into(),
        };
        let plans = plan_orders(
            &s(&[
                "example.com",
                "shop.example.com",
                "custom.org",
                "orphan.org",
                "x.apps.host.net",
                "*.wild.org",
            ]),
            &rows,
            Some(&wildcard),
        );
        let ids: Vec<_> = plans.iter().map(|p| p.id.as_str()).collect();
        // shop.example.com rides on *.example.com, x.apps.host.net on the
        // instance wildcard, custom.org is uploaded, *.wild.org needs DNS.
        assert_eq!(ids, vec!["*.apps.host.net", "example.com", "orphan.org"]);
        assert_eq!(plans[1].names, s(&["example.com", "*.example.com"]));
        assert_eq!(
            plans[1].challenge,
            Challenge::Dns01 {
                provider_id: "p1".into()
            }
        );
        // A dns row whose provider was deleted falls back to HTTP-01.
        assert_eq!(plans[2].challenge, Challenge::Http01);
    }

    #[test]
    fn renewal_is_jittered_and_stable() {
        let not_after = Utc::now() + chrono::Duration::days(90);
        let a = renewal_due_at(not_after, "a.example.com");
        assert_eq!(a, renewal_due_at(not_after, "a.example.com"));
        let earliest = not_after
            - chrono::Duration::days(RENEW_BEFORE_DAYS)
            - chrono::Duration::hours(RENEW_JITTER_HOURS);
        assert!(a > earliest && a <= not_after - chrono::Duration::days(RENEW_BEFORE_DAYS));
        assert_ne!(jitter("a.example.com"), jitter("b.example.com"));
    }

    #[test]
    fn backoff_doubles_and_caps() {
        assert_eq!(backoff(1), chrono::Duration::minutes(15));
        assert_eq!(backoff(2), chrono::Duration::minutes(30));
        assert_eq!(backoff(4), chrono::Duration::minutes(120));
        assert_eq!(backoff(30), chrono::Duration::hours(24));
        assert_eq!(backoff(0), chrono::Duration::minutes(15));
    }

    #[test]
    fn rate_budget_slides() {
        let mut b = RateBudget::new(2, chrono::Duration::hours(3));
        let t0 = Utc::now();
        assert!(b.try_take(t0));
        assert!(b.try_take(t0));
        assert!(!b.try_take(t0 + chrono::Duration::hours(1)));
        assert!(b.try_take(t0 + chrono::Duration::hours(3)));
    }

    #[test]
    fn registered_domain_handles_two_level_suffixes() {
        assert_eq!(registered_domain("a.b.example.com"), "example.com");
        assert_eq!(registered_domain("*.shop.example.co.uk"), "example.co.uk");
        assert_eq!(registered_domain("example.com"), "example.com");
    }

    #[test]
    fn dir_names_round_trip_wildcards() {
        assert_eq!(dir_name("*.example.com"), "_wildcard.example.com");
        assert_eq!(id_from_dir("_wildcard.example.com"), "*.example.com");
    }

    #[tokio::test]
    async fn saved_certs_load_back_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let (c, k) = store::tests::make_cert(&["*.example.com"], 30);
        save_domain_cert(dir.path(), "*.example.com", &c, &k)
            .await
            .unwrap();
        let loaded = load_dir_certs(&dir.path().join("domains"), id_from_dir).await;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, "*.example.com");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode =
                std::fs::metadata(domain_dir(dir.path(), "*.example.com").join("privkey.pem"))
                    .unwrap()
                    .permissions()
                    .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
