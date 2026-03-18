# ARC CLI - Phase 8: Core Subsystem Implementation

This phase implements specific infrastructure necessary to index codebases, manage context token limits, and integrate the CLI into automated CI/CD workflows.

## Subsystems Implemented:

### 1. Context File Loader (`ARC.md`)
The CLI parses an `ARC.md` (or `.arc.md`) file at the workspace root during initialization. The text is appended to the system prompt. This ensures project-specific architectural rules and library constraints are included in the LLM context window prior to execution.

### 2. Prompt Caching Headers
Anthropic `ephemeral` cache control headers were added to the `arc-providers` framework. This explicitly targets repetitive system instructions and static codebase states (like the `ARC.md` contents and repository maps) to reduce input token billing on extended sessions.

### 3. CI Benchmark Automation
Added `.github/workflows/benchmarks.yml`. This workflow runs `cargo bench` on the `main` branch to track parsing allocation metrics over time using Criterion.

### 4. Tree-sitter AST Repomap (`arc-repomap`)
Implemented a codebase mapping tool using `tree-sitter` and `tree-sitter-rust`. It recursively parses `.rs` files to extract `fn`, `struct`, `impl`, and `trait` signatures while omitting function bodies. Integration tests on `arc-repomap` itself confirm this structural extraction reduces prompt token volume by approximately 80% compared to naïve file inclusion.

### 5. Configurable Execution Hooks (`arc-hooks`)
Added `.arc/hooks.toml` parsing to configure `pre_commit`, `post_edit`, and `on_success` commands. This allows the CLI to execute standard formatters (e.g. `cargo fmt`) or linters immediately after modifying source files.

### 6. Headless Output Mode
Added `--headless` and `--output-format json` command line arguments. This bypasses the interactive TUI layer and emits structured JSON to standard output for use in automated deployment scripts.
