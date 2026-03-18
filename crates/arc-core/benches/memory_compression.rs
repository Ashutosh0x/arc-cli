use arc_core::memory::compressor::Compressor;
use arc_core::memory::working::MemoryMessage;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tokio::runtime::Runtime;

fn generate_messages(count: usize) -> Vec<MemoryMessage> {
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let content = format!(
            "This is a simulated message number {}. It contains a reasonably long string of text to ensure we use up a realistic amount of allocation and processing time during compression. We want to see how the arena allocator performs and how fast strings can be concatenated.",
            i
        );
        messages.push(MemoryMessage::new(role, content));
    }
    messages
}

fn bench_compression(c: &mut Criterion) {
    let compressor = Compressor::new();
    let runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("Memory Compression");
    group.measurement_time(Duration::from_secs(3));

    for &size in &[10, 50, 100, 500] {
        let raw_messages = generate_messages(size);

        group.bench_with_input(BenchmarkId::new("compress_n_messages", size), &size, |b, &_size| {
            b.to_async(&runtime).iter(|| async {
                compressor.compress(&raw_messages).await.unwrap()
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_compression);
criterion_main!(benches);
