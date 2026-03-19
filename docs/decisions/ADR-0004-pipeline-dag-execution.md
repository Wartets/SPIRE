# ADR-0004: DAG-based pipeline execution with stale propagation

- **Status:** Accepted
- **Date:** 2026-03-18

## Context

The node-based automation graph must execute deterministically, support partial recomputation, and provide inspectable edge payloads.

## Decision

Implement execution as a DAG runtime:

- Topological scheduling with cycle detection.
- Node/edge runtime states (`idle`, `running`, `completed`, `error`, `stale`).
- Stale propagation from edited nodes to downstream descendants.
- Caches for node outputs and edge payloads to enable incremental reruns.

## Consequences

### Positive

- Deterministic execution order and clearer diagnostics.
- Efficient recomputation for local edits.
- Better debuggability via wire payload inspection.

### Negative

- Additional runtime state complexity.
- Strict cycle rejection may surprise users expecting implicit feedback loops.

### Follow-up

- Provide clearer cycle diagnostics in inspector UI.
- Consider explicit loop-node semantics in a future ADR rather than implicit cyclic graphs.
