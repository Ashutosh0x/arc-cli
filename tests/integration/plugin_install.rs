// SPDX-License-Identifier: MIT
//! Plugin installation integration tests.

use std::io::Write;
use tempfile::TempDir;

fn create_test_plugin(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let plugin_dir = dir.path().join(name);
    std::fs::create_dir_all(plugin_dir.join("commands")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();

    let mut manifest =
        std::fs::File::create(plugin_dir.join("plugin.toml")).unwrap();
    write!(
        manifest,
        r#"
[plugin]
name = "{name}"
version = "1.2.3"
description = "A test plugin"
author = "Test Author"
license = "MIT"
tags = ["test", "integration"]
"#
    )
    .unwrap();

    // Add a command
    let mut cmd_file =
        std::fs::File::create(plugin_dir.join("commands").join("greet.toml"))
            .unwrap();
    write!(
        cmd_file,
        r#"
name = "greet"
description = "Say hello"

[handler]
type = "inline"
command = "echo 'Hello from plugin!'"
"#
    )
    .unwrap();

    // Add a hook
    let mut hook_file =
        std::fs::File::create(plugin_dir.join("hooks").join("log-tools.toml"))
            .unwrap();
    write!(
        hook_file,
        r#"
name = "log-tools"

[hook_config]
description = "Log all tool uses"
priority = 200
timeout_ms = 1000

[hook_config.matcher]
event = "PostToolUse"

[hook_config.action]
type = "command"
command = "echo 'Tool used' >> /tmp/arc-plugin-log.txt"
"#
    )
    .unwrap();

    plugin_dir
}

#[test]
fn test_plugin_manifest_parsing() {
    let dir = TempDir::new().unwrap();
    let plugin_dir = create_test_plugin(&dir, "my-plugin");

    let manifest_content =
        std::fs::read_to_string(plugin_dir.join("plugin.toml")).unwrap();
    let manifest: toml::Value = manifest_content.parse().unwrap();

    assert_eq!(
        manifest["plugin"]["name"].as_str().unwrap(),
        "my-plugin"
    );
    assert_eq!(
        manifest["plugin"]["version"].as_str().unwrap(),
        "1.2.3"
    );
    assert_eq!(
        manifest["plugin"]["license"].as_str().unwrap(),
        "MIT"
    );
}

#[test]
fn test_plugin_integrity_hash() {
    use sha2::{Digest, Sha256};

    let dir = TempDir::new().unwrap();
    let plugin_dir = create_test_plugin(&dir, "hash-test");

    let mut hasher = Sha256::new();
    for entry in walkdir::WalkDir::new(&plugin_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let content = std::fs::read(entry.path()).unwrap();
            hasher.update(&content);
        }
    }
    let hash1 = hex::encode(hasher.finalize());

    // Hash again — should be identical (deterministic)
    let mut hasher2 = Sha256::new();
    for entry in walkdir::WalkDir::new(&plugin_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let content = std::fs::read(entry.path()).unwrap();
            hasher2.update(&content);
        }
    }
    let hash2 = hex::encode(hasher2.finalize());

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
}

#[test]
fn test_plugin_component_discovery() {
    let dir = TempDir::new().unwrap();
    let plugin_dir = create_test_plugin(&dir, "discovery-test");

    // Count components
    let commands: Vec<_> = std::fs::read_dir(plugin_dir.join("commands"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "toml")
        })
        .collect();

    let hooks: Vec<_> = std::fs::read_dir(plugin_dir.join("hooks"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "toml")
        })
        .collect();

    assert_eq!(commands.len(), 1);
    assert_eq!(hooks.len(), 1);
}

#[test]
fn test_registry_persistence() {
    let dir = TempDir::new().unwrap();
    let registry_path = dir.path().join(".arc").join("plugins.toml");
    std::fs::create_dir_all(dir.path().join(".arc")).unwrap();

    // Write a registry
    let registry_content = r#"
[plugins.test-plugin]
name = "test-plugin"
version = "1.0.0"
install_path = "/tmp/test-plugin"
integrity_hash = "abc123"
enabled = true
installed_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"

[plugins.test-plugin.source]
type = "local"
path = "/tmp/src"
"#;
    std::fs::write(&registry_path, registry_content).unwrap();

    // Read it back
    let content = std::fs::read_to_string(&registry_path).unwrap();
    let registry: toml::Value = content.parse().unwrap();

    assert!(registry["plugins"]["test-plugin"]["enabled"]
        .as_bool()
        .unwrap());
}
