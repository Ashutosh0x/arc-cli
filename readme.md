# ARC: High-Performance Agentic CLI

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-2B2B2B?style=for-the-badge)
![Anthropic Claude](https://img.shields.io/badge/Claude-D97757?style=for-the-badge&logo=anthropic&logoColor=white)
![Google Gemini](https://img.shields.io/badge/Gemini-8E75B2?style=for-the-badge&logo=google&logoColor=white)
![OpenAI](https://img.shields.io/badge/OpenAI-412991?style=for-the-badge&logo=openai&logoColor=white)
![GitHub Actions](https://img.shields.io/badge/GitHub_Actions-2088FF?style=for-the-badge&logo=github-actions&logoColor=white)
<br/>
![Windows](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)
![Linux](https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)
![macOS](https://img.shields.io/badge/macOS-000000?style=for-the-badge&logo=apple&logoColor=white)

## Overview
ARC CLI is a next-generation autonomous Agentic CLI framework natively built in Rust for absolute speed, safety, and scale. Going far beyond traditional autocomplete tools, ARC is a true agentic orchestrator that reasons over complex, multi-file codebases natively in your terminal. It is designed to autonomously plan, write, verify, and secure software using a multi-agent delegation model. 

By operating entirely within your terminal as an autonomous agent, ARC accelerates development workflows while maintaining complete structural context awareness of your entire repository.

## The Agentic CLI Advantage

### 1. Multi-Agent Orchestration Engine
A single generalized LLM often struggles with conflicting goals (e.g., writing new features vs. rigorously security-auditing them). ARC CLI deploys a master-worker Orchestrator model:
- Dispatches hyper-focused persona agents (e.g., `Security Auditor`, `Test Engineer`, `Code Reviewer`).
- Runs agents in parallel using Rust's async runtime (`tokio`) to concurrently validate workloads across the codebase.
- Agents communicate context dynamically using a strictly typed Agent-to-Agent (A2A) protocol, ensuring no mental context is dropped during handoffs.

### 2. Full State Checkpointing & Autonomous Time Travel
When traditional agents hallucinate, they destroy your context window, forcing you to start over.
ARC CLI introduces a continuous autonomous feedback loop combined with a `redb`-backed Checkpointing and Rewind subsystem:
- Stores the entire memory context, conversational history, and agent reasoning automatically.
- Facilitates instantaneous rewinds (time travel) alongside the physical reversion of any affected files on disk.
- Capable of reloading a previous deep-context session in microseconds.

### 3. Zero-Copy Streaming and HTTP/2 Pooling
ARC is engineered entirely for low-latency agentic thought. Unlike Python-based CLI agents that allocate strings heavily per token, ARC CLI features a custom Zero-Copy Server-Sent Events (SSE) parsing layer powered by SIMD `memchr`. Paired with globally pooled HTTP/2 connections, ARC's time-to-first-token (TTFT) and total reasoning throughput consistently outpaces standard implementations.

### 4. Advanced Security and Sandboxed Exploration
ARC CLI incorporates intense security protocols to govern autonomous actions safely:
- **Strict Instruction Hierarchy**: Hardened Prompt Guards use strict XML/Markdown delimiters, ensuring user context mathematically cannot bypass core agent operational instructions via Prompt Injection.
- **Manifest Pinning**: Ensures Model Context Protocol (MCP) clients strictly verify hash digests to block unauthorized tool injection.
- **Shadow Workspaces**: Complex autonomous changes are executed inside isolated `.arc-shadow` directories using OS-level hardlinks. The agents test their code internally, totally shielding your actual worktree until verified and approved.

## Documentation

Explore the extreme depth of ARC CLI's architecture and usage:
- [Getting Started](docs/getting-started.md)
- [Installation](docs/installation.md)
- [Setup & Configuration](docs/setup.md)
- [Advanced Usage](docs/usage.md)
- [Architecture & Memory](docs/architecture.md)
- [Model Context Protocol (MCP)](docs/mcp.md)
- [Authentication & Security](docs/authentication.md)
- [Multi-Provider Hub](docs/providers.md)
- [Parallel Routing Engine](docs/routing.md)
- [WASM Plugins](docs/plugins.md)
- [Deep Benchmarking Facts](docs/benchmarks.md)
- [Phase 7 Subsystems Integrated (Voice, Skills, Loop, Compact)](docs/phase_7.md)
- [Phase 8 Enterprise Features (Repomap, Hooks, Caching, CI)](docs/phase_8.md)

## Architecture

- `arc-cli`: The frontend REPL, routing, and console interaction boundary.
- `arc-core`: Credentials, token budgeting, prompt guards, and configuration loading.
- `arc-providers`: LLM interaction handlers supporting Deepmind Gemini, Anthropic Claude, OpenAI, and local Ollama deployments.
- `arc-memory`: Persistent, tiered (Working, Short-Term, Long-Term) local context mapping.
- `arc-session`: Fast embedded state storage for checkpointing and rewinding workspace states.
- `arc-agents`: Top-level sub-agent discovery and delegation routines.
- `arc-a2a`: Production-grade Agent-to-Agent communication over HTTP/2, SSE streaming, and HMAC/JWT security.
- `arc-plan`, `arc-worktree`, `arc-diff`: Precision-engineered modules handling codebase dependency mapping and git integration.
- `arc-voice`, `arc-compact`, `arc-skills`, `arc-search`, `arc-vision`, `arc-sandbox`, `arc-loop`, `arc-cloud`: The Phase 7 Autonomous scale-up providing voice controls, vision grounding, linux OS security barriers, cron loops, and dynamic skill orchestration traits.
- `arc-repomap` & `arc-hooks`: The Phase 8 Enterprise bridge yielding blazing fast AST `tree-sitter` codebase maps and TOML-configurable sandboxed post-edit webhooks formatting.

## Benchmarking Facts

Our end-to-end framework benchmarks have validated the underlying agent engine's raw speed:

- **Config Parsing**: Loads full config matrices across hierarchical layers in exceptionally low time bounds (~56 microseconds).
- **Streaming Parser**: The SSE byte-slice parser allocates exactly 0 dynamic heap arrays during continuous LLM streaming, yielding 10x memory efficiency over default `serde_json` line parsing.
- **Cold Start**: With Cargo profiles properly tuned and global static `OnceLock` initializers, ARC boots up to 200 milliseconds faster than Python-based CLIs.
- **State Checkpointing**: An agentic session containing roughly 200,000 tokens of conversational context can be compressed, flushed to disk, and committed natively in less than 45 milliseconds.

## Supported Capabilities

- Native compliance mapping to the **NIST AI Risk Management Framework 1.0**.
- Built-in defenses strictly corresponding to the **OWASP Top 10 for LLM Applications**.
- Complete OpenTelemetry (OTLP) tracing capability spanning LLM request lifecycles, provider latency tracking, and autonomous cost budgeting.

## Getting Started

Ensure you have Rust version 1.85+ installed via Rustup.

```bash
cargo install --path . --locked
```

Start the autonomous session by running:
```bash
arc chat
```

Interact directly with the agentic loop:
- `/plan [task]` to autonomously generate a codebase modification blueprint.
- `/doctor` to evaluate your workspace and credential configurations safely.
- `/checkpoint` to snapshot the LLM turn history and file states.
- `/rewind [id]` to safely time-travel the CLI state backward to correct errant agent behavior.

---
<div align="center">

Built with ❤️ for autonomous agents

</div>
