/**
 * SPIRE - Abstract Backend Interface
 *
 * Defines the complete contract between the frontend UI and the physics
 * computation engine.  Every backend strategy (Tauri IPC, WebAssembly,
 * Mock) implements this interface identically, allowing the UI layer to
 * remain entirely transport-agnostic.
 *
 * ## Design Rationale
 *
 * The Strategy pattern decouples Svelte components from any particular
 * execution environment.  Components call `backend.loadModel(...)` without
 * knowledge of whether the call routes through Tauri IPC to a native Rust
 * process, through a WASM module running in a Web Worker, or through a
 * static mock for offline UI development.
 *
 * All methods return Promises so that callers always use the same async
 * control flow regardless of the underlying transport latency.
 */

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
  EventDisplayData,
  LagrangianExpr,
  ExternalField,
  DerivedVertexRule,
  ValidationResult,
  FieldSpin,
  GaugeSymmetry,
  FieldGaugeInfo,
  RgeFlowConfig,
  RgeFlowResult,
  SlhaDocument,
  UfoFileContents,
  UfoModel,
  CountertermResult,
  ScanConfig1D,
  ScanResult1D,
  ScanConfig2D,
  ScanResult2D,
  CalcDecayTable,
  NloConfig,
  ShowerToggleConfig,
  RelicConfig,
  RelicDensityReport,
  LatticeInputs,
  WilsonCoefficients,
  BMixingResult,
  FlavorObservableReport,
  McmcFitRequest,
  McmcFitStatus,
  GoodnessOfFitResult,
  ObservableFitInput,
  GlobalObservableFitResult,
} from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Backend Environment Classification
// ---------------------------------------------------------------------------

/**
 * Identifies which execution environment is currently active.
 *
 * - `"tauri"` - Native Tauri IPC bridge to the Rust backend process.
 * - `"wasm"`  - In-browser WebAssembly module (spire-kernel compiled to wasm32).
 * - `"mock"`  - Static fallback returning safe placeholder data.
 */
export type BackendKind = "tauri" | "wasm" | "mock";

// ---------------------------------------------------------------------------
// Standardised Error Type
// ---------------------------------------------------------------------------

/**
 * A structured error from any backend implementation.
 *
 * UI components can catch `BackendError` uniformly regardless of the
 * active transport.  The `kind` discriminator enables targeted error
 * handling (e.g., showing a "reconnect" action for transport failures
 * vs. a "fix input" hint for validation errors).
 */
export class BackendError extends Error {
  /** Error category for programmatic branching. */
  public readonly kind: BackendErrorKind;
  /** The backend that produced this error. */
  public readonly backend: BackendKind;
  /** Optional upstream error or diagnostic payload. */
  public readonly cause?: unknown;

  constructor(
    kind: BackendErrorKind,
    backend: BackendKind,
    message: string,
    cause?: unknown,
  ) {
    super(message);
    this.name = "BackendError";
    this.kind = kind;
    this.backend = backend;
    this.cause = cause;
  }
}

/**
 * Discriminated error categories.
 *
 * - `"transport"` - IPC / network / serialisation failure.
 * - `"kernel"`    - The physics kernel returned an error (bad input, etc.).
 * - `"unsupported"` - The active backend does not implement this operation.
 * - `"timeout"`   - The operation exceeded its time budget.
 */
export type BackendErrorKind =
  | "transport"
  | "kernel"
  | "unsupported"
  | "timeout";

// ---------------------------------------------------------------------------
// SpireBackend Interface
// ---------------------------------------------------------------------------

/**
 * The canonical backend contract.
 *
 * Every method corresponds 1-to-1 with a Tauri `#[tauri::command]` and
 * an exported function in `$lib/api.ts`.  Implementations may throw
 * `BackendError` on failure.
 */
export interface SpireBackend {
  /** Which execution environment this backend represents. */
  readonly kind: BackendKind;

  // ── Model Loading ──────────────────────────────────────────────────────
  loadModel(
    particlesToml: string,
    verticesToml: string,
    modelName?: string,
  ): Promise<TheoreticalModel>;

  // ── Reaction Construction ──────────────────────────────────────────────
  constructReaction(
    initialIds: string[],
    finalIds: string[],
    cmsEnergy: number,
    model: TheoreticalModel,
  ): Promise<Reaction>;

  reconstructReaction(
    initialIds: string[],
    cmsEnergy: number,
    model: TheoreticalModel,
  ): Promise<ReconstructedFinalState[]>;

  // ── Feynman Diagrams ───────────────────────────────────────────────────
  generateDiagrams(
    reaction: Reaction,
    model: TheoreticalModel,
    maxLoopOrder?: number,
  ): Promise<TopologySet>;

  // ── Amplitude ──────────────────────────────────────────────────────────
  deriveAmplitude(diagram: FeynmanDiagram): Promise<AmplitudeResult>;

  deriveAmplitudeSteps(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
  ): Promise<DerivationStep[]>;

  simplifyExpression(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
    observable?: ObservableKind,
  ): Promise<SimplifiedExpressionResult>;

  verifyDimensions(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
    observable?: ObservableKind,
  ): Promise<DimensionalCheckReport>;

  exportAmplitudeLatex(diagram: FeynmanDiagram): Promise<string>;

