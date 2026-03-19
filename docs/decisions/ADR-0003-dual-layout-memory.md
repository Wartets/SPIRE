# ADR-0003: Dual layout memory for canvas and docking

- **Status:** Accepted
- **Date:** 2026-03-18

## Context

SPIRE supports both docking layout and infinite-canvas layout. A single mutable layout state made mode switches destructive and caused user context loss.

## Decision

Persist independent snapshots for both paradigms:

- `dockingState` for recursive split/tab topology
- `canvasState` for world-space widget placement
- Shared viewport/mode metadata

Mode switching restores the corresponding snapshot and only performs spatial inference fallback when required.

## Consequences

### Positive

- Non-destructive mode toggling.
- Better UX continuity for exploratory workflows.
- Cleaner persistence semantics in workspace serialization.

### Negative

- More complex state synchronization and migration paths.
- Slightly higher memory use due to dual snapshots.

### Follow-up

- Keep migration logic explicit and versioned.
- Validate both states during import/export and autosave recovery.
