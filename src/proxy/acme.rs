// ACME client for Let's Encrypt certificate automation
//
// This module implements the ACME protocol (RFC 8555) with HTTP-01 and DNS-01
// challenges for automatic TLS certificate provisioning. Scheduling (which
// names to order, renewals, backoff) lives in `proxy::certs`.

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use dashmap::DashMap;
use ring::rand::SystemRandom;
use ring::signature::{EcdsaKeyPair, KeyPair as RingKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, info, warn};

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
    /// Override the ACME directory URL (tests, or another ACME CA).
    pub directory_url: Option<String>,
}

impl Default for AcmeConfig {
    fn default() -> Self {
        Self {
            email: String::new(),
            cache_dir: PathBuf::from("./data/acme"),
            staging: true,
            directory_url: None,
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
    /// Delay between authorization/order status polls.
    poll_interval: Duration,
}

impl AcmeClient {
    /// Create a new ACME client
    pub async fn new(config: AcmeConfig) -> Result<Self> {
        Self::with_challenges(config, AcmeChallenges::new()).await
    }

    /// Create a client that publishes HTTP-01 tokens into `challenges`, which
    /// the port-80 proxy already serves.
    pub async fn with_challenges(config: AcmeConfig, challenges: AcmeChallenges) -> Result<Self> {
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
            challenges,
            poll_interval: Duration::from_secs(2),
        };

        // Initialize directory and account
        client.fetch_directory().await?;
        client.load_or_create_account().await?;

        Ok(client)
    }

    /// Change the delay between status polls (tests use a short one).
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Get the challenges store for HTTP-01 challenge serving
    pub fn challenges(&self) -> AcmeChallenges {
        self.challenges.clone()
    }

    /// Fetch the ACME directory
    async fn fetch_directory(&self) -> Result<()> {
        let url = match self.config.directory_url.as_deref() {
            Some(url) => url,
            None if self.config.staging => LETS_ENCRYPT_STAGING,
            None => LETS_ENCRYPT_PRODUCTION,
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

    /// Request a certificate for `domains` using HTTP-01 challenges.
    pub async fn request_certificate(&self, domains: &[String]) -> Result<CertificateResult> {
        self.request_certificate_with(domains, &ChallengeSolver::Http01)
            .await
    }

    /// Request a certificate for `domains`, proving control with `solver`.
    /// Wildcard names (`*.example.com`) need DNS-01.
    pub async fn request_certificate_with(
        &self,
        domains: &[String],
        solver: &ChallengeSolver<'_>,
    ) -> Result<CertificateResult> {
        info!(domains = ?domains, challenge = solver.kind(), "Requesting certificate");

        let new_order_url = {
            let directory = self.directory.read().await;
            let directory = directory.as_ref().context("Directory not loaded")?;
            directory.new_order.clone()
        };

        let identifiers: Vec<_> = domains
            .iter()
            .map(|d| serde_json::json!({ "type": "dns", "value": d }))
            .collect();
        let payload = serde_json::json!({ "identifiers": identifiers });

        let (response, _) = self
            .signed_request(&new_order_url, Some(payload), false)
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create order: {}", acme_error_detail(&error_text));
        }

        let order_url = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .context("No order URL in response")?;

        let order: Order = response.json().await.context("Failed to parse order")?;
        debug!(order_url = %order_url, status = ?order.status, "Order created");

        // Prove control of every name. Cleanup runs whether or not this worked.
        let mut cleanup = Vec::new();
        let mut result = Ok(());
        for auth_url in &order.authorizations {
            if let Err(e) = self
                .process_authorization(auth_url, solver, &mut cleanup)
                .await
            {
                result = Err(e);
                break;
            }
        }
        self.cleanup_challenges(solver, cleanup).await;
        result?;

        let order = self.poll_order(&order_url, OrderStatus::Ready).await?;

        let (private_key_pem, csr_der) = self.generate_csr(domains)?;
        let payload = serde_json::json!({ "csr": URL_SAFE_NO_PAD.encode(&csr_der) });
        let (response, _) = self
            .signed_request(&order.finalize, Some(payload), false)
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to finalize order: {}",
                acme_error_detail(&error_text)
            );
        }

        let order = self.poll_order(&order_url, OrderStatus::Valid).await?;

        let cert_url = order.certificate.context("No certificate URL")?;
        let (response, _) = self.signed_request(&cert_url, None, false).await?;
        let certificate_chain_pem = response
            .text()
            .await
            .context("Failed to download certificate")?;

        info!(domains = ?domains, "Certificate obtained successfully");

        Ok(CertificateResult {
            private_key_pem,
            certificate_chain_pem,
            domains: domains.to_vec(),
        })
    }

