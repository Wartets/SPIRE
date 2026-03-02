/**
 * SPIRE Physics Store
 *
 * Central Svelte writable stores for the application's reactive state.
 * These stores hold the active reaction, selected theoretical framework,
 * generated diagrams, and computed results. Svelte components subscribe
 * to these stores and re-render automatically when the kernel produces
 * new data.
 */

import { writable, derived } from "svelte/store";
import type {
  Reaction,
  ReconstructedFinalState,
  TopologySet,
  AmplitudeResult,
  MandelstamVars,
  KinematicsReport,
  TheoreticalFramework,
  TheoreticalModel,
  WorkspaceState,
} from "../types/spire";

// ---------------------------------------------------------------------------
// Model & Framework
// ---------------------------------------------------------------------------

/** The currently active theoretical framework (SM by default). */
export const activeFramework = writable<TheoreticalFramework>("StandardModel");

/** The loaded theoretical model (particles + vertices + Feynman rules). */
export const theoreticalModel = writable<TheoreticalModel | null>(null);

// ---------------------------------------------------------------------------
// Reaction
// ---------------------------------------------------------------------------

/** The reaction currently being analysed (null when no reaction is set). */
export const activeReaction = writable<Reaction | null>(null);

/** Reconstructed possible final states (from incomplete initial state). */
export const reconstructedStates = writable<ReconstructedFinalState[]>([]);

// ---------------------------------------------------------------------------
// Diagrams & Amplitudes
// ---------------------------------------------------------------------------

/** Generated Feynman diagram topologies for the active reaction. */
export const generatedDiagrams = writable<TopologySet | null>(null);

/** Computed amplitude results for individual diagrams. */
export const amplitudeResults = writable<AmplitudeResult[]>([]);

/** The symbolic expression for the currently selected amplitude. */
export const activeAmplitude = writable<string>("");

// ---------------------------------------------------------------------------
// Kinematics
// ---------------------------------------------------------------------------

/** Full kinematics report (threshold, phase space, boundaries). */
export const kinematics = writable<KinematicsReport | null>(null);

/** Kinematic Mandelstam variables for the active process. */
export const mandelstamVars = writable<MandelstamVars | null>(null);

// ---------------------------------------------------------------------------
// Application Log
// ---------------------------------------------------------------------------

/** Rolling log of system messages, errors, and computation summaries. */
export const logs = writable<string[]>([]);

/** Append a timestamped message to the log store. */
export function appendLog(message: string): void {
  const ts = new Date().toLocaleTimeString();
  logs.update((prev) => [...prev, `[${ts}] ${message}`]);
}

// ---------------------------------------------------------------------------
// Convenience Aggregate
// ---------------------------------------------------------------------------

/** Full workspace state (convenience aggregate). */
export const workspaceState = writable<WorkspaceState>({
  framework: "StandardModel",
  active_reaction: null,
  diagrams: null,
  amplitudes: [],
  kinematics: null,
});

/** Derived flag: true when a model is loaded and ready. */
export const isModelLoaded = derived(theoreticalModel, ($m) => $m !== null);

/** Derived flag: true when diagrams have been generated. */
export const hasDiagrams = derived(
  generatedDiagrams,
  ($d) => $d !== null && $d.diagrams.length > 0,
);
