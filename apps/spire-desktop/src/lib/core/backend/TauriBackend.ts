/**
 * SPIRE - Tauri IPC Backend
 *
 * Implements the `SpireBackend` interface by delegating every call to
 * the Tauri `invoke()` IPC bridge.  This is the primary backend when
 * the application runs inside the native Tauri webview, where the Rust
 * process is available as the computation host.
 *
 * All calls are thin wrappers: serialise arguments → invoke command →
 * deserialise JSON response.  Error mapping converts Tauri's string
 * errors into structured `BackendError` instances.
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

import { z } from "zod";
import {
  TheoreticalModelSchema,
  ReactionSchema,
  ReconstructedFinalStateSchema,
  TopologySetSchema,
  AmplitudeResultSchema,
  KinematicsReportSchema,
  DalitzPlotDataSchema,
  DerivationStepSchema,
  UfoExportResultSchema,
  AnalysisResultSchema,
  EventDisplayDataSchema,
  LagrangianExprSchema,
  DerivedVertexRuleSchema,
  ValidationResultSchema,
  RgeFlowResultSchema,
  SlhaDocumentSchema,
  UfoModelSchema,
  CountertermResultSchema,
  ScanResult1DSchema,
  ScanResult2DSchema,
  CalcDecayTableSchema,
  validateResponse,
} from "$lib/core/domain/schemas";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Lazy-cached reference to the Tauri `invoke` function.
 *
 * We import dynamically so that this module can be loaded (but not
 * instantiated) in non-Tauri environments without throwing at import time.
 */
let _invoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

async function getInvoke(): Promise<(cmd: string, args?: Record<string, unknown>) => Promise<unknown>> {
  if (_invoke) return _invoke;
  try {
    const mod = await import("@tauri-apps/api/tauri");
    _invoke = mod.invoke;
    return _invoke;
  } catch (err) {
    throw new BackendError(
      "transport",
      "tauri",
      "Failed to load @tauri-apps/api/tauri - is this running inside Tauri?",
      err,
    );
  }
}

/**
 * Wrap a Tauri invoke call with standardised error handling.
 */
async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = await getInvoke();
  try {
    return (await invoke(command, args)) as T;
  } catch (err) {
    throw new BackendError(
      "kernel",
      "tauri",
      typeof err === "string" ? err : `Tauri command "${command}" failed: ${String(err)}`,
      err,
    );
  }
}

/**
 * Invoke a Tauri command **and** validate the response shape with a Zod schema.
 *
 * Catches data-shape mismatches early at the IPC boundary rather than
 * letting them propagate as cryptic runtime errors deep in UI code.
 *
 * The generic parameter `T` is the declared TS return type; the schema
 * performs the runtime validation, then we cast to `T` because recursive
 * Zod schemas (CasExpr, LagrangianExpr) infer as `unknown`.
 */
async function tauriInvokeValidated<T>(
  command: string,
  schema: z.ZodTypeAny,
  args?: Record<string, unknown>,
): Promise<T> {
  const raw = await tauriInvoke<unknown>(command, args);
  return validateResponse(schema, raw, command) as T;
}

// ---------------------------------------------------------------------------
// TauriBackend
// ---------------------------------------------------------------------------

export class TauriBackend implements SpireBackend {
  public readonly kind: BackendKind = "tauri";

