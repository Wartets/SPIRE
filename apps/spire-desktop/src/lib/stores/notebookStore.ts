/**
 * SPIRE - Notebook Store
 *
 * Manages the dynamic workbench layout.  Each "cell" in the notebook
 * is a WidgetInstance placed on a CSS-Grid canvas.  The store exposes
 * actions to add, remove, reposition, resize, and update per-widget
 * data.  This decouples individual widget state from the global
 * physics store so that, e.g., two DiagramWidgets can display
 * different diagrams side-by-side.
 */

import { writable, derived, get } from "svelte/store";

// ---------------------------------------------------------------------------
// Widget Type Enum
// ---------------------------------------------------------------------------

/** Exhaustive list of spawnable widget types. */
export type WidgetType =
  | "model"
  | "reaction"
  | "diagram"
  | "amplitude"
  | "kinematics"
  | "dalitz"
  | "analysis"
  | "event_display"
  | "diagram_editor"
  | "lagrangian_workbench"
  | "external_models"
  | "compute_grid"
  | "references"
  | "telemetry"
  | "log"
  | "notebook"
  | "parameter_scanner"
  | "decay_calculator"
  | "cosmology"
  | "flavor_workbench"
  | "plugin_manager"
  | "global_fit_dashboard";

// ---------------------------------------------------------------------------
// Widget Instance Model
// ---------------------------------------------------------------------------

export interface WidgetInstance {
  /** Unique identifier for this widget instance. */
  id: string;
  /** Which component this widget renders. */
  type: WidgetType;
  /** Grid column start (1-indexed). */
  col: number;
  /** Grid row start (1-indexed). */
  row: number;
  /** Width in grid columns. */
  colSpan: number;
  /** Height in grid rows. */
  rowSpan: number;
  /** Per-instance data that the widget can read/write. */
  data: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Widget Definitions (metadata for the Toolbox)
// ---------------------------------------------------------------------------

export interface WidgetDefinition {
  type: WidgetType;
  label: string;
  /** Default width in grid columns when spawned. */
  defaultColSpan: number;
  /** Default height in grid rows when spawned. */
  defaultRowSpan: number;
}

export const WIDGET_DEFINITIONS: WidgetDefinition[] = [
  { type: "model",      label: "Model Loader",        defaultColSpan: 2, defaultRowSpan: 2 },
  { type: "reaction",   label: "Reaction Workspace",  defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "diagram",    label: "Diagram Visualizer",  defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "amplitude",  label: "Amplitude Panel",     defaultColSpan: 2, defaultRowSpan: 2 },
  { type: "kinematics", label: "Kinematics",          defaultColSpan: 2, defaultRowSpan: 2 },
  { type: "dalitz",     label: "Dalitz Plot",         defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "analysis",       label: "Analysis",             defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "event_display", label: "Event Display",        defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "lagrangian_workbench", label: "Lagrangian Workbench", defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "external_models",      label: "External Models",       defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "compute_grid",          label: "Compute Grid",          defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "references",             label: "References",             defaultColSpan: 2, defaultRowSpan: 2 },
  { type: "telemetry",              label: "Telemetry",              defaultColSpan: 2, defaultRowSpan: 2 },
  { type: "log",            label: "Console",             defaultColSpan: 4, defaultRowSpan: 1 },
  { type: "notebook",        label: "Notebook",            defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "parameter_scanner", label: "Parameter Scanner",  defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "decay_calculator",  label: "Decay Calculator",   defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "cosmology",          label: "Cosmology",           defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "flavor_workbench",    label: "Flavor Workbench",     defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "plugin_manager",      label: "Plugin Manager",       defaultColSpan: 2, defaultRowSpan: 3 },
  { type: "global_fit_dashboard", label: "Global Fit Dashboard", defaultColSpan: 2, defaultRowSpan: 3 },
];

// ---------------------------------------------------------------------------
// Default Layout
// ---------------------------------------------------------------------------

/** The initial workspace layout shown on first load. */
function createDefaultLayout(): WidgetInstance[] {
  return [
    { id: makeId(), type: "model",      col: 1, row: 1, colSpan: 1, rowSpan: 2, data: {} },
    { id: makeId(), type: "diagram",    col: 2, row: 1, colSpan: 2, rowSpan: 2, data: {} },
    { id: makeId(), type: "amplitude",  col: 4, row: 1, colSpan: 1, rowSpan: 1, data: {} },
    { id: makeId(), type: "kinematics", col: 4, row: 2, colSpan: 1, rowSpan: 1, data: {} },
    { id: makeId(), type: "reaction",   col: 1, row: 3, colSpan: 1, rowSpan: 1, data: {} },
    { id: makeId(), type: "log",        col: 1, row: 4, colSpan: 4, rowSpan: 1, data: {} },
  ];
}

// ---------------------------------------------------------------------------
// ID Generation
// ---------------------------------------------------------------------------

let _counter = 0;

function makeId(): string {
  _counter += 1;
  return `w-${_counter}-${Date.now().toString(36)}`;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const widgets = writable<WidgetInstance[]>(createDefaultLayout());

/** Number of grid columns in the canvas. */
export const GRID_COLUMNS = 4;

// ---------------------------------------------------------------------------
// Derived Helpers
// ---------------------------------------------------------------------------

/** Total number of active widgets. */
export const widgetCount = derived(widgets, ($w) => $w.length);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/**
 * Spawn a new widget of the given type.  It is placed at the
 * bottom of the grid (appending a new row).
 */
export function addWidget(type: WidgetType): void {
  const def = WIDGET_DEFINITIONS.find((d) => d.type === type);
  if (!def) return;

  const current = get(widgets);

  // Find the next available row (below all existing widgets).
  const maxRow = current.reduce(
    (acc, w) => Math.max(acc, w.row + w.rowSpan),
    1,
  );

  const instance: WidgetInstance = {
    id: makeId(),
    type,
    col: 1,
    row: maxRow,
    colSpan: Math.min(def.defaultColSpan, GRID_COLUMNS),
    rowSpan: def.defaultRowSpan,
    data: {},
  };

  widgets.update((prev) => [...prev, instance]);
}

/** Remove a widget by its unique ID. */
export function removeWidget(id: string): void {
  widgets.update((prev) => prev.filter((w) => w.id !== id));
}

/** Update the grid position and size of a widget. */
export function updateWidgetLayout(
  id: string,
  layout: Partial<Pick<WidgetInstance, "col" | "row" | "colSpan" | "rowSpan">>,
): void {
  widgets.update((prev) =>
    prev.map((w) => (w.id === id ? { ...w, ...layout } : w)),
  );
}

/** Merge per-instance data into a widget's `data` bag. */
export function updateWidgetData(
  id: string,
  patch: Record<string, unknown>,
): void {
  widgets.update((prev) =>
    prev.map((w) =>
      w.id === id ? { ...w, data: { ...w.data, ...patch } } : w,
    ),
  );
}

/** Reset the layout to the default arrangement. */
export function resetLayout(): void {
  widgets.set(createDefaultLayout());
}
