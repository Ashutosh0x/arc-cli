// SPDX-License-Identifier: MIT
//! Multi-agent throughput benchmark.
//! Measures overhead of dispatching N parallel agent tasks.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn simulate_agent_work(agent_id: usize, work_ms: u64) -> (usize, String) {
    // Simulate LLM latency
    tokio::time::sleep(tokio::time::Duration::from_millis(work_ms)).await;
    (
        agent_id,
        format!("Agent {} completed in {}ms", agent_id, work_ms),
    )
}

fn bench_parallel_agent_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_agents");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for agent_count in [2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("dispatch_and_collect", agent_count),
            &agent_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let semaphore = Arc::new(Semaphore::new(count));
                        let mut handles = Vec::with_capacity(count);

                        for i in 0..count {
                            let sem = semaphore.clone();
                            let handle = tokio::spawn(async move {
                                let _permit = sem.acquire().await.unwrap();
                                simulate_agent_work(i, 1).await
                            });
                            handles.push(handle);
                        }

                        let mut results = Vec::with_capacity(count);
                        for handle in handles {
                            results.push(handle.await.unwrap());
                        }
                        criterion::black_box(results.len());
                    });
                });
            },
        );
    }

    // Measure pure dispatch overhead (no simulated work)
    for agent_count in [2, 4, 8, 16, 32] {
        group.bench_with_input(
            BenchmarkId::new("spawn_overhead_only", agent_count),
            &agent_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(count);
                        for i in 0..count {
                            handles.push(tokio::spawn(async move {
                                criterion::black_box(i * i);
                                i
                            }));
                        }
                        for handle in handles {
                            let _ = handle.await;
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parallel_agent_dispatch);
criterion_main!(benches);
