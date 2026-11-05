//! Upstream pools for apps that run on several servers: round robin or least
//! connections, passive health (an upstream that refuses a connection is taken
//! out for a cool-down) and optional sticky sessions by cookie.
//!
//! The pool only picks an address. Request handling, in-flight counting and
//! retries stay in the handler.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Cookie that pins a client to one upstream when sticky sessions are on.
pub const STICKY_COOKIE: &str = "rivetr_upstream";

/// How long an upstream stays out of rotation after a connection error.
pub const PASSIVE_DOWN_FOR: Duration = Duration::from_secs(10);

/// Load balancing policy of a pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LbPolicy {
    #[default]
    RoundRobin,
    LeastConn,
}

impl LbPolicy {
    pub fn parse(s: &str) -> Self {
        match s {
            "least_conn" => Self::LeastConn,
            _ => Self::RoundRobin,
        }
    }
}

/// Stable token for an upstream address, used as the sticky cookie value so the
/// cookie does not reveal internal addresses. FNV-1a 64, hex.
pub fn sticky_token(addr: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in addr.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:016x}", hash)
}

/// Value of `name` in a `Cookie` header, if present.
pub fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k.trim() == name).then(|| v.trim())
    })
}

/// `Set-Cookie` value that pins the client to `addr`.
pub fn sticky_set_cookie(addr: &str, secure: bool) -> String {
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax{}",
        STICKY_COOKIE,
        sticky_token(addr),
        if secure { "; Secure" } else { "" }
    )
}

/// Add the sticky cookie for `addr` to a response.
pub fn append_sticky_cookie<B>(response: &mut hyper::Response<B>, addr: &str, secure: bool) {
    if let Ok(value) = hyper::header::HeaderValue::from_str(&sticky_set_cookie(addr, secure)) {
        response
            .headers_mut()
            .append(hyper::header::SET_COOKIE, value);
    }
}

/// The pool's pick for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// "host:port" to forward to.
    pub addr: String,
    /// Whether the response should set the sticky cookie (sticky pool and the
    /// client had no valid cookie for a live upstream).
    pub set_cookie: bool,
}

/// A set of upstream addresses for one domain.
#[derive(Debug)]
pub struct UpstreamPool {
    upstreams: Vec<String>,
    policy: LbPolicy,
    sticky: bool,
    cursor: AtomicUsize,
    down_until: DashMap<String, Instant>,
    down_for: Duration,
}

impl UpstreamPool {
    pub fn new(upstreams: Vec<String>, policy: LbPolicy, sticky: bool) -> Self {
        let mut deduped: Vec<String> = Vec::with_capacity(upstreams.len());
        for u in upstreams {
            if !deduped.contains(&u) {
                deduped.push(u);
            }
        }
        Self {
            upstreams: deduped,
            policy,
            sticky,
            cursor: AtomicUsize::new(0),
            down_until: DashMap::new(),
            down_for: PASSIVE_DOWN_FOR,
        }
    }

    /// Override the passive cool-down (tests).
    pub fn with_down_for(mut self, down_for: Duration) -> Self {
        self.down_for = down_for;
        self
    }

    pub fn upstreams(&self) -> &[String] {
        &self.upstreams
    }

    pub fn policy(&self) -> LbPolicy {
        self.policy
    }

    pub fn sticky(&self) -> bool {
        self.sticky
    }

    pub fn len(&self) -> usize {
        self.upstreams.len()
    }

    pub fn is_empty(&self) -> bool {
        self.upstreams.is_empty()
    }

    /// Take `addr` out of rotation for the cool-down (connection refused, reset, timeout).
    pub fn mark_down(&self, addr: &str) {
        if self.upstreams.iter().any(|u| u == addr) {
            self.down_until
                .insert(addr.to_string(), Instant::now() + self.down_for);
        }
    }

    /// Put `addr` back into rotation now.
    pub fn mark_up(&self, addr: &str) {
        self.down_until.remove(addr);
    }

    /// Whether `addr` is passively marked down at `now`.
    pub fn is_down_at(&self, addr: &str, now: Instant) -> bool {
        self.down_until.get(addr).is_some_and(|until| *until > now)
    }

