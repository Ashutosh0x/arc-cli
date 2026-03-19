//! SSE Parser Benchmark — the single most latency-critical path.
//!
//! Target: 0 heap allocations per event line.
//! Target: >2 GB/s throughput on synthetic SSE streams.
//! Comparison baseline: naive String::split_once approach.

use criterion::{
    black_box, criterion_group, criterion_main,
    BatchSize, BenchmarkId, Criterion, Throughput,
};
use std::hint::black_box as bb;

/// Simulates the zero-copy SSE parser from arc-providers.
/// In production this uses memchr SIMD acceleration.
mod sse_parser {
    /// A parsed SSE event referencing borrowed data (zero-copy).
    #[derive(Debug)]
    pub struct SseEvent<'a> {
        pub event_type: &'a str,
        pub data: &'a str,
        pub id: Option<&'a str>,
    }

    /// Parse a single SSE frame from a byte slice.
    /// Returns (event, bytes_consumed).
    #[inline]
    pub fn parse_frame(buf: &[u8]) -> Option<(SseEvent<'_>, usize)> {
        // Find double newline (end of frame)
        let frame_end = find_double_newline(buf)?;
        let frame = std::str::from_utf8(&buf[..frame_end]).ok()?;

        let mut event_type = "message";
        let mut data_start = None;
        let mut data_end = 0;
        let mut id = None;

        for line in frame.lines() {
            if let Some(value) = line.strip_prefix("event: ") {
                event_type = value.trim();
            } else if let Some(value) = line.strip_prefix("data: ") {
                if data_start.is_none() {
                    // Record position within the frame string
                    let offset = value.as_ptr() as usize - frame.as_ptr() as usize;
                    data_start = Some(offset);
                }
                data_end = value.as_ptr() as usize - frame.as_ptr() as usize + value.len();
            } else if let Some(value) = line.strip_prefix("id: ") {
                id = Some(value.trim());
            }
        }

        let data = match data_start {
            Some(start) => &frame[start..data_end],
            None => "",
        };

        Some((
            SseEvent {
                event_type,
                data,
                id,
            },
            frame_end + 2, // +2 for \n\n
        ))
    }

    /// Parse all SSE frames from a buffer. Returns events + total bytes consumed.
    pub fn parse_all(buf: &[u8]) -> (Vec<SseEvent<'_>>, usize) {
        let mut events = Vec::new();
        let mut offset = 0;

        while offset < buf.len() {
            match parse_frame(&buf[offset..]) {
                Some((event, consumed)) => {
                    events.push(event);
                    offset += consumed;
                }
                None => break,
            }
        }

        (events, offset)
    }

    /// Find double newline using memchr-style scanning.
    #[inline]
    fn find_double_newline(buf: &[u8]) -> Option<usize> {
        if buf.len() < 2 {
            return None;
        }

        // Use memchr for SIMD-accelerated search
        let mut pos = 0;
        while pos < buf.len() - 1 {
            if let Some(idx) = memchr_naive(b'\n', &buf[pos..]) {
                let abs = pos + idx;
                if abs + 1 < buf.len() && buf[abs + 1] == b'\n' {
                    return Some(abs);
                }
                pos = abs + 1;
            } else {
                break;
            }
        }
        None
    }

    #[inline]
    fn memchr_naive(needle: u8, haystack: &[u8]) -> Option<usize> {
        // In production, use the `memchr` crate for SIMD.
        // This benchmark intentionally uses a naive fallback
        // to show the baseline before SIMD optimization.
        haystack.iter().position(|&b| b == needle)
    }

    /// Naive baseline: uses String allocation per event.
    pub fn parse_frame_naive(input: &str) -> Option<(String, String)> {
        let frame_end = input.find("\n\n")?;
        let frame = &input[..frame_end];

        let mut event_type = String::from("message");
        let mut data = String::new();

        for line in frame.lines() {
            if let Some(v) = line.strip_prefix("event: ") {
                event_type = v.to_string(); // ALLOCATION
            } else if let Some(v) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n'); // ALLOCATION
                }
                data.push_str(v); // ALLOCATION
            }
        }

        Some((event_type, data))
    }
}

fn generate_sse_stream(event_count: usize) -> Vec<u8> {
    let mut stream = Vec::with_capacity(event_count * 200);
    for i in 0..event_count {
        let event = format!(
            "event: content_block_delta\n\
             data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"Token number {} in the stream \"}}}}\n\n",
            i
        );
        stream.extend_from_slice(event.as_bytes());
    }
    stream
}

fn generate_anthropic_realistic_stream(token_count: usize) -> Vec<u8> {
    let mut stream = Vec::new();

    // message_start
    stream.extend_from_slice(
        b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[],\"usage\":{\"input_tokens\":25,\"output_tokens\":1}}}\n\n"
    );

    // content_block_start
    stream.extend_from_slice(
        b"event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n"
    );

    // content_block_delta (the hot path)
    for i in 0..token_count {
        let delta = format!(
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"word{} \"}}}}\n\n",
            i
        );
        stream.extend_from_slice(delta.as_bytes());
    }

    // content_block_stop
    stream.extend_from_slice(
        b"event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n"
    );

    // message_stop
    stream.extend_from_slice(
        b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
    );

    stream
}

fn bench_sse_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sse_parser");

    // Throughput benchmarks at various stream sizes
    for event_count in [10, 100, 1_000, 10_000, 50_000] {
        let stream = generate_sse_stream(event_count);
        let stream_size = stream.len() as u64;

        group.throughput(Throughput::Bytes(stream_size));

        group.bench_with_input(
            BenchmarkId::new("zero_copy", event_count),
            &stream,
            |b, stream| {
                b.iter(|| {
                    let (events, consumed) = sse_parser::parse_all(black_box(stream));
                    assert_eq!(events.len(), event_count);
                    black_box((events, consumed));
                });
            },
        );

        let stream_str = String::from_utf8_lossy(&stream).to_string();
        group.bench_with_input(
            BenchmarkId::new("naive_allocating", event_count),
            &stream_str,
            |b, stream| {
                b.iter(|| {
                    let mut count = 0;
                    let mut remaining = stream.as_str();
                    while let Some((event_type, data)) =
                        sse_parser::parse_frame_naive(remaining)
                    {
                        count += 1;
                        if let Some(end) = remaining.find("\n\n") {
                            remaining = &remaining[end + 2..];
                        } else {
                            break;
                        }
                        black_box((&event_type, &data));
                    }
                    assert_eq!(count, event_count);
                });
            },
        );
    }

    group.finish();
}

fn bench_realistic_anthropic_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("sse_anthropic_realistic");

    for token_count in [100, 500, 2_000, 10_000] {
        let stream = generate_anthropic_realistic_stream(token_count);

        group.throughput(Throughput::Bytes(stream.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_full_response", token_count),
            &stream,
            |b, stream| {
                b.iter(|| {
                    let (events, _) = sse_parser::parse_all(black_box(stream));
                    black_box(&events);
                });
            },
        );
    }

    group.finish();
}

fn bench_single_frame(c: &mut Criterion) {
    let frame = b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello world \"}}\n\n";

    c.bench_function("sse_single_frame_parse", |b| {
        b.iter(|| {
            let result = sse_parser::parse_frame(black_box(frame));
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_sse_parsing,
    bench_realistic_anthropic_stream,
    bench_single_frame,
);
criterion_main!(benches);
