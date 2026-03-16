# ADR 0001: REST API runtime, JSON model, and lifecycle

- Status: Accepted
- Date: 2026-03-14

## Context

`windows-mtr` is currently a CLI-first diagnostics tool (`mtr`) with stable command behavior and script compatibility expectations. We want to add a REST API surface for automation and integration use cases without regressing existing CLI workflows.

The API stack should:

- Reuse Rust ecosystem components with strong production maturity.
- Preserve current CLI compatibility by default.
- Keep dependency expansion minimal and auditable.
- Support explicit, predictable shutdown semantics so in-flight probes are handled cleanly.

## Decision

### 1) Runtime and framework

Use **Tokio + Axum** for the v1 REST API server:

- Runtime: `tokio` (multi-thread runtime + signal handling)
- HTTP framework: `axum`

### 2) JSON serialization approach

Use `serde` for DTO derives and `axum::Json<T>` for request/response binding.

`serde_json` remains the underlying JSON codec and can also be used directly when constructing explicit JSON payloads.

### 3) Graceful shutdown model

Use a **signal-driven graceful shutdown** model with Tokio:

- Listen for `Ctrl+C` via `tokio::signal::ctrl_c()`.
- Trigger Axum server shutdown through `with_graceful_shutdown(...)`.
- Stop accepting new connections after shutdown is triggered.
- Allow in-flight request handlers to complete before process exit.

For v1, no custom background job draining protocol is introduced beyond request completion.

### 4) Process model / binary layout

Run the API inside the existing **`mtr` binary** behind explicit CLI flags:

- Keep probe CLI mode as the default startup path.
- Enable REST API serving only when `--api` is passed.
- Allow bind override via `--api-bind <ADDR>` while preserving localhost defaults and security validation.

This preserves CLI compatibility by making API behavior explicit and opt-in while avoiding a second deployable executable.

## Minimal dependency set in `Cargo.toml`

The API stack introduces only these new crates:

1. `tokio` with `rt-multi-thread`, `macros`, `signal`, `net`
   - Why: async runtime, entrypoint macro, signal handling, and socket server support.
   - Security/perf rationale: mature runtime with broad ecosystem scrutiny; efficient async I/O scheduling for many concurrent lightweight requests.
2. `axum`
   - Why: routing, extraction, response handling, and built-in graceful shutdown integration.
   - Security/perf rationale: strongly typed extractors reduce input handling errors; built on Hyper/Tower with efficient request pipeline composition.
3. `serde` with `derive`
   - Why: explicit schema structs for request/response serialization.
   - Security/perf rationale: typed schema boundaries reduce ad-hoc parsing bugs; derive-based serialization avoids bespoke codec logic and unnecessary allocations from manual map construction.

No additional persistence, auth, or distributed systems crates are added in v1 to keep the trusted dependency base minimal.

## Consequences

### Positive

- Preserves existing CLI behavior and invocation contracts in default mode.
- Uses common, well-maintained Rust async/web stack.
- Keeps implementation straightforward for future API endpoints.
- Provides deterministic server lifecycle behavior on shutdown.
- Avoids maintaining a separate API-only binary artifact.

### Trade-offs

- CLI now contains an additional explicit startup mode (`--api`), which must remain clearly documented.
- Initial API implementation remains intentionally narrow and does not solve multi-node/stateful orchestration concerns.

## Explicit non-goals for v1 API

- No long-term persistence/database storage.
- No distributed coordination or leader election.
- No cross-process/shared-state clustering.
- No websocket/streaming protocol commitment (REST only for v1).
- No implicit changes to existing CLI defaults or output formats.
