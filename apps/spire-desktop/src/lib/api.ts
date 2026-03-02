/**
 * SPIRE TypeScript API Client
 *
 * Typed wrapper functions around Tauri's `invoke()` IPC mechanism.
 * Each function corresponds to a `#[tauri::command]` in the Rust backend
 * (`src-tauri/src/main.rs`) and returns strongly-typed results matching
 * the interfaces in `./types/spire.ts`.
 *
 * ## Usage
 *
 * ```typescript
 * import { loadModel, constructReaction, generateDiagrams } from '$lib/api';
 *
 * const model = await loadModel(particlesToml, verticesToml);
 * const reaction = await constructReaction(['e-', 'e+'], ['mu-', 'mu+'], 10.0, model);
 * const diagrams = await generateDiagrams(reaction, model);
 * ```
 */

import { invoke } from "@tauri-apps/api/tauri";

import type {
  Reaction,
  ReconstructedFinalState,
  TopologySet,
  AmplitudeResult,
  KinematicsReport,
  DalitzPlotData,
  TheoreticalModel,
  FeynmanDiagram,
  UfoExportResult,
} from "./types/spire";

// ---------------------------------------------------------------------------
// Model Loading
// ---------------------------------------------------------------------------

/**
 * Load a theoretical model from TOML file contents.
 *
 * @param particlesToml - Raw contents of `particles.toml`.
 * @param verticesToml - Raw contents of `sm_vertices.toml` (or BSM model).
 * @param modelName - Optional model name (defaults to "Standard Model").
 * @returns The complete `TheoreticalModel`.
 */
export async function loadModel(
  particlesToml: string,
  verticesToml: string,
  modelName?: string,
): Promise<TheoreticalModel> {
  return invoke<TheoreticalModel>("load_theoretical_model", {
    particlesToml,
    verticesToml,
    modelName: modelName ?? null,
  });
}

// ---------------------------------------------------------------------------
// Reaction Construction & Reconstruction
// ---------------------------------------------------------------------------

/**
 * Construct and validate a specific particle reaction.
 *
 * @param initialIds - Initial-state particle IDs (e.g., `["e-", "e+"]`).
 * @param finalIds - Final-state particle IDs (e.g., `["mu-", "mu+"]`).
 * @param cmsEnergy - Centre-of-mass energy in GeV.
 * @param model - The active `TheoreticalModel`.
 * @returns The validated `Reaction` object.
 */
export async function constructReaction(
  initialIds: string[],
  finalIds: string[],
  cmsEnergy: number,
  model: TheoreticalModel,
): Promise<Reaction> {
  return invoke<Reaction>("validate_and_reconstruct_reaction", {
    initialIds,
    cmsEnergy,
    model,
    finalIds,
  });
}

/**
 * Reconstruct all kinematically and dynamically allowed two-body final
 * states for a given initial state and energy.
 *
 * @param initialIds - Initial-state particle IDs.
 * @param cmsEnergy - Centre-of-mass energy in GeV.
 * @param model - The active `TheoreticalModel`.
 * @returns Array of possible `ReconstructedFinalState` objects.
 */
export async function reconstructReaction(
  initialIds: string[],
  cmsEnergy: number,
  model: TheoreticalModel,
): Promise<ReconstructedFinalState[]> {
  return invoke<ReconstructedFinalState[]>(
    "validate_and_reconstruct_reaction",
    {
      initialIds,
      cmsEnergy,
      model,
      finalIds: null,
    },
  );
}

// ---------------------------------------------------------------------------
// Feynman Diagram Generation
// ---------------------------------------------------------------------------

/**
 * Generate all topologically distinct Feynman diagrams for a reaction.
 *
 * @param reaction - A validated `Reaction` from a prior construction call.
 * @param model - The active `TheoreticalModel`.
 * @param maxLoopOrder - Maximum loop order (0 = tree-level). Defaults to 0.
 * @returns The complete `TopologySet` of diagrams.
 */
export async function generateDiagrams(
  reaction: Reaction,
  model: TheoreticalModel,
  maxLoopOrder: number = 0,
): Promise<TopologySet> {
  return invoke<TopologySet>("generate_feynman_diagrams", {
    reaction,
    model,
    maxLoopOrder: maxLoopOrder > 0 ? maxLoopOrder : null,
  });
}

// ---------------------------------------------------------------------------
// Amplitude Computation
// ---------------------------------------------------------------------------

/**
 * Compute the invariant amplitude for a specific Feynman diagram.
 *
 * @param diagram - A `FeynmanDiagram` from a prior diagram generation.
 * @returns The symbolic `AmplitudeResult`.
 */
export async function deriveAmplitude(
  diagram: FeynmanDiagram,
): Promise<AmplitudeResult> {
  return invoke<AmplitudeResult>("derive_amplitude", {
    diagram,
  });
}

// ---------------------------------------------------------------------------
// Kinematics
// ---------------------------------------------------------------------------

/**
 * Compute kinematic properties for a set of final-state masses at a given
 * energy.
 *
 * @param finalMasses - Array of final-state particle masses in GeV.
 * @param cmsEnergy - Available centre-of-mass energy in GeV.
 * @param targetMass - Optional target mass for fixed-target kinematics.
 * @param externalMasses - Optional 4-element array for Mandelstam boundaries.
 * @returns A `KinematicsReport` with threshold, phase space, and boundaries.
 */
export async function computeKinematics(
  finalMasses: number[],
  cmsEnergy: number,
  targetMass?: number,
  externalMasses?: [number, number, number, number],
): Promise<KinematicsReport> {
  return invoke<KinematicsReport>("compute_kinematics", {
    finalMasses,
    cmsEnergy,
    targetMass: targetMass ?? null,
    externalMasses: externalMasses ?? null,
  });
}

// ---------------------------------------------------------------------------
// Dalitz Plot
// ---------------------------------------------------------------------------

/**
 * Generate Dalitz plot phase-space data for a 3-body decay.
 *
 * @param motherMass - Mass of the decaying particle (GeV).
 * @param mA - Mass of daughter A (GeV).
 * @param mB - Mass of daughter B (GeV).
 * @param mC - Mass of daughter C (GeV).
 * @param nPoints - Approximate number of phase-space points to generate.
 * @returns `DalitzPlotData` with boundaries and coordinate pairs.
 */
export async function computeDalitzData(
  motherMass: number,
  mA: number,
  mB: number,
  mC: number,
  nPoints: number = 3000,
): Promise<DalitzPlotData> {
  return invoke<DalitzPlotData>("compute_dalitz_data", {
    motherMass,
    mA,
    mB,
    mC,
    nPoints,
  });
}

// ---------------------------------------------------------------------------
// Export (LaTeX & UFO)
// ---------------------------------------------------------------------------

/**
 * Export a Feynman diagram's amplitude as a LaTeX string.
 *
 * @param diagram - A `FeynmanDiagram` from a prior diagram generation.
 * @returns The LaTeX string, e.g. `i\mathcal{M} = \bar{u}(p_3) ...`.
 */
export async function exportAmplitudeLatex(
  diagram: FeynmanDiagram,
): Promise<string> {
  return invoke<string>("export_amplitude_latex", {
    diagram,
  });
}

/**
 * Export the theoretical model in simplified UFO format.
 *
 * @param model - The active `TheoreticalModel`.
 * @returns A map of `{ filename: content }` (particles.py, parameters.py, __init__.py).
 */
export async function exportModelUfo(
  model: TheoreticalModel,
): Promise<UfoExportResult> {
  return invoke<UfoExportResult>("export_model_ufo", {
    model,
  });
}
