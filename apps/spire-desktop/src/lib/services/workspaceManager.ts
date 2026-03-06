/**
 * SPIRE - Workspace Manager Service
 *
 * Handles serialisation, deserialisation, validation, and persistence
 * of workspace state.  All operations are pure functions that read from
 * and write to the application's Svelte stores.
 *
 * Persistence targets:
 *   - **Auto-save**: `localStorage` key `spire_autosave` (debounced)
 *   - **Export**: `.spire.json` file downloaded via the Web File API
 *   - **Import**: `.spire.json` file parsed and hydrated into stores
 *
 * v2.0 - Layout serialised as a recursive LayoutNode tree + canvas items.
 * v1.0 - Legacy flat CSS grid (auto-migrated on import).
 */

import { get } from "svelte/store";
import type { WidgetType } from "$lib/stores/notebookStore";
import {
  layoutRoot,
  canvasItems,
  setLayoutRoot,
  resetDockingLayout,
  clearCanvas,
  getDefaultLayout,
} from "$lib/stores/layoutStore";
import type { LayoutNode, CanvasItem } from "$lib/stores/layoutStore";
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
  "analysis",
  "event_display",
  "lagrangian_workbench",
  "external_models",
  "compute_grid",
  "diagram_editor",
  "references",
  "scripting",
  "telemetry",
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

  return {
    version: WORKSPACE_SCHEMA_VERSION,
    metadata: {
      name: name ?? "Untitled Workspace",
      createdAt: now,
      updatedAt: now,
    },
    layout: {
      layoutTree: JSON.parse(JSON.stringify(get(layoutRoot))) as LayoutNode,
      canvasItems: JSON.parse(JSON.stringify(get(canvasItems))) as CanvasItem[],
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
 * Accepts both v1.0 (flat grid) and v2.0 (layout tree) formats.
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
  } else if (obj.version !== "1.0" && obj.version !== "2.0") {
    errors.push(
      `Unsupported schema version "${obj.version}". ` +
        `Expected "1.0" or "2.0".`,
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

  // Layout - accept either v1.0 (widgets array) or v2.0 (layoutTree)
  if (!("layout" in obj) || typeof obj.layout !== "object" || obj.layout === null) {
    errors.push("Missing or invalid 'layout' field.");
  } else {
    const layout = obj.layout as Record<string, unknown>;
    const version = (obj as Record<string, unknown>).version;

    if (version === "1.0") {
      // v1.0 flat grid validation
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
        }
      }
    } else {
      // v2.0 layout tree validation
      if (!("layoutTree" in layout) || typeof layout.layoutTree !== "object" || layout.layoutTree === null) {
        errors.push("layout.layoutTree must be a valid layout node object.");
      } else {
        const tree = layout.layoutTree as Record<string, unknown>;
        if (!["row", "col", "stack", "widget"].includes(tree.type as string)) {
          errors.push(`layout.layoutTree.type "${tree.type}" is not a valid node type.`);
        }
      }
      if ("canvasItems" in layout && !Array.isArray(layout.canvasItems)) {
        errors.push("layout.canvasItems must be an array.");
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
// v1.0 → v2.0 Migration
// ---------------------------------------------------------------------------

/**
 * Convert a v1.0 flat widget list into a v2.0 column layout tree.
 * Each widget becomes a leaf in a single top-level column.
 */
function migrateV1Widgets(serialized: SerializedWidget[]): LayoutNode {
  const validWidgets = serialized.filter((w) => KNOWN_WIDGET_TYPES.has(w.type));
  if (validWidgets.length === 0) {
    return getDefaultLayout();
  }

  const children: LayoutNode[] = validWidgets.map((w, i) => ({
    type: "widget" as const,
    id: `migrated-${i}-${Date.now().toString(36)}`,
    widgetType: w.type as WidgetType,
    widgetData: w.data ?? {},
  }));

  return {
    type: "col",
    id: `migrated-root-${Date.now().toString(36)}`,
    children,
    sizes: children.map(() => 1),
  };
}

// ---------------------------------------------------------------------------
// Import (JSON → Stores)
// ---------------------------------------------------------------------------

/**
 * Import a workspace from a parsed JSON object.
 *
 * Validates the data, hydrates all stores, and logs a summary.
 * Supports both v1.0 (flat grid → auto-migrated) and v2.0 (layout tree).
 * Does NOT trigger any computation - the user must explicitly
 * Load Model / Construct Reaction.
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

  // --- Hydrate layout ---
  if (workspace.version === "1.0") {
    // Legacy v1.0 flat grid → migrate to tree
    const layout = workspace.layout as unknown as { widgets?: SerializedWidget[] };
    const tree = migrateV1Widgets(layout.widgets ?? []);
    setLayoutRoot(tree);
    clearCanvas();
    appendLog(
      `Workspace "${workspace.metadata.name}" imported (v1.0 → v2.0 migration) - ` +
        `framework: ${workspace.physics.framework}`,
    );
  } else {
    // v2.0 layout tree
    setLayoutRoot(workspace.layout.layoutTree);
    if (Array.isArray(workspace.layout.canvasItems)) {
      canvasItems.set(workspace.layout.canvasItems);
    } else {
      clearCanvas();
    }
    appendLog(
      `Workspace "${workspace.metadata.name}" imported - ` +
        `framework: ${workspace.physics.framework}`,
    );
  }

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
 * Clears physics inputs, resets the docking layout tree and canvas,
 * clears auto-save, and resets the framework to StandardModel.
 */
export function resetWorkspace(): void {
  resetAllInputs();
  resetDockingLayout();
  clearCanvas();
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
