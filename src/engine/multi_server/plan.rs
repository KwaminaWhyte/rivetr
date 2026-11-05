//! Pure planning for multi-server deployments: which container slots exist,
//! in which rolling batches they are replaced, when a failing rollout is
//! rolled back, and how each server is addressed.

use crate::db::{AppServerPlacement, Server, LOCAL_SERVER_ID, MAX_REPLICAS_PER_SERVER};

/// One container of a multi-server deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSlot {
    pub server_id: String,
    pub replica_index: u32,
}

/// Expand placements into container slots. Slots are interleaved across servers
/// (A0, B0, A1, B1, ...) so a rolling batch never replaces all of one server's
/// containers while another server still has old ones untouched. Placement order
/// is `sort_order`, then the order given. Replicas are clamped to 1..=10 and a
/// server listed twice is only used once.
pub fn plan_slots(placements: &[AppServerPlacement]) -> Vec<TargetSlot> {
    let mut ordered: Vec<&AppServerPlacement> = placements.iter().collect();
    ordered.sort_by_key(|p| p.sort_order);
    let mut servers: Vec<(&str, u32)> = Vec::new();
    for p in ordered {
        if servers.iter().any(|(id, _)| *id == p.server_id) {
            continue;
        }
        let replicas = p.replicas.clamp(1, MAX_REPLICAS_PER_SERVER) as u32;
        servers.push((p.server_id.as_str(), replicas));
    }
    let max = servers.iter().map(|(_, r)| *r).max().unwrap_or(0);
    let mut slots = Vec::new();
    for idx in 0..max {
        for (server_id, replicas) in &servers {
            if idx < *replicas {
                slots.push(TargetSlot {
                    server_id: (*server_id).to_string(),
                    replica_index: idx,
                });
            }
        }
    }
    slots
}

/// Number of distinct servers in a placement list.
pub fn server_count(placements: &[AppServerPlacement]) -> usize {
    let mut ids: Vec<&str> = placements.iter().map(|p| p.server_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.len()
}

/// Split `n` slots into rolling batches of at most `max_unavailable` (at least 1).
pub fn rolling_batches(n: usize, max_unavailable: usize) -> Vec<Vec<usize>> {
    let size = max_unavailable.max(1);
    (0..n)
        .collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.to_vec())
        .collect()
}

/// Whether a rollout with `failed` of `total` slots failing must be rolled back.
/// `threshold_pct` 0 rolls back on any failure; 100 only when every slot failed.
pub fn should_rollback(failed: usize, total: usize, threshold_pct: u8) -> bool {
    if failed == 0 || total == 0 {
        return false;
    }
    if failed >= total {
        return true;
    }
    let threshold = usize::from(threshold_pct.min(100));
    failed * 100 >= threshold * total
}

/// Where a container publishes its port on the target host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindAddr {
    /// A fixed interface address (`127.0.0.1` locally, a Tailscale IP remotely).
    Ip(String),
    /// Look the Tailscale IPv4 up on the server at start time (`tailscale ip -4`),
    /// falling back to all interfaces when it has none.
    TailscaleLookup,
    /// All interfaces.
    All,
}

/// How the Rivetr host reaches one target server's containers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    /// Host part of the upstream address the proxy and health checks use.
    pub upstream_host: String,
    pub bind: BindAddr,
    /// Whether published ports are reachable on a public interface.
    pub public_exposure: bool,
}

/// Addressing for the Rivetr host itself: loopback only.
pub fn local_addressing() -> Addressing {
    Addressing {
        upstream_host: "127.0.0.1".to_string(),
        bind: BindAddr::Ip("127.0.0.1".to_string()),
        public_exposure: false,
    }
}

