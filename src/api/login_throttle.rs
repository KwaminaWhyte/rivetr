//! Per-account failed-login throttling (SEC-H3).
//!
//! IP rate limiting alone cannot stop distributed or proxy-rotated credential
//! stuffing, so failed attempts are also counted per account (lowercased email
//! for password login, user id for the 2FA step). After `MAX_FAILURES` failures
//! inside `FAILURE_WINDOW` the account is locked with an exponential backoff.
//!
//! Tradeoff: an attacker who knows an email can keep that account locked for up
//! to `MAX_LOCKOUT`. The backoff is capped so the owner always recovers within
//! 15 minutes, and a successful login clears the counter.

use dashmap::DashMap;
use std::time::{Duration, Instant};

/// Failures allowed before the account is locked
const MAX_FAILURES: u32 = 5;
/// Failures older than this are forgotten
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
/// Lockout applied on the first failure over the limit (doubles each failure)
const BASE_LOCKOUT: Duration = Duration::from_secs(30);
/// Upper bound on a single lockout
const MAX_LOCKOUT: Duration = Duration::from_secs(15 * 60);
/// Hard cap on tracked keys, to bound memory under random-email floods
const MAX_TRACKED: usize = 50_000;

#[derive(Debug, Clone)]
struct Entry {
    failures: u32,
    last_failure: Instant,
    locked_until: Option<Instant>,
}

/// Thread-safe failed-attempt tracker.
#[derive(Debug, Default)]
pub struct LoginThrottle {
    entries: DashMap<String, Entry>,
}

impl LoginThrottle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalize a login identifier into a throttle key.
    pub fn key(identifier: &str) -> String {
        identifier.trim().to_lowercase()
    }

    /// Returns `Err(retry_after_secs)` if the key is currently locked out.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        self.check_at(key, Instant::now())
    }

    /// Record a failed attempt, locking the key once over the limit.
    pub fn record_failure(&self, key: &str) {
        self.record_failure_at(key, Instant::now());
    }

    /// Forget a key after a successful authentication.
    pub fn record_success(&self, key: &str) {
        self.entries.remove(key);
    }

    /// Drop entries that are neither locked nor inside the failure window.
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.entries.retain(|_, e| {
            let locked = e.locked_until.is_some_and(|until| until > now);
            locked || now.duration_since(e.last_failure) < FAILURE_WINDOW
        });
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<(), u64> {
        if let Some(entry) = self.entries.get(key) {
            if let Some(until) = entry.locked_until {
                if until > now {
                    return Err(until.duration_since(now).as_secs().max(1));
                }
            }
        }
        Ok(())
    }

    fn record_failure_at(&self, key: &str, now: Instant) {
        if self.entries.len() >= MAX_TRACKED && !self.entries.contains_key(key) {
            self.cleanup();
            if self.entries.len() >= MAX_TRACKED {
                return;
            }
        }

        let mut entry = self.entries.entry(key.to_string()).or_insert(Entry {
            failures: 0,
            last_failure: now,
            locked_until: None,
        });

        // Forget stale failures, unless a lockout is still running
        let lock_active = entry.locked_until.is_some_and(|until| until > now);
        if !lock_active && now.duration_since(entry.last_failure) >= FAILURE_WINDOW {
            entry.failures = 0;
            entry.locked_until = None;
        }

        entry.failures = entry.failures.saturating_add(1);
        entry.last_failure = now;

        if entry.failures >= MAX_FAILURES {
            let exp = (entry.failures - MAX_FAILURES).min(16);
            let lockout = BASE_LOCKOUT
                .checked_mul(1u32 << exp)
                .unwrap_or(MAX_LOCKOUT)
                .min(MAX_LOCKOUT);
            entry.locked_until = Some(now + lockout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_until_max_failures_then_locks() {
        let t = LoginThrottle::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES - 1 {
            t.record_failure_at("a@x.io", now);
            assert!(t.check_at("a@x.io", now).is_ok());
        }
        t.record_failure_at("a@x.io", now);
        let retry = t.check_at("a@x.io", now).unwrap_err();
        assert!((1..=BASE_LOCKOUT.as_secs()).contains(&retry));
        // Other accounts are unaffected
        assert!(t.check_at("b@x.io", now).is_ok());
    }

    #[test]
    fn lockout_expires_and_backs_off_exponentially() {
        let t = LoginThrottle::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            t.record_failure_at("a@x.io", now);
        }
        let later = now + BASE_LOCKOUT + Duration::from_secs(1);
        assert!(t.check_at("a@x.io", later).is_ok());
        // Next failure doubles the lockout
        t.record_failure_at("a@x.io", later);
        let retry = t.check_at("a@x.io", later).unwrap_err();
        assert!(retry > BASE_LOCKOUT.as_secs());
    }

    #[test]
    fn lockout_is_capped() {
        let t = LoginThrottle::new();
        let mut now = Instant::now();
        for _ in 0..40 {
            t.record_failure_at("a@x.io", now);
            now += Duration::from_secs(1);
        }
        let retry = t.check_at("a@x.io", now).unwrap_err();
        assert!(retry <= MAX_LOCKOUT.as_secs());
    }

    #[test]
    fn success_clears_failures() {
        let t = LoginThrottle::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES {
            t.record_failure_at("a@x.io", now);
        }
        assert!(t.check_at("a@x.io", now).is_err());
        t.record_success("a@x.io");
        assert!(t.check_at("a@x.io", now).is_ok());
    }

    #[test]
    fn stale_failures_are_forgotten() {
        let t = LoginThrottle::new();
        let now = Instant::now();
        for _ in 0..MAX_FAILURES - 1 {
            t.record_failure_at("a@x.io", now);
        }
        let later = now + FAILURE_WINDOW + Duration::from_secs(1);
        t.record_failure_at("a@x.io", later);
        assert!(t.check_at("a@x.io", later).is_ok());
    }

    #[test]
    fn key_is_case_and_whitespace_insensitive() {
        assert_eq!(
            LoginThrottle::key("  Admin@Example.COM "),
            "admin@example.com"
        );
    }
}
