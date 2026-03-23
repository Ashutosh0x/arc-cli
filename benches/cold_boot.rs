// SPDX-License-Identifier: MIT
//! Cold boot benchmark — measures total CLI startup time.
//! Target: <20ms to interactive prompt.
//! Competitor reference: Claude Code ~800ms, Aider ~2s.

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Instant;

fn bench_cold_boot(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_boot");
    group.sample_size(20); // Fewer samples since each spawns a process
    group.measurement_time(std::time::Duration::from_secs(30));

    // Measure time to run `arc --version` (fastest possible invocation)
    group.bench_function("arc_version", |b| {
        b.iter(|| {
            let start = Instant::now();
            let output = std::process::Command::new("cargo")
                .args(["run", "--release", "-q", "--", "--version"])
                .output();
            let elapsed = start.elapsed();
            criterion::black_box(elapsed);
        });
    });

    // Measure time to run `arc doctor` (loads config + checks credentials)
    group.bench_function("arc_doctor", |b| {
        b.iter(|| {
            let start = Instant::now();
            let output = std::process::Command::new("cargo")
                .args(["run", "--release", "-q", "--", "doctor"])
                .output();
            let elapsed = start.elapsed();
            criterion::black_box(elapsed);
        });
    });

    group.finish();
}

/// Measures component initialization times individually.
fn bench_component_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_init");

    group.bench_function("toml_parse_empty", |b| {
        b.iter(|| {
            let val: toml::Value = "".parse().unwrap_or(toml::Value::Table(Default::default()));
            criterion::black_box(val);
        });
    });

    group.bench_function("uuid_generate", |b| {
        b.iter(|| {
            let id = uuid::Uuid::new_v4();
            criterion::black_box(id);
        });
    });

    group.bench_function("redb_create_temp", |b| {
        b.iter_batched(
            || tempfile::NamedTempFile::new().unwrap(),
            |file| {
                let db = redb::Database::create(file.path()).unwrap();
                criterion::black_box(db);
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_cold_boot, bench_component_init);
criterion_main!(benches);
