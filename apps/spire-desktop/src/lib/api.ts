/**
 * SPIRE TypeScript API Client
 *
 * Compatibility façade that delegates all backend calls through the
 * environment-aware transport layer (`$lib/core/backend`).
 *
 * Prior to Phase 36 this module called `invoke()` from
 * `@tauri-apps/api/tauri` directly - which crashed in any
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
  ObservableKind,
  SimplifiedExpressionResult,
  DimensionalCheckReport,
  AnalysisConfig,
  AnalysisResult,
  GoodnessOfFitResult,
  ObservableFitInput,
  GlobalObservableFitResult,
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
 * Run modular rule-based simplification on a diagram CAS expression.
 */
export async function simplifyExpression(
  diagram: FeynmanDiagram,
  dim?: SpacetimeDimension,
  observable?: ObservableKind,
): Promise<SimplifiedExpressionResult> {
  return getBackend().simplifyExpression(diagram, dim, observable);
}

/**
 * Verify mass-dimension consistency of a diagram-derived expression.
 */
export async function verifyDimensions(
  diagram: FeynmanDiagram,
  dim?: SpacetimeDimension,
  observable?: ObservableKind,
): Promise<DimensionalCheckReport> {
  return getBackend().verifyDimensions(diagram, dim, observable);
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

/**
 * Compute χ² between one theory histogram and one experimental dataset.
 */
export async function computeChiSquare(
  theoryBinContents: number[],
  theoryBinEdges: number[],
  expCsv: string,
  expLabel: string,
): Promise<GoodnessOfFitResult> {
  return getBackend().computeChiSquare(theoryBinContents, theoryBinEdges, expCsv, expLabel);
}

/**
 * Compute global χ² over multiple observables.
 */
export async function computeGlobalFit(
  observables: ObservableFitInput[],
  nParams: number,
): Promise<GlobalObservableFitResult> {
  return getBackend().computeGlobalFit(observables, nParams);
}

// ---------------------------------------------------------------------------
// Scripting - Observables & Cuts
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
 * @param slhaText - Raw SLHA file content (blocks + decay tables).
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
 * @param fileContents - The UFO file contents.
 * @param modelName - Human-readable model name (e.g., "MSSM").
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
 * @param input - Lagrangian term string (e.g., "e * psi_bar * gamma_mu * psi * A^mu").
 * @param knownFields - Field → spin mapping for the parser.
 * @param externalFields - External legs for vertex derivation.
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

// ── Decay Calculator ────────────────────────────────────────────────────────

/**
 * Calculate the full decay table for a particle within a theoretical model.
 *
 * Discovers all kinematically allowed decay channels and computes
 * partial widths and branching ratios.
 */
export async function calculateDecayTable(
  model: import("$lib/types/spire").TheoreticalModel,
  particleId: string,
): Promise<import("$lib/types/spire").CalcDecayTable> {
  return getBackend().calculateDecayTable(model, particleId);
}

/**
 * Export a particle's decay table in SLHA format.
 *
 * Returns the SLHA DECAY block as a string, ready for writing to file
 * or pasting into external tools.
 */
export async function exportDecaySlha(
  model: import("$lib/types/spire").TheoreticalModel,
  particleId: string,
  pdgCode: number,
): Promise<string> {
  return getBackend().exportDecaySlha(model, particleId, pdgCode);
}

// ── NLO Configuration ───────────────────────────────────────────────────────

/**
 * Configure the NLO dipole-subtraction scheme.
 *
 * Sends the given NLO configuration to the backend so that
 * subsequent cross-section calculations can include next-to-leading-order
 * corrections via the Catani–Seymour (or other) subtraction scheme.
 */
export async function configureNlo(
  config: import("$lib/types/spire").NloConfig,
): Promise<void> {
  return getBackend().configureNlo(config);
}

// ── Parton Shower Configuration ─────────────────────────────────────────────

/**
 * Configure the external parton shower provider.
 *
 * Stores the user's shower settings (provider executable, hadronisation
 * toggle, etc.) so that downstream event generation can optionally pipe
 * partonic events through an external shower program.
 */
export async function configureShower(
  config: import("$lib/types/spire").ShowerToggleConfig,
): Promise<void> {
  return getBackend().configureShower(config);
}

// ── Proof Generation ────────────────────────────────────────────────────────

/**
 * Generate a complete mathematical proof document for a Feynman diagram.
 *
 * Drives the algebraic proof tracker through the amplitude derivation
 * pipeline and compiles the result into a standalone LaTeX source string
 * using the `revtex4-2` document class.
 *
 * The returned string is a complete `.tex` file ready for `pdflatex`.
 *
 * @param diagram - A `FeynmanDiagram` from a prior `generateDiagrams` call.
 * @param processLabel - Human-readable process label (e.g., "e⁺e⁻ → μ⁺μ⁻").
 * @param dim - Spacetime dimension configuration.
 * @returns The complete LaTeX source as a string.
 */
export async function generateMathematicalProof(
  diagram: import("$lib/types/spire").FeynmanDiagram,
  processLabel: string,
  dim: import("$lib/types/spire").SpacetimeDimension,
): Promise<string> {
  return getBackend().generateMathematicalProof(diagram, processLabel, dim);
}

// ── Data Provenance ─────────────────────────────────────────────────────────

/**
 * Compute a SHA-256 provenance hash for the current computational state.
 *
 * Serializes the model, reaction, kinematic configuration, and random
 * seed into a canonical JSON form and returns the hex-encoded digest
 * alongside the full serialized payload.
 */
export async function computeProvenanceHash(
  model: import("$lib/types/spire").TheoreticalModel,
  reaction: import("$lib/types/spire").Reaction | null,
  cmsEnergy: number,
  numEvents: number,
  seed: number,
): Promise<{ sha256: string; payload: string }> {
  return getBackend().computeProvenanceHash(model, reaction, cmsEnergy, numEvents, seed);
}

/**
 * Load and verify a provenance payload, restoring the full computational
 * state. Recomputes the SHA-256 hash to verify data integrity before
 * returning the deserialized state.
 *
 * @param payload - JSON string of a serialized ProvenanceRecord.
 */
export async function loadProvenanceState(
  payload: string,
): Promise<Record<string, unknown>> {
  return getBackend().loadProvenanceState(payload);
}

// ---------------------------------------------------------------------------
// Cosmological Relic Density
// ---------------------------------------------------------------------------

/**
 * Compute the cosmological relic density for a dark matter candidate.
 *
 * Integrates the Boltzmann equation through the thermal freeze-out epoch.
 * Returns Ω h², freeze-out temperature, and evolution curve for log-log plotting.
 */
export async function calculateRelicDensity(
  config: import("$lib/types/spire").RelicConfig,
): Promise<import("$lib/types/spire").RelicDensityReport> {
  return getBackend().calculateRelicDensity(config);
}

// ---------------------------------------------------------------------------
// Flavor Physics - Lattice QCD & EFT
// ---------------------------------------------------------------------------

/**
 * Compute neutral B-meson mixing mass differences ΔMd and ΔMs.
 *
 * Uses Lattice QCD inputs (decay constants, bag parameters) combined
 * with the Inami–Lim function for the top-quark box diagram.
 */
export async function calculateBMixing(
  lattice: import("$lib/types/spire").LatticeInputs,
): Promise<import("$lib/types/spire").BMixingResult> {
  return getBackend().calculateBMixing(lattice);
}

/**
 * Compute the full flavor observable report for B → K ℓ⁺ℓ⁻.
 *
 * Integrates the differential decay rate over a q² window,
 * evaluates B-meson mixing, and returns the differential spectrum
 * for plotting.
 *
 * @param q2Min - Lower bound of the q² integration window in GeV².
 * @param q2Max - Upper bound of the q² integration window in GeV².
 * @param wilsonCoeffs - Wilson coefficients (SM + BSM shifts).
 * @param lattice - Lattice QCD inputs.
 * @param nPoints - Number of evaluation points for the spectrum.
 */
export async function calculateBToKll(
  q2Min: number,
  q2Max: number,
  wilsonCoeffs: import("$lib/types/spire").WilsonCoefficients,
  lattice: import("$lib/types/spire").LatticeInputs,
  nPoints?: number,
): Promise<import("$lib/types/spire").FlavorObservableReport> {
  return getBackend().calculateBToKll(q2Min, q2Max, wilsonCoeffs, lattice, nPoints);
}

// ---------------------------------------------------------------------------
// Hardware Backend Query
// ---------------------------------------------------------------------------

/**
 * Hardware capabilities report from the backend.
 */
export interface HardwareReport {
  /** Whether the binary was compiled with the `gpu` feature. */
  gpu_feature_compiled: boolean;
  /** Whether a compatible GPU adapter was found at runtime. */
  gpu_adapter_available: boolean;
  /** Human-readable GPU adapter name (empty if none). */
  adapter_name: string;
  /** Description of the CPU backend. */
  cpu_backend: string;
  /** Description of the GPU backend. */
  gpu_backend: string;
}

/**
 * Query the backend for available hardware compute backends.
 *
 * Returns whether GPU acceleration is compiled in and available at
 * runtime, along with adapter descriptions.  The frontend uses this
 * to conditionally display a GPU toggle in the Analysis widget.
 */
export async function queryHardwareBackends(): Promise<HardwareReport> {
  return getBackend().queryHardwareBackends();
}

// ---------------------------------------------------------------------------
// Plugin System (Phase 54)
// ---------------------------------------------------------------------------

/** Plugin information returned by the backend. */
export interface PluginInfo {
  /** Human-readable plugin name. */
  name: string;
  /** Semver version string. */
  version: string;
  /** One-line description. */
  description: string;
  /** Plugin author(s). */
  author: string;
  /** Declared capabilities (hook names). */
  capabilities: string[];
  /** Whether the plugin is currently active. */
  enabled: boolean;
}

/**
 * Load a WASM plugin from the given file path.
 *
 * The backend compiles, sandboxes, and validates the plugin before
 * returning its metadata summary.
 */
export async function loadPlugin(path: string): Promise<PluginInfo> {
  return getBackend().loadPlugin(path);
}

/**
 * List all currently loaded and active plugins.
 */
export async function listActivePlugins(): Promise<PluginInfo[]> {
  return getBackend().listActivePlugins();
}

/**
 * Unload a plugin by its unique name.
 */
export async function unloadPlugin(name: string): Promise<void> {
  return getBackend().unloadPlugin(name);
}

// ---------------------------------------------------------------------------
// Global Fits & MCMC (Phase 55)
// ---------------------------------------------------------------------------

/**
 * Start a background MCMC global fit.
 *
 * The fit runs asynchronously in a dedicated thread. Use `getMcmcStatus()`
 * to poll progress, and `stopMcmcFit()` to cancel.
 *
 * @param request - The model + fit configuration.
 */
export async function startMcmcFit(
  request: import("$lib/types/spire").McmcFitRequest,
): Promise<void> {
  return getBackend().startMcmcFit(request);
}

/**
 * Poll the current MCMC fit status and optionally retrieve samples.
 *
 * @param includeSamples - If true, the response includes the flat
 *   samples array (burn-in already stripped). Set to false for
 *   lightweight progress polling.
 */
export async function getMcmcStatus(
  includeSamples: boolean,
): Promise<import("$lib/types/spire").McmcFitStatus> {
  return getBackend().getMcmcStatus(includeSamples);
}

/**
 * Request the background MCMC fit to stop gracefully.
 *
 * Waits for the sampler to finish its current step before returning.
 */
export async function stopMcmcFit(): Promise<void> {
  return getBackend().stopMcmcFit();
}
// ---------------------------------------------------------------------------
// Particle Data Group (PDG) Integration (Phase 73)
// ---------------------------------------------------------------------------

import type {
  PdgMetadata,
  PdgParticleRecord,
  PdgDecayTable,
  PdgExtractionPolicy,
} from "./types/spire";

/**
 * Get metadata about the PDG database (edition, version, timestamp).
 *
 * @returns PDG metadata with edition, version, timestamp, and source file paths.
 */
export async function getPdgMetadata(): Promise<PdgMetadata> {
  return getBackend().pdgGetMetadata();
}

/**
 * Look up a particle by its PDG Monte Carlo ID (MCID).
 *
 * @param mcid - The Monte Carlo ID (e.g., 11 for electron).
 * @returns The `PdgParticleRecord` with mass, width, lifetime, and quantum numbers.
 */
export async function lookupParticleByMcid(mcid: number): Promise<PdgParticleRecord> {
  return getBackend().pdgLookupParticleByMcid(mcid);
}

/**
 * Look up a particle by its PDG ID or name.
 *
 * @param pdgid - The PDG identifier or name (e.g., "e-", "electron", "11").
 * @returns The `PdgParticleRecord` with mass, width, lifetime, and quantum numbers.
 */
export async function lookupParticleByPdgid(pdgid: string): Promise<PdgParticleRecord> {
  return getBackend().pdgLookupParticleByPdgid(pdgid);
}

/**
 * Get full particle properties from the PDG database.
 *
 * Equivalent to `lookupParticleByMcid` but uses the standard canonical API.
 *
 * @param mcid - The Monte Carlo ID.
 * @returns The `PdgParticleRecord`.
 */
export async function getParticleProperties(mcid: number): Promise<PdgParticleRecord> {
  return getBackend().pdgGetParticleProperties(mcid);
}

/**
 * Get the decay table for a particle with optional policy filtering.
 *
 * `StrictPhysical` returns only concrete, Monte-Carlo-samplable channels.
 * `Catalog` returns all channels including generic placeholders.
 *
 * @param mcid - The parent particle's Monte Carlo ID.
 * @param policy - The extraction policy ("StrictPhysical" or "Catalog").
 * @returns The `PdgDecayTable` with decay channels and branching ratios.
 */
export async function getDecayTable(
  mcid: number,
  policy: PdgExtractionPolicy = "Catalog",
): Promise<PdgDecayTable> {
  return getBackend().pdgGetDecayTable(mcid, policy);
}

/**
 * Synchronize a theoretical model with PDG database values.
 *
 * Takes an entire `TheoreticalModel`, looks up particles in the PDG database,
 * and returns the model with updated mass/width values.
 *
 * Adheres to stateless isomorphism: no shared mutable state across IPC boundary.
 *
 * @param model - The model to synchronize.
 * @returns The updated model with PDG values.
 */
export async function syncModel(model: TheoreticalModel): Promise<TheoreticalModel> {
  return getBackend().pdgSyncModel(model);
}

/**
 * Synchronize a theoretical model with PDG database values using explicit
 * merge/bootstrap controls.
 */
export async function syncModelWithOptions(
  model: TheoreticalModel,
  options: import("$lib/types/spire").PdgSyncOptions,
): Promise<TheoreticalModel> {
  return getBackend().pdgSyncModel(model, options);
}

/**
 * Search for particles by name, label, or identifier fragment.
 *
 * @param query - The search query (e.g., "electron", "e-", "photon").
 * @returns Array of matching `PdgParticleRecord` objects.
 */
export async function searchParticles(query: string): Promise<PdgParticleRecord[]> {
  return getBackend().pdgSearchIdentifiers(query);
}