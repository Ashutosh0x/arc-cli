# OWASP Top 10 for LLM Applications — ARC Compliance

The architecture of ARC CLI is designed to structurally mitigate risks categorized by the Open Web Application Security Project (OWASP) Top 10 for LLMs.

| Vulnerability | Status | Remediation/Implementation Strategy in ARC |
| --- | --- | --- |
| **LLM01: Prompt Injection** | Mitigated | `prompt_guard.rs` isolates user context via strict XML and markup delimiters. We also scan for "Jailbreak" and "Forget previous instructions" patterns continuously. |
| **LLM02: Insecure Output Handling** | Mitigated | `session_guard.rs` enforces multi-stage output scanning against regex patterns representing credentials, system exfiltration URIs, and dangerous commands before rendering to the user or shell. |
| **LLM03: Training Data Poisoning** | N/A | ARC interacts with commercially hardened models (OpenAI, Google) over API. We do not organically fine-tune models from user sessions using open-market datasets. |
| **LLM04: Model Denial of Service** | Mitigated | `CancellationToken` integration restricts unbounded streams. Strict token budgeting in the `Arena` limits memory allocations. Rate Limiters restrict the amount of API calls per session. |
| **LLM05: Supply Chain Vulnerabilities** | Mitigated | Automated `cargo-audit`, `cargo-deny`, and `cargo-vet` workflows block malicious crates. Binary tracing is injected via `cargo-auditable`. |
| **LLM06: Sensitive Information Disclosure** | Mitigated | Data guards intercept payload matching for secrets before hitting the LLM network request. Secrets management uses OS-level Keyring encryption natively. |
| **LLM07: Insecure Plugin Design** | Mitigated | MCP Server architecture includes Manifest Pinning (`verify_manifest_pin`) and Context Minimization (`minimize_context`) to enforce strict payload limits and deny untrusted plugins via `arc-mcp::security`. |
| **LLM08: Excessive Agency** | Mitigated | Operational mode requires manual `arc doctor` clearance and human-in-the-loop explicit approvals for bash code execution and file overriding within Sandboxes (`arc-sandbox`). |
| **LLM09: Overreliance** | Mitigated | ARC clearly displays uncertainty via `StreamingSpinner` phases, prompts for tests, and transparently logs provider traces (`tracing_opentelemetry`). |
| **LLM10: Model Theft** | Mitigated | System relies solely on secure authentication tokens and does not act as a hosted platform susceptible to arbitrary weight exfiltration. |
