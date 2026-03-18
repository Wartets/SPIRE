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
  Field,
  WorkspaceState,
  ObservableScript,
  CutScript,
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

export type LogLevel = "info" | "warn" | "error" | "success" | "debug";

export type LogCategory =
  | "General"
  | "Workspace"
  | "Model"
  | "Reaction"
  | "Diagram"
  | "Amplitude"
  | "Kinematics"
  | "Analysis"
  | "Atlas"
  | "Lagrangian"
  | "ExternalModels"
  | "EventDisplay"
  | "Scanner"
  | "Decay"
  | "Dalitz"
  | "ComputeGrid"
  | "Notebook"
  | "Telemetry"
  | "Cosmology"
  | "Flavor"
  | "Plugin"
  | "GlobalFit"
  | "Pipeline"
  | "ImportExport"
  | "Storage"
  | "System";

type AppendLogOptions = {
  level?: LogLevel;
  category?: LogCategory | string;
};

function normalizeCategory(value: string): LogCategory | string {
  const key = value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "");
  const aliases: Record<string, LogCategory> = {
    scanner: "Scanner",
    decaycalc: "Decay",
    decay: "Decay",
    dalitz: "Dalitz",
    atlas: "Atlas",
    reaction: "Reaction",
    model: "Model",
    workspace: "Workspace",
    lagrangian: "Lagrangian",
    analysis: "Analysis",
    amplitude: "Amplitude",
    diagram: "Diagram",
    kinematics: "Kinematics",
    eventdisplay: "EventDisplay",
    computegrid: "ComputeGrid",
    notebook: "Notebook",
    telemetry: "Telemetry",
    cosmology: "Cosmology",
    flavor: "Flavor",
    plugin: "Plugin",
    globalfit: "GlobalFit",
    pipeline: "Pipeline",
    importexport: "ImportExport",
    storage: "Storage",
    system: "System",
  };
  return aliases[key] ?? value.trim();
}

function detectLogLevel(message: string): LogLevel {
  const m = message.toLowerCase();

  if (/(^|\b)(error|failed|failure|exception|fatal|panic|invalid|erreur|échec)(\b|:)/i.test(m)) {
    return "error";
  }

  if (/(^|\b)(warn|warning|deprecated|avertissement)(\b|:)/i.test(m)) {
    return "warn";
  }

  if (/(^|\b)(debug|trace|verbose)(\b|:)/i.test(m)) {
    return "debug";
  }

  if (
    /(^|\b)(success|loaded|generated|derived|complete|completed|saved|exported|imported|restored|done)(\b|:)/i.test(
      m,
    )
  ) {
    return "success";
  }

  return "info";
}

function detectLogCategory(message: string): LogCategory | string {
  const explicitTag = message.match(/^\[([^\]]{2,32})\]/)?.[1];
  if (explicitTag) return normalizeCategory(explicitTag);

  const m = message.toLowerCase();
  const categoryRules: Array<{ category: LogCategory; test: RegExp }> = [
    { category: "Storage", test: /\b(storage|localstorage|migration|persist|restore)\b/i },
    { category: "ImportExport", test: /\b(import|export|ufo|slha|json|csv|clipboard|download)\b/i },
    { category: "Workspace", test: /\bworkspace\b/i },
    { category: "Model", test: /\bmodel|framework|vertex|field\b/i },
    { category: "Reaction", test: /\breaction|final state|initial state|reconstruct\b/i },
    { category: "Diagram", test: /\bdiagram|feynman|topolog\w*\b/i },
    { category: "Amplitude", test: /\bamplitude\b/i },
    { category: "Kinematics", test: /\bkinematic|mandelstam|threshold\b/i },
    { category: "Dalitz", test: /\bdalitz\b/i },
    { category: "Analysis", test: /\banalysis|run card|cross section\b/i },
    { category: "Lagrangian", test: /\blagrangian|gauge|counterterm|rge\b/i },
    { category: "ExternalModels", test: /\bexternal model|ufo import|slha import\b/i },
    { category: "EventDisplay", test: /\bevent display|detector|event(s)? buffered\b/i },
    { category: "Scanner", test: /\bscan|scanner|benchmark scan\b/i },
    { category: "Decay", test: /\bdecay|branching ratio|br\b/i },
    { category: "ComputeGrid", test: /\bcompute grid|distributed|worker\b/i },
    { category: "Notebook", test: /\bnotebook|cell\b/i },
    { category: "Telemetry", test: /\btelemetry|metrics|perf\w*\b/i },
    { category: "Cosmology", test: /\bcosmolog|lcdm|relic\b/i },
    { category: "Flavor", test: /\bflavor|b-mixing|r_k\b/i },
    { category: "Plugin", test: /\bplugin\b/i },
    { category: "GlobalFit", test: /\bglobal fit|mcmc|fit session\b/i },
    { category: "Pipeline", test: /\bpipeline|widget bus|synced settings\b/i },
  ];

  for (const rule of categoryRules) {
    if (rule.test.test(m)) return rule.category;
  }

  return "General";
}

/** Append a timestamped message to the log store. */
export function appendLog(
  message: string,
  options?: LogLevel | AppendLogOptions,
): void {
  const ts = new Date().toLocaleTimeString();
  const resolvedLevel =
    typeof options === "string"
      ? options
      : options?.level ?? detectLogLevel(message);
  const resolvedCategory =
    typeof options === "string"
      ? detectLogCategory(message)
      : options?.category ?? detectLogCategory(message);

  const levelTag = resolvedLevel.toUpperCase();
  const categoryTag = String(resolvedCategory || "General").trim() || "General";
  logs.update((prev) => [...prev, `[${ts}] [${levelTag}] [${categoryTag}] ${message}`]);
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

// ---------------------------------------------------------------------------
// Scripting - User-Defined Observables & Cuts
// ---------------------------------------------------------------------------

/** User-defined observable scripts. */
export const observableScripts = writable<ObservableScript[]>([]);

/** User-defined kinematic cut scripts. */
export const cutScripts = writable<CutScript[]>([]);

/**
 * Inject or upsert a custom particle field into the active theoretical model.
 *
 * - If no model is loaded, this is a no-op.
 * - If a field with the same ID already exists, it is replaced.
 * - New custom fields are marked by convention using `custom_` prefixed IDs.
 */
export function upsertModelField(field: Field): void {
  theoreticalModel.update((model) => {
    if (!model) return model;
    const existingIdx = model.fields.findIndex((f) => f.id === field.id);
    const nextFields = [...model.fields];
    if (existingIdx >= 0) {
      nextFields[existingIdx] = field;
    } else {
      nextFields.push(field);
    }
    return {
      ...model,
      fields: nextFields,
    } as TheoreticalModel;
  });
}
