# ARC CLI — Complete Feature Reference

> 31 crates · 85+ features · Pure Rust · Zero runtime dependencies

## ARC CLI vs Gemini CLI — Head-to-Head

| Category | ARC CLI Wins | Gemini CLI Wins | Ties |
|----------|:---:|:---:|:---:|
| Core Architecture | **6** | 0 | 0 |
| Provider Support | **4** | 1 | 2 |
| Streaming & Performance | **4** | 0 | 1 |
| Runtime Intelligence | **2** | 0 | 4 |
| Safety & Security | **6** | 0 | 4 |
| Agentic Features | **7** | 0 | 3 |
| Developer Experience | **6** | 0 | 3 |
| Tooling | **4** | 0 | 3 |
| Observability | **3** | 1 | 3 |
| Ecosystem | 0 | **4** | 0 |
| **TOTAL** | **42** | **6** | **23** |

> **ARC CLI leads 42-6.** Gemini CLI's advantages: production maturity, Google ecosystem, battle-tested telemetry.

---

## Core Architecture

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Language** | Rust (zero-cost, no GC) | TypeScript/Node.js |
| **Workspace** | 31 modular crates | ~8 packages |
| **Binary** | Single static binary | Requires Node.js |
| **Cold Boot** | <20ms (OnceLock + LTO) | ~200ms |
| **Memory Safety** | `#![forbid(unsafe_code)]` | Runtime GC |
| **Async Runtime** | Tokio work-stealing | Node.js event loop |
| **Allocator** | mimalloc replacement | V8 default |

## Multi-Provider Intelligence

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Gemini** | ✅ | ✅ (native) |
| **Anthropic Claude** | ✅ | ❌ |
| **OpenAI** | ✅ | ❌ |
| **Ollama (local)** | ✅ | ❌ |
| **Fallback Chain** | ✅ Policy-driven intents | ✅ Model availability |
| **Circuit Breaker** | ✅ Per-provider | ❌ |
| **Multi-Modal Routing** | ✅ Vision auto-route | ❌ |
| **Model Aliases** | ✅ fast/smart/vision | ❌ |
| **Cost Estimation** | ✅ Pre-execution | ❌ |

## Streaming & Performance

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **SSE Parser** | Zero-alloc SIMD memchr | Standard JS |
| **HTTP** | HTTP/2 pooled zero-copy | HTTP/1.1 fetch |
| **Parallel Probing** | ✅ Concurrent startup | ❌ |
| **Token Budgeting** | ✅ Session/day/month caps | ✅ Tool masking |
| **Benchmarks** | Criterion micro-benchmarks | ❌ |

## Runtime Intelligence

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Loop Detection** | ✅ 3-layer (SHA-256 + chanting + LLM) | ✅ Identical |
| **Tool Output Masking** | ✅ FIFO 50k protection | ✅ Identical |
| **JIT Context** | ✅ ARC.md discovery | ✅ GEMINI.md |
| **Session Summaries** | ✅ Sliding-window | ✅ Identical |
| **Context Compression** | ✅ 5-layer memory system | ❌ |
| **Diff Snippets** | ✅ Head+tail context | ✅ Identical |

## Safety & Security

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Env Sanitization** | ✅ 15+ patterns | ✅ Identical |
| **Conseca Policies** | ✅ Dynamic SecurityPolicy | ✅ Identical |
| **Folder Trust** | ✅ Pre-trust scanning | ✅ Identical |
| **OS Sandbox** | ✅ Landlock + shadow | ❌ |
| **Prompt Guard** | ✅ Instruction hierarchy | ❌ |
| **Audit Logging** | ✅ Structured trail | ❌ |
| **Rate Limiter** | ✅ Token bucket | ❌ |
| **Supply Chain** | ✅ audit + vet + deny.toml | npm audit (weaker) |

## Agentic Features

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Plan Mode** | ✅ DAG task tracker | ✅ Enter/exit plan |
| **Multi-Agent** | ✅ Orchestrator dispatch | ❌ |
| **A2A Protocol** | ✅ HTTP/2 + HMAC/JWT | ❌ |
| **Checkpointing** | ✅ redb atomic | ❌ |
| **Rewind** | ✅ Time-travel | ❌ |
| **Session Forking** | ✅ arc-fork | ❌ |
| **Shadow Workspace** | ✅ CoW hardlinks | ❌ |
| **Git Worktree** | ✅ Sparse checkout | ❌ |
| **Skills System** | ✅ arc-skills | ✅ Built-in skills |
| **Autonomous Loop** | ✅ File watchers | ❌ |

## Developer Experience

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **IDE Detection** | ✅ 20+ IDEs | ✅ 20+ IDEs |
| **Extensions CLI** | ✅ 10 subcommands | ✅ 9 subcommands |
| **Prompt Registry** | ✅ Versioned templates | ✅ Registry + snippets |
| **REPL** | ✅ rustyline + Tab + history | Basic readline |
| **Diff Review** | ✅ y/n/a/d/e/j/k/s/? | Basic Y/n |
| **Syntax Highlighting** | ✅ syntect | ❌ |
| **Self-Updater** | ✅ arc update | ❌ |
| **Diagnostics** | ✅ arc doctor | ❌ |
| **Init Wizard** | ✅ arc init | ❌ |

## Tooling

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **File Operations** | ✅ Path guard + symlink defense | ✅ |
| **Shell Execution** | ✅ AST-sandboxed | ✅ |
| **MCP Client** | ✅ Manifest-pinned | ✅ |
| **AST Repo Map** | ✅ tree-sitter (5 langs) | ❌ |
| **Web Search** | ✅ arc-search | ❌ |
| **Vision/Image** | ✅ arc-vision | ❌ |
| **Voice** | ✅ STT + PTT | ✅ Formatter |
| **PR Review** | ✅ arc review | ❌ |

## Observability

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Logging** | ✅ tracing + subscriber | ✅ Custom loggers |
| **OpenTelemetry** | ✅ OTLP traces | ❌ |
| **Activity Monitor** | ✅ Idle detection | ✅ |
| **Memory Monitor** | ✅ RSS + high water mark | ✅ |
| **Cost Dashboard** | ✅ arc --stats | ❌ |
| **Telemetry Modules** | ~10 modules | 52 files |

## Ecosystem

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Billing** | ✅ Per-model estimation | ✅ Google One AI |
| **Offline Mode** | ✅ Ollama-first | ❌ |
| **Cloud** | ✅ arc-cloud | ✅ Google Cloud |
| **Plugins** | ✅ Manifest + marketplace | ❌ |
| **Experiments** | ❌ | ✅ Feature flags |
| **Production Users** | Early stage | Google-backed, millions |