/// Addressing for a remote server. With `bind_private` and a Tailscale address
/// the port is published on the Tailscale interface only and the proxy connects
/// over the tailnet. Otherwise the port is published on every interface and the
/// proxy uses the address Rivetr reaches the server on (public host or the
/// Tailscale address when the server is set to connect over Tailscale).
pub fn remote_addressing(server: &Server, bind_private: bool) -> Addressing {
    let ts = server
        .tailscale_host
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match ts {
        Some(ts) if bind_private => Addressing {
            upstream_host: ts.to_string(),
            bind: if ts.parse::<std::net::IpAddr>().is_ok() {
                BindAddr::Ip(ts.to_string())
            } else {
                BindAddr::TailscaleLookup
            },
            public_exposure: false,
        },
        _ => Addressing {
            upstream_host: server.ssh_host().to_string(),
            bind: BindAddr::All,
            public_exposure: true,
        },
    }
}

/// Container name for one slot: `rivetr-<app>-ms-<server>-<replica>-<deployment>`.
/// Distinct per deployment so the new container starts next to the old one.
pub fn container_name(
    app_name: &str,
    server_id: &str,
    replica_index: u32,
    deployment_id: &str,
) -> String {
    let short = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(8)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    let server = if server_id == LOCAL_SERVER_ID {
        "local".to_string()
    } else {
        short(server_id)
    };
    let app: String = app_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!(
        "rivetr-{}-ms-{}-{}-{}",
        app,
        server,
        replica_index,
        short(deployment_id)
    )
}

