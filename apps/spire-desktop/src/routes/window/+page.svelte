<!--
  SPIRE - Tear-Off Window Route

  Minimalist standalone page for widgets torn off into separate
  native OS windows.  Reads `widgetType` and `nodeId` from the
  URL query string, subscribes to `spire://store-sync` events
  from the main window to keep physics stores updated, and renders
  only the requested widget.

  URL format: /window?widgetType=diagram&nodeId=ln-42-abc
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { WidgetType } from "$lib/stores/notebookStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import { initTearOffSync } from "$lib/core/services/StoreSyncService";

  // ── Inner Components ──
  import ModelLoader from "$lib/components/ModelLoader.svelte";
  import ReactionWorkspace from "$lib/components/ReactionWorkspace.svelte";
  import DiagramVisualizer from "$lib/components/DiagramVisualizer.svelte";
  import AmplitudePanel from "$lib/components/AmplitudePanel.svelte";
  import KinematicsView from "$lib/components/KinematicsView.svelte";
  import DalitzPlotter from "$lib/components/DalitzPlotter.svelte";
  import AnalysisWidget from "$lib/components/AnalysisWidget.svelte";
  import EventDisplay from "$lib/components/EventDisplay.svelte";
  import DiagramEditor from "$lib/components/DiagramEditor.svelte";
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

  let widgetType: WidgetType | null = null;
  let nodeId: string = "";
  let label: string = "";
  let unlisten: (() => void) | null = null;

  // Parse query params from the current URL (no SvelteKit stores needed)
  function parseParams(): void {
    const params = new URLSearchParams(window.location.search);
    widgetType = (params.get("widgetType") as WidgetType) ?? null;
    nodeId = params.get("nodeId") ?? "";
    label = widgetType ? (WIDGET_LABELS[widgetType] ?? widgetType) : "Unknown";
  }

  onMount(async () => {
    parseParams();

    // Initialise bidirectional store sync with the main window.
    // This also sends an initial `request-sync` to receive current state.
    unlisten = await initTearOffSync();

    if (label) {
      document.title = `SPIRE - ${label}`;
    }
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<div class="tearoff-shell">
  <header class="tearoff-header">
    <span class="tearoff-logo">SPIRE</span>
    <span class="tearoff-title">{label}</span>
    <span class="tearoff-badge">Tear-Off</span>
  </header>

  <main class="tearoff-body">
    {#if widgetType === "model"}
      <ModelLoader />
    {:else if widgetType === "reaction"}
      <ReactionWorkspace />
    {:else if widgetType === "diagram"}
      <DiagramVisualizer />
    {:else if widgetType === "amplitude"}
      <AmplitudePanel />
    {:else if widgetType === "kinematics"}
      <KinematicsView />
    {:else if widgetType === "dalitz"}
      <DalitzPlotter />
    {:else if widgetType === "analysis"}
      <AnalysisWidget />
    {:else if widgetType === "event_display"}
      <EventDisplay />
    {:else if widgetType === "diagram_editor"}
      <DiagramEditor />
    {:else if widgetType === "lagrangian_workbench"}
      <LagrangianWorkbench />
    {:else if widgetType === "external_models"}
      <ExternalModels />
    {:else if widgetType === "compute_grid"}
      <ComputeGridWidget />
    {:else if widgetType === "references"}
      <ReferencesPanel />
    {:else if widgetType === "telemetry"}
      <TelemetryPanel />
    {:else if widgetType === "log"}
      <LogConsole />
    {:else if widgetType === "notebook"}
      <NotebookWidget />
    {:else if widgetType === "parameter_scanner"}
      <ParameterScanner />
    {:else if widgetType === "decay_calculator"}
      <DecayCalculator />
    {:else if widgetType === "cosmology"}
      <CosmologyPanel />
    {:else if widgetType === "flavor_workbench"}
      <FlavorWorkbench />
    {:else if widgetType === "plugin_manager"}
      <PluginManager />
    {:else}
      <div class="tearoff-error">
        <p>No widget type specified.</p>
        <p>This page is designed to be opened from the main SPIRE workspace via the Tear Off button.</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .tearoff-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary, #121212);
    color: var(--fg-primary, #e8e8e8);
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  .tearoff-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.6rem;
    background: var(--bg-inset, #0e0e0e);
    border-bottom: 1px solid var(--border, #333);
    flex-shrink: 0;
  }

  .tearoff-logo {
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--fg-accent, #fff);
  }

  .tearoff-title {
    font-size: 0.68rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary, #888);
  }

  .tearoff-badge {
    margin-left: auto;
    font-size: 0.55rem;
    padding: 0.1rem 0.3rem;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, #888);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .tearoff-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
    min-height: 0;
  }

  .tearoff-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--fg-secondary, #888);
    font-style: italic;
    font-size: 0.8rem;
    text-align: center;
    gap: 0.3rem;
  }

  .tearoff-error p {
    margin: 0;
  }
</style>