  // ── Kinematics ─────────────────────────────────────────────────────────
  computeKinematics(
    finalMasses: number[],
    cmsEnergy: number,
    targetMass?: number,
    externalMasses?: [number, number, number, number],
  ): Promise<KinematicsReport>;

  computeDalitzData(
    motherMass: number,
    mA: number,
    mB: number,
    mC: number,
    nPoints?: number,
  ): Promise<DalitzPlotData>;

  // ── Export ─────────────────────────────────────────────────────────────
  exportModelUfo(model: TheoreticalModel): Promise<UfoExportResult>;

  // ── Analysis Pipeline ──────────────────────────────────────────────────
  runAnalysis(config: AnalysisConfig): Promise<AnalysisResult>;
  computeChiSquare(
    theoryBinContents: number[],
    theoryBinEdges: number[],
    expCsv: string,
    expLabel: string,
  ): Promise<GoodnessOfFitResult>;
  computeGlobalFit(
    observables: ObservableFitInput[],
    nParams: number,
  ): Promise<GlobalObservableFitResult>;

  // ── Scripting ──────────────────────────────────────────────────────────
  validateScript(script: string): Promise<void>;
  testObservableScript(script: string): Promise<number>;
  testCutScript(script: string): Promise<boolean>;

  // ── 3D Event Display ───────────────────────────────────────────────────
  generateDisplayEvent(
    cmsEnergy: number,
    finalMasses: number[],
    detectorPreset: string,
    particleKinds?: string[] | null,
  ): Promise<EventDisplayData>;

  generateDisplayBatch(
    cmsEnergy: number,
    finalMasses: number[],
    detectorPreset: string,
    batchSize: number,
    particleKinds?: string[] | null,
  ): Promise<EventDisplayData[]>;

  // ── Lagrangian Workbench ───────────────────────────────────────────────
  parseLagrangianTerm(
    input: string,
    knownFields: Record<string, FieldSpin>,
  ): Promise<LagrangianExpr>;

  deriveVertexRuleFromAst(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<DerivedVertexRule>;

  validateLagrangianTerm(
    input: string,
    knownFields: Record<string, FieldSpin>,
    gaugeSymmetry?: GaugeSymmetry | null,
    fieldGaugeInfo?: Record<string, FieldGaugeInfo>,
  ): Promise<ValidationResult>;

  runRgeFlow(config: RgeFlowConfig): Promise<RgeFlowResult>;

  // ── External Theory Bridge ─────────────────────────────────────────────
  importSlhaString(slhaText: string): Promise<SlhaDocument>;

  importUfoModel(
    fileContents: UfoFileContents,
    modelName: string,
  ): Promise<[UfoModel, TheoreticalModel]>;

  deriveCounterterms(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<CountertermResult>;

  // ── Parameter Scanner ──────────────────────────────────────────────────
  runParameterScan1D(config: ScanConfig1D): Promise<ScanResult1D>;
  runParameterScan2D(config: ScanConfig2D): Promise<ScanResult2D>;

  // ── Decay Calculator ──────────────────────────────────────────────────
  calculateDecayTable(model: TheoreticalModel, particleId: string): Promise<CalcDecayTable>;
  exportDecaySlha(model: TheoreticalModel, particleId: string, pdgCode: number): Promise<string>;

  configureNlo(config: NloConfig): Promise<void>;
  configureShower(config: ShowerToggleConfig): Promise<void>;

  // ── Proof Generation ──────────────────────────────────────────────────
  generateMathematicalProof(
    diagram: FeynmanDiagram,
    processLabel: string,
    dim: SpacetimeDimension,
  ): Promise<string>;

  // ── Data Provenance ───────────────────────────────────────────────────
  computeProvenanceHash(
    model: TheoreticalModel,
    reaction: Reaction | null,
    cmsEnergy: number,
    numEvents: number,
    seed: number,
  ): Promise<{ sha256: string; payload: string }>;

  loadProvenanceState(
    payload: string,
  ): Promise<Record<string, unknown>>;

  // ── Cosmological Relic Density ────────────────────────────────────────
  calculateRelicDensity(config: RelicConfig): Promise<RelicDensityReport>;

  // ── Flavor Physics ──────────────────────────────────────────────────
  calculateBMixing(lattice: LatticeInputs): Promise<BMixingResult>;

  calculateBToKll(
    q2Min: number,
    q2Max: number,
    wilsonCoeffs: WilsonCoefficients,
    lattice: LatticeInputs,
    nPoints?: number,
  ): Promise<FlavorObservableReport>;

  // ── Hardware Backend Query ──────────────────────────────────────────
  queryHardwareBackends(): Promise<{
    gpu_feature_compiled: boolean;
    gpu_adapter_available: boolean;
    adapter_name: string;
    cpu_backend: string;
    gpu_backend: string;
  }>;

  // ── Plugin System (Phase 54) ──────────────────────────────────────
  loadPlugin(path: string): Promise<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }>;

  listActivePlugins(): Promise<Array<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }>>;

  unloadPlugin(name: string): Promise<void>;

  // ── Global Fits & MCMC (Phase 55) ─────────────────────────────────
  startMcmcFit(request: McmcFitRequest): Promise<void>;
  getMcmcStatus(includeSamples: boolean): Promise<McmcFitStatus>;
  stopMcmcFit(): Promise<void>;
}
