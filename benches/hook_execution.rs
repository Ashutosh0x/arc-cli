// SPDX-License-Identifier: MIT
//! Hook engine dispatch latency benchmark.
//! Target: <50ms per hook (vs Claude Code's ~200ms).
//! Target: <5ms for hook matching (no execution).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::io::Write;
use tempfile::TempDir;

// Inline minimal hook structures for benchmarking
// (In production, these come from arc-hooks)

mod hook_bench {
    use regex::Regex;
    use std::collections::HashMap;

    pub struct HookMatcher {
        pub event: String,
        pub tool_pattern: Option<Regex>,
    }

    impl HookMatcher {
        pub fn matches(&self, event_name: &str, tool_name: Option<&str>) -> bool {
            if self.event != event_name {
                return false;
            }
            match (&self.tool_pattern, tool_name) {
                (Some(pattern), Some(tool)) => pattern.is_match(tool),
                (Some(_), None) => false,
                (None, _) => true,
            }
        }
    }

    pub struct HookConfig {
        pub hooks: Vec<(String, HookMatcher)>,
    }

    impl HookConfig {
        pub fn matching_hooks<'a>(
            &'a self,
            event_name: &str,
            tool_name: Option<&str>,
        ) -> Vec<&'a str> {
            self.hooks
                .iter()
                .filter(|(_, m)| m.matches(event_name, tool_name))
                .map(|(name, _)| name.as_str())
                .collect()
        }
    }

    pub fn build_test_config(hook_count: usize) -> HookConfig {
        let events = [
            "SessionStart",
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "UserPromptSubmit",
        ];
        let tools = [
            "bash",
            "file_write",
            "file_read",
            "grep",
            "shell",
            "python",
        ];

        let mut hooks = Vec::with_capacity(hook_count);
        for i in 0..hook_count {
            let event = events[i % events.len()].to_string();
            let pattern = if i % 3 == 0 {
                Some(
                    Regex::new(&format!("^{}$", tools[i % tools.len()])).unwrap(),
                )
            } else {
                None
            };

            hooks.push((
                format!("hook_{:04}", i),
                HookMatcher {
                    event,
                    tool_pattern: pattern,
                },
            ));
        }

        HookConfig { hooks }
    }
}

fn bench_hook_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("hook_matching");

    for hook_count in [5, 20, 50, 100, 500] {
        let config = hook_bench::build_test_config(hook_count);

        group.bench_with_input(
            BenchmarkId::new("match_PreToolUse_bash", hook_count),
            &config,
            |b, config| {
                b.iter(|| {
                    let matches = config.matching_hooks(
                        criterion::black_box("PreToolUse"),
                        criterion::black_box(Some("bash")),
                    );
                    criterion::black_box(&matches);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("match_SessionStart_none", hook_count),
            &config,
            |b, config| {
                b.iter(|| {
                    let matches = config.matching_hooks(
                        criterion::black_box("SessionStart"),
                        criterion::black_box(None),
                    );
                    criterion::black_box(&matches);
                });
            },
        );
    }

    group.finish();
}

fn bench_hook_command_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("hook_command_execution");

    // Benchmark the overhead of spawning a trivial shell command
    // (the hook itself — not the child process work)
    group.bench_function("spawn_trivial_command", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let output = tokio::process::Command::new("true")
                    .output()
                    .await
                    .unwrap();
                criterion::black_box(output.status.code());
            });
        });
    });

    // Benchmark piping JSON stdin to a command
    group.bench_function("pipe_json_stdin", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let json = r#"{"event":"PreToolUse","payload":{"tool_name":"bash","command":"ls -la"}}"#;

        b.iter(|| {
            rt.block_on(async {
                use tokio::io::AsyncWriteExt;

                let mut child = tokio::process::Command::new("cat")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();

                if let Some(mut stdin) = child.stdin.take() {
                    stdin
                        .write_all(criterion::black_box(json.as_bytes()))
                        .await
                        .unwrap();
                    drop(stdin);
                }

                let output = child.wait_with_output().await.unwrap();
                criterion::black_box(output.stdout.len());
            });
        });
    });

    // Benchmark JSON serialization (done once per event, not per hook)
    group.bench_function("serialize_event_payload", |b| {
        let payload = serde_json::json!({
            "event": "PreToolUse",
            "payload": {
                "session_id": "550e8400-e29b-41d4-a716-446655440000",
                "tool_name": "bash",
                "tool_input": {
                    "command": "cargo test --release -- --test-threads=4"
                },
                "target_path": null,
                "command": "cargo test --release -- --test-threads=4"
            }
        });

        b.iter(|| {
            let json = serde_json::to_string(criterion::black_box(&payload)).unwrap();
            criterion::black_box(json.len());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hook_matching, bench_hook_command_execution);
criterion_main!(benches);
