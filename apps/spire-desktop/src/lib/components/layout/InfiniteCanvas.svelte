<!--
  SPIRE — Infinite Canvas

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
  } from "$lib/stores/layoutStore";
  import type { CanvasItem } from "$lib/stores/layoutStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";

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

  // ── Pan (middle-click) ──

  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panStartPanX = 0;
  let panStartPanY = 0;

  function handleCanvasMouseDown(event: MouseEvent): void {
    // Middle-click (button 1) or Ctrl+Left-click for pan
    if (event.button === 1 || (event.button === 0 && event.altKey)) {
      event.preventDefault();
      isPanning = true;
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
      commitViewport();
    }
    if (isDragging) {
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

  onDestroy(() => {
    unsubViewport();
  });
</script>

<div
  class="infinite-canvas"
  bind:this={canvasEl}
  on:mousedown={handleCanvasMouseDown}
  on:mousemove={handleCanvasMouseMove}
  on:mouseup={handleCanvasMouseUp}
  on:wheel={handleWheel}
  role="application"
  aria-label="Infinite Canvas Workspace"
>
  <!-- Grid dots (background pattern) -->
  <div
    class="canvas-grid"
    style="
      background-position: {panX}px {panY}px;
      background-size: {40 * zoom}px {40 * zoom}px;
    "
  ></div>

  <!-- Transform layer -->
  <div
    class="canvas-transform"
    style="transform: translate({panX}px, {panY}px) scale({zoom});"
  >
    {#each $canvasItems as item (item.id)}
      <div
        class="canvas-widget"
        style="
          left: {item.x}px;
          top: {item.y}px;
          width: {item.width}px;
          height: {item.height}px;
        "
      >
        <!-- Widget header (draggable) -->
        <header
          class="cw-header"
          on:mousedown={(e) => handleWidgetDragStart(e, item)}
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
          {:else}
            <p style="color: var(--hl-error);">Unknown: {item.widgetType}</p>
          {/if}
        </div>

        <!-- Resize handle -->
        <div
          class="cw-resize"
          on:mousedown={(e) => handleResizeStart(e, item)}
          role="separator"
          aria-label="Resize"
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

  .canvas-grid {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background-image: radial-gradient(circle, var(--border) 1px, transparent 1px);
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
