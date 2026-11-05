//! Small IP / CIDR helpers used by the API allowlist and the outbound policy.

use std::net::IpAddr;

/// An IP network written as `addr/prefix`. A bare address is a /32 or /128.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpCidr {
    addr: IpAddr,
    prefix: u8,
}

impl IpCidr {
    /// Parse `1.2.3.4`, `10.0.0.0/8`, `::1` or `fd00::/8`.
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        let (addr_part, prefix_part) = match raw.split_once('/') {
            Some((a, p)) => (a, Some(p)),
            None => (raw, None),
        };
        let addr: IpAddr = addr_part.trim().parse().ok()?;
        let max = if addr.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix_part {
            Some(p) => p.trim().parse::<u8>().ok().filter(|p| *p <= max)?,
            None => max,
        };
        Some(Self { addr, prefix })
    }

    /// True when `ip` is inside this network. IPv4-mapped IPv6 addresses
    /// (`::ffff:1.2.3.4`) are compared as IPv4.
    pub fn contains(&self, ip: &IpAddr) -> bool {
        let ip = normalize(ip);
        match (self.addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = mask_u32(self.prefix);
                (u32::from(net) & mask) == (u32::from(ip) & mask)
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = mask_u128(self.prefix);
                (u128::from(net) & mask) == (u128::from(ip) & mask)
            }
            _ => false,
        }
    }
}

fn normalize(ip: &IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(*v6)),
        v4 => *v4,
    }
}

fn mask_u32(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix))
    }
}

fn mask_u128(prefix: u8) -> u128 {
    if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - u32::from(prefix))
    }
}

/// Split a free-form list (newlines, commas or spaces) into trimmed entries.
pub fn split_list(raw: &str) -> Vec<String> {
    raw.split([',', '\n', ' ', '\t', '\r'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parse every entry, returning the first invalid one as an error.
pub fn parse_cidrs(entries: &[String]) -> Result<Vec<IpCidr>, String> {
    entries
        .iter()
        .map(|e| IpCidr::parse(e).ok_or_else(|| e.clone()))
        .collect()
}

/// Is `ip` allowed by `allowlist`? An empty allowlist allows every source.
pub fn ip_allowed(allowlist: &[IpCidr], ip: &IpAddr) -> bool {
    allowlist.is_empty() || allowlist.iter().any(|c| c.contains(ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn parses_addresses_and_networks() {
        assert!(IpCidr::parse("10.0.0.0/8").is_some());
        assert!(IpCidr::parse("1.2.3.4").is_some());
        assert!(IpCidr::parse("fd00::/8").is_some());
        assert!(IpCidr::parse(" ::1 ").is_some());
        assert!(IpCidr::parse("10.0.0.0/33").is_none());
        assert!(IpCidr::parse("not-an-ip").is_none());
        assert!(IpCidr::parse("").is_none());
    }

    #[test]
    fn empty_allowlist_allows_everything() {
        assert!(ip_allowed(&[], &ip("203.0.113.7")));
        assert!(ip_allowed(&[], &ip("::1")));
    }

    #[test]
    fn allowlist_matches_cidrs_and_single_ips() {
        let list = parse_cidrs(&split_list("10.0.0.0/8, 203.0.113.7\n2001:db8::/32")).unwrap();
        assert!(ip_allowed(&list, &ip("10.1.2.3")));
        assert!(ip_allowed(&list, &ip("203.0.113.7")));
        assert!(!ip_allowed(&list, &ip("203.0.113.8")));
        assert!(ip_allowed(&list, &ip("2001:db8:1::5")));
        assert!(!ip_allowed(&list, &ip("2001:db9::1")));
        assert!(!ip_allowed(&list, &ip("192.168.1.1")));
    }

    #[test]
    fn ipv4_mapped_ipv6_matches_ipv4_entries() {
        let list = parse_cidrs(&["192.168.0.0/16".to_string()]).unwrap();
        assert!(ip_allowed(&list, &ip("::ffff:192.168.4.4")));
    }

    #[test]
    fn zero_prefix_matches_whole_family() {
        let list = parse_cidrs(&["0.0.0.0/0".to_string()]).unwrap();
        assert!(ip_allowed(&list, &ip("8.8.8.8")));
        assert!(!ip_allowed(&list, &ip("2001:db8::1")));
    }

    #[test]
    fn invalid_entry_is_reported() {
        let err = parse_cidrs(&["10.0.0.0/8".into(), "nope".into()]).unwrap_err();
        assert_eq!(err, "nope");
    }
}
