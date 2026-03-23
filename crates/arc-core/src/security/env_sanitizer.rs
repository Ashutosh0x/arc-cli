// SPDX-License-Identifier: MIT
//! Environment Variable Sanitization
//!
//! Blocks sensitive environment variables from being passed to the LLM:
//! - Name patterns: TOKEN, SECRET, PASSWORD, KEY, AUTH, etc.
//! - Value patterns: RSA keys, JWTs, GitHub tokens, AWS keys, Stripe keys
//! - Strict mode for CI environments (only explicit allowlist passes)

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

// ── Always-allowed variables ────────────────────────────────────────────────

static ALWAYS_ALLOWED: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Cross-platform
        "PATH",
        // Windows
        "SYSTEMROOT",
        "COMSPEC",
        "PATHEXT",
        "WINDIR",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "SYSTEMDRIVE",
        // Unix/macOS
        "HOME",
        "LANG",
        "SHELL",
        "TMPDIR",
        "USER",
        "LOGNAME",
        // Terminal
        "TERM",
        "COLORTERM",
        // ARC CLI
        "ARC_CLI_CONFIG",
        "ARC_CLI_DEBUG",
    ]
    .into_iter()
    .collect()
});

// ── Never-allowed variable names ────────────────────────────────────────────

static NEVER_ALLOWED_NAMES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "CLIENT_ID",
        "DB_URI",
        "CONNECTION_STRING",
        "AWS_DEFAULT_REGION",
        "AZURE_CLIENT_ID",
        "AZURE_TENANT_ID",
        "SLACK_WEBHOOK_URL",
        "TWILIO_ACCOUNT_SID",
        "DATABASE_URL",
        "GOOGLE_CLOUD_PROJECT",
        "GOOGLE_CLOUD_ACCOUNT",
        "FIREBASE_PROJECT_ID",
    ]
    .into_iter()
    .collect()
});

// ── Name patterns (regex) ───────────────────────────────────────────────────

static NAME_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "KEY",
        "AUTH",
        "CREDENTIAL",
        "CREDS",
        "PRIVATE",
        "CERT",
    ]
    .iter()
    .map(|p| Regex::new(&format!("(?i){p}")).expect("valid regex"))
    .collect()
});

// ── Value patterns (detect secrets in env var values) ───────────────────────

static VALUE_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // RSA/SSH/PGP private keys
        Regex::new(r"(?i)-----BEGIN (RSA|OPENSSH|EC|PGP) PRIVATE KEY-----").expect("valid"),
        // Certificates
        Regex::new(r"-----BEGIN CERTIFICATE-----").expect("valid"),
        // Credentials in URLs
        Regex::new(r"(?i)(https?|ftp|smtp)://[^:\s]{1,1024}:[^@\s]{1,1024}@").expect("valid"),
        // GitHub tokens (classic, fine-grained, OAuth)
        Regex::new(r"(ghp|gho|ghu|ghs|ghr|github_pat)_[a-zA-Z0-9_]{36,}").expect("valid"),
        // Google API keys
        Regex::new(r"AIzaSy[a-zA-Z0-9_\-]{33}").expect("valid"),
        // AWS Access Key ID
        Regex::new(r"AKIA[A-Z0-9]{16}").expect("valid"),
        // JWT tokens
        Regex::new(r"eyJ[a-zA-Z0-9_\-]{10,}\.[a-zA-Z0-9_\-]{10,}\.[a-zA-Z0-9_\-]{10,}")
            .expect("valid"),
        // Stripe API keys
        Regex::new(r"(s|r)k_(live|test)_[0-9a-zA-Z]{24}").expect("valid"),
        // Slack tokens
        Regex::new(r"xox[abpr]-[a-zA-Z0-9\-]+").expect("valid"),
    ]
});

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SanitizationConfig {
    pub allowed: Vec<String>,
    pub blocked: Vec<String>,
    pub enable_redaction: bool,
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            allowed: Vec::new(),
            blocked: Vec::new(),
            enable_redaction: true,
        }
    }
}

// ── Core sanitization function ──────────────────────────────────────────────

