//! DNS providers for ACME DNS-01 challenges (wildcard certificates).
//!
//! Supported: Cloudflare (API token), AWS Route 53 (access key, SigV4),
//! Hetzner (Cloud DNS API token) and DigitalOcean (API token). Credentials are
//! stored in `dns_providers.credentials` as JSON, encrypted with the instance
//! key when one is configured.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use hmac::{Hmac, KeyInit, Mac};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use tracing::debug;

use crate::proxy::acme::Dns01Solver;

pub const PROVIDERS: &[&str] = &["cloudflare", "route53", "hetzner", "digitalocean"];

/// Resolvers used to confirm propagation when none are set in Settings.
const DEFAULT_RESOLVERS: &[&str] = &["1.1.1.1", "8.8.8.8"];
const PROPAGATION_TIMEOUT: Duration = Duration::from_secs(180);
const PROPAGATION_POLL: Duration = Duration::from_secs(5);
const TXT_TTL: u32 = 60;

/// Provider credentials. Which fields are required depends on the provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsCredentials {
    /// Cloudflare, Hetzner and DigitalOcean API token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
    /// Route 53 access key id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// Route 53 secret access key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// Route 53 signing region (default us-east-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Route 53 hosted zone id, to skip zone lookup (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_zone_id: Option<String>,
}

impl DnsCredentials {
    /// Check the fields `provider` needs are present.
    pub fn validate(&self, provider: &str) -> std::result::Result<(), String> {
        let has = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.trim().is_empty());
        match provider {
            "cloudflare" | "hetzner" | "digitalocean" => {
                if !has(&self.api_token) {
                    return Err("An API token is required".into());
                }
            }
            "route53" => {
                if !has(&self.access_key_id) || !has(&self.secret_access_key) {
                    return Err("An access key id and secret access key are required".into());
                }
            }
            other => return Err(format!("Unknown DNS provider: {other}")),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DnsProviderRow {
    pub id: String,
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing)]
    pub credentials: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list(db: &SqlitePool) -> sqlx::Result<Vec<DnsProviderRow>> {
    sqlx::query_as("SELECT * FROM dns_providers ORDER BY name")
        .fetch_all(db)
        .await
}

pub async fn get(db: &SqlitePool, id: &str) -> sqlx::Result<Option<DnsProviderRow>> {
    sqlx::query_as("SELECT * FROM dns_providers WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Decrypt a row's credentials.
pub fn credentials(row: &DnsProviderRow, key: Option<&[u8; 32]>) -> Result<DnsCredentials> {
    let json = crate::crypto::decrypt_if_encrypted(&row.credentials, key)?;
    serde_json::from_str(&json).context("Stored DNS credentials are not valid JSON")
}

/// Encrypt credentials for storage.
pub fn seal(creds: &DnsCredentials, key: Option<&[u8; 32]>) -> Result<String> {
    crate::crypto::encrypt_if_key_available(&serde_json::to_string(creds)?, key)
}

// ---------------------------------------------------------------------------
// Provider APIs
// ---------------------------------------------------------------------------

/// Minimal record management a DNS-01 challenge needs.
#[async_trait]
pub trait DnsApi: Send + Sync {
    /// Create a TXT record; returns an opaque id for `delete_txt`.
    async fn create_txt(&self, fqdn: &str, value: &str) -> Result<String>;
    async fn delete_txt(&self, fqdn: &str, record_id: &str) -> Result<()>;
    /// Check the credentials work; returns a short description (zones seen).
    async fn verify(&self) -> Result<String>;
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// Candidate zone names for `fqdn`, longest first, never a bare TLD.
fn zone_candidates(fqdn: &str) -> Vec<String> {
    let name = fqdn.trim_end_matches('.');
    let labels: Vec<&str> = name.split('.').collect();
    (1..labels.len().saturating_sub(1))
        .map(|i| labels[i..].join("."))
        .collect()
}

/// `fqdn` relative to `zone` (`_acme-challenge.app` in `example.com`).
fn relative_name(fqdn: &str, zone: &str) -> String {
    let fqdn = fqdn.trim_end_matches('.');
    fqdn.strip_suffix(zone)
        .map(|s| s.trim_end_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "@".to_string())
}

async fn json_or_error(resp: reqwest::Response, what: &str) -> Result<Value> {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "{what} failed ({status}): {}",
            body.chars().take(300).collect::<String>()
        );
    }
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&body).with_context(|| format!("{what}: unexpected response"))
}

// --- Cloudflare -------------------------------------------------------------

pub struct Cloudflare {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Cloudflare {
    pub fn new(token: &str, base: Option<&str>) -> Self {
        Self {
            base: base
                .unwrap_or("https://api.cloudflare.com/client/v4")
                .to_string(),
            token: token.to_string(),
            http: http(),
        }
    }

