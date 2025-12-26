// TLS configuration for HTTPS proxy
//
// This module handles TLS certificate loading and configuration
// for the HTTPS reverse proxy.

use anyhow::{Context, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

/// TLS configuration for a domain
#[derive(Clone)]
pub struct TlsConfig {
    /// TLS acceptor for incoming connections
    pub acceptor: TlsAcceptor,
}

impl TlsConfig {
    /// Create TLS config from certificate and key files
    pub fn from_files(cert_path: &Path, key_path: &Path) -> Result<Self> {
        let certs = load_certs(cert_path)?;
        let key = load_private_key(key_path)?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to create TLS server config")?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        info!(
            cert = %cert_path.display(),
            key = %key_path.display(),
            "Loaded TLS certificate"
        );

        Ok(Self { acceptor })
    }

    /// Create TLS config from PEM strings (for dynamically loaded certs)
    pub fn from_pem(cert_pem: &str, key_pem: &str) -> Result<Self> {
        let certs = load_certs_from_pem(cert_pem)?;
        let key = load_private_key_from_pem(key_pem)?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to create TLS server config")?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        Ok(Self { acceptor })
    }
}

/// Load certificates from a PEM file
fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path).context("Failed to open certificate file")?;
    let mut reader = BufReader::new(file);

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse certificates")?;

    if certs.is_empty() {
        anyhow::bail!("No certificates found in file");
    }

    Ok(certs)
}

/// Load certificates from a PEM string
fn load_certs_from_pem(pem: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(pem.as_bytes());

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse certificates")?;

    if certs.is_empty() {
        anyhow::bail!("No certificates found in PEM data");
    }

    Ok(certs)
}

/// Load a private key from a PEM file
fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>> {
    let file = File::open(path).context("Failed to open private key file")?;
    let mut reader = BufReader::new(file);

    // Try to read various private key formats
    loop {
        match rustls_pemfile::read_one(&mut reader)? {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Some(_) => continue, // Skip other items
            None => break,
        }
    }

    anyhow::bail!("No private key found in file")
}

/// Load a private key from a PEM string
fn load_private_key_from_pem(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(pem.as_bytes());

    loop {
        match rustls_pemfile::read_one(&mut reader)? {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Some(_) => continue,
            None => break,
        }
    }

    anyhow::bail!("No private key found in PEM data")
}

/// Certificate storage for multiple domains
#[derive(Default)]
pub struct CertStore {
    /// Default certificate for unknown domains
    default_cert: Option<TlsConfig>,
    /// Per-domain certificates
    domain_certs: dashmap::DashMap<String, TlsConfig>,
}

impl CertStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default certificate for unknown domains
    pub fn set_default(&mut self, config: TlsConfig) {
        self.default_cert = Some(config);
    }

    /// Add a certificate for a specific domain
    pub fn add_domain(&self, domain: String, config: TlsConfig) {
        self.domain_certs.insert(domain, config);
    }

    /// Remove a certificate for a domain
    pub fn remove_domain(&self, domain: &str) {
        self.domain_certs.remove(domain);
    }

    /// Get the TLS config for a domain (returns default if not found)
    pub fn get(&self, domain: &str) -> Option<TlsConfig> {
        self.domain_certs
            .get(domain)
            .map(|c| c.clone())
            .or_else(|| self.default_cert.clone())
    }

    /// Check if there's any certificate configured
    pub fn has_any_cert(&self) -> bool {
        self.default_cert.is_some() || !self.domain_certs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_store_default() {
        let store = CertStore::new();
        assert!(!store.has_any_cert());
        assert!(store.get("example.com").is_none());
    }

    // Note: More tests would require actual certificate files
}
