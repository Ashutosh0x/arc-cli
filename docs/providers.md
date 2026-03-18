# Provider Framework Configuration

ARC CLI dynamically interfaces with industry-leading LLMs via the `arc-providers` crate. 

## Supported Providers
* **Anthropic** (`Claude 3.5 Sonnet`, `Opus`)
* **OpenAI** (`GPT-4o`, `o1-mini`)
* **Google Gemini** (`Gemini 1.5 Pro`)
* **Ollama** (Local execution: `llama3`, `mistral`)

All providers implement a unified `Provider` trait interface executing ultra-fast, zero-copy Server-Sent Events (SSE). 

## Provider Fallbacks
Providers are securely loaded via the OS-level Keychain. If an API key is heavily rate-limited, the `arc-router` automatically transitions to the next available provider listed within your `~/.arc/config.toml` priority queue.

## Streaming Architecture
Instead of buffering huge chunks of text or mapping directly heavily via `serde_json`, our custom `SseStream` loop intercepts raw byte chunks on the TCP socket using SIMD `memchr`, slicing the exact array boundaries for the data payload, and feeding it directly to the UI rendering loop. 
This operates under `async` constraints allowing token display speeds mapping entirely to your physical network bandwidth limits.