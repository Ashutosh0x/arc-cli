// SPDX-License-Identifier: MIT
//! Token-bucket rate limiter per provider.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Configuration for a rate limiter bucket.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests in the window.
    pub max_requests: u32,
    /// Window duration in seconds.
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,
            window_secs: 60,
        }
    }
}

struct Bucket {
    tokens: u32,
    max: u32,
    window_secs: u64,
    last_refill: Instant,
}

impl Bucket {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            tokens: config.max_requests,
            max: config.max_requests,
            window_secs: config.window_secs,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs();
        if elapsed >= self.window_secs {
            self.tokens = self.max;
            self.last_refill = Instant::now();
        }
    }

    fn remaining(&mut self) -> u32 {
        self.refill();
        self.tokens
    }
}

/// Thread-safe rate limiter for multiple providers.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    default_config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter with the given default config.
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            default_config,
        }
    }

    /// Try to acquire a request slot for the given provider.
    ///
    /// Returns `true` if allowed, `false` if rate-limited.
    pub fn try_acquire(&self, provider: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        buckets
            .entry(provider.to_owned())
            .or_insert_with(|| Bucket::new(&self.default_config))
            .try_acquire()
    }

    /// Remaining requests in the current window.
    pub fn remaining(&self, provider: &str) -> u32 {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        buckets
            .entry(provider.to_owned())
            .or_insert_with(|| Bucket::new(&self.default_config))
            .remaining()
    }

    /// Configure a custom limit for a specific provider.
    pub fn set_provider_limit(&self, provider: &str, config: RateLimitConfig) {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        buckets.insert(provider.to_owned(), Bucket::new(&config));
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_rate_limiting() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window_secs: 60,
        });

        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(limiter.try_acquire("test"));
        assert!(!limiter.try_acquire("test")); // 4th blocked

        // Different provider has its own bucket.
        assert!(limiter.try_acquire("other"));
    }

    #[test]
    fn remaining_count() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 5,
            window_secs: 60,
        });

        assert_eq!(limiter.remaining("test"), 5);
        limiter.try_acquire("test");
        assert_eq!(limiter.remaining("test"), 4);
    }
}
