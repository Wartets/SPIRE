<!--
  SPIRE - Infinite Canvas

  Whiteboard-mode workspace.  Widgets are placed at arbitrary (x, y)
  positions on a pannable, zoomable 2D plane.

  Controls:
  - Middle-click + drag → Pan the viewport
  - Scroll wheel       → Zoom to cursor
  - Left-click header  → Drag widget to reposition
  - Corner handle      → Resize widget

  The canvas state (items, viewport) lives in `layoutStore`.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    canvasItems,
    canvasViewport,
    updateCanvasItem,
    removeCanvasItem,
    addCanvasItem,
  } from "$lib/stores/layoutStore";
  import type { CanvasItem } from "$lib/stores/layoutStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { WIDGET_DEFINITIONS } from "$lib/stores/notebookStore";

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
  import GlobalFitDashboard from "$lib/components/GlobalFitDashboard.svelte";

  let canvasEl: HTMLDivElement;

  // ── Viewport ──

  let panX = 0;
  let panY = 0;
  let zoom = 1;

  // Subscribe to store
  const unsubViewport = canvasViewport.subscribe((vp) => {
    panX = vp.panX;
    panY = vp.panY;
    zoom = vp.zoom;
  });

  function commitViewport(): void {
    canvasViewport.set({ panX, panY, zoom });
  }

  // ── Multi-Level Dot Grid ──
  // Minor dots (20px spacing) fade when zoomed out; major dots (100px) persist.
  const MINOR_GRID = 20;
  const MAJOR_GRID = 100;

  $: minorSize = MINOR_GRID * zoom;
  $: majorSize = MAJOR_GRID * zoom;
  $: minorOpacity = Math.min(1, Math.max(0, (zoom - 0.3) / 0.4));
  $: minorDotR = Math.max(0.5, 0.8 * zoom);
  $: majorDotR = Math.max(1, 1.4 * zoom);

  $: gridStyle = `
    background-position: ${panX}px ${panY}px;
    background-size:
      ${minorSize}px ${minorSize}px,
      ${majorSize}px ${majorSize}px;
    background-image:
      radial-gradient(circle, rgba(136,136,136,${minorOpacity * 0.45}) ${minorDotR}px, transparent ${minorDotR}px),
      radial-gradient(circle, rgba(170,170,170,0.55) ${majorDotR}px, transparent ${majorDotR}px);
  `;

  // ── Magnetic Snapping ──
  const SNAP_GRID = 20;

  function snapToGrid(value: number): number {
    return Math.round(value / SNAP_GRID) * SNAP_GRID;
  }

  // ── Spacebar Hold (grab-to-pan) ──

  let spaceHeld = false;
  let spacePanning = false;

  function handleKeyDown(event: KeyboardEvent): void {
    if (event.code === "Space" && !spaceHeld) {
      const tag = (event.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      event.preventDefault();
      spaceHeld = true;
    }
  }

  function handleKeyUp(event: KeyboardEvent): void {
    if (event.code === "Space") {
      spaceHeld = false;
      spacePanning = false;
    }
  }

  // ── Pan (middle-click or spacebar+left-click) ──

  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panStartPanX = 0;
  let panStartPanY = 0;

  function handleCanvasMouseDown(event: MouseEvent): void {
    // Middle-click (button 1) or Alt+Left-click or Spacebar+Left-click for pan
    if (event.button === 1 || (event.button === 0 && event.altKey) || (event.button === 0 && spaceHeld)) {
      event.preventDefault();
      isPanning = true;
      if (spaceHeld) spacePanning = true;
      panStartX = event.clientX;
      panStartY = event.clientY;
      panStartPanX = panX;
      panStartPanY = panY;
    }
  }

  function handleCanvasMouseMove(event: MouseEvent): void {
    if (isPanning) {
      panX = panStartPanX + (event.clientX - panStartX);
      panY = panStartPanY + (event.clientY - panStartY);
    }

    if (isDragging && dragItem) {
      const dx = (event.clientX - dragStartX) / zoom;
      const dy = (event.clientY - dragStartY) / zoom;
      updateCanvasItem(dragItem.id, {
        x: dragItemStartX + dx,
        y: dragItemStartY + dy,
      });
    }

    if (isResizing && resizeItem) {
      const dx = (event.clientX - resizeStartX) / zoom;
      const dy = (event.clientY - resizeStartY) / zoom;
      updateCanvasItem(resizeItem.id, {
        width: Math.max(200, resizeStartW + dx),
        height: Math.max(120, resizeStartH + dy),
      });
    }
  }

  function handleCanvasMouseUp(): void {
    if (isPanning) {
      isPanning = false;
      spacePanning = false;
      commitViewport();
    }
    if (isDragging && dragItem) {
      // Magnetic snap to grid on drop
      const snappedX = snapToGrid(dragItem.x);
      const snappedY = snapToGrid(dragItem.y);
      updateCanvasItem(dragItem.id, { x: snappedX, y: snappedY });
      isDragging = false;
      dragItem = null;
    }
    if (isResizing) {
      isResizing = false;
      resizeItem = null;
    }
  }

  // ── Zoom (scroll wheel) ──

  function handleWheel(event: WheelEvent): void {
    event.preventDefault();
    const factor = event.deltaY > 0 ? 0.92 : 1.08;
    const newZoom = Math.max(0.15, Math.min(5, zoom * factor));

    // Keep the world point under the cursor fixed
    const rect = canvasEl.getBoundingClientRect();
    const mouseX = event.clientX - rect.left;
    const mouseY = event.clientY - rect.top;

    const worldX = (mouseX - panX) / zoom;
    const worldY = (mouseY - panY) / zoom;

    panX = mouseX - worldX * newZoom;
    panY = mouseY - worldY * newZoom;
    zoom = newZoom;
    commitViewport();
  }

  // ── Widget Selection & Z-Index ──

  let selectedWidgetId: string | null = null;

  function selectWidget(item: CanvasItem): void {
    selectedWidgetId = item.id;
  }

  // ── Widget Drag (left-click on header) ──

  let isDragging = false;
  let dragItem: CanvasItem | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragItemStartX = 0;
  let dragItemStartY = 0;

  function handleWidgetDragStart(event: MouseEvent, item: CanvasItem): void {
    if (event.button !== 0) return;
    event.stopPropagation();
    selectWidget(item);
    isDragging = true;
    dragItem = item;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    dragItemStartX = item.x;
    dragItemStartY = item.y;
  }

  // ── Widget Resize (bottom-right corner) ──

  let isResizing = false;
  let resizeItem: CanvasItem | null = null;
  let resizeStartX = 0;
  let resizeStartY = 0;
  let resizeStartW = 0;
  let resizeStartH = 0;

  function handleResizeStart(event: MouseEvent, item: CanvasItem): void {
    if (event.button !== 0) return;
    event.stopPropagation();
    isResizing = true;
    resizeItem = item;
    resizeStartX = event.clientX;
    resizeStartY = event.clientY;
    resizeStartW = item.width;
    resizeStartH = item.height;
  }

  function handleClose(item: CanvasItem): void {
    removeCanvasItem(item.id);
  }

  // ── Canvas Context Menu ──

  function handleCanvasContextMenu(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    // Convert viewport coords to world coords for widget placement
    const rect = canvasEl.getBoundingClientRect();
    const worldX = (event.clientX - rect.left - panX) / zoom;
    const worldY = (event.clientY - rect.top - panY) / zoom;

    const widgetItems = WIDGET_DEFINITIONS.map((def) => ({
      id: `ctx-add-${def.type}`,
      label: `Add ${def.label}`,
      icon: "+",
      action: () => addCanvasItem(def.type, worldX, worldY),
    }));

    showContextMenu(event.clientX, event.clientY, [
      { id: "ctx-add-header", label: "Add Widget", icon: "⊞", disabled: true, action: () => {} },
      ...widgetItems,
    ]);
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
  });

  onDestroy(() => {
    unsubViewport();
    window.removeEventListener("keydown", handleKeyDown);
    window.removeEventListener("keyup", handleKeyUp);
  });
