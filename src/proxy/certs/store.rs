//! In-memory certificate store and the SNI resolver the HTTPS listener uses.
//!
//! Certificates come from three sources, in increasing priority:
//! - `Legacy`: the old shared SAN certificate (`certs/<instance_domain>/`),
//!   served for its names until per-domain certificates replace it;
//! - `Acme`: per-domain certificates issued by Let's Encrypt (HTTP-01 or DNS-01);
//! - `Custom`: certificates uploaded by a user.
//!
//! Each source keeps its certificates by id (normally the domain). On every
//! change the layers are flattened into one `name -> key` map behind an
//! `ArcSwap`, so the TLS handshake path is a lock-free hash lookup.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use serde::Serialize;

use crate::proxy::tls::{alpn_protocols, load_certs_from_pem, load_private_key_from_pem};

/// Where a certificate came from. Later variants win over earlier ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CertSource {
    Legacy,
    Acme,
    Custom,
}

/// Facts read from a certificate's leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CertInfo {
    /// DNS names the certificate covers (SANs, or the CN when there are none).
    pub names: Vec<String>,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
}

/// A parsed certificate ready to serve.
#[derive(Clone)]
pub struct LoadedCert {
    pub key: Arc<CertifiedKey>,
    pub info: CertInfo,
}

impl std::fmt::Debug for LoadedCert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedCert")
            .field("info", &self.info)
            .finish()
    }
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Parse a PEM chain and private key into a servable certificate. Fails when
/// either does not parse or the key does not belong to the leaf certificate.
pub fn load_pem(cert_pem: &str, key_pem: &str) -> Result<LoadedCert> {
    let chain = load_certs_from_pem(cert_pem)?;
    let key = load_private_key_from_pem(key_pem)?;
    let info = cert_info(chain[0].as_ref())?;
    let certified = CertifiedKey::from_der(chain, key, &provider())
        .map_err(|e| anyhow::anyhow!("The private key does not match the certificate: {e}"))?;
    Ok(LoadedCert {
        key: Arc::new(certified),
        info,
    })
}

/// Read names, issuer and validity from a DER certificate.
pub fn cert_info(der: &[u8]) -> Result<CertInfo> {
    use x509_parser::prelude::*;
    let (_, cert) = X509Certificate::from_der(der).context("Could not parse the certificate")?;

    let mut names = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::DNSName(dns) = name {
                    names.push(dns.to_ascii_lowercase());
                }
            }
        }
    }
    if names.is_empty() {
        if let Some(cn) = cert
            .subject()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
        {
            names.push(cn.to_ascii_lowercase());
        }
    }
    let issuer = cert
        .issuer()
        .iter_organization()
        .chain(cert.issuer().iter_common_name())
        .find_map(|a| a.as_str().ok().map(str::to_string))
        .unwrap_or_default();
    let ts = |t: ASN1Time| {
        DateTime::<Utc>::from_timestamp(t.timestamp(), 0).unwrap_or(DateTime::<Utc>::MIN_UTC)
    };
    Ok(CertInfo {
        names,
        issuer,
        not_before: ts(cert.validity().not_before),
        not_after: ts(cert.validity().not_after),
    })
}

/// Does certificate name `pattern` (possibly `*.example.com`) cover `host`?
/// A wildcard matches exactly one label, as in RFC 6125.
pub fn name_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim_end_matches('.').to_ascii_lowercase();
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if let Some(suffix) = pattern.strip_prefix("*.") {
        match host.split_once('.') {
            Some((label, rest)) => !label.is_empty() && rest == suffix,
            None => false,
        }
    } else {
        pattern == host
    }
}

/// The wildcard name that would cover `host` (`a.b.com` -> `*.b.com`).
pub fn wildcard_for(host: &str) -> Option<String> {
    let (_, rest) = host.split_once('.')?;
    rest.contains('.').then(|| format!("*.{rest}"))
}

