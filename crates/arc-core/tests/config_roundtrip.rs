//! Config serialization round-trip tests.

use arc_core::config::ArcConfig;

#[test]
fn default_config_roundtrip() {
    let config = ArcConfig::default();
    let toml_str = toml::to_string_pretty(&config).expect("serialize");
    let parsed: ArcConfig = toml::from_str(&toml_str).expect("deserialize");

    // Verify key fields survived the round-trip.
    assert_eq!(config.default_provider, parsed.default_provider);
    assert_eq!(config.default_model, parsed.default_model);
}

#[test]
fn config_with_all_providers() {
    let toml_input = r#"
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"

[routing]
strategy = "fallback-chain"
chain = ["anthropic", "openai", "ollama"]

[providers.anthropic]
enabled = true

[providers.openai]
enabled = true

[providers.ollama]
enabled = true
base_url = "http://localhost:11434"
"#;

    let config: ArcConfig = toml::from_str(toml_input).expect("parse config");
    assert_eq!(config.default_provider.as_deref(), Some("anthropic"));

    // Roundtrip back.
    let serialized = toml::to_string_pretty(&config).expect("serialize");
    let reparsed: ArcConfig = toml::from_str(&serialized).expect("re-parse");
    assert_eq!(config.default_provider, reparsed.default_provider);
}

#[test]
fn config_missing_optional_fields() {
    let minimal = r#"
default_provider = "ollama"
"#;

    let config: ArcConfig = toml::from_str(minimal).expect("parse minimal");
    assert_eq!(config.default_provider.as_deref(), Some("ollama"));
    // All other fields should have sensible defaults.
}
