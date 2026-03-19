# Widget Development

This page mirrors the in-repo guide at `docs/guide/widget-development.md`.

## Core workflow

1. Build widget component
2. Add `WidgetType` and default definition
3. Register in `WidgetRegistry`
4. Expose command-palette add action (recommended)
5. Wire backend abstraction if compute is required
6. Validate in docking/canvas/tear-off

For the full end-to-end walkthrough (including the "Phase Space Visualizer" example), refer to:

- `docs/guide/widget-development.md`
