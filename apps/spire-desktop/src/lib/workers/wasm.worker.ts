/**
 * SPIRE - WASM Compute Worker
 *
 * Dedicated Web Worker that hosts the spire-kernel WebAssembly module.
 * Receives JSON-serialised command requests from the `WasmBackend` on
 * the main thread, invokes the corresponding WASM entry point, and
 * posts back the result.
 *
 * ## Module Loading
 *
 * The WASM binary (`spire_kernel_bg.wasm`) and its JS glue are loaded
 * lazily on the first incoming request.  If the WASM artefact is not
 * available (e.g., not yet compiled), all requests fail gracefully with
 * an error response rather than crashing the worker.
 *
 * ## Message Protocol
 *
 * Inbound:  { id: number, command: string, args: Record<string, unknown> }
 * Outbound: { id: number, ok: boolean, data?: unknown, error?: string }
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface WasmRequest {
  id: number;
  command: string;
  args: Record<string, unknown>;
}

interface WasmResponse {
  id: number;
  ok: boolean;
  data?: unknown;
  error?: string;
}

// ---------------------------------------------------------------------------
// WASM Module Handle
// ---------------------------------------------------------------------------

type WasmExportFn = (jsonArgs: string) => unknown;

interface WasmExports {
  loadTheoreticalModel?: WasmExportFn;
  validateAndReconstructReaction?: WasmExportFn;
  generateFeynmanDiagrams?: WasmExportFn;
  deriveAmplitude?: WasmExportFn;
  deriveAmplitudeSteps?: WasmExportFn;
  simplifyExpression?: WasmExportFn;
  verifyDimensions?: WasmExportFn;
  exportAmplitudeLatex?: WasmExportFn;
  computeKinematics?: WasmExportFn;
  computeDalitzData?: WasmExportFn;
  exportModelUfo?: WasmExportFn;
  runAnalysis?: WasmExportFn;
  validateScript?: WasmExportFn;
  testObservableScript?: WasmExportFn;
  testCutScript?: WasmExportFn;
  generateDisplayEvent?: WasmExportFn;
  generateDisplayBatch?: WasmExportFn;
  parseLagrangianTerm?: WasmExportFn;
  deriveVertexRuleFromAst?: WasmExportFn;
  validateLagrangianTerm?: WasmExportFn;
  runRgeFlow?: WasmExportFn;
  importSlhaString?: WasmExportFn;
  importUfoModel?: WasmExportFn;
  deriveCounterterms?: WasmExportFn;
  runParameterScan1d?: WasmExportFn;
  runParameterScan2d?: WasmExportFn;
  calculateDecayTable?: WasmExportFn;
  exportDecaySlha?: WasmExportFn;
  configureNlo?: WasmExportFn;
  configureShower?: WasmExportFn;
  generateMathematicalProof?: WasmExportFn;
  computeProvenanceHash?: WasmExportFn;
  loadProvenanceState?: WasmExportFn;
  calculateRelicDensity?: WasmExportFn;
  calculateBMixing?: WasmExportFn;
  calculateBToKLl?: WasmExportFn;
}

const SUPPORTED_COMMANDS = [
  "load_theoretical_model",
  "validate_and_reconstruct_reaction",
  "generate_feynman_diagrams",
  "derive_amplitude",
  "derive_amplitude_steps",
  "simplify_expression",
  "verify_dimensions",
  "export_amplitude_latex",
  "compute_kinematics",
  "compute_dalitz_data",
  "export_model_ufo",
  "run_analysis",
  "validate_script",
  "test_observable_script",
  "test_cut_script",
  "generate_display_event",
  "generate_display_batch",
  "parse_lagrangian_term",
  "derive_vertex_rule_from_ast",
  "validate_lagrangian_term",
  "run_rge_flow",
  "import_slha_string",
  "import_ufo_model",
  "derive_counterterms",
  "run_parameter_scan_1d",
  "run_parameter_scan_2d",
  "calculate_decay_table",
  "export_decay_slha",
  "configure_nlo",
  "configure_shower",
  "generate_mathematical_proof",
  "compute_provenance_hash",
  "load_provenance_state",
  "calculate_relic_density",
  "calculate_b_mixing",
  "calculate_b_to_k_ll",
] as const;

type WasmCommand = (typeof SUPPORTED_COMMANDS)[number];

const SUPPORTED_COMMAND_SET = new Set<string>(SUPPORTED_COMMANDS);

function isWasmCommand(command: string): command is WasmCommand {
  return SUPPORTED_COMMAND_SET.has(command);
}

let wasmModule: WasmExports | null = null;
let wasmLoadAttempted = false;
let wasmLoadError: string | null = null;

/**
 * Attempt to load the spire-kernel WASM module.
 *
 * This is a best-effort loader.  If the wasm-pack artefact is not
 * present on the server, it catches the error and marks the module
 * as unavailable so subsequent calls fail fast.
 */
