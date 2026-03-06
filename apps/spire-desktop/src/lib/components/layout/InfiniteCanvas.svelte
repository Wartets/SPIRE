<!--
  SPIRE - Infinite Canvas

  Whiteboard-mode workspace.  Widgets are placed at arbitrary (x, y)
  positions on a pannable, zoomable 2D plane.

  Controls:
  - Left-click + drag on background → Pan the viewport
  - Middle-click + drag             → Pan the viewport
  - Scroll wheel on background      → Zoom to cursor
  - Left-click on widget header     → Drag widget to reposition
  - Corner handle                   → Resize widget
  - Click on zoom indicator         → Reset to 100%

  The canvas state (items, viewport) lives in `layoutStore`.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
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
  import { getWidgetContextItems } from "$lib/core/services/widgetContextActions";

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

  // ── Edge Snapping ──────────────────────────────────────────────────────
  // When dragging, snap widget edges to sibling widget edges and to the
  // visible viewport edges.  Threshold in world-space pixels.
  const SNAP_THRESHOLD = 8;

  interface SnapResult { x: number; y: number; }

  function snapToEdges(
    dragId: string,
    x: number,
    y: number,
    w: number,
    h: number,
  ): SnapResult {
    let sx = x;
    let sy = y;
    let bestDx = SNAP_THRESHOLD + 1;
    let bestDy = SNAP_THRESHOLD + 1;

    // Visible viewport in world coords
    const rect = canvasEl?.getBoundingClientRect();
    const vpLeft = -panX / zoom;
    const vpTop = -panY / zoom;
    const vpRight = vpLeft + (rect?.width ?? 900) / zoom;
    const vpBottom = vpTop + (rect?.height ?? 600) / zoom;

    // Candidate edges: viewport bounds
    const xEdges = [vpLeft, vpRight];
    const yEdges = [vpTop, vpBottom];

    // Sibling widgets
    const items = get(canvasItems);
    for (const sib of items) {
      if (sib.id === dragId) continue;
      xEdges.push(sib.x, sib.x + sib.width);
      yEdges.push(sib.y, sib.y + sib.height);
    }

    // Snap left/right edges of dragged widget
    for (const edge of xEdges) {
      const dLeft = Math.abs(x - edge);
      const dRight = Math.abs(x + w - edge);
      if (dLeft < bestDx) { bestDx = dLeft; sx = edge; }
      if (dRight < bestDx) { bestDx = dRight; sx = edge - w; }
    }

    // Snap top/bottom edges of dragged widget
    for (const edge of yEdges) {
      const dTop = Math.abs(y - edge);
      const dBottom = Math.abs(y + h - edge);
      if (dTop < bestDy) { bestDy = dTop; sy = edge; }
      if (dBottom < bestDy) { bestDy = dBottom; sy = edge - h; }
    }

    return { x: sx, y: sy };
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

  // ── Pan ──
  // Left-click on background, middle-click anywhere, or spacebar+left.

  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let panStartPanX = 0;
  let panStartPanY = 0;

  /** Check whether a mousedown target is the canvas background (not a widget). */
  function isCanvasBackground(target: EventTarget | null): boolean {
    if (!target || !(target instanceof HTMLElement)) return false;
    // The canvas background elements have these classes
    return (
      target.classList.contains("infinite-canvas") ||
      target.classList.contains("canvas-grid") ||
      target.classList.contains("canvas-transform")
    );
  }

  function handleCanvasMouseDown(event: MouseEvent): void {
    // Middle-click (button 1) always pans
    const middleClick = event.button === 1;
    // Left-click pans when on background, or with Alt/Space modifier
    const leftOnBg =
      event.button === 0 &&
      (event.altKey || spaceHeld || isCanvasBackground(event.target));

    if (middleClick || leftOnBg) {
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

    if (isDragging && dragItemId) {
      const dx = (event.clientX - dragStartX) / zoom;
      const dy = (event.clientY - dragStartY) / zoom;
      const newX = dragItemStartX + dx;
      const newY = dragItemStartY + dy;
      // Track live position for snapping in mouseUp
      dragCurrentX = newX;
      dragCurrentY = newY;
      updateCanvasItem(dragItemId, { x: newX, y: newY });
    }

    if (isResizing && resizeItemId) {
      const dx = (event.clientX - resizeStartX) / zoom;
      const dy = (event.clientY - resizeStartY) / zoom;
      updateCanvasItem(resizeItemId, {
        width: Math.max(280, resizeStartW + dx),
        height: Math.max(200, resizeStartH + dy),
      });
    }
  }

  function handleCanvasMouseUp(): void {
    if (isPanning) {
      isPanning = false;
      spacePanning = false;
      commitViewport();
    }
    if (isDragging && dragItemId) {
      // Edge snap on drop
      const current = get(canvasItems).find((i) => i.id === dragItemId);
      if (current) {
        const snapped = snapToEdges(
          dragItemId,
          dragCurrentX,
          dragCurrentY,
          current.width,
          current.height,
        );
        updateCanvasItem(dragItemId, { x: snapped.x, y: snapped.y });
      }
      isDragging = false;
      dragItemId = null;
    }
    if (isResizing) {
      isResizing = false;
      resizeItemId = null;
    }
  }

  // ── Zoom (scroll wheel) ──
  // Only zoom when scrolling on the canvas background. Inside a widget
  // body the wheel event should scroll the widget content normally.

  function handleWheel(event: WheelEvent): void {
    // Let the wheel scroll inside widget content
    const target = event.target as HTMLElement | null;
    if (target?.closest(".cw-body")) return;

    event.preventDefault();
    const factor = event.deltaY > 0 ? 0.92 : 1.08;
    const newZoom = Math.max(0.15, Math.min(5, zoom * factor));

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
  let zCounter = 10;
  let zMap: Record<string, number> = {};

  function selectWidget(item: CanvasItem): void {
    selectedWidgetId = item.id;
    zCounter += 1;
    zMap[item.id] = zCounter;
  }

  function zIndexOf(item: CanvasItem): number {
    return zMap[item.id] ?? 1;
  }

  function resetZoom(): void {
    zoom = 1;
    commitViewport();
  }

  // ── Widget Drag (left-click on header) ──

  let isDragging = false;
  let dragItemId: string | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragItemStartX = 0;
  let dragItemStartY = 0;
  let dragCurrentX = 0;
  let dragCurrentY = 0;

  function handleWidgetDragStart(event: MouseEvent, item: CanvasItem): void {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    selectWidget(item);
    isDragging = true;
    dragItemId = item.id;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    dragItemStartX = item.x;
    dragItemStartY = item.y;
    dragCurrentX = item.x;
    dragCurrentY = item.y;
  }

  // ── Widget Resize (bottom-right corner) ──

  let isResizing = false;
  let resizeItemId: string | null = null;
  let resizeStartX = 0;
  let resizeStartY = 0;
  let resizeStartW = 0;
  let resizeStartH = 0;

  function handleResizeStart(event: MouseEvent, item: CanvasItem): void {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    isResizing = true;
    resizeItemId = item.id;
    resizeStartX = event.clientX;
    resizeStartY = event.clientY;
    resizeStartW = item.width;
    resizeStartH = item.height;
  }

  function handleClose(item: CanvasItem): void {
    removeCanvasItem(item.id);
  }

  /** Right-click on a canvas widget body — widget-specific items + close. */
  function handleWidgetBodyContext(e: MouseEvent, item: CanvasItem): void {
    e.preventDefault();
    e.stopPropagation();
    const widgetItems = getWidgetContextItems(item.widgetType);
    const items = [
      ...widgetItems,
      ...(widgetItems.length > 0
        ? [{ id: "sep-cw", label: "", separator: true, action: () => {} }]
        : []),
      { id: "cw-close", label: "Close Widget", icon: "✕", action: () => removeCanvasItem(item.id) },
    ];
    showContextMenu(e.clientX, e.clientY, items);
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

  // ── Lifecycle ──

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
  class:is-panning={isPanning}
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
          z-index: {zIndexOf(item)};
        "
        on:mousedown={() => selectWidget(item)}
        on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
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
        <div class="cw-body" on:contextmenu={(e) => handleWidgetBodyContext(e, item)} role="region">
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

  <!-- Zoom indicator (click to reset to 100%) -->
  <button class="zoom-indicator" on:click={resetZoom} title="Reset zoom to 100%">
    {Math.round(zoom * 100)}%
  </button>
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

  .infinite-canvas.space-panning,
  .infinite-canvas.is-panning {
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
    cursor: pointer;
    z-index: 10;
    transition: border-color 0.15s, color 0.15s;
  }

  .zoom-indicator:hover {
    border-color: var(--hl-symbol);
    color: var(--fg-primary);
  }
</style>
