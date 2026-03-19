# ADR-0002: Dynamic widget registry as canonical routing

- **Status:** Accepted
- **Date:** 2026-03-18

## Context

Widget rendering previously relied on repeated conditional chains in multiple containers (docking, canvas, tear-off). This caused drift when adding widget types and made feature rollout error-prone.

## Decision

Adopt a central registry (`$lib/core/registry/WidgetRegistry.ts`) containing:

- Widget labels (`WIDGET_LABELS`)
- Full components (`WIDGET_COMPONENTS`)
- Optional summary components (`WIDGET_SUMMARY_COMPONENTS`) for LOD

All renderers resolve components through shared helper functions instead of hardcoding branch chains.

## Consequences

### Positive

- Single source of truth for widget type metadata.
- Lower maintenance overhead and fewer missed integration points.
- Natural extension point for LOD summaries and unknown-widget fallback behavior.

### Negative

- Registry integrity becomes critical; broken imports affect multiple views.
- Requires disciplined updates whenever new widget types are introduced.

### Follow-up

- Keep registry lookups null-safe with explicit fallback behavior.
- Ensure widget guide and checklist require registry updates before merge.
