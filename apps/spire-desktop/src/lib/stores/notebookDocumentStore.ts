/**
 * SPIRE - Notebook Document Store
 *
 * Reactive store that manages the active notebook document: cell CRUD,
 * reordering, and execution dispatch to the Rust backend via Tauri IPC.
 *
 * ## Design
 *
 * - One store instance manages a single `NotebookDocument`.
 * - Execution is async: the store sets `cell.running = true`, awaits the
 *   IPC call, then writes the result back and clears `running`.
 * - The session is lazily created on the first execution request.
 *
 * ## State Isolation
 *
 * The notebook store is independent of the physics store (`physicsStore`)
 * and layout store (`layoutStore`).  Notebook execution cannot corrupt
 * the GUI workspace state.
 */

import { writable, derived, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/tauri";
import {
  type CellType,
  type CellData,
  type CellExecutionResult,
  type NotebookDocument,
  createCell,
  createNotebookDocument,
} from "$lib/core/domain/notebook";

// ===========================================================================
// Store
// ===========================================================================

export const notebookDocument = writable<NotebookDocument>(
  createNotebookDocument(),
);

// ===========================================================================
// Derived Helpers
// ===========================================================================

/** Number of cells in the active document. */
export const cellCount = derived(notebookDocument, ($d) => $d.cells.length);

/** Whether any cell is currently running. */
export const isExecuting = derived(notebookDocument, ($d) =>
  $d.cells.some((c) => c.running),
);

// ===========================================================================
// Internal: Session Lifecycle
// ===========================================================================

/**
 * Ensure a backend session exists, creating one lazily if needed.
 * Returns the session ID.
 */
async function ensureSession(): Promise<string> {
  const doc = get(notebookDocument);
  if (doc.sessionId) return doc.sessionId;

  const sessionId: string = await invoke("session_create");

  notebookDocument.update((d) => ({ ...d, sessionId }));
  return sessionId;
}

// ===========================================================================
// Cell CRUD Actions
// ===========================================================================

/** Insert a new cell of the given type after the cell at `afterIndex`.
 *  If `afterIndex` is -1, insert at the top. */
export function insertCell(
  type: CellType,
  afterIndex: number,
  source = "",
): string {
  const cell = createCell(type, source);
  notebookDocument.update((d) => {
    const cells = [...d.cells];
    cells.splice(afterIndex + 1, 0, cell);
    return { ...d, cells, dirty: true };
  });
  return cell.id;
}

/** Append a new cell at the end of the document. */
export function appendCell(type: CellType, source = ""): string {
  const cell = createCell(type, source);
  notebookDocument.update((d) => ({
    ...d,
    cells: [...d.cells, cell],
    dirty: true,
  }));
  return cell.id;
}

/** Remove a cell by ID. */
export function removeCell(id: string): void {
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.filter((c) => c.id !== id),
    dirty: true,
  }));
}

/** Update the source text of a cell. */
export function updateCellSource(id: string, source: string): void {
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.map((c) => (c.id === id ? { ...c, source } : c)),
    dirty: true,
  }));
}

/** Change the type of a cell (clears its execution result). */
export function changeCellType(id: string, type: CellType): void {
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.map((c) =>
      c.id === id
        ? { ...c, type, executionCount: null, lastResult: null }
        : c,
    ),
    dirty: true,
  }));
}

/** Move a cell from one position to another. */
export function moveCell(fromIndex: number, toIndex: number): void {
  notebookDocument.update((d) => {
    const cells = [...d.cells];
    const [moved] = cells.splice(fromIndex, 1);
    if (!moved) return d;
    cells.splice(toIndex, 0, moved);
    return { ...d, cells, dirty: true };
  });
}

// ===========================================================================
// Execution Actions
// ===========================================================================

/**
 * Execute a single cell.  Markdown cells are no-ops.
 * Returns the execution result (or null for markdown).
 */
export async function executeCell(
  id: string,
): Promise<CellExecutionResult | null> {
  const doc = get(notebookDocument);
  const cell = doc.cells.find((c) => c.id === id);
  if (!cell) return null;

  // Markdown cells cannot be executed.
  if (cell.type === "markdown") return null;

  const sessionId = await ensureSession();

  // Mark as running
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.map((c) => (c.id === id ? { ...c, running: true } : c)),
  }));

  try {
    const command =
      cell.type === "script"
        ? "session_execute_script"
        : "session_execute_config";

    const argKey = cell.type === "script" ? "script" : "tomlContent";

    const result: CellExecutionResult = await invoke(command, {
      sessionId,
      [argKey]: cell.source,
    });

    // Write result back
    notebookDocument.update((d) => ({
      ...d,
      cells: d.cells.map((c) =>
        c.id === id
          ? {
              ...c,
              running: false,
              lastResult: result,
              executionCount: (c.executionCount ?? 0) + 1,
            }
          : c,
      ),
    }));

    return result;
  } catch (err) {
    const errorResult: CellExecutionResult = {
      success: false,
      output: "",
      error: String(err),
      duration_ms: 0,
      return_value: null,
    };

    notebookDocument.update((d) => ({
      ...d,
      cells: d.cells.map((c) =>
        c.id === id
          ? { ...c, running: false, lastResult: errorResult }
          : c,
      ),
    }));

    return errorResult;
  }
}

/**
 * Execute all executable cells in order, from top to bottom.
 * Stops on the first error if `stopOnError` is true.
 */
export async function executeAllCells(
  stopOnError = true,
): Promise<void> {
  const doc = get(notebookDocument);
  for (const cell of doc.cells) {
    if (cell.type === "markdown") continue;

    const result = await executeCell(cell.id);
    if (stopOnError && result && !result.success) break;
  }
}

/** Clear all execution results and counters. */
export function clearAllOutputs(): void {
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.map((c) => ({
      ...c,
      executionCount: null,
      lastResult: null,
      running: false,
    })),
  }));
}

/**
 * Reset the backend session (clears Rhai scope and model) and
 * clear all cell outputs.
 */
export async function resetSession(): Promise<void> {
  const doc = get(notebookDocument);
  if (doc.sessionId) {
    try {
      await invoke("session_reset", { sessionId: doc.sessionId });
    } catch {
      // If the session was already gone, ignore
    }
  }
  clearAllOutputs();
}

/**
 * Destroy the backend session entirely.
 * Called when the notebook widget is unmounted.
 */
export async function destroySession(): Promise<void> {
  const doc = get(notebookDocument);
  if (doc.sessionId) {
    try {
      await invoke("session_destroy", { sessionId: doc.sessionId });
    } catch {
      // Ignore
    }
    notebookDocument.update((d) => ({ ...d, sessionId: null }));
  }
}

/** Replace the entire document (e.g. when loading from file). */
export function loadDocument(doc: NotebookDocument): void {
  notebookDocument.set(doc);
}

/** Reset to a fresh empty document and destroy any backend session. */
export async function newDocument(title = "Untitled"): Promise<void> {
  await destroySession();
  notebookDocument.set(createNotebookDocument(title));
}
