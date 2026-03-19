/**
 * SPIRE - WebAssembly Backend
 *
 * Implements the `SpireBackend` interface by dispatching computation
 * requests to the `spire-kernel` WebAssembly module.  In a standard
 * browser (without Tauri), this provides full computational capability
 * by running the Rust physics engine compiled to `wasm32-unknown-unknown`.
 *
 * ## Architecture
 *
 * Requests are serialized as JSON and posted to a dedicated Web Worker
 * that hosts the WASM module.  The worker deserializes the request,
 * invokes the corresponding Rust entry point, and posts back the JSON
 * result.  This keeps the main thread free for UI rendering.
 *
 * If the WASM module is not yet compiled or loaded, calls fall back
 * to a `BackendError` with kind `"unsupported"`.
 */

import type { SpireBackend } from "./BackendInterface";
import { BackendError } from "./BackendInterface";
import type { BackendKind } from "./BackendInterface";

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
} from "$lib/types/spire";

// ---------------------------------------------------------------------------
// WASM Worker Message Protocol
// ---------------------------------------------------------------------------

/** Inbound message to the WASM worker. */
interface WasmRequest {
  id: number;
  command: string;
  args: Record<string, unknown>;
}

/** Outbound message from the WASM worker. */
interface WasmResponse {
  id: number;
  ok: boolean;
  data?: unknown;
  error?: string;
}

// ---------------------------------------------------------------------------
// WasmBackend
// ---------------------------------------------------------------------------

export class WasmBackend implements SpireBackend {
  public readonly kind: BackendKind = "wasm";

  private worker: Worker | null = null;
  private nextId = 1;
  private pending = new Map<number, {
    resolve: (value: unknown) => void;
    reject: (reason: unknown) => void;
  }>();

  constructor() {
    this.initWorker();
  }

  // ── Worker Lifecycle ─────────────────────────────────────────────────

  private initWorker(): void {
    try {
      this.worker = new Worker(
        new URL("$lib/workers/wasm.worker.ts", import.meta.url),
        { type: "module" },
      );
      this.worker.onmessage = (event: MessageEvent<WasmResponse>) => {
        const { id, ok, data, error } = event.data;
        const entry = this.pending.get(id);
        if (!entry) return;
        this.pending.delete(id);
        if (ok) {
          entry.resolve(data);
        } else {
          entry.reject(
            new BackendError("kernel", "wasm", error ?? "Unknown WASM error"),
          );
        }
      };
      this.worker.onerror = (err) => {
        // Reject all pending requests on catastrophic worker failure.
        const msg = `WASM worker error: ${err.message}`;
        for (const [, entry] of this.pending) {
          entry.reject(new BackendError("transport", "wasm", msg));
        }
        this.pending.clear();
      };
    } catch {
      // Worker creation failed - calls will throw "unsupported".
      this.worker = null;
    }
  }

  /**
   * Send a command to the WASM worker and await its response.
   */
  private call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
    if (!this.worker) {
      return Promise.reject(
        new BackendError(
          "unsupported",
          "wasm",
          `WASM worker is not available. Command "${command}" cannot be executed.`,
        ),
      );
    }

