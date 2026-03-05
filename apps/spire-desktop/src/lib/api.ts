/**
 * SPIRE TypeScript API Client
 *
 * Compatibility façade that delegates all backend calls through the
 * environment-aware transport layer (`$lib/core/backend`).
 *
 * Prior to Phase 36 this module called `invoke()` from
 * `@tauri-apps/api/tauri` directly — which crashed in any
 * non-Tauri browser with:
 *   `window.__TAURI_IPC__ is not a function`
 *
 * Now every function resolves the active `SpireBackend` singleton
 * (Tauri, WASM, or Mock) and forwards the call.  Existing import
 * paths (`from "$lib/api"`) continue to work unchanged.
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

import { getBackend } from "$lib/core/backend";

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
  DerivationStep,
  SpacetimeDimension,
  AnalysisConfig,
  AnalysisResult,
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
  return getBackend().loadModel(particlesToml, verticesToml, modelName);
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
  return getBackend().constructReaction(initialIds, finalIds, cmsEnergy, model);
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
  return getBackend().reconstructReaction(initialIds, cmsEnergy, model);
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
  return getBackend().generateDiagrams(reaction, model, maxLoopOrder);
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
  return getBackend().deriveAmplitude(diagram);
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
  return getBackend().computeKinematics(finalMasses, cmsEnergy, targetMass, externalMasses);
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
  return getBackend().computeDalitzData(motherMass, mA, mB, mC, nPoints);
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
  return getBackend().exportAmplitudeLatex(diagram);
}

// ---------------------------------------------------------------------------
// CAS Derivation Steps
// ---------------------------------------------------------------------------

/**
 * Derive a step-by-step symbolic amplitude for a Feynman diagram using the
 * CAS engine. Each step shows one algebraic transformation (Feynman rules,
 * metric contractions, Dirac equation, simplification, etc.).
 *
 * @param diagram - A `FeynmanDiagram` from a prior diagram generation.
 * @param dim - Spacetime dimension specification (default: 4).
 * @returns Ordered array of `DerivationStep` objects.
 */
export async function deriveAmplitudeSteps(
  diagram: FeynmanDiagram,
  dim?: SpacetimeDimension,
): Promise<DerivationStep[]> {
  return getBackend().deriveAmplitudeSteps(diagram, dim);
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
  return getBackend().exportModelUfo(model);
}

// ---------------------------------------------------------------------------
// Analysis Pipeline
// ---------------------------------------------------------------------------

/**
 * Run a complete analysis pipeline: compile scripts, generate Monte Carlo
 * events, fill histograms, and return results.
 *
 * @param config - Analysis configuration with plot definitions, cuts,
 *   CMS energy, final masses, and number of events.
 * @returns An `AnalysisResult` with filled histograms and cross-section.
 */
export async function runAnalysis(
  config: AnalysisConfig,
): Promise<AnalysisResult> {
  return getBackend().runAnalysis(config);
}

// ---------------------------------------------------------------------------
// Scripting — Observables & Cuts
// ---------------------------------------------------------------------------

/**
 * Validate a Rhai script for syntax correctness.
 *
 * @param script - The Rhai script source code.
 * @returns Resolves if valid; rejects with an error message if invalid.
 */
export async function validateScript(script: string): Promise<void> {
  return getBackend().validateScript(script);
}

/**
 * Compile an observable script and evaluate it against a synthetic test event.
 *
 * The test event is two massless back-to-back particles at 100 GeV CMS with
 * two final-state particles at known momenta. This provides immediate feedback
 * without running a full integration.
 *
 * @param script - The Rhai script for the observable.
 * @returns The numeric result of evaluating the observable on the test event.
 */
export async function testObservableScript(
  script: string,
): Promise<number> {
  return getBackend().testObservableScript(script);
}

/**
 * Compile a kinematic cut script and test it against a synthetic event.
 *
 * @param script - The Rhai script for the cut predicate.
 * @returns `true` if the test event passes the cut, `false` otherwise.
 */
export async function testCutScript(
  script: string,
): Promise<boolean> {
  return getBackend().testCutScript(script);
}

// ---------------------------------------------------------------------------
// 3D Event Display
// ---------------------------------------------------------------------------

/**
 * Generate a single reconstructed event for 3D visualisation.
 *
 * Runs one RAMBO event through the specified detector model and returns
 * an `EventDisplayData` for rendering jets, tracks, and MET.
 *
 * @param cmsEnergy - Centre-of-mass energy (GeV).
 * @param finalMasses - Final-state particle masses.
 * @param detectorPreset - Detector preset name (e.g., "lhc_like").
 * @param particleKinds - Optional particle kind labels per final-state leg.
 */
export async function generateDisplayEvent(
  cmsEnergy: number,
  finalMasses: number[],
  detectorPreset: string,
  particleKinds?: string[] | null,
): Promise<import("$lib/types/spire").EventDisplayData> {
  return getBackend().generateDisplayEvent(cmsEnergy, finalMasses, detectorPreset, particleKinds);
}

/**
 * Generate a batch of Monte-Carlo events for animated 3D playback.
 *
 * @param cmsEnergy - Centre-of-mass energy (GeV).
 * @param finalMasses - Final-state particle masses.
 * @param detectorPreset - Detector preset name (e.g., "lhc_like").
 * @param batchSize - Number of events to generate (clamped to 1..100).
 * @param particleKinds - Optional particle kind labels per final-state leg.
 */
