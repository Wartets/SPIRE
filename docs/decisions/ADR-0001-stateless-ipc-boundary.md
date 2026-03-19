# ADR-0001: Stateless IPC boundary

- **Status:** Accepted
- **Date:** 2026-03-18

## Context

SPIRE spans three layers:

1. Rust kernel (`spire-kernel`) for physics and mathematics
2. Tauri backend bridge (`apps/spire-desktop/src-tauri/src/main.rs`)
3. Svelte frontend state layer (`apps/spire-desktop/src`)

Early coupling pressure appears whenever commands try to keep long-lived mutable kernel objects in backend-global state. That pattern complicates synchronization, testing, and WASM parity.

## Decision

Use a **stateless IPC contract**:

- Frontend owns application state and command sequencing.
- IPC commands pass domain values (for example `TheoreticalModel`, `Reaction`) by value.
- Tauri commands stay thin wrappers that call kernel functions and map `SpireError` to frontend-safe messages.
- Kernel modules remain the only place for physics logic.

## Consequences

### Positive

- Predictable command behavior and simpler reproducibility.
- Easier WASM/Tauri parity via shared backend interfaces.
- Lower risk of hidden mutable state across commands.

### Negative

- Larger payloads over IPC boundaries in some flows.
- Frontend must orchestrate multi-step pipelines explicitly.

### Follow-up

- Keep serialization schemas strict and validated at runtime.
- Avoid adding backend-global mutable model/reaction stores except for explicitly scoped services (for example, background job manager state).
