//! In-flight request tracking per backend address, used to drain an old
//! container before it is stopped after a route swap.
//!
//! Only plain HTTP forwards are counted (up to response headers). WebSocket
//! tunnels are long-lived and are not waited for; they are closed when the
//! container stops.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::time::Instant;

use dashmap::DashMap;

/// Extra wait after the count reaches zero, so a request that looked up the
/// old route just before the swap, and response bodies still streaming, can finish.
const GRACE: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(100);

/// Default drain timeout in seconds, overridden at startup from config.
static DRAIN_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(30);

fn counters() -> &'static DashMap<String, Arc<AtomicUsize>> {
    static COUNTERS: OnceLock<DashMap<String, Arc<AtomicUsize>>> = OnceLock::new();
    COUNTERS.get_or_init(DashMap::new)
}

/// Set the maximum time to wait for in-flight requests to finish (0 disables draining).
pub fn set_drain_timeout_secs(secs: u64) {
    DRAIN_TIMEOUT_SECS.store(secs, Ordering::Relaxed);
}

pub fn drain_timeout() -> Duration {
    Duration::from_secs(DRAIN_TIMEOUT_SECS.load(Ordering::Relaxed))
}

/// RAII guard: counts one in-flight request for a backend address until dropped.
pub struct InFlightGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Start tracking a request to `addr` ("host:port").
pub fn track(addr: &str) -> InFlightGuard {
    let counter = counters()
        .entry(addr.to_string())
        .or_insert_with(|| Arc::new(AtomicUsize::new(0)))
        .clone();
    counter.fetch_add(1, Ordering::AcqRel);
    InFlightGuard { counter }
}

pub fn in_flight(addr: &str) -> usize {
    counters()
        .get(addr)
        .map(|c| c.load(Ordering::Acquire))
        .unwrap_or(0)
}

/// Wait until no requests are in flight to `addr`, then a short grace period.
/// Returns the number of requests still in flight when the wait ended
/// (0 means fully drained).
pub async fn drain(addr: &str, timeout: Duration) -> usize {
    if timeout.is_zero() {
        return in_flight(addr);
    }
    let deadline = Instant::now() + timeout;
    let mut idle_since: Option<Instant> = None;
    loop {
        let now = Instant::now();
        if in_flight(addr) == 0 {
            let since = *idle_since.get_or_insert(now);
            if now.duration_since(since) >= GRACE {
                break;
            }
        } else {
            idle_since = None;
        }
        if now >= deadline {
            break;
        }
        tokio::time::sleep(POLL).await;
    }
    let remaining = in_flight(addr);
    // Drop the entry once idle so the map does not grow with every deploy.
    if remaining == 0 {
        counters().remove_if(addr, |_, c| c.load(Ordering::Acquire) == 0);
    }
    remaining
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_counts_and_releases() {
        let addr = "127.0.0.1:59001";
        assert_eq!(in_flight(addr), 0);
        let a = track(addr);
        let b = track(addr);
        assert_eq!(in_flight(addr), 2);
        drop(a);
        assert_eq!(in_flight(addr), 1);
        drop(b);
        assert_eq!(in_flight(addr), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn drain_waits_for_guard_then_grace() {
        let addr = "127.0.0.1:59002";
        let guard = track(addr);
        let started = Instant::now();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            drop(guard);
        });
        let left = drain(addr, Duration::from_secs(30)).await;
        assert_eq!(left, 0);
        assert!(started.elapsed() >= Duration::from_secs(4));
    }

    #[tokio::test(start_paused = true)]
    async fn drain_gives_up_at_timeout() {
        let addr = "127.0.0.1:59003";
        let _guard = track(addr);
        let left = drain(addr, Duration::from_secs(5)).await;
        assert_eq!(left, 1);
    }

    #[tokio::test]
    async fn zero_timeout_returns_immediately() {
        let addr = "127.0.0.1:59004";
        let _guard = track(addr);
        assert_eq!(drain(addr, Duration::ZERO).await, 1);
    }
}
