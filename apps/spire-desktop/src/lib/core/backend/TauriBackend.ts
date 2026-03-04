/**
 * SPIRE — Tauri IPC Backend
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
} from "$lib/types/spire";

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
      "Failed to load @tauri-apps/api/tauri — is this running inside Tauri?",
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
    return tauriInvoke<TheoreticalModel>("load_theoretical_model", {
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
    return tauriInvoke<Reaction>("validate_and_reconstruct_reaction", {
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
    return tauriInvoke<ReconstructedFinalState[]>("validate_and_reconstruct_reaction", {
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
    return tauriInvoke<TopologySet>("generate_feynman_diagrams", {
      reaction,
      model,
      maxLoopOrder: maxLoopOrder > 0 ? maxLoopOrder : null,
    });
  }

  async deriveAmplitude(diagram: FeynmanDiagram): Promise<AmplitudeResult> {
    return tauriInvoke<AmplitudeResult>("derive_amplitude", { diagram });
  }

  async deriveAmplitudeSteps(
    diagram: FeynmanDiagram,
    dim?: SpacetimeDimension,
  ): Promise<DerivationStep[]> {
    return tauriInvoke<DerivationStep[]>("derive_amplitude_steps", {
      diagram,
      dim: dim ?? { Fixed: 4 },
    });
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
    return tauriInvoke<KinematicsReport>("compute_kinematics", {
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
    return tauriInvoke<DalitzPlotData>("compute_dalitz_data", {
      motherMass,
      mA,
      mB,
      mC,
      nPoints,
    });
  }

  async exportModelUfo(model: TheoreticalModel): Promise<UfoExportResult> {
    return tauriInvoke<UfoExportResult>("export_model_ufo", { model });
  }

  async runAnalysis(config: AnalysisConfig): Promise<AnalysisResult> {
    return tauriInvoke<AnalysisResult>("run_analysis", { config });
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
    return tauriInvoke<EventDisplayData>("generate_display_event", {
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
    return tauriInvoke<EventDisplayData[]>("generate_display_batch", {
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
    return tauriInvoke<LagrangianExpr>("parse_lagrangian_term", {
      input,
      knownFields,
    });
  }

  async deriveVertexRuleFromAst(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<DerivedVertexRule> {
    return tauriInvoke<DerivedVertexRule>("derive_vertex_rule_from_ast", {
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
    return tauriInvoke<ValidationResult>("validate_lagrangian_term", {
      input,
      knownFields,
      gaugeSymmetry: gaugeSymmetry ?? null,
      fieldGaugeInfo: fieldGaugeInfo ?? {},
    });
  }

  async runRgeFlow(config: RgeFlowConfig): Promise<RgeFlowResult> {
    return tauriInvoke<RgeFlowResult>("run_rge_flow", { config });
  }

  async importSlhaString(slhaText: string): Promise<SlhaDocument> {
    return tauriInvoke<SlhaDocument>("import_slha_string", { slhaText });
  }

  async importUfoModel(
    fileContents: UfoFileContents,
    modelName: string,
  ): Promise<[UfoModel, TheoreticalModel]> {
    return tauriInvoke<[UfoModel, TheoreticalModel]>("import_ufo_model", {
      fileContents,
      modelName,
    });
  }

  async deriveCounterterms(
    input: string,
    knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<CountertermResult> {
    return tauriInvoke<CountertermResult>("derive_counterterms", {
      input,
      knownFields,
      externalFields,
    });
  }
}
