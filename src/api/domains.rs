/// DNS validation endpoint for custom domains.
///
/// GET /api/domains/check?domain=example.com
///
/// Resolves the provided domain and reports whether it points to this server.
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
}

/// GET /api/domains/check — check whether a domain resolves to this server
pub async fn check_domain_dns(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DnsCheckQuery>,
) -> Json<DnsCheckResponse> {
    let domain = params.domain.trim().to_lowercase();

    // Determine the server's public IP from instance settings or config.
    // We try (in order):
    //   1. `instance_domain` stored in the DB (which is what the user configured as their
    //      Rivetr domain — not directly the IP, but useful for wildcard subdomain setups).
    //   2. A best-effort self-lookup by resolving the instance domain itself.
    //   3. The `server.host` value from rivetr.toml (may be "0.0.0.0").
    //
    // For the most accurate result the operator should configure an explicit IP in instance
    // settings. For now we surface whatever we can determine.
    let server_ip = get_server_ip(&state).await;

    if domain.is_empty() {
        return Json(DnsCheckResponse {
            domain,
            resolves: false,
            points_to_server: false,
            resolved_ips: vec![],
            server_ip,
        });
    }

    // Resolve the domain using the OS resolver (same resolver as the container runtime).
    let lookup_host = format!("{}:80", domain);
    match tokio::net::lookup_host(&lookup_host).await {
        Ok(addrs) => {
            let resolved_ips: Vec<String> = addrs
                .map(|addr| addr.ip().to_string())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let points_to_server = !server_ip.is_empty()
                && resolved_ips.iter().any(|ip| ip == &server_ip);

            Json(DnsCheckResponse {
                domain,
                resolves: !resolved_ips.is_empty(),
                points_to_server,
                resolved_ips,
                server_ip,
            })
        }
        Err(_) => Json(DnsCheckResponse {
            domain,
            resolves: false,
            points_to_server: false,
            resolved_ips: vec![],
            server_ip,
        }),
    }
}

/// Attempt to determine the server's public IP address.
///
/// Priority:
/// 1. If the instance domain is set (e.g. "rivetr.site"), try to resolve it and use the
///    first resulting IP. This handles the case where the operator already configured DNS.
/// 2. Fall back to the `server.host` config value (but not "0.0.0.0" since that is
///    unroutable).
async fn get_server_ip(state: &Arc<AppState>) -> String {
    // Try to load instance settings from the DB.
    if let Ok(settings) = crate::db::InstanceSettings::load(&state.db).await {
        if let Some(ref instance_domain) = settings.instance_domain {
            if !instance_domain.is_empty() {
                // Resolve the instance domain to find this server's IP.
                let lookup = format!("{}:80", instance_domain);
                if let Ok(mut addrs) = tokio::net::lookup_host(&lookup).await {
                    if let Some(addr) = addrs.next() {
                        return addr.ip().to_string();
                    }
                }
            }
        }
    }

    // Fall back to server.host — only useful if it's an actual IP (not 0.0.0.0).
    let host = &state.config.server.host;
    if host != "0.0.0.0" && host != "::" && !host.is_empty() {
        return host.clone();
    }

    String::new()
}
