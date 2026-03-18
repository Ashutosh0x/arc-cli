use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_config_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("Startup");
    
    group.bench_function("Config::load", |b| {
        b.iter(|| {
            // we use the synchronous load to simulate CLI start block
            black_box(arc_core::config::ArcConfig::load().unwrap_or_default())
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_config_load);
criterion_main!(benches);
