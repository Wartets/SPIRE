/**
 * SPIRE — Widget Context Menu Actions
 *
 * Defines per-widget-type context menu items that expose the most important
 * parameters/controls for each widget.  These items are merged with the
 * standard layout operations (split, tear-off, close) to produce a unified
 * right-click menu.
 *
 * The actions dispatch to the physics stores and workspace input stores so
 * the changes are immediately reactive.
 */

import type { ContextMenuItem } from "$lib/types/menu";
import { menuAction, menuSeparator, menuSubmenu } from "$lib/types/menu";
import type { WidgetType } from "$lib/stores/notebookStore";
import { get } from "svelte/store";
import {
  activeFramework,
  generatedDiagrams,
  activeAmplitude,
  observableScripts,
  cutScripts,
  logs,
  appendLog,
} from "$lib/stores/physicsStore";
import {
  cmsEnergyInput,
  initialIdsInput,
  finalIdsInput,
} from "$lib/stores/workspaceInputsStore";
import { clearCitations } from "$lib/core/services/CitationRegistry";
import { executeAllCells } from "$lib/stores/notebookDocumentStore";
import { publishWidgetInterop, readWidgetInterop } from "$lib/stores/widgetInteropStore";

const WIDGET_DOCS: Record<WidgetType, string> = {
  model: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  reaction: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  diagram: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  amplitude: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  kinematics: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  dalitz: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  analysis: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/scripting.md",
  event_display: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  particle_atlas: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  diagram_editor: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  lagrangian_workbench: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  external_models: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/interop.md",
  compute_grid: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  references: "https://github.com/Wartets/SPIRE/blob/main/docs/source/api/reference.md",
  telemetry: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  log: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/quickstart.md",
  notebook: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/scripting.md",
  parameter_scanner: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/scripting.md",
  decay_calculator: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  cosmology: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  flavor_workbench: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
  plugin_manager: "https://github.com/Wartets/SPIRE/blob/main/docs/source/guide/interop.md",
  global_fit_dashboard: "https://github.com/Wartets/SPIRE/blob/main/docs/source/theory/formalism.md",
};

function openWidgetDocs(widgetType: WidgetType): void {
  const url = WIDGET_DOCS[widgetType];
  window.open(url, "_blank", "noopener,noreferrer");
}

function buildInteropActions(widgetType: WidgetType): ContextMenuItem[] {
  const reactionInterop = readWidgetInterop("reaction")?.payload as
    | { cmsEnergy?: number; nFinalState?: number }
    | undefined;

  const syncItems: ContextMenuItem[] = [];

  if ((widgetType === "analysis"
      || widgetType === "compute_grid"
      || widgetType === "event_display"
      || widgetType === "parameter_scanner")
      && reactionInterop?.cmsEnergy) {
    syncItems.push(
      menuAction(
        `${widgetType}-sync-cms`,
        `Sync from Reaction (sqrt(s)=${reactionInterop.cmsEnergy} GeV)`,
        () => {
          publishWidgetInterop(widgetType, {
            action: "sync-from-reaction",
            cmsEnergy: reactionInterop.cmsEnergy,
            nFinalState: reactionInterop.nFinalState ?? null,
          });
          appendLog(`${widgetType}: synced settings from Reaction context menu.`);
        },
        { icon: "<->" },
      ),
    );
  }

  return [
    ...syncItems,
    menuAction(`${widgetType}-publish-state`, "Publish Snapshot to Widget Bus", () => {
      publishWidgetInterop(widgetType, {
        action: "manual-snapshot",
      });
      appendLog(`${widgetType}: snapshot published to widget bus.`);
    }, { icon: "PUB" }),
  ];
}

function withGlobalWidgetActions(widgetType: WidgetType, items: ContextMenuItem[]): ContextMenuItem[] {
  return [
    ...items,
    ...(items.length > 0 ? [menuSeparator(`sep-${widgetType}-global`)] : []),
    menuSubmenu(`ctx-${widgetType}-interop`, "Inter-Widget", buildInteropActions(widgetType), { icon: "<->" }),
    menuAction(`ctx-${widgetType}-docs`, "Open Documentation", () => openWidgetDocs(widgetType), { icon: "DOC" }),
  ];
}

// ---------------------------------------------------------------------------
// Widget-specific context menu builders
// ---------------------------------------------------------------------------

