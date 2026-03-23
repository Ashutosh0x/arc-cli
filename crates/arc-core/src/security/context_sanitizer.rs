// SPDX-License-Identifier: MIT
use regex::Regex;
use std::sync::OnceLock;

/// Sanitizes sensitive secrets from being leaked into LLM context windows.
/// Scrutinizes any text before it gets transmitted to a model.
pub struct SecretSanitizer;

impl SecretSanitizer {
    /// Redacts known secret patterns from the input string.
    pub fn redact(input: &str) -> String {
        static AWS_RE: OnceLock<Regex> = OnceLock::new();
        static JWT_RE: OnceLock<Regex> = OnceLock::new();
        static PRIVATE_KEY_RE: OnceLock<Regex> = OnceLock::new();

        let aws_re = AWS_RE.get_or_init(|| Regex::new(r#"(?i)(AKIA|ASIA)[A-Z0-9]{16}"#).unwrap());
        // Simple JWT pattern
        let jwt_re = JWT_RE.get_or_init(|| {
            Regex::new(r#"ey[A-Za-z0-9-_=]+\.[A-Za-z0-9-_=]+\.?[A-Za-z0-9-_.+/=]*"#).unwrap()
        });
        let pk_re = PRIVATE_KEY_RE.get_or_init(|| {
            Regex::new(
                r#"-----BEGIN (?:RSA|EC|DSA|OPENSSH) PRIVATE KEY-----[\s\S]*?-----END (?:RSA|EC|DSA|OPENSSH) PRIVATE KEY-----"#,
            )
            .unwrap()
        });

        let mut output = input.to_string();
        output = aws_re
            .replace_all(&output, "[REDACTED AWS KEY]")
            .to_string();
        output = jwt_re.replace_all(&output, "[REDACTED JWT]").to_string();
        output = pk_re
            .replace_all(&output, "[REDACTED PRIVATE KEY]")
            .to_string();

        output
    }
}
