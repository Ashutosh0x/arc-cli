# Advanced Tooling & Code Intelligence

The final iterations of ARC CLI focus extensively on **Developer Ergonomics, High-Performance execution, and Advanced Code Intelligence**. We transformed the raw autonomous engine into a highly refined, daily-driver desktop application capable of operating entirely within a standard terminal workflow.

## UX, Distribution, and Trust
To minimize the setup overhead, we introduced seamless bootstrapping utilities:
1. **`arc init`**: Natively scans your repository's language and bootstraps optimal `ARC.md` context parameters alongside `.arc/hooks.toml`.
2. **Cost Hub (`arc --stats`)**: Developers can track exact token counts and financial costs bounded to current working sessions dynamically.
3. **Shell Completions**: Generates native `zsh/bash/fish` arguments bounding subcommands properly into the shell environment.
4. **Context Routing via `@`-mention**: Inside the CLI prompt, simply typing `@src/` triggers `rustyline` plugins to fuzzy-find local codebase states directly, mapping files to the prompt exactly like advanced UI IDEs.

## Deep Ergonomics & Extreme Performance
This phase introduced significant underlying mechanical upgrades mapping to memory speedups:
1. **Hardware Interrupt & REPL Flows**: `CancellationToken` integration cleanly traps `Ctrl+C` to cancel expensive LLM streaming generations natively. `Ctrl+D` traps EOF to trigger `redb` state commitments. `Tab` dynamically triggers `rustyline` completion plugins for fast `@src/` path routing.
2. **AST Scale-Out**: Upgraded the `arc-repomap` parser to `tree-sitter 0.23` bindings, unlocking TypeScript, Python, Golang, and C++ structural context extractions natively.
3. **Core Performance Allocators**: Explicitly swapped standard OS memory bounds for `mimalloc`, providing exceptionally low-latency heap abstractions, while using `rayon` to spawn threaded generation maps for massive monorepos.

## Code Intelligence & Subagents
To separate ARC from other CLI tools, we injected true CI-native workflows:
1. **`arc review` (PR Auto-Review)**: Operates a standalone local subagent capability. By pulling `git diff origin/main`, ARC packages uncommitted patch states and streams them to an Anthropic endpoint. The Agent evaluates vulnerabilities, architecture, and ergonomic flaws *before* you push to GitHub.
2. **Syntax-Highlighted Diff Previews & Multi-Key Routing**: Replaced naive `dialoguer` abstraction layers inside `arc-tools::file_edit` with a zero-latency `console::Term::read_key` loop using Atomic booleans to enforce thread bounds on Rust 1.85. Deployed interleaved `syntect` token extraction arrays onto the exact diff representations matching exactly how `bat` executes file views.
   - **Keyboard Ergonomics**: Natively bounds `Enter/y` (Accept), `Esc/n` (Reject), `a` (Accept All), `d` (Deny All), `e` (Open `$EDITOR`), and `j/k` (Vim Scroll) for absolute operator mastery over autonomous file modifications.
3. **Self-Updater (`arc update`)**: Integrated `self_update` hooks validating GitHub releases and updating your binary dynamically.

These updates formally mark ARC CLI as a definitive, production-ready framework providing extreme capability directly atop local system environments.
