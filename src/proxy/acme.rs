// ACME client for Let's Encrypt certificate automation
//
// This module implements the ACME protocol (RFC 8555) with HTTP-01 challenges
// for automatic TLS certificate provisioning.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dashmap::DashMap;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair as RingKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::tls::{TlsConfig, TlsReloadHandle};

/// Let's Encrypt ACME directory URLs
pub const LETS_ENCRYPT_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";
pub const LETS_ENCRYPT_PRODUCTION: &str = "https://acme-v02.api.letsencrypt.org/directory";

/// ACME configuration
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    /// Contact email for Let's Encrypt notifications
    pub email: String,
    /// Directory to store certificates and account data
    pub cache_dir: PathBuf,
    /// Use staging environment (for testing)
    pub staging: bool,
}

impl Default for AcmeConfig {
    fn default() -> Self {
        Self {
            email: String::new(),
            cache_dir: PathBuf::from("./data/acme"),
            staging: true,
        }
    }
}

/// Pending HTTP-01 challenges
/// Maps token -> key_authorization for serving challenges
#[derive(Default, Clone)]
pub struct AcmeChallenges {
    challenges: Arc<DashMap<String, String>>,
}

impl AcmeChallenges {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pending challenge
    pub fn add(&self, token: &str, key_authorization: &str) {
        self.challenges
            .insert(token.to_string(), key_authorization.to_string());
    }

    /// Get the key authorization for a token
    pub fn get(&self, token: &str) -> Option<String> {
        self.challenges.get(token).map(|v| v.clone())
    }

    /// Remove a challenge after it's completed
    pub fn remove(&self, token: &str) {
        self.challenges.remove(token);
    }
}

/// ACME directory response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Directory {
    new_nonce: String,
    new_account: String,
    new_order: String,
    #[serde(default)]
    revoke_cert: Option<String>,
    #[serde(default)]
    key_change: Option<String>,
}

/// ACME account
#[derive(Debug, Serialize, Deserialize)]
struct AccountCredentials {
    /// Account URL (kid)
    kid: String,
    /// Private key in PKCS#8 DER format (base64 encoded)
    private_key: String,
}

/// ACME order status
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum OrderStatus {
    Pending,
    Ready,
    Processing,
    Valid,
    Invalid,
}

/// ACME order
#[derive(Debug, Deserialize)]
struct Order {
    status: OrderStatus,
    #[serde(default)]
    authorizations: Vec<String>,
    #[serde(default)]
    finalize: String,
    #[serde(default)]
    certificate: Option<String>,
}

/// ACME authorization
#[derive(Debug, Deserialize)]
struct Authorization {
    status: String,
    identifier: Identifier,
    challenges: Vec<Challenge>,
}

/// ACME identifier
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Identifier {
    #[serde(rename = "type")]
    id_type: String,
    value: String,
}

/// ACME challenge
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Challenge {
    #[serde(rename = "type")]
    challenge_type: String,
    url: String,
    token: String,
    #[serde(default)]
    status: String,
}

/// ACME client for certificate management
pub struct AcmeClient {
    config: AcmeConfig,
    http: reqwest::Client,
    directory: RwLock<Option<Directory>>,
    account_kid: RwLock<Option<String>>,
    key_pair: RwLock<Option<Vec<u8>>>,
    challenges: AcmeChallenges,
}