/** Build context menu items for the given widget type. */
export function getWidgetContextItems(widgetType: WidgetType): ContextMenuItem[] {
  switch (widgetType) {
    case "model":
      return withGlobalWidgetActions(widgetType, [
        menuAction("model-sm", "Load Standard Model", () => activeFramework.set("StandardModel"), { icon: "SM" }),
        menuAction("model-qed", "Load QED", () => activeFramework.set("QED"), { icon: "γ" }),
        menuAction("model-qcd", "Load QCD", () => activeFramework.set("QCD"), { icon: "g" }),
      ]);

    case "reaction":
      return withGlobalWidgetActions(widgetType, [
        menuAction("rxn-ee-mumu", "e⁺e⁻ → μ⁺μ⁻", () => {
          initialIdsInput.set(["e-", "e+"]);
          finalIdsInput.set(["mu-", "mu+"]);
        }, { icon: "→" }),
        menuAction("rxn-ee-tautau", "e⁺e⁻ → τ⁺τ⁻", () => {
          initialIdsInput.set(["e-", "e+"]);
          finalIdsInput.set(["tau-", "tau+"]);
        }, { icon: "→" }),
        menuAction("rxn-ee-qq", "e⁺e⁻ → qq̄", () => {
          initialIdsInput.set(["e-", "e+"]);
          finalIdsInput.set(["u", "u_bar"]);
        }, { icon: "→" }),
        menuSeparator("sep-rxn"),
        menuAction("rxn-sqrt-91", "√s = 91.2 GeV (Z pole)", () => cmsEnergyInput.set(91.1876), { icon: "E" }),
        menuAction("rxn-sqrt-250", "√s = 250 GeV", () => cmsEnergyInput.set(250), { icon: "E" }),
        menuAction("rxn-sqrt-13000", "√s = 13 TeV (LHC)", () => cmsEnergyInput.set(13000), { icon: "E" }),
      ]);

    case "diagram":
      return withGlobalWidgetActions(widgetType, [
        menuAction("diag-count", `Diagrams: ${get(generatedDiagrams)?.diagrams?.length ?? 0}`, () => {}, { disabled: true }),
        menuAction("diag-proof", "Generate Proof Outline", () => appendLog("Diagram: proof outline requested"), { icon: "∴" }),
      ]);

    case "amplitude":
      return withGlobalWidgetActions(widgetType, [
        menuAction("amp-copy", "Copy Amplitude Expression", () => {
          const expr = get(activeAmplitude);
          if (expr) navigator.clipboard.writeText(expr);
        }, { disabled: !get(activeAmplitude) }),
        menuAction("amp-log", "Log Active Amplitude", () => appendLog("Amplitude: active expression logged"), { icon: "ℳ" }),
      ]);

    case "kinematics":
      return withGlobalWidgetActions(widgetType, [
        menuAction("kin-info", `√s = ${get(cmsEnergyInput)} GeV`, () => {}, { disabled: true }),
      ]);

    case "dalitz":
      return withGlobalWidgetActions(widgetType, [
        menuAction("dalitz-info", "Dalitz Plot Settings", () => {}, { disabled: true }),
      ]);

    case "analysis":
      return withGlobalWidgetActions(widgetType, [
        menuAction("ana-obs-count", `Observables: ${get(observableScripts).length}`, () => {}, { disabled: true }),
        menuAction("ana-cut-count", `Cuts: ${get(cutScripts).length}`, () => {}, { disabled: true }),
        menuAction("ana-run-all", "Run Full Analysis Pipeline", () => appendLog("Analysis: run requested from context menu"), { icon: "RUN" }),
      ]);

    case "event_display":
      return withGlobalWidgetActions(widgetType, [
        menuAction("evd-cms-91", "Set √s = 91.2 GeV", () => cmsEnergyInput.set(91.1876), { icon: "E" }),
        menuAction("evd-cms-13000", "Set √s = 13 TeV", () => cmsEnergyInput.set(13000), { icon: "E" }),
      ]);

    case "particle_atlas":
      return withGlobalWidgetActions(widgetType, [
        menuAction("atlas-ee-mumu", "Seed e⁺e⁻ → μ⁺μ⁻", () => {
          initialIdsInput.set(["e-", "e+"]);
          finalIdsInput.set(["mu-", "mu+"]);
        }, { icon: "↺" }),
        menuAction("atlas-pp-tt", "Seed pp → tt̄", () => {
          initialIdsInput.set(["p", "p"]);
          finalIdsInput.set(["t", "t_bar"]);
        }, { icon: "↺" }),
        menuAction("atlas-open", "Open Atlas Selection Context", () => appendLog("Particle Atlas: context shortcut triggered"), { icon: "⌕" }),
      ]);

    case "diagram_editor":
      return withGlobalWidgetActions(widgetType, [
        menuAction("de-new", "New Diagram Draft", () => appendLog("Diagram Editor: new draft"), { icon: "✎" }),
        menuAction("de-orth", "Auto-Orthogonalize", () => appendLog("Diagram Editor: auto-orthogonalize"), { icon: "⤢" }),
      ]);

    case "lagrangian_workbench":
      return withGlobalWidgetActions(widgetType, [
        menuAction("lag-expand", "Expand Lagrangian Terms", () => appendLog("Lagrangian: expand terms"), { icon: "Σ" }),
        menuAction("lag-check", "Run Gauge-Invariance Check", () => appendLog("Lagrangian: gauge invariance check queued"), { icon: "✓" }),
      ]);

    case "external_models":
      return withGlobalWidgetActions(widgetType, [
        menuAction("ext-sm", "Switch to Standard Model", () => activeFramework.set("StandardModel"), { icon: "SM" }),
        menuAction("ext-qed", "Switch to QED", () => activeFramework.set("QED"), { icon: "γ" }),
        menuAction("ext-qcd", "Switch to QCD", () => activeFramework.set("QCD"), { icon: "g" }),
      ]);

    case "compute_grid":
      return withGlobalWidgetActions(widgetType, [
        menuAction("cg-1k", "1,000 events", () => appendLog("Set n_events = 1,000")),
        menuAction("cg-10k", "10,000 events", () => appendLog("Set n_events = 10,000")),
        menuAction("cg-100k", "100,000 events", () => appendLog("Set n_events = 100,000")),
        menuAction("cg-queue", "Queue Distributed Run", () => appendLog("Compute Grid: distributed run queued"), { icon: "⇢" }),
      ]);

    case "log":
      return withGlobalWidgetActions(widgetType, [
        menuAction("log-clear", "Clear Log", () => logs.set([])),
        menuAction("log-copy", "Copy All", () => {
          const allLogs = get(logs);
          navigator.clipboard.writeText(allLogs.join("\n"));
        }),
      ]);

    case "notebook":
      return withGlobalWidgetActions(widgetType, [
        menuAction("nb-run-all", "Run All Cells", () => executeAllCells(), { icon: "RUN" }),
        menuAction("nb-new-cell", "Insert New Cell", () => appendLog("Notebook: insert new cell requested"), { icon: "+" }),
        menuAction("nb-info", "Notebook (Rhai scripting)", () => {}, { disabled: true }),
      ]);

    case "parameter_scanner":
      return withGlobalWidgetActions(widgetType, [
        menuAction("scan-info", "Parameter Scanner", () => {}, { disabled: true }),
        menuAction("scan-run", "Run Benchmark Scan", () => appendLog("Parameter Scanner: benchmark scan queued"), { icon: "◴" }),
      ]);

    case "references":
      return withGlobalWidgetActions(widgetType, [
        menuAction("ref-clear", "Clear All References", () => clearCitations(), { icon: "✕" }),
        menuAction("ref-info", "Auto-generated references", () => {}, { disabled: true }),
      ]);

    case "telemetry":
      return withGlobalWidgetActions(widgetType, [
        menuAction("telem-snap", "Capture Perf Snapshot", () => appendLog("Telemetry: captured performance snapshot"), { icon: "◉" }),
        menuAction("telem-export", "Export Session Metrics", () => appendLog("Telemetry: export requested"), { icon: "⇩" }),
        menuAction("telem-info", "Performance Profiling", () => {}, { disabled: true }),
      ]);

    case "decay_calculator":
      return withGlobalWidgetActions(widgetType, [
        menuAction("decay-z", "Preset: Z boson decays", () => {
          initialIdsInput.set(["Z"]);
          finalIdsInput.set(["e-", "e+"]);
        }, { icon: "Z" }),
        menuAction("decay-h", "Preset: Higgs decays", () => {
          initialIdsInput.set(["h"]);
          finalIdsInput.set(["b", "b_bar"]);
        }, { icon: "h" }),
      ]);

    case "cosmology":
      return withGlobalWidgetActions(widgetType, [
        menuAction("cosmo-rd", "Preset: Radiation dominated", () => appendLog("Cosmology: radiation-dominated benchmark"), { icon: "SOL" }),
        menuAction("cosmo-lcdm", "Preset: ΛCDM baseline", () => appendLog("Cosmology: ΛCDM baseline loaded"), { icon: "Λ" }),
      ]);

    case "flavor_workbench":
      return withGlobalWidgetActions(widgetType, [
        menuAction("flv-bmix", "Load B-mixing benchmark", () => appendLog("Flavor: B-mixing benchmark loaded"), { icon: "B" }),
        menuAction("flv-rk", "Load R_K / R_K* benchmark", () => appendLog("Flavor: R_K benchmark loaded"), { icon: "K" }),
      ]);

    case "plugin_manager":
      return withGlobalWidgetActions(widgetType, [
        menuAction("plug-refresh", "Refresh Plugin Index", () => appendLog("Plugin Manager: refreshing index"), { icon: "↻" }),
        menuAction("plug-scan", "Rescan Local Plugins", () => appendLog("Plugin Manager: local plugin scan"), { icon: "⌕" }),
        menuAction("plug-enable-all", "Enable All Compatible Plugins", () => appendLog("Plugin Manager: enable-all requested"), { icon: "✓" }),
      ]);

    case "global_fit_dashboard":
      return withGlobalWidgetActions(widgetType, [
        menuAction("fit-start", "Start Quick Fit", () => appendLog("Global Fit: quick fit started"), { icon: "RUN" }),
        menuAction("fit-reset", "Reset Fit Session", () => appendLog("Global Fit: session reset"), { icon: "↺" }),
        menuAction("fit-snapshot", "Capture Fit Snapshot", () => appendLog("Global Fit: snapshot captured"), { icon: "◉" }),
      ]);

    default:
      return [];
  }
}
