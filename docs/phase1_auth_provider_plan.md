# Phase 1: Authentication & Provider Setup — Implementation Plan

## Repository structure investigation (current state)

The workspace is already organized into focused crates that align with the requested roadmap:

- `crates/arc-core`: shared domain logic (`config`, `credentials`, `models`, `model_picker`, `auth`, `error`, security, telemetry).
- `crates/arc-providers`: provider abstractions and transport/runtime implementation.
- `crates/arc-cli`: command surface and user flows (`auth`, setup, doctor, diagnostics, repl).
- Supporting crates for routing, policy, hooks, session/worktree orchestration, sandboxing, and UI.

The first-phase modules requested in this ticket already exist in `arc-core`:

- `src/config.rs`
- `src/credentials.rs`
- `src/models.rs`
- `src/model_picker.rs`
- `src/auth/oauth_google.rs`
- `src/auth/api_key.rs`
- `src/auth/mod.rs`
- `src/error.rs`

## Gap assessment for requested Phase 1 hard requirements

1. **Workspace resolver**
   - Required: `resolver = "3"`
   - Current: `resolver = "2"` in root `Cargo.toml`.

2. **Edition**
   - Required: `edition = "2024"`
   - Current: already set via `[workspace.package]`.

3. **Strict Clippy lints**
   - Required: deny `unwrap_used`, `expect_used`, `dbg_macro`
   - Current: already configured under `[workspace.lints.clippy]`.

4. **rustfmt policy**
   - Required: enforced formatting rules
   - Current: `rustfmt.toml` exists with explicit rules.

5. **reqwest TLS policy**
   - Required: rustls-only
   - Current: `reqwest` includes `rustls-tls` but does not disable default features, so native-tls can still be pulled transitively.

## Phase 1 implementation sequence

1. Update workspace metadata:
   - set `resolver = "3"` in root `Cargo.toml`.

2. Enforce rustls-only HTTP client baseline:
   - set `default-features = false` for workspace `reqwest` dependency,
   - keep required features (`json`, `http2`, `rustls-tls`, `stream`, `brotli`, `gzip`, `multipart`).

3. Validate build health after policy changes:
   - `cargo check -p arc-core`
   - `cargo check -p arc-cli`

4. Phase 1 execution checkpoints (follow-up implementation PRs):
   - Auth subcommands and setup wizard UX validation in `arc-cli`.
   - Provider capability schema harmonization in `arc-providers`.
   - Model discovery + picker end-to-end wiring test coverage.
   - Centralized error taxonomy consistency checks across crates.

## Approval gate

Please approve this plan before proceeding with the full multi-phase implementation backlog.

Suggested approval text:

> Approved: proceed with Phase 1 implementation (auth/provider setup) and then advance phase-by-phase with checkpoints.
