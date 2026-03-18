# ARC Agentic Framework Architecture

ARC CLI is constructed exclusively for high-performance scale, structured through a modular Crates.io taxonomy.

## 1. System Topology

### `arc-cli`
The entrypoint process. Uses `ratatui`/`crossterm` for spinning and progress indicators, hosts the primary REPL command loop, and acts as the synchronous controller delegating down to async crates.

### `arc-core`
Hosts foundational components.
- **`Memory Arenas`**: Uses `bumpalo` to enable ultra-fast scratch allocations for context reduction algorithms without dropping memory dynamically. Context bounds are organized into `Working`, `ShortTerm`, and `LongTerm` layers.
- **`Security`**: Defines the `PromptGuard` enforcing strict XML tags against Prompt Injection, the `RateLimiter`, and `Data Guards`.

### `arc-agents`
The absolute core of autonomous behavior. Exposes the `Orchestrator` struct which dynamically instantiates logical sub-agents:
- `Reviewer`
- `Tester`
- `Coder`
- `Security Auditor`
Agents share context cleanly utilizing a highly structured `Agent-to-Agent` (A2A) protocol to prevent context diffusion.

### `arc-session` & `arc-rewind`
State preservation layer. Because ARC can run fully autonomous workflows mapping across hundreds of files, it inherently utilizes embedded `redb` local key-value databases to commit physical filesystem hashes mapped against token conversations. If a hallucination event strikes, `arc-rewind` can dynamically travel backwards safely.

### `arc-shadow` & `arc-worktree`
The "Sandboxing" mechanics. `arc-shadow` duplicates the user workspace into a `.arc-shadow/` localized temporary volume leveraging fast file-system hardlinks on Linux/Mac, preventing the LLM from destroying source repositories during unverified autonomous compilations.

### `arc-io`
The custom High-Performance proxy targeting platform-specific system calls (like `io_uring` on Linux and `IOCP` on Windows) pushing physical I/O read/write thresholds dramatically higher than default synchronous disk wrappers.