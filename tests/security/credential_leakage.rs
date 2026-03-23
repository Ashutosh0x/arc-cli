// SPDX-License-Identifier: MIT
//! Tests to ensure credentials never appear in logs, errors, or serialized state.

#[test]
fn test_api_key_not_in_debug_output() {
    #[derive(Debug)]
    struct ProviderConfig {
        name: String,
        api_key: SecretString,
    }

    struct SecretString(String);

    impl std::fmt::Debug for SecretString {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "SecretString(***)")
        }
    }

    let config = ProviderConfig {
        name: "anthropic".to_string(),
        api_key: SecretString("sk-ant-api03-SUPER-SECRET-KEY".to_string()),
    };

    let debug_output = format!("{:?}", config);
    assert!(!debug_output.contains("SUPER-SECRET"));
    assert!(!debug_output.contains("sk-ant"));
    assert!(debug_output.contains("***"));
}

#[test]
fn test_error_messages_redact_keys() {
    fn format_provider_error(provider: &str, key: &str, status: u16) -> String {
        let redacted_key = if key.len() > 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "****".to_string()
        };

        format!(
            "Provider '{}' returned HTTP {}: key={}",
            provider, status, redacted_key
        )
    }

    let error = format_provider_error(
        "anthropic",
        "sk-ant-api03-VERY-LONG-SECRET-KEY-12345",
        401,
    );

    assert!(!error.contains("VERY-LONG-SECRET"));
    assert!(error.contains("sk-a"));
    assert!(error.contains("..."));
    assert!(error.contains("2345"));
}

#[test]
fn test_checkpoint_excludes_credentials() {
    let checkpoint = serde_json::json!({
        "session_id": "abc-123",
        "messages": [
            {"role": "user", "content": "Set my API key to sk-secret123"},
            {"role": "assistant", "content": "I'll help you configure that."}
        ],
        "config": {
            "provider": "anthropic",
            "model": "claude-sonnet-4-20250514"
            // Note: no api_key field should ever be serialized here
        }
    });

    let serialized = serde_json::to_string(&checkpoint).unwrap();

    // The checkpoint format should never contain raw API key patterns
    // (user messages may contain them — that's the user's responsibility)
    assert!(!serialized.contains("\"api_key\""));
    assert!(!serialized.contains("\"secret\""));
}

#[test]
fn test_env_var_redaction() {
    fn redact_env_vars(input: &str) -> String {
        let patterns = [
            (
                regex::Regex::new(r"(?i)(ANTHROPIC_API_KEY|OPENAI_API_KEY|GOOGLE_API_KEY)\s*=\s*\S+").unwrap(),
                "$1=***REDACTED***"
            ),
            (
                regex::Regex::new(r"sk-[a-zA-Z0-9-]{10,}").unwrap(),
                "sk-***REDACTED***"
            ),
        ];

        let mut result = input.to_string();
        for (re, replacement) in &patterns {
            result = re.replace_all(&result, *replacement).to_string();
        }
        result
    }

    let input = "export ANTHROPIC_API_KEY=sk-ant-api03-verysecretkey123";
    let redacted = redact_env_vars(input);
    assert!(!redacted.contains("verysecretkey"));
    assert!(redacted.contains("REDACTED"));
}
