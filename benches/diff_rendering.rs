//! Diff rendering benchmark using syntect.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn generate_unified_diff(hunk_count: usize) -> String {
    let mut diff = String::from("--- a/src/main.rs\n+++ b/src/main.rs\n");

    for h in 0..hunk_count {
        let start = h * 20 + 1;
        diff.push_str(&format!("@@ -{start},10 +{start},12 @@\n"));

        for i in 0..5 {
            diff.push_str(&format!(" fn existing_function_{}() {{}}\n", start + i));
        }
        diff.push_str(&format!("-fn removed_function_{}() {{}}\n", h));
        diff.push_str(&format!("-fn also_removed_{}() {{}}\n", h));
        diff.push_str(&format!("+fn new_function_{}() {{\n", h));
        diff.push_str(&format!("+    println!(\"Added in hunk {}\");\n", h));
        diff.push_str("+}\n");
        diff.push_str(&format!("+fn another_new_{}() {{}}\n", h));

        for i in 5..10 {
            diff.push_str(&format!(" fn existing_function_{}() {{}}\n", start + i));
        }
    }

    diff
}

fn strip_ansi(input: &str) -> String {
    // Simple ANSI strip for measurement
    let mut result = String::with_capacity(input.len());
    let mut in_escape = false;
    for ch in input.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape && ch == 'm' {
            in_escape = false;
        } else if !in_escape {
            result.push(ch);
        }
    }
    result
}

fn colorize_diff_simple(diff: &str) -> String {
    let mut output = String::with_capacity(diff.len() * 2);

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            output.push_str("\x1b[32m"); // Green
            output.push_str(line);
            output.push_str("\x1b[0m\n");
        } else if line.starts_with('-') && !line.starts_with("---") {
            output.push_str("\x1b[31m"); // Red
            output.push_str(line);
            output.push_str("\x1b[0m\n");
        } else if line.starts_with("@@") {
            output.push_str("\x1b[36m"); // Cyan
            output.push_str(line);
            output.push_str("\x1b[0m\n");
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    output
}

fn bench_diff_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_rendering");

    for hunk_count in [1, 5, 20, 100] {
        let diff = generate_unified_diff(hunk_count);

        group.bench_with_input(
            BenchmarkId::new("ansi_colorize", hunk_count),
            &diff,
            |b, diff| {
                b.iter(|| {
                    let colored = colorize_diff_simple(criterion::black_box(diff));
                    criterion::black_box(colored.len());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("line_parse_only", hunk_count),
            &diff,
            |b, diff| {
                b.iter(|| {
                    let lines: Vec<_> = diff.lines().collect();
                    let added = lines.iter().filter(|l| l.starts_with('+')).count();
                    let removed = lines.iter().filter(|l| l.starts_with('-')).count();
                    criterion::black_box((added, removed));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_diff_rendering);
criterion_main!(benches);