/// Why a custom certificate was rejected.
pub fn validate_custom(
    cert_pem: &str,
    key_pem: &str,
    domain: &str,
    now: DateTime<Utc>,
) -> std::result::Result<LoadedCert, String> {
    let loaded = load_pem(cert_pem, key_pem).map_err(|e| format!("{e:#}"))?;
    if loaded.info.not_after <= now {
        return Err(format!(
            "The certificate expired on {}",
            loaded.info.not_after.format("%Y-%m-%d")
        ));
    }
    if loaded.info.not_before > now {
        return Err(format!(
            "The certificate is not valid until {}",
            loaded.info.not_before.format("%Y-%m-%d")
        ));
    }
    if !loaded.info.names.iter().any(|n| name_matches(n, domain)) {
        return Err(format!(
            "The certificate does not cover {domain} (it covers: {})",
            loaded.info.names.join(", ")
        ));
    }
    Ok(loaded)
}

type NameMap = HashMap<String, Arc<CertifiedKey>>;

/// All certificates the HTTPS listener can serve.
pub struct CertStore {
    layers: Mutex<BTreeMap<CertSource, BTreeMap<String, LoadedCert>>>,
    /// Flattened `name -> key` map (exact names and `*.` wildcards).
    names: ArcSwap<NameMap>,
    /// Name served when SNI is missing or unknown (the instance domain).
    default_name: RwLock<Option<String>>,
    /// Self-signed certificate used when nothing else matches.
    fallback: Arc<CertifiedKey>,
}

impl std::fmt::Debug for CertStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertStore")
            .field("names", &self.names.load().len())
            .finish()
    }
}

impl CertStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            layers: Mutex::new(BTreeMap::new()),
            names: ArcSwap::from_pointee(HashMap::new()),
            default_name: RwLock::new(None),
            fallback: Arc::new(self_signed_fallback()?),
        })
    }

    /// Set the name served to clients without (or with unknown) SNI.
    pub fn set_default_name(&self, name: Option<String>) {
        *self.default_name.write() = name.map(|n| n.to_ascii_lowercase());
    }

    pub fn default_name(&self) -> Option<String> {
        self.default_name.read().clone()
    }

    /// Add or replace a certificate.
    pub fn insert(&self, source: CertSource, id: &str, cert: LoadedCert) {
        let mut layers = self.layers.lock();
        layers
            .entry(source)
            .or_default()
            .insert(id.to_ascii_lowercase(), cert);
        self.rebuild(&layers);
    }

    /// Remove a certificate; returns whether one was present.
    pub fn remove(&self, source: CertSource, id: &str) -> bool {
        let mut layers = self.layers.lock();
        let removed = layers
            .get_mut(&source)
            .and_then(|l| l.remove(&id.to_ascii_lowercase()))
            .is_some();
        if removed {
            self.rebuild(&layers);
        }
        removed
    }

    /// The certificate stored under `id` for `source`.
    pub fn get(&self, source: CertSource, id: &str) -> Option<LoadedCert> {
        self.layers
            .lock()
            .get(&source)
            .and_then(|l| l.get(&id.to_ascii_lowercase()))
            .cloned()
    }

    /// Ids stored for `source`.
    pub fn ids(&self, source: CertSource) -> Vec<String> {
        self.layers
            .lock()
            .get(&source)
            .map(|l| l.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The certificate that would be served for `host`, with its source.
    pub fn serving(&self, host: &str) -> Option<(CertSource, LoadedCert)> {
        let layers = self.layers.lock();
        let host = host.to_ascii_lowercase();
        let mut best: Option<(u8, CertSource, LoadedCert)> = None;
        for (source, certs) in layers.iter() {
            for cert in certs.values() {
                for name in &cert.info.names {
                    if !name_matches(name, &host) {
                        continue;
                    }
                    // Exact names beat wildcards; within that, later sources win.
                    let rank = if name.starts_with("*.") { 0 } else { 1 };
                    if best
                        .as_ref()
                        .is_none_or(|(r, s, _)| (rank, *source) >= (*r, *s))
                    {
                        best = Some((rank, *source, cert.clone()));
                    }
                }
            }
        }
        best.map(|(_, s, c)| (s, c))
    }

    fn rebuild(&self, layers: &BTreeMap<CertSource, BTreeMap<String, LoadedCert>>) {
        let mut map: NameMap = HashMap::new();
        let now = Utc::now();
        // BTreeMap iterates Legacy, Acme, Custom: later sources overwrite.
        for (source, certs) in layers.iter() {
            for cert in certs.values() {
                // An expired legacy SAN cert must not shadow a newer wildcard.
                if *source == CertSource::Legacy && cert.info.not_after <= now {
                    continue;
                }
                for name in &cert.info.names {
                    map.insert(name.clone(), cert.key.clone());
                }
            }
        }
        self.names.store(Arc::new(map));
    }

    /// Certificate for an SNI name: exact match, then a one-label wildcard.
    pub fn lookup(&self, host: &str) -> Option<Arc<CertifiedKey>> {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        let names = self.names.load();
        if let Some(k) = names.get(&host) {
            return Some(k.clone());
        }
        wildcard_for(&host).and_then(|w| names.get(&w).cloned())
    }

    /// Whether a real (non-fallback) certificate covers `host`. Used to decide
    /// whether redirecting HTTP to HTTPS is safe for that host.
    pub fn covers(&self, host: &str) -> bool {
        let host = host.split(':').next().unwrap_or(host);
        self.lookup(host).is_some()
    }

    fn resolve_name(&self, sni: Option<&str>) -> Arc<CertifiedKey> {
        sni.and_then(|n| self.lookup(n))
            .or_else(|| {
                self.default_name
                    .read()
                    .as_deref()
                    .and_then(|n| self.lookup(n))
            })
            .unwrap_or_else(|| self.fallback.clone())
    }

    /// rustls server config resolving certificates from this store, offering
    /// HTTP/2 and HTTP/1.1 via ALPN.
    pub fn server_config(self: &Arc<Self>) -> Result<ServerConfig> {
        let mut config = ServerConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()
            .context("Failed to select TLS protocol versions")?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(SniResolver(self.clone())));
        config.alpn_protocols = alpn_protocols();
        Ok(config)
    }
}

