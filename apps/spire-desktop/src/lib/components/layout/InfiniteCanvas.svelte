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

  // ── LOD & Frustum Culling ──
  import CanvasLODWrapper from "$lib/components/canvas/CanvasLODWrapper.svelte";
  import {
    type LodLevel,
    type ViewportRect,
    zoomToLod,
    computeViewport,
    expandByOverscan,
    isVisible,
    WIDGET_ICONS,
    WIDGET_ACCENT,
  } from "$lib/components/canvas/lodUtils";

  // ── Summary components (medium-zoom LOD) ──
  import DiagramSummary from "$lib/components/canvas/summaries/DiagramSummary.svelte";
  import AmplitudeSummary from "$lib/components/canvas/summaries/AmplitudeSummary.svelte";
  import AnalysisSummary from "$lib/components/canvas/summaries/AnalysisSummary.svelte";

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

  // ── Frustum Culling & LOD ──────────────────────────────────────────────
  // Compute the visible world-space viewport, expanded by overscan, and
  // the current LOD tier.  Widgets outside the frustum are replaced by
  // lightweight empty placeholders to save DOM nodes and GPU memory.

  /** Screen dimensions (updated via ResizeObserver). */
  let screenW = 900;
  let screenH = 600;

  $: worldViewport = expandByOverscan(
    computeViewport(panX, panY, zoom, screenW, screenH),
  );

  $: lodLevel = zoomToLod(zoom);

  /** Test whether a canvas item is within the visible frustum. */
  function itemIsVisible(item: CanvasItem): boolean {
    return isVisible(item.x, item.y, item.width, item.height, worldViewport);
  }

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
      attachWindowGrabListeners();
    }
  }

  // ── Window-level grab listeners ──
  // When the user starts a pan, drag, or resize we attach move/up
  // listeners to `window` so events keep firing even if the cursor
  // leaves the canvas boundary.

  let _grabListenersAttached = false;

  function attachWindowGrabListeners(): void {
    if (_grabListenersAttached) return;
    _grabListenersAttached = true;
    window.addEventListener("mousemove", handleCanvasMouseMove);
    window.addEventListener("mouseup", handleCanvasMouseUp);
  }

  function detachWindowGrabListeners(): void {
    if (!_grabListenersAttached) return;
    _grabListenersAttached = false;
    window.removeEventListener("mousemove", handleCanvasMouseMove);
    window.removeEventListener("mouseup", handleCanvasMouseUp);
  }

  // ── RAF-throttled store updates ──
  // During drag/resize the mouse fires much faster than the display refresh
  // rate (often 120 Hz+).  Writing the Svelte store on *every* mousemove
  // allocates a new array and triggers re-renders of ALL canvas widgets.
  // Instead we accumulate the latest desired position and flush once per
  // animation frame.

  let _rafPending = false;
  let _pendingDragPatch: { id: string; x: number; y: number } | null = null;
  let _pendingResizePatch: { id: string; width: number; height: number } | null = null;

  function flushCanvasUpdates(): void {
    _rafPending = false;
    if (_pendingDragPatch) {
      updateCanvasItem(_pendingDragPatch.id, {
        x: _pendingDragPatch.x,
        y: _pendingDragPatch.y,
      });
      _pendingDragPatch = null;
    }
    if (_pendingResizePatch) {
      updateCanvasItem(_pendingResizePatch.id, {
        width: _pendingResizePatch.width,
        height: _pendingResizePatch.height,
      });
      _pendingResizePatch = null;
    }
  }

  function scheduleCanvasFlush(): void {
    if (!_rafPending) {
      _rafPending = true;
      requestAnimationFrame(flushCanvasUpdates);
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
      // Batch — only flush to store at display refresh rate
      _pendingDragPatch = { id: dragItemId, x: newX, y: newY };
      scheduleCanvasFlush();
    }

    if (isResizing && resizeItemId) {
      const dx = (event.clientX - resizeStartX) / zoom;
      const dy = (event.clientY - resizeStartY) / zoom;
      _pendingResizePatch = {
        id: resizeItemId,
        width: Math.max(280, resizeStartW + dx),
        height: Math.max(200, resizeStartH + dy),
      };
      scheduleCanvasFlush();
    }
  }

  function handleCanvasMouseUp(): void {
    // Flush any pending RAF updates immediately so snap reads the latest state
    if (_rafPending) {
      flushCanvasUpdates();
    }

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
    detachWindowGrabListeners();
  }

  // ── Zoom (scroll wheel) ──
  // Priority-based wheel event routing:
  //
  //   1. If cursor is over a widget with an interactive <canvas> element
  //      (Three.js OrbitControls, WebGL, etc.) or an element tagged with
  //      data-wheel-capture → let the widget handle it (no canvas zoom).
  //
  //   2. If cursor is over a widget whose .cw-body content is scrollable
  //      AND the content can still scroll in the wheel direction →
  //      let the browser scroll the widget content.
  //
  //   3. Otherwise (cursor on background, or widget content is not
  //      scrollable / is at scroll boundary) → zoom the canvas.

  /**
   * Walk from `el` up to (but not including) `boundary` and return true
   * if any ancestor captures wheel events — marked via the
   * `data-wheel-capture` attribute.  Widgets whose content uses its own
   * wheel-driven interaction (Three.js OrbitControls, map viewers, etc.)
   * should place this attribute on the relevant container.
   */
  function elementCapturesWheel(el: HTMLElement | null, boundary: HTMLElement): boolean {
    let cur = el;
    while (cur && cur !== boundary) {
      if (cur.hasAttribute("data-wheel-capture")) return true;
      cur = cur.parentElement;
    }
    return false;
  }

  /**
   * Return true when the scrollable element can still move in the
   * direction indicated by deltaY.  If the user scrolls down but the
   * element is already at the bottom (or up when at the top), the event
   * should fall through to the canvas.
   */
  function canScrollInDirection(el: HTMLElement, deltaY: number): boolean {
    const tolerance = 1; // sub-pixel rounding guard
    if (deltaY < 0) {
      // Scrolling up — can scroll if not at the top
      return el.scrollTop > tolerance;
    }
    if (deltaY > 0) {
      // Scrolling down — can scroll if not at the bottom
      return el.scrollTop + el.clientHeight < el.scrollHeight - tolerance;
    }
    return false;
  }

  function handleWheel(event: WheelEvent): void {
    const target = event.target as HTMLElement | null;
    const cwBody = target?.closest(".cw-body") as HTMLElement | null;

    if (cwBody) {
      // ── Cursor is inside a widget ──

      // 1. Interactive canvas (Three.js, WebGL) or explicit capture
      if (target && elementCapturesWheel(target, cwBody)) {
        // The widget's own controller (OrbitControls, etc.) handles
        // the event.  Do NOT also zoom the canvas.
        return;
      }

      // 2. Scrollable widget content
      const hasOverflow =
        cwBody.scrollHeight > cwBody.clientHeight + 1 ||
        cwBody.scrollWidth > cwBody.clientWidth + 1;

      if (hasOverflow && canScrollInDirection(cwBody, event.deltaY)) {
        // Let the browser scroll the widget body natively.
        return;
      }

      // 3. Widget content is not scrollable (or is at scroll boundary)
      //    → fall through to canvas zoom below.
    }

    // ── Global canvas zoom (cursor on background or non-interactive widget) ──
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

  /** Bring an item to the foreground by ID (used when items are added). */
  function bringToFrontById(id: string): void {
    selectedWidgetId = id;
    zCounter += 1;
    zMap[id] = zCounter;
  }

  function zIndexOf(item: CanvasItem): number {
    return zMap[item.id] ?? 1;
  }

  // ── Auto-select newly created widgets ──
  // Track the previous item count; when it grows, bring the last item to front.
  let _prevItemCount = 0;

  const unsubCanvasItems = canvasItems.subscribe((items) => {
    if (items.length > _prevItemCount && items.length > 0) {
      const newest = items[items.length - 1];
      bringToFrontById(newest.id);
    }
    _prevItemCount = items.length;
  });

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
    attachWindowGrabListeners();
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
    attachWindowGrabListeners();
  }

  function handleClose(item: CanvasItem): void {
    removeCanvasItem(item.id);
  }

  /** Right-click on a canvas widget body — widget-specific items + close. */
  function handleWidgetBodyContext(e: MouseEvent, item: CanvasItem): void {
    e.preventDefault();
    e.stopPropagation();
    const widgetItems = getWidgetContextItems(item.widgetType);
    const items: import("$lib/types/menu").ContextMenuItem[] = [
      ...widgetItems,
      ...(widgetItems.length > 0
        ? [{ type: "separator" as const, id: "sep-cw" }]
        : []),
      { type: "action" as const, id: "cw-close", label: "Close Widget", icon: "✕", action: () => removeCanvasItem(item.id) },
    ];
    showContextMenu(e.clientX, e.clientY, items);
  }

  // ── Canvas Context Menu ──

  function handleCanvasContextMenu(event: MouseEvent): void {
    // Shift + Right-click bypasses SPIRE and opens the native browser menu
    if (event.shiftKey) return;
    event.preventDefault();
    event.stopPropagation();

    // Convert viewport coords to world coords for widget placement
    const rect = canvasEl.getBoundingClientRect();
    const worldX = (event.clientX - rect.left - panX) / zoom;
    const worldY = (event.clientY - rect.top - panY) / zoom;

    const widgetSubItems: import("$lib/types/menu").ContextMenuItem[] = WIDGET_DEFINITIONS.map((def) => ({
      type: "action" as const,
      id: `ctx-add-${def.type}`,
      label: def.label,
      action: () => addCanvasItem(def.type, worldX, worldY),
    }));

    showContextMenu(event.clientX, event.clientY, [
      { type: "submenu", id: "ctx-add-widget", label: "Add Widget", icon: "+", children: widgetSubItems },
      { type: "separator", id: "sep-canvas" },
      { type: "action", id: "ctx-reset-zoom", label: "Reset Zoom", shortcut: "Click %", action: () => resetZoom() },
      { type: "action", id: "ctx-center-view", label: "Center View", action: () => { panX = 0; panY = 0; commitViewport(); } },
    ]);
  }

  // ── Global Shortcut Event Handlers ──

  function handleDeleteSelected(): void {
    if (selectedWidgetId) {
      removeCanvasItem(selectedWidgetId);
      selectedWidgetId = null;
    }
  }

  function handleDuplicateSelected(): void {
    if (!selectedWidgetId) return;
    const items = get(canvasItems);
    const src = items.find((i) => i.id === selectedWidgetId);
    if (!src) return;
    addCanvasItem(src.widgetType, src.x + 30, src.y + 30);
  }

  function handleSelectAll(): void {
    // In single-select mode, select the first item as a starting point.
    const items = get(canvasItems);
    if (items.length > 0) {
      selectedWidgetId = items[0].id;
    }
  }

  // ── Lifecycle ──

  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("spire:canvas:delete-selected", handleDeleteSelected as EventListener);
    window.addEventListener("spire:canvas:duplicate-selected", handleDuplicateSelected as EventListener);
    window.addEventListener("spire:canvas:select-all", handleSelectAll as EventListener);

    // Track canvas element dimensions for frustum culling
    if (canvasEl) {
      screenW = canvasEl.clientWidth;
      screenH = canvasEl.clientHeight;
      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          screenW = entry.contentRect.width;
          screenH = entry.contentRect.height;
        }
      });
      resizeObserver.observe(canvasEl);
    }
  });

  onDestroy(() => {
    unsubViewport();
    unsubCanvasItems();
    resizeObserver?.disconnect();
    detachWindowGrabListeners();
    window.removeEventListener("keydown", handleKeyDown);
    window.removeEventListener("keyup", handleKeyUp);
    window.removeEventListener("spire:canvas:delete-selected", handleDeleteSelected as EventListener);
    window.removeEventListener("spire:canvas:duplicate-selected", handleDuplicateSelected as EventListener);
    window.removeEventListener("spire:canvas:select-all", handleSelectAll as EventListener);
  });