/// Final upstream set after a rollout: new healthy upstreams, plus the previous
/// upstream of every slot whose new container failed (it keeps serving).
pub fn final_upstreams(
    new_healthy: &[String],
    kept_previous: &[String],
    fallback_previous: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for u in new_healthy.iter().chain(kept_previous.iter()) {
        if !out.contains(u) {
            out.push(u.clone());
        }
    }
    if out.is_empty() {
        return fallback_previous.to_vec();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(server: &str, replicas: i64, order: i64) -> AppServerPlacement {
        AppServerPlacement {
            id: format!("p-{server}"),
            app_id: "app".into(),
            server_id: server.into(),
            replicas,
            sort_order: order,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn server(host: &str, mode: &str, ts: Option<&str>) -> Server {
        serde_json::from_value(serde_json::json!({
            "id": "s1", "name": "web-1", "host": host, "port": 22, "username": "root",
            "ssh_private_key": null, "ssh_password": null, "status": "online",
            "last_seen_at": null, "cpu_usage": null, "memory_usage": null, "disk_usage": null,
            "memory_total": null, "disk_total": null, "os_info": null, "docker_version": null,
            "team_id": null, "created_at": "", "updated_at": "",
            "network_mode": mode, "tailscale_host": ts,
        }))
        .unwrap()
    }

    fn ids(slots: &[TargetSlot]) -> Vec<String> {
        slots
            .iter()
            .map(|s| format!("{}#{}", s.server_id, s.replica_index))
            .collect()
    }

    #[test]
    fn slots_interleave_servers_in_sort_order() {
        let slots = plan_slots(&[
            placement("b", 1, 1),
            placement("localhost", 2, 0),
            placement("c", 3, 2),
        ]);
        assert_eq!(
            ids(&slots),
            ["localhost#0", "b#0", "c#0", "localhost#1", "c#1", "c#2"]
        );
    }

    #[test]
    fn slots_clamp_replicas_and_ignore_duplicate_servers() {
        let slots = plan_slots(&[
            placement("a", 0, 0),
            placement("a", 5, 1),
            placement("b", 99, 2),
        ]);
        assert_eq!(slots.iter().filter(|s| s.server_id == "a").count(), 1);
        assert_eq!(slots.iter().filter(|s| s.server_id == "b").count(), 10);
        assert!(plan_slots(&[]).is_empty());
    }

    #[test]
    fn server_count_is_distinct() {
        assert_eq!(
            server_count(&[placement("a", 1, 0), placement("b", 4, 1)]),
            2
        );
        assert_eq!(server_count(&[]), 0);
    }

    #[test]
    fn rolling_batches_respect_max_unavailable() {
        assert_eq!(rolling_batches(5, 2), vec![vec![0, 1], vec![2, 3], vec![4]]);
        assert_eq!(rolling_batches(3, 1), vec![vec![0], vec![1], vec![2]]);
        assert_eq!(rolling_batches(3, 10), vec![vec![0, 1, 2]]);
        // Zero is treated as one so a rollout always progresses.
        assert_eq!(rolling_batches(2, 0), vec![vec![0], vec![1]]);
        assert!(rolling_batches(0, 3).is_empty());
    }

    #[test]
    fn rollback_threshold_decisions() {
        // Nothing failed: never roll back.
        assert!(!should_rollback(0, 4, 0));
        // 0% threshold: any failure rolls back.
        assert!(should_rollback(1, 10, 0));
        // 50%: 1 of 4 is fine, 2 of 4 rolls back.
        assert!(!should_rollback(1, 4, 50));
        assert!(should_rollback(2, 4, 50));
        // 100%: only a total failure rolls back.
        assert!(!should_rollback(3, 4, 100));
        assert!(should_rollback(4, 4, 100));
        // A single target that fails always rolls back.
        assert!(should_rollback(1, 1, 100));
        assert!(!should_rollback(0, 0, 50));
    }

    #[test]
    fn local_addressing_is_loopback_only() {
        let a = local_addressing();
        assert_eq!(a.upstream_host, "127.0.0.1");
        assert_eq!(a.bind, BindAddr::Ip("127.0.0.1".into()));
        assert!(!a.public_exposure);
    }

    #[test]
    fn remote_addressing_prefers_tailscale_when_binding_privately() {
        let s = server("203.0.113.5", "public", Some("100.64.0.7"));
        let a = remote_addressing(&s, true);
        assert_eq!(a.upstream_host, "100.64.0.7");
        assert_eq!(a.bind, BindAddr::Ip("100.64.0.7".into()));
        assert!(!a.public_exposure);

        let s = server("203.0.113.5", "tailscale", Some("web-1.tail1234.ts.net"));
        let a = remote_addressing(&s, true);
        assert_eq!(a.upstream_host, "web-1.tail1234.ts.net");
        assert_eq!(a.bind, BindAddr::TailscaleLookup);
    }

    #[test]
    fn remote_addressing_falls_back_to_public_exposure() {
        let s = server("203.0.113.5", "public", None);
        let a = remote_addressing(&s, true);
        assert_eq!(a.upstream_host, "203.0.113.5");
        assert_eq!(a.bind, BindAddr::All);
        assert!(a.public_exposure);

        // Private binding off: publish on all interfaces, reach over the SSH address.
        let s = server("203.0.113.5", "tailscale", Some("100.64.0.7"));
        let a = remote_addressing(&s, false);
        assert_eq!(a.upstream_host, "100.64.0.7");
        assert_eq!(a.bind, BindAddr::All);
    }

    #[test]
    fn container_names_are_unique_per_slot_and_deployment() {
        let a = container_name("my app", "localhost", 0, "1234abcd-5678");
        assert_eq!(a, "rivetr-my-app-ms-local-0-1234abcd");
        let b = container_name("api", "9f8e7d6c-aaaa", 2, "deadbeef-0000");
        assert_eq!(b, "rivetr-api-ms-9f8e7d6c-2-deadbeef");
        assert_ne!(
            container_name("api", "s1", 0, "d1"),
            container_name("api", "s1", 0, "d2")
        );
        // Never collides with the orphan sweep for zero-downtime restarts.
        assert!(!b.contains("-restart-"));
    }

    #[test]
    fn final_upstreams_merge_new_and_kept() {
        let new = vec!["a:1".to_string(), "b:2".to_string()];
        let kept = vec!["c:3".to_string(), "a:1".to_string()];
        assert_eq!(final_upstreams(&new, &kept, &[]), ["a:1", "b:2", "c:3"]);
        let prev = vec!["old:1".to_string()];
        assert_eq!(final_upstreams(&[], &[], &prev), ["old:1"]);
    }
}
