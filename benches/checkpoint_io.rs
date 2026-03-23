// SPDX-License-Identifier: MIT
//! Checkpoint I/O benchmark — redb write/read speed.
//! Target: <50ms for 200k token session checkpoint.

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use redb::{Database, ReadableTable, TableDefinition};
use tempfile::NamedTempFile;

const CHECKPOINTS: TableDefinition<&str, &[u8]> = TableDefinition::new("checkpoints");

fn generate_session_data(token_count: usize) -> Vec<u8> {
    // Simulate a conversation with N tokens (~4 chars per token)
    let avg_chars_per_token = 4;
    let total_chars = token_count * avg_chars_per_token;

    let mut data = Vec::with_capacity(total_chars + 1024);

    // JSON-ish structure simulating messages
    data.extend_from_slice(b"{\"messages\":[");
    let words = [
        "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
        "function", "return", "async", "await", "struct", "impl", "pub",
        "let", "mut", "const", "match", "enum", "trait", "type", "where",
    ];

    let mut char_count = 0;
    let mut msg_idx = 0;
    while char_count < total_chars {
        if msg_idx > 0 {
            data.push(b',');
        }
        let role = if msg_idx % 2 == 0 { "user" } else { "assistant" };
        let content_start = format!("{{\"role\":\"{role}\",\"content\":\"");
        data.extend_from_slice(content_start.as_bytes());

        // Fill with words
        let msg_tokens = std::cmp::min(500, (total_chars - char_count) / 4);
        for t in 0..msg_tokens {
            if t > 0 {
                data.push(b' ');
            }
            let word = words[(msg_idx * 7 + t) % words.len()];
            data.extend_from_slice(word.as_bytes());
            char_count += word.len() + 1;
        }

        data.extend_from_slice(b"\"}");
        msg_idx += 1;
    }

    data.extend_from_slice(b"]}");
    data
}

fn bench_checkpoint_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_io");

    for token_count in [1_000, 10_000, 50_000, 200_000, 500_000] {
        let data = generate_session_data(token_count);
        let data_size = data.len() as u64;

        group.throughput(Throughput::Bytes(data_size));

        // Write benchmark
        group.bench_with_input(
            BenchmarkId::new("redb_write", token_count),
            &data,
            |b, data| {
                b.iter_batched(
                    || {
                        let file = NamedTempFile::new().unwrap();
                        let db = Database::create(file.path()).unwrap();
                        (db, file)
                    },
                    |(db, _file)| {
                        let write_txn = db.begin_write().unwrap();
                        {
                            let mut table = write_txn.open_table(CHECKPOINTS).unwrap();
                            table.insert("checkpoint_001", data.as_slice()).unwrap();
                        }
                        write_txn.commit().unwrap();
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Read benchmark
        group.bench_with_input(
            BenchmarkId::new("redb_read", token_count),
            &data,
            |b, data| {
                // Setup: create DB with data
                let file = NamedTempFile::new().unwrap();
                let db = Database::create(file.path()).unwrap();
                {
                    let write_txn = db.begin_write().unwrap();
                    {
                        let mut table = write_txn.open_table(CHECKPOINTS).unwrap();
                        table.insert("checkpoint_001", data.as_slice()).unwrap();
                    }
                    write_txn.commit().unwrap();
                }

                b.iter(|| {
                    let read_txn = db.begin_read().unwrap();
                    let table = read_txn.open_table(CHECKPOINTS).unwrap();
                    let value = table.get("checkpoint_001").unwrap().unwrap();
                    let bytes = value.value();
                    criterion::black_box(bytes.len());
                });
            },
        );
    }

    group.finish();
}

fn bench_multi_checkpoint_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint_multi_write");

    // Simulate writing 10 incremental checkpoints
    let checkpoints: Vec<Vec<u8>> = (0..10)
        .map(|i| generate_session_data(5000 * (i + 1)))
        .collect();

    group.bench_function("10_incremental_checkpoints", |b| {
        b.iter_batched(
            || {
                let file = NamedTempFile::new().unwrap();
                let db = Database::create(file.path()).unwrap();
                (db, file)
            },
            |(db, _file)| {
                for (i, data) in checkpoints.iter().enumerate() {
                    let write_txn = db.begin_write().unwrap();
                    {
                        let mut table = write_txn.open_table(CHECKPOINTS).unwrap();
                        let key = format!("checkpoint_{:03}", i);
                        table.insert(key.as_str(), data.as_slice()).unwrap();
                    }
                    write_txn.commit().unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_checkpoint_io, bench_multi_checkpoint_write);
criterion_main!(benches);
