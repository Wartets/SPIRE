import type { WidgetType } from "$lib/stores/notebookStore";
type WidgetComponent = new (...args: any[]) => any;


import ModelLoader from "$lib/components/ModelLoader.svelte";
import ReactionWorkspace from "$lib/components/ReactionWorkspace.svelte";
import DiagramVisualizer from "$lib/components/DiagramVisualizer.svelte";
import AmplitudePanel from "$lib/components/AmplitudePanel.svelte";
import KinematicsView from "$lib/components/KinematicsView.svelte";
import DalitzPlotter from "$lib/components/DalitzPlotter.svelte";
import AnalysisWidget from "$lib/components/AnalysisWidget.svelte";
import EventDisplay from "$lib/components/EventDisplay.svelte";
import ParticleAtlas from "$lib/components/ParticleAtlas.svelte";
import LagrangianWorkbench from "$lib/components/LagrangianWorkbench.svelte";
import ExternalModels from "$lib/components/ExternalModels.svelte";
import ComputeGridWidget from "$lib/components/ComputeGridWidget.svelte";
import ReferencesPanel from "$lib/components/ReferencesPanel.svelte";
import TelemetryPanel from "$lib/components/TelemetryPanel.svelte";
import LogConsole from "$lib/components/LogConsole.svelte";
import NotebookWidget from "$lib/components/notebook/NotebookWidget.svelte";
import ParameterScanner from "$lib/components/ParameterScanner.svelte";
import DecayCalculator from "$lib/components/DecayCalculator.svelte";
import CosmologyPanel from "$lib/components/CosmologyPanel.svelte";
import FlavorWorkbench from "$lib/components/FlavorWorkbench.svelte";
import PluginManager from "$lib/components/PluginManager.svelte";
import GlobalFitDashboard from "$lib/components/GlobalFitDashboard.svelte";

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

/** Dynamic component map for full widget rendering. */
export const WIDGET_COMPONENTS: Record<WidgetType, WidgetComponent> = {
  model: ModelLoader,
  reaction: ReactionWorkspace,
  diagram: DiagramVisualizer,
  amplitude: AmplitudePanel,
  kinematics: KinematicsView,
  dalitz: DalitzPlotter,
  analysis: AnalysisWidget,
  event_display: EventDisplay,
  particle_atlas: ParticleAtlas,
  diagram_editor: DiagramVisualizer,
  lagrangian_workbench: LagrangianWorkbench,
  external_models: ExternalModels,
  compute_grid: ComputeGridWidget,
  references: ReferencesPanel,
  telemetry: TelemetryPanel,
  log: LogConsole,
  notebook: NotebookWidget,
  parameter_scanner: ParameterScanner,
  decay_calculator: DecayCalculator,
  cosmology: CosmologyPanel,
  flavor_workbench: FlavorWorkbench,
  plugin_manager: PluginManager,
  global_fit_dashboard: GlobalFitDashboard,
};

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
  return (WIDGET_COMPONENTS as Record<string, WidgetComponent>)[widgetType] ?? null;
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
