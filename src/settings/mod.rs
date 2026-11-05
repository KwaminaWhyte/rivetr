//! Runtime instance settings stored in the `instance_settings` key/value table.
//!
//! Each submodule owns a group of keys, applies defaults (usually from
//! `rivetr.toml`) and exposes pure helpers so precedence rules are unit tested.

pub mod advanced;
pub mod dns;
pub mod email;
pub mod net;
pub mod updates;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use sqlx::SqlitePool;

pub(crate) const KEY_PUBLIC_IPV4: &str = "public_ipv4";
pub(crate) const KEY_PUBLIC_IPV6: &str = "public_ipv6";

/// Parse a stored boolean ("true"/"false"/"1"/"0"). Anything else is `None`.
pub fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Load settings caches at startup. Call once after the DB is migrated.
pub async fn init(db: &SqlitePool) {
    if let Err(e) = advanced::reload(db).await {
        tracing::warn!(error = %e, "Failed to load advanced instance settings; using defaults");
    }
}

/// Save the public IPs shown on the General page. Empty strings clear a value.
pub async fn save_public_ips(
    db: &SqlitePool,
    ipv4: Option<&str>,
    ipv6: Option<&str>,
) -> Result<(), String> {
    if let Some(v4) = ipv4.map(str::trim).filter(|v| !v.is_empty()) {
        v4.parse::<Ipv4Addr>()
            .map_err(|_| format!("'{v4}' is not a valid IPv4 address"))?;
    }
    if let Some(v6) = ipv6.map(str::trim).filter(|v| !v.is_empty()) {
        v6.parse::<Ipv6Addr>()
            .map_err(|_| format!("'{v6}' is not a valid IPv6 address"))?;
    }
    for (key, value) in [(KEY_PUBLIC_IPV4, ipv4), (KEY_PUBLIC_IPV6, ipv6)] {
        if let Some(v) = value {
            let v = v.trim();
            crate::db::set_setting_value(db, key, (!v.is_empty()).then_some(v))
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Ask public "what is my IP" services for this host's IPv4 and IPv6 address.
/// Each family is forced by binding the local socket to that family, and each
/// lookup gives up after a few seconds.
pub async fn detect_public_ips() -> (Option<String>, Option<String>) {
    let v4 = detect_family(
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        &["https://api4.ipify.org", "https://ipv4.icanhazip.com"],
    );
    let v6 = detect_family(
        IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        &["https://api6.ipify.org", "https://ipv6.icanhazip.com"],
    );
    let (v4, v6) = tokio::join!(v4, v6);
    (
        v4.filter(|ip| ip.is_ipv4()).map(|ip| ip.to_string()),
        v6.filter(|ip| ip.is_ipv6()).map(|ip| ip.to_string()),
    )
}

async fn detect_family(bind: IpAddr, urls: &[&str]) -> Option<IpAddr> {
    let client = reqwest::Client::builder()
        .local_address(bind)
        .timeout(Duration::from_secs(4))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .ok()?;
    for url in urls {
        if let Ok(res) = client.get(*url).send().await {
            if let Ok(body) = res.text().await {
                if let Ok(ip) = body.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_booleans() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool(" 0 "), Some(false));
        assert_eq!(parse_bool("maybe"), None);
    }
}
