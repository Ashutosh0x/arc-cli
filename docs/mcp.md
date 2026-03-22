# Model Context Protocol (MCP): The Defensive Execution Engine

ARC CLI does not merely "support" the Anthropic Model Context Protocol (MCP); it fundamentally wraps it in a multi-layered security and execution hypervisor. Traditional Agent CLIs (like Aider) blindly execute whatever arbitrary scripts an LLM suggests. ARC recognizes that third-party MCP servers (often pulled dynamically via `npx` or `uvx`) are the single greatest vector for machine compromise.

## 1. How ARC's MCP Engine Truly Works

The `arc-mcp` crate manages the lifecycle of external tools. Instead of integrating MCP via network sockets (which opens ports loopback ports susceptible to SSRF attacks), ARC forces all MCP traffic strictly over **Standard I/O (Stdio)** wrapped inside isolated process bounds.

### The JSON-RPC Subsystem
When the `arc-router` determines the LLM requires a tool (e.g., querying GitHub or reading an SQLite DB), it routes the payload to the `arc-mcp::Client`.
1. **Instantiation**: `tokio::process::Command` spans the subprocess (`npx @modelcontextprotocol/server-github`).
2. **Streaming Interface**: A dedicated async thread pair binds `stdout` and `stdin`. The LLM's raw XML tool payload is parsed and converted natively into JSON-RPC 2.0 specs.
3. **Execution**: The MCP server evaluates the logic and returns standard JSON-RPC.

## 2. Advanced Security & Manifest Pinning (`arc-mcp::security`)

Because `npx` dynamically fetches unverified code, ARC implements physical execution blockades before the sub-process even spawns.

### Hash Integrity (Manifest Pinning)
In `crates/arc-mcp/src/security.rs`, the `verify_manifest_pin` function enforces cryptographically signed execution:
- Before any MCP tool runs, ARC scans `~/.config/arc/mcp_servers.json`.
- It hashes the target package repository tree (using `sha2::Sha256`).
- If the resolved executable hash does not exactly match the static hash you pinned using `arc mcp pin`, the orchestrator returns an immediate `ArcError::SecurityViolation` and physically kills the tokio process.

### Context Minimization Protocol
LLMs constantly leak sensitive context (like `GOOGLE_API_KEY` or environment variables) into their tool calls mathematically by accident. 
- The `minimize_context()` function structurally analyzes the outward JSON-RPC bound packet.
- It leverages internal `arc-tools` AST parsers to aggressively prune out any variable structures wrapped in standard `.env` patterns, nullifying data exfiltration attempts.

## 3. Sandboxing & OS Hardening

Even if a malicious MCP server bypasses the payload minimization, it executes within absolute constraint.
- **Shadow Workspaces**: Any filesystem-altering capability requested by an MCP tool is intercepted. The OS dynamically shifts the `CWD` into `.arc-shadow/{uuid}` (created via fast hard-links). The MCP tool executes its logic in this physical sandbox. The ARC `Reviewer` agent then inspects the shadow diffs natively before deciding to splice it back to your real `src/` directory.
- **Resource Limits**: The `RateLimiter` ensures MCP servers cannot flood the system with infinite I/O spin-loops by throttling stdio throughput dynamically natively avoiding internal DoS.

## 4. Code Intelligence Graph (codebase-memory-mcp)

ARC ships with native integration for [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) — a zero-dependency structural analysis engine that builds a persistent knowledge graph from your codebase.

### Architecture

```
arc-cli (graph commands)
    ↓
arc-mcp/client.rs     JSON-RPC 2.0 stdio client
    ↓ spawns
codebase-memory-mcp   Pure C binary, tree-sitter AST, SQLite graph
    ↓ returns
arc-mcp/tools.rs      Typed result structs → agent context
```

### Configuration

The MCP server auto-configures in `~/.arc/config.toml`:

```toml
[[mcp.servers]]
name = "codebase-memory"
command = "codebase-memory-mcp"
args = []
auto_start = true
enabled = true
```

### 14 MCP Tools Available

| Category | Tools |
|----------|-------|
| **Indexing** | `index_repository`, `list_projects`, `delete_project`, `index_status` |
| **Querying** | `search_graph`, `trace_call_path`, `detect_changes`, `query_graph` |
| **Analysis** | `get_architecture`, `get_graph_schema`, `get_code_snippet`, `search_code` |
| **Management** | `manage_adr`, `ingest_traces` |

### CLI Interface

```sh
arc graph index                    # index current project
arc graph search ".*Handler.*"     # structural search
arc graph trace main               # call graph from main()
arc graph architecture             # full architecture overview
arc graph impact                   # blast radius of uncommitted changes
arc graph query 'MATCH ...'        # Cypher-like queries
```

### Performance Impact

- **99.2% token reduction** — 5 structural queries consume ~3,400 tokens vs ~412,000 via grep
- **<1ms query latency** — pre-indexed graph responses
- **64 languages** — vendored tree-sitter grammars vs 5 in the basic repomap
- **Persistent memory** — SQLite-backed graph survives sessions and context compaction