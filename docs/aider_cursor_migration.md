# Migrating to ARC from Aider or Cursor

Welcome to ARC! If you're coming from established AI coding assistants, you'll find ARC familiar but fundamentally different under the hood. 

## If you're coming from Aider...

Aider is a fantastic tool that edits files in-place using unified diffs. 

### What's Similar:
- **Terminal Native**: Both live in your terminal.
- **Git Bound**: Both respect `.gitignore` and commit changes for you.

### What's Different:
- **Multi-Agent Orchestration**: Aider uses a single system prompt. ARC uses a graph of specialized agents (Planner -> Architect -> Coder), resulting in far fewer hallucinated architectural breaks on complex files.
- **Headless Mode**: ARC can operate completely headlessly in CI/CD pipelines (e.g. `arc review --base origin/main`).
- **Shadow Workspace**: By default, ARC operates in `.arc-shadow/`. Changes are compiled and verified via `cargo check` (or your language's equivalent) **before** they are copied to your main workspace, eliminating broken state errors.

## If you're coming from Cursor...

Cursor is a fork of VSCode that deeply integrates AI into the IDE layout.

### What's Similar:
- **Project Wide Context**: Cursor's "Composer" and ARC's "Plan Mode" both gather massive project-wide context to execute multi-file changes.
- **Provider Bring-Your-Own-Key**: Both support Anthropic, OpenAI, etc.

### What's Different:
- **No Vendor Lock-in**: You don't need a special IDE. ARC works identically whether you use Vim, Neovim, JetBrains, or stock VSCode. You pipe ARC directly into whatever workflow you already have.
- **Open Source Security**: Your API keys are strictly local by default. They are never bounced through a centralized proxy server (unlike Cursor's default architecture) unless you explicitly configure an enterprise gateway.

---
**Quick Start for Aider Users**:
Run `arc chat` (identical to running `aider`).
**Quick Start for Cursor Composer Users**:
Run `arc --plan "implement the new auth system"` to see the multi-agent graph build out the feature.
