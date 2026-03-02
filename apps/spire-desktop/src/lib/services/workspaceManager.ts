/**
 * SPIRE — Workspace Manager Service
 *
 * Handles serialisation, deserialisation, validation, and persistence
 * of workspace state.  All operations are pure functions that read from
 * and write to the application's Svelte stores.
 *
 * Persistence targets:
 *   - **Auto-save**: `localStorage` key `spire_autosave` (debounced)
 *   - **Export**: `.spire.json` file downloaded via the Web File API
 *   - **Import**: `.spire.json` file parsed and hydrated into stores
 */

import { get } from "svelte/store";
import { widgets, GRID_COLUMNS, resetLayout } from "$lib/stores/notebookStore";
import type { WidgetInstance, WidgetType } from "$lib/stores/notebookStore";
import { activeFramework, appendLog } from "$lib/stores/physicsStore";
import {
  particlesTomlInput,
  verticesTomlInput,
  modelNameInput,
  initialIdsInput,
  finalIdsInput,
  cmsEnergyInput,
  maxLoopOrderInput,
  setAllInputs,
  resetAllInputs,
} from "$lib/stores/workspaceInputsStore";
import type {
  SpireWorkspace,
  SerializedWidget,
  WorkspaceValidationResult,
} from "$lib/types/workspace";
import { WORKSPACE_SCHEMA_VERSION } from "$lib/types/workspace";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** LocalStorage key for the auto-saved workspace. */
export const AUTOSAVE_KEY = "spire_autosave";

/** Known widget types for validation. */
const KNOWN_WIDGET_TYPES: ReadonlySet<string> = new Set<string>([
  "model",
  "reaction",
  "diagram",
  "amplitude",
  "kinematics",
  "dalitz",
  "log",
]);

// ---------------------------------------------------------------------------
// Export (Snapshot → JSON)
// ---------------------------------------------------------------------------

/**
 * Capture the current application state as a serialisable workspace.
 *
 * @param name - Optional workspace name (defaults to "Untitled Workspace").
 */
export function exportWorkspace(name?: string): SpireWorkspace {
  const now = new Date().toISOString();
  const currentWidgets = get(widgets);

  const serializedWidgets: SerializedWidget[] = currentWidgets.map((w) => ({
    type: w.type,
    col: w.col,
    row: w.row,
    colSpan: w.colSpan,
    rowSpan: w.rowSpan,
    data: { ...w.data },
  }));

  return {
    version: WORKSPACE_SCHEMA_VERSION,
    metadata: {
      name: name ?? "Untitled Workspace",
      createdAt: now,
      updatedAt: now,
    },
    layout: {
      widgets: serializedWidgets,
      gridColumns: GRID_COLUMNS,
    },
    physics: {
      framework: get(activeFramework),
      particlesToml: get(particlesTomlInput),
      verticesToml: get(verticesTomlInput),
      modelName: get(modelNameInput),
      initialStateIds: [...get(initialIdsInput)],
      finalStateIds: [...get(finalIdsInput)],
      cmsEnergy: get(cmsEnergyInput),
      maxLoopOrder: get(maxLoopOrderInput),
    },
  };
}

/**
 * Serialise the current workspace to a JSON string.
 */
