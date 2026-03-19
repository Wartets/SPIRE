# Widget Development Guide

This guide explains how to add a new SPIRE widget end-to-end with the current dynamic registry architecture.

## Scope

A production widget in SPIRE usually touches:

1. A Svelte component
2. Widget type metadata
3. Dynamic registry entries
4. All render surfaces (docking/canvas/tear-off)
5. Optional command palette action
6. Optional backend/API wiring if compute calls are needed

## Prerequisites

- Build passes (`npm run check` in `apps/spire-desktop`)
- Familiarity with `WidgetType` in `src/lib/stores/notebookStore.ts`
- Familiarity with registry layer in `src/lib/core/registry/WidgetRegistry.ts`

## Step-by-step: "Phase Space Visualizer" example

### 1) Create the widget component

Create `apps/spire-desktop/src/lib/components/PhaseSpaceVisualizer.svelte`.

Suggested structure:

- Header title and controls
- Inputs: masses / energy / sampling controls
- Action buttons: compute/export/reset
- Results panel with clear empty/error/running states

Component contract block should include:

- `@component`
- `@reads` and `@writes` for any stores used
- `@fires` if events are emitted

### 2) Add a new widget type

Edit `apps/spire-desktop/src/lib/stores/notebookStore.ts`:

- Extend `WidgetType` union with `"phase_space_visualizer"`
- Add `WIDGET_DEFINITIONS` entry with label and default size

Example fields to include:

- `type`
- `label`
- `defaultColSpan`
- `defaultRowSpan`

### 3) Register in dynamic registry

Edit `apps/spire-desktop/src/lib/core/registry/WidgetRegistry.ts`:

- Import `PhaseSpaceVisualizer`
- Add label in `WIDGET_LABELS`
- Add component in `WIDGET_COMPONENTS`
- Optionally add a compact summary in `WIDGET_SUMMARY_COMPONENTS`

This is the canonical registration point and should be treated as required.

### 4) Ensure all renderers can resolve it

If your renderers are registry-driven, this is automatic after step 3. If any legacy conditional route remains in your branch, update:

- `WidgetCell.svelte`
- `WidgetContainer.svelte`
- `InfiniteCanvas.svelte`
- `routes/window/+page.svelte`

### 5) Add command palette entry (optional but recommended)

In `apps/spire-desktop/src/routes/+page.svelte`, register a command such as:

- id: `spire.ui.add_phase_space_visualizer`
- category: `UI`
- execute: spawn widget in current mode

This keeps discoverability consistent with existing widgets.

### 6) Wire backend/API calls (if needed)

For new physics capability:

1. Add kernel function in `crates/spire-kernel`
2. Expose Tauri command in `apps/spire-desktop/src-tauri/src/main.rs`
3. Add method to `BackendInterface.ts`
4. Implement in `TauriBackend.ts` / `WasmBackend.ts` / `MockBackend.ts`
5. Export façade in `src/lib/api.ts`
6. Add runtime schema validation where applicable

Keep IPC stateless: pass model/reaction payloads by value.

## Interop and data-publishing recommendations

If the widget publishes data to other widgets, use `widgetInteropStore` with clear payload keys and version-safe defaults.

Recommended payload shape:

- Stable top-level keys
- Primitive + JSON-safe values only
- Explicit booleans for availability flags (`hasResult`, `isValid`)

## UX and quality checklist

Before merging, verify:

- [ ] Component has `@component` contract docs
- [ ] `WidgetType` + definitions updated
- [ ] Registry mappings updated
- [ ] Command palette add command present (or intentionally omitted)
- [ ] Context menu and tooltip behavior is consistent
- [ ] Works in docking, canvas, and tear-off
- [ ] Workspace save/load retains widget instances
- [ ] `npm run check` passes

## Common pitfalls

- Adding a widget type but forgetting registry entry
- Direct backend calls from UI components instead of `$lib/api.ts`
- Unclear payload contracts in inter-widget publish/subscribe
- Missing mock backend behavior causing tests/dev-mode failures

## Summary

Use the registry-first pattern: define type, register once, and let rendering resolve dynamically. Keep compute logic behind the backend abstraction and keep widget contracts documented for maintainable growth.