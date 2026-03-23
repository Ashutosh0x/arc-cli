// SPDX-License-Identifier: MIT
//! Property tests for config serialization roundtrips.

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestConfig {
    name: String,
    enabled: bool,
    priority: u32,
    timeout_ms: u64,
    tags: Vec<String>,
}

proptest! {
    #[test]
    fn toml_roundtrip(
        name in "[a-zA-Z][a-zA-Z0-9_-]{0,30}",
        enabled in any::<bool>(),
        priority in 0u32..1000,
        timeout_ms in 100u64..60000,
        tags in proptest::collection::vec("[a-z]{1,10}", 0..5),
    ) {
        let config = TestConfig {
            name,
            enabled,
            priority,
            timeout_ms,
            tags,
        };

        let serialized = toml::to_string(&config).unwrap();
        let deserialized: TestConfig = toml::from_str(&serialized).unwrap();

        prop_assert_eq!(&config, &deserialized);
    }

    #[test]
    fn json_roundtrip(
        name in "[a-zA-Z][a-zA-Z0-9_-]{0,30}",
        enabled in any::<bool>(),
        priority in 0u32..1000,
        timeout_ms in 100u64..60000,
    ) {
        let config = TestConfig {
            name,
            enabled,
            priority,
            timeout_ms,
            tags: vec![],
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: TestConfig = serde_json::from_str(&serialized).unwrap();

        prop_assert_eq!(&config, &deserialized);
    }
}