export function exportWorkspaceJson(name?: string): string {
  return JSON.stringify(exportWorkspace(name), null, 2);
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/**
 * Validate an unknown JSON blob against the workspace schema.
 *
 * Returns a structured result with errors and warnings.
 * Unknown widget types are flagged as warnings (they will be filtered
 * during import rather than rejecting the entire workspace).
 */
export function validateWorkspace(data: unknown): WorkspaceValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (typeof data !== "object" || data === null) {
    errors.push("Workspace must be a JSON object.");
    return { valid: false, errors, warnings };
  }

  const obj = data as Record<string, unknown>;

  // Version
  if (!("version" in obj) || typeof obj.version !== "string") {
    errors.push("Missing or invalid 'version' field.");
  } else if (obj.version !== WORKSPACE_SCHEMA_VERSION) {
    errors.push(
      `Unsupported schema version "${obj.version}". ` +
        `Expected "${WORKSPACE_SCHEMA_VERSION}".`,
    );
  }

  // Metadata
  if (!("metadata" in obj) || typeof obj.metadata !== "object" || obj.metadata === null) {
    errors.push("Missing or invalid 'metadata' field.");
  } else {
    const meta = obj.metadata as Record<string, unknown>;
    if (typeof meta.name !== "string") {
      errors.push("metadata.name must be a string.");
    }
  }

  // Layout
  if (!("layout" in obj) || typeof obj.layout !== "object" || obj.layout === null) {
    errors.push("Missing or invalid 'layout' field.");
  } else {
    const layout = obj.layout as Record<string, unknown>;
    if (!Array.isArray(layout.widgets)) {
      errors.push("layout.widgets must be an array.");
    } else {
      for (let i = 0; i < layout.widgets.length; i++) {
        const w = layout.widgets[i] as Record<string, unknown>;
        if (typeof w.type !== "string") {
          errors.push(`layout.widgets[${i}].type must be a string.`);
        } else if (!KNOWN_WIDGET_TYPES.has(w.type)) {
          warnings.push(
            `layout.widgets[${i}].type "${w.type}" is unknown and will be skipped.`,
          );
        }
        if (typeof w.col !== "number" || typeof w.row !== "number") {
          errors.push(`layout.widgets[${i}] has invalid col/row.`);
        }
        if (typeof w.colSpan !== "number" || typeof w.rowSpan !== "number") {
          errors.push(`layout.widgets[${i}] has invalid colSpan/rowSpan.`);
        }
      }
    }
  }

  // Physics
  if (!("physics" in obj) || typeof obj.physics !== "object" || obj.physics === null) {
    errors.push("Missing or invalid 'physics' field.");
  } else {
    const phys = obj.physics as Record<string, unknown>;
    if (typeof phys.framework !== "string") {
      errors.push("physics.framework must be a string.");
    }
    if (typeof phys.particlesToml !== "string") {
      errors.push("physics.particlesToml must be a string.");
    }
    if (typeof phys.verticesToml !== "string") {
      errors.push("physics.verticesToml must be a string.");
    }
    if (typeof phys.modelName !== "string") {
      errors.push("physics.modelName must be a string.");
    }
    if (!Array.isArray(phys.initialStateIds)) {
      errors.push("physics.initialStateIds must be an array.");
    }
    if (!Array.isArray(phys.finalStateIds)) {
      errors.push("physics.finalStateIds must be an array.");
    }
    if (typeof phys.cmsEnergy !== "number") {
      errors.push("physics.cmsEnergy must be a number.");
    }
    if (typeof phys.maxLoopOrder !== "number") {
      errors.push("physics.maxLoopOrder must be a number.");
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

// ---------------------------------------------------------------------------
// Import (JSON → Stores)
// ---------------------------------------------------------------------------

/** Counter for regenerating unique widget IDs on import. */
let _importCounter = 0;

function makeImportId(): string {
  _importCounter += 1;
  return `w-i${_importCounter}-${Date.now().toString(36)}`;
}

/**
 * Import a workspace from a parsed JSON object.
 *
 * Validates the data, hydrates all stores, filters unknown widget types,
 * and logs a summary.  Does NOT trigger any computation — the user must
 * explicitly Load Model / Construct Reaction.
 *
 * @returns `true` if import succeeded, `false` if validation failed.
 */
export function importWorkspace(data: unknown): boolean {
  const validation = validateWorkspace(data);

  if (!validation.valid) {
    for (const err of validation.errors) {
      appendLog(`IMPORT ERROR: ${err}`);
    }
    return false;
  }

  for (const warn of validation.warnings) {
    appendLog(`IMPORT WARNING: ${warn}`);
  }

  const workspace = data as SpireWorkspace;

  // --- Hydrate physics inputs ---
  activeFramework.set(workspace.physics.framework);
  setAllInputs({
    particlesToml: workspace.physics.particlesToml,
    verticesToml: workspace.physics.verticesToml,
    modelName: workspace.physics.modelName,
    initialIds: workspace.physics.initialStateIds,
    finalIds: workspace.physics.finalStateIds,
    cmsEnergy: workspace.physics.cmsEnergy,
    maxLoopOrder: workspace.physics.maxLoopOrder,
  });

  // --- Hydrate layout (regenerate IDs, filter unknown types) ---
  const hydratedWidgets: WidgetInstance[] = workspace.layout.widgets
    .filter((w) => KNOWN_WIDGET_TYPES.has(w.type))
    .map((w) => ({
      id: makeImportId(),
      type: w.type as WidgetType,
      col: w.col,
      row: w.row,
      colSpan: w.colSpan,
      rowSpan: w.rowSpan,
      data: w.data ?? {},
    }));

  widgets.set(hydratedWidgets);

  appendLog(
    `Workspace "${workspace.metadata.name}" imported — ` +
      `${hydratedWidgets.length} widgets, ` +
      `framework: ${workspace.physics.framework}`,
  );

  return true;
}

/**
 * Import a workspace from a raw JSON string.
 *
 * @returns `true` if import succeeded, `false` on parse or validation failure.
 */
export function importWorkspaceJson(json: string): boolean {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    appendLog("IMPORT ERROR: Invalid JSON.");
    return false;
  }
  return importWorkspace(parsed);
}

// ---------------------------------------------------------------------------
// Auto-Save (LocalStorage)
// ---------------------------------------------------------------------------

/**
 * Write the current workspace to localStorage.
 */
export function autoSave(): void {
  try {
    const json = exportWorkspaceJson("Auto-Save");
    localStorage.setItem(AUTOSAVE_KEY, json);
  } catch {
    // Silently ignore quota errors
  }
}

/**
 * Attempt to restore the workspace from the auto-save slot.
 *
 * @returns `true` if a valid auto-save was found and restored.
 */
export function autoRestore(): boolean {
  try {
    const json = localStorage.getItem(AUTOSAVE_KEY);
    if (!json) return false;
    return importWorkspaceJson(json);
  } catch {
    return false;
  }
}

/**
 * Check whether an auto-save exists in localStorage.
 */
export function hasAutoSave(): boolean {
  return localStorage.getItem(AUTOSAVE_KEY) !== null;
}

/**
 * Clear the auto-save from localStorage.
 */
export function clearAutoSave(): void {
  localStorage.removeItem(AUTOSAVE_KEY);
}

// ---------------------------------------------------------------------------
// File I/O Helpers (Web File API)
// ---------------------------------------------------------------------------

/**
 * Download the current workspace as a `.spire.json` file.
 */
export function downloadWorkspace(name?: string): void {
  const fileName = (name ?? "workspace").replace(/[^a-zA-Z0-9_-]/g, "_");
  const json = exportWorkspaceJson(name);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);

  const a = document.createElement("a");
  a.href = url;
  a.download = `${fileName}.spire.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);

  appendLog(`Workspace "${name ?? "workspace"}" exported to file.`);
}

/**
 * Read a `.spire.json` file from a File input and import it.
 *
 * @returns Promise that resolves to `true` if import succeeded.
 */
export async function importFromFile(file: File): Promise<boolean> {
  try {
    const text = await file.text();
    return importWorkspaceJson(text);
  } catch {
    appendLog("IMPORT ERROR: Could not read file.");
    return false;
  }
}

// ---------------------------------------------------------------------------
// Full Reset
// ---------------------------------------------------------------------------

/**
 * Reset the entire workspace to factory defaults.
 *
 * Clears physics inputs, resets the widget layout, clears auto-save,
 * and resets the framework to StandardModel.
 */
export function resetWorkspace(): void {
  resetAllInputs();
  resetLayout();
  activeFramework.set("StandardModel");
  clearAutoSave();
  appendLog("Workspace reset to defaults.");
}

// ---------------------------------------------------------------------------
// Debounce Utility
// ---------------------------------------------------------------------------

/**
 * Create a debounced version of a function.
 *
 * @param fn - The function to debounce.
 * @param ms - Delay in milliseconds.
 */
export function debounce<T extends (...args: unknown[]) => void>(
  fn: T,
  ms: number,
): (...args: Parameters<T>) => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return (...args: Parameters<T>) => {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => {
      fn(...args);
      timer = null;
    }, ms);
  };
}
