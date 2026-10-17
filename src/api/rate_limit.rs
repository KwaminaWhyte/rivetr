//! Rate limiting middleware using a sliding window algorithm.
//!
//! This module provides rate limiting for API endpoints using a token bucket
//! approach with sliding window for smooth rate limiting.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::login_throttle::LoginThrottle;
use crate::config::RateLimitConfig;
use crate::AppState;

/// Rate limit tier for different endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitTier {
    /// General API endpoints (100 req/min default)
    Api,
    /// Webhook endpoints (500 req/min default)
    Webhook,
    /// Auth endpoints (20 req/min default)
    Auth,
}

/// Entry in the rate limit tracker
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Tokens remaining in the current window
    tokens: u32,
    /// Start of the current window
    window_start: Instant,
    /// Last request time (for sliding window)
    last_request: Instant,
}

impl RateLimitEntry {
    fn new(max_tokens: u32) -> Self {
        let now = Instant::now();
        Self {
            tokens: max_tokens,
            window_start: now,
            last_request: now,
        }
    }
}

/// Thread-safe rate limiter using dashmap
#[derive(Debug)]
pub struct RateLimiter {
    /// Map of (IP, Tier) -> RateLimitEntry
    entries: DashMap<(IpAddr, RateLimitTier), RateLimitEntry>,
    /// Configuration
    config: RateLimitConfig,
    /// Window duration
    window_duration: Duration,
    /// Proxies whose forwarding headers are trusted when resolving client IPs
    trusted_proxies: TrustedProxies,
    /// Per-account failed login / 2FA throttle (SEC-H3)
    login_throttle: LoginThrottle,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            entries: DashMap::new(),
            window_duration: Duration::from_secs(config.window_seconds),
            trusted_proxies: TrustedProxies::parse(&config.trusted_proxies),
            login_throttle: LoginThrottle::new(),
            config,
        }
    }

    /// Per-account failed login / 2FA throttle
    pub fn login_throttle(&self) -> &LoginThrottle {
        &self.login_throttle
    }

    /// Resolve the real client IP for a request. The TCP peer address is used
    /// unless the peer is a trusted proxy, in which case forwarding headers apply.
    pub fn client_ip(&self, headers: &HeaderMap, peer: Option<SocketAddr>) -> IpAddr {
        resolve_client_ip(headers, peer.map(|p| p.ip()), &self.trusted_proxies)
    }

    /// Check if a request should be allowed and consume a token if so.
    /// Returns Ok(remaining_tokens) if allowed, Err(retry_after_seconds) if rate limited.
    pub fn check_rate_limit(&self, ip: IpAddr, tier: RateLimitTier) -> Result<RateLimitInfo, u64> {
        if !self.config.enabled {
            return Ok(RateLimitInfo {
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_after: 0,
            });
        }

        let max_tokens = self.get_max_tokens(tier);
        let key = (ip, tier);
        let now = Instant::now();

        let mut entry = self
            .entries
            .entry(key)
            .or_insert_with(|| RateLimitEntry::new(max_tokens));

        // Check if we need to reset the window
        let elapsed = now.duration_since(entry.window_start);
        if elapsed >= self.window_duration {
            // Reset the window
            entry.tokens = max_tokens;
            entry.window_start = now;
        } else {
            // Apply sliding window: gradually replenish tokens based on time elapsed since last request
            let since_last = now.duration_since(entry.last_request);
            let replenish_rate = max_tokens as f64 / self.window_duration.as_secs_f64();
            let replenished = (since_last.as_secs_f64() * replenish_rate) as u32;
            entry.tokens = (entry.tokens + replenished).min(max_tokens);
        }

        entry.last_request = now;

        if entry.tokens > 0 {
            entry.tokens -= 1;
            let remaining = entry.tokens;
            let reset_after = self.window_duration.saturating_sub(elapsed).as_secs();
            Ok(RateLimitInfo {
                remaining,
                limit: max_tokens,
                reset_after,
            })
        } else {
            // Calculate retry after based on when tokens will be available
            let retry_after = self
                .window_duration
                .saturating_sub(elapsed)
                .as_secs()
                .max(1);
            Err(retry_after)
        }
    }

    /// Get the maximum tokens for a given tier
    fn get_max_tokens(&self, tier: RateLimitTier) -> u32 {
        match tier {
            RateLimitTier::Api => self.config.api_requests_per_window,
            RateLimitTier::Webhook => self.config.webhook_requests_per_window,
            RateLimitTier::Auth => self.config.auth_requests_per_window,
        }
    }

    /// Clean up expired entries to prevent memory leaks
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let expiry = self.window_duration * 2; // Keep entries for 2x window duration

        self.entries
            .retain(|_, entry| now.duration_since(entry.window_start) < expiry);
        self.login_throttle.cleanup();
    }

    /// Get the number of tracked entries (for monitoring)
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Information about rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Remaining requests in the current window
    pub remaining: u32,
    /// Maximum requests per window
    pub limit: u32,
    /// Seconds until the window resets
    pub reset_after: u64,
}

/// A set of trusted reverse proxies (IPs and CIDRs).
///
/// Loopback addresses are always trusted: a loopback peer is a local process
/// (the embedded proxy, nginx, Caddy or Traefik on the same host), never a
/// remote attacker. Docker bridge or other private ranges must be listed
/// explicitly via `rate_limit.trusted_proxies`.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    nets: Vec<(IpAddr, u8)>,
}

impl TrustedProxies {
    /// Parse a list of IPs / CIDRs. Invalid entries are skipped with a warning.
    pub fn parse(entries: &[String]) -> Self {
        let mut nets = Vec::new();
        for entry in entries {
            match parse_cidr(entry) {
                Some(net) => nets.push(net),
                None => tracing::warn!(entry = %entry, "Ignoring invalid trusted_proxies entry"),
            }
        }
        Self { nets }
    }

    /// Whether `ip` is a trusted proxy.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        if ip.is_loopback() {
            return true;
        }
        self.nets
            .iter()
            .any(|(net, prefix)| cidr_contains(*net, *prefix, ip))
    }
}

fn parse_cidr(entry: &str) -> Option<(IpAddr, u8)> {
    let entry = entry.trim();
    let (addr, prefix) = match entry.split_once('/') {
        Some((a, p)) => (a, Some(p.trim().parse::<u8>().ok()?)),
        None => (entry, None),
    };
    let addr = addr.trim().parse::<IpAddr>().ok()?.to_canonical();
    let max = if addr.is_ipv4() { 32 } else { 128 };
    let prefix = prefix.unwrap_or(max);
    if prefix > max {
        return None;
    }
    Some((addr, prefix))
}

fn cidr_contains(net: IpAddr, prefix: u8, ip: IpAddr) -> bool {
    match (net, ip) {
        (IpAddr::V4(n), IpAddr::V4(i)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix as u32)
            };
            u32::from(n) & mask == u32::from(i) & mask
        }
        (IpAddr::V6(n), IpAddr::V6(i)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix as u32)
            };
            u128::from(n) & mask == u128::from(i) & mask
        }
        _ => false,
    }
}

/// Resolve the client IP from the TCP peer and forwarding headers.
///
/// Headers are honored only when the peer is a trusted proxy. `X-Forwarded-For`
/// is walked right to left, skipping trusted proxies; the first untrusted hop is
/// the client. This ignores any values a client prepended to the header.
/// Without a peer address (should not happen in production) loopback is returned
/// and headers are ignored.
pub fn resolve_client_ip(
    headers: &HeaderMap,
    peer: Option<IpAddr>,
    trusted: &TrustedProxies,
) -> IpAddr {
    let Some(peer) = peer.map(|p| p.to_canonical()) else {
        return IpAddr::from([127, 0, 0, 1]);
    };
    if !trusted.contains(peer) {
        return peer;
    }

    let mut hops: Vec<IpAddr> = Vec::new();
    for value in headers.get_all("x-forwarded-for") {
        if let Ok(value) = value.to_str() {
            for part in value.split(',') {
                match part.trim().parse::<IpAddr>() {
                    Ok(ip) => hops.push(ip.to_canonical()),
                    // Unparsable hop: use an unspecified placeholder so it is never trusted
                    Err(_) => hops.push(IpAddr::from([0, 0, 0, 0])),
                }
            }
        }
    }
    if !hops.is_empty() {
        for hop in hops.iter().rev() {
            if !trusted.contains(*hop) {
                return *hop;
            }
        }
        // Every hop is a trusted proxy; the left-most is the origin
        return hops[0];
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
            return ip.to_canonical();
        }
    }

    peer
}

