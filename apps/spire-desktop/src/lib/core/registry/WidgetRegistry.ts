import type { WidgetType } from "$lib/stores/notebookStore";
type WidgetComponent = new (...args: any[]) => any;

import DiagramSummary from "$lib/components/canvas/summaries/DiagramSummary.svelte";
import AmplitudeSummary from "$lib/components/canvas/summaries/AmplitudeSummary.svelte";
import AnalysisSummary from "$lib/components/canvas/summaries/AnalysisSummary.svelte";

/** Human-readable labels for each widget type. */
export const WIDGET_LABELS: Record<WidgetType, string> = {
  model: "Model Loader",
  reaction: "Reaction Workspace",
  diagram: "Diagram Visualizer",
  amplitude: "Amplitude Panel",
  kinematics: "Kinematics",
  dalitz: "Dalitz Plot",
  analysis: "Analysis",
  event_display: "Event Display",
  particle_atlas: "Particle Atlas",
  diagram_editor: "Unified Diagram Visualizer",
  lagrangian_workbench: "Lagrangian Workbench",
  external_models: "External Models",
  compute_grid: "Compute Grid",
  references: "References",
  telemetry: "Telemetry",
  log: "Console",
  notebook: "Notebook",
  parameter_scanner: "Parameter Scanner",
  decay_calculator: "Decay Calculator",
  cosmology: "Cosmology",
  flavor_workbench: "Flavor Workbench",
  plugin_manager: "Plugin Manager",
  global_fit_dashboard: "Global Fit Dashboard",
};

type WidgetLoader = () => Promise<{ default: WidgetComponent }>;

/** Lazy component loaders for full widget rendering. */
export const WIDGET_COMPONENT_LOADERS: Record<WidgetType, WidgetLoader> = {
  model: () => import("$lib/components/ModelLoader.svelte"),
  reaction: () => import("$lib/components/ReactionWorkspace.svelte"),
  diagram: () => import("$lib/components/DiagramVisualizer.svelte"),
  amplitude: () => import("$lib/components/AmplitudePanel.svelte"),
  kinematics: () => import("$lib/components/KinematicsView.svelte"),
  dalitz: () => import("$lib/components/DalitzPlotter.svelte"),
  analysis: () => import("$lib/components/AnalysisWidget.svelte"),
  event_display: () => import("$lib/components/EventDisplay.svelte"),
  particle_atlas: () => import("$lib/components/ParticleAtlas.svelte"),
  diagram_editor: () => import("$lib/components/DiagramVisualizer.svelte"),
  lagrangian_workbench: () => import("$lib/components/LagrangianWorkbench.svelte"),
  external_models: () => import("$lib/components/ExternalModels.svelte"),
  compute_grid: () => import("$lib/components/ComputeGridWidget.svelte"),
  references: () => import("$lib/components/ReferencesPanel.svelte"),
  telemetry: () => import("$lib/components/TelemetryPanel.svelte"),
  log: () => import("$lib/components/LogConsole.svelte"),
  notebook: () => import("$lib/components/notebook/NotebookWidget.svelte"),
  parameter_scanner: () => import("$lib/components/ParameterScanner.svelte"),
  decay_calculator: () => import("$lib/components/DecayCalculator.svelte"),
  cosmology: () => import("$lib/components/CosmologyPanel.svelte"),
  flavor_workbench: () => import("$lib/components/FlavorWorkbench.svelte"),
  plugin_manager: () => import("$lib/components/PluginManager.svelte"),
  global_fit_dashboard: () => import("$lib/components/GlobalFitDashboard.svelte"),
};

const widgetComponentCache = new Map<WidgetType, WidgetComponent>();
const widgetComponentInflight = new Map<WidgetType, Promise<WidgetComponent | null>>();

/** Optional low-detail summary components for medium zoom LOD in canvas mode. */
export const WIDGET_SUMMARY_COMPONENTS: Partial<Record<WidgetType, WidgetComponent>> = {
  diagram: DiagramSummary,
  amplitude: AmplitudeSummary,
  analysis: AnalysisSummary,
};

const FALLBACK_LABEL = "Unknown Widget";

/**
 * Resolve a human-readable label for a widget type.
 *
 * @param widgetType - Runtime widget type key.
 * @returns Display label, or a stable fallback for unknown types.
 */
export function getWidgetLabel(widgetType: string): string {
  return (WIDGET_LABELS as Record<string, string>)[widgetType] ?? FALLBACK_LABEL;
}

/**
 * Resolve the full-detail Svelte component for a widget type.
 *
 * @param widgetType - Runtime widget type key.
 * @returns Component constructor, or `null` when not registered.
 */
export function getWidgetComponent(widgetType: string): WidgetComponent | null {
  return widgetComponentCache.get(widgetType as WidgetType) ?? null;
}

/**
 * Resolve the full-detail Svelte component asynchronously.
 */
export async function loadWidgetComponent(widgetType: string): Promise<WidgetComponent | null> {
  const typed = widgetType as WidgetType;
  const cached = widgetComponentCache.get(typed);
  if (cached) return cached;

  const loader = (WIDGET_COMPONENT_LOADERS as Record<string, WidgetLoader>)[widgetType];
  if (!loader) return null;

  const inflight = widgetComponentInflight.get(typed);
  if (inflight) return inflight;

  const next = loader()
    .then((mod) => {
      const component = mod.default ?? null;
      if (component) widgetComponentCache.set(typed, component);
      return component;
    })
    .catch(() => null)
    .finally(() => {
      widgetComponentInflight.delete(typed);
    });

  widgetComponentInflight.set(typed, next);
  return next;
}

/**
 * Resolve the optional summary component used for medium-zoom canvas LOD.
 *
 * @param widgetType - Runtime widget type key.
 * @returns Summary component constructor, or `null` when unavailable.
 */
export function getWidgetSummaryComponent(widgetType: string): WidgetComponent | null {
  return (WIDGET_SUMMARY_COMPONENTS as Record<string, WidgetComponent | undefined>)[widgetType] ?? null;
}
