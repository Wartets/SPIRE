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

import type { ContextMenuItem } from "$lib/stores/contextMenuStore";
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
        {
          id: "model-sm",
          label: "Load Standard Model",
          icon: "⚛",
          action: () => activeFramework.set("StandardModel"),
        },
        {
          id: "model-qed",
          label: "Load QED",
          icon: "γ",
          action: () => activeFramework.set("QED"),
        },
        {
          id: "model-qcd",
          label: "Load QCD",
          icon: "g",
          action: () => activeFramework.set("QCD"),
        },
      ];

    case "reaction":
      return [
        {
          id: "rxn-ee-mumu",
          label: "e⁺e⁻ → μ⁺μ⁻",
          icon: "→",
          action: () => {
            initialIdsInput.set(["e-", "e+"]);
            finalIdsInput.set(["mu-", "mu+"]);
          },
        },
        {
          id: "rxn-ee-tautau",
          label: "e⁺e⁻ → τ⁺τ⁻",
          icon: "→",
          action: () => {
            initialIdsInput.set(["e-", "e+"]);
            finalIdsInput.set(["tau-", "tau+"]);
          },
        },
        {
          id: "rxn-ee-qq",
          label: "e⁺e⁻ → qq̄",
          icon: "→",
          action: () => {
            initialIdsInput.set(["e-", "e+"]);
            finalIdsInput.set(["u", "u_bar"]);
          },
        },
        { id: "sep-rxn", label: "", separator: true, action: () => {} },
        {
          id: "rxn-sqrt-91",
          label: "√s = 91.2 GeV (Z pole)",
          icon: "⚡",
          action: () => cmsEnergyInput.set(91.1876),
        },
        {
          id: "rxn-sqrt-250",
          label: "√s = 250 GeV",
          icon: "⚡",
          action: () => cmsEnergyInput.set(250),
        },
        {
          id: "rxn-sqrt-13000",
          label: "√s = 13 TeV (LHC)",
          icon: "⚡",
          action: () => cmsEnergyInput.set(13000),
        },
      ];

    case "diagram":
      return [
        {
          id: "diag-count",
          label: `Diagrams: ${get(generatedDiagrams)?.diagrams?.length ?? 0}`,
          icon: "📊",
          disabled: true,
          action: () => {},
        },
      ];

    case "amplitude":
      return [
        {
          id: "amp-copy",
          label: "Copy Amplitude Expression",
          icon: "📋",
          disabled: !get(activeAmplitude),
          action: () => {
            const expr = get(activeAmplitude);
            if (expr) navigator.clipboard.writeText(expr);
          },
        },
      ];

    case "kinematics":
      return [
        {
          id: "kin-info",
          label: `√s = ${get(cmsEnergyInput)} GeV`,
          icon: "📐",
          disabled: true,
          action: () => {},
        },
      ];

    case "dalitz":
      return [
        {
          id: "dalitz-info",
          label: "Dalitz Plot Settings",
          icon: "📈",
          disabled: true,
          action: () => {},
        },
      ];

    case "analysis":
      return [
        {
          id: "ana-obs-count",
          label: `Observables: ${get(observableScripts).length}`,
          icon: "📊",
          disabled: true,
          action: () => {},
        },
        {
          id: "ana-cut-count",
          label: `Cuts: ${get(cutScripts).length}`,
          icon: "✂",
          disabled: true,
          action: () => {},
        },
      ];

    case "event_display":
      return [
        {
          id: "evd-cms-91",
          label: "Set √s = 91.2 GeV",
          icon: "⚡",
          action: () => cmsEnergyInput.set(91.1876),
        },
        {
          id: "evd-cms-13000",
          label: "Set √s = 13 TeV",
          icon: "⚡",
          action: () => cmsEnergyInput.set(13000),
        },
      ];

    case "compute_grid":
      return [
        {
          id: "cg-1k",
          label: "1,000 events",
          icon: "🔢",
          action: () => appendLog("Set n_events = 1,000"),
        },
        {
          id: "cg-10k",
          label: "10,000 events",
          icon: "🔢",
          action: () => appendLog("Set n_events = 10,000"),
        },
        {
          id: "cg-100k",
          label: "100,000 events",
          icon: "🔢",
          action: () => appendLog("Set n_events = 100,000"),
        },
      ];

    case "log":
      return [
        {
          id: "log-clear",
          label: "Clear Log",
          icon: "🧹",
          action: () => logs.set([]),
        },
        {
          id: "log-copy",
          label: "Copy All",
          icon: "📋",
          action: () => {
            const allLogs = get(logs);
            navigator.clipboard.writeText(allLogs.join("\n"));
          },
        },
      ];

    case "notebook":
      return [
        {
          id: "nb-info",
          label: "Notebook (Rhai scripting)",
          icon: "📓",
          disabled: true,
          action: () => {},
        },
      ];

    case "parameter_scanner":
      return [
        {
          id: "scan-info",
          label: "Parameter Scanner",
          icon: "📉",
          disabled: true,
          action: () => {},
        },
      ];

    case "references":
      return [
        {
          id: "ref-info",
          label: "Auto-generated references",
          icon: "📚",
          disabled: true,
          action: () => {},
        },
      ];

    case "telemetry":
      return [
        {
          id: "telem-info",
          label: "Performance Profiling",
          icon: "⏱",
          disabled: true,
          action: () => {},
        },
      ];

    default:
      return [];
  }
}