    async fn zone_id(&self, fqdn: &str) -> Result<String> {
        for zone in zone_candidates(fqdn) {
            let resp = self
                .http
                .get(format!("{}/zones", self.base))
                .bearer_auth(&self.token)
                .query(&[("name", zone.as_str())])
                .send()
                .await?;
            let v = json_or_error(resp, "Cloudflare zone lookup").await?;
            if let Some(id) = v["result"][0]["id"].as_str() {
                return Ok(id.to_string());
            }
        }
        bail!("No Cloudflare zone found for {fqdn}")
    }
}

#[async_trait]
impl DnsApi for Cloudflare {
    async fn create_txt(&self, fqdn: &str, value: &str) -> Result<String> {
        let zone = self.zone_id(fqdn).await?;
        let resp = self
            .http
            .post(format!("{}/zones/{zone}/dns_records", self.base))
            .bearer_auth(&self.token)
            .json(&json!({ "type": "TXT", "name": fqdn, "content": value, "ttl": TXT_TTL }))
            .send()
            .await?;
        let v = json_or_error(resp, "Cloudflare record create").await?;
        let id = v["result"]["id"]
            .as_str()
            .context("Cloudflare returned no record id")?;
        Ok(format!("{zone}/{id}"))
    }

    async fn delete_txt(&self, _fqdn: &str, record_id: &str) -> Result<()> {
        let (zone, id) = record_id
            .split_once('/')
            .context("Malformed Cloudflare record id")?;
        let resp = self
            .http
            .delete(format!("{}/zones/{zone}/dns_records/{id}", self.base))
            .bearer_auth(&self.token)
            .send()
            .await?;
        json_or_error(resp, "Cloudflare record delete").await?;
        Ok(())
    }

    async fn verify(&self) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/zones", self.base))
            .bearer_auth(&self.token)
            .query(&[("per_page", "50")])
            .send()
            .await?;
        let v = json_or_error(resp, "Cloudflare zone list").await?;
        let zones: Vec<&str> = v["result"]
            .as_array()
            .map(|a| a.iter().filter_map(|z| z["name"].as_str()).collect())
            .unwrap_or_default();
        Ok(describe_zones(&zones))
    }
}

fn describe_zones(zones: &[&str]) -> String {
    if zones.is_empty() {
        "Credentials work, but no zones are visible to them".to_string()
    } else {
        format!("Zones: {}", zones.join(", "))
    }
}

// --- DigitalOcean -----------------------------------------------------------

pub struct DigitalOcean {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl DigitalOcean {
    pub fn new(token: &str, base: Option<&str>) -> Self {
        Self {
            base: base.unwrap_or("https://api.digitalocean.com").to_string(),
            token: token.to_string(),
            http: http(),
        }
    }

    async fn zone(&self, fqdn: &str) -> Result<String> {
        for zone in zone_candidates(fqdn) {
            let resp = self
                .http
                .get(format!("{}/v2/domains/{zone}", self.base))
                .bearer_auth(&self.token)
                .send()
                .await?;
            if resp.status().is_success() {
                return Ok(zone);
            }
            if resp.status() != reqwest::StatusCode::NOT_FOUND {
                json_or_error(resp, "DigitalOcean domain lookup").await?;
            }
        }
        bail!("No DigitalOcean domain found for {fqdn}")
    }
}

#[async_trait]
impl DnsApi for DigitalOcean {
    async fn create_txt(&self, fqdn: &str, value: &str) -> Result<String> {
        let zone = self.zone(fqdn).await?;
        let resp = self
            .http
            .post(format!("{}/v2/domains/{zone}/records", self.base))
            .bearer_auth(&self.token)
            .json(&json!({
                "type": "TXT",
                "name": relative_name(fqdn, &zone),
                "data": value,
                "ttl": 30,
            }))
            .send()
            .await?;
        let v = json_or_error(resp, "DigitalOcean record create").await?;
        let id = v["domain_record"]["id"]
            .as_i64()
            .context("DigitalOcean returned no record id")?;
        Ok(format!("{zone}/{id}"))
    }