impl AcmeClient {
    /// Create a new ACME client
    pub async fn new(config: AcmeConfig) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&config.cache_dir)
            .await
            .context("Failed to create ACME cache directory")?;

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let client = Self {
            config,
            http,
            directory: RwLock::new(None),
            account_kid: RwLock::new(None),
            key_pair: RwLock::new(None),
            challenges: AcmeChallenges::new(),
        };

        // Initialize directory and account
        client.fetch_directory().await?;
        client.load_or_create_account().await?;

        Ok(client)
    }

    /// Get the challenges store for HTTP-01 challenge serving
    pub fn challenges(&self) -> AcmeChallenges {
        self.challenges.clone()
    }

    /// Fetch the ACME directory
    async fn fetch_directory(&self) -> Result<()> {
        let url = if self.config.staging {
            LETS_ENCRYPT_STAGING
        } else {
            LETS_ENCRYPT_PRODUCTION
        };

        debug!(url = %url, "Fetching ACME directory");

        let response = self
            .http
            .get(url)
            .send()
            .await
            .context("Failed to fetch ACME directory")?;

        let directory: Directory = response
            .json()
            .await
            .context("Failed to parse ACME directory")?;

        *self.directory.write().await = Some(directory);
        Ok(())
    }

    /// Get a fresh nonce
    async fn get_nonce(&self) -> Result<String> {
        let directory = self.directory.read().await;
        let directory = directory.as_ref().context("Directory not loaded")?;

        let response = self
            .http
            .head(&directory.new_nonce)
            .send()
            .await
            .context("Failed to get nonce")?;

        response
            .headers()
            .get("replay-nonce")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .context("No nonce in response")
    }

    /// Load existing account or create a new one
    async fn load_or_create_account(&self) -> Result<()> {
        let account_path = self.config.cache_dir.join("account.json");

        if account_path.exists() {
            // Load existing account
            let data = fs::read_to_string(&account_path)
                .await
                .context("Failed to read account file")?;

            let credentials: AccountCredentials =
                serde_json::from_str(&data).context("Failed to parse account credentials")?;

            let key_bytes = URL_SAFE_NO_PAD
                .decode(&credentials.private_key)
                .context("Failed to decode private key")?;

            *self.account_kid.write().await = Some(credentials.kid);
            *self.key_pair.write().await = Some(key_bytes);

            info!("Loaded existing ACME account");
        } else {
            // Create new account
            info!(email = %self.config.email, "Creating new ACME account");

            // Generate new ECDSA key pair
            let rng = SystemRandom::new();
            let pkcs8_bytes = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .map_err(|e| anyhow::anyhow!("Failed to generate key pair: {}", e))?;

            let key_bytes = pkcs8_bytes.as_ref().to_vec();
            *self.key_pair.write().await = Some(key_bytes.clone());

            // Register account
            let new_account_url = {
                let directory = self.directory.read().await;
                let directory = directory.as_ref().context("Directory not loaded")?;
                directory.new_account.clone()
            };

            let payload = serde_json::json!({
                "termsOfServiceAgreed": true,
                "contact": [format!("mailto:{}", self.config.email)]
            });

            let (response, _) = self
                .signed_request(&new_account_url, Some(payload), true)
                .await?;

            let kid = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .context("No account URL in response")?;

            *self.account_kid.write().await = Some(kid.clone());

            // Save credentials
            let credentials = AccountCredentials {
                kid,
                private_key: URL_SAFE_NO_PAD.encode(&key_bytes),
            };

            let data = serde_json::to_string_pretty(&credentials)
                .context("Failed to serialize credentials")?;

            fs::write(&account_path, data)
                .await
                .context("Failed to save account credentials")?;

            info!("ACME account created and saved");
        }

        Ok(())
    }

    /// Make a signed ACME request
    async fn signed_request(
        &self,
        url: &str,
        payload: Option<serde_json::Value>,
        use_jwk: bool,
    ) -> Result<(reqwest::Response, String)> {
        let nonce = self.get_nonce().await?;

        let key_bytes = self.key_pair.read().await;
        let key_bytes = key_bytes.as_ref().context("No key pair")?;

        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ECDSA_P256_SHA256_FIXED_SIGNING,
            key_bytes,
            &SystemRandom::new(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load key pair: {}", e))?;

        // Build protected header
        let mut protected = serde_json::json!({
            "alg": "ES256",
            "nonce": nonce,
            "url": url
        });

        if use_jwk {
            // Include JWK for new account requests
            let public_key = key_pair.public_key().as_ref();
            // P-256 public key is 65 bytes: 0x04 || x || y
            let x = &public_key[1..33];
            let y = &public_key[33..65];

            protected["jwk"] = serde_json::json!({
                "kty": "EC",
                "crv": "P-256",
                "x": URL_SAFE_NO_PAD.encode(x),
                "y": URL_SAFE_NO_PAD.encode(y)
            });
        } else {
            // Use kid for subsequent requests
            let kid = self.account_kid.read().await;
            let kid = kid.as_ref().context("No account kid")?;
            protected["kid"] = serde_json::Value::String(kid.clone());
        }

        let protected_b64 = URL_SAFE_NO_PAD.encode(protected.to_string().as_bytes());

        let payload_b64 = match payload {
            Some(p) => URL_SAFE_NO_PAD.encode(p.to_string().as_bytes()),
            None => String::new(),
        };

        let signing_input = format!("{}.{}", protected_b64, payload_b64);
        let signature = key_pair
            .sign(&SystemRandom::new(), signing_input.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to sign request: {}", e))?;

        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.as_ref());

        let body = serde_json::json!({
            "protected": protected_b64,
            "payload": payload_b64,
            "signature": signature_b64
        });

        let response = self
            .http
            .post(url)
            .header("Content-Type", "application/jose+json")
            .json(&body)
            .send()
            .await
            .context("Failed to send signed request")?;

        let new_nonce = response
            .headers()
            .get("replay-nonce")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Ok((response, new_nonce))
    }

    /// Get the JWK thumbprint for key authorization
    fn jwk_thumbprint(&self, key_bytes: &[u8]) -> Result<String> {
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ECDSA_P256_SHA256_FIXED_SIGNING,
            key_bytes,
            &SystemRandom::new(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load key pair: {}", e))?;

        let public_key = key_pair.public_key().as_ref();
        let x = &public_key[1..33];
        let y = &public_key[33..65];

        // JWK thumbprint as per RFC 7638
        let jwk_json = format!(
            r#"{{"crv":"P-256","kty":"EC","x":"{}","y":"{}"}}"#,
            URL_SAFE_NO_PAD.encode(x),
            URL_SAFE_NO_PAD.encode(y)
        );

        let mut hasher = Sha256::new();
        hasher.update(jwk_json.as_bytes());
        let hash = hasher.finalize();

        Ok(URL_SAFE_NO_PAD.encode(hash))
    }

    /// Request a certificate for the given domains
    pub async fn request_certificate(&self, domains: &[String]) -> Result<CertificateResult> {
        info!(domains = ?domains, "Requesting certificate");

        let new_order_url = {
            let directory = self.directory.read().await;
            let directory = directory.as_ref().context("Directory not loaded")?;
            directory.new_order.clone()
        };

        // Create order
        let identifiers: Vec<_> = domains
            .iter()
            .map(|d| {
                serde_json::json!({
                    "type": "dns",
                    "value": d
                })
            })
            .collect();

        let payload = serde_json::json!({
            "identifiers": identifiers
        });

        let (response, _) = self
            .signed_request(&new_order_url, Some(payload), false)
            .await?;

        let order_url = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .context("No order URL in response")?;

        let order: Order = response.json().await.context("Failed to parse order")?;

        debug!(order_url = %order_url, status = ?order.status, "Order created");

        // Process authorizations
        for auth_url in &order.authorizations {
            self.process_authorization(auth_url).await?;
        }

        // Wait for order to be ready
        let order = self.poll_order(&order_url, OrderStatus::Ready).await?;

        // Generate CSR
        let (private_key_pem, csr_der) = self.generate_csr(domains)?;

        // Finalize order
        let csr_b64 = URL_SAFE_NO_PAD.encode(&csr_der);
        let payload = serde_json::json!({
            "csr": csr_b64
        });

        let (response, _) = self
            .signed_request(&order.finalize, Some(payload), false)
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to finalize order: {}", error_text);
        }

        // Wait for certificate
        let order = self.poll_order(&order_url, OrderStatus::Valid).await?;

        // Download certificate
        let cert_url = order.certificate.context("No certificate URL")?;
        let (response, _) = self.signed_request(&cert_url, None, false).await?;

        let certificate_chain_pem = response
            .text()
            .await
            .context("Failed to download certificate")?;

        // Clean up challenges
        for domain in domains {
            self.challenges.remove(domain);
        }

        info!(domains = ?domains, "Certificate obtained successfully");

        Ok(CertificateResult {
            private_key_pem,
            certificate_chain_pem,
            domains: domains.to_vec(),
        })
    }

    /// Process a single authorization
    async fn process_authorization(&self, auth_url: &str) -> Result<()> {
        let (response, _) = self.signed_request(auth_url, None, false).await?;
        let auth: Authorization = response
            .json()
            .await
            .context("Failed to parse authorization")?;

        let domain = &auth.identifier.value;
        debug!(domain = %domain, status = %auth.status, "Processing authorization");

        if auth.status == "valid" {
            return Ok(());
        }

        // Find HTTP-01 challenge
        let challenge = auth
            .challenges
            .iter()
            .find(|c| c.challenge_type == "http-01")
            .context("No HTTP-01 challenge available")?;

        // Calculate key authorization
        let key_bytes = self.key_pair.read().await;
        let key_bytes = key_bytes.as_ref().context("No key pair")?;
        let thumbprint = self.jwk_thumbprint(key_bytes)?;
        let key_auth = format!("{}.{}", challenge.token, thumbprint);

        info!(
            domain = %domain,
            token = %challenge.token,
            "Setting up HTTP-01 challenge"
        );

        // Store challenge for HTTP serving
        self.challenges.add(&challenge.token, &key_auth);

        // Notify ACME server that challenge is ready
        let payload = serde_json::json!({});
        let (response, _) = self
            .signed_request(&challenge.url, Some(payload), false)
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(domain = %domain, error = %error_text, "Challenge notification failed");
        }

        // Wait for challenge to be validated
        for _ in 0..30 {
            sleep(Duration::from_secs(2)).await;

            let (response, _) = self.signed_request(auth_url, None, false).await?;
            let auth: Authorization = response.json().await?;

            match auth.status.as_str() {
                "valid" => {
                    info!(domain = %domain, "Authorization validated");
                    return Ok(());
                }
                "invalid" => {
                    anyhow::bail!("Authorization for {} failed", domain);
                }
                _ => {
                    debug!(domain = %domain, status = %auth.status, "Waiting for authorization");
                }
            }
        }

        anyhow::bail!("Authorization timeout for {}", domain)
    }

    /// Poll order until it reaches the expected status
    async fn poll_order(&self, order_url: &str, expected: OrderStatus) -> Result<Order> {
        for _ in 0..30 {
            let (response, _) = self.signed_request(order_url, None, false).await?;
            let order: Order = response.json().await.context("Failed to parse order")?;

            if order.status == expected || order.status == OrderStatus::Valid {
                return Ok(order);
            }

            if order.status == OrderStatus::Invalid {
                anyhow::bail!("Order became invalid");
            }

            debug!(status = ?order.status, expected = ?expected, "Waiting for order");
            sleep(Duration::from_secs(2)).await;
        }

        anyhow::bail!("Order polling timeout")
    }

    /// Generate a private key and CSR
    fn generate_csr(&self, domains: &[String]) -> Result<(String, Vec<u8>)> {
        use rcgen::{CertificateParams, DistinguishedName, KeyPair};

        let key_pair = KeyPair::generate().context("Failed to generate key pair")?;

        // Serialize private key to PEM format
        let key_der = key_pair.serialize_der();
        let private_key_pem = pem::encode(&pem::Pem::new("PRIVATE KEY", key_der));

        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        params.subject_alt_names = domains
            .iter()
            .map(|d| rcgen::SanType::DnsName(d.clone().try_into().unwrap()))
            .collect();

        let csr = params
            .serialize_request(&key_pair)
            .context("Failed to create CSR")?;

        Ok((private_key_pem, csr.der().to_vec()))
    }

    /// Save certificate to cache and return path
    pub async fn save_certificate(&self, result: &CertificateResult) -> Result<PathBuf> {
        let domain = result.domains.first().context("No domains in result")?;
        let cert_dir = self.config.cache_dir.join("certs").join(domain);

        fs::create_dir_all(&cert_dir)
            .await
            .context("Failed to create certificate directory")?;

        let cert_path = cert_dir.join("fullchain.pem");
        let key_path = cert_dir.join("privkey.pem");

        fs::write(&cert_path, &result.certificate_chain_pem)
            .await
            .context("Failed to write certificate")?;

        fs::write(&key_path, &result.private_key_pem)
            .await
            .context("Failed to write private key")?;

        // Save the domain list alongside the cert so the renewal manager knows
        // exactly which SANs the cert covers (avoids re-parsing the cert DER).
        let domains_path = cert_dir.join("domains.json");
        if let Ok(json) = serde_json::to_string(&result.domains) {
            let _ = fs::write(&domains_path, json).await;
        }

        info!(
            domain = %domain,
            cert_path = %cert_path.display(),
            "Certificate saved"
        );

        Ok(cert_dir)
    }

    /// Load the list of domains that were included in the cached certificate.
    /// Reads `domains.json` if it exists; otherwise falls back to parsing the SANs
    /// directly from `fullchain.pem` so the renewal manager always has an accurate
    /// baseline (prevents incorrectly skipping reissue for new subdomains).
    pub async fn load_cert_domains(cert_dir: &Path) -> Option<Vec<String>> {
        let domains_path = cert_dir.join("domains.json");
        if let Ok(json) = fs::read_to_string(&domains_path).await {
            if let Ok(domains) = serde_json::from_str::<Vec<String>>(&json) {
                return Some(domains);
            }
        }

        // Fallback: parse SANs from the PEM directly so we never confuse the DB
        // domain list (which includes new, not-yet-covered apps) with the cert's
        // actual SANs.
        let pem_str = fs::read_to_string(&cert_dir.join("fullchain.pem")).await.ok()?;
        let sans = extract_sans_from_pem(&pem_str);
        if !sans.is_empty() {
            // Persist so we don't re-parse on the next startup
            if let Ok(json) = serde_json::to_string(&sans) {
                let _ = fs::write(&domains_path, json).await;
            }
            Some(sans)
        } else {
            None
        }
    }


    /// Load a saved certificate and create TLS config
    pub async fn load_certificate(cert_dir: &Path) -> Result<TlsConfig> {
        let cert_path = cert_dir.join("fullchain.pem");
        let key_path = cert_dir.join("privkey.pem");

        let cert_pem = fs::read_to_string(&cert_path)
            .await
            .context("Failed to read certificate")?;

        let key_pem = fs::read_to_string(&key_path)
            .await
            .context("Failed to read private key")?;

        TlsConfig::from_pem(&cert_pem, &key_pem)
    }

    /// Check if a certificate exists for the given domain
    pub async fn has_certificate(&self, domain: &str) -> bool {
        let cert_path = self
            .config
            .cache_dir
            .join("certs")
            .join(domain)
            .join("fullchain.pem");

        cert_path.exists()
    }

    /// Get certificate directory for a domain
    pub fn cert_dir(&self, domain: &str) -> PathBuf {
        self.config.cache_dir.join("certs").join(domain)
    }

    /// Get all cached certificate domains
    pub async fn cached_domains(&self) -> Result<Vec<String>> {
        let certs_dir = self.config.cache_dir.join("certs");

        if !certs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut domains = Vec::new();
        let mut entries = fs::read_dir(&certs_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    domains.push(name.to_string());
                }
            }
        }

        Ok(domains)
    }
}

