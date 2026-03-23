// SPDX-License-Identifier: MIT
//! Config parsing benchmark.
//! Target: <100µs for hierarchical config merge (global + project + local).

use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Write;
use tempfile::TempDir;

fn create_sample_configs(dir: &TempDir) {
    // Global config
    let global_dir = dir.path().join("global");
    std::fs::create_dir_all(&global_dir).unwrap();
    let mut f = std::fs::File::create(global_dir.join("config.toml")).unwrap();
    write!(
        f,
        r#"
[general]
default_provider = "anthropic"
default_model = "claude-sonnet-4-20250514"
theme = "dark"
auto_compact = true
max_context_tokens = 200000

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
max_retries = 3
timeout_seconds = 120

[providers.google]
api_key_env = "GOOGLE_API_KEY"
project_id = "my-project"

[providers.openai]
api_key_env = "OPENAI_API_KEY"
organization = "org-123"

[security]
block_patterns = ["*.env", "*.pem", "id_rsa*"]
sandbox_mode = "permissive"
audit_log = true

[hooks]
enabled = true
security_defaults = true
"#
    )
    .unwrap();

    // Project config
    let project_dir = dir.path().join("project").join(".arc");
    std::fs::create_dir_all(&project_dir).unwrap();
    let mut f = std::fs::File::create(project_dir.join("config.toml")).unwrap();
    write!(
        f,
        r#"
[general]
default_model = "gemini-2.5-pro"

[providers.google]
project_id = "override-project"

[worktree]
auto_cleanup = true
sparse_paths = ["crates/", "src/", "tests/"]
max_worktrees = 5
"#
    )
    .unwrap();
}

fn parse_toml_config(path: &std::path::Path) -> toml::Value {
    let content = std::fs::read_to_string(path).unwrap();
    content.parse::<toml::Value>().unwrap()
}

fn merge_configs(base: &mut toml::Value, overlay: &toml::Value) {
    if let (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) =
        (base, overlay)
    {
        for (key, value) in overlay_table {
            if let Some(existing) = base_table.get_mut(key) {
                if existing.is_table() && value.is_table() {
                    merge_configs(existing, value);
                    continue;
                }
            }
            base_table.insert(key.clone(), value.clone());
        }
    }
}

fn bench_config_parsing(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    create_sample_configs(&dir);

    let global_path = dir.path().join("global").join("config.toml");
    let project_path = dir
        .path()
        .join("project")
        .join(".arc")
        .join("config.toml");

    let mut group = c.benchmark_group("config_parsing");

    group.bench_function("single_file_parse", |b| {
        b.iter(|| {
            let config = parse_toml_config(criterion::black_box(&global_path));
            criterion::black_box(&config);
        });
    });

    group.bench_function("hierarchical_merge", |b| {
        b.iter(|| {
            let mut base = parse_toml_config(&global_path);
            let overlay = parse_toml_config(&project_path);
            merge_configs(&mut base, &overlay);
            criterion::black_box(&base);
        });
    });

    // Benchmark deserialization into typed struct
    group.bench_function("typed_deserialize", |b| {
        let content = std::fs::read_to_string(&global_path).unwrap();
        b.iter(|| {
            let config: toml::Value =
                toml::from_str(criterion::black_box(&content)).unwrap();
            criterion::black_box(&config);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_config_parsing);
criterion_main!(benches);
