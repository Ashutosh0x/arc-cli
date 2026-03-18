# Architecture Overview

ARC CLI is constructed as a modern, multi-crate Rust workspace designed for absolute maximum performance, safety, and strict observability. The system architecture isolates responsibilities across specialized crates, preventing dependency bloat and ensuring zero-cost abstractions where necessary.

## Crate Layout

### 1. arc-cli
The primary binary entry point.
- **Duties**: CLI argument parsing via `clap`, terminal output bootstrapping, structured tracing initialization, and interactive chat loop coordination.
- **Design Philosophy**: Minimal synchronous blocking. Delegates all heavy lifting to the core library crates.

### 2. arc-core
The unified backbone of the system.
- **Duties**: 
  - Central configuration and secret management natively linked to the OS cryptographic keychain.
  - LLM Session handling and database telemetry (`redb`, `hdrhistogram`).
  - Strict security perimeters spanning Prompt Guards, Context Isolation, and Lethal Trifecta calculations.
  - Multi-tier Memory Arena subsystem orchestrating Short-Term and Long-Term context windows via `compact_str` and `bumpalo`.

### 3. arc-providers
The high-speed networking and LLM integration layer.
- **Duties**: Specialized `struct` and `trait` implementations mapping tightly to OpenAI, Anthropic, Google Gemini, and Ollama specifications.
- **Design Philosophy**: Extreme speed. JSON payloads are parsed dynamically using zero-copy Server-Sent Events (SSE) bounded by SIMD accelerated buffers to avoid `String` instantiations during heavy streaming generation.

### 4. arc-router
The intelligent multi-model load balancer.
- **Duties**: Selects the optimal provider based on token budgets, requested capabilities (e.g., Vision, Tool Calling), and active availability. Supports `race_providers` to concurrently dispatch identical requests globally and seamlessly pivot to the fastest response.

### 5. arc-tools & arc-mcp
The execution and extensibility capabilities.
- **arc-tools**: Bound tightly into sandboxed environments (`arc-sandbox`). Responsible for executing sub-shells, validating diffs, and reading host file systems securely.
- **arc-mcp**: Implementation of the Model Context Protocol. Secures third-party plugins with SHA256 manifest pinning and context minimization.

### 6. arc-tui
The professional terminal rendering engine.
- **Duties**: Abstracting `crossterm` and async UI states. Features a highly efficient `StreamingSpinner` which logs token generation velocities real-time without interrupting the async executor.

## Concurrency Model

ARC CLI primarily runs under a `tokio` multi-threaded async executor. CPU-bound operations (such as token counting using `tiktoken-rs` and memory compression arrays) are either offloaded to `spawn_blocking` or handled via tight SIMD structures when synchronously executing inline. 

All HTTP invocations share a unified, persistent `reqwest::Client` mapped directly into a global `OnceLock`, radically improving HTTPS TLS handshake reuse over successive loops.