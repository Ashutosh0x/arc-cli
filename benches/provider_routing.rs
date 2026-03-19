//! Provider routing benchmark — model selection and routing decision speed.

use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct ProviderCapabilities {
    streaming: bool,
    function_calling: bool,
    vision: bool,
    max_context: u64,
    cost_per_1k_input: f64,
    cost_per_1k_output: f64,
    median_latency_ms: u64,
}

#[derive(Clone, Debug)]
struct ModelEntry {
    provider: String,
    model_id: String,
    capabilities: ProviderCapabilities,
}

fn build_model_registry() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            provider: "anthropic".into(),
            model_id: "claude-sonnet-4-20250514".into(),
            capabilities: ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_context: 200_000,
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                median_latency_ms: 800,
            },
        },
        ModelEntry {
            provider: "google".into(),
            model_id: "gemini-2.5-pro".into(),
            capabilities: ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_context: 1_000_000,
                cost_per_1k_input: 0.00125,
                cost_per_1k_output: 0.005,
                median_latency_ms: 600,
            },
        },
        ModelEntry {
            provider: "openai".into(),
            model_id: "gpt-4o".into(),
            capabilities: ProviderCapabilities {
                streaming: true,
                function_calling: true,
                vision: true,
                max_context: 128_000,
                cost_per_1k_input: 0.005,
                cost_per_1k_output: 0.015,
                median_latency_ms: 700,
            },
        },
        ModelEntry {
            provider: "ollama".into(),
            model_id: "llama3.1:70b".into(),
            capabilities: ProviderCapabilities {
                streaming: true,
                function_calling: false,
                vision: false,
                max_context: 131_072,
                cost_per_1k_input: 0.0,
                cost_per_1k_output: 0.0,
                median_latency_ms: 2000,
            },
        },
    ]
}

fn select_best_model(
    registry: &[ModelEntry],
    required_context: u64,
    needs_vision: bool,
    needs_functions: bool,
    prefer_cost: bool,
) -> Option<&ModelEntry> {
    let mut candidates: Vec<_> = registry
        .iter()
        .filter(|m| {
            m.capabilities.max_context >= required_context
                && (!needs_vision || m.capabilities.vision)
                && (!needs_functions || m.capabilities.function_calling)
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    if prefer_cost {
        candidates.sort_by(|a, b| {
            a.capabilities
                .cost_per_1k_input
                .partial_cmp(&b.capabilities.cost_per_1k_input)
                .unwrap()
        });
    } else {
        candidates.sort_by_key(|m| m.capabilities.median_latency_ms);
    }

    candidates.first().copied()
}

fn bench_provider_routing(c: &mut Criterion) {
    let registry = build_model_registry();

    let mut group = c.benchmark_group("provider_routing");

    group.bench_function("select_best_model_cost", |b| {
        b.iter(|| {
            let model = select_best_model(
                criterion::black_box(&registry),
                50_000,
                false,
                true,
                true,
            );
            criterion::black_box(model);
        });
    });

    group.bench_function("select_best_model_latency", |b| {
        b.iter(|| {
            let model = select_best_model(
                criterion::black_box(&registry),
                50_000,
                false,
                true,
                false,
            );
            criterion::black_box(model);
        });
    });

    group.bench_function("select_with_vision_large_context", |b| {
        b.iter(|| {
            let model = select_best_model(
                criterion::black_box(&registry),
                500_000,
                true,
                true,
                true,
            );
            criterion::black_box(model);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_provider_routing);
criterion_main!(benches);
