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
ARC CLI is an autonomous agentic CLI framework written in Rust. It reasons over local codebases to plan, write, and verify software using a multi-agent delegation model. 
<img width="1444" height="1127" alt="image" src="https://github.com/user-attachments/assets/14510ed0-e668-42fc-a637-48406bfd36e1" />

## Core Infrastructure

### 1. Multi-Agent Orchestration
ARC CLI uses an Orchestrator model to dispatch specialized subagents.
- Runs agents in parallel using the `tokio` async runtime.
- Agents communicate context using a strictly typed, zero-trust Agent-to-Agent (A2A) protocol over HTTP/2.

### 2. State Checkpointing & Rewind
ARC CLI maintains local session state via a `redb`-backed key-value store.
- Stores conversation history and agent reasoning locally.
- Allows you to rewind to a previous session checkpoint, safely reverting affected files on disk.

### 3. Low-Latency Streaming
ARC CLI uses a custom Server-Sent Events (SSE) parsing layer powered by SIMD `memchr`. Paired with globally pooled HTTP/2 connections, this eliminates dynamic string allocations during token streams.

### 4. Sandboxed Execution Safety
ARC CLI incorporates security layers to limit autonomous actions:
- **Instruction Hierarchy**: Uses strict XML delimiters to separate user context from system instructions.
- **Manifest Pinning**: Model Context Protocol (MCP) clients verify hash digests to block unauthorized tool injection.
- **Shadow Workspaces**: Autonomous changes run against isolated `.arc-shadow` directories using OS-level hardlinks or Landlock syscall filters on Linux to prevent unauthorized writes.

### 5. Algorithmic Context Management (New)
ARC CLI implements advanced context preservation algorithms inspired by the latest Gemini research:
- **Verified Summarization**: A dual-phase compression loop that validates all technical facts and file paths before committing to long-term memory.
- **Reverse Token Budgeting**: Automatically offloads older massive tool outputs to temporary disk stores to prioritize fresh prompt context, preventing the "forgetting" of recent user instructions.
- **Graceful State Recovery**: Injects a specialized "final turn" prompt when agents hit execution limits, allowing them to cleanly summarize and checkpoint status instead of hard-failing.
- **Native AST Safety**: Uses tree-sitter for Bash and native PowerShell AST walkers for Windows to audit shell commands at the syntax level, blocking destructive operations before they hit the kernel.

### 6. Runtime Intelligence (New — Phase 28)
Production-grade systems ported from deep analysis of Gemini CLI's 500+ file codebase:
- **3-Layer Loop Detection**: SHA-256 tool call dedup → sliding-window content chanting → LLM double-check with adaptive intervals. Prevents runaway sessions without blunt timeouts.
- **Tool Output Masking**: Hybrid Backward-Scanned FIFO with 50k token protection window and 30k batch thresholds. Offloads large outputs to `.arc/tool-outputs/` with head+tail previews.
- **Environment Sanitization**: Regex-based blocking of 15+ secret patterns (JWT, AWS `AKIA*`, GitHub `ghp_*`, RSA keys, Stripe, Slack tokens) with strict CI mode.
- **Conseca Dynamic Safety**: LLM-generated SecurityPolicies per user prompt — adapts tool permissions, arg constraints, and path restrictions per request.
- **Model Availability & Fallback**: Terminal vs sticky-retry health tracking with policy-driven fallback chains (retry_always, retry_once, stop, upgrade intents).
- **JIT Context Loading**: Dynamically discovers and injects `ARC.md` files as the agent navigates subdirectories via high-intent tools.
- **IDE Detection**: Auto-detects 20+ IDEs (VS Code, Cursor, Zed, JetBrains suite, Xcode, Neovim, Emacs) for environment-aware suggestions.
- **Session Summaries**: Auto-generates ≤80 char session titles via fast-model sliding-window analysis.
- **Extensions CLI**: Full plugin lifecycle (install, uninstall, link, update, configure, enable, disable, validate, new).
- **Billing & Quota**: Cost estimation per model, credit balance tracking, overage strategies.
- **Prompt Registry**: Versioned prompt templates with variable substitution and 6 built-in prompts.
- **Advanced Telemetry**: ActivityDetector, MemoryMonitor, StartupProfiler, HighWaterMarkTracker.

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
- [Autonomous Agents & Modality (Voice, Skills, Loop)](docs/autonomous_agents.md)
- [Enterprise Code Intelligence (Repomap, Hooks, Caching)](docs/enterprise_features.md)
- [Advanced Tooling & Ergonomics (Syntect, PR Review, Tree-sitter)](docs/advanced_tooling.md)