export async function generateDisplayBatch(
  cmsEnergy: number,
  finalMasses: number[],
  detectorPreset: string,
  batchSize: number,
  particleKinds?: string[] | null,
): Promise<import("$lib/types/spire").EventDisplayData[]> {
  return getBackend().generateDisplayBatch(cmsEnergy, finalMasses, detectorPreset, batchSize, particleKinds);
}

// ---------------------------------------------------------------------------
// Lagrangian Workbench (Phase 32)
// ---------------------------------------------------------------------------

/**
 * Parse a Lagrangian term string into its AST representation.
 */
export async function parseLagrangianTerm(
  input: string,
  knownFields: Record<string, import("$lib/types/spire").FieldSpin>,
): Promise<import("$lib/types/spire").LagrangianExpr> {
  return getBackend().parseLagrangianTerm(input, knownFields);
}

/**
 * Derive a vertex rule from a Lagrangian term via functional differentiation.
 */
export async function deriveVertexRuleFromAst(
  input: string,
  knownFields: Record<string, import("$lib/types/spire").FieldSpin>,
  externalFields: import("$lib/types/spire").ExternalField[],
): Promise<import("$lib/types/spire").DerivedVertexRule> {
  return getBackend().deriveVertexRuleFromAst(input, knownFields, externalFields);
}

/**
 * Validate a Lagrangian term for theoretical consistency.
 */
export async function validateLagrangianTerm(
  input: string,
  knownFields: Record<string, import("$lib/types/spire").FieldSpin>,
  gaugeSymmetry?: import("$lib/types/spire").GaugeSymmetry | null,
  fieldGaugeInfo?: Record<string, import("$lib/types/spire").FieldGaugeInfo>,
): Promise<import("$lib/types/spire").ValidationResult> {
  return getBackend().validateLagrangianTerm(input, knownFields, gaugeSymmetry, fieldGaugeInfo);
}

/**
 * Run an RGE coupling flow integration.
 */
export async function runRgeFlow(
  config: import("$lib/types/spire").RgeFlowConfig,
): Promise<import("$lib/types/spire").RgeFlowResult> {
  return getBackend().runRgeFlow(config);
}

// ---------------------------------------------------------------------------
// External Theory Bridge (Phase 33)
// ---------------------------------------------------------------------------

/**
 * Parse an SLHA spectrum string and return the structured document.
 *
 * @param slhaText — Raw SLHA file content (blocks + decay tables).
 * @returns Parsed `SlhaDocument` with blocks and decays.
 */
export async function importSlhaString(
  slhaText: string,
): Promise<import("$lib/types/spire").SlhaDocument> {
  return getBackend().importSlhaString(slhaText);
}

/**
 * Import a UFO model from its raw Python file contents.
 *
 * Pass an object with the string content of each `.py` file
 * (particles.py, vertices.py, etc.). Missing files can be set to `null`.
 *
 * @param fileContents — The UFO file contents.
 * @param modelName — Human-readable model name (e.g., "MSSM").
 * @returns A tuple of the raw `UfoModel` and the converted `TheoreticalModel`.
 */
export async function importUfoModel(
  fileContents: import("$lib/types/spire").UfoFileContents,
  modelName: string,
): Promise<[import("$lib/types/spire").UfoModel, import("$lib/types/spire").TheoreticalModel]> {
  return getBackend().importUfoModel(fileContents, modelName);
}

/**
 * Generate NLO counterterms from a Lagrangian expression string.
 *
 * Automatically builds a default renormalization scheme from the
 * expression's field and coupling content, then generates all
 * linear-in-δ counterterm vertices.
 *
 * @param input — Lagrangian term string (e.g., "e * psi_bar * gamma_mu * psi * A^mu").
 * @param knownFields — Field → spin mapping for the parser.
 * @param externalFields — External legs for vertex derivation.
 * @returns Full `CountertermResult` with counterterm ASTs and Feynman rules.
 */
export async function deriveCounterterms(
  input: string,
  knownFields: Record<string, import("$lib/types/spire").FieldSpin>,
  externalFields: import("$lib/types/spire").ExternalField[],
): Promise<import("$lib/types/spire").CountertermResult> {
  return getBackend().deriveCounterterms(input, knownFields, externalFields);
}

// ---------------------------------------------------------------------------
// Parameter Scanner (Phase 44)
// ---------------------------------------------------------------------------

/**
 * Run a 1D parameter scan.
 *
 * Sweeps a single parameter (field mass, width, coupling, or CMS energy)
 * across a range and evaluates the cross-section at each point.
 */
export async function runParameterScan1D(
  config: import("$lib/types/spire").ScanConfig1D,
): Promise<import("$lib/types/spire").ScanResult1D> {
  return getBackend().runParameterScan1D(config);
}

/**
 * Run a 2D parameter scan.
 *
 * Sweeps two parameters across a grid and evaluates the cross-section
 * at each point.
 */
export async function runParameterScan2D(
  config: import("$lib/types/spire").ScanConfig2D,
): Promise<import("$lib/types/spire").ScanResult2D> {
  return getBackend().runParameterScan2D(config);
}
