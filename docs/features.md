# ARC CLI — Complete Feature Reference

> 31 crates · 120+ features · Pure Rust · Zero runtime dependencies

## ARC CLI vs Gemini CLI — Head-to-Head

| Category | ARC CLI Wins | Gemini CLI Wins | Ties |
|----------|:---:|:---:|:---:|
| Core Architecture | **6** | 0 | 0 |
| Provider Support | **4** | 1 | 2 |
| Streaming & Performance | **4** | 0 | 1 |
| Runtime Intelligence | **2** | 0 | 4 |
| Safety & Security | **8** | 0 | 4 |
| Agentic Features | **10** | 0 | 3 |
| Developer Experience | **11** | 0 | 3 |
| Tooling | **8** | 0 | 3 |
| Observability | **3** | 1 | 3 |
| Ecosystem & Platform | **4** | **2** | 1 |
| **TOTAL** | **60** | **4** | **24** |

> **ARC CLI leads 60-4.** Gemini CLI's advantages: production maturity, Google ecosystem.

---

## Core Architecture

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Language** | Rust (zero-cost, no GC) | TypeScript/Node.js |
| **Workspace** | 31 crates, 180+ source files | ~8 packages |
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
| **Permission System** | ✅ 3-tier allow/ask/deny + bash parsing | ❌ |
| **Sandbox Network Policy** | ✅ allowedDomains, proxy, Unix sockets | ❌ |
| **Security Review** | ✅ `/security-review` 9 vuln patterns | ❌ |
| **Audit Logging** | ✅ Structured trail | ❌ |
| **Rate Limiter** | ✅ Token bucket | ❌ |
| **Supply Chain** | ✅ audit + vet + deny.toml | npm audit (weaker) |

## Agentic Features

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Plan Mode (Full)** | ✅ Accept/reject/feedback, step tracking | ✅ Enter/exit plan |
| **Multi-Agent** | ✅ Orchestrator dispatch | ❌ |
| **Agent Teams** | ✅ Leader/teammate/background, messaging | ❌ |
| **A2A Protocol** | ✅ HTTP/2 + HMAC/JWT | ❌ |
| **Checkpointing** | ✅ redb atomic | ❌ |
| **Rewind** | ✅ Time-travel | ❌ |
| **Session Forking** | ✅ /fork with independent plans | ❌ |
| **Shadow Workspace** | ✅ CoW hardlinks | ❌ |
| **Git Worktree** | ✅ Sparse checkout + auto-cleanup | ❌ |
| **Skills System** | ✅ Recursive discovery, dedup | ✅ Built-in skills |
| **Hook System** | ✅ 12 events, command/HTTP executors | ❌ |
| **Ralph Loop** | ✅ Autonomous iteration, completion detect | ❌ |
| **Autonomous Loop** | ✅ /loop cron scheduling | ❌ |

## Developer Experience

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **IDE Detection** | ✅ 20+ IDEs | ✅ 20+ IDEs |
| **Extensions CLI** | ✅ 10 subcommands | ✅ 9 subcommands |
| **Prompt Registry** | ✅ Versioned templates | ✅ Registry + snippets |
| **Slash Commands** | ✅ .md + YAML frontmatter + $ARGUMENTS | ❌ |
| **Auto-Memory** | ✅ /memory persistent store | ❌ |
| **Agent Definitions** | ✅ .arc/agents/*.md (10 fields) | ❌ |
| **Settings Hierarchy** | ✅ User→Project→Managed + hot-reload | ❌ |
| **Effort Levels** | ✅ ○ ◐ ● with auto mode | ❌ |
| **Statusline** | ✅ Model, effort, ctx%, rate limits | ❌ |
| **Copy Picker** | ✅ /copy interactive block selection | ❌ |
| **REPL** | ✅ rustyline + Tab + history | Basic readline |
| **Diff Review** | ✅ y/n/a/d/e/j/k/s/? | Basic Y/n |
| **Syntax Highlighting** | ✅ syntect | ❌ |
| **Self-Updater** | ✅ arc update | ❌ |

## Tooling

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **File Operations** | ✅ Path guard + symlink defense | ✅ |
| **Shell Execution** | ✅ AST-sandboxed | ✅ |
| **MCP Client** | ✅ Manifest-pinned | ✅ |
| **AST Repo Map** | ✅ tree-sitter (5 langs) | ❌ |
| **Web Search** | ✅ arc-search | ❌ |
| **Vision/Image** | ✅ arc-vision | ❌ |
| **Voice** | ✅ 20 languages, PTT | ✅ Formatter |
| **PR Review** | ✅ 6 specialized agents | ❌ |
| **Tool Search** | ✅ Deferred loading, tag search | ❌ |
| **Background Tasks** | ✅ 5GB cap, kill-all, Ctrl+B | ❌ |

## Observability

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Logging** | ✅ tracing + subscriber | ✅ Custom loggers |
| **OpenTelemetry** | ✅ OTLP traces | ❌ |
| **Activity Monitor** | ✅ Idle detection | ✅ |
| **Memory Monitor** | ✅ RSS + high water mark | ✅ |
| **Cost Dashboard** | ✅ arc --stats | ❌ |
| **Telemetry Modules** | ~10 modules | 52 files |

## Ecosystem & Platform

| Feature | ARC CLI 🦀 | Gemini CLI 🔷 |
| :--- | :--- | :--- |
| **Billing** | ✅ Per-model estimation | ✅ Google One AI |
| **Offline Mode** | ✅ Ollama-first | ❌ |
| **Cloud** | ✅ arc-cloud | ✅ Google Cloud |
| **Plugin Marketplace** | ✅ Git/local, manifests, trust | ❌ |
| **Feature Flags** | ✅ Disk-cached toggles | ✅ Feature flags |
| **Feature-Dev Workflow** | ✅ 7-phase structured dev | ❌ |
| **Platform Hardening** | ✅ Win/Mac/Linux full parity | Partial |