    async fn delete_txt(&self, _fqdn: &str, record_id: &str) -> Result<()> {
        let (zone, id) = record_id
            .rsplit_once('/')
            .context("Malformed DigitalOcean record id")?;
        let resp = self
            .http
            .delete(format!("{}/v2/domains/{zone}/records/{id}", self.base))
            .bearer_auth(&self.token)
            .send()
            .await?;
        json_or_error(resp, "DigitalOcean record delete").await?;
        Ok(())
    }

    async fn verify(&self) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/v2/domains", self.base))
            .bearer_auth(&self.token)
            .send()
            .await?;
        let v = json_or_error(resp, "DigitalOcean domain list").await?;
        let zones: Vec<&str> = v["domains"]
            .as_array()
            .map(|a| a.iter().filter_map(|z| z["name"].as_str()).collect())
            .unwrap_or_default();
        Ok(describe_zones(&zones))
    }
}

// --- Hetzner (Cloud DNS) ----------------------------------------------------

pub struct Hetzner {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Hetzner {
    pub fn new(token: &str, base: Option<&str>) -> Self {
        Self {
            base: base.unwrap_or("https://api.hetzner.cloud/v1").to_string(),
            token: token.to_string(),
            http: http(),
        }
    }

    async fn zone(&self, fqdn: &str) -> Result<String> {
        for zone in zone_candidates(fqdn) {
            let resp = self
                .http
                .get(format!("{}/zones", self.base))
                .bearer_auth(&self.token)
                .query(&[("name", zone.as_str())])
                .send()
                .await?;
            let v = json_or_error(resp, "Hetzner zone lookup").await?;
            if v["zones"].as_array().is_some_and(|z| !z.is_empty()) {
                return Ok(zone);
            }
        }
        bail!("No Hetzner DNS zone found for {fqdn}")
    }

    async fn rrset_action(&self, zone: &str, name: &str, action: &str, value: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!(
                "{}/zones/{zone}/rrsets/{name}/TXT/actions/{action}",
                self.base
            ))
            .bearer_auth(&self.token)
            .json(&json!({ "ttl": TXT_TTL, "records": [{ "value": format!("\"{value}\"") }] }))
            .send()
            .await?;
        json_or_error(resp, "Hetzner record update").await?;
        Ok(())
    }
}

#[async_trait]
impl DnsApi for Hetzner {
    async fn create_txt(&self, fqdn: &str, value: &str) -> Result<String> {
        let zone = self.zone(fqdn).await?;
        let name = relative_name(fqdn, &zone);
        self.rrset_action(&zone, &name, "add_records", value)
            .await?;
        Ok(format!("{zone}|{name}|{value}"))
    }

    async fn delete_txt(&self, _fqdn: &str, record_id: &str) -> Result<()> {
        let mut parts = record_id.splitn(3, '|');
        let (Some(zone), Some(name), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            bail!("Malformed Hetzner record id");
        };
        let resp = self
            .http
            .post(format!(
                "{}/zones/{zone}/rrsets/{name}/TXT/actions/remove_records",
                self.base
            ))
            .bearer_auth(&self.token)
            .json(&json!({ "records": [{ "value": format!("\"{value}\"") }] }))
            .send()
            .await?;
        json_or_error(resp, "Hetzner record delete").await?;
        Ok(())
    }

    async fn verify(&self) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/zones", self.base))
            .bearer_auth(&self.token)
            .send()
            .await?;
        let v = json_or_error(resp, "Hetzner zone list").await?;
        let zones: Vec<&str> = v["zones"]
            .as_array()
            .map(|a| a.iter().filter_map(|z| z["name"].as_str()).collect())
            .unwrap_or_default();
        Ok(describe_zones(&zones))
    }
}

