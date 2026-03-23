// SPDX-License-Identifier: MIT
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse as a hook config TOML
        if let Ok(val) = s.parse::<toml::Value>() {
            // Try to extract hook fields
            if let Some(table) = val.as_table() {
                for (_name, hook_val) in table {
                    if let Some(hook_table) = hook_val.as_table() {
                        let _ = hook_table.get("event");
                        let _ = hook_table.get("tool_pattern");
                        if let Some(pattern) = hook_table
                            .get("tool_pattern")
                            .and_then(|v| v.as_str())
                        {
                            // Attempt regex compilation — must not panic
                            let _ = regex::Regex::new(pattern);
                        }
                    }
                }
            }
        }
    }
});
