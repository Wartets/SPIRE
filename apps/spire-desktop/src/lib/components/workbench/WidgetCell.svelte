<!--
  SPIRE — Widget Cell

  Instantiates the correct inner component based on the
  WidgetInstance type, wraps it in BaseWidget, and wires
  the close event back to notebookStore.removeWidget().

  Props:
    instance — The WidgetInstance from the notebook store.
-->
<script lang="ts">
  import type { WidgetInstance } from "$lib/stores/notebookStore";
  import { removeWidget } from "$lib/stores/notebookStore";
  import { WIDGET_LABELS } from "./widgetRegistry";
  import BaseWidget from "./BaseWidget.svelte";

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
  import LogConsole from "$lib/components/LogConsole.svelte";

  export let instance: WidgetInstance;

  $: label = WIDGET_LABELS[instance.type] ?? instance.type;

  function handleClose(): void {
    removeWidget(instance.id);
  }
</script>

<BaseWidget title={label} widgetId={instance.id} on:close={handleClose}>
  {#if instance.type === "model"}
    <ModelLoader />
  {:else if instance.type === "reaction"}
    <ReactionWorkspace />
  {:else if instance.type === "diagram"}
    <DiagramVisualizer />
  {:else if instance.type === "amplitude"}
    <AmplitudePanel />
  {:else if instance.type === "kinematics"}
    <KinematicsView />
  {:else if instance.type === "dalitz"}
    <DalitzPlotter />
  {:else if instance.type === "analysis"}
    <AnalysisWidget />
  {:else if instance.type === "event_display"}
    <EventDisplay />
  {:else if instance.type === "diagram_editor"}
    <DiagramEditor />
  {:else if instance.type === "lagrangian_workbench"}
    <LagrangianWorkbench />
  {:else if instance.type === "external_models"}
    <ExternalModels />
  {:else if instance.type === "log"}
    <LogConsole />
  {:else}
    <p style="color: var(--hl-error);">Unknown widget type: {instance.type}</p>
  {/if}
</BaseWidget>
