# ARC CLI - Frequently Asked Questions

## General
**Q: How is ARC different from Aider or Cursor?**
A: Cursor is an IDE fork. Aider is an excellent terminal chat tool. ARC is a multi-agent orchestration framework. It runs headless background agents (Planner, Architect, Coder), hooks directly into your Git lifecycle, enforces structural diffs, and can route between completely different providers automatically if one goes down.

## Providers & Models
**Q: Does ARC send my code to the cloud?**
A: Only the specific files identified as relevant to your prompt are sent to the LLM you've configured. If privacy is paramount, you can configure ARC to use `ollama` and process everything 100% locally on your machine.

**Q: Which models are best for ARC?**
A:
- **Claude 3.5 Sonnet**: The undisputed champion for the "Coder" agent.
- **OpenAI o1 / o3-mini**: Outstanding for the "Planner" and "Architect" agents analyzing deep logic.
- **gpt-4o-mini**: Great for rapid, cheap file-summarization.

## Execution & Safety
**Q: Can ARC break my code?**
A: ARC operates by default in `--mode ask`. Before single-handedly applying changes, it will show you a structured `git diff` and ask for approval. You can also run it in a `shadow workspace` which validates compilation before merging back to your main tree.

**Q: Where are my API keys stored?**
A: ARC never writes your API keys to plaintext config files. They are securely injected into your operating system's native keychain (Keychain on Mac, Secret Service on Linux, Credential Manager on Windows) using the `keyring` crate.
