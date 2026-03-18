# Agent-to-Agent (A2A) Protocol

The ARC CLI comes with a robust, production-grade `arc-a2a` subsystem that allows agents to securely communicate, coordinate, and execute tasks across multiple nodes.

## Core Features

1. **Zero-Trust Security**: Every message is authenticated. Teams can use JWT Bearer tokens with short expirations, or HMAC-SHA256 signatures for zero-allocation symmetric key authentication.
2. **Server-Sent Events (SSE)**: We use custom SSE streaming to stream task progression in real-time. This eliminates the need for expensive polling and ensures low-latency task updates.
3. **Agent Discovery**: Agents host a `/.well-known/agent.json` route to advertise their capabilities ("Agent Cards"). Remote agents can dynamically discover and cache these capabilities.
4. **Resilient HTTP/2 Pooling**: Handled via `reqwest` connection pooling and exponential backoff retry algorithms to guarantee delivery across noisy networks.
5. **State Machine Integrity**: A strictly typed `TaskRegistry` validates every step of a task lifecycle concurrently.

## Walkthrough

- Connect to an agent using `client.discover(endpoint).await?`
- Submit a task via `client.submit_task()`.
- The server registers the task and dispatches a background worker.
- The server continually updates the client through a persistent SSE connection (`client.read_stream()`).

```rust
let client = A2AClient::new(config);
let card = client.discover("https://agent.example.com").await?;
println!("Agent {} possesses scales: {:?}", card.name, card.skills);
```
