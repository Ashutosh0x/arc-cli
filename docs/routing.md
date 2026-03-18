# The ARC Routing Engine

The `arc-router` crate acts as the central brain dictating how to communicate with the provider framework.

## Parallel Probing

When executing a complex command via `arc ask`, precision and speed are invaluable. 
When configured, the router utilizes `futures::future::select_ok` to initiate parallel connections.

1. The Prompt and Sandbox Context are compressed and serialized.
2. Identical requests fire over HTTP/2 simultaneously to Anthropic Claude, OpenAI GPT-4, and Google Gemini.
3. The absolute fastest Time-To-First-Token (TTFT) stream naturally returns first.
4. The router immediately bridges this open stream back to your shell output, seamlessly terminating the sibling requests to prevent billing overflow.

## Model Selection Engine
The router calculates requested user features (e.g. Vision support, 128k+ Context requirements) mapped against a static registry of models. If a user asks to scan an image, the Router natively upgrades the target model (e.g. from `gpt-4o-mini` to `gpt-4o` or `gemini-1.5-pro`) dynamically before firing.