/// Sanitize environment variables, removing any that may contain secrets.
pub fn sanitize_environment(
    env: &HashMap<String, String>,
    config: &SanitizationConfig,
) -> HashMap<String, String> {
    let is_ci = env.contains_key("GITHUB_SHA")
        || env.get("CI").map_or(false, |v| v == "true")
        || env.get("SURFACE").map_or(false, |v| v == "Github");

    if !config.enable_redaction && !is_ci {
        return env.clone();
    }

    let allowed_set: HashSet<String> = config.allowed.iter().map(|k| k.to_uppercase()).collect();
    let blocked_set: HashSet<String> = config.blocked.iter().map(|k| k.to_uppercase()).collect();

    let mut result = HashMap::new();

    for (key, value) in env {
        if !should_redact(key, value, &allowed_set, &blocked_set, is_ci) {
            result.insert(key.clone(), value.clone());
        }
    }

    result
}

fn should_redact(
    key: &str,
    value: &str,
    allowed: &HashSet<String>,
    blocked: &HashSet<String>,
    is_strict: bool,
) -> bool {
    let upper_key = key.to_uppercase();

    // ARC CLI's own vars are always safe
    if upper_key.starts_with("ARC_CLI_") {
        return false;
    }

    // Check value patterns first (catches secrets regardless of key name)
    for pattern in VALUE_PATTERNS.iter() {
        if pattern.is_match(value) {
            return true;
        }
    }

    // Git config vars are safe
    if upper_key.starts_with("GIT_CONFIG_") {
        return false;
    }

    // User-specified allow/block lists
    if allowed.contains(&upper_key) {
        return false;
    }
    if blocked.contains(&upper_key) {
        return true;
    }

    // Built-in always-allowed
    if ALWAYS_ALLOWED.contains(upper_key.as_str()) {
        return false;
    }

    // Built-in never-allowed
    if NEVER_ALLOWED_NAMES.contains(upper_key.as_str()) {
        return true;
    }

    // In strict (CI) mode, block everything not explicitly allowed
    if is_strict {
        return true;
    }

    // Check name patterns
    for pattern in NAME_PATTERNS.iter() {
        if pattern.is_match(&upper_key) {
            return true;
        }
    }

    false
}

/// Get a secure sanitization config that can't be bypassed.
pub fn get_secure_config(requested: &SanitizationConfig) -> SanitizationConfig {
    let safe_allowed: Vec<String> = requested
        .allowed
        .iter()
        .filter(|key| {
            let upper = key.to_uppercase();
            !NEVER_ALLOWED_NAMES.contains(upper.as_str())
                && !NAME_PATTERNS.iter().any(|p| p.is_match(&upper))
        })
        .cloned()
        .collect();

    SanitizationConfig {
        allowed: safe_allowed,
        blocked: requested.blocked.clone(),
        enable_redaction: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocks_github_token_in_value() {
        let mut env = HashMap::new();
        env.insert(
            "MY_VAR".to_string(),
            "ghp_abcdefghijklmnopqrstuvwxyz1234567890".to_string(),
        );
        let result = sanitize_environment(&env, &SanitizationConfig::default());
        assert!(!result.contains_key("MY_VAR"));
    }

    #[test]
    fn test_allows_path() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        let result = sanitize_environment(&env, &SanitizationConfig::default());
        assert!(result.contains_key("PATH"));
    }

    #[test]
    fn test_blocks_name_pattern() {
        let mut env = HashMap::new();
        env.insert("MY_SECRET_KEY".to_string(), "harmless_value".to_string());
        let result = sanitize_environment(&env, &SanitizationConfig::default());
        assert!(!result.contains_key("MY_SECRET_KEY"));
    }

    #[test]
    fn test_blocks_jwt_in_value() {
        let mut env = HashMap::new();
        env.insert(
            "SAFE_NAME".to_string(),
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123def456ghi789".to_string(),
        );
        let result = sanitize_environment(&env, &SanitizationConfig::default());
        assert!(!result.contains_key("SAFE_NAME"));
    }
}
