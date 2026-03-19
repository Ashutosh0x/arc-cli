//! Session snapshot benchmark — fork/checkpoint speed.
//! Target: <100ms to snapshot a 50-file project with 200k token conversation.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use sha2::{Digest, Sha256};
use std::io::Write;
use tempfile::TempDir;

fn create_test_project(dir: &TempDir, file_count: usize) {
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    for i in 0..file_count {
        let mut f =
            std::fs::File::create(src_dir.join(format!("module_{i:04}.rs"))).unwrap();
        // ~2KB per file
        for line in 0..50 {
            writeln!(
                f,
                "pub fn function_{line}(x: i32) -> i32 {{ x * {line} + {i} }}"
            )
            .unwrap();
        }
    }

    // Add some config files
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    std::fs::write(dir.path().join("README.md"), "# Test Project\n".repeat(50)).unwrap();
}

fn snapshot_files(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut results = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let content = std::fs::read(entry.path()).unwrap();
            let hash = hex::encode(Sha256::digest(&content));
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap()
                .display()
                .to_string();
            results.push((rel, hash));
        }
    }

    results
}

fn bench_snapshot_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_snapshot");

    for file_count in [10, 50, 100, 500] {
        let dir = TempDir::new().unwrap();
        create_test_project(&dir, file_count);

        group.bench_with_input(
            BenchmarkId::new("snapshot_files", file_count),
            &dir,
            |b, dir| {
                b.iter(|| {
                    let files = snapshot_files(criterion::black_box(dir.path()));
                    criterion::black_box(files.len());
                });
            },
        );
    }

    group.finish();
}

fn bench_snapshot_diff(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    create_test_project(&dir, 50);
    let snapshot_a = snapshot_files(dir.path());

    // Modify 5 files
    for i in 0..5 {
        let path = dir
            .path()
            .join("src")
            .join(format!("module_{i:04}.rs"));
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(f, "// Modified for diff test").unwrap();
    }

    let snapshot_b = snapshot_files(dir.path());

    c.bench_function("snapshot_diff_50_files_5_changed", |b| {
        b.iter(|| {
            let mut changed = Vec::new();
            let mut added = Vec::new();
            let mut removed = Vec::new();

            let a_map: std::collections::HashMap<_, _> =
                snapshot_a.iter().cloned().collect();
            let b_map: std::collections::HashMap<_, _> =
                snapshot_b.iter().cloned().collect();

            for (path, hash) in &b_map {
                match a_map.get(path) {
                    Some(old_hash) if old_hash != hash => changed.push(path.clone()),
                    None => added.push(path.clone()),
                    _ => {}
                }
            }
            for path in a_map.keys() {
                if !b_map.contains_key(path) {
                    removed.push(path.clone());
                }
            }

            criterion::black_box((changed.len(), added.len(), removed.len()));
        });
    });
}

criterion_group!(benches, bench_snapshot_creation, bench_snapshot_diff);
criterion_main!(benches);
