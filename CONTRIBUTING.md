# Contributing to ARC CLI

Welcome! ARC is a high-performance Agentic CLI built natively in Rust. We appreciate your help in building the most structurally sound multi-agent orchestrator.

## Principles

1. **Zero-Cost Abstractions**: If your PR introduces string allocations (`String::from`, `.clone()`) inside a streaming hot path, it will be rejected. Use `&[u8]`, `Cow`, or `Bytes`.
2. **Fearless Security**: Any code touching `arc-shadow` or `arc-mcp` MUST include rigorous integration tests verifying that symlinks and path-traverals cannot escape the `.arc-shadow/` boundary.
3. **No Unwraps**: The workspace strictly forbids `.unwrap()` or `.expect()`. Propagate errors using `?` entirely up to the `run_repl()` entrypoint via `arc-core::error::ArcError`.

## PR Process

1. Fork the repo and create a `feature/your-feature` branch.
2. Run the formatter: `cargo fmt --all`
3. Run the linter: `cargo clippy --workspace -- -D warnings`
4. Run the test suite: `cargo test --workspace`
5. Submit your PR detailing the exact micro-second latency impact if affecting `arc-providers` or `arc-router`.
