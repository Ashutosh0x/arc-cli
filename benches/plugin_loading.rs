//! Plugin manifest loading and registry operations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::io::Write;
use tempfile::TempDir;

fn create_plugin_dir(dir: &TempDir, plugin_name: &str, hook_count: usize) {
    let plugin_dir = dir.path().join(plugin_name);
    std::fs::create_dir_all(plugin_dir.join("commands")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("hooks")).unwrap();
    std::fs::create_dir_all(plugin_dir.join("agents")).unwrap();

    let mut manifest =
        std::fs::File::create(plugin_dir.join("plugin.toml")).unwrap();
    write!(
        manifest,
        r#"
[plugin]
name = "{plugin_name}"
version = "1.0.0"
description = "Test plugin with {hook_count} hooks"
author = "Test Author"
license = "MIT"
tags = ["test", "benchmark"]
"#
    )
    .unwrap();

    for i in 0..hook_count {
        let mut hook_file =
            std::fs::File::create(plugin_dir.join("hooks").join(format!("hook_{i}.toml")))
                .unwrap();
        write!(
            hook_file,
            r#"
name = "hook_{i}"

[hook_config]
description = "Test hook {i}"
priority = {priority}
timeout_ms = 5000

[hook_config.matcher]
event = "PreToolUse"
tool_pattern = "^bash$"

[hook_config.action]
type = "command"
command = "exit 0"
"#,
            i = i,
            priority = 100 + i
        )
        .unwrap();
    }
}

fn bench_plugin_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("plugin_loading");

    for hook_count in [1, 5, 20, 50] {
        let dir = TempDir::new().unwrap();
        create_plugin_dir(&dir, "test-plugin", hook_count);

        group.bench_with_input(
            BenchmarkId::new("load_manifest", hook_count),
            &dir,
            |b, dir| {
                b.iter(|| {
                    let manifest_path =
                        dir.path().join("test-plugin").join("plugin.toml");
                    let content =
                        std::fs::read_to_string(criterion::black_box(&manifest_path))
                            .unwrap();
                    let parsed: toml::Value = content.parse().unwrap();
                    criterion::black_box(parsed);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("integrity_hash", hook_count),
            &dir,
            |b, dir| {
                b.iter(|| {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    for entry in walkdir::WalkDir::new(dir.path().join("test-plugin"))
                        .sort_by_file_name()
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_type().is_file() {
                            let content = std::fs::read(entry.path()).unwrap();
                            hasher.update(&content);
                        }
                    }
                    let hash = hex::encode(hasher.finalize_reset());
                    criterion::black_box(hash);
                });
            },
        );
    }

    group.finish();
}

fn bench_registry_operations(c: &mut Criterion) {
    c.bench_function("registry_serialize_50_plugins", |b| {
        let mut plugins = std::collections::HashMap::new();
        for i in 0..50 {
            plugins.insert(
                format!("plugin-{i}"),
                serde_json::json!({
                    "name": format!("plugin-{i}"),
                    "version": "1.0.0",
                    "enabled": true,
                    "install_path": format!("/home/user/.arc/plugins/plugin-{i}"),
                    "integrity_hash": format!("{:064x}", i * 12345),
                }),
            );
        }

        b.iter(|| {
            let json = serde_json::to_string(criterion::black_box(&plugins)).unwrap();
            criterion::black_box(json.len());
        });
    });
}

criterion_group!(benches, bench_plugin_loading, bench_registry_operations);
criterion_main!(benches);
