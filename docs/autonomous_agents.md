# ARC CLI - Autonomous Agents Overview

With the autonomous agent architecture complete, the ARC CLI has finalized the last 28 advanced agentic features. This document natively details the 8 new subsystem crates integrated into the workspace and how they plug into the unified agent execution loops.

## The Eight Subsystems Integrated:

1. **`[arc-voice]`**: 
   Introduces local high-fidelity Push-to-Talk (PTT) input natively mapped over `crossterm`. It streams `cpal` hardware buffers straight into a `wav` formatter, piping it over to Whisper/Deepgram STT APIs to transform operator voice commands into `String` payloads directly into the agent.

2. **`[arc-compact]`**: 
   Zero-cost context length management. Implements a streaming token analyzer using `tiktoken-rs` inside a `SlidingWindow` buffer. Older memories are purged gracefully or lossily compressed via LLM fast-summarization calls without dropping `System` prompts.

3. **`[arc-skills]`**: 
   The bedrock of dynamic tool use. Provides an `Arc`-shared `SkillRegistry` mapped over `DashMap`. Agents can eagerly fetch and execute isolated, asynchronous traits conforming to the `Skill` boundary dynamically without blocking core event loops.

4. **`[arc-search]`**: 
   Injects factual grounding via `scraper` and `reqwest`. Supports zero-setup `DuckDuckGo` semantic scraping. Agents use this asynchronously to read factual information before mutating files, preventing destructive hallucination errors.

5. **`[arc-vision]`**: 
   Binds binary `image` and `base64` decoders bridging into multimodal AI provider calls. Currently staged to permit layout understanding, diagram translations, and UI testing loops natively.

6. **`[arc-sandbox]`**: 
   A strict security moat written around `landlock` modules in Linux. Isolates execution contexts ensuring unauthorized binary writes cannot escape out into the OS boundaries, cementing ARC as a safe platform for remote execution routines.

7. **`[arc-loop]`**: 
   The heart of chronic tasks. Watches the workspace memory via `notify` polling and triggers asynchronous `Cron`-style callbacks, executing continuous evaluation sweeps without waking up heavy processes recursively.

8. **`[arc-cloud]`**: 
   Establishes async boundary delegation payloads representing `SQS`/`Kafka` integration stubs for heavy cloud-oriented batch computing steps when the local rig capacity drains.

Together, these represent a complete, production-grade leap from a chat REPL to a 100% unsupervised AI Terminal. 
