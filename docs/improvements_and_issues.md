# ARC CLI: Codebase Issues & Improvement Analysis

After building and reviewing the entire ARC CLI multi-crate workspace spanning thousands of lines of Rust code and 17 core agentic features, the following specific architectural issues, technical debts, and optimization vectors have been identified.

## 1. High Priority Structural Bottlenecks

### `arc-session` (Redb Concurrency Lock)
- **Issue**: The `arc-session` crate relies on `redb` as the underlying local Key-Value store. Currently, `redb` requires exclusive sweeping locks for standard disk transactions. In a highly parallel Multi-Agent orchestrator (`arc-agents`), if three separate agents simultaneously append Memory nodes or Context Checkpoints to `redb`, the resulting thread-locking causes severe async executor stalling across `tokio`, cratering multi-agent execution times.
- **Improvement**: Switch to an Actor-Model channel approach using `mpsc`. A dedicated `StoreActor` thread should consume `MemoryToken` events linearly from an async queue rather than enabling individual SubAgents to establish database locks natively. Alternatively, evaluate completely lock-free async MVCC engines instead of embedded transactional redb.

### `arc-io` (tokio-uring Portability Limits)
- **Issue**: The `read_file_fast` integration manually configures `#cfg[target_os="linux"]` to map to `tokio-uring`. While this produces extreme speedups on Ubuntu servers, macOS and Windows completely bypass zero-copy networking relying exclusively on `tokio::fs::write`.
- **Improvement**: Implement native Completion Ports (IOCP) via the `windows-sys` and `mio` abstractions directly for Windows instead of treating it as an afterthought. macOS could similarly support native `libc::aio` bindings or raw memory maps (`mmap`) rather than standard POSIX fd polling for zero-copy.

### `arc-diff` (Greedy String Allocation Memory Spikes)
- **Issue**: `arc-diff/src/lib.rs` loads the entirety of `original: &str` and `modified: &str` directly to RAM. When executing shadow-workspace merges on generated assets over ~50MB (e.g., massive `.json` schemas or serialized artifacts), `similar::TextDiff` allocates continuous parallel arrays causing catastrophic spikes traversing out of L1/L2 Cache parameters.
- **Improvement**: Integrate a Streaming Diff interface. `similar` supports incremental byte slicing via `slice::chunks`. We must implement a sliding window Diff protocol that generates Patch manifests `diff_strings()` incrementally out to disk rather than retaining it structurally internally.

## 2. Agent Framework & Semantic Safety

### `arc-shadow` (Hardlink Safety Violations)
- **Issue**: The `.arc-shadow/workspace.rs` replicates environments using `std::fs::hard_link()`. While theoretically 0-cost on disk format, compiling inside a shadow workspace containing hardlinked source files means Rust `cargo build` output will directly mutate the binary byte sequences of the linked index, physically corrupting the user's primary un-shadowed workspace in the event of `std::fs::write`.
- **Improvement**: On macOS, swap hardlinks out for `clonefile` `(fclonefileat)`. On Linux, use `BTRFS_IOC_CLONE` or `FICLONE` via `ioctl`. For strict filesystem independence, implement an optimized Copy-on-Write (COW) using system-call intercepts, falling back to full structural copying if native fs COW fails, completely eliminating Hardlink-induced corruption risks.

### `arc-policy` (Regex Evaluation Efficiency)
- **Issue**: Evaluating rules recursively across every single path file write inside the `AutonomousLoop` uses greedy Regex loops. This slows down raw A2A generation rates substantially because every event evaluates multiple linear constraints.
- **Improvement**: Compile policies down to a finite state automaton (FSA) using `aho-corasick` or `regex-automata` loaded inside a global `OnceLock` singleton matrix, generating an algorithmic complexity of O(n) relative entirely to path string length rather than regex breadth.

### `arc-hooks` (Blocking TTY Lifecycle)
- **Issue**: Standard git `pre-commit` hooks expect a strict TTY zero-exit stdout. Booting up the full `tokio` multi-threaded async executor locally for `arc-hooks` inside a terminal environment causes perceptible heavy (~200ms) pauses prior to git logging commits, resulting in sluggish DevEx.
- **Improvement**: Delegate `arc-hooks` into a native persistent background Daemon (`arc-loop serve --daemon`). Rather than booting the multi-layer framework stack inside the `.git/hooks/` namespace, emit a rapid lightning-fast socket transmission via Unix-Socket/NamedPipe to the Daemon to instantaneously return the constraint matrix.

## 3. Tooling & Security Overlays

### `arc-mcp` (Tool Execution Isolation)
- **Issue**: Currently MCP security relies heavily on `Manifest Pinning` effectively avoiding unverified servers. It lacks execution-level sandbox bounds around the actual third-party JS scripts spawned by `npx`.
- **Improvement**: Execute `mcpServers` wrapped recursively inside native specific OS isolation constraints such as `seccomp-bpf` namespaces on Linux, or AppContainer contexts on Windows. Completely preventing tool execution processes from bleeding out beyond their internal parameters.

### `arc-router` (Model Token Truncation Bias)
- **Issue**: The `router.rs` classification determines Specialist agent routing. However, user input queries aren't actively token-clipped. A massive context context block will overwhelm the base router LLM (e.g., Haiku or GPT-4o-mini), resulting in classification hallucination and dispatching a query to the `Test Engineer` when it structurally requires the `Security Auditor`.
- **Improvement**: Use `arc-tools` leveraging `tree-sitter` natively to automatically filter out Code AST definitions passing exclusively raw markdown reasoning patterns to the Router, shrinking tokens significantly while radically increasing Router classification accuracy. 