/// Extract the client IP for a request (TCP peer unless behind a trusted proxy)
fn extract_client_ip(limiter: &RateLimiter, request: &Request<Body>) -> IpAddr {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr);
    limiter.client_ip(request.headers(), peer)
}

/// Rate limiting middleware for general API endpoints
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
pub async fn rate_limit_api(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Api).await
}

/// Rate limiting middleware for webhook endpoints
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
pub async fn rate_limit_webhook(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Webhook).await
}

/// Rate limiting middleware for auth endpoints
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
pub async fn rate_limit_auth(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Auth).await
}

/// Core rate limiting logic
#[allow(clippy::result_large_err)] // Err carries the ready-to-send rejection response
async fn rate_limit_with_tier(
    state: Arc<AppState>,
    request: Request<Body>,
    next: Next,
    tier: RateLimitTier,
) -> Result<Response, Response> {
    let ip = extract_client_ip(&state.rate_limiter, &request);

    match state.rate_limiter.check_rate_limit(ip, tier) {
        Ok(info) => {
            let response = next.run(request).await;

            // Add rate limit headers to successful responses
            let (mut parts, body) = response.into_parts();
            parts
                .headers
                .insert("X-RateLimit-Limit", info.limit.to_string().parse().unwrap());
            parts.headers.insert(
                "X-RateLimit-Remaining",
                info.remaining.to_string().parse().unwrap(),
            );
            parts.headers.insert(
                "X-RateLimit-Reset",
                info.reset_after.to_string().parse().unwrap(),
            );

            Ok(Response::from_parts(parts, body))
        }
        Err(retry_after) => {
            let response = (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("Retry-After", retry_after.to_string()),
                    (
                        "X-RateLimit-Limit",
                        state
                            .rate_limiter
                            .config
                            .api_requests_per_window
                            .to_string(),
                    ),
                    ("X-RateLimit-Remaining", "0".to_string()),
                    ("X-RateLimit-Reset", retry_after.to_string()),
                ],
                format!("Rate limit exceeded. Try again in {} seconds.", retry_after),
            );
            Err(response.into_response())
        }
    }
}

