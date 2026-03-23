// SPDX-License-Identifier: MIT
use criterion::{criterion_group, criterion_main, Criterion};

pub fn dummy_bench(c: &mut Criterion) {
    c.bench_function("dummy", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, dummy_bench);
criterion_main!(benches);
