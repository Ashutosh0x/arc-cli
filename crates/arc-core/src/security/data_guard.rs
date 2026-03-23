// SPDX-License-Identifier: MIT
//! Data privacy & leak prevention.
//! Scans outgoing prompts and incoming responses for sensitive data.

use regex::Regex;
use std::sync::OnceLock;
use tracing::warn;

static PII_PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

fn get_pii_patterns() -> &'static Vec<(Regex, &'static str)> {
    PII_PATTERNS.get_or_init(|| {
        let patterns = [
            (r"\b\d{3}-\d{2}-\d{4}\b", "US SSN"),
            (r"\b\d{16}\b", "Credit card number"),
            (
                r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",
                "Credit card (formatted)",
            ),
            (
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
                "Email address",
            ),
            (
                r"(?i)\b(password|passwd|pwd)\s*[:=]\s*\S+",
                "Plaintext password",
            ),
            (r"sk-ant-[A-Za-z0-9_-]{20,}", "Anthropic API key"),
            (r"sk-[A-Za-z0-9]{20,}", "OpenAI API key"),
            (r"AIza[A-Za-z0-9_-]{35}", "Google API key"),
            (r"ghp_[A-Za-z0-9]{36}", "GitHub PAT"),
            (r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----", "Private key"),
        ];

        patterns
            .iter()
            .filter_map(|(p, name)| Regex::new(p).ok().map(|re| (re, *name)))
            .collect()
    })
}

/// Scan text for PII and sensitive data. Returns list of findings.
pub fn scan_for_sensitive_data(text: &str) -> Vec<String> {
    let patterns = get_pii_patterns();
    let mut findings = Vec::new();

    for (re, name) in patterns.iter() {
        if re.is_match(text) {
            findings.push(format!("Sensitive data detected: {name}"));
            warn!("Data guard: {name} detected in content");
        }
    }

    findings
}

/// Redact sensitive data from text, replacing matches with [REDACTED].
pub fn redact_sensitive_data(text: &str) -> String {
    let patterns = get_pii_patterns();
    let mut result = text.to_string();

    for (re, _name) in patterns.iter() {
        result = re.replace_all(&result, "[REDACTED]").to_string();
    }

    result
}
