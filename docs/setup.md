# ARC CLI: Configuration & Setup

After installing ARC CLI, you must connect the agent framework to an external intelligence provider and optionally integrate your Model Context Protocol (MCP) servers.

## 1. Automated Setup Wizard

ARC CLI provides a native, interactive UI wizard for seamless setup. Start it by running:

```bash
arc setup
```

The wizard will prompt you with multiple choices:
- Select your default underlying Provider (Anthropic Claude, Google Gemini, OpenAI, or Local Ollama).
- Test connection bandwidth and latency natively.
- Scaffold your global configuration file (`~/.config/arc/config.toml`).

## 2. Manual Environment Configuration

If running in CI/CD pipelines, ARC can be securely initialized by passing environmental variables. ARC expects keys securely saved in OS-level Keyrings by default, but falls back to `ENV`:

- `ANTHROPIC_API_KEY`: The API key for Claude 3.5 Sonnet / Opus.
- `GOOGLE_API_KEY`: API Key for Gemini 1.5 Pro deep context interactions.
- `OPENAI_API_KEY`: Standard GPT-4o fallback compatibility.

## 3. Creating Workspace Rules (`ARC.md`)

ARC introduces highly-aware structural memory natively. 

In the root folder of any software project repository, you can create a file named `ARC.md`. This file dictates the unshakeable context guidelines of the repository.

```markdown
# ARC.md
# Architecture
This project uses Rust multi-workspace traits. 
Do not use `serde` directly in the CLI crate, keep it within `arc-core`.

# Exclusions
Never edit files inside the `tests/fixtures/` directly. 
```
Every agent deployed via `arc chat` or `arc diff` permanently reads this document into the `System` instruction prompt guard.

## 4. MCP Server Registration

The Agent framework relies heavily on tools for introspection and manipulation.

Register standard tools by editing `~/.config/arc/mcp_servers.json`:

```json
{
  "mcpServers": {
    "sqlite": {
      "command": "uvx",
      "args": ["mcp-server-sqlite", "--db-path", "./test.db"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"]
    }
  }
}
```

Ensure you pin your manifests using the `arc mcp pin` command if operating in high-security bounds.
