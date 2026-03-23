// SPDX-License-Identifier: MIT
//! Unit tests for the rate limiter.

use arc_core::security::rate_limiter::{RateLimitConfig, RateLimiter};

#[test]
fn exhausts_bucket() {
    let rl = RateLimiter::new(RateLimitConfig {
        max_requests: 2,
        window_secs: 3600, // 1 hour — won't refill during test
    });

    assert!(rl.try_acquire("p"));
    assert!(rl.try_acquire("p"));
    assert!(!rl.try_acquire("p"));
    assert!(!rl.try_acquire("p"));
}

#[test]
fn providers_are_isolated() {
    let rl = RateLimiter::new(RateLimitConfig {
        max_requests: 1,
        window_secs: 3600,
    });

    assert!(rl.try_acquire("a"));
    assert!(!rl.try_acquire("a"));
    // Provider "b" is independent.
    assert!(rl.try_acquire("b"));
    assert!(!rl.try_acquire("b"));
}

#[test]
fn custom_provider_config() {
    let rl = RateLimiter::default();
    rl.set_provider_limit(
        "premium",
        RateLimitConfig {
            max_requests: 1000,
            window_secs: 60,
        },
    );

    assert_eq!(rl.remaining("premium"), 1000);
    assert_eq!(rl.remaining("default_provider"), 60); // default
}
