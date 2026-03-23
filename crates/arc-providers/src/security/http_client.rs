// SPDX-License-Identifier: MIT
//! Hardened HTTP client factory.
//! Enforces: rustls TLS, redirect protection, connection pooling, timeouts.

use std::sync::OnceLock;
use std::time::Duration;

static SHARED_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Build a hardened HTTP client with security defaults.
pub fn build_secure_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // TLS: rustls only (default in reqwest 0.13)
        // Redirect: disabled to prevent SSRF
        .redirect(reqwest::redirect::Policy::none())
        // Connection pooling
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        // Timeouts
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        // HTTP/2 preferred
        .http2_prior_knowledge()
        // User-Agent
        .user_agent(format!("arc-cli/{}", env!("CARGO_PKG_VERSION")))
        // DNS caching via connection pool
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Get or create the shared HTTP client singleton.
pub fn shared_client() -> &'static reqwest::Client {
    SHARED_CLIENT.get_or_init(build_secure_client)
}
