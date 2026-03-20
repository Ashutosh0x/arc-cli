# ARC: High-Performance Agentic CLI
<img width="2048" height="861" alt="arc cli" src="https://github.com/user-attachments/assets/4952031a-b3fa-4074-8573-18fe7a9832d8" />

![Build](https://img.shields.io/badge/Build-Passing-brightgreen?style=for-the-badge)
![Features](https://img.shields.io/badge/Features-120+-blue?style=for-the-badge)
![Crates](https://img.shields.io/badge/Crates-31-orange?style=for-the-badge)
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

## Installation

### Quick Install (Recommended)

**Linux / macOS:**
```sh
curl -fsSL https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Ashutosh0x/arc-cli/main/install.ps1 | iex
```

**From Source:**
```sh
cargo install --git https://github.com/Ashutosh0x/arc-cli --locked
```

| Platform | Command |
|----------|---------|
| Homebrew | `brew install arc-cli` |
| Scoop | `scoop install arc-cli` |
| Cargo | `cargo install arc-cli` |
| AUR | `yay -S arc-cli` |

## Why ARC?

| | ARC CLI | Claude Code | Gemini CLI |
|---|---------|-------------|------------|
| **Language** | Rust (native binary) | Node.js | Node.js |
| **Cold Boot** | <20ms | ~500ms | ~400ms |
| **Runtime Deps** | None | Node 18+ | Node 18+ |
| **Offline Support** | ✅ Ollama | ❌ | ❌ |
| **Multi-Provider** | ✅ 4 providers | ❌ Anthropic only | ❌ Google only |
| **Memory Safety** | `#![forbid(unsafe_code)]` | N/A | N/A |
| **Binary Size** | ~15MB static | ~200MB+ | ~150MB+ |


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

### 7. Claude Code Parity — Full Feature Gap Closure (Phase 29-33)
Complete parity with Claude Code's 80+ production releases, plus 6 areas where ARC leads:

- **Hook System**: Event-driven PreToolUse/PostToolUse/Stop hooks with command/HTTP executors and 6 operators (eq, contains, regex, glob, starts_with, not).
- **Permission System**: 3-tier allow/ask/deny per tool with compound bash command parsing, enterprise managed policies, and 50+ auto-approved safe commands.
- **Context Compaction**: 3-phase auto-compaction (strip media → truncate tools → summarize), circuit breaker, `/compact` command, and `/context` diagnostics.
- **Plugin Marketplace**: Git & local plugin install with `plugin.json` manifests, version pinning, trust levels (Untrusted/UserTrusted/ManagedTrusted), update, and validate.
- **Voice Mode**: Push-to-talk framework supporting 20 languages with CoreAudio (macOS), WASAPI (Windows), PulseAudio/ALSA (Linux) backend detection.
- **Agent Teams**: Leader/teammate/background roles with inter-agent messaging, worktree-based isolation, and configurable max agents.
- **Plan Mode (Full)**: `/plan` with accept/reject/feedback, step-by-step tracking, plan history, and compaction-resistant persistence.
- **Session Fork/Branch**: `/fork` creates conversation branches with independent plans and selective state copying.
- **Slash Commands**: `.md` commands with YAML frontmatter (description, argument-hint, allowed-tools, effort), `$ARGUMENTS` interpolation, and auto-discovery.
- **Auto-Memory**: `/memory` with persistent JSON store, auto/user/session sources, timestamps, and access counting.
- **Agent Definitions**: `.arc/agents/*.md` with 10 frontmatter fields (name, tools, model, color, effort, background, isolation, memory).
- **Skills System**: Recursive SKILL.md discovery with deduplication and `${ARC_SKILL_DIR}` variable resolution.
- **Settings Hierarchy**: User → Project → Managed layer precedence with Windows Registry, macOS plist, and Linux `/etc` stubs.
- **Effort Levels**: Low (○) / Medium (◐) / High (●) with auto-mode and `/effort` command.
- **Feature Flags**: Disk-cached dynamic feature toggles with stale value prevention.
- **Background Tasks**: 5GB output cap with kill-all support and `Ctrl+B` queries.
- **Security Review**: `/security-review` scans merge-base diffs against 9 vulnerability patterns with severity-rated findings.
- **Tool Search**: Deferred tool loading reduces initial context — loads schemas on demand via tag/name search.
- **Ralph Loop**: Autonomous iteration with `--max-iterations`, completion promise, pause/resume, and iteration history.
- **PR Review Toolkit**: 6 specialized agents (comment-analyzer, pr-test-analyzer, silent-failure-hunter, type-design-analyzer, code-reviewer, code-simplifier).
- **Feature-Dev Workflow**: 7-phase structured development (Discovery → Exploration → Questions → Architecture → Implementation → QA → Summary).
- **Sandbox Network Policy**: `allowedDomains`, proxy ports, Unix sockets, filesystem isolation with Landlock/sandbox-exec platform deps.
- **Statusline**: Configurable segments (model, effort, context%, rate limits, worktree) with custom scripts.
- **Copy Picker**: `/copy` with interactive code block selection, copy-to-clipboard, and write-to-file.
- **Platform Hardening**: Path normalization (Windows drive case), CRLF detection/conversion, WSL detection, XDG-compliant directories.

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
| `arc-core` | **Foundation Layer (50+ modules)** | Credentials, config, hooks, permissions, compaction, plugins, voice, effort, settings, memory, slash commands, agent defs, skills, security review, tool search, ralph loop, PR review, feature flags, background tasks, platform hardening, and 30+ more. |
| `arc-providers` | **Model Gateways** | Native Gemini, Claude, OpenAI clients + model availability + fallback handler. |
| `arc-agents` | **Swarm Delegation** | Orchestrator dispatcher + agent teams (leader/teammate/background). |
| `arc-a2a` | **Agent Protocol** | HTTP/2 SSE communications secured by HMAC/JWT signatures. |
| `arc-session` | **Persistent Memory** | `redb` K-V storage, checkpoint undo/rewind, auto session summaries, session fork/branch. |
| `arc-policy` | **Safety Engine** | Conseca dynamic policies, 3-tier permission system, sandbox network policy. |
| `arc-plan` | **Planning Mode** | Full plan mode with accept/reject/feedback, DAG tracker, 7-phase feature-dev workflow. |
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
- `/plan [task]` to autonomously generate a codebase modification blueprint (accept/reject/feedback).
- `/doctor` to evaluate your workspace and credential configurations safely.
- `/checkpoint` to snapshot the LLM turn history and file states.
- `/rewind [id]` to safely time-travel the CLI state backward to correct errant agent behavior.
- `/compact` to manually trigger context compaction when nearing token limits.
- `/context` to display token usage diagnostics and context window breakdown.
- `/memory [key] [value]` to save persistent context across sessions.
- `/effort [low|medium|high]` to adjust the AI's response depth and thoroughness.
- `/fork [name]` to branch the current conversation with selective state copying.
- `/loop [interval] [prompt]` to schedule recurring task execution.
- `/copy` to interactively pick and copy code blocks from responses.
- `/security-review` to audit your branch diff against 9 vulnerability patterns.
- `arc init` to bootstrap ARC rules recursively across un-initialized repos.
- `arc --stats` for live LLM cost accounting and budget tracking.
- `arc review` to generate pre-push architectural critiques via 6 specialized agents.

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

## Complete Feature Matrix (120+ Features)

### Core Architecture
| Feature | Implementation |
| :--- | :--- |
| **Language** | Rust — zero-cost abstractions, no garbage collector |
| **Workspace** | 31 modular crates with 180+ source files and strict dependency boundaries |
| **Binary** | Single static binary — no Node.js/Python runtime required |
| **Cold Boot** | <20ms startup via `OnceLock` + LTO + codegen-units=1 |
| **Memory Safety** | `#![forbid(unsafe_code)]` — compile-time guarantee across entire workspace |
| **Async Runtime** | Tokio work-stealing scheduler with epoll (Linux) / IOCP (Windows) |
| **Allocator** | mimalloc drop-in replacement for 10-15% throughput gains |

### Multi-Provider Intelligence
| Feature | Implementation |
| :--- | :--- |
| **Gemini** | Native Google Gemini API client with streaming |
| **Anthropic Claude** | Full Claude 3.5/4 streaming client |
| **OpenAI** | GPT-4o / GPT-4o-mini compatible |
| **Ollama** | Local model support — works fully offline |
| **Fallback Chain** | Policy-driven: retry_always → retry_once → stop → upgrade |
| **Model Availability** | Terminal vs sticky-retry health tracking per model |
| **Circuit Breaker** | Per-provider trip after N failures with cooldown |
| **Multi-Modal Routing** | Image input → auto-routes to vision-capable provider |
| **Model Aliases** | `"fast"` → gpt-4o-mini, `"smart"` → claude-sonnet |
| **Cost Estimation** | Pre-execution cost display ("This will cost ~$0.34") |

### Streaming & Performance
| Feature | Implementation |
| :--- | :--- |
| **SSE Parser** | Zero-alloc SIMD `memchr` byte-slicing — 0 heap allocations |
| **HTTP** | HTTP/2 globally pooled connections with zero-copy streaming |
| **Parallel Probing** | Startup health-checks all configured providers concurrently |
| **Token Budgeting** | Hard caps per session / day / month |
| **Benchmarks** | Criterion micro-benchmarks for SSE parser, config, memory |

### Runtime Intelligence
| Feature | Implementation |
| :--- | :--- |
| **Loop Detection** | 3-layer: SHA-256 tool-call dedup → content chanting → LLM double-check |
| **Tool Output Masking** | Hybrid Backward-Scanned FIFO — 50k protection window, 30k batch threshold |
| **JIT Context** | Dynamically discovers `ARC.md` files as agent navigates subdirectories |
| **Session Summaries** | Auto-generates ≤80 char titles via fast-model sliding-window |
| **Context Compression** | 5-layer memory (Arena → Working → ShortTerm → LongTerm → Compressor) |
| **Diff Snippets** | Head+tail context generator around changed lines with merged ranges |

### Safety & Security
| Feature | Implementation |
| :--- | :--- |
| **Env Sanitization** | Regex blocks 15+ secret patterns: JWT, AWS `AKIA*`, GitHub `ghp_*`, RSA keys |
| **Conseca Policies** | LLM-generated SecurityPolicy per request — adapts tool/arg constraints |
| **Folder Trust** | Pre-trust scanning for commands, MCPs, hooks, skills, agents |
| **OS Sandbox** | Landlock syscall filters (Linux) + shadow workspace CoW isolation |
| **Prompt Guard** | Instruction hierarchy + lethal trifecta detection + context isolation |
| **Credential Manager** | OS keyring + zeroize memory scrubbing |
| **Audit Logging** | Structured audit trail for all tool executions |
| **Rate Limiter** | Token-bucket per provider with configurable burst |
| **Supply Chain** | cargo-audit + cargo-vet + cargo-auditable + deny.toml |
| **Session Guard** | Multi-stage attack detection with auto-escalation |

### Agentic Features
| Feature | Implementation |
| :--- | :--- |
| **Plan Mode (Full)** | `/plan` with accept/reject/feedback, step tracking, plan history, compaction-resistant |
| **Multi-Agent** | Orchestrator dispatches specialized subagents (Planner, Architect, Coder) |
| **Agent Teams** | Leader/teammate/background roles with inter-agent messaging and isolation |
| **A2A Protocol** | HTTP/2 SSE agent-to-agent communication with HMAC/JWT auth |
| **Checkpointing** | Atomic `redb` snapshots of full session state |
| **Rewind** | Time-travel to any checkpoint — safely reverts files on disk |
| **Session Forking** | `/fork` branching with selective state copying and independent plans |
| **Shadow Workspace** | CoW hardlink isolation for autonomous file changes |
| **Git Worktree** | First-class sparse-checkout with auto-cleanup of stale worktrees |
| **Skills System** | Recursive SKILL.md discovery with dedup and `${ARC_SKILL_DIR}` |
| **Autonomous Loop** | `/loop` cron scheduling (s/m/h intervals, max iterations) |
| **Ralph Loop** | Autonomous iteration with `--max-iterations` and completion detection |
| **Hook System** | Event-driven PreToolUse/PostToolUse/Stop/SessionStart hooks with executors |

### Developer Experience
| Feature | Implementation |
| :--- | :--- |
| **IDE Detection** | Auto-detects 20+ IDEs: VS Code, Cursor, Zed, JetBrains, Xcode, Neovim |
| **Extensions CLI** | 10 subcommands: install, uninstall, link, update, configure, enable, disable, validate, new, list |
| **Prompt Registry** | Versioned templates with variable substitution (6 built-in prompts) |
| **Slash Commands** | `.md` commands with YAML frontmatter, `$ARGUMENTS`, and auto-discovery |
| **Auto-Memory** | `/memory` persistent store with auto/user/session sources and timestamps |
| **Agent Definitions** | `.arc/agents/*.md` with 10 frontmatter fields |
| **Settings Hierarchy** | User → Project → Managed with platform-specific managed settings |
| **Effort Levels** | Low ○ / Medium ◐ / High ● with auto mode and `/effort` command |
| **Statusline** | Configurable segments: model, effort, context%, rate limits, worktree |
| **Copy Picker** | `/copy` interactive code block selection with write-to-file |
| **REPL** | rustyline with Tab completion, Ctrl+R SQLite history, @-mention file resolution |
| **Diff Review** | Granular multi-key review: y/n/a/d/e/j/k/s/? |
| **Syntax Highlighting** | syntect-powered diff previews |
| **Self-Updater** | `arc update` binary self-update via GitHub releases |
| **Diagnostics** | `arc doctor` diagnostic bundle (config + logs + system info) |

### Tooling
| Feature | Implementation |
| :--- | :--- |
| **File Operations** | Read, write, edit with path guard + symlink defense |
| **Shell Execution** | Sandboxed with AST-based safety (PowerShell AST on Windows) |
| **MCP Client** | Manifest-pinned with hash verification against tool injection |
| **AST Repo Map** | tree-sitter extraction for Python, TypeScript, Go, C++, Rust |
| **Web Search** | Grounded web search via arc-search |
| **Vision/Image** | Image input processing via arc-vision |
| **Voice** | STT + push-to-talk — 20 languages, CoreAudio/WASAPI/PulseAudio detection |
| **PR Review** | 6 specialized agents: comment-analyzer, test-analyzer, failure-hunter, type-design, reviewer, simplifier |
| **Security Review** | `/security-review` merge-base diff scanning, 9 vuln patterns, severity findings |
| **Tool Search** | Deferred tool loading via tag/name search, reduces initial context |
| **Background Tasks** | 5GB cap, process lifecycle, kill-all, `Ctrl+B` queries |

### Observability & Telemetry
| Feature | Implementation |
| :--- | :--- |
| **Structured Logging** | tracing + tracing-subscriber with span context |
| **OpenTelemetry** | Full OTLP traces for provider API calls |
| **Activity Monitor** | Tracks user activity patterns with idle detection |
| **Memory Monitor** | RSS/heap tracking with high water mark alerts |
| **Startup Profiler** | Checkpoint-based timing for cold boot analysis |
| **Cost Dashboard** | `arc --stats` live token/cost tracking per session |
| **Billing** | Per-model cost estimation with overage strategies |

### Ecosystem & Distribution
| Feature | Implementation |
| :--- | :--- |
| **Offline Mode** | Ollama-first routing, graceful degradation, request queuing |
| **Config Management** | Schema versioning, hot-reload, XDG compliance, env profiles |
| **Shell Completions** | bash / zsh / fish via `arc completions` |
| **JSON Mode** | `--json` machine-readable output for all commands |
| **Error Codes** | Structured ARC-0001 through ARC-9999 with doc URL mapping |
| **Cloud Delegation** | arc-cloud for async task offloading |
| **Plugin Marketplace** | Git/local install, `plugin.json` manifests, trust levels, validate, update |
| **Feature Flags** | Disk-cached dynamic toggles with stale value prevention |
| **Feature-Dev Workflow** | 7-phase structured development (Discovery → Summary) |

### Platform Hardening
| Feature | Implementation |
| :--- | :--- |
| **Windows** | Drive letter casing normalization, CRLF detection/conversion, Registry managed settings, OneDrive compat |
| **macOS** | CoreAudio backend, `sandbox-exec` integration, keychain stubs, managed plist settings |
| **Linux** | Landlock syscall filters, PulseAudio/ALSA detection, `/etc/arc-cli` managed settings, WSL detection |
| **Cross-Platform** | Unicode clipboard (PowerShell/pbcopy/xclip/wl-copy), XDG dirs, platform shell detection |

---
<div align="center">

Built with ❤️ for autonomous agents

</div>
