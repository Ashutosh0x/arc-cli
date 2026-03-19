#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(val) = s.parse::<toml::Value>() {
            // Validate expected plugin manifest structure
            if let Some(plugin) = val.get("plugin").and_then(|v| v.as_table()) {
                let _ = plugin.get("name").and_then(|v| v.as_str());
                let _ = plugin.get("version").and_then(|v| v.as_str());
                let _ = plugin.get("description").and_then(|v| v.as_str());
            }
        }
    }
});
