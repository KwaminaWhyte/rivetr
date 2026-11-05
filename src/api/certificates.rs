//! Per-domain TLS certificate status, mode and custom uploads for an app's or
//! a service's domains.
//!
//! Routes live under `/api/apps/{id}/...` and `/api/services/{id}/...`, so the
//! resource authorization middleware already checks access (and the
//! developer role for writes). Handlers only check that the domain belongs to
//! the resource.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::User;
use crate::proxy::certs::{self, db as certdb, CertSource};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::error::ApiError;

/// Certificate state of one domain as shown in the dashboard.
#[derive(Debug, Serialize)]
pub struct DomainCertificateStatus {
    pub domain: String,
    /// `auto` (HTTP-01), `dns` (DNS-01 via `dns_provider_id`) or `custom`.
    pub mode: String,
    pub dns_provider_id: Option<String>,
    /// `issued`, `pending`, `failed`, `custom`, `expiring`, `expired` or `no_https`.
    pub status: String,
    /// Which certificate is served right now: `custom`, `acme`, `legacy` or none.
    pub serving: Option<CertSource>,
    pub issuer: Option<String>,
    pub names: Vec<String>,
    pub not_after: Option<String>,
    pub last_error: Option<String>,
    pub failure_count: i64,
    pub next_attempt_at: Option<String>,
    /// Whether the HTTPS listener is running.
    pub https_enabled: bool,
    /// Whether Let's Encrypt issuance is configured.
    pub acme_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCertificateMode {
    pub mode: String,
    pub dns_provider_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadCertificate {
    /// PEM certificate chain, leaf first.
    pub certificate: String,
    /// PEM private key (PKCS#8, PKCS#1 or SEC1).
    pub private_key: String,
}

/// Domains routed to an app (legacy column, domains JSON, auto subdomain).
async fn app_domains(state: &AppState, app_id: &str) -> Result<Vec<String>, ApiError> {
    let row: Option<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT domain, domains, auto_subdomain FROM apps WHERE id = ?")
            .bind(app_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::database(e.to_string()))?;
    let (legacy, json, auto) = row.ok_or_else(|| ApiError::not_found("App not found"))?;
    let mut out: Vec<String> = Vec::new();
    let mut add = |d: String| {
        let d = d.trim().trim_end_matches('.').to_ascii_lowercase();
        if !d.is_empty() && !out.contains(&d) {
            out.push(d);
        }
    };
    if let Some(d) = legacy {
        add(d);
    }
    if let Some(json) = json {
        if let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&json) {
            for v in values {
                match v {
                    serde_json::Value::String(s) => add(s),
                    serde_json::Value::Object(m) => {
                        if let Some(d) = m.get("domain").and_then(|x| x.as_str()) {
                            add(d.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(d) = auto {
        add(d);
    }
    Ok(out)
}

async fn service_domains(state: &AppState, service_id: &str) -> Result<Vec<String>, ApiError> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT domain FROM services WHERE id = ?")
        .bind(service_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    let (domain,) = row.ok_or_else(|| ApiError::not_found("Service not found"))?;
    Ok(domain
        .map(|d| d.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|d| !d.is_empty())
        .into_iter()
        .collect())
}

#[derive(Clone, Copy)]
enum Resource {
    App,
    Service,
}

impl Resource {
    fn kind(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Service => "service",
        }
    }
}

async fn domains_of(state: &AppState, kind: Resource, id: &str) -> Result<Vec<String>, ApiError> {
    match kind {
        Resource::App => app_domains(state, id).await,
        Resource::Service => service_domains(state, id).await,
    }
}

async fn require_domain(
    state: &AppState,
    kind: Resource,
    id: &str,
    domain: &str,
) -> Result<String, ApiError> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domains_of(state, kind, id).await?.contains(&domain) {
        Ok(domain)
    } else {
        Err(ApiError::not_found(format!(
            "{domain} is not a domain of this {}",
            kind.kind()
        )))
    }
}

async fn status_for(state: &AppState, domain: &str) -> Result<DomainCertificateStatus, ApiError> {
    let row = certdb::get(&state.db, domain)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .unwrap_or_default();
    let runtime = certs::runtime();
    let serving = runtime.as_ref().and_then(|r| r.store.serving(domain));
    let now = Utc::now();
    let mode = if row.mode.is_empty() {
        certdb::MODE_AUTO.to_string()
    } else {
        row.mode.clone()
    };

    let status = match (&runtime, &serving) {
        (None, _) => "no_https",
        (Some(_), Some((_, cert))) if cert.info.not_after <= now => "expired",
        (Some(_), Some((_, cert)))
            if cert.info.not_after - now
                < chrono::Duration::days(certs::CUSTOM_EXPIRY_WARNING_DAYS) =>
        {
            "expiring"
        }
        (Some(_), Some((CertSource::Custom, _))) => "custom",
        (Some(_), Some(_)) if row.status == "failed" => "failed",
        (Some(_), Some(_)) => "issued",
        (Some(_), None) if row.status == "failed" => "failed",
        (Some(_), None) => "pending",
    }
    .to_string();

    Ok(DomainCertificateStatus {
        domain: domain.to_string(),
        mode,
        dns_provider_id: row.dns_provider_id,
        status,
        serving: serving.as_ref().map(|(s, _)| *s),
        issuer: serving.as_ref().map(|(_, c)| c.info.issuer.clone()),
        names: serving
            .as_ref()
            .map(|(_, c)| c.info.names.clone())
            .unwrap_or_default(),
        not_after: serving.as_ref().map(|(_, c)| c.info.not_after.to_rfc3339()),
        last_error: row.last_error,
        failure_count: row.failure_count,
        next_attempt_at: row.next_attempt_at,
        https_enabled: runtime.is_some(),
        acme_enabled: runtime.as_ref().is_some_and(|r| r.acme_enabled),
    })
}

async fn list(
    state: &AppState,
    kind: Resource,
    id: &str,
) -> Result<Json<Vec<DomainCertificateStatus>>, ApiError> {
    let mut out = Vec::new();
    for d in domains_of(state, kind, id).await? {
        out.push(status_for(state, &d).await?);
    }
    Ok(Json(out))
}

async fn update_mode(
    state: &AppState,
    user: &User,
    ip: &ClientIp,
    kind: Resource,
    id: &str,
    domain: &str,
    req: UpdateCertificateMode,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    let domain = require_domain(state, kind, id, domain).await?;
    let provider: Option<String> = match req.mode.as_str() {
        certdb::MODE_AUTO => None,
        certdb::MODE_DNS => {
            let pid = req
                .dns_provider_id
                .filter(|p| !p.is_empty())
                .ok_or_else(|| ApiError::bad_request("Choose a DNS provider"))?;
            super::dns_providers::require_provider(&state.db, &pid).await?;
            Some(pid)
        }
        certdb::MODE_CUSTOM => {
            return Err(ApiError::bad_request(
                "Upload a certificate to use a custom certificate",
            ))
        }
        other => return Err(ApiError::bad_request(format!("Unknown mode: {other}"))),
    };
    let current = certdb::get(&state.db, &domain)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    if current.is_some_and(|r| r.mode == certdb::MODE_CUSTOM) {
        remove_custom_cert(state, &domain).await?;
    }
    certdb::set_mode(&state.db, &domain, &req.mode, provider.as_deref())
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    if let Some(rt) = certs::runtime() {
        rt.wake();
    }
    audit_log(
        state,
        "certificate.mode",
        kind.kind(),
        Some(id),
        Some(&domain),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({ "mode": req.mode })),
    )
    .await;
    Ok(Json(status_for(state, &domain).await?))
}

async fn upload(
    state: &AppState,
    user: &User,
    ip: &ClientIp,
    kind: Resource,
    id: &str,
    domain: &str,
    req: UploadCertificate,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    let domain = require_domain(state, kind, id, domain).await?;
    let loaded =
        certs::store::validate_custom(&req.certificate, &req.private_key, &domain, Utc::now())
            .map_err(ApiError::bad_request)?;
    let key = state
        .config
        .auth
        .encryption_key
        .as_deref()
        .map(crate::crypto::derive_key);
    let stored_key = crate::crypto::encrypt_if_key_available(req.private_key.trim(), key.as_ref())
        .map_err(|e| ApiError::internal(format!("Could not encrypt the private key: {e}")))?;
    certdb::set_custom(
        &state.db,
        &domain,
        req.certificate.trim(),
        &stored_key,
        &loaded.info,
    )
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    if let Some(rt) = certs::runtime() {
        rt.store.insert(CertSource::Custom, &domain, loaded.clone());
    }
    audit_log(
        state,
        "certificate.upload",
        kind.kind(),
        Some(id),
        Some(&domain),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({
            "names": loaded.info.names,
            "not_after": loaded.info.not_after.to_rfc3339(),
        })),
    )
    .await;
    Ok(Json(status_for(state, &domain).await?))
}

async fn remove_custom_cert(state: &AppState, domain: &str) -> Result<(), ApiError> {
    certdb::clear_custom(&state.db, domain)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    if let Some(rt) = certs::runtime() {
        rt.store.remove(CertSource::Custom, domain);
        rt.wake();
    }
    Ok(())
}

async fn delete_custom(
    state: &AppState,
    user: &User,
    ip: &ClientIp,
    kind: Resource,
    id: &str,
    domain: &str,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    let domain = require_domain(state, kind, id, domain).await?;
    remove_custom_cert(state, &domain).await?;
    audit_log(
        state,
        "certificate.remove_custom",
        kind.kind(),
        Some(id),
        Some(&domain),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
    Ok(Json(status_for(state, &domain).await?))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/apps/{id}/certificates
pub async fn list_app_certificates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DomainCertificateStatus>>, ApiError> {
    list(&state, Resource::App, &id).await
}

/// PUT /api/apps/{id}/certificates/{domain}
pub async fn update_app_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
    Json(req): Json<UpdateCertificateMode>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    update_mode(&state, &user, &ip, Resource::App, &id, &domain, req).await
}

/// POST /api/apps/{id}/certificates/{domain}/custom
pub async fn upload_app_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
    Json(req): Json<UploadCertificate>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    upload(&state, &user, &ip, Resource::App, &id, &domain, req).await
}

/// DELETE /api/apps/{id}/certificates/{domain}/custom
pub async fn delete_app_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    delete_custom(&state, &user, &ip, Resource::App, &id, &domain).await
}

/// GET /api/services/{id}/certificates
pub async fn list_service_certificates(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<DomainCertificateStatus>>, ApiError> {
    list(&state, Resource::Service, &id).await
}

/// PUT /api/services/{id}/certificates/{domain}
pub async fn update_service_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
    Json(req): Json<UpdateCertificateMode>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    update_mode(&state, &user, &ip, Resource::Service, &id, &domain, req).await
}

/// POST /api/services/{id}/certificates/{domain}/custom
pub async fn upload_service_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
    Json(req): Json<UploadCertificate>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    upload(&state, &user, &ip, Resource::Service, &id, &domain, req).await
}

/// DELETE /api/services/{id}/certificates/{domain}/custom
pub async fn delete_service_certificate(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path((id, domain)): Path<(String, String)>,
) -> Result<Json<DomainCertificateStatus>, ApiError> {
    delete_custom(&state, &user, &ip, Resource::Service, &id, &domain).await
}
