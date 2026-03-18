# ARC CLI: Advanced Usage Guide

Once installed and configured, ARC acts as an embedded team of software engineers directly adjacent to your code. 

## 1. The Autonomous REPL Loop

The standard execution method is engaging the persistent Chat REPL. 

```bash
arc chat
```

### REPL Commands
Inside the `arc chat` context, you can type natural language (e.g., "Extract this function into a macro") or use slash commands to leverage specific Agent functions:

- `/plan [prompt]` - Deploys the Read-Only `Planner` agent to draft an `implementation_plan.md` artifact without executing any code.
- `/checkpoint` - Instantly snapshots your git status, `SessionState` DB, and LLM conversations in an atomic token block.
- `/rewind [id]` - Visually scrubs back your memory tape if the model starts hallucinating, restoring exact physical workstation file hashes.
- `/voice` - Invokes the native WebSocket audio link for real-time speech-to-text driving the REPL. 
- `/stats` - Prints telemetry matrices about your session (Dollar cost, Token counts, Median latency).

## 2. Headless Git Lifecycle Hooks

ARC thrives completely asynchronously. You can register ARC to automatically peer-review changes directly when you commit software to git.

```bash
arc hooks install pre-commit
```
Whenever you type `git commit`, the `arc-policy` and `arc-agents` constraints engine will boot inside the shadow-workspace, generate an audit, and output potential fatal flaws straight to stdout before allowing the commit.

## 3. High-Security Sandboxing

When telling ARC to build experimental complex refactors or install dependencies, enforce the shadow space:

```bash
arc sandbox "Implement a native web-server using axum"
```
This forces ARC to write all files into `.arc-shadow/{uuid}`. It will use its `Tester` agent to compile and verify inside the sandbox. Only upon total success does ARC request a manifest transplant back into your primary workspace.

## 4. Worktree Multi-Agent Mode

For exceptionally large scopes:

```bash
arc orchestrate "Migrate parsing logic from serde to rkyv"
```

1. ARC allocates three separate `git worktrees`.
2. The Orchestrator boots the `Coder` agent in physical directory ONE. 
3. The Orchestrator boots the `Reviewer` agent in directory TWO, providing A2A (Agent-to-Agent) async feedback.
4. When finished, it merges the clean worktree onto your master branch.
