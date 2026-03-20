<div align="center">

# ARC CLI v1.0.0

**High-Performance Agentic CLI Framework in Rust**

[![Rust](https://img.shields.io/badge/Rust-1.85+-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-Async_Runtime-2B2B2B?style=for-the-badge)](https://tokio.rs/)
[![License](https://img.shields.io/badge/License-MIT-3DA639?style=for-the-badge)](LICENSE)
[![Platform](https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-0078D6?style=for-the-badge)](#downloads)

[![Anthropic Claude](https://img.shields.io/badge/Claude-D97757?style=flat-square&logo=anthropic&logoColor=white)](https://anthropic.com)
[![Google Gemini](https://img.shields.io/badge/Gemini-8E75B2?style=flat-square&logo=google&logoColor=white)](https://ai.google.dev)
[![OpenAI](https://img.shields.io/badge/OpenAI-412991?style=flat-square&logo=openai&logoColor=white)](https://openai.com)
[![Ollama](https://img.shields.io/badge/Ollama-000000?style=flat-square)](https://ollama.com)

[Documentation](https://github.com/Ashutosh0x/arc-cli/tree/main/docs) | [Features](https://github.com/Ashutosh0x/arc-cli#complete-feature-matrix-120-features) | [Security](SECURITY.md) | [Changelog](CHANGELOG.md)

</div>

---

## Quick Install

**Linux / macOS:**
```sh
curl -fsSL https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.ps1 | iex
```

**From source:**
```sh
cargo install --git https://github.com/Ashutosh0x/arc-cli
```

---

## Downloads

| Platform | Architecture | File | Size |
|----------|-------------|------|------|
| Windows | x86_64 | `arc-v1.0.0-x86_64-pc-windows-msvc.zip` | ~8 MB |
| macOS | Intel | `arc-v1.0.0-x86_64-apple-darwin.tar.gz` | ~7 MB |
| macOS | Apple Silicon (M1/M2/M3) | `arc-v1.0.0-aarch64-apple-darwin.tar.gz` | ~7 MB |
| Linux | x86_64 (static musl) | `arc-v1.0.0-x86_64-unknown-linux-musl.tar.gz` | ~8 MB |
| Linux | ARM64 (static musl) | `arc-v1.0.0-aarch64-unknown-linux-musl.tar.gz` | ~7 MB |

Verify integrity: `sha256sum -c SHA256SUMS.txt`

---

## Overview

ARC is a native agentic CLI framework built in pure Rust. It brings autonomous AI agents to your terminal with sub-20ms boot time, zero runtime dependencies, and multi-provider support across Claude, Gemini, OpenAI, and Ollama.

| | ARC CLI | Claude Code | Gemini CLI |
|---|---------|-------------|------------|
| **Language** | Rust (native binary) | Node.js | Node.js |
| **Cold Boot** | <20ms | ~500ms | ~400ms |
| **Runtime Dependencies** | None | Node 18+ | Node 18+ |
| **Binary Size** | ~15MB static | ~200MB+ | ~150MB+ |
| **Offline Support** | Ollama | No | No |
| **Multi-Provider** | 4 providers | Anthropic only | Google only |
| **Memory Safety** | `#![forbid(unsafe_code)]` | N/A | N/A |
| **License** | MIT | Proprietary EULA | Apache 2.0 |

---

## Architecture

31 modular crates, 180+ source files, strict dependency boundaries.

```
arc-cli          CLI entrypoint, REPL, command dispatch
arc-core         Config, credentials, security, memory, models
arc-agents       Orchestrator, sub-agents, registry, contracts
arc-plan         Planner, dependency mapper, plan renderer
arc-providers    Gemini, Claude, OpenAI, Ollama streaming clients
arc-tools        File I/O, search, shell exec, MCP integration
arc-session      Redb-backed checkpointing, forking, history
arc-ui           Terminal renderer, layout engine, theming
arc-remote       WebSocket remote control, JWT auth
arc-cloud        Cloud task delegation
arc-shadow       Shadow workspace isolation
arc-worktree     Git worktree management
arc-hooks        Event-driven hook system
arc-policy       Permission engine, approval modes
arc-a2a          Agent-to-Agent protocol over HTTP/2
arc-bench        Criterion benchmarks
arc-diff         Unified diff generation
arc-compact      Context compaction engine
arc-vision       Screenshot and vision capabilities
arc-io           Streaming I/O utilities
arc-router       Model routing and fallback chains
```

---

## What's Included

### Core Capabilities
- Multi-agent orchestration with A2A protocol (Planner, Architect, Coder)
- Session checkpointing and time-travel rewind via redb
- Zero-allocation SIMD streaming parser (memchr SSE byte-slicing)
- Sandboxed execution: Landlock syscall filters (Linux), shadow workspaces
- Hook system with 6 event types (PreToolUse, PostToolUse, Stop, etc.)
- 3-tier permission engine: allow/ask/deny per tool with compound bash parsing

### Developer Experience
- Plan mode with accept/reject/feedback workflow
- Session forking and branching
- Slash commands with YAML frontmatter and auto-discovery
- Auto-memory persistence with /memory store
- 20+ IDE auto-detection (VS Code, Cursor, Zed, JetBrains, Xcode)
- Syntax-highlighted diff review with collapse/expand

### Advanced Features
- Plugin marketplace with git/local install and trust levels
- Voice mode: STT + push-to-talk, 20 languages, CoreAudio/WASAPI/PulseAudio
- Agent teams: leader/teammate/background roles with inter-agent messaging
- 6 PR review specialized agents
- Security vulnerability scanner (9 patterns)
- Ralph autonomous loop with max-iteration control

### Platform Hardening
- Windows: CRLF-safe I/O, PowerShell Set-Clipboard for CJK/Unicode
- macOS: sandbox-exec integration, universal binary support
- Linux: Landlock syscall filters, musl static linking
- Cross-platform: tmux/screen clipboard compat, SSH detection, WSL awareness

---

## Getting Started

```sh
# Start an agentic session
arc chat

# Initialize ARC rules in a repo
arc init

# Run diagnostics
arc doctor

# See all commands
arc --help
```

### Key Commands

| Command | Description |
|---------|-------------|
| `/plan [task]` | Generate and review a modification blueprint |
| `/checkpoint` | Snapshot current session state |
| `/rewind [id]` | Time-travel to a previous checkpoint |
| `/compact` | Manually compress context window |
| `/memory [k] [v]` | Save persistent context |
| `/fork [name]` | Branch the conversation |
| `/security-review` | Scan for vulnerabilities |

---

## Performance

| Subsystem | Performance |
|-----------|------------|
| Config parsing | ~56 us |
| SSE streaming | 0 allocations |
| Cold boot | <20 ms |
| Checkpointing | ~45 ms (200k tokens) |

---

## Security

- Instruction hierarchy with XML delimiters
- MCP manifest hash verification
- Landlock syscall filters (Linux)
- Shadow workspace isolation
- Environment secret sanitization (15+ patterns)
- Conseca dynamic security policies
- `#![forbid(unsafe_code)]` across entire workspace

Report vulnerabilities via [SECURITY.md](SECURITY.md).

---

## Documentation

- [Full Feature Matrix](https://github.com/Ashutosh0x/arc-cli#complete-feature-matrix-120-features)
- [Architecture Guide](docs/features.md)
- [Security Model](SECURITY.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)

---

## Tech Stack

[![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-2B2B2B?style=flat-square)](https://tokio.rs/)
[![tree-sitter](https://img.shields.io/badge/tree--sitter-6DB33F?style=flat-square)](https://tree-sitter.github.io/)
[![redb](https://img.shields.io/badge/redb-4B0082?style=flat-square)](https://github.com/cberner/redb)
[![memchr](https://img.shields.io/badge/memchr-SIMD-FF6600?style=flat-square)](https://github.com/BurntSushi/memchr)
[![syntect](https://img.shields.io/badge/syntect-Highlighting-2196F3?style=flat-square)](https://github.com/trishume/syntect)
[![axum](https://img.shields.io/badge/axum-HTTP-009688?style=flat-square)](https://github.com/tokio-rs/axum)
[![serde](https://img.shields.io/badge/serde-Serialization-E91E63?style=flat-square)](https://serde.rs/)

---

<div align="center">

**Built with Rust. Zero unsafe code. Production ready.**

[Star this repository](https://github.com/Ashutosh0x/arc-cli) if ARC helps you ship faster.

</div>
