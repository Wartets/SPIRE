# ADR-0005: Backend transport abstraction with runtime validation

- **Status:** Accepted
- **Date:** 2026-03-18

## Context

SPIRE must run in multiple environments (Tauri desktop, WASM/browser, mock/testing). Direct UI calls to transport-specific APIs increase coupling and break portability.

## Decision

Route all frontend backend calls through:

1. `BackendInterface`
2. Environment-specific implementations (`TauriBackend`, `WasmBackend`, `MockBackend`)
3. API façade functions (`$lib/api.ts`)
4. Runtime schema validation for IPC payloads in domain schemas

## Consequences

### Positive

- Transport-independent UI and easier testing.
- Better contract safety through runtime decoding/validation.
- Graceful fallback paths in non-Tauri execution contexts.

### Negative

- Additional abstraction layers to maintain.
- Slight overhead from schema validation.

### Follow-up

- Keep command naming consistent end-to-end.
- Require schema updates when IPC response structures evolve.