// --- AWS Route 53 -----------------------------------------------------------

pub struct Route53 {
    base: String,
    access_key: String,
    secret_key: String,
    region: String,
    zone_id: Option<String>,
    http: reqwest::Client,
    /// Values currently published per name: Route 53 replaces a whole record
    /// set, so a wildcard plus apex order must publish both values together.
    values: Mutex<HashMap<String, Vec<String>>>,
}

type HmacSha256 = Hmac<Sha256>;

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// AWS Signature Version 4 `Authorization` header for a request signed over
/// `host` and `x-amz-date`. `query` must already be canonical (sorted, encoded).
#[allow(clippy::too_many_arguments)]
pub fn sigv4_authorization(
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    method: &str,
    host: &str,
    path: &str,
    query: &str,
    amz_date: &str,
    body: &[u8],
) -> String {
    let date = &amz_date[..8];
    let canonical_request = format!(
        "{method}\n{path}\n{query}\nhost:{host}\nx-amz-date:{amz_date}\n\nhost;x-amz-date\n{}",
        sha256_hex(body)
    );
    let scope = format!("{date}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let k_date = hmac(format!("AWS4{secret_key}").as_bytes(), date);
    let k_region = hmac(&k_date, region);
    let k_service = hmac(&k_region, service);
    let k_signing = hmac(&k_service, "aws4_request");
    let signature = hex::encode(hmac(&k_signing, &string_to_sign));
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders=host;x-amz-date, Signature={signature}"
    )
}

fn xml_tag<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(&close) else { break };
        out.push(&after[..end]);
        rest = &after[end + close.len()..];
    }
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl Route53 {
    pub fn new(creds: &DnsCredentials, base: Option<&str>) -> Self {
        Self {
            base: base.unwrap_or("https://route53.amazonaws.com").to_string(),
            access_key: creds.access_key_id.clone().unwrap_or_default(),
            secret_key: creds.secret_access_key.clone().unwrap_or_default(),
            region: creds
                .region
                .clone()
                .filter(|r| !r.trim().is_empty())
                .unwrap_or_else(|| "us-east-1".into()),
            zone_id: creds
                .hosted_zone_id
                .clone()
                .filter(|z| !z.trim().is_empty()),
            http: http(),
            values: Mutex::new(HashMap::new()),
        }
    }

    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<String>,
    ) -> Result<String> {
        let url = reqwest::Url::parse(&format!("{}{path}", self.base))?;
        let host = match url.port() {
            Some(p) => format!("{}:{p}", url.host_str().unwrap_or_default()),
            None => url.host_str().unwrap_or_default().to_string(),
        };
        let mut pairs: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (aws_encode(k), aws_encode(v)))
            .collect();
        pairs.sort();
        let canonical_query = pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let amz_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let body = body.unwrap_or_default();
        let auth = sigv4_authorization(
            &self.access_key,
            &self.secret_key,
            &self.region,
            "route53",
            method.as_str(),
            &host,
            path,
            &canonical_query,
            &amz_date,
            body.as_bytes(),
        );
        let full = if canonical_query.is_empty() {
            format!("{}{path}", self.base)
        } else {
            format!("{}{path}?{canonical_query}", self.base)
        };
        let resp = self
            .http
            .request(method, full)
            .header("x-amz-date", &amz_date)
            .header("authorization", auth)
            .header("content-type", "text/xml")
            .body(body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            let msg = xml_tag(&text, "Message").first().copied().unwrap_or(&text);
            bail!("Route 53 request failed ({status}): {msg}");
        }
        Ok(text)
    }

    async fn zone(&self, fqdn: &str) -> Result<String> {
        if let Some(z) = &self.zone_id {
            return Ok(z.trim_start_matches("/hostedzone/").to_string());
        }
        for zone in zone_candidates(fqdn) {
            let xml = self
                .send(
                    reqwest::Method::GET,
                    "/2013-04-01/hostedzonesbyname",
                    &[("dnsname", &zone), ("maxitems", "1")],
                    None,
                )
                .await?;
            let names = xml_tag(&xml, "Name");
            let ids = xml_tag(&xml, "Id");
            if names.first().map(|n| n.trim_end_matches('.')) == Some(zone.as_str()) {
                if let Some(id) = ids.first() {
                    return Ok(id.trim_start_matches("/hostedzone/").to_string());
                }
            }
        }
        bail!("No Route 53 hosted zone found for {fqdn}")
    }

    async fn change(&self, zone: &str, action: &str, fqdn: &str, values: &[String]) -> Result<()> {
        let records: String = values
            .iter()
            .map(|v| {
                format!(
                    "<ResourceRecord><Value>{}</Value></ResourceRecord>",
                    xml_escape(&format!("\"{v}\""))
                )
            })
            .collect();
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<ChangeResourceRecordSetsRequest xmlns=\"https://route53.amazonaws.com/doc/2013-04-01/\">\
<ChangeBatch><Changes><Change><Action>{action}</Action><ResourceRecordSet>\
<Name>{}.</Name><Type>TXT</Type><TTL>{TXT_TTL}</TTL><ResourceRecords>{records}</ResourceRecords>\
</ResourceRecordSet></Change></Changes></ChangeBatch></ChangeResourceRecordSetsRequest>",
            fqdn.trim_end_matches('.')
        );
        self.send(
            reqwest::Method::POST,
            &format!("/2013-04-01/hostedzone/{zone}/rrset"),
            &[],
            Some(body),
        )
        .await?;
        Ok(())
    }
}

