// SPDX-License-Identifier: MIT
//! Memory/context compression benchmark.
//! Measures compression ratio and speed for conversation compaction.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn generate_conversation(turn_count: usize) -> String {
    let mut conversation = String::new();
    let code_samples = [
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
        "pub struct Config {\n    pub name: String,\n    pub value: i64,\n}\n",
        "async fn fetch_data(url: &str) -> Result<Response, Error> {\n    let client = Client::new();\n    client.get(url).send().await\n}\n",
        "impl Iterator for MyIter {\n    type Item = u32;\n    fn next(&mut self) -> Option<Self::Item> {\n        self.pos += 1;\n        Some(self.pos)\n    }\n}\n",
    ];

    for i in 0..turn_count {
        // User message
        conversation.push_str(&format!(
            "<|user|>\nPlease implement feature #{} for the project. It should handle edge cases properly and include tests.\n",
            i
        ));

        // Assistant message with code
        conversation.push_str("<|assistant|>\nI'll implement that feature. Here's the code:\n\n```rust\n");
        conversation.push_str(code_samples[i % code_samples.len()]);
        conversation.push_str("```\n\nI've also added tests:\n\n```rust\n#[test]\nfn test_feature() {\n    assert!(true);\n}\n```\n\n");
    }

    conversation
}

/// Simple extractive compressor: keeps first/last N turns and summarizes middle.
fn compress_extractive(conversation: &str, keep_turns: usize) -> String {
    let turns: Vec<&str> = conversation.split("<|user|>").collect();
    let total = turns.len();

    if total <= keep_turns * 2 {
        return conversation.to_string();
    }

    let mut result = String::with_capacity(conversation.len() / 3);

    // Keep first N turns
    for turn in turns.iter().take(keep_turns) {
        result.push_str("<|user|>");
        result.push_str(turn);
    }

    // Summary of middle
    let skipped = total - keep_turns * 2;
    result.push_str(&format!(
        "\n<|system|>\n[{} turns summarized: discussed implementation of features, wrote code, ran tests]\n",
        skipped
    ));

    // Keep last N turns
    for turn in turns.iter().skip(total - keep_turns) {
        result.push_str("<|user|>");
        result.push_str(turn);
    }

    result
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_compression");

    for turn_count in [10, 50, 100, 500] {
        let conversation = generate_conversation(turn_count);
        let input_size = conversation.len() as u64;

        group.throughput(Throughput::Bytes(input_size));

        group.bench_with_input(
            BenchmarkId::new("extractive_compress", turn_count),
            &conversation,
            |b, conv| {
                b.iter(|| {
                    let compressed =
                        compress_extractive(criterion::black_box(conv), 5);
                    criterion::black_box(compressed.len());
                });
            },
        );
    }

    // Measure compression ratios
    group.bench_function("compression_ratio_report", |b| {
        b.iter(|| {
            for turns in [10, 50, 100, 500] {
                let original = generate_conversation(turns);
                let compressed = compress_extractive(&original, 5);
                let ratio = compressed.len() as f64 / original.len() as f64;
                criterion::black_box(ratio);
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_compression);
criterion_main!(benches);