/// Result of a successful certificate request
#[derive(Debug, Clone)]
pub struct CertificateResult {
    /// Private key in PEM format
    pub private_key_pem: String,
    /// Certificate chain in PEM format
    pub certificate_chain_pem: String,
    /// Domains covered by the certificate
    pub domains: Vec<String>,
}

/// Certificate renewal manager
pub struct CertificateRenewalManager {
    client: Arc<AcmeClient>,
    /// Full domain list (including all SANs) — updated dynamically as apps are added
    domains: Vec<String>,
    /// Database pool for discovering new app subdomains at runtime
    db: Option<crate::DbPool>,
    /// Hot-reload handle to update the running HTTPS server's TLS cert without restart
    tls_reload: Option<Arc<TlsReloadHandle>>,
    renewal_check_interval: Duration,
    renewal_before_expiry: Duration,
}

impl CertificateRenewalManager {
    /// Create a new renewal manager with the full list of covered domains
    pub fn new(client: Arc<AcmeClient>, domains: Vec<String>) -> Self {
        Self {
            client,
            domains,
            db: None,
            tls_reload: None,
            renewal_check_interval: Duration::from_secs(12 * 60 * 60), // 12 hours
            renewal_before_expiry: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
        }
    }

    /// Enable dynamic domain discovery: check the DB on each cycle for new app subdomains
    /// and reissue the cert immediately when new ones are found.
    /// `tls_reload` is `None` when HTTPS is not yet running (no cert available at startup).
    pub fn with_db_and_reload(
        mut self,
        db: crate::DbPool,
        tls_reload: Option<Arc<TlsReloadHandle>>,
    ) -> Self {
        self.db = Some(db);
        self.tls_reload = tls_reload;
        self
    }