</script>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
  class="infinite-canvas"
  class:space-held={spaceHeld && !spacePanning}
  class:space-panning={spacePanning}
  class:is-panning={isPanning}
  style="--canvas-zoom: {zoom};"
  bind:this={canvasEl}
  on:mousedown={handleCanvasMouseDown}
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
      {#if itemIsVisible(item)}
        <!-- Visible widget — render at current LOD tier -->
        <div
          class="canvas-widget"
          class:cw-selected={item.id === selectedWidgetId}
          class:cw-lod-summary={lodLevel === "summary"}
          class:cw-lod-minimal={lodLevel === "minimal"}
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
          <!-- Widget header (draggable) — hidden at minimal LOD -->
          {#if lodLevel !== "minimal"}
            <header
              class="cw-header"
              on:mousedown={(e) => handleWidgetDragStart(e, item)}
              role="toolbar"
              tabindex="-1"
              aria-label="{WIDGET_LABELS[item.widgetType] ?? item.widgetType} controls"
            >
              <span class="cw-header-icon" style="color: {WIDGET_ACCENT[item.widgetType] ?? '#5eb8ff'};">
                {@html WIDGET_ICONS[item.widgetType] ?? ''}
              </span>
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
          {/if}

          <!-- Widget body -->
          <div
            class="cw-body"
            class:cw-body-minimal={lodLevel === "minimal"}
            on:mousedown={(e) => { if (lodLevel === "minimal") handleWidgetDragStart(e, item); }}
            on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
            role="region"
          >
            <CanvasLODWrapper widgetType={item.widgetType} {zoom}>
              <!-- ═══ FULL LOD (zoom ≥ 0.6): interactive widget ═══ -->
              <div slot="full" class="zoom-adaptive-text">
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

              <!-- ═══ SUMMARY LOD (0.3 ≤ zoom < 0.6): simplified card ═══ -->
              <div slot="summary">
                {#if item.widgetType === "diagram"}
                  <DiagramSummary />
                {:else if item.widgetType === "amplitude"}
                  <AmplitudeSummary />
                {:else if item.widgetType === "analysis"}
                  <AnalysisSummary />
                {:else}
                  <span style="color: var(--fg-secondary); font-size: 0.68rem; font-style: italic;">
                    {WIDGET_LABELS[item.widgetType] ?? item.widgetType}
                  </span>
                {/if}
              </div>
            </CanvasLODWrapper>
          </div>

          <!-- Resize handle (hidden at minimal LOD) -->
          {#if lodLevel !== "minimal"}
            <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
            <div
              class="cw-resize"
              on:mousedown={(e) => handleResizeStart(e, item)}
              role="separator"
              aria-label="Resize"
              tabindex="-1"
            ></div>
          {/if}
        </div>
      {:else}
        <!-- Off-screen placeholder: maintains layout geometry without DOM cost -->
        <div
          class="cw-placeholder"
          style="
            left: {item.x}px;
            top: {item.y}px;
            width: {item.width}px;
            height: {item.height}px;
          "
          aria-hidden="true"
        ></div>
      {/if}
    {/each}
  </div>

  <!-- Zoom indicator (click to reset to 100%) -->
  <button class="zoom-indicator" on:click={resetZoom} title="Reset zoom to 100%">
    {Math.round(zoom * 100)}%
    {#if lodLevel !== "full"}
      <span class="lod-badge">{lodLevel === "summary" ? "SUM" : "MIN"}</span>
    {/if}
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

  .cw-header-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }

  .cw-header-icon :global(svg) {
    width: 13px;
    height: 13px;
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

  .cw-body-minimal {
    padding: 0;
    cursor: grab;
  }

  .cw-body-minimal:active {
    cursor: grabbing;
  }

  /* ── Frustum Culling Placeholder ── */
  .cw-placeholder {
    position: absolute;
    pointer-events: none;
    /* No background — completely invisible lightweight box */
  }

  /* ── LOD Visual States ── */
  .cw-lod-summary {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }

  .cw-lod-minimal {
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    border-width: 1px;
  }

  /* ── Font Scaling Floor ──
     When the canvas is zoomed out aggressively, text rendered inside
     widgets gets multiplied by the CSS zoom and can become microscopic.
     The .zoom-adaptive-text utility applies a compensating scale so
     that no text drops below ~9px screen-space.  Below the threshold
     a translucent block replaces text to indicate content exists
     without wasting GPU cycles on illegible glyphs. */
  .zoom-adaptive-text {
    /* Content container for full-LOD widgets.  The --canvas-zoom
       variable is inherited and available to child components that
       need zoom-aware styling (e.g. font scaling floors). */
    width: 100%;
    height: 100%;
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
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .zoom-indicator:hover {
    border-color: var(--hl-symbol);
    color: var(--fg-primary);
  }

  .lod-badge {
    font-size: 0.5rem;
    padding: 0 0.2rem;
    border: 1px solid var(--hl-value);
    color: var(--hl-value);
    letter-spacing: 0.04em;
  }
</style>
