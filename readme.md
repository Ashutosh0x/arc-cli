# ARC

<img width="2048" height="861" alt="arc cli" src="https://github.com/user-attachments/assets/4952031a-b3fa-4074-8573-18fe7a9832d8" />

[![Release](https://img.shields.io/github/v/release/Ashutosh0x/arc-cli?style=for-the-badge&logo=github)](https://github.com/Ashutosh0x/arc-cli/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Ashutosh0x/arc-cli/total?style=for-the-badge&logo=github)](https://github.com/Ashutosh0x/arc-cli/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/Ashutosh0x/arc-cli/test.yml?branch=main&style=for-the-badge&logo=github-actions&label=CI)](https://github.com/Ashutosh0x/arc-cli/actions)
[![License](https://img.shields.io/badge/License-MIT-3DA639?style=for-the-badge)](LICENSE)
![Rust](https://img.shields.io/badge/Rust_1.89+-DEA584?style=for-the-badge&logo=rust&logoColor=white)

A native agentic CLI that reasons over codebases. One binary. No runtime. Works offline.

---

## Install

```sh
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.sh | sh

# Windows
irm https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.ps1 | iex

# From source
cargo install --git https://github.com/Ashutosh0x/arc-cli
```

## What it does

ARC is a terminal-native AI coding agent. You point it at a codebase, describe what you want, and it plans, writes, and verifies the changes autonomously.

It connects to Claude, Gemini, OpenAI, or Ollama (fully offline). You pick the provider. If one goes down, it fails over to the next.

```sh
arc chat                    # start a session
/plan "add OAuth2 login"    # generate a plan, review it, accept or reject
/checkpoint                 # snapshot current state
/rewind 3                   # undo to checkpoint 3
/fork experiment            # branch the conversation
arc review                  # PR critique via 6 specialized agents
```

## Why not Claude Code or Gemini CLI?

| | ARC | Claude Code | Gemini CLI |
|---|-----|-------------|------------|
| Written in | Rust | Node.js | Node.js |
| Cold start | <20ms | ~500ms | ~400ms |
| Binary size | ~15MB | ~200MB+ | ~150MB+ |
| Runtime deps | None | Node 18+ | Node 18+ |
| Offline mode | Ollama | No | No |
| Providers | 4 (Claude, Gemini, OpenAI, Ollama) | Anthropic only | Google only |
| Unsafe code | `#![forbid(unsafe_code)]` | N/A | N/A |

ARC is a single static binary with no interpreter, no package manager, and no `node_modules`. It boots in under 20 milliseconds.

## How it works

**31 crates.** Each crate owns one concern:

```
arc-cli          REPL, command dispatch, extensions
arc-core         Config, credentials, security, memory
arc-providers    Claude, Gemini, OpenAI, Ollama clients
arc-agents       Orchestrator, subagent dispatch, A2A protocol
arc-session      redb checkpointing, fork, rewind
arc-plan         Plan mode, dependency mapping, DAG tracking
arc-tools        File ops, shell exec, MCP client, search
arc-policy       Permission engine, sandbox policies
arc-hooks        Event-driven pre/post tool hooks
arc-diff         Structural diffs, patch engine
arc-repomap      tree-sitter AST extraction, context compression
arc-sandbox      Landlock (Linux), shadow workspace isolation
arc-compact      Context compaction, token budgeting
arc-ui           Terminal renderer, statusline, theming
arc-voice        Push-to-talk, 20 languages, multi-backend
arc-plugins      Marketplace, trust levels, manifest validation
arc-remote       WebSocket remote control, JWT auth
arc-a2a          Agent-to-agent HTTP/2 protocol
```

**Streaming.** Custom SSE parser built on SIMD `memchr`. Zero heap allocations during token streams. HTTP/2 connection pooling across providers.

**Safety.** Every shell command goes through tree-sitter (Bash) or PowerShell AST analysis before execution. Environment variables are scanned for 15+ secret patterns. Landlock syscall filters sandbox file access on Linux.

**Memory.** 5-layer context system (Arena, Working, ShortTerm, LongTerm, Compressor) with automatic compaction when approaching token limits. Sessions persist to `redb` with atomic snapshots.

## Key features

**Agentic core** — Plan mode with accept/reject/feedback. Session checkpointing and time-travel rewind. Conversation forking. Multi-agent orchestration with specialized subagents. Autonomous iteration loop with completion detection.

**Security** — 3-tier permission engine (allow/ask/deny per tool). Landlock syscall sandboxing. Shadow workspace isolation. Secret pattern blocking. MCP manifest hash pinning. Dynamic per-request security policies.

**Developer tools** — Slash commands with YAML frontmatter. Persistent memory store. 20+ IDE auto-detection. Syntax-highlighted diff review with vim keybindings. Plugin marketplace. Voice mode. Background task management. PR review with 6 specialized agents.

**Platform support** — Windows (MSVC), macOS (Intel + Apple Silicon), Linux (x86_64 + ARM64, static musl). CRLF handling, WSL detection, XDG-compliant config paths.

## Performance

| Subsystem | Measurement |
|-----------|------------|
| Config parsing | ~56 us |
| SSE token stream | 0 allocations |
| Cold boot | <20 ms |
| Session checkpoint (200k tokens) | ~45 ms |

## Commands

| Command | What it does |
|---------|-------------|
| `arc chat` | Start an interactive session |
| `arc init` | Bootstrap ARC config in a repo |
| `arc doctor` | Run diagnostics on your setup |
| `arc review` | AI-powered PR review |
| `arc --stats` | Token usage and cost tracking |
| `/plan [task]` | Generate and review a modification plan |
| `/checkpoint` | Save session state |
| `/rewind [id]` | Restore a previous checkpoint |
| `/compact` | Compress context window |
| `/memory [k] [v]` | Persistent key-value store |
| `/fork [name]` | Branch the conversation |
| `/security-review` | Scan diffs for vulnerability patterns |
| `/copy` | Pick and copy code blocks |

## Diff review keybindings

When ARC proposes file changes, you review them interactively:

| Key | Action |
|-----|--------|
| `y` / `Enter` | Accept this change |
| `n` / `Esc` | Reject this change |
| `a` | Accept all remaining |
| `d` | Reject all remaining |
| `e` | Open in `$EDITOR` |
| `j` / `k` | Scroll through large diffs |

## Docs

- [Getting Started](docs/getting-started.md)
- [Architecture](docs/architecture.md)
- [Security Model](docs/authentication.md)
- [Provider Setup](docs/providers.md)
- [MCP Integration](docs/mcp.md)
- [Benchmarks](docs/benchmarks.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The codebase enforces `#![forbid(unsafe_code)]`, `cargo fmt`, and `cargo clippy` on every PR.

## License

MIT
