//! Hostname resolution for domain checks, optionally through admin-chosen
//! nameservers instead of the OS resolver.

use std::net::IpAddr;
use std::time::Duration;

use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::Resolver;

/// Resolve `host` to its A/AAAA addresses. With an empty `nameservers` list
/// the OS resolver is used; otherwise only the given servers are queried.
pub async fn resolve_host(host: &str, nameservers: &[String]) -> anyhow::Result<Vec<IpAddr>> {
    let servers: Vec<IpAddr> = nameservers
        .iter()
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let mut ips: Vec<IpAddr> = if servers.is_empty() {
        tokio::net::lookup_host((host, 80))
            .await?
            .map(|sa| sa.ip())
            .collect()
    } else {
        let config = ResolverConfig::from_name_servers(
            servers
                .into_iter()
                .map(NameServerConfig::udp_and_tcp)
                .collect(),
        );
        let mut builder = Resolver::builder_with_config(config, TokioRuntimeProvider::default());
        builder.options_mut().timeout = Duration::from_secs(3);
        builder.options_mut().attempts = 2;
        let resolver = builder.build()?;
        // A trailing dot makes the name fully qualified (no search domains).
        let fqdn = if host.ends_with('.') {
            host.to_string()
        } else {
            format!("{host}.")
        };
        resolver.lookup_ip(fqdn.as_str()).await?.iter().collect()
    };

    ips.sort();
    ips.dedup();
    Ok(ips)
}