    const id = this.nextId++;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: resolve as (v: unknown) => void,
        reject,
      });
      const msg: WasmRequest = { id, command, args };
      this.worker!.postMessage(msg);
    });
  }

  // ── SpireBackend Implementation ────────────────────────────────────

  async loadModel(
    particlesToml: string,
    verticesToml: string,
    modelName?: string,
  ): Promise<TheoreticalModel> {
    return this.call<TheoreticalModel>("load_theoretical_model", {
      particlesToml,
      verticesToml,
      modelName: modelName ?? null,
    });
  }

  async constructReaction(
    initialIds: string[],
    finalIds: string[],
    cmsEnergy: number,
    model: TheoreticalModel,
  ): Promise<Reaction> {
    return this.call<Reaction>("validate_and_reconstruct_reaction", {
      initialIds,
      cmsEnergy,
      model,
      finalIds,
    });
  }

  async reconstructReaction(
    initialIds: string[],
    cmsEnergy: number,
    model: TheoreticalModel,
  ): Promise<ReconstructedFinalState[]> {
    return this.call<ReconstructedFinalState[]>("validate_and_reconstruct_reaction", {
      initialIds,
      cmsEnergy,
      model,
      finalIds: null,
    });
  }

  async generateDiagrams(
    reaction: Reaction,
    model: TheoreticalModel,
    maxLoopOrder: number = 0,
  ): Promise<TopologySet> {
    return this.call<TopologySet>("generate_feynman_diagrams", {
      reaction,
      model,
      maxLoopOrder: maxLoopOrder > 0 ? maxLoopOrder : null,
    });
  }

  async deriveAmplitude(diagram: FeynmanDiagram): Promise<AmplitudeResult> {
    return this.call<AmplitudeResult>("derive_amplitude", { diagram });
  }

  async deriveAmplitudeSteps(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
  ): Promise<DerivationStep[]> {
    return this.call<DerivationStep[]>("derive_amplitude_steps", {
      diagram,
      dim: dim ?? { Fixed: 4 },
    });
  }

  async simplifyExpression(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
    observable?: ObservableKind,
  ): Promise<SimplifiedExpressionResult> {
    return this.call<SimplifiedExpressionResult>("simplify_expression", {
      diagram,
      dim: dim ?? { Fixed: 4 },
      observable: observable ?? "Amplitude",
    });
  }

  async verifyDimensions(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
    observable?: ObservableKind,
  ): Promise<DimensionalCheckReport> {
    return this.call<DimensionalCheckReport>("verify_dimensions", {
      diagram,
      dim: dim ?? { Fixed: 4 },
      observable: observable ?? "Amplitude",
    });
  }

  async exportAmplitudeLatex(diagram: FeynmanDiagram): Promise<string> {
    return this.call<string>("export_amplitude_latex", { diagram });
  }

  async computeKinematics(
    finalMasses: number[],
    cmsEnergy: number,
    targetMass?: number,
    externalMasses?: [number, number, number, number],
  ): Promise<KinematicsReport> {
    return this.call<KinematicsReport>("compute_kinematics", {
      finalMasses,
      cmsEnergy,
      targetMass: targetMass ?? null,
      externalMasses: externalMasses ?? null,
    });
  }

  async computeDalitzData(
    motherMass: number,
    mA: number,
    mB: number,
    mC: number,
    nPoints: number = 3000,
  ): Promise<DalitzPlotData> {
    return this.call<DalitzPlotData>("compute_dalitz_data", {
      motherMass,
      mA,
      mB,
      mC,
      nPoints,
    });
  }

  async exportModelUfo(model: TheoreticalModel): Promise<UfoExportResult> {
    return this.call<UfoExportResult>("export_model_ufo", { model });
  }

  async runAnalysis(config: AnalysisConfig): Promise<AnalysisResult> {
    return this.call<AnalysisResult>("run_analysis", { config });
  }

  async validateScript(script: string): Promise<void> {
    return this.call<void>("validate_script", { script });
  }

  async testObservableScript(script: string): Promise<number> {
    return this.call<number>("test_observable_script", { script });
  }

  async testCutScript(script: string): Promise<boolean> {
    return this.call<boolean>("test_cut_script", { script });
  }

  async generateDisplayEvent(
    cmsEnergy: number,
    finalMasses: number[],
    detectorPreset: string,
    particleKinds?: string[] | null,
  ): Promise<EventDisplayData> {
    return this.call<EventDisplayData>("generate_display_event", {
      cmsEnergy,
      finalMasses,
      detectorPreset,
      particleKinds: particleKinds ?? null,
    });
  }

  async generateDisplayBatch(
    cmsEnergy: number,
    finalMasses: number[],
    detectorPreset: string,
    batchSize: number,
    particleKinds?: string[] | null,
  ): Promise<EventDisplayData[]> {
    return this.call<EventDisplayData[]>("generate_display_batch", {
      cmsEnergy,
      finalMasses,
      detectorPreset,
      particleKinds: particleKinds ?? null,
      batchSize,
    });
  }

  async parseLagrangianTerm(
    input: string,
    knownFields: Record<string, FieldSpin>,
  ): Promise<LagrangianExpr> {
    return this.call<LagrangianExpr>("parse_lagrangian_term", {
      input,
      knownFields,
    });
  }

  async deriveVertexRuleFromAst(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<DerivedVertexRule> {
    return this.call<DerivedVertexRule>("derive_vertex_rule_from_ast", {
      input,
      knownFields,
      externalFields,
    });
  }

  async validateLagrangianTerm(
    input: string,
    knownFields: Record<string, FieldSpin>,
    gaugeSymmetry?: GaugeSymmetry | null,
    fieldGaugeInfo?: Record<string, FieldGaugeInfo>,
  ): Promise<ValidationResult> {
    return this.call<ValidationResult>("validate_lagrangian_term", {
      input,
      knownFields,
      gaugeSymmetry: gaugeSymmetry ?? null,
      fieldGaugeInfo: fieldGaugeInfo ?? {},
    });
  }

  async runRgeFlow(config: RgeFlowConfig): Promise<RgeFlowResult> {
    return this.call<RgeFlowResult>("run_rge_flow", { config });
  }

  async importSlhaString(slhaText: string): Promise<SlhaDocument> {
    return this.call<SlhaDocument>("import_slha_string", { slhaText });
  }

  async importUfoModel(
    fileContents: UfoFileContents,
    modelName: string,
  ): Promise<[UfoModel, TheoreticalModel]> {
    return this.call<[UfoModel, TheoreticalModel]>("import_ufo_model", {
      fileContents,
      modelName,
    });
  }

  async deriveCounterterms(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<CountertermResult> {
    return this.call<CountertermResult>("derive_counterterms", {
      input,
      knownFields,
      externalFields,
    });
  }

  async runParameterScan1D(config: ScanConfig1D): Promise<ScanResult1D> {
    return this.call<ScanResult1D>("run_parameter_scan_1d", { config });
  }

  async runParameterScan2D(config: ScanConfig2D): Promise<ScanResult2D> {
    return this.call<ScanResult2D>("run_parameter_scan_2d", { config });
  }

  async calculateDecayTable(model: TheoreticalModel, particleId: string): Promise<CalcDecayTable> {
    return this.call<CalcDecayTable>("calculate_decay_table", { model, particle_id: particleId });
  }

  async exportDecaySlha(model: TheoreticalModel, particleId: string, pdgCode: number): Promise<string> {
    return this.call<string>("export_decay_slha", { model, particle_id: particleId, pdg_code: pdgCode });
  }

  async configureNlo(config: NloConfig): Promise<void> {
    await this.call<void>("configure_nlo", { config });
  }

  async configureShower(config: ShowerToggleConfig): Promise<void> {
    await this.call<void>("configure_shower", { config });
  }

  async generateMathematicalProof(
    diagram: FeynmanDiagram,
    processLabel: string,
    dim: SpacetimeDimension,
  ): Promise<string> {
    return this.call<string>("generate_mathematical_proof", {
      diagram,
      processLabel,
      dim,
    });
  }

  async computeProvenanceHash(
    model: TheoreticalModel,
    reaction: Reaction | null,
    cmsEnergy: number,
    numEvents: number,
    seed: number,
  ): Promise<{ sha256: string; payload: string }> {
    return this.call<{ sha256: string; payload: string }>("compute_provenance_hash", {
      model,
      reaction,
      cms_energy: cmsEnergy,
      num_events: numEvents,
      seed,
    });
  }

  async loadProvenanceState(
    payload: string,
  ): Promise<Record<string, unknown>> {
    return this.call<Record<string, unknown>>("load_provenance_state", { payload });
  }

  async calculateRelicDensity(config: RelicConfig): Promise<RelicDensityReport> {
    return this.call<RelicDensityReport>("calculate_relic_density", { config });
  }

  async calculateBMixing(lattice: LatticeInputs): Promise<BMixingResult> {
    return this.call<BMixingResult>("calculate_b_mixing", { lattice });
  }

  async calculateBToKll(
    q2Min: number,
    q2Max: number,
    wilsonCoeffs: WilsonCoefficients,
    lattice: LatticeInputs,
    nPoints?: number,
  ): Promise<FlavorObservableReport> {
    return this.call<FlavorObservableReport>("calculate_b_to_k_ll", {
      q2_min: q2Min,
      q2_max: q2Max,
      wilson_coeffs: wilsonCoeffs,
      lattice,
      n_points: nPoints ?? 100,
    });
  }

  async queryHardwareBackends(): Promise<{
    gpu_feature_compiled: boolean;
    gpu_adapter_available: boolean;
    adapter_name: string;
    cpu_backend: string;
    gpu_backend: string;
  }> {
    // WASM backend runs in the browser - GPU is never available via
    // the spire-kernel compute pipeline (browser WebGPU is separate).
    return {
      gpu_feature_compiled: false,
      gpu_adapter_available: false,
      adapter_name: "",
      cpu_backend: "CPU (WASM single-thread)",
      gpu_backend: "GPU (not available in WASM mode)",
    };
  }

  async loadPlugin(_path: string): Promise<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }> {
    throw new Error("Plugin loading is not supported in WASM backend mode");
  }

  async listActivePlugins(): Promise<Array<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }>> {
    return [];
  }

  async unloadPlugin(_name: string): Promise<void> {
    throw new Error("Plugin unloading is not supported in WASM backend mode");
  }

  async startMcmcFit(_request: import("$lib/types/spire").McmcFitRequest): Promise<void> {
    throw new Error("MCMC fits are not supported in WASM backend mode");
  }

  async getMcmcStatus(_includeSamples: boolean): Promise<import("$lib/types/spire").McmcFitStatus> {
    throw new Error("MCMC fits are not supported in WASM backend mode");
  }

  async stopMcmcFit(): Promise<void> {
    throw new Error("MCMC fits are not supported in WASM backend mode");
  }
}