## Architecture

| Crate / Module | Responsibility | Key Features |
| :--- | :--- | :--- |
| `arc-cli` | **Client Interface** | Frontend REPL, token routing, extensions CLI, and `rustyline` plugins. |
| `arc-core` | **Foundation Layer** | Credentials, config, loop detection, tool masking, JIT context, IDE detect, billing, prompt registry. |
| `arc-providers` | **Model Gateways** | Native Gemini, Claude, OpenAI clients + model availability + fallback handler. |
| `arc-agents` | **Swarm Delegation** | Orchestrator dispatcher mapping specialized sub-agent routines. |
| `arc-a2a` | **Agent Protocol** | HTTP/2 SSE communications secured by HMAC/JWT signatures. |
| `arc-session` | **Persistent Memory** | `redb` K-V storage, checkpoint undo/rewind, auto session summaries. |
| `arc-policy` | **Safety Engine** | Conseca dynamic policies, static rule engine, LLM-generated SecurityPolicy. |
| `arc-plan` | **Planning Mode** | Read-only analysis, persistent task tracker with DAG validation. |
| `arc-diff` | **Structural Diffing** | Semantic diffs, context snippet generator, patch engine. |
| `arc-repomap` | **Code Intelligence** | `tree-sitter` AST extraction powering massive 10x context compressions. |
| **Subsystems** | **Capability Expansions** | `arc-voice`, `arc-vision`, `arc-sandbox`, `arc-skills` native scale-outs. |

## Benchmarking Facts

Our end-to-end framework benchmarks have validated the underlying agent engine's raw speed:

| Subsystem | Speed Metric | Architectural Advantage |
| :--- | :--- | :--- |
| **Config Parsing** | **~56 µs** | Zero-copy matrices loaded across hierarchical TOML/JSON boundaries. |
| **Streaming Parser** | **0 Allocations** | SIMD `memchr`-powered SSE byte-slicing avoids runtime heap fragmentation. |
| **Cold Boot** | **< 20 ms** | `OnceLock` and tuned LTO profiles eliminate Python/Node runtime overheads. |
| **Checkpointing** | **~45 ms** | Commits 200k+ token sessions directly to `redb` block-aligned disk stores. |

## Supported Capabilities

| Capability Domain | Native Implementation | Specification Reference |
| :--- | :--- | :--- |
| **Telemetry** | Full OpenTelemetry (OTLP) metrics for LLM inference streams | Global Observability Standards |
| **Cybersecurity** | Instruction hierarchies & Agent network kernel-level firewalls | OWASP Top 10 for LLM Apps |
| **Compliance** | Explicit logging mapped to enterprise requirements natively | NIST AI RMF 1.0 |

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
- `arc init` to bootstrap ARC rules recursively across un-initialized repos.
- `arc --stats` for live LLM cost accounting and budget tracking.
- `arc review` to generate pre-push architectural critiques of your working branches via LLM swarms.

## Keyboard Ergonomics & Flow

ARC CLI is rigorously built for keyboard-only efficiency:

### REPL Shortcuts
| Keybind | Action | Capability |
| :--- | :--- | :--- |
| `Ctrl+C` | **Graceful Agent Halt** | Intercepts HTTP/2 LLM streams globally, preserving existing context natively. |
| `Ctrl+D` | **Save & Terminate** | Triggers EOF to gracefully snap context to the `redb` state store and yield the CLI. |
| `Tab` | **Auto-Complete** | Completes subcommands, slash-routines, and `@src/` dynamic fuzzy-file finding hooks. |

### Syntax-Highlighted Diff Review
When agents autonomously modify codebase files, bypass the naive `Y/n` prompt with our granular `read_key` terminal loop:

| Keybind | Action | Effect |
| :--- | :--- | :--- |
| `Enter` / `y` | **Accept** | Stages the current grouped diff patch for execution. |
| `Esc` / `n` | **Reject** | Drops the current chronologically grouped diff patch. |
| `a` | **Accept All** | Instantly accepts the current file and *ALL remaining queued files*. |
| `d` | **Deny All** | Scraps the entire outstanding generation sequence immediately. |
| `e` | **Open in Editor** | Pops the unified diff directly into `$EDITOR` for manual semantic correction. |
| `j` / `k` | **Vim Scroll** | Traverses extremely large codebase patches natively. |

---
<div align="center">

Built with ❤️ for autonomous agents

</div>
