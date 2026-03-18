# Authentication & Security Architecture

ARC CLI employs extreme measures to ensure that autonomous operations happen securely, protecting credentials both at rest and in transit. The authentication architecture fundamentally bridges 3 distinct areas:
1. Provider Authentication (LLMs).
2. Human-Delegated OAuth (Google, GitHub, external services).
3. Agent-to-Agent (A2A) Identity verification.

---

## 1. Provider Authentication

ARC integrates natively with deep LLM provider ecosystems (Anthropic, Gemini, OpenAI) utilizing standard static API keys or cloud-default credentials.

### Credential Protection at Rest
Traditional CLI tools store plain-text API keys in `~/.config/arc/config.toml` or `~/.bashrc`. ARC strictly forbids this by default.
Instead, ARC utilizes the native OS **Secure Enclave / Keyring** (via `keyring` in Rust):
- **macOS**: Keychain Services.
- **Linux**: Secret Service API / D-Bus (e.g., GNOME Keyring, KWallet).
- **Windows**: Windows Credential Manager.

```bash
# Safely prompt and insert the API key into the OS secure enclave natively
arc auth login --provider anthropic
```

---

## 2. Hardened OAuth2 with Refresh Token Rotation (RTR)

When ARC needs to act on your behalf to fetch code from a private GitHub repo or access a Google Drive document, it uses **Hardened OAuth2**:

1. **PKCE (Proof Key for Code Exchange)** guarantees ARC intercepts the authorization code securely preventing interception attacks locally.
2. **Refresh Token Rotation (RTR)**: Every time ARC mints a new access token, the identity provider strictly issues a *new* refresh token, completely invalidating the old one. If an attacker manages to exfiltrate an old refresh token, attempting to use it will instantly trigger a credential compromise alarm and revoke the entire token chain.
3. **Strict URI Matching Rules**: Enforces that redirect URIs natively match exact expected ports without wildcard vulnerabilities.

---

## 3. Agent-to-Agent (A2A) Security

When ARC spawns sub-agents or connects to remote cluster nodes over the A2A protocol (`arc-a2a`), the agents must verify their own identities autonomously without human intervention.

ARC supports two Zero-Trust paradigms for A2A routing:

### Symmetric Verification (HMAC-SHA256)
For extremely tight, latency-sensitive internal clusters:
- The sender hashes the entire `A2AMessage` payload with a shared secret `$A2A_SECRET` natively generating a 256-bit signature.
- The receiving node evaluates the payload integrity and cryptographic signature mathematically using `hmac` and `sha2`.
- Prevents Data Tampering, Man-In-The-Middle attacks, and replay injections.

### Asymmetric Verification (JWT Bearer)
For distributed networks or multi-tenant agent deployments:
- ARC generates natively signed short-lived JSON Web Tokens (JWT) for the delegating agent.
- Tokens expire tightly (e.g., 60 seconds) reducing the attack window if intercepted.
- Embedded scopes (`task:submit`, `task:query`) explicitly restrict permissions.
- Validated dynamically utilizing `jsonwebtoken` strict validation parameters (`aud`, `iss`).

---

## 4. Path Guards and Context Sandboxing

Even if physically authenticated, ARC limits the *blast radius* of an agent using Path Guards:
- Agents undergo **Context Minimization** before sending payloads to third-party MCP servers, mathematically stripping out any irrelevant project files to prevent accidental mass context exfiltration.
- The CLI enforces Sandboxing boundaries, terminating any read/write path resolution operations attempting to escape the git repository root via symlink traversals.

_For details on Prompt Injection defenses, see the [Architecture Document](architecture.md)._