async function ensureWasmLoaded(): Promise<void> {
  if (wasmModule) return;
  if (wasmLoadAttempted) return;
  wasmLoadAttempted = true;

  try {
    // @ts-expect-error - spire-kernel-wasm is an optional dependency built separately
    const mod = await import(/* @vite-ignore */ "spire-kernel-wasm");
    if (typeof mod.default === "function") {
      await mod.default(); // init() for wasm-bindgen
    }
    wasmModule = mod as unknown as WasmExports;
  } catch (err) {
    wasmLoadError =
      err instanceof Error
        ? err.message
        : "Failed to load spire-kernel WASM module.";
  }
}

// ---------------------------------------------------------------------------
// Command Dispatch
// ---------------------------------------------------------------------------

/**
 * Map an IPC command name to the corresponding WASM function.
 *
 * The WASM entry points are expected to follow the naming convention
 * produced by `wasm-bindgen`: snake_case Rust → camelCase JS.
 * Each function takes a single JSON-string argument and returns a
 * JSON string.
 */
async function callWasmExport(
  command: WasmCommand,
  exportName: string,
  fn: WasmExportFn | undefined,
  args: Record<string, unknown>,
): Promise<unknown> {
  if (typeof fn !== "function") {
    throw new Error(
      `WASM module does not export function "${exportName}" (command: "${command}").`,
    );
  }

  const result = fn(JSON.stringify(args));

  // If the WASM function returns a Promise (async), await it.
  const resolved = result instanceof Promise ? await result : result;

  // If the result is a string, parse it as JSON.
  if (typeof resolved === "string") {
    try {
      return JSON.parse(resolved);
    } catch {
      return resolved;
    }
  }

  return resolved;
}