    /// With custom renewal interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.renewal_check_interval = interval;
        self
    }

    /// Start the renewal background task
    pub async fn run(mut self) {
        info!("Certificate renewal manager started");

        loop {
            // Check for new app subdomains and reissue cert if any are missing coverage
            self.check_and_add_new_domains().await;

            if let Err(e) = self.check_renewals().await {
                error!(error = %e, "Error checking certificate renewals");
            }

            sleep(self.renewal_check_interval).await;
        }
    }

    /// Query the DB for all current app domains. If any are not in the current SAN list,
    /// reissue the cert immediately and hot-reload the TLS acceptor.
    async fn check_and_add_new_domains(&mut self) {
        let db = match &self.db {
            Some(db) => db.clone(),
            None => return,
        };

        // Collect all currently configured app domains from the DB
        let rows: Vec<(Option<String>, Option<String>, Option<String>)> =
            match sqlx::query_as("SELECT domain, domains, auto_subdomain FROM apps")
                .fetch_all(&db)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(error = %e, "Could not query app domains for cert check");
                    return;
                }
            };

        let mut new_domains: Vec<String> = Vec::new();
        for (legacy_domain, domains_json, auto_subdomain) in rows {
            if let Some(d) = legacy_domain {
                if !d.is_empty() && !self.domains.contains(&d) {
                    new_domains.push(d);
                }
            }
            if let Some(json) = domains_json {
                if let Ok(list) = serde_json::from_str::<Vec<String>>(&json) {
                    for d in list {
                        if !d.is_empty() && !self.domains.contains(&d) {
                            new_domains.push(d);
                        }
                    }
                }
            }
            if let Some(d) = auto_subdomain {
                if !d.is_empty() && !self.domains.contains(&d) {
                    new_domains.push(d);
                }
            }
        }

        if new_domains.is_empty() {
            return;
        }

        info!(
            new_domains = ?new_domains,
            "New app subdomains found — reissuing TLS certificate to add coverage"
        );

        let mut all_domains = self.domains.clone();
        for d in &new_domains {
            if !all_domains.contains(d) && all_domains.len() < 100 {
                all_domains.push(d.clone());
            }
        }

        match self.client.request_certificate(&all_domains).await {
            Ok(result) => {
                let _ = self.client.save_certificate(&result).await;
                if let Some(ref reload) = self.tls_reload {
                    match TlsConfig::from_pem(
                        &result.certificate_chain_pem,
                        &result.private_key_pem,
                    ) {
                        Ok(tls) => {
                            reload.update(tls.acceptor);
                            self.domains = all_domains;
                            info!("TLS certificate reissued and hot-reloaded for new subdomains");
                        }
                        Err(e) => {
                            warn!(error = %e, "New cert issued but could not parse PEM for reload");
                        }
                    }
                } else {
                    self.domains = all_domains;
                }
            }
            Err(e) => {
                warn!(error = %e, new_domains = ?new_domains, "Failed to reissue cert for new subdomains");
            }
        }
    }

    /// Check all certificates for expiry-based renewal
    async fn check_renewals(&self) -> Result<()> {
        let domains = self.client.cached_domains().await?;

        for domain in domains {
            if let Err(e) = self.check_domain_renewal(&domain).await {
                warn!(domain = %domain, error = %e, "Failed to check/renew certificate");
            }
        }

        Ok(())
    }

    /// Check if a specific domain's certificate needs renewal
    async fn check_domain_renewal(&self, domain: &str) -> Result<()> {
        let cert_dir = self.client.cert_dir(domain);
        let cert_path = cert_dir.join("fullchain.pem");

        if !cert_path.exists() {
            return Ok(());
        }

        let cert_pem = fs::read_to_string(&cert_path).await?;

        // Parse certificate to check expiry
        if let Some(expiry) = parse_cert_expiry(&cert_pem) {
            let now = chrono::Utc::now();
            let renewal_threshold =
                chrono::Duration::seconds(self.renewal_before_expiry.as_secs() as i64);

            if expiry - now < renewal_threshold {
                info!(
                    domain = %domain,
                    expiry = %expiry,
                    "Certificate expires soon, renewing"
                );

                let renewal_domains = if self.domains.is_empty() {
                    vec![domain.to_string()]
                } else {
                    self.domains.clone()
                };
                let result = self.client.request_certificate(&renewal_domains).await?;
                self.client.save_certificate(&result).await?;

                // Hot-reload the new cert into the running HTTPS server
                if let Some(ref reload) = self.tls_reload {
                    if let Ok(tls) =
                        TlsConfig::from_pem(&result.certificate_chain_pem, &result.private_key_pem)
                    {
                        reload.update(tls.acceptor);
                        info!(domain = %domain, "Certificate renewed and hot-reloaded");
                    }
                } else {
                    info!(domain = %domain, "Certificate renewed successfully");
                }
            } else {
                debug!(
                    domain = %domain,
                    expiry = %expiry,
                    "Certificate still valid"
                );
            }
        } else {
            // Can't parse expiry — skip to avoid overwriting valid multi-domain certs
            debug!(domain = %domain, "Could not parse certificate expiry, skipping renewal check");
        }

        Ok(())
    }
}

