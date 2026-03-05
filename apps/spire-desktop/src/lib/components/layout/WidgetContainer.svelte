<!--
  SPIRE — Widget Container

  Replacement for the old WidgetCell + BaseWidget combination in the
  docking system.  Renders the widget header bar with docking controls
  (split horizontal, split vertical, tear off, close) and delegates
  the inner content to the appropriate physics widget component.

  Props:
    node — The WidgetLeaf from the layout tree.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { WidgetLeaf } from "$lib/stores/layoutStore";
  import { closeNode, splitNode, moveNode } from "$lib/stores/layoutStore";
  import type { DropPosition } from "$lib/stores/layoutStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import { tearOffWidget } from "$lib/core/services/WindowManager";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { pipelineLinks } from "$lib/core/services/PipelineService";

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

  export let node: WidgetLeaf;

  $: label = WIDGET_LABELS[node.widgetType] ?? node.widgetType;
  $: linked = $pipelineLinks.some(
    (l) => l.source.widgetId === node.id || l.sink.widgetId === node.id,
  );

  function handleSplitH(): void {
    splitNode(node.id, "row", node.widgetType);
  }

  function handleSplitV(): void {
    splitNode(node.id, "col", node.widgetType);
  }

  function handleTearOff(): void {
    tearOffWidget(node.widgetType, node.id, label);
  }

  function handleClose(): void {
    closeNode(node.id);
  }

  function handleHeaderContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, [
      { id: "split-h", label: "Split Horizontal", action: handleSplitH },
      { id: "split-v", label: "Split Vertical", action: handleSplitV },
      { id: "tear-off", label: "Tear Off to Window", action: handleTearOff },
      { id: "sep-1", label: "", separator: true, action: () => {} },
      { id: "close", label: "Close Widget", action: handleClose },
    ]);
  }

  // ── Docking Drag-and-Drop ──

  let dropZone: DropPosition | null = null;

  function handleDragStart(e: DragEvent): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData("text/plain", node.id);
    e.dataTransfer.effectAllowed = "move";
  }

  function handleDragOver(e: DragEvent): void {
    e.preventDefault();
    if (!e.dataTransfer) return;
    e.dataTransfer.dropEffect = "move";

    // Compute drop zone from cursor position within the container
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const relX = (e.clientX - rect.left) / rect.width;
    const relY = (e.clientY - rect.top) / rect.height;

    const edgeThreshold = 0.25;
    if (relX < edgeThreshold) dropZone = "left";
    else if (relX > 1 - edgeThreshold) dropZone = "right";
    else if (relY < edgeThreshold) dropZone = "top";
    else if (relY > 1 - edgeThreshold) dropZone = "bottom";
    else dropZone = "center";
  }

  function handleDragLeave(): void {
    dropZone = null;
  }

  function handleDrop(e: DragEvent): void {
    e.preventDefault();
    const sourceId = e.dataTransfer?.getData("text/plain");
    if (sourceId && sourceId !== node.id && dropZone) {
      moveNode(sourceId, node.id, dropZone);
    }
    dropZone = null;
  }
</script>

<div
  class="widget-container"
  class:drop-left={dropZone === "left"}
  class:drop-right={dropZone === "right"}
  class:drop-top={dropZone === "top"}
  class:drop-bottom={dropZone === "bottom"}
  class:drop-center={dropZone === "center"}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  role="group"