/// Spawn a background task to periodically clean up expired rate limit entries
pub fn spawn_cleanup_task(rate_limiter: Arc<RateLimiter>, cleanup_interval_secs: u64) {
    tokio::spawn(async move {
        let interval = Duration::from_secs(cleanup_interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            crate::utils::supervise::guarded("rate_limit_cleanup", async {
                rate_limiter.cleanup_expired()
            })
            .await;
            tracing::debug!(
                "Rate limiter cleanup complete, {} entries remaining",
                rate_limiter.entry_count()
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RateLimitConfig {
        RateLimitConfig {
            enabled: true,
            api_requests_per_window: 10,
            webhook_requests_per_window: 50,
            auth_requests_per_window: 5,
            window_seconds: 60,
            cleanup_interval: 300,
            trusted_proxies: Vec::new(),
        }
    }

    #[test]
    fn test_rate_limiter_allows_requests_under_limit() {
        let limiter = RateLimiter::new(test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Should allow 10 requests
        for i in 0..10 {
            let result = limiter.check_rate_limit(ip, RateLimitTier::Api);
            assert!(result.is_ok(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_after_limit() {
        let limiter = RateLimiter::new(test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Exhaust all tokens
        for _ in 0..10 {
            let _ = limiter.check_rate_limit(ip, RateLimitTier::Api);
        }

        // Next request should be blocked
        let result = limiter.check_rate_limit(ip, RateLimitTier::Api);
        assert!(result.is_err(), "Request should be rate limited");
    }

    #[test]
    fn test_different_ips_have_separate_limits() {
        let limiter = RateLimiter::new(test_config());
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        // Exhaust ip1's tokens
        for _ in 0..10 {
            let _ = limiter.check_rate_limit(ip1, RateLimitTier::Api);
        }

        // ip2 should still be able to make requests
        let result = limiter.check_rate_limit(ip2, RateLimitTier::Api);
        assert!(result.is_ok(), "Different IP should have its own limit");
    }

    #[test]
    fn test_different_tiers_have_different_limits() {
        let limiter = RateLimiter::new(test_config());
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Auth tier has lower limit (5)
        for _ in 0..5 {
            let _ = limiter.check_rate_limit(ip, RateLimitTier::Auth);
        }

        // Auth should be blocked
        assert!(
            limiter.check_rate_limit(ip, RateLimitTier::Auth).is_err(),
            "Auth should be rate limited"
        );

        // But API tier should still work
        assert!(
            limiter.check_rate_limit(ip, RateLimitTier::Api).is_ok(),
            "API should still be allowed"
        );
    }

    #[test]
    fn test_disabled_rate_limiting() {
        let mut config = test_config();
        config.enabled = false;
        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Should allow unlimited requests when disabled
        for _ in 0..100 {
            let result = limiter.check_rate_limit(ip, RateLimitTier::Api);
            assert!(
                result.is_ok(),
                "All requests should be allowed when disabled"
            );
        }
    }

    #[test]
    fn test_cleanup_expired() {
        let mut config = test_config();
        config.window_seconds = 1; // 1 second window for testing
        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // Add an entry
        let _ = limiter.check_rate_limit(ip, RateLimitTier::Api);
        assert_eq!(limiter.entry_count(), 1);

        // Cleanup shouldn't remove recent entries
        limiter.cleanup_expired();
        assert_eq!(limiter.entry_count(), 1);
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn test_untrusted_peer_ignores_forwarding_headers() {
        let trusted = TrustedProxies::parse(&[]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "5.6.7.8")]);
        assert_eq!(
            resolve_client_ip(&h, Some(ip("203.0.113.9")), &trusted),
            ip("203.0.113.9")
        );
    }

    #[test]
    fn test_loopback_peer_is_trusted_by_default() {
        let trusted = TrustedProxies::parse(&[]);
        let h = headers(&[("x-forwarded-for", "198.51.100.7")]);
        assert_eq!(
            resolve_client_ip(&h, Some(ip("127.0.0.1")), &trusted),
            ip("198.51.100.7")
        );
        assert_eq!(
            resolve_client_ip(&h, Some(ip("::1")), &trusted),
            ip("198.51.100.7")
        );
        assert_eq!(
            resolve_client_ip(&h, Some(ip("::ffff:127.0.0.1")), &trusted),
            ip("198.51.100.7")
        );
    }

    #[test]
    fn test_trusted_cidr_peer_uses_rightmost_untrusted_hop() {
        let trusted = TrustedProxies::parse(&["172.17.0.0/16".to_string(), "10.0.0.5".to_string()]);
        // Client-supplied spoof "9.9.9.9" is prepended; real client is 198.51.100.7
        let h = headers(&[("x-forwarded-for", "9.9.9.9, 198.51.100.7, 10.0.0.5")]);
        assert_eq!(
            resolve_client_ip(&h, Some(ip("172.17.0.1")), &trusted),
            ip("198.51.100.7")
        );
        // Peer outside the CIDR is not trusted
        assert_eq!(
            resolve_client_ip(&h, Some(ip("172.18.0.1")), &trusted),
            ip("172.18.0.1")
        );
    }

    #[test]
    fn test_x_real_ip_used_when_no_xff_from_trusted_peer() {
        let trusted = TrustedProxies::parse(&[]);
        let h = headers(&[("x-real-ip", "198.51.100.8")]);
        assert_eq!(
            resolve_client_ip(&h, Some(ip("127.0.0.1")), &trusted),
            ip("198.51.100.8")
        );
    }

    #[test]
    fn test_missing_peer_ignores_headers() {
        let trusted = TrustedProxies::parse(&[]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        assert_eq!(resolve_client_ip(&h, None, &trusted), ip("127.0.0.1"));
    }

    #[test]
    fn test_cidr_parsing() {
        assert!(parse_cidr("10.0.0.0/8").is_some());
        assert!(parse_cidr("fd00::/8").is_some());
        assert!(parse_cidr("10.0.0.0/33").is_none());
        assert!(parse_cidr("not-an-ip").is_none());
        let t = TrustedProxies::parse(&["10.0.0.0/8".to_string(), "bogus".to_string()]);
        assert!(t.contains(ip("10.9.8.7")));
        assert!(!t.contains(ip("11.0.0.1")));
        let all = TrustedProxies::parse(&["0.0.0.0/0".to_string()]);
        assert!(all.contains(ip("8.8.8.8")));
    }
}