/// RFC 3986 unreserved-only percent encoding, as SigV4 requires.
fn aws_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[async_trait]
impl DnsApi for Route53 {
    async fn create_txt(&self, fqdn: &str, value: &str) -> Result<String> {
        let zone = self.zone(fqdn).await?;
        let values = {
            let mut map = self.values.lock();
            let entry = map.entry(fqdn.to_string()).or_default();
            entry.push(value.to_string());
            entry.clone()
        };
        self.change(&zone, "UPSERT", fqdn, &values).await?;
        Ok(format!("{zone}|{value}"))
    }

    async fn delete_txt(&self, fqdn: &str, record_id: &str) -> Result<()> {
        let (zone, value) = record_id
            .split_once('|')
            .context("Malformed Route 53 record id")?;
        let (before, after) = {
            let mut map = self.values.lock();
            let entry = map.entry(fqdn.to_string()).or_default();
            let before = entry.clone();
            entry.retain(|v| v != value);
            (before, entry.clone())
        };
        if after.is_empty() {
            self.change(zone, "DELETE", fqdn, &before).await
        } else {
            self.change(zone, "UPSERT", fqdn, &after).await
        }
    }

    async fn verify(&self) -> Result<String> {
        let xml = self
            .send(
                reqwest::Method::GET,
                "/2013-04-01/hostedzone",
                &[("maxitems", "50")],
                None,
            )
            .await?;
        let names: Vec<&str> = xml_tag(&xml, "Name")
            .into_iter()
            .map(|n| n.trim_end_matches('.'))
            .collect();
        Ok(describe_zones(&names))
    }
}

/// Build the API client for a provider. `base` overrides the endpoint (tests).
pub fn build_api(
    provider: &str,
    creds: &DnsCredentials,
    base: Option<&str>,
) -> Result<Box<dyn DnsApi>> {
    creds.validate(provider).map_err(|e| anyhow::anyhow!(e))?;
    let token = creds.api_token.as_deref().unwrap_or_default().trim();
    Ok(match provider {
        "cloudflare" => Box::new(Cloudflare::new(token, base)),
        "digitalocean" => Box::new(DigitalOcean::new(token, base)),
        "hetzner" => Box::new(Hetzner::new(token, base)),
        "route53" => Box::new(Route53::new(creds, base)),
        other => bail!("Unknown DNS provider: {other}"),
    })
}

// ---------------------------------------------------------------------------
// DNS-01 solver
// ---------------------------------------------------------------------------

/// Publishes TXT records through a provider API and waits until the
/// configured resolvers return them.
pub struct ProviderSolver {
    api: Box<dyn DnsApi>,
    resolvers: Vec<IpAddr>,
    timeout: Duration,
}

