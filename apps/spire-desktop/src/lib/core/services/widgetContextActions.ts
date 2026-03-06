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
import { menuAction, menuSeparator } from "$lib/types/menu";
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

// ---------------------------------------------------------------------------
// Widget-specific context menu builders
// ---------------------------------------------------------------------------

/** Build context menu items for the given widget type. */
export function getWidgetContextItems(widgetType: WidgetType): ContextMenuItem[] {
  switch (widgetType) {
    case "model":
      return [
        menuAction("model-sm", "Load Standard Model", () => activeFramework.set("StandardModel"), { icon: "⚛" }),
        menuAction("model-qed", "Load QED", () => activeFramework.set("QED"), { icon: "γ" }),
        menuAction("model-qcd", "Load QCD", () => activeFramework.set("QCD"), { icon: "g" }),
      ];

    case "reaction":
      return [
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
        menuAction("rxn-sqrt-91", "√s = 91.2 GeV (Z pole)", () => cmsEnergyInput.set(91.1876), { icon: "⚡" }),
        menuAction("rxn-sqrt-250", "√s = 250 GeV", () => cmsEnergyInput.set(250), { icon: "⚡" }),
        menuAction("rxn-sqrt-13000", "√s = 13 TeV (LHC)", () => cmsEnergyInput.set(13000), { icon: "⚡" }),
      ];

    case "diagram":
      return [
        menuAction("diag-count", `Diagrams: ${get(generatedDiagrams)?.diagrams?.length ?? 0}`, () => {}, { disabled: true }),
      ];

    case "amplitude":
      return [
        menuAction("amp-copy", "Copy Amplitude Expression", () => {
          const expr = get(activeAmplitude);
          if (expr) navigator.clipboard.writeText(expr);
        }, { disabled: !get(activeAmplitude) }),
      ];

    case "kinematics":
      return [
        menuAction("kin-info", `√s = ${get(cmsEnergyInput)} GeV`, () => {}, { disabled: true }),
      ];

    case "dalitz":
      return [
        menuAction("dalitz-info", "Dalitz Plot Settings", () => {}, { disabled: true }),
      ];

    case "analysis":
      return [
        menuAction("ana-obs-count", `Observables: ${get(observableScripts).length}`, () => {}, { disabled: true }),
        menuAction("ana-cut-count", `Cuts: ${get(cutScripts).length}`, () => {}, { disabled: true }),
      ];

    case "event_display":
      return [
        menuAction("evd-cms-91", "Set √s = 91.2 GeV", () => cmsEnergyInput.set(91.1876), { icon: "⚡" }),
        menuAction("evd-cms-13000", "Set √s = 13 TeV", () => cmsEnergyInput.set(13000), { icon: "⚡" }),
      ];

    case "compute_grid":
      return [
        menuAction("cg-1k", "1,000 events", () => appendLog("Set n_events = 1,000")),
        menuAction("cg-10k", "10,000 events", () => appendLog("Set n_events = 10,000")),
        menuAction("cg-100k", "100,000 events", () => appendLog("Set n_events = 100,000")),
      ];

    case "log":
      return [
        menuAction("log-clear", "Clear Log", () => logs.set([])),
        menuAction("log-copy", "Copy All", () => {
          const allLogs = get(logs);
          navigator.clipboard.writeText(allLogs.join("\n"));
        }),
      ];

    case "notebook":
      return [
        menuAction("nb-info", "Notebook (Rhai scripting)", () => {}, { disabled: true }),
      ];

    case "parameter_scanner":
      return [
        menuAction("scan-info", "Parameter Scanner", () => {}, { disabled: true }),
      ];

    case "references":
      return [
        menuAction("ref-info", "Auto-generated references", () => {}, { disabled: true }),
      ];

    case "telemetry":
      return [
        menuAction("telem-info", "Performance Profiling", () => {}, { disabled: true }),
      ];

    default:
      return [];
  }
}