/// `ResolvesServerCert` adapter over the shared store.
#[derive(Debug)]
struct SniResolver(Arc<CertStore>);

impl ResolvesServerCert for SniResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(self.0.resolve_name(client_hello.server_name()))
    }
}

/// A throwaway self-signed certificate so the listener can complete handshakes
/// (and show a clear browser warning) for names without a certificate yet.
fn self_signed_fallback() -> Result<CertifiedKey> {
    let key = rcgen::KeyPair::generate().context("Failed to generate fallback key")?;
    let mut params = rcgen::CertificateParams::new(vec!["rivetr.invalid".to_string()])
        .context("Failed to build fallback certificate")?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Rivetr default certificate");
    let cert = params
        .self_signed(&key)
        .context("Failed to sign fallback certificate")?;
    let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(key.serialize_der().into());
    CertifiedKey::from_der(vec![cert.der().clone()], key_der, &provider())
        .map_err(|e| anyhow::anyhow!("Fallback certificate invalid: {e}"))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rcgen::{CertificateParams, KeyPair};

    pub(crate) fn make_cert(names: &[&str], days_valid: i64) -> (String, String) {
        let key = KeyPair::generate().unwrap();
        let mut params =
            CertificateParams::new(names.iter().map(|n| n.to_string()).collect::<Vec<_>>())
                .unwrap();
        let now = time::OffsetDateTime::now_utc();
        if days_valid >= 0 {
            params.not_before = now - time::Duration::days(1);
            params.not_after = now + time::Duration::days(days_valid);
        } else {
            params.not_before = now - time::Duration::days(90);
            params.not_after = now + time::Duration::days(days_valid);
        }
        let cert = params.self_signed(&key).unwrap();
        (
            pem::encode(&pem::Pem::new("CERTIFICATE", cert.der().to_vec())),
            pem::encode(&pem::Pem::new("PRIVATE KEY", key.serialize_der())),
        )
    }

    fn loaded(names: &[&str]) -> LoadedCert {
        let (c, k) = make_cert(names, 30);
        load_pem(&c, &k).unwrap()
    }

    fn same(a: &Arc<CertifiedKey>, b: &LoadedCert) -> bool {
        Arc::ptr_eq(a, &b.key)
    }

    #[test]
    fn wildcard_matches_exactly_one_label() {
        assert!(name_matches("*.example.com", "a.example.com"));
        assert!(name_matches("*.Example.com", "A.example.com."));
        assert!(!name_matches("*.example.com", "example.com"));
        assert!(!name_matches("*.example.com", "a.b.example.com"));
        assert!(name_matches("app.example.com", "APP.example.com"));
        assert!(!name_matches("app.example.com", "www.app.example.com"));
        assert_eq!(wildcard_for("a.b.com").as_deref(), Some("*.b.com"));
        assert_eq!(wildcard_for("b.com"), None);
    }

    #[test]
    fn sni_lookup_exact_then_wildcard_then_fallback() {
        let store = Arc::new(CertStore::new().unwrap());
        let exact = loaded(&["app.example.com"]);
        let wild = loaded(&["*.example.com", "example.com"]);
        store.insert(CertSource::Acme, "app.example.com", exact.clone());
        store.insert(CertSource::Acme, "*.example.com", wild.clone());

        assert!(same(&store.lookup("app.example.com").unwrap(), &exact));
        assert!(same(&store.lookup("other.example.com").unwrap(), &wild));
        assert!(same(&store.lookup("example.com").unwrap(), &wild));
        assert!(store.lookup("deep.a.example.com").is_none());
        assert!(store.lookup("unknown.org").is_none());

        // Unknown SNI and missing SNI get the self-signed fallback...
        let fb = store.resolve_name(Some("unknown.org"));
        assert!(Arc::ptr_eq(&fb, &store.fallback));
        assert!(Arc::ptr_eq(&store.resolve_name(None), &store.fallback));
        // ...or the default name's certificate once one is set.
        store.set_default_name(Some("app.example.com".into()));
        assert!(same(&store.resolve_name(None), &exact));
        assert!(store.covers("app.example.com:443"));
        assert!(!store.covers("unknown.org"));
    }

    #[test]
    fn custom_beats_acme_beats_legacy() {
        let store = CertStore::new().unwrap();
        let legacy = loaded(&["a.example.com", "b.example.com"]);
        let acme = loaded(&["a.example.com"]);
        let custom = loaded(&["a.example.com"]);
        store.insert(CertSource::Legacy, "example.com", legacy.clone());
        assert!(same(&store.lookup("a.example.com").unwrap(), &legacy));
        store.insert(CertSource::Acme, "a.example.com", acme.clone());
        assert!(same(&store.lookup("a.example.com").unwrap(), &acme));
        // The legacy SAN cert still serves names nothing else covers.
        assert!(same(&store.lookup("b.example.com").unwrap(), &legacy));
        store.insert(CertSource::Custom, "a.example.com", custom.clone());
        assert!(same(&store.lookup("a.example.com").unwrap(), &custom));
        assert_eq!(
            store.serving("a.example.com").map(|(s, _)| s),
            Some(CertSource::Custom)
        );
        assert!(store.remove(CertSource::Custom, "a.example.com"));
        assert!(same(&store.lookup("a.example.com").unwrap(), &acme));
    }

    #[test]
    fn validate_custom_rejects_bad_input() {
        let now = Utc::now();
        let (cert, key) = make_cert(&["*.example.com"], 30);
        assert!(validate_custom(&cert, &key, "app.example.com", now).is_ok());

        // Wrong host (wildcards cover one label only).
        let err = validate_custom(&cert, &key, "a.b.example.com", now).unwrap_err();
        assert!(err.contains("does not cover"), "{err}");

        // Key from another certificate.
        let (_, other_key) = make_cert(&["app.example.com"], 30);
        let err = validate_custom(&cert, &other_key, "app.example.com", now).unwrap_err();
        assert!(err.contains("does not match"), "{err}");

        // Expired.
        let (old, old_key) = make_cert(&["app.example.com"], -1);
        let err = validate_custom(&old, &old_key, "app.example.com", now).unwrap_err();
        assert!(err.contains("expired"), "{err}");

        // Garbage.
        assert!(validate_custom("nope", &key, "app.example.com", now).is_err());
        assert!(validate_custom(&cert, "nope", "app.example.com", now).is_err());
    }

    #[test]
    fn cert_info_reads_names_and_validity() {
        let (cert, key) = make_cert(&["App.Example.com", "www.example.com"], 10);
        let info = load_pem(&cert, &key).unwrap().info;
        assert_eq!(info.names, vec!["app.example.com", "www.example.com"]);
        let left = info.not_after - Utc::now();
        assert!(left.num_days() >= 9 && left.num_days() <= 10);
    }
}
