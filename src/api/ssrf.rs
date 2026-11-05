//! SSRF egress guard (SEC-H1 / SEC-M1).
//!
//! User-supplied outbound URLs (DockerHub `callback_url`, notification webhooks,
//! log-drain endpoints) must not be able to make Rivetr reach internal,
//! loopback, link-local, or cloud-metadata addresses. `validate_external_url`
//! parses the URL, requires http(s), and rejects any host that is — or resolves
//! to — a disallowed range. Resolving the hostname here also blocks the common
//! "public DNS name → 169.254.169.254" trick (best-effort; not a full
//! DNS-rebinding defense, which also needs the HTTP client to pin the checked IP).

//!
//! Admins can relax the guard in Settings > Advanced > Outbound: specific
//! internal CIDRs can be allowed, and localhost can be allowed.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::settings::net::IpCidr;

use super::error::ApiError;

/// Admin-configured exceptions to the internal-address block.
#[derive(Debug, Clone, Default)]
pub struct OutboundPolicy<'a> {
    pub allowed_targets: &'a [IpCidr],
    pub allow_localhost: bool,
}

/// Is `ip` blocked once the admin policy is applied?
pub fn is_blocked_with_policy(ip: &IpAddr, policy: &OutboundPolicy<'_>) -> bool {
    if !is_blocked_ip(ip) {
        return false;
    }
    if policy.allowed_targets.iter().any(|c| c.contains(ip)) {
        return false;
    }
    if policy.allow_localhost && is_loopback(ip) {
        return false;
    }
    true
}

fn is_loopback(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6
                    .to_ipv4_mapped()
                    .map(|v4| v4.is_loopback())
                    .unwrap_or(false)
        }
    }
}

/// Validate that a user-supplied URL is safe to fetch (not internal). Async
/// because it performs DNS resolution for hostnames.
pub async fn validate_external_url(raw: &str) -> Result<(), ApiError> {
    let url = reqwest::Url::parse(raw).map_err(|_| ApiError::bad_request("Invalid URL"))?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(ApiError::bad_request("URL scheme must be http or https")),
    }

    let host = url
        .host_str()
        .ok_or_else(|| ApiError::bad_request("URL has no host"))?;
    let port = url.port_or_known_default().unwrap_or(443);

    let cached = crate::settings::advanced::current();
    let policy = OutboundPolicy {
        allowed_targets: &cached.outbound_allowlist,
        allow_localhost: cached.settings.outbound_allow_localhost,
    };
    // Url::host_str keeps the brackets around IPv6 literals.
    let host = host.trim_start_matches('[').trim_end_matches(']');

    // Host is an IP literal — check directly, no DNS.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if is_blocked_with_policy(&ip, &policy) {
            Err(ApiError::bad_request(
                "URL points at a disallowed internal address",
            ))
        } else {
            Ok(())
        };
    }

    // Hostname — resolve and reject if ANY address is internal.
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| ApiError::bad_request("Could not resolve URL host"))?;

    let mut resolved_any = false;
    for sa in addrs {
        resolved_any = true;
        if is_blocked_with_policy(&sa.ip(), &policy) {
            return Err(ApiError::bad_request(
                "URL resolves to a disallowed internal address",
            ));
        }
    }
    if !resolved_any {
        return Err(ApiError::bad_request("URL host did not resolve"));
    }
    Ok(())
}

fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

fn is_blocked_v4(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private()            // 10/8, 172.16/12, 192.168/16
        || ip.is_loopback()    // 127/8
        || ip.is_link_local()  // 169.254/16 (cloud metadata)
        || ip.is_broadcast()   // 255.255.255.255
        || ip.is_unspecified() // 0.0.0.0
        || ip.is_documentation()
        || o[0] == 0           // 0.0.0.0/8
        || (o[0] == 100 && (64..128).contains(&o[1])) // CGNAT 100.64/10
}

fn is_blocked_v6(ip: &Ipv6Addr) -> bool {
    let s = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || (s[0] & 0xfe00) == 0xfc00 // unique-local fc00::/7
        || (s[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        // IPv4-mapped / -compatible embedding an internal v4 address
        || ip.to_ipv4_mapped().map(|v4| is_blocked_v4(&v4)).unwrap_or(false)
        || ip.to_ipv4().map(|v4| is_blocked_v4(&v4)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_internal_v4() {
        for ip in [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.4.4",
            "192.168.1.1",
            "169.254.169.254", // cloud metadata
            "0.0.0.0",
            "100.64.0.1",
        ] {
            assert!(
                is_blocked_ip(&ip.parse().unwrap()),
                "{ip} should be blocked"
            );
        }
    }

    #[test]
    fn allows_public_v4() {
        for ip in ["1.1.1.1", "8.8.8.8", "93.184.216.34"] {
            assert!(
                !is_blocked_ip(&ip.parse().unwrap()),
                "{ip} should be allowed"
            );
        }
    }

    #[test]
    fn policy_allows_listed_targets_and_localhost() {
        let targets = vec![IpCidr::parse("10.20.0.0/16").unwrap()];
        let strict = OutboundPolicy::default();
        let relaxed = OutboundPolicy {
            allowed_targets: &targets,
            allow_localhost: true,
        };
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();

        assert!(is_blocked_with_policy(&ip("10.20.1.1"), &strict));
        assert!(!is_blocked_with_policy(&ip("10.20.1.1"), &relaxed));
        // Other internal ranges stay blocked.
        assert!(is_blocked_with_policy(&ip("10.21.1.1"), &relaxed));
        assert!(is_blocked_with_policy(&ip("169.254.169.254"), &relaxed));

        assert!(is_blocked_with_policy(&ip("127.0.0.1"), &strict));
        assert!(!is_blocked_with_policy(&ip("127.0.0.1"), &relaxed));
        assert!(!is_blocked_with_policy(&ip("::1"), &relaxed));
        // Public addresses are never blocked.
        assert!(!is_blocked_with_policy(&ip("8.8.8.8"), &strict));
    }

    #[test]
    fn blocks_internal_v6() {
        for ip in ["::1", "::", "fc00::1", "fe80::1", "::ffff:127.0.0.1"] {
            assert!(
                is_blocked_ip(&ip.parse().unwrap()),
                "{ip} should be blocked"
            );
        }
    }
}