>
  <header class="wc-header" on:contextmenu={handleHeaderContext}>
    <span
      class="wc-drag-handle"
      draggable="true"
      on:dragstart={handleDragStart}
      title="Drag to reorganise"
      role="button"
      tabindex="-1"
      aria-label="Drag handle"
    >⠿</span>
    <span class="wc-title">{label}</span>
    {#if linked}
      <span class="wc-link-badge" title="Connected via pipeline">⟷</span>
    {/if}
    <div class="wc-controls">
      <button
        class="wc-btn"
        on:click={handleSplitH}
        title="Split Horizontal"
        aria-label="Split Horizontal"
      >⬌</button>
      <button
        class="wc-btn"
        on:click={handleSplitV}
        title="Split Vertical"
        aria-label="Split Vertical"
      >⬍</button>
      <button
        class="wc-btn"
        on:click={handleTearOff}
        title="Tear Off to New Window"
        aria-label="Tear Off"
      >⧉</button>
      <button
        class="wc-btn wc-close"
        on:click={handleClose}
        title="Close Widget"
        aria-label="Close"
      >&times;</button>
    </div>
  </header>

  <div class="wc-body">
    {#if node.widgetType === "model"}
      <ModelLoader />
    {:else if node.widgetType === "reaction"}
      <ReactionWorkspace />
    {:else if node.widgetType === "diagram"}
      <DiagramVisualizer />
    {:else if node.widgetType === "amplitude"}
      <AmplitudePanel />
    {:else if node.widgetType === "kinematics"}
      <KinematicsView />
    {:else if node.widgetType === "dalitz"}
      <DalitzPlotter />
    {:else if node.widgetType === "analysis"}
      <AnalysisWidget />
    {:else if node.widgetType === "event_display"}
      <EventDisplay />
    {:else if node.widgetType === "diagram_editor"}
      <DiagramEditor />
    {:else if node.widgetType === "lagrangian_workbench"}
      <LagrangianWorkbench />
    {:else if node.widgetType === "external_models"}
      <ExternalModels />
    {:else if node.widgetType === "compute_grid"}
      <ComputeGridWidget />
    {:else if node.widgetType === "references"}
      <ReferencesPanel />
    {:else if node.widgetType === "telemetry"}
      <TelemetryPanel />
    {:else if node.widgetType === "log"}
      <LogConsole />
    {:else if node.widgetType === "notebook"}
      <NotebookWidget />
    {:else if node.widgetType === "parameter_scanner"}
      <ParameterScanner />
    {:else if node.widgetType === "decay_calculator"}
      <DecayCalculator />
    {:else}
      <p style="color: var(--hl-error);">Unknown widget: {node.widgetType}</p>
    {/if}
  </div>
</div>

<style>
  .widget-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .wc-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.4rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 1.5rem;
    user-select: none;
  }

  .wc-title {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .wc-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .wc-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 0 0.18rem;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .wc-btn:hover {
    color: var(--fg-accent);
    border-color: var(--border-focus);
  }

  .wc-close:hover {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }

  .wc-link-badge {
    font-size: 0.6rem;
    color: var(--hl-symbol);
    opacity: 0.8;
    flex-shrink: 0;
  }

  /* ── Drag Handle ──────────────────────────────────────────── */
  .wc-drag-handle {
    cursor: grab;
    color: var(--fg-secondary);
    font-size: 0.7rem;
    line-height: 1;
    padding: 0 0.1rem;
    flex-shrink: 0;
    user-select: none;
    opacity: 0.5;
    transition: opacity 0.1s;
  }

  .wc-drag-handle:hover {
    opacity: 1;
    color: var(--fg-accent);
  }

  .wc-drag-handle:active {
    cursor: grabbing;
  }

  /* ── Drop Zone Indicators ─────────────────────────────────── */
  .widget-container.drop-left   { border-left:   3px solid var(--hl-symbol); }
  .widget-container.drop-right  { border-right:  3px solid var(--hl-symbol); }
  .widget-container.drop-top    { border-top:    3px solid var(--hl-symbol); }
  .widget-container.drop-bottom { border-bottom: 3px solid var(--hl-symbol); }
  .widget-container.drop-center { outline: 2px dashed var(--hl-symbol); outline-offset: -2px; }

  .wc-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
    min-height: 0;
    min-width: 0;
  }

  /* ── Responsive ──────────────────────────────────────────── */
  @media (max-width: 768px) {
    .wc-header { padding: 0.1rem 0.25rem; gap: 0.15rem; }
    .wc-title { font-size: 0.6rem; }
    .wc-btn { font-size: 0.62rem; padding: 0 0.1rem; }
    .wc-body { padding: 0.25rem; }
  }
</style>
