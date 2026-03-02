/**
 * SPIRE — Workspace Serialisation Schema
 *
 * Versioned JSON format for persisting and sharing computational
 * environments.  Strictly separates layout state (widget arrangement)
 * from domain state (physics inputs).  Computed results (diagrams,
 * amplitudes, kinematics) are NOT serialised — they are re-derived
 * from the stored inputs via the computation pipeline.
 *
 * File convention: `*.spire.json`
 * Current schema version: "1.0"
 */

import type { WidgetType } from "$lib/stores/notebookStore";
import type { TheoreticalFramework } from "./spire";

// ---------------------------------------------------------------------------
// Schema Version
// ---------------------------------------------------------------------------

/** The current workspace schema version. */
export const WORKSPACE_SCHEMA_VERSION = "1.0" as const;

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/** Descriptive metadata attached to every saved workspace. */
export interface WorkspaceMetadata {
  /** User-assigned workspace name. */
  name: string;
  /** ISO 8601 creation timestamp. */
  createdAt: string;
  /** ISO 8601 last-updated timestamp. */
  updatedAt: string;
}

// ---------------------------------------------------------------------------
// Layout State
// ---------------------------------------------------------------------------

/**
 * A serialised widget.
 *
 * Widget IDs are **not** persisted — they are regenerated on import.
 * This avoids collisions across workspaces and simplifies merging.
 */
export interface SerializedWidget {
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

/** Complete grid layout of the workbench. */
export interface WorkspaceLayout {
  /** Ordered list of widget placements. */
  widgets: SerializedWidget[];
  /** Number of grid columns (for forward-compatibility). */
  gridColumns: number;
}

// ---------------------------------------------------------------------------
// Domain State (Physics Inputs)
// ---------------------------------------------------------------------------

/**
 * User-provided physics parameters.
 *
 * These are the *inputs* to the computation pipeline, not the results.
 * On import the user must explicitly trigger Load Model → Construct
 * Reaction → Generate Diagrams etc. to reproduce computed state.
 */
export interface WorkspacePhysics {
  /** Active theoretical framework. */
  framework: TheoreticalFramework;
  /** Raw TOML content defining particle fields. */
  particlesToml: string;
  /** Raw TOML content defining interaction vertices. */
  verticesToml: string;
  /** Human-readable model name. */
  modelName: string;
  /** Particle IDs for the initial state (e.g. ["e-", "e+"]). */
  initialStateIds: string[];
  /** Particle IDs for the final state (e.g. ["mu-", "mu+"]). */
  finalStateIds: string[];
  /** Centre-of-mass energy in GeV. */
  cmsEnergy: number;
  /** Maximum perturbative loop order. */
  maxLoopOrder: number;
}

// ---------------------------------------------------------------------------
// Top-Level Workspace Envelope
// ---------------------------------------------------------------------------

/**
 * The complete serialised workspace.
 *
 * This is the root object written to `.spire.json` files and to
 * the `spire_autosave` localStorage key.
 */
export interface SpireWorkspace {
  /** Schema version for forward-compatible parsing. */
  version: typeof WORKSPACE_SCHEMA_VERSION;
  /** Descriptive metadata. */
  metadata: WorkspaceMetadata;
  /** Widget grid layout. */
  layout: WorkspaceLayout;
  /** User-provided physics inputs. */
  physics: WorkspacePhysics;
}

// ---------------------------------------------------------------------------
// Validation Result
// ---------------------------------------------------------------------------

/** Result of validating an imported workspace JSON blob. */
export interface WorkspaceValidationResult {
  /** Whether the workspace passed all validation checks. */
  valid: boolean;
  /** Human-readable warnings (e.g. unknown widget types filtered). */
  warnings: string[];
  /** Human-readable errors (e.g. missing required fields). */
  errors: string[];
}
