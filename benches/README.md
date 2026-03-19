# ARC CLI Benchmarks

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench sse_parser
cargo bench --bench checkpoint_io
cargo bench --bench hook_execution

# Run with HTML reports (opens in browser)
cargo bench -- --output-format html
open target/criterion/report/index.html
```

## Performance Targets

| Subsystem | Metric | Target | Competitor Reference |
|---|---|---|---|
| SSE Parser | Throughput | >2 GB/s | Claude Code: N/A (V8 overhead) |
| SSE Parser | Allocations | 0 per event | Claude Code: ~3 per event |
| Config Parse | Latency | <100 µs | Aider: ~50ms (Python) |
| Checkpoint Write | 200k tokens | <50 ms | Claude Code: ~200ms |
| Cold Boot | Interactive | <20 ms | Claude Code: ~800ms, Aider: ~2s |
| Hook Dispatch | Match 100 hooks | <100 µs | Claude Code: ~200ms per hook |
| Hook Execution | Single command | <50 ms | Claude Code: ~200ms |
| Snapshot Create | 50 files | <100 ms | Claude Code: N/A |
| Agent Spawn | 8 parallel | <5 ms overhead | N/A |

## CI Integration

Benchmarks run on every PR via `.github/workflows/bench.yml`.
Performance gate tests (`tests/regression/performance_gates.rs`) fail CI
if any metric regresses beyond acceptable thresholds.

## Generating Flamegraphs

```bash
cargo install flamegraph
cargo flamegraph --bench sse_parser -- --bench "zero_copy/10000"
```

---

## Summary Table

| Category | Files | Test Count | Purpose |
|---|---|---|---|
| **Criterion Benchmarks** | 10 | 40+ bench functions | Prove raw speed claims |
| **Integration Tests** | 8 | 50+ tests | Verify features work end-to-end |
| **Property Tests** | 4 | 20+ props | Prove correctness under random input |
| **Security Tests** | 3 | 30+ tests | Prove attack patterns are blocked |
| **Stress Tests** | 2 | 5 tests | Prove stability under load |
| **Regression Tests** | 1 | 5 gate tests | Prevent performance decay in CI |
| **Fuzz Targets** | 4 | Continuous | Find crashes in parsers |
| **CI Workflows** | 3 | 7 jobs | Automate everything |

**Total: 35 files, 150+ individual test/bench functions, 4 fuzz targets, 3 CI pipelines.**
