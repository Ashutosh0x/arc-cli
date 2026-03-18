use criterion::{black_box, criterion_group, criterion_main, Criterion};
use arc_providers::streaming::SseDecoder;
use futures::StreamExt;
use tokio_util::codec::FramedRead;

fn bench_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("Streaming");
    
    let simulated_sse = b"data: {\"choices\": [{\"delta\": {\"content\": \"hello\"}}]}\n\n".repeat(100);

    group.bench_function("zero_copy_parse", |b| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        b.to_async(&rt).iter(|| async {
            let reader = &simulated_sse[..];
            let mut stream = FramedRead::new(reader, SseDecoder);
            
            let mut parse_count = 0;
            while let Some(Ok(chunk)) = stream.next().await {
                black_box(chunk);
                parse_count += 1;
            }
            black_box(parse_count);
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_streaming);
criterion_main!(benches);