async function dispatch(command: string, args: Record<string, unknown>): Promise<unknown> {
  await ensureWasmLoaded();

  if (!wasmModule) {
    throw new Error(
      wasmLoadError ?? "WASM module is not available.",
    );
  }

  if (!isWasmCommand(command)) {
    throw new Error(`Unsupported WASM command: "${command}".`);
  }

  switch (command) {
    case "load_theoretical_model":
      return callWasmExport(command, "loadTheoreticalModel", wasmModule.loadTheoreticalModel, args);
    case "validate_and_reconstruct_reaction":
      return callWasmExport(command, "validateAndReconstructReaction", wasmModule.validateAndReconstructReaction, args);
    case "generate_feynman_diagrams":
      return callWasmExport(command, "generateFeynmanDiagrams", wasmModule.generateFeynmanDiagrams, args);
    case "derive_amplitude":
      return callWasmExport(command, "deriveAmplitude", wasmModule.deriveAmplitude, args);
    case "derive_amplitude_steps":
      return callWasmExport(command, "deriveAmplitudeSteps", wasmModule.deriveAmplitudeSteps, args);
    case "simplify_expression":
      return callWasmExport(command, "simplifyExpression", wasmModule.simplifyExpression, args);
    case "verify_dimensions":
      return callWasmExport(command, "verifyDimensions", wasmModule.verifyDimensions, args);
    case "export_amplitude_latex":
      return callWasmExport(command, "exportAmplitudeLatex", wasmModule.exportAmplitudeLatex, args);
    case "compute_kinematics":
      return callWasmExport(command, "computeKinematics", wasmModule.computeKinematics, args);
    case "compute_dalitz_data":
      return callWasmExport(command, "computeDalitzData", wasmModule.computeDalitzData, args);
    case "export_model_ufo":
      return callWasmExport(command, "exportModelUfo", wasmModule.exportModelUfo, args);
    case "run_analysis":
      return callWasmExport(command, "runAnalysis", wasmModule.runAnalysis, args);
    case "validate_script":
      return callWasmExport(command, "validateScript", wasmModule.validateScript, args);
    case "test_observable_script":
      return callWasmExport(command, "testObservableScript", wasmModule.testObservableScript, args);
    case "test_cut_script":
      return callWasmExport(command, "testCutScript", wasmModule.testCutScript, args);
    case "generate_display_event":
      return callWasmExport(command, "generateDisplayEvent", wasmModule.generateDisplayEvent, args);
    case "generate_display_batch":
      return callWasmExport(command, "generateDisplayBatch", wasmModule.generateDisplayBatch, args);
    case "parse_lagrangian_term":
      return callWasmExport(command, "parseLagrangianTerm", wasmModule.parseLagrangianTerm, args);
    case "derive_vertex_rule_from_ast":
      return callWasmExport(command, "deriveVertexRuleFromAst", wasmModule.deriveVertexRuleFromAst, args);
    case "validate_lagrangian_term":
      return callWasmExport(command, "validateLagrangianTerm", wasmModule.validateLagrangianTerm, args);
    case "run_rge_flow":
      return callWasmExport(command, "runRgeFlow", wasmModule.runRgeFlow, args);
    case "import_slha_string":
      return callWasmExport(command, "importSlhaString", wasmModule.importSlhaString, args);
    case "import_ufo_model":
      return callWasmExport(command, "importUfoModel", wasmModule.importUfoModel, args);
    case "derive_counterterms":
      return callWasmExport(command, "deriveCounterterms", wasmModule.deriveCounterterms, args);
    case "run_parameter_scan_1d":
      return callWasmExport(command, "runParameterScan1d", wasmModule.runParameterScan1d, args);
    case "run_parameter_scan_2d":
      return callWasmExport(command, "runParameterScan2d", wasmModule.runParameterScan2d, args);
    case "calculate_decay_table":
      return callWasmExport(command, "calculateDecayTable", wasmModule.calculateDecayTable, args);
    case "export_decay_slha":
      return callWasmExport(command, "exportDecaySlha", wasmModule.exportDecaySlha, args);
    case "configure_nlo":
      return callWasmExport(command, "configureNlo", wasmModule.configureNlo, args);
    case "configure_shower":
      return callWasmExport(command, "configureShower", wasmModule.configureShower, args);
    case "generate_mathematical_proof":
      return callWasmExport(command, "generateMathematicalProof", wasmModule.generateMathematicalProof, args);
    case "compute_provenance_hash":
      return callWasmExport(command, "computeProvenanceHash", wasmModule.computeProvenanceHash, args);
    case "load_provenance_state":
      return callWasmExport(command, "loadProvenanceState", wasmModule.loadProvenanceState, args);
    case "calculate_relic_density":
      return callWasmExport(command, "calculateRelicDensity", wasmModule.calculateRelicDensity, args);
    case "calculate_b_mixing":
      return callWasmExport(command, "calculateBMixing", wasmModule.calculateBMixing, args);
    case "calculate_b_to_k_ll":
      return callWasmExport(command, "calculateBToKLl", wasmModule.calculateBToKLl, args);
  }
}

// ---------------------------------------------------------------------------
// Message Handler
// ---------------------------------------------------------------------------

self.onmessage = async (event: MessageEvent<WasmRequest>) => {
  const { id, command, args } = event.data;

  try {
    const data = await dispatch(command, args);
    const response: WasmResponse = { id, ok: true, data };
    self.postMessage(response);
  } catch (err) {
    const response: WasmResponse = {
      id,
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
    self.postMessage(response);
  }
};