impl ProviderSolver {
    pub fn new(api: Box<dyn DnsApi>, resolvers: &[String]) -> Self {
        let mut ips: Vec<IpAddr> = resolvers
            .iter()
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if ips.is_empty() {
            ips = DEFAULT_RESOLVERS
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
        }
        Self {
            api,
            resolvers: ips,
            timeout: PROPAGATION_TIMEOUT,
        }
    }
}

async fn txt_values(resolver: IpAddr, fqdn: &str) -> Result<Vec<String>> {
    use hickory_resolver::config::{NameServerConfig, ResolverConfig};
    use hickory_resolver::net::runtime::TokioRuntimeProvider;
    use hickory_resolver::proto::rr::RData;
    use hickory_resolver::Resolver;

    let config = ResolverConfig::from_name_servers(vec![NameServerConfig::udp_and_tcp(resolver)]);
    let mut builder = Resolver::builder_with_config(config, TokioRuntimeProvider::default());
    builder.options_mut().timeout = Duration::from_secs(3);
    builder.options_mut().attempts = 2;
    // Never answer from a cache: we poll for a record that is about to appear.
    builder.options_mut().cache_size = 0;
    let resolver = builder.build()?;
    let name = format!("{}.", fqdn.trim_end_matches('.'));
    let lookup = resolver.txt_lookup(name.as_str()).await?;
    Ok(lookup
        .answers()
        .iter()
        .filter_map(|r| match &r.data {
            RData::TXT(txt) => Some(
                txt.txt_data
                    .iter()
                    .map(|part| String::from_utf8_lossy(part).to_string())
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect())
}

#[async_trait]
impl Dns01Solver for ProviderSolver {
    async fn present(&self, fqdn: &str, value: &str) -> Result<String> {
        self.api.create_txt(fqdn, value).await
    }

    async fn wait_for_propagation(&self, fqdn: &str, value: &str) -> Result<()> {
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            let mut all = true;
            for r in &self.resolvers {
                match txt_values(*r, fqdn).await {
                    Ok(values) if values.iter().any(|v| v == value) => {}
                    Ok(_) => all = false,
                    Err(e) => {
                        debug!(resolver = %r, error = %e, "TXT lookup failed");
                        all = false;
                    }
                }
            }
            if all {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                bail!(
                    "not visible on {} after {}s",
                    self.resolvers
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    self.timeout.as_secs()
                );
            }
            tokio::time::sleep(PROPAGATION_POLL).await;
        }
    }

    async fn cleanup(&self, fqdn: &str, record_id: &str) -> Result<()> {
        self.api.delete_txt(fqdn, record_id).await
    }
}

/// Solver for a stored provider, using the resolvers from Settings > Advanced.
pub async fn solver_for(
    db: &SqlitePool,
    key: Option<&[u8; 32]>,
    provider_id: &str,
) -> Result<ProviderSolver> {
    let row = get(db, provider_id)
        .await?
        .with_context(|| format!("DNS provider {provider_id} no longer exists"))?;
    let creds = credentials(&row, key)?;
    let api = build_api(&row.provider, &creds, None)?;
    let resolvers = crate::settings::advanced::current()
        .settings
        .dns_servers
        .clone();
    Ok(ProviderSolver::new(api, &resolvers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        extract::State,
        http::{HeaderMap, Method, Uri},
        response::IntoResponse,
        Router,
    };
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct Seen {
        method: String,
        uri: String,
        auth: String,
        body: String,
    }

    type Log = Arc<Mutex<Vec<Seen>>>;

    /// A local HTTP server that records requests and answers from `reply`.
    async fn mock(reply: fn(&str, &str) -> (u16, String)) -> (String, Log) {
        let log: Log = Default::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new().fallback(
            move |State(log): State<Log>,
                  method: Method,
                  uri: Uri,
                  headers: HeaderMap,
                  body: Bytes| async move {
                let seen = Seen {
                    method: method.to_string(),
                    uri: uri.to_string(),
                    auth: headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default()
                        .to_string(),
                    body: String::from_utf8_lossy(&body).to_string(),
                };
                let (status, body) = reply(&seen.method, &seen.uri);
                log.lock().push(seen);
                (axum::http::StatusCode::from_u16(status).unwrap(), body).into_response()
            },
        );
        let app = app.with_state(log.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (base, log)
    }

    #[test]
    fn zone_candidates_and_relative_names() {
        assert_eq!(
            zone_candidates("_acme-challenge.app.example.com"),
            vec!["app.example.com", "example.com"]
        );
        assert_eq!(
            relative_name("_acme-challenge.app.example.com", "example.com"),
            "_acme-challenge.app"
        );
        assert_eq!(relative_name("example.com", "example.com"), "@");
    }

    #[test]
    fn credentials_are_checked_per_provider() {
        let token = DnsCredentials {
            api_token: Some("t".into()),
            ..Default::default()
        };
        assert!(token.validate("cloudflare").is_ok());
        assert!(token.validate("route53").is_err());
        assert!(DnsCredentials::default().validate("hetzner").is_err());
        assert!(token.validate("bogus").is_err());
    }

    #[test]
    fn sigv4_matches_the_aws_test_suite() {
        // get-vanilla from the AWS SigV4 test suite.
        let auth = sigv4_authorization(
            "AKIDEXAMPLE",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "us-east-1",
            "service",
            "GET",
            "example.amazonaws.com",
            "/",
            "",
            "20150830T123600Z",
            b"",
        );
        assert_eq!(
            auth,
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
             SignedHeaders=host;x-amz-date, \
             Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    #[tokio::test]
    async fn cloudflare_finds_zone_then_creates_and_deletes() {
        let (base, log) = mock(|method, uri| match (method, uri) {
            ("GET", u) if u.contains("name=app.example.com") => {
                (200, r#"{"success":true,"result":[]}"#.into())
            }
            ("GET", u) if u.contains("name=example.com") => {
                (200, r#"{"success":true,"result":[{"id":"z1"}]}"#.into())
            }
            ("POST", _) => (200, r#"{"success":true,"result":{"id":"r9"}}"#.into()),
            ("DELETE", _) => (200, r#"{"success":true,"result":{"id":"r9"}}"#.into()),
            _ => (404, "{}".into()),
        })
        .await;
        let api = Cloudflare::new("tok", Some(&base));
        let id = api
            .create_txt("_acme-challenge.app.example.com", "v1")
            .await
            .unwrap();
        assert_eq!(id, "z1/r9");
        api.delete_txt("_acme-challenge.app.example.com", &id)
            .await
            .unwrap();

        let log = log.lock();
        assert_eq!(log.len(), 4);
        assert!(log.iter().all(|s| s.auth == "Bearer tok"));
        assert_eq!(log[2].method, "POST");
        assert_eq!(log[2].uri, "/zones/z1/dns_records");
        let body: Value = serde_json::from_str(&log[2].body).unwrap();
        assert_eq!(
            body,
            json!({"type":"TXT","name":"_acme-challenge.app.example.com","content":"v1","ttl":60})
        );
        assert_eq!(log[3].method, "DELETE");
        assert_eq!(log[3].uri, "/zones/z1/dns_records/r9");
    }

    #[tokio::test]
    async fn digitalocean_uses_relative_names() {
        let (base, log) = mock(|method, uri| match (method, uri) {
            ("GET", "/v2/domains/example.com") => {
                (200, r#"{"domain":{"name":"example.com"}}"#.into())
            }
            ("GET", _) => (404, r#"{"id":"not_found"}"#.into()),
            ("POST", _) => (201, r#"{"domain_record":{"id":42}}"#.into()),
            ("DELETE", _) => (204, String::new()),
            _ => (500, String::new()),
        })
        .await;
        let api = DigitalOcean::new("tok", Some(&base));
        let id = api
            .create_txt("_acme-challenge.example.com", "v")
            .await
            .unwrap();
        assert_eq!(id, "example.com/42");
        api.delete_txt("_acme-challenge.example.com", &id)
            .await
            .unwrap();
        let log = log.lock();
        let post = log.iter().find(|s| s.method == "POST").unwrap();
        assert_eq!(post.uri, "/v2/domains/example.com/records");
        let body: Value = serde_json::from_str(&post.body).unwrap();
        assert_eq!(body["name"], "_acme-challenge");
        assert_eq!(body["data"], "v");
        assert_eq!(
            log.last().unwrap().uri,
            "/v2/domains/example.com/records/42"
        );
    }

    #[tokio::test]
    async fn hetzner_adds_and_removes_quoted_records() {
        let (base, log) = mock(|method, uri| match (method, uri) {
            ("GET", u) if u.contains("name=example.com") => {
                (200, r#"{"zones":[{"id":1,"name":"example.com"}]}"#.into())
            }
            ("GET", _) => (200, r#"{"zones":[]}"#.into()),
            ("POST", _) => (201, r#"{"action":{"id":1}}"#.into()),
            _ => (500, String::new()),
        })
        .await;
        let api = Hetzner::new("tok", Some(&base));
        let id = api
            .create_txt("_acme-challenge.shop.example.com", "v")
            .await
            .unwrap();
        api.delete_txt("_acme-challenge.shop.example.com", &id)
            .await
            .unwrap();
        let log = log.lock();
        let posts: Vec<_> = log.iter().filter(|s| s.method == "POST").collect();
        assert_eq!(
            posts[0].uri,
            "/zones/example.com/rrsets/_acme-challenge.shop/TXT/actions/add_records"
        );
        assert_eq!(
            posts[1].uri,
            "/zones/example.com/rrsets/_acme-challenge.shop/TXT/actions/remove_records"
        );
        let body: Value = serde_json::from_str(&posts[0].body).unwrap();
        assert_eq!(body["records"][0]["value"], "\"v\"");
    }

    #[tokio::test]
    async fn route53_signs_requests_and_keeps_both_values() {
        let (base, log) = mock(|method, uri| match (method, uri) {
            ("GET", u) if u.contains("dnsname=example.com") => (
                200,
                "<ListHostedZonesByNameResponse><HostedZones><HostedZone>\
                 <Id>/hostedzone/Z123</Id><Name>example.com.</Name></HostedZone>\
                 </HostedZones></ListHostedZonesByNameResponse>"
                    .into(),
            ),
            ("GET", _) => (200, "<HostedZones></HostedZones>".into()),
            ("POST", _) => (200, "<ChangeInfo><Id>c</Id></ChangeInfo>".into()),
            _ => (500, String::new()),
        })
        .await;
        let creds = DnsCredentials {
            access_key_id: Some("AKID".into()),
            secret_access_key: Some("secret".into()),
            ..Default::default()
        };
        let api = Route53::new(&creds, Some(&base));
        let fqdn = "_acme-challenge.example.com";
        let a = api.create_txt(fqdn, "one").await.unwrap();
        let b = api.create_txt(fqdn, "two").await.unwrap();
        api.delete_txt(fqdn, &a).await.unwrap();
        api.delete_txt(fqdn, &b).await.unwrap();

        let log = log.lock();
        assert!(log
            .iter()
            .all(|s| s.auth.starts_with("AWS4-HMAC-SHA256 Credential=AKID/")
                && s.auth.contains("/us-east-1/route53/aws4_request")));
        let posts: Vec<_> = log.iter().filter(|s| s.method == "POST").collect();
        assert_eq!(posts.len(), 4);
        assert!(posts
            .iter()
            .all(|p| p.uri == "/2013-04-01/hostedzone/Z123/rrset"));
        // Second UPSERT publishes both values; the last change deletes the set.
        assert!(posts[1].body.contains("<Value>\"one\"</Value>"));
        assert!(posts[1].body.contains("<Value>\"two\"</Value>"));
        assert!(posts[2].body.contains("<Action>UPSERT</Action>"));
        assert!(!posts[2].body.contains("\"one\""));
        assert!(posts[3].body.contains("<Action>DELETE</Action>"));
        assert!(posts[3].body.contains("<Value>\"two\"</Value>"));
    }

    #[tokio::test]
    async fn provider_errors_surface_the_response() {
        let (base, _log) = mock(|_, _| {
            (
                403,
                r#"{"success":false,"errors":[{"message":"Invalid API Token"}]}"#.into(),
            )
        })
        .await;
        let err = Cloudflare::new("bad", Some(&base))
            .verify()
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Invalid API Token"), "{err}");
    }
}
