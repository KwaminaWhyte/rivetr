//! Rate limiting middleware using a sliding window algorithm.
//!
//! This module provides rate limiting for API endpoints using a token bucket
//! approach with sliding window for smooth rate limiting.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            entries: DashMap::new(),
            window_duration: Duration::from_secs(config.window_seconds),
            config,
        }
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

        let mut entry = self.entries.entry(key).or_insert_with(|| RateLimitEntry::new(max_tokens));

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
            let retry_after = self.window_duration.saturating_sub(elapsed).as_secs().max(1);
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

        self.entries.retain(|_, entry| {
            now.duration_since(entry.window_start) < expiry
        });
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

/// Extract client IP from request headers
fn extract_client_ip(request: &Request<Body>) -> IpAddr {
    // Check X-Forwarded-For header first (for reverse proxy setups)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            // Take the first IP in the list (original client)
            if let Some(ip_str) = value.split(',').next() {
                if let Ok(ip) = ip_str.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            if let Ok(ip) = value.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // Fallback to a default IP (in production, you'd want to get the actual connection IP)
    // This is a reasonable fallback for local development
    "127.0.0.1".parse().unwrap()
}

/// Rate limiting middleware for general API endpoints
pub async fn rate_limit_api(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Api).await
}

/// Rate limiting middleware for webhook endpoints
pub async fn rate_limit_webhook(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Webhook).await
}

/// Rate limiting middleware for auth endpoints
pub async fn rate_limit_auth(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    rate_limit_with_tier(state, request, next, RateLimitTier::Auth).await
}

/// Core rate limiting logic
async fn rate_limit_with_tier(
    state: Arc<AppState>,
    request: Request<Body>,
    next: Next,
    tier: RateLimitTier,
) -> Result<Response, Response> {
    let ip = extract_client_ip(&request);

    match state.rate_limiter.check_rate_limit(ip, tier) {
        Ok(info) => {
            let response = next.run(request).await;

            // Add rate limit headers to successful responses
            let (mut parts, body) = response.into_parts();
            parts.headers.insert(
                "X-RateLimit-Limit",
                info.limit.to_string().parse().unwrap(),
            );
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
                    ("X-RateLimit-Limit", state.rate_limiter.config.api_requests_per_window.to_string()),
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
            rate_limiter.cleanup_expired();
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
            assert!(result.is_ok(), "All requests should be allowed when disabled");
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
}
