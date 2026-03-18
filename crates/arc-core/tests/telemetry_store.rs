//! Integration tests for the telemetry store.

use arc_core::telemetry::store::TelemetryStore;
use arc_core::telemetry::types::RequestRecord;
use tempfile::tempdir;

fn make_record(provider: &str, input: u32, output: u32, latency: u64, cost: f64) -> RequestRecord {
    RequestRecord {
        timestamp_ms: 1_700_000_000_000 + latency, // slightly different timestamps
        provider: provider.to_owned(),
        model: format!("{provider}-latest"),
        input_tokens: input,
        output_tokens: output,
        latency_ms: latency,
        cost_usd: cost,
        success: true,
        error: None,
    }
}

#[test]
fn empty_store_returns_zero_summary() {
    let dir = tempdir().unwrap();
    let store = TelemetryStore::open(dir.path()).unwrap();
    let summary = store.aggregate_all().unwrap();
    assert_eq!(summary.total_requests, 0);
    assert_eq!(summary.total_cost_usd, 0.0);
    assert!(summary.providers.is_empty());
}

#[test]
fn records_persist_across_reopen() {
    let dir = tempdir().unwrap();

    {
        let store = TelemetryStore::open(dir.path()).unwrap();
        store
            .record(&make_record("anthropic", 100, 50, 200, 0.01))
            .unwrap();
        store
            .record(&make_record("openai", 200, 100, 300, 0.02))
            .unwrap();
    }

    // Reopen the database.
    let store = TelemetryStore::open(dir.path()).unwrap();
    assert_eq!(store.record_count().unwrap(), 2);

    let summary = store.aggregate_all().unwrap();
    assert_eq!(summary.total_requests, 2);
    assert_eq!(summary.total_input_tokens, 300);
    assert_eq!(summary.total_output_tokens, 150);
    assert!((summary.total_cost_usd - 0.03).abs() < 1e-10);
}

#[test]
fn per_provider_breakdown() {
    let dir = tempdir().unwrap();
    let store = TelemetryStore::open(dir.path()).unwrap();

    for _ in 0..5 {
        store
            .record(&make_record("anthropic", 100, 50, 200, 0.005))
            .unwrap();
    }
    for _ in 0..3 {
        store
            .record(&make_record("openai", 150, 75, 350, 0.008))
            .unwrap();
    }

    let summary = store.aggregate_all().unwrap();
    assert_eq!(summary.providers.len(), 2);
    assert_eq!(summary.providers["anthropic"].total_requests, 5);
    assert_eq!(summary.providers["openai"].total_requests, 3);
    assert_eq!(summary.providers["openai"].total_input_tokens, 450);
}

#[test]
fn latency_percentiles_correctness() {
    let dir = tempdir().unwrap();
    let store = TelemetryStore::open(dir.path()).unwrap();

    // Insert records with known latencies: 100, 200, ..., 1000.
    for i in 1..=10 {
        let mut rec = make_record("test", 50, 25, i * 100, 0.001);
        rec.latency_ms = i * 100;
        store.record(&rec).unwrap();
    }

    let summary = store.aggregate_all().unwrap();
    let stats = &summary.providers["test"];

    assert_eq!(stats.p50(), Some(500));
    assert!(stats.p95().unwrap() >= 900);
    assert_eq!(stats.p99(), Some(1000));
}
