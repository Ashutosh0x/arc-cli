# Plugins & Extensibility Architecture

ARC CLI is built utilizing a fundamentally sandboxed, infinitely extensible plugin architecture. Given the inherent risks of autonomous agents executing code, standard local OS execution is minimized. Instead, the framework leans heavily into **WebAssembly (WASM)** and the **Model Context Protocol (MCP)**.

---

## 1. WebAssembly (WASM) Plugins (`arc-plugins`)

WASM is the absolute gold standard for executing unverified, third-party code fast and securely natively. The `arc-plugins` subsystem leverages a WASM runtime (like `wasmtime` or `extism`) to mount user-defined or community-defined plugins.

### Zero-Trust Sandboxing
All plugins run internally inside a WASM linear memory bound. They mathematically cannot access the host machine's filesystem, environment variables, or network sockets without explicitly passing through ARC's extreme capability delegation layer (`arc-tools::security::sandbox`).
- **Network Bound**: Plugins must request an explicit outbound domain allowlist.
- **Path Bound**: Only the active repository workspace is mapped into WASM VFS arrays.

### Plugin Contracts
Plugins communicate with the `arc-core` utilizing standard JSON payloads pushed over memory arrays. Agents can dynamically discover registered WASM plugins locally in `~/.arc/plugins/` and load them into memory instantly natively with `mmap`.

```rust
// Internally handled by ARC Agent delegates natively:
let plugin = WasmEngine::load("typescript_linter.wasm");
let analysis = plugin.call("analyze_ast", &target_file);
```

---

## 2. Model Context Protocol (MCP) (`arc-mcp`)

While WASM handles high-performance stateless analysis plugins, ARC utilizes standard **Model Context Protocol (MCP)** for connecting the autonomous agent to external enterprise state systems natively.

### What is MCP?
MCP is an open standard originally created by Anthropic that standardizes how LLM agents interface with massive external APIs, SaaS applications, and enterprise data warehouses natively.

### Architecture in ARC
When an agent is assigned a task, it utilizes `arc-mcp` to negotiate capabilities symmetrically with local or remote MCP servers utilizing stdio or SSE.
- **Manifest Pinning**: ARC evaluates the MCP tool manifest returned by the server explicitly mapping a cryptographic hash matrix securely preventing supply chain tool injection.
- **Context Minimization**: Even if an MCP server requests raw data, ARC utilizes strict privacy bounds natively censoring irrelevant data points preventing data exfiltration to massive 3rd party host networks natively.

---

## 3. Git Hooks & Code Lifecycle (`arc-hooks`)

The final layer natively ties automated execution tightly into the Git lifecycle:
- **Pre-Commit**: Agents can be hooked explicitly mapping into `.git/hooks/pre-commit` to read diffs natively, automatically validating code boundaries and security rules using `arc-policy` before code hits version control securely.

The entire system dynamically supports robust automation allowing enterprise organizations to natively define, restrict, and execute multi-agent workloads safely without sacrificing developer iteration speed natively.