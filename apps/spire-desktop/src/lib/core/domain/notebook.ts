/**
 * SPIRE - Notebook Domain Types
 *
 * Data structures for the cell-based execution engine.  A notebook
 * document is an ordered list of typed cells (Markdown, Script, Config)
 * that are executed sequentially against a persistent Rhai session.
 *
 * All types are plain JSON-serialisable objects - no class instances
 * or circular references - so the entire document can be saved and
 * restored with `JSON.stringify()` / `JSON.parse()`.
 */

// ===========================================================================
// Cell Types
// ===========================================================================

/**
 * Discriminator for cell content types.
 *
 * - `"markdown"` - Rich-text annotation (not executed).
 * - `"script"`   - Rhai source code evaluated by the kernel.
 * - `"config"`   - TOML payload parsed to load a theoretical model.
 */
export type CellType = "markdown" | "script" | "config";

// ===========================================================================
// Execution Result (mirrors Rust `ExecutionResult`)
// ===========================================================================

/** Outcome of executing a single Script or Config cell. */
export interface CellExecutionResult {
  /** Whether execution completed without error. */
  success: boolean;
  /** Captured `print()` / `debug()` output (may be empty). */
  output: string;
  /** Error message if `success` is false. */
  error: string | null;
  /** Wall-clock execution time in milliseconds. */
  duration_ms: number;
  /**
   * JSON-encoded return value of the last expression.
   * `null` when the cell returns `()` or on error.
   */
  return_value: unknown | null;
}

// ===========================================================================
// Cell Data
// ===========================================================================

/** A single cell in a notebook document. */
export interface CellData {
  /** Unique stable identifier. */
  id: string;
  /** Content type discriminator. */
  type: CellType;
  /** Raw source text (Markdown / Rhai / TOML). */
  source: string;
  /**
   * Monotonically increasing execution counter.
   * `null` if the cell has never been executed.
   */
  executionCount: number | null;
  /** Result from the most recent execution, if any. */
  lastResult: CellExecutionResult | null;
  /** Whether this cell is currently being executed. */
  running: boolean;
}

// ===========================================================================
// Notebook Document
// ===========================================================================

/** A complete notebook comprising an ordered list of cells. */
export interface NotebookDocument {
  /** Display title for the notebook tab/header. */
  title: string;
  /** Ordered cells. */
  cells: CellData[];
  /**
   * Backend session ID returned by `session_create`.
   * `null` before the session has been initialised.
   */
  sessionId: string | null;
  /** Whether the document has unsaved modifications. */
  dirty: boolean;
}

// ===========================================================================
// Factory Helpers
// ===========================================================================

let _cellCounter = 0;

/** Generate a unique cell ID. */
export function makeCellId(): string {
  _cellCounter += 1;
  return `cell-${_cellCounter}-${Date.now().toString(36)}`;
}

/** Create a blank cell of the given type. */
export function createCell(type: CellType, source = ""): CellData {
  return {
    id: makeCellId(),
    type,
    source,
    executionCount: null,
    lastResult: null,
    running: false,
  };
}

/** Create a fresh, empty notebook document. */
export function createNotebookDocument(title = "Untitled"): NotebookDocument {
  return {
    title,
    cells: [
      createCell("markdown", "# New Notebook\n\nBegin by adding cells below."),
      createCell("script", "// Rhai script cell\nlet x = 42;\nprint(x);"),
    ],
    sessionId: null,
    dirty: false,
  };
}
