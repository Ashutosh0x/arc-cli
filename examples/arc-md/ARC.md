# ARC Instructions
# This file serves as the system prompt layer for this specific project.
# The ARC CLI agents explicitly read this file on boot and inject it into the context window.

## Code Conventions
- Use async/await throughout the IO layers.
- Prefer `anyhow::Result` for application-level error handling.
- We strictly adhere to `rustfmt` standard style.
- NO `unwrap()` or `expect()`. Use proper `?` bubbling.

## Architecture Guidelines
- The `arc-core` crate should know nothing about specific LLM providers.
- The `arc-providers` crate handles all HTTP streaming logic.
- Data structures passed between crates must be Send + Sync.

## Build Rules
- Do not run `cargo clean` arbitrarily.
- Ensure all CI tests pass (`cargo test --workspace`) before requesting user approval.
