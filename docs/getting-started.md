# Starting with Developer Workflows

ARC isn't just an experimental REPL; it acts as a natively integrated toolchain designed heavily for daily production workflow constraints.

### 1. Initialize
Run the configuration Wizard at any point sequentially to re-index API connections and set your global models.
```bash
arc setup
```

### 2. Single-Shot Agents
To cleanly execute a single refactor autonomously without trapping your active terminal sessions:
```bash
arc run "Refactor src/logging to use tracing instead of standard stdout"
```
The agent orchestrates natively inside `.arc-shadow` and actively returns control when verified.

### 3. Persistent Workspace Monitoring
To hook an Auditor actively against changes happening natively in real-time, leverage `arc-loop` mechanisms:
```bash
arc loop start --target src/
```
The agent actively monitors `.git` and triggers security lint analysis in parallel automatically returning terminal notifications whenever you push broken changes.

### 4. Direct Git Integrations
ARC can directly bind logic specifically into pre-commit and post-merge hooks enforcing enterprise bounds:
```bash
arc hooks install pre-commit
```
Before a commit binds to disk, ARC policy engines assess API key leakage, massive context structural damage, and specific style guidelines outlined cleanly inside your `ARC.md` constraint bounds.