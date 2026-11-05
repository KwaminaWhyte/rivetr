/// DNS validation endpoint for custom domains.
///
/// GET /api/domains/check?domain=example.com
///
/// Resolves the provided domain and reports whether it points to this server.
/// Settings > Advanced controls whether the check runs at all and which
/// nameservers it uses (the OS resolver when none are set).
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

/// Query parameters for the DNS check endpoint
#[derive(Debug, Deserialize)]
pub struct DnsCheckQuery {
    pub domain: String,
}

/// Response for the DNS check endpoint
#[derive(Debug, Serialize)]
pub struct DnsCheckResponse {
    /// The domain that was checked
    pub domain: String,
    /// Whether the domain resolved to any IP address at all
    pub resolves: bool,
    /// Whether at least one resolved IP matches the configured server IP
    pub points_to_server: bool,
    /// All IP addresses the domain resolved to
    pub resolved_ips: Vec<String>,
    /// The server's configured public IP (or empty string if not configured)
    pub server_ip: String,
    /// All known public IPs of this server (IPv4 and IPv6 when set)
    pub server_ips: Vec<String>,
    /// True when DNS validation is turned off in Settings > Advanced; the
    /// other fields are then empty and callers should not show a warning.
    pub validation_disabled: bool,
}

/// GET /api/domains/check — check whether a domain resolves to this server
pub async fn check_domain_dns(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DnsCheckQuery>,
) -> Json<DnsCheckResponse> {
    let domain = params.domain.trim().trim_end_matches('.').to_lowercase();
    let advanced = crate::settings::advanced::current();

    if !advanced.settings.dns_validation_enabled {
        return Json(DnsCheckResponse {
            domain,
            resolves: false,
            points_to_server: false,
            resolved_ips: vec![],
            server_ip: String::new(),
            server_ips: vec![],
            validation_disabled: true,
        });
    }

    let nameservers = advanced.settings.dns_servers.clone();
    let server_ips = get_server_ips(&state, &nameservers).await;
    let server_ip = server_ips.first().cloned().unwrap_or_default();

    let resolved_ips: Vec<String> = if domain.is_empty() {
        vec![]
    } else {
        crate::settings::dns::resolve_host(&domain, &nameservers)
            .await
            .map(|ips| ips.into_iter().map(|ip| ip.to_string()).collect())
            .unwrap_or_default()
    };

    let points_to_server = resolved_ips.iter().any(|ip| server_ips.contains(ip));

    Json(DnsCheckResponse {
        domain,
        resolves: !resolved_ips.is_empty(),
        points_to_server,
        resolved_ips,
        server_ip,
        server_ips,
        validation_disabled: false,
    })
}

/// Determine this server's public IP addresses.
///
/// Priority:
/// 1. The public IPv4 / IPv6 saved on Settings > General.
/// 2. The addresses the instance domain resolves to.
/// 3. The `server.host` config value (but not "0.0.0.0" since that is
///    unroutable).
async fn get_server_ips(state: &Arc<AppState>, nameservers: &[String]) -> Vec<String> {
    let (v4, v6) = crate::settings::advanced::public_ips(&state.db).await;
    let saved: Vec<String> = [v4, v6].into_iter().flatten().collect();
    if !saved.is_empty() {
        return saved;
    }

    if let Ok(settings) = crate::db::InstanceSettings::load(&state.db).await {
        if let Some(instance_domain) = settings.instance_domain.filter(|d| !d.is_empty()) {
            if let Ok(ips) = crate::settings::dns::resolve_host(&instance_domain, nameservers).await
            {
                if !ips.is_empty() {
                    return ips.into_iter().map(|ip| ip.to_string()).collect();
                }
            }
        }
    }

    // Fall back to server.host — only useful if it's an actual IP (not 0.0.0.0).
    let host = &state.config.server.host;
    if host != "0.0.0.0" && host != "::" && !host.is_empty() {
        return vec![host.clone()];
    }

    Vec::new()
}