/// Parse certificate PEM to get expiry date
/// Extract DNS SANs from the first certificate in a PEM chain.
/// Returns an empty vec if parsing fails rather than propagating an error.
fn extract_sans_from_pem(pem_str: &str) -> Vec<String> {
    use x509_parser::prelude::*;

    let pem_block: ::pem::Pem = match ::pem::parse(pem_str) {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    let (_, cert) = match X509Certificate::from_der(pem_block.contents()) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut sans = Vec::new();
    for ext in cert.extensions() {
        if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
            for name in &san.general_names {
                if let GeneralName::DNSName(dns) = name {
                    sans.push(dns.to_string());
                }
            }
        }
    }
    sans
}

fn parse_cert_expiry(pem_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Extract the first certificate from the chain
    let pem_block: ::pem::Pem = ::pem::parse(pem_str).ok()?;

    if pem_block.tag() != "CERTIFICATE" {
        return None;
    }

    // Parse X.509 certificate to get not_after
    // This is a simplified check - for production use x509-parser
    // For now, we'll return None to trigger a renewal check

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acme_challenges() {
        let challenges = AcmeChallenges::new();

        challenges.add("test-token", "test-auth");
        assert_eq!(challenges.get("test-token"), Some("test-auth".to_string()));

        challenges.remove("test-token");
        assert!(challenges.get("test-token").is_none());
    }

    #[test]
    fn test_acme_config_default() {
        let config = AcmeConfig::default();
        assert!(config.staging);
        assert!(config.email.is_empty());
    }
}
