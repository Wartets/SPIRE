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
import {
  type CellType,
  type CellData,
  type CellExecutionResult,
  type NotebookDocument,
  createCell,
  createNotebookDocument,
} from "$lib/core/domain/notebook";
import {
  theoreticalModel,
  activeReaction,
  generatedDiagrams,
  amplitudeResults,
  activeAmplitude,
  kinematics,
  observableScripts,
  cutScripts,
} from "$lib/stores/physicsStore";
import {
  cmsEnergyInput,
  initialIdsInput,
  finalIdsInput,
} from "$lib/stores/workspaceInputsStore";

let _invoke:
  | ((command: string, args?: Record<string, unknown>) => Promise<unknown>)
  | null = null;

async function invokeTauri(
  command: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  if (!_invoke) {
    const mod = await import("@tauri-apps/api/tauri");
    _invoke = mod.invoke;
  }
  return _invoke(command, args);
}

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

  const sessionId = (await invokeTauri("session_create")) as string;

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

/** Duplicate an existing cell by ID and insert clone directly below it. */
export function duplicateCell(id: string): string | null {
  const doc = get(notebookDocument);
  const index = doc.cells.findIndex((c) => c.id === id);
  if (index < 0) return null;

  const source = doc.cells[index].source;
  const type = doc.cells[index].type;
  return insertCell(type, index, source);
}

// ===========================================================================
// Physics Context Gathering
// ===========================================================================

/**
 * Build a JSON-serializable snapshot of the current physics workspace state.
 * This is injected into the Rhai scope so notebook cells can access live
 * workspace data (diagrams, amplitudes, kinematics, sqrt_s, etc.).
 */
function gatherPhysicsContext(): Record<string, unknown> {
  const ctx: Record<string, unknown> = {};

  // Model
  const model = get(theoreticalModel);
  if (model) {
    ctx.model_name = model.name ?? "Unknown";
    ctx.n_fields = model.fields?.length ?? 0;
    ctx.n_vertices = model.vertex_factors?.length ?? 0;
    ctx.field_names = (model.fields ?? []).map((f: any) => f.id ?? f.name ?? "");
    ctx.field_masses = (model.fields ?? []).map((f: any) => [
      f.id ?? f.name ?? "",
      f.mass ?? 0,
    ]);
  }

  // Reaction (Reaction.initial / Reaction.final_state are AsymptoticState)
  const rxn = get(activeReaction);
  if (rxn) {
    ctx.reaction_initial = (rxn.initial?.states ?? []).map(
      (s: any) => s.particle?.field?.id ?? "",
    );
    ctx.reaction_final = (rxn.final_state?.states ?? []).map(
      (s: any) => s.particle?.field?.id ?? "",
    );
    ctx.perturbative_order = rxn.perturbative_order ?? 0;
  }

  // Diagrams
  const diags = get(generatedDiagrams);
  if (diags) {
    ctx.n_diagrams = diags.diagrams?.length ?? 0;
    ctx.diagram_names = (diags.diagrams ?? []).map(
      (d: any, i: number) => d.label ?? d.name ?? `topology_${i}`,
    );
  }

  // Amplitudes
  const amps = get(amplitudeResults);
  if (amps && amps.length > 0) {
    ctx.amplitudes = amps.map((a: any) => a.expression ?? a.symbolic ?? "");
  }
  const activeAmp = get(activeAmplitude);
  if (activeAmp) {
    ctx.active_amplitude = activeAmp;
  }

  // Kinematics (KinematicsReport → .threshold.threshold_energy)
  const kin = get(kinematics);
  if (kin) {
    ctx.threshold_energy = kin.threshold?.threshold_energy ?? 0;
    ctx.is_kinematically_allowed = kin.is_allowed ?? false;
  }

  // Workspace inputs (individual stores)
  ctx.sqrt_s = get(cmsEnergyInput) ?? 0;
  ctx.initial_ids = get(initialIdsInput) ?? [];
  ctx.final_ids = get(finalIdsInput) ?? [];

  // Observable & cut names
  const obs = get(observableScripts);
  if (obs && obs.length > 0) {
    ctx.observable_names = obs.map((o: any) => o.name ?? "");
  }
  const cuts = get(cutScripts);
  if (cuts && cuts.length > 0) {
    ctx.cut_names = cuts.map((c: any) => c.name ?? "");
  }

  return ctx;
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

    // For script cells, gather live physics context from the workspace stores
    const invokeArgs: Record<string, unknown> = {
      sessionId,
      [argKey]: cell.source,
    };
    if (cell.type === "script") {
      invokeArgs.physicsContext = gatherPhysicsContext();
    }

    const result = (await invokeTauri(command, invokeArgs)) as CellExecutionResult;

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

/** Clear the execution result of a single cell. */
export function clearCellOutput(id: string): void {
  notebookDocument.update((d) => ({
    ...d,
    cells: d.cells.map((c) =>
      c.id === id
        ? { ...c, executionCount: null, lastResult: null, running: false }
        : c,
    ),
  }));
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
      await invokeTauri("session_reset", { sessionId: doc.sessionId });
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
      await invokeTauri("session_destroy", { sessionId: doc.sessionId });
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
