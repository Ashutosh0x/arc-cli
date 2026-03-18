# Multi-Model Provider Hub

The `arc-providers` crate maintains a dynamically connected asynchronous framework binding models securely in real-time.

### Supported Frameworks
- **Anthropic Claude**: Full `xml` tags and native Tool-Use endpoints utilizing strictly un-polyfilled endpoints.
- **Google Gemini**: Integrates specifically utilizing `system_instructions` alongside immense context token budgets scaling massively across 1M+ buffers dynamically mapped into raw JSON payloads.
- **OpenAI**: Legacy universal compatibility.
- **Ollama**: Natively parses standard base endpoints mapped locally against `127.0.0.1:11434` enabling un-censored model interactions permanently detached from internet protocols securely.

### High-Performance Networking Pipeline
Throughputs natively handle immense volumes bypassing standard connection lag:
- Instantiated utilizing a persistent explicit `reqwest::Client` global configuration block. 
- Mapped forcibly against HTTP/2 (`http2_prior_knowledge = true`) avoiding TCP handshake bloat completely mapping across massive continuous multi-turn LLM reasoning evaluations. 
- Evaluates utilizing `brotli` payload streams directly slashing network byte transmissions efficiently.