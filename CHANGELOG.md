# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-03-22

### Added
- **Code Intelligence Graph** — native integration with [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) for structural code analysis across 64 languages.
- **`arc graph` subcommands** — `index`, `search`, `trace`, `architecture`, `impact`, `query` — persistent knowledge graph queries from the terminal.
- **MCP JSON-RPC 2.0 stdio client** (`arc-mcp/client.rs`) — spawns and communicates with external MCP servers as sidecar processes.
- **Typed MCP tool wrappers** (`arc-mcp/tools.rs`) — `index_repository`, `search_graph`, `trace_call_path`, `detect_changes`, `get_architecture`, `query_graph`, and 4 more.
- **MCP server configuration** — `[[mcp.servers]]` section in `~/.arc/config.toml` with auto-start, enable/disable per server.
- **Release-plz configuration** — `release-plz.toml` to disable premature crates.io publishing.

### Fixed
- Clippy deny errors: regex backreference in `context_sanitizer.rs`, `suspicious_open_options` in `instance_lock.rs`.
- Arena test (`bumpalo::Bump`) rewritten for functional verification — no longer depends on `allocated_bytes()` counter behavior.
- MSRV bumped to 1.89 for `incompatible_msrv` lint compliance.
- `cargo fmt` applied workspace-wide (170+ files).
- Config roundtrip tests fixed for nested `[general]` section.
- Missing `parse_sse_chunk` function implemented in `arc-providers`.
- Benchmarks CI fixed (`libasound2-dev` dependency).

## [0.5.0] - 2026-03-18

### Added
- Initial Open-Source release of the ARC Framework structure.
- Implementation of the `arc-cli` REPL loop and terminal routing.
- Advanced Agentic sub-components (`arc-agents`, `arc-mcp`, `arc-plan`).
- `docs/` highly detailed architectural writeups for providers, routing, and benchmarks.
- Automated GitHub Actions deployment workflows.

### Planned (Upcoming)
- True Zero-Copy SSE Parsing via `memchr`.
- Deep Native integration with Claude 3.5 Sonnet Tool Use APIs.
- Redb physical disk checkpointing for session rewind operations.