    /// Pick an upstream. `extra_healthy` lets the caller exclude addresses its
    /// active health check found down; `in_flight` gives the open request count
    /// per address for least connections. `sticky_cookie` is the client's cookie
    /// value. Returns `None` when every upstream is down.
    pub fn select(
        &self,
        sticky_cookie: Option<&str>,
        extra_healthy: impl Fn(&str) -> bool,
        in_flight: impl Fn(&str) -> usize,
    ) -> Option<Selection> {
        let now = Instant::now();
        let live: Vec<&String> = self
            .upstreams
            .iter()
            .filter(|u| !self.is_down_at(u, now) && extra_healthy(u))
            .collect();
        if live.is_empty() {
            return None;
        }

        if self.sticky {
            if let Some(token) = sticky_cookie {
                if let Some(addr) = live.iter().find(|u| sticky_token(u) == token) {
                    return Some(Selection {
                        addr: (*addr).clone(),
                        set_cookie: false,
                    });
                }
            }
        }

        let start = self.cursor.fetch_add(1, Ordering::Relaxed);
        let picked = match self.policy {
            LbPolicy::RoundRobin => live[start % live.len()],
            LbPolicy::LeastConn => {
                // Fewest open requests; ties rotate so equal upstreams share load.
                let n = live.len();
                (0..n)
                    .map(|i| live[(start + i) % n])
                    .min_by_key(|u| in_flight(u))
                    .unwrap_or(live[0])
            }
        };
        Some(Selection {
            addr: picked.clone(),
            set_cookie: self.sticky,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(addrs: &[&str], policy: LbPolicy, sticky: bool) -> UpstreamPool {
        UpstreamPool::new(
            addrs.iter().map(|s| s.to_string()).collect(),
            policy,
            sticky,
        )
    }

    fn pick(p: &UpstreamPool) -> String {
        p.select(None, |_| true, |_| 0).unwrap().addr
    }

    #[test]
    fn round_robin_cycles_through_all_upstreams() {
        let p = pool(&["a:1", "b:2", "c:3"], LbPolicy::RoundRobin, false);
        let picks: Vec<String> = (0..6).map(|_| pick(&p)).collect();
        assert_eq!(picks, ["a:1", "b:2", "c:3", "a:1", "b:2", "c:3"]);
    }

    #[test]
    fn duplicates_are_removed() {
        let p = pool(&["a:1", "a:1", "b:2"], LbPolicy::RoundRobin, false);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn least_conn_prefers_the_idlest_upstream() {
        let p = pool(&["a:1", "b:2", "c:3"], LbPolicy::LeastConn, false);
        let load = |u: &str| match u {
            "a:1" => 5,
            "b:2" => 1,
            _ => 3,
        };
        for _ in 0..5 {
            assert_eq!(p.select(None, |_| true, load).unwrap().addr, "b:2");
        }
    }

    #[test]
    fn least_conn_rotates_between_equal_upstreams() {
        let p = pool(&["a:1", "b:2"], LbPolicy::LeastConn, false);
        let picks: Vec<String> = (0..4)
            .map(|_| p.select(None, |_| true, |_| 0).unwrap().addr)
            .collect();
        assert!(picks.contains(&"a:1".to_string()));
        assert!(picks.contains(&"b:2".to_string()));
    }

    #[test]
    fn passive_down_marking_skips_and_then_recovers() {
        let p = pool(&["a:1", "b:2"], LbPolicy::RoundRobin, false)
            .with_down_for(Duration::from_millis(50));
        p.mark_down("a:1");
        for _ in 0..4 {
            assert_eq!(pick(&p), "b:2");
        }
        std::thread::sleep(Duration::from_millis(70));
        let picks: Vec<String> = (0..4).map(|_| pick(&p)).collect();
        assert!(picks.contains(&"a:1".to_string()));
    }

    #[test]
    fn all_down_returns_none_and_mark_up_restores() {
        let p = pool(&["a:1", "b:2"], LbPolicy::RoundRobin, false);
        p.mark_down("a:1");
        p.mark_down("b:2");
        assert!(p.select(None, |_| true, |_| 0).is_none());
        p.mark_up("b:2");
        assert_eq!(pick(&p), "b:2");
    }

    #[test]
    fn mark_down_ignores_unknown_addresses() {
        let p = pool(&["a:1"], LbPolicy::RoundRobin, false);
        p.mark_down("z:9");
        assert_eq!(pick(&p), "a:1");
    }

    #[test]
    fn active_health_filter_is_respected() {
        let p = pool(&["a:1", "b:2"], LbPolicy::RoundRobin, false);
        for _ in 0..3 {
            assert_eq!(p.select(None, |u| u != "a:1", |_| 0).unwrap().addr, "b:2");
        }
    }

    #[test]
    fn sticky_cookie_pins_to_the_same_upstream() {
        let p = pool(&["a:1", "b:2", "c:3"], LbPolicy::RoundRobin, true);
        let first = p.select(None, |_| true, |_| 0).unwrap();
        assert!(first.set_cookie);
        let token = sticky_token(&first.addr);
        for _ in 0..5 {
            let s = p.select(Some(&token), |_| true, |_| 0).unwrap();
            assert_eq!(s.addr, first.addr);
            assert!(!s.set_cookie);
        }
    }

    #[test]
    fn sticky_cookie_for_a_down_upstream_is_reassigned() {
        let p = pool(&["a:1", "b:2"], LbPolicy::RoundRobin, true);
        p.mark_down("a:1");
        let s = p
            .select(Some(&sticky_token("a:1")), |_| true, |_| 0)
            .unwrap();
        assert_eq!(s.addr, "b:2");
        assert!(s.set_cookie);
    }

    #[test]
    fn non_sticky_pool_ignores_cookie_and_never_sets_it() {
        let p = pool(&["a:1", "b:2"], LbPolicy::RoundRobin, false);
        let s = p
            .select(Some(&sticky_token("b:2")), |_| true, |_| 0)
            .unwrap();
        assert_eq!(s.addr, "a:1");
        assert!(!s.set_cookie);
    }

    #[test]
    fn cookie_parsing_and_set_cookie_format() {
        let header = "theme=dark; rivetr_upstream=abc123 ; other=1";
        assert_eq!(cookie_value(header, STICKY_COOKIE), Some("abc123"));
        assert_eq!(cookie_value(header, "missing"), None);
        let set = sticky_set_cookie("a:1", true);
        assert!(set.starts_with("rivetr_upstream="));
        assert!(set.contains("HttpOnly"));
        assert!(set.ends_with("; Secure"));
        assert!(!sticky_set_cookie("a:1", false).contains("Secure"));
        // Token is stable and does not contain the address.
        assert_eq!(sticky_token("10.0.0.1:8080"), sticky_token("10.0.0.1:8080"));
        assert!(!sticky_token("10.0.0.1:8080").contains("10.0.0.1"));
    }

    #[test]
    fn policy_parsing() {
        assert_eq!(LbPolicy::parse("least_conn"), LbPolicy::LeastConn);
        assert_eq!(LbPolicy::parse("round_robin"), LbPolicy::RoundRobin);
        assert_eq!(LbPolicy::parse("anything"), LbPolicy::RoundRobin);
    }
}
