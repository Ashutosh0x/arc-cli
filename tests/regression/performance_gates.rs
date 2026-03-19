//! Performance gate tests — these FAIL CI if performance regresses.
//! Run with: cargo test --test regression -- --ignored

use std::time::Instant;

/// Gate: Config parsing must complete in under 1ms.
#[test]
#[ignore] // Run in CI with --ignored flag
fn gate_config_parse_under_1ms() {
    let config = r#"
[general]
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"
theme = "dark"

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
max_retries = 3

[security]
block_patterns = ["*.env", "*.pem"]
"#;

    let iterations = 1000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _: toml::Value = config.parse().unwrap();
    }
    let elapsed = start.elapsed();
    let per_parse = elapsed / iterations;

    assert!(
        per_parse.as_micros() < 1000,
        "Config parse took {}µs (limit: 1000µs)",
        per_parse.as_micros()
    );
}

/// Gate: UUID generation must be under 1µs.
#[test]
#[ignore]
fn gate_uuid_generation_under_1us() {
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = uuid::Uuid::new_v4();
    }
    let elapsed = start.elapsed();
    let per_gen = elapsed / iterations as u32;

    assert!(
        per_gen.as_nanos() < 1000,
        "UUID generation took {}ns (limit: 1000ns)",
        per_gen.as_nanos()
    );
}

/// Gate: JSON event serialization must be under 5µs.
#[test]
#[ignore]
fn gate_event_serialization_under_5us() {
    let event = serde_json::json!({
        "event": "PreToolUse",
        "payload": {
            "session_id": uuid::Uuid::new_v4().to_string(),
            "tool_name": "bash",
            "tool_input": {"command": "cargo test"},
            "target_path": null,
            "command": "cargo test"
        }
    });

    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let json = serde_json::to_string(&event).unwrap();
        std::hint::black_box(json);
    }
    let elapsed = start.elapsed();
    let per_ser = elapsed / iterations as u32;

    assert!(
        per_ser.as_micros() < 5,
        "Event serialization took {}µs (limit: 5µs)",
        per_ser.as_micros()
    );
}

/// Gate: SHA-256 file hash for 1MB file under 5ms.
#[test]
#[ignore]
fn gate_file_hash_1mb_under_5ms() {
    use sha2::{Digest, Sha256};

    let data = vec![0xABu8; 1_048_576]; // 1MB

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let hash = Sha256::digest(&data);
        std::hint::black_box(hash);
    }
    let elapsed = start.elapsed();
    let per_hash = elapsed / iterations as u32;

    assert!(
        per_hash.as_millis() < 5,
        "1MB SHA-256 took {}ms (limit: 5ms)",
        per_hash.as_millis()
    );
}

/// Gate: Regex hook matching for 100 hooks under 100µs.
#[test]
#[ignore]
fn gate_hook_matching_100_hooks_under_100us() {
    let hooks: Vec<(String, regex::Regex)> = (0..100)
        .map(|i| {
            let pattern = format!("^tool_{}$", i % 10);
            (format!("hook_{i}"), regex::Regex::new(&pattern).unwrap())
        })
        .collect();

    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _matches: Vec<_> = hooks
            .iter()
            .filter(|(_, re)| re.is_match("tool_5"))
            .map(|(name, _)| name.as_str())
            .collect();
        std::hint::black_box(&_matches);
    }
    let elapsed = start.elapsed();
    let per_match = elapsed / iterations as u32;

    assert!(
        per_match.as_micros() < 100,
        "100-hook matching took {}µs (limit: 100µs)",
        per_match.as_micros()
    );
}
