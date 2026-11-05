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

        let config = single_cert_config(certs, key)?;

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

        let config = single_cert_config(certs, key)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));

        Ok(Self { acceptor })
    }
}

/// ALPN protocols the HTTPS proxy offers, in preference order.
pub fn alpn_protocols() -> Vec<Vec<u8>> {
    vec![b"h2".to_vec(), b"http/1.1".to_vec()]
}

/// Server config for one certificate, offering HTTP/2 and HTTP/1.1 via ALPN.
fn single_cert_config(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig> {
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .context("Failed to select TLS protocol versions")?
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .context("Failed to create TLS server config")?;
    config.alpn_protocols = alpn_protocols();
    Ok(config)
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
pub fn load_certs_from_pem(pem: &str) -> Result<Vec<CertificateDer<'static>>> {
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
pub fn load_private_key_from_pem(pem: &str) -> Result<PrivateKeyDer<'static>> {
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
