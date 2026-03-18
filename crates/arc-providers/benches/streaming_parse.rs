use criterion::{black_box, criterion_group, criterion_main, Criterion};
use memchr::memmem;

// A sample chunk of SSE traffic typical of LLM streaming
const SSE_PAYLOAD: &[u8] = b"data: {\"choices\": [{\"delta\": {\"content\": \"hello\"}}]}\n\ndata: {\"choices\": [{\"delta\": {\"content\": \" world\"}}]}\n\n";

/// Simulates the exact logic in arc-providers SseStream natively
fn zero_copy_parse(buffer: &[u8]) -> Vec<&[u8]> {
    let mut results = Vec::with_capacity(2000);
    let mut offset = 0;
    
    while let Some(idx) = memmem::find(&buffer[offset..], b"\n\n") {
        let chunk = &buffer[offset..offset + idx + 2];
        if chunk.starts_with(b"data: ") {
            let json_slice = &chunk[6..chunk.len() - 2];
            results.push(json_slice);
        }
        offset += idx + 2;
    }
    
    results
}

/// Simulates traditional Python/Node.js or naive Rust line-parsing
fn standard_parse(buffer: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(buffer);
    let mut results = Vec::with_capacity(2000);
    
    for block in s.split("\n\n") {
        if block.starts_with("data: ") {
            let json_str = &block[6..];
            results.push(json_str.to_string());
        }
    }
    
    results
}

fn bench_sse_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("SSE Streaming Parser");
    
    // Create a 1MB payload to measure throughput and allocation bottlenecks accurately
    let mut large_payload = Vec::with_capacity(1_000_000);
    for _ in 0..10000 {
        large_payload.extend_from_slice(SSE_PAYLOAD);
    }
    
    group.bench_function("Zero-Copy (memchr)", |b| {
        b.iter(|| zero_copy_parse(black_box(&large_payload)))
    });
    
    group.bench_function("Standard (String::split) (Naive)", |b| {
        b.iter(|| standard_parse(black_box(&large_payload)))
    });
    
    group.finish();
}

criterion_group!(benches, bench_sse_parser);
criterion_main!(benches);