    async fn cleanup_challenges(&self, solver: &ChallengeSolver<'_>, cleanup: Vec<PendingCleanup>) {
        for item in cleanup {
            match item {
                PendingCleanup::Http01 { token } => self.challenges.remove(&token),
                PendingCleanup::Dns01 { fqdn, record_id } => {
                    if let ChallengeSolver::Dns01(dns) = solver {
                        if let Err(e) = dns.cleanup(&fqdn, &record_id).await {
                            warn!(fqdn = %fqdn, error = %e, "Failed to remove DNS-01 TXT record");
                        }
                    }
                }
            }
        }
    }

    /// Process a single authorization with the given solver.
    async fn process_authorization(
        &self,
        auth_url: &str,
        solver: &ChallengeSolver<'_>,
        cleanup: &mut Vec<PendingCleanup>,
    ) -> Result<()> {
        let (response, _) = self.signed_request(auth_url, None, false).await?;
        let auth: Authorization = response
            .json()
            .await
            .context("Failed to parse authorization")?;

        let domain = auth.identifier.value.clone();
        debug!(domain = %domain, status = %auth.status, "Processing authorization");

        if auth.status == "valid" {
            return Ok(());
        }

        let challenge_type = solver.kind();
        let challenge = auth
            .challenges
            .iter()
            .find(|c| c.challenge_type == challenge_type)
            .with_context(|| format!("No {challenge_type} challenge offered for {domain}"))?;

        let key_bytes = self.key_pair.read().await;
        let key_bytes = key_bytes.as_ref().context("No key pair")?;
        let thumbprint = self.jwk_thumbprint(key_bytes)?;
        let key_auth = format!("{}.{}", challenge.token, thumbprint);

        match solver {
            ChallengeSolver::Http01 => {
                info!(domain = %domain, token = %challenge.token, "Setting up HTTP-01 challenge");
                self.challenges.add(&challenge.token, &key_auth);
                cleanup.push(PendingCleanup::Http01 {
                    token: challenge.token.clone(),
                });
            }
            ChallengeSolver::Dns01(dns) => {
                let fqdn = dns01_record_name(&domain);
                let value = dns01_txt_value(&key_auth);
                info!(domain = %domain, record = %fqdn, "Setting up DNS-01 challenge");
                let record_id = dns
                    .present(&fqdn, &value)
                    .await
                    .with_context(|| format!("Could not create TXT record {fqdn}"))?;
                cleanup.push(PendingCleanup::Dns01 {
                    fqdn: fqdn.clone(),
                    record_id,
                });
                dns.wait_for_propagation(&fqdn, &value)
                    .await
                    .with_context(|| format!("TXT record {fqdn} did not propagate"))?;
            }
        }

        // Tell the CA the challenge is ready.
        let (response, _) = self
            .signed_request(&challenge.url, Some(serde_json::json!({})), false)
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            warn!(domain = %domain, error = %error_text, "Challenge notification failed");
        }

        for _ in 0..30 {
            sleep(self.poll_interval).await;

            let (response, _) = self.signed_request(auth_url, None, false).await?;
            let body = response.text().await.unwrap_or_default();
            let auth: Authorization =
                serde_json::from_str(&body).context("Failed to parse authorization")?;

            match auth.status.as_str() {
                "valid" => {
                    info!(domain = %domain, "Authorization validated");
                    return Ok(());
                }
                "invalid" => {
                    anyhow::bail!(
                        "Authorization for {} failed: {}",
                        domain,
                        challenge_error(&body).unwrap_or_else(|| "no detail".to_string())
                    );
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
            sleep(self.poll_interval).await;
        }

        anyhow::bail!("Order polling timeout")
    }

    /// Generate a private key and CSR
    fn generate_csr(&self, domains: &[String]) -> Result<(String, Vec<u8>)> {
        use rcgen::{CertificateParams, DistinguishedName, KeyPair};

        let key_pair = KeyPair::generate().context("Failed to generate key pair")?;
        let key_der = key_pair.serialize_der();
        let private_key_pem = pem::encode(&pem::Pem::new("PRIVATE KEY", key_der));

        let mut params = CertificateParams::default();
        params.distinguished_name = DistinguishedName::new();
        params.subject_alt_names = domains
            .iter()
            .map(|d| {
                d.clone()
                    .try_into()
                    .map(rcgen::SanType::DnsName)
                    .map_err(|e| anyhow::anyhow!("Invalid domain name {d}: {e}"))
            })
            .collect::<Result<_>>()?;

        let csr = params
            .serialize_request(&key_pair)
            .context("Failed to create CSR")?;

        Ok((private_key_pem, csr.der().to_vec()))
    }
}

/// How the client proves control of the names in an order.
pub enum ChallengeSolver<'a> {
    /// Serve the key authorization on port 80 (`AcmeChallenges`).
    Http01,
    /// Publish a TXT record through a DNS provider. Needed for wildcards.
    Dns01(&'a dyn Dns01Solver),
}

impl ChallengeSolver<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Self::Http01 => "http-01",
            Self::Dns01(_) => "dns-01",
        }
    }
}

