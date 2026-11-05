//! DNS provider credentials for wildcard certificates (ACME DNS-01), and the
//! instance wildcard certificate setting.
//!
//! Providers are instance-wide: any signed-in user can list them (names only)
//! to pick one for a domain; creating, changing, verifying and deleting them
//! and the wildcard setting are admin only.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db::User;
use crate::proxy::certs::{self, dns};
use crate::AppState;

use super::audit::{audit_log, ClientIp};
use super::authz::is_privileged_user;
use super::error::ApiError;

fn require_admin(user: &User) -> Result<(), ApiError> {
    if is_privileged_user(user) {
        Ok(())
    } else {
        Err(ApiError::forbidden("Admin access required"))
    }
}

fn key(state: &AppState) -> Option<[u8; 32]> {
    state
        .config
        .auth
        .encryption_key
        .as_deref()
        .map(crate::crypto::derive_key)
}

/// Fail with 400 unless `id` names a stored provider.
pub async fn require_provider(db: &SqlitePool, id: &str) -> Result<(), ApiError> {
    match dns::get(db, id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(ApiError::bad_request("Unknown DNS provider")),
        Err(e) => Err(ApiError::database(e.to_string())),
    }
}

#[derive(Debug, Serialize)]
pub struct DnsProviderView {
    pub id: String,
    pub name: String,
    pub provider: String,
    /// Non-secret hints (Route 53 region and zone, last four of the key id).
    pub summary: String,
    /// Domains whose certificates use this provider.
    pub used_by: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

async fn view(state: &AppState, row: dns::DnsProviderRow) -> Result<DnsProviderView, ApiError> {
    let k = key(state);
    let summary = match dns::credentials(&row, k.as_ref()) {
        Ok(c) if row.provider == "route53" => {
            let akid = c.access_key_id.unwrap_or_default();
            let tail: String = akid
                .chars()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!(
                "Access key ...{tail}, region {}",
                c.region.unwrap_or_else(|| "us-east-1".into())
            )
        }
        Ok(_) => "API token".to_string(),
        Err(_) => "Credentials unreadable (encryption key changed?)".to_string(),
    };
    let mut used_by: Vec<String> = sqlx::query_scalar(
        "SELECT domain FROM domain_certificates WHERE mode = 'dns' AND dns_provider_id = ?",
    )
    .bind(&row.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    if let Some(w) = certs::manager::load_wildcard_setting(&state.db).await {
        if w.provider_id == row.id {
            used_by.insert(0, w.domain);
        }
    }
    Ok(DnsProviderView {
        id: row.id,
        name: row.name,
        provider: row.provider,
        summary,
        used_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

/// GET /api/dns-providers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    _user: User,
) -> Result<Json<Vec<DnsProviderView>>, ApiError> {
    let rows = dns::list(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(view(&state, row).await?);
    }
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct CreateDnsProvider {
    pub name: String,
    pub provider: String,
    pub credentials: dns::DnsCredentials,
}

/// POST /api/dns-providers
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Json(req): Json<CreateDnsProvider>,
) -> Result<(StatusCode, Json<DnsProviderView>), ApiError> {
    require_admin(&user)?;
    let name = req.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::bad_request("Name must be 1 to 100 characters"));
    }
    req.credentials
        .validate(&req.provider)
        .map_err(ApiError::bad_request)?;
    let sealed = dns::seal(&req.credentials, key(&state).as_ref())
        .map_err(|e| ApiError::internal(format!("Could not encrypt credentials: {e}")))?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO dns_providers (id, name, provider, credentials) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(&req.provider)
        .bind(&sealed)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    audit_log(
        &state,
        "dns_provider.create",
        "dns_provider",
        Some(&id),
        Some(name),
        Some(&user.id),
        ip.as_deref(),
        Some(serde_json::json!({ "provider": req.provider })),
    )
    .await;
    let row = dns::get(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::internal("Provider vanished after insert"))?;
    Ok((StatusCode::CREATED, Json(view(&state, row).await?)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDnsProvider {
    pub name: Option<String>,
    /// Replaces the stored credentials when present.
    pub credentials: Option<dns::DnsCredentials>,
}

/// PUT /api/dns-providers/{id}
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path(id): Path<String>,
    Json(req): Json<UpdateDnsProvider>,
) -> Result<Json<DnsProviderView>, ApiError> {
    require_admin(&user)?;
    let row = dns::get(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("DNS provider not found"))?;
    let name = match req.name.as_deref().map(str::trim) {
        Some(n) if n.is_empty() || n.len() > 100 => {
            return Err(ApiError::bad_request("Name must be 1 to 100 characters"))
        }
        Some(n) => n.to_string(),
        None => row.name.clone(),
    };
    let sealed = match req.credentials {
        Some(c) => {
            c.validate(&row.provider).map_err(ApiError::bad_request)?;
            dns::seal(&c, key(&state).as_ref())
                .map_err(|e| ApiError::internal(format!("Could not encrypt credentials: {e}")))?
        }
        None => row.credentials.clone(),
    };
    sqlx::query(
        "UPDATE dns_providers SET name = ?, credentials = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&name)
    .bind(&sealed)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    audit_log(
        &state,
        "dns_provider.update",
        "dns_provider",
        Some(&id),
        Some(&name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
    let row = dns::get(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("DNS provider not found"))?;
    Ok(Json(view(&state, row).await?))
}

/// DELETE /api/dns-providers/{id}
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&user)?;
    let row = dns::get(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("DNS provider not found"))?;
    let used = view(&state, row.clone()).await?.used_by;
    if !used.is_empty() {
        return Err(ApiError::bad_request(format!(
            "In use by {}; switch those certificates to another option first",
            used.join(", ")
        )));
    }
    sqlx::query("DELETE FROM dns_providers WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?;
    audit_log(
        &state,
        "dns_provider.delete",
        "dns_provider",
        Some(&id),
        Some(&row.name),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub ok: bool,
    pub message: String,
}

/// POST /api/dns-providers/{id}/verify: call the provider with the stored credentials.
pub async fn verify_provider(
    State(state): State<Arc<AppState>>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<VerifyResult>, ApiError> {
    require_admin(&user)?;
    let row = dns::get(&state.db, &id)
        .await
        .map_err(|e| ApiError::database(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("DNS provider not found"))?;
    let result = async {
        let creds = dns::credentials(&row, key(&state).as_ref())?;
        dns::build_api(&row.provider, &creds, None)?.verify().await
    }
    .await;
    Ok(Json(match result {
        Ok(message) => VerifyResult { ok: true, message },
        Err(e) => VerifyResult {
            ok: false,
            message: format!("{e:#}"),
        },
    }))
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WildcardCertificate {
    /// `*.apps.example.com`, or empty to turn it off.
    pub domain: Option<String>,
    pub dns_provider_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WildcardCertificateView {
    pub domain: Option<String>,
    pub dns_provider_id: Option<String>,
    /// `*.<proxy.base_domain>`, the usual choice for auto-generated domains.
    pub suggested_domain: Option<String>,
    pub status: Option<String>,
    pub not_after: Option<String>,
    pub last_error: Option<String>,
}

async fn wildcard_view(state: &AppState) -> Result<WildcardCertificateView, ApiError> {
    let setting = certs::manager::load_wildcard_setting(&state.db).await;
    let row = match &setting {
        Some(w) => certs::db::get(&state.db, &w.domain)
            .await
            .map_err(|e| ApiError::database(e.to_string()))?,
        None => None,
    };
    Ok(WildcardCertificateView {
        domain: setting.as_ref().map(|w| w.domain.clone()),
        dns_provider_id: setting.map(|w| w.provider_id),
        suggested_domain: state
            .config
            .proxy
            .base_domain
            .as_deref()
            .filter(|d| !d.is_empty())
            .map(|d| format!("*.{}", d.trim_start_matches("*."))),
        status: row.as_ref().map(|r| r.status.clone()),
        not_after: row.as_ref().and_then(|r| r.not_after.clone()),
        last_error: row.and_then(|r| r.last_error),
    })
}

/// GET /api/certificates/wildcard
pub async fn get_wildcard(
    State(state): State<Arc<AppState>>,
    user: User,
) -> Result<Json<WildcardCertificateView>, ApiError> {
    require_admin(&user)?;
    Ok(Json(wildcard_view(&state).await?))
}

/// Normalize `apps.example.com` or `*.apps.example.com` to the wildcard form.
fn normalize_wildcard(input: &str) -> Result<Option<String>, ApiError> {
    let d = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if d.is_empty() {
        return Ok(None);
    }
    let d = if d.starts_with("*.") {
        d
    } else {
        format!("*.{d}")
    };
    if !certs::is_certifiable(&d) || d.matches('.').count() < 2 {
        return Err(ApiError::bad_request(
            "Use a wildcard under a domain you control, such as *.apps.example.com",
        ));
    }
    Ok(Some(d))
}

async fn set_setting(db: &SqlitePool, key: &str, value: Option<&str>) -> Result<(), ApiError> {
    sqlx::query(
        "INSERT INTO instance_settings (key, value, updated_at) VALUES (?, ?, datetime('now')) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await
    .map_err(|e| ApiError::database(e.to_string()))?;
    Ok(())
}

/// PUT /api/certificates/wildcard
pub async fn update_wildcard(
    State(state): State<Arc<AppState>>,
    user: User,
    ip: ClientIp,
    Json(req): Json<WildcardCertificate>,
) -> Result<Json<WildcardCertificateView>, ApiError> {
    require_admin(&user)?;
    let domain = normalize_wildcard(req.domain.as_deref().unwrap_or(""))?;
    let provider = req.dns_provider_id.filter(|p| !p.is_empty());
    if domain.is_some() {
        let pid = provider
            .as_deref()
            .ok_or_else(|| ApiError::bad_request("Choose a DNS provider"))?;
        require_provider(&state.db, pid).await?;
    }
    set_setting(&state.db, certs::KEY_WILDCARD_DOMAIN, domain.as_deref()).await?;
    set_setting(
        &state.db,
        certs::KEY_WILDCARD_PROVIDER,
        domain.as_ref().and(provider.as_deref()),
    )
    .await?;
    if let Some(rt) = certs::runtime() {
        rt.wake();
    }
    audit_log(
        &state,
        "settings.update",
        "instance_settings",
        Some("wildcard_certificate"),
        domain.as_deref(),
        Some(&user.id),
        ip.as_deref(),
        None,
    )
    .await;
    Ok(Json(wildcard_view(&state).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_input_is_normalized() {
        assert_eq!(
            normalize_wildcard("Apps.Example.com.").unwrap().as_deref(),
            Some("*.apps.example.com")
        );
        assert_eq!(
            normalize_wildcard("*.apps.example.com").unwrap().as_deref(),
            Some("*.apps.example.com")
        );
        assert_eq!(normalize_wildcard("  ").unwrap(), None);
        assert!(normalize_wildcard("com").is_err());
        assert!(normalize_wildcard("*.x.sslip.io").is_err());
    }
}