  async loadModel(
    particlesToml: string,
    verticesToml: string,
    modelName?: string,
  ): Promise<TheoreticalModel> {
    return tauriInvokeValidated("load_theoretical_model", TheoreticalModelSchema, {
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
    return tauriInvokeValidated("validate_and_reconstruct_reaction", ReactionSchema, {
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
    return tauriInvokeValidated(
      "validate_and_reconstruct_reaction",
      z.array(ReconstructedFinalStateSchema),
      {
        initialIds,
        cmsEnergy,
        model,
        finalIds: null,
      },
    );
  }

  async generateDiagrams(
    reaction: Reaction,
    model: TheoreticalModel,
    maxLoopOrder: number = 0,
  ): Promise<TopologySet> {
    return tauriInvokeValidated("generate_feynman_diagrams", TopologySetSchema, {
      reaction,
      model,
      maxLoopOrder: maxLoopOrder > 0 ? maxLoopOrder : null,
    });
  }

  async deriveAmplitude(diagram: FeynmanDiagram): Promise<AmplitudeResult> {
    return tauriInvokeValidated("derive_amplitude", AmplitudeResultSchema, { diagram });
  }

  async deriveAmplitudeSteps(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
  ): Promise<DerivationStep[]> {
    return tauriInvokeValidated(
      "derive_amplitude_steps",
      z.array(DerivationStepSchema),
      {
        diagram,
        dim: dim ?? { Fixed: 4 },
      },
    );
  }

  async exportAmplitudeLatex(diagram: FeynmanDiagram): Promise<string> {
    return tauriInvoke<string>("export_amplitude_latex", { diagram });
  }

  async computeKinematics(
    finalMasses: number[],
    cmsEnergy: number,
    targetMass?: number,
    externalMasses?: [number, number, number, number],
  ): Promise<KinematicsReport> {
    return tauriInvokeValidated("compute_kinematics", KinematicsReportSchema, {
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
    return tauriInvokeValidated("compute_dalitz_data", DalitzPlotDataSchema, {
      motherMass,
      mA,
      mB,
      mC,
      nPoints,
    });
  }

  async exportModelUfo(model: TheoreticalModel): Promise<UfoExportResult> {
    return tauriInvokeValidated("export_model_ufo", UfoExportResultSchema, { model });
  }

  async runAnalysis(config: AnalysisConfig): Promise<AnalysisResult> {
    return tauriInvokeValidated("run_analysis", AnalysisResultSchema, { config });
  }

  async validateScript(script: string): Promise<void> {
    return tauriInvoke<void>("validate_script", { script });
  }

  async testObservableScript(script: string): Promise<number> {
    return tauriInvoke<number>("test_observable_script", { script });
  }

  async testCutScript(script: string): Promise<boolean> {
    return tauriInvoke<boolean>("test_cut_script", { script });
  }

  async generateDisplayEvent(
    cmsEnergy: number,
    finalMasses: number[],
    detectorPreset: string,
    particleKinds?: string[] | null,
  ): Promise<EventDisplayData> {
    return tauriInvokeValidated("generate_display_event", EventDisplayDataSchema, {
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
    return tauriInvokeValidated(
      "generate_display_batch",
      z.array(EventDisplayDataSchema),
      {
        cmsEnergy,
        finalMasses,
        detectorPreset,
        particleKinds: particleKinds ?? null,
        batchSize,
      },
    );
  }

  async parseLagrangianTerm(
    input: string,
    knownFields: Record<string, FieldSpin>,
  ): Promise<LagrangianExpr> {
    return tauriInvokeValidated("parse_lagrangian_term", LagrangianExprSchema, {
      input,
      knownFields,
    });
  }

  async deriveVertexRuleFromAst(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<DerivedVertexRule> {
    return tauriInvokeValidated("derive_vertex_rule_from_ast", DerivedVertexRuleSchema, {
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
    return tauriInvokeValidated("validate_lagrangian_term", ValidationResultSchema, {
      input,
      knownFields,
      gaugeSymmetry: gaugeSymmetry ?? null,
      fieldGaugeInfo: fieldGaugeInfo ?? {},
    });
  }

  async runRgeFlow(config: RgeFlowConfig): Promise<RgeFlowResult> {
    return tauriInvokeValidated("run_rge_flow", RgeFlowResultSchema, { config });
  }

  async importSlhaString(slhaText: string): Promise<SlhaDocument> {
    return tauriInvokeValidated("import_slha_string", SlhaDocumentSchema, { slhaText });
  }

  async importUfoModel(
    fileContents: UfoFileContents,
    modelName: string,
  ): Promise<[UfoModel, TheoreticalModel]> {
    return tauriInvokeValidated(
      "import_ufo_model",
      z.tuple([UfoModelSchema, TheoreticalModelSchema]),
      {
        fileContents,
        modelName,
      },
    );
  }

  async deriveCounterterms(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<CountertermResult> {
    return tauriInvokeValidated("derive_counterterms", CountertermResultSchema, {
      input,
      knownFields,
      externalFields,
    });
  }

  async runParameterScan1D(config: ScanConfig1D): Promise<ScanResult1D> {
    return tauriInvokeValidated("run_parameter_scan_1d", ScanResult1DSchema, { config });
  }

  async runParameterScan2D(config: ScanConfig2D): Promise<ScanResult2D> {
    return tauriInvokeValidated("run_parameter_scan_2d", ScanResult2DSchema, { config });
  }

  async calculateDecayTable(model: TheoreticalModel, particleId: string): Promise<CalcDecayTable> {
    return tauriInvokeValidated("calculate_decay_table_cmd", CalcDecayTableSchema, {
      model,
      particleId,
    });
  }

  async exportDecaySlha(model: TheoreticalModel, particleId: string, pdgCode: number): Promise<string> {
    const raw = await tauriInvoke("export_decay_slha", {
      model,
      particleId,
      pdgCode,
    });
    return raw as string;
  }

  async configureNlo(config: NloConfig): Promise<void> {
    await tauriInvoke("configure_nlo", { config });
  }

  async configureShower(config: ShowerToggleConfig): Promise<void> {
    await tauriInvoke("configure_shower", { config });
  }

  async generateMathematicalProof(
    diagram: FeynmanDiagram,
    processLabel: string,
    dim: SpacetimeDimension,
  ): Promise<string> {
    return tauriInvoke("generate_mathematical_proof", {
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
    return tauriInvoke("compute_provenance_hash", {
      model,
      reaction,
      cmsEnergy,
      numEvents,
      seed,
    });
  }

  async loadProvenanceState(
    payload: string,
  ): Promise<Record<string, unknown>> {
    return tauriInvoke("load_provenance_state", { payload });
  }

  async calculateRelicDensity(config: RelicConfig): Promise<RelicDensityReport> {
    return tauriInvoke("calculate_relic_density", { config });
  }

  async calculateBMixing(lattice: LatticeInputs): Promise<BMixingResult> {
    return tauriInvoke("calculate_b_mixing", { lattice });
  }

  async calculateBToKll(
    q2Min: number,
    q2Max: number,
    wilsonCoeffs: WilsonCoefficients,
    lattice: LatticeInputs,
    nPoints?: number,
  ): Promise<FlavorObservableReport> {
    return tauriInvoke("calculate_b_to_k_ll", {
      q2Min,
      q2Max,
      wilsonCoeffs,
      lattice,
      nPoints: nPoints ?? 100,
    });
  }

  async queryHardwareBackends(): Promise<{
    gpu_feature_compiled: boolean;
    gpu_adapter_available: boolean;
    adapter_name: string;
    cpu_backend: string;
    gpu_backend: string;
  }> {
    return tauriInvoke("query_hardware_backends", {});
  }

  async loadPlugin(path: string): Promise<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }> {
    return tauriInvoke("load_plugin", { path });
  }

  async listActivePlugins(): Promise<Array<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }>> {
    return tauriInvoke("list_active_plugins", {});
  }

  async unloadPlugin(name: string): Promise<void> {
    return tauriInvoke("unload_plugin", { name });
  }
}