/// Publishes and removes `_acme-challenge` TXT records.
#[async_trait::async_trait]
pub trait Dns01Solver: Send + Sync {
    /// Create the TXT record; returns an id used for cleanup.
    async fn present(&self, fqdn: &str, value: &str) -> Result<String>;
    /// Wait until the record is visible to public resolvers.
    async fn wait_for_propagation(&self, fqdn: &str, value: &str) -> Result<()>;
    /// Delete the record created by `present`.
    async fn cleanup(&self, fqdn: &str, record_id: &str) -> Result<()>;
}

enum PendingCleanup {
    Http01 { token: String },
    Dns01 { fqdn: String, record_id: String },
}

/// `_acme-challenge.<domain>` for an authorization identifier. Wildcard
/// authorizations carry the base domain, so the `*.` is stripped either way.
pub fn dns01_record_name(domain: &str) -> String {
    format!("_acme-challenge.{}", domain.trim_start_matches("*."))
}

/// TXT value for a key authorization: base64url(SHA-256(key_authorization)).
pub fn dns01_txt_value(key_authorization: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(key_authorization.as_bytes()))
}

/// The `detail` of an ACME problem document, or the raw body.
fn acme_error_detail(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("detail").and_then(|d| d.as_str()).map(str::to_string))
        .unwrap_or_else(|| body.chars().take(300).collect())
}

/// The error detail of the first failed challenge in an authorization body.
fn challenge_error(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("challenges")?
        .as_array()?
        .iter()
        .find_map(|c| c.get("error")?.get("detail")?.as_str().map(str::to_string))
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

/// Every hostname the proxy routes that should be covered by the TLS certificate:
/// app domains (the legacy `domain` column, the `domains` JSON list and the
/// auto-generated subdomain) and Docker Compose service domains.
///
/// Startup and the renewal loop both use this, so the two can no longer disagree
/// about what belongs on the certificate. Service domains used to be left out, so
/// a compose service with a domain was routed but served the instance certificate.
pub async fn collect_cert_domains(db: &crate::DbPool) -> Result<Vec<String>> {
    let apps: Vec<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT domain, domains, auto_subdomain FROM apps")
            .fetch_all(db)
            .await?;
    let services: Vec<(Option<String>,)> = sqlx::query_as("SELECT domain FROM services")
        .fetch_all(db)
        .await?;

    let mut result: Vec<String> = Vec::new();
    let mut add = |d: String| {
        let d = d.trim().to_lowercase();
        if !d.is_empty() && !result.contains(&d) {
            result.push(d);
        }
    };

    for (legacy_domain, domains_json, auto_subdomain) in apps {
        if let Some(d) = legacy_domain {
            add(d);
        }
        if let Some(json) = domains_json {
            add_json_domains(&json, &mut add);
        }
        if let Some(d) = auto_subdomain {
            // Wildcard-DNS helper hosts cannot get a certificate from us.
            if !d.ends_with(".traefik.me") && !d.ends_with(".sslip.io") {
                add(d);
            }
        }
    }
    for (domain,) in services {
        if let Some(d) = domain {
            add(d);
        }
    }
    Ok(result)
}

/// The `domains` column is persisted as an array of Domain objects
/// (`[{"domain":"...","primary":...}]`), but older data may store a plain
/// `["..."]` array of strings. Accept both shapes.
fn add_json_domains(json: &str, add: &mut impl FnMut(String)) {
    if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
        for v in values {
            match v {
                serde_json::Value::String(s) => add(s),
                serde_json::Value::Object(ref map) => {
                    if let Some(d) = map.get("domain").and_then(|x| x.as_str()) {
                        add(d.to_string());
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collect_cert_domains_covers_apps_and_services() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for ddl in [
            "CREATE TABLE apps (domain TEXT, domains TEXT, auto_subdomain TEXT)",
            "CREATE TABLE services (domain TEXT)",
            "INSERT INTO apps VALUES ('app.example.com', '[{\"domain\":\"www.example.com\",\"primary\":false}]', 'x.sslip.io')",
            "INSERT INTO apps VALUES (NULL, '[\"legacy.example.com\"]', 'auto.example.com')",
            "INSERT INTO apps VALUES ('', NULL, NULL)",
            "INSERT INTO services VALUES ('Sign.Example.com')",
            "INSERT INTO services VALUES ('app.example.com')",
            "INSERT INTO services VALUES (NULL)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }

        let domains = collect_cert_domains(&pool).await.unwrap();
        assert_eq!(
            domains,
            vec![
                "app.example.com",
                "www.example.com",
                "legacy.example.com",
                "auto.example.com",
                "sign.example.com",
            ]
        );
    }

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