</script>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
  class="infinite-canvas"
  class:space-held={spaceHeld && !spacePanning}
  class:space-panning={spacePanning}
  bind:this={canvasEl}
  on:mousedown={handleCanvasMouseDown}
  on:mousemove={handleCanvasMouseMove}
  on:mouseup={handleCanvasMouseUp}
  on:wheel={handleWheel}
  on:contextmenu={handleCanvasContextMenu}
  role="application"
  aria-label="Infinite Canvas Workspace"
  tabindex="-1"
>
  <!-- Multi-level dot grid (minor 20px + major 100px) -->
  <div class="canvas-grid" style={gridStyle}></div>

  <!-- Transform layer -->
  <div
    class="canvas-transform"
    style="transform: translate({panX}px, {panY}px) scale({zoom});"
  >
    {#each $canvasItems as item (item.id)}
      <div
        class="canvas-widget"
        class:cw-selected={item.id === selectedWidgetId}
        style="
          left: {item.x}px;
          top: {item.y}px;
          width: {item.width}px;
          height: {item.height}px;
          z-index: {item.id === selectedWidgetId ? 50 : 1};
        "
        on:mousedown={() => selectWidget(item)}
        role="group"
        aria-label="{WIDGET_LABELS[item.widgetType] ?? item.widgetType} widget"
      >
        <!-- Widget header (draggable) -->
        <header
          class="cw-header"
          on:mousedown={(e) => handleWidgetDragStart(e, item)}
          role="toolbar"
          tabindex="-1"
          aria-label="{WIDGET_LABELS[item.widgetType] ?? item.widgetType} controls"
        >
          <span class="cw-title">
            {WIDGET_LABELS[item.widgetType] ?? item.widgetType}
          </span>
          <button
            class="cw-close"
            on:click={() => handleClose(item)}
            on:mousedown|stopPropagation
            title="Close"
          >&times;</button>
        </header>

        <!-- Widget body -->
        <div class="cw-body">
          {#if item.widgetType === "model"}
            <ModelLoader />
          {:else if item.widgetType === "reaction"}
            <ReactionWorkspace />
          {:else if item.widgetType === "diagram"}
            <DiagramVisualizer />
          {:else if item.widgetType === "amplitude"}
            <AmplitudePanel />
          {:else if item.widgetType === "kinematics"}
            <KinematicsView />
          {:else if item.widgetType === "dalitz"}
            <DalitzPlotter />
          {:else if item.widgetType === "analysis"}
            <AnalysisWidget />
          {:else if item.widgetType === "event_display"}
            <EventDisplay />
          {:else if item.widgetType === "diagram_editor"}
            <DiagramEditor />
          {:else if item.widgetType === "lagrangian_workbench"}
            <LagrangianWorkbench />
          {:else if item.widgetType === "external_models"}
            <ExternalModels />
          {:else if item.widgetType === "compute_grid"}
            <ComputeGridWidget />
          {:else if item.widgetType === "references"}
            <ReferencesPanel />
          {:else if item.widgetType === "telemetry"}
            <TelemetryPanel />
          {:else if item.widgetType === "log"}
            <LogConsole />
          {:else if item.widgetType === "notebook"}
            <NotebookWidget />
          {:else if item.widgetType === "parameter_scanner"}
            <ParameterScanner />
          {:else if item.widgetType === "decay_calculator"}
            <DecayCalculator />
          {:else if item.widgetType === "cosmology"}
            <CosmologyPanel />
          {:else if item.widgetType === "flavor_workbench"}
            <FlavorWorkbench />
          {:else if item.widgetType === "plugin_manager"}
            <PluginManager />
          {:else if item.widgetType === "global_fit_dashboard"}
            <GlobalFitDashboard />
          {:else}
            <p style="color: var(--hl-error);">Unknown: {item.widgetType}</p>
          {/if}
        </div>

        <!-- Resize handle -->
        <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
        <div
          class="cw-resize"
          on:mousedown={(e) => handleResizeStart(e, item)}
          role="separator"
          aria-label="Resize"
          tabindex="-1"
        ></div>
      </div>
    {/each}
  </div>

  <!-- Zoom indicator -->
  <div class="zoom-indicator">
    {Math.round(zoom * 100)}%
  </div>
</div>

<style>
  .infinite-canvas {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    cursor: default;
    user-select: none;
  }

  .infinite-canvas.space-held {
    cursor: grab;
  }

  .infinite-canvas.space-panning {
    cursor: grabbing;
  }

  .canvas-grid {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .canvas-transform {
    position: absolute;
    top: 0;
    left: 0;
    transform-origin: 0 0;
    pointer-events: none;
  }

  .canvas-widget {
    position: absolute;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    overflow: hidden;
    pointer-events: auto;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    transition: box-shadow 0.12s, border-color 0.12s;
  }

  .canvas-widget.cw-selected {
    border-color: var(--hl-symbol);
    box-shadow: 0 4px 16px rgba(94, 184, 255, 0.2), 0 2px 8px rgba(0, 0, 0, 0.4);
  }

  .cw-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.4rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 1.4rem;
    cursor: grab;
  }

  .cw-header:active {
    cursor: grabbing;
  }

  .cw-title {
    flex: 1;
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cw-close {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    padding: 0 0.15rem;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .cw-close:hover {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }

  .cw-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
    min-height: 0;
  }

  .cw-resize {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
    background: linear-gradient(
      135deg,
      transparent 50%,
      var(--fg-secondary) 50%,
      var(--fg-secondary) 60%,
      transparent 60%,
      transparent 70%,
      var(--fg-secondary) 70%,
      var(--fg-secondary) 80%,
      transparent 80%
    );
    opacity: 0.5;
  }

  .cw-resize:hover {
    opacity: 1;
  }

  .zoom-indicator {
    position: absolute;
    bottom: 8px;
    right: 8px;
    padding: 0.15rem 0.4rem;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    font-size: 0.62rem;
    font-family: var(--font-mono);
    pointer-events: none;
    z-index: 10;
  }
</style>
