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
  import { tooltip } from "$lib/actions/tooltip";
  import { longpress } from "$lib/actions/longpress";
  import { type ResizeDirection } from "$lib/actions/interactable";
  import { draggable } from "$lib/actions/draggable";
  import { resizable } from "$lib/actions/resizable";
  import { get } from "svelte/store";
  import {
    canvasItems,
    canvasViewport,
    updateCanvasItem,
    removeCanvasItem,
    addCanvasItem,
  } from "$lib/stores/layoutStore";
  import type { CanvasItem } from "$lib/stores/layoutStore";
  import type { WidgetType } from "$lib/stores/notebookStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import {
    getWidgetComponent,
    getWidgetSummaryComponent,
    getWidgetLabel,
  } from "$lib/core/registry/WidgetRegistry";
  import UnknownWidget from "$lib/components/shared/UnknownWidget.svelte";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { WIDGET_DEFINITIONS } from "$lib/stores/notebookStore";
  import { getWidgetContextItems } from "$lib/core/services/widgetContextActions";
  import {
    createLink,
    pipelineLinks,
    registerSink,
    registerSource,
    removeAllLinksForWidget,
    removeLink,
    unregisterSink,
    unregisterSource,
    type PipelineDataType,
  } from "$lib/core/services/PipelineService";
  import { createZIndexManager } from "$lib/core/layout/zIndexManager";

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

  let canvasEl: HTMLDivElement;
  let repaintKickHandle: number | null = null;

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

  function nudgeCanvasRendering(): void {
    if (typeof window === "undefined") return;
    if (repaintKickHandle !== null) {
      cancelAnimationFrame(repaintKickHandle);
    }
    repaintKickHandle = requestAnimationFrame(() => {
      repaintKickHandle = requestAnimationFrame(() => {
        screenW = canvasEl?.clientWidth ?? screenW;
        screenH = canvasEl?.clientHeight ?? screenH;
        window.dispatchEvent(new Event("resize"));
        repaintKickHandle = null;
      });
    });
  }

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
  const MIN_WIDGET_WIDTH = 280;
  const MIN_WIDGET_HEIGHT = 200;

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
  // Pointer-unified pan: left on background (or Alt/Space), and middle-click.

  let isPanning = false;
  let activePanPointerId: number | null = null;
  let panStartX = 0;
  let panStartY = 0;
  let panStartPanX = 0;
  let panStartPanY = 0;

  /** Check whether a pointer target is the canvas background (not a widget). */
  function isCanvasBackground(target: EventTarget | null): boolean {
    if (!target || !(target instanceof HTMLElement)) return false;
    // The canvas background elements have these classes
    return (
      target.classList.contains("infinite-canvas") ||
      target.classList.contains("canvas-grid") ||
      target.classList.contains("canvas-transform")
    );
  }

  function handleCanvasPointerDown(event: PointerEvent): void {
    const middleClick = event.button === 1;
    const leftPanGesture =
      event.button === 0 &&
      (event.altKey || spaceHeld || isCanvasBackground(event.target));

    if (!middleClick && !leftPanGesture) return;

    event.preventDefault();
    activePanPointerId = event.pointerId;
    isPanning = true;
    if (spaceHeld) spacePanning = true;
    panStartX = event.clientX;
    panStartY = event.clientY;
    panStartPanX = panX;
    panStartPanY = panY;
    attachWindowGrabListeners();
  }

  // ── Window-level grab listeners ──
  // When the user starts a pan, drag, or resize we attach move/up
  // listeners to `window` so events keep firing even if the cursor
  // leaves the canvas boundary.

  let _grabListenersAttached = false;

  function attachWindowGrabListeners(): void {
    if (_grabListenersAttached) return;
    _grabListenersAttached = true;
    window.addEventListener("pointermove", handleCanvasPointerMove, { passive: false });
    window.addEventListener("pointerup", handleCanvasPointerUp, { passive: false });
    window.addEventListener("pointercancel", handleCanvasPointerUp, { passive: false });
  }

  function detachWindowGrabListeners(): void {
    if (!_grabListenersAttached) return;
    _grabListenersAttached = false;
    window.removeEventListener("pointermove", handleCanvasPointerMove);
    window.removeEventListener("pointerup", handleCanvasPointerUp);
    window.removeEventListener("pointercancel", handleCanvasPointerUp);
  }

  // ── RAF-throttled store updates ──
  // During drag/resize the mouse fires much faster than the display refresh
  // rate (often 120 Hz+).  Writing the Svelte store on *every* mousemove
  // allocates a new array and triggers re-renders of ALL canvas widgets.
  // Instead we accumulate the latest desired position and flush once per
  // animation frame.

  let _rafPending = false;
  let _pendingDragPatch: { id: string; x: number; y: number } | null = null;
  let _pendingResizePatch: {
    id: string;
    x: number;
    y: number;
    width: number;
    height: number;
  } | null = null;

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
        x: _pendingResizePatch.x,
        y: _pendingResizePatch.y,
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

  function handleInteractionMove(clientX: number, clientY: number): void {
    if (isPanning) {
      panX = panStartPanX + (clientX - panStartX);
      panY = panStartPanY + (clientY - panStartY);
    }
  }

  function handleCanvasPointerMove(event: PointerEvent): void {
    if (activePanPointerId !== null && event.pointerId !== activePanPointerId) return;
    if (!isPanning) return;
    event.preventDefault();
    handleInteractionMove(event.clientX, event.clientY);
  }

  function stopCanvasPan(): void {
    if (isPanning) {
      isPanning = false;
      spacePanning = false;
      commitViewport();
    }
    activePanPointerId = null;
    detachWindowGrabListeners();
  }

  function handleCanvasPointerUp(event: PointerEvent): void {
    if (activePanPointerId !== null && event.pointerId !== activePanPointerId) return;
    event.preventDefault();
    stopCanvasPan();
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

  const zIndexManager = createZIndexManager();
  let selectedWidgetId: string | null = null;
  let automationEnabled = false;
  let connectSourceWidgetId: string | null = null;

  interface WidgetAutomationPorts {
    source?: PipelineDataType;
    sink?: PipelineDataType;
  }

  const WIDGET_AUTOMATION_PORTS: Partial<Record<WidgetType, WidgetAutomationPorts>> = {
    model: { source: "model" },
    reaction: { sink: "model", source: "reaction" },
    diagram: { sink: "reaction", source: "diagram" },
    amplitude: { sink: "diagram", source: "amplitude" },
    kinematics: { sink: "reaction", source: "kinematics" },
    dalitz: { sink: "kinematics" },
    event_display: { sink: "kinematics" },
    decay_calculator: { sink: "model", source: "decay_table" },
    analysis: { sink: "amplitude", source: "analysis_result" },
    notebook: { sink: "analysis_result" },
    log: { sink: "analysis_result" },
  };

  function itemCenter(item: CanvasItem): { x: number; y: number } {
    return {
      x: item.x + item.width / 2,
      y: item.y + item.height / 2,
    };
  }

  function linkPath(linkId: string): string {
    const link = $pipelineLinks.find((l) => l.id === linkId);
    if (!link) return "";

    const source = $canvasItems.find((item) => item.id === link.source.widgetId);
    const sink = $canvasItems.find((item) => item.id === link.sink.widgetId);
    if (!source || !sink) return "";

    const from = itemCenter(source);
    const to = itemCenter(sink);
    const dx = Math.abs(to.x - from.x);
    const offset = Math.max(70, Math.min(240, dx * 0.45));
    const c1x = from.x + offset;
    const c2x = to.x - offset;
    return `M ${from.x} ${from.y} C ${c1x} ${from.y}, ${c2x} ${to.y}, ${to.x} ${to.y}`;
  }

  function canAutomate(item: CanvasItem): boolean {
    return Boolean(WIDGET_AUTOMATION_PORTS[item.widgetType]);
  }

  function connectButtonTitle(item: CanvasItem): string {
    const ports = WIDGET_AUTOMATION_PORTS[item.widgetType];
    if (!ports?.source && !ports?.sink) return "No automation ports";
    if (!automationEnabled) return "Enable automation first";
    if (connectSourceWidgetId === item.id) return "Cancel source selection";
    if (!connectSourceWidgetId) {
      return ports?.source
        ? "Select as connection source"
        : "This widget only accepts input";
    }
    return "Connect selected source to this widget";
  }

  function handleConnectClick(item: CanvasItem): void {
    if (!automationEnabled || !canAutomate(item)) return;
    if (connectSourceWidgetId === item.id) {
      connectSourceWidgetId = null;
      return;
    }

    if (!connectSourceWidgetId) {
      if (!WIDGET_AUTOMATION_PORTS[item.widgetType]?.source) {
        return;
      }
      connectSourceWidgetId = item.id;
      selectWidget(item);
      return;
    }

    const created = createLink(connectSourceWidgetId, item.id);
    connectSourceWidgetId = null;
    if (!created) {
      // Incompatible ports are intentionally ignored.
    }
  }

  function toggleAutomation(): void {
    automationEnabled = !automationEnabled;
    if (!automationEnabled) {
      connectSourceWidgetId = null;
    }
  }

  function selectWidget(item: CanvasItem): void {
    bringToFrontById(item.id);
  }

  /** Bring an item to the foreground by ID (used when items are added). */
  function bringToFrontById(id: string): void {
    zIndexManager.bringToFront(id);
    selectedWidgetId = zIndexManager.snapshot().selectedId;
    nudgeCanvasRendering();
  }

  function zIndexOf(item: CanvasItem): number {
    return zIndexManager.zIndexOf(item.id);
  }

  // ── Auto-select newly created widgets ──
  // Track the previous item count; when it grows, bring the last item to front.
  let _prevItemCount = 0;
  let _previousCanvasIds = new Set<string>();

  const unsubCanvasItems = canvasItems.subscribe((items) => {
    const ids = new Set(items.map((item) => item.id));
    zIndexManager.sync(items.map((item) => item.id));
    selectedWidgetId = zIndexManager.snapshot().selectedId;

    if (items.length > _prevItemCount && items.length > 0) {
      const newest = items[items.length - 1];
      bringToFrontById(newest.id);
    } else if (_prevItemCount === 0 && items.length > 0) {
      selectedWidgetId = selectedWidgetId && ids.has(selectedWidgetId)
        ? selectedWidgetId
        : zIndexManager.snapshot().selectedId;
      nudgeCanvasRendering();
    }
    _prevItemCount = items.length;

    for (const removedId of _previousCanvasIds) {
      if (!ids.has(removedId)) {
        unregisterSource(removedId);
        unregisterSink(removedId);
        if (selectedWidgetId === removedId) {
          selectedWidgetId = null;
        }
      }
    }

    for (const link of get(pipelineLinks)) {
      if (!ids.has(link.source.widgetId) || !ids.has(link.sink.widgetId)) {
        removeLink(link.id);
      }
    }

    for (const item of items) {
      const ports = WIDGET_AUTOMATION_PORTS[item.widgetType];
      if (ports?.source) {
        registerSource(item.id, ports.source, `${WIDGET_LABELS[item.widgetType] ?? item.widgetType} output`);
      }
      if (ports?.sink) {
        registerSink(item.id, ports.sink, `${WIDGET_LABELS[item.widgetType] ?? item.widgetType} input`, () => {});
      }
    }

    _previousCanvasIds = ids;
  });

  function resetZoom(): void {
    zoom = 1;
    commitViewport();
  }

  function duplicateCanvasWidget(item: CanvasItem): void {
    addCanvasItem(item.widgetType, item.x + 36, item.y + 36);
  }

  function resetCanvasWidgetSize(item: CanvasItem): void {
    const def = WIDGET_DEFINITIONS.find((entry) => entry.type === item.widgetType);
    const width = (def?.defaultColSpan ?? 2) * 320;
    const height = (def?.defaultRowSpan ?? 2) * 220;
    updateCanvasItem(item.id, { width, height });
  }

  function queueDragPatch(itemId: string, x: number, y: number): void {
    _pendingDragPatch = { id: itemId, x, y };
    scheduleCanvasFlush();
  }

  function queueResizePatch(itemId: string, patch: { x: number; y: number; width: number; height: number }): void {
    _pendingResizePatch = {
      id: itemId,
      x: patch.x,
      y: patch.y,
      width: patch.width,
      height: patch.height,
    };
    scheduleCanvasFlush();
  }

  function handleWidgetDragMove(itemId: string, patch: { x: number; y: number }): void {
    queueDragPatch(itemId, patch.x, patch.y);
  }

  function handleWidgetDragEnd(itemId: string, patch: { x: number; y: number }): void {
    if (_rafPending) {
      flushCanvasUpdates();
    }
    const current = get(canvasItems).find((entry) => entry.id === itemId);
    if (!current) return;
    const snapped = snapToEdges(itemId, patch.x, patch.y, current.width, current.height);
    updateCanvasItem(itemId, { x: snapped.x, y: snapped.y });
  }

  function handleWidgetResizeMove(itemId: string, patch: { x: number; y: number; width: number; height: number }): void {
    queueResizePatch(itemId, patch);
  }

  function handleWidgetResizeEnd(itemId: string, patch: { x: number; y: number; width: number; height: number }): void {
    queueResizePatch(itemId, patch);
    if (_rafPending) {
      flushCanvasUpdates();
    }
  }

  function dragInteractableOptions(item: CanvasItem, enabled = true) {
    return {
      itemId: item.id,
      enabled,
      getZoom: () => zoom,
      getOrigin: () => ({ x: item.x, y: item.y }),
      onBringToFront: bringToFrontById,
      onMove: (patch: { x: number; y: number }) => handleWidgetDragMove(item.id, patch),
      onEnd: (patch: { x: number; y: number }) => handleWidgetDragEnd(item.id, patch),
    };
  }

  function resizeInteractableOptions(item: CanvasItem, direction: ResizeDirection) {
    return {
      itemId: item.id,
      direction,
      minWidth: MIN_WIDGET_WIDTH,
      minHeight: MIN_WIDGET_HEIGHT,
      getZoom: () => zoom,
      getOrigin: () => ({ x: item.x, y: item.y, width: item.width, height: item.height }),
      onBringToFront: bringToFrontById,
      onMove: (patch: { x: number; y: number; width: number; height: number }) => handleWidgetResizeMove(item.id, patch),
      onEnd: (patch: { x: number; y: number; width: number; height: number }) => handleWidgetResizeEnd(item.id, patch),
    };
  }

  function handleClose(item: CanvasItem): void {
    removeAllLinksForWidget(item.id);
    removeCanvasItem(item.id);
    if (connectSourceWidgetId === item.id) {
      connectSourceWidgetId = null;
    }
  }

  /** Open a widget context menu at viewport coordinates. */
  function openWidgetContextAt(x: number, y: number, item: CanvasItem): void {
    const widgetItems = getWidgetContextItems(item.widgetType);
    const items: import("$lib/types/menu").ContextMenuItem[] = [
      ...widgetItems,
      ...(widgetItems.length > 0
        ? [{ type: "separator" as const, id: "sep-cw" }]
        : []),
      { type: "action" as const, id: "cw-front", label: "Bring to Front", icon: "⇡", action: () => selectWidget(item) },
      { type: "action" as const, id: "cw-duplicate", label: "Duplicate Widget", icon: "⧉", action: () => duplicateCanvasWidget(item) },
      { type: "action" as const, id: "cw-reset-size", label: "Reset Widget Size", icon: "□", action: () => resetCanvasWidgetSize(item) },
      { type: "separator" as const, id: "sep-cw-layout" },
      { type: "action" as const, id: "cw-close", label: "Close Widget", icon: "✕", action: () => handleClose(item) },
    ];
    showContextMenu(x, y, items);
  }

  /** Right-click on a canvas widget body — widget-specific items + close. */
  function handleWidgetBodyContext(e: MouseEvent, item: CanvasItem): void {
    e.preventDefault();
    e.stopPropagation();
    openWidgetContextAt(e.clientX, e.clientY, item);
  }

  function handleCanvasDragOver(event: DragEvent): void {
    if (!event.dataTransfer?.types.includes("application/x-spire-widget-type")) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
  }

  function handleCanvasDrop(event: DragEvent): void {
    const widgetType = event.dataTransfer?.getData("application/x-spire-widget-type");
    if (!widgetType) return;

    event.preventDefault();
    const rect = canvasEl.getBoundingClientRect();
    const worldX = (event.clientX - rect.left - panX) / zoom;
    const worldY = (event.clientY - rect.top - panY) / zoom;
    addCanvasItem(widgetType as WidgetType, worldX, worldY);
    nudgeCanvasRendering();
  }

  function handleCanvasFocusRequest(event: Event): void {
    const detail = (event as CustomEvent<{ widgetId?: string }>).detail;
    if (!detail?.widgetId) return;
    bringToFrontById(detail.widgetId);
  }

  /**
   * Svelte action that registers an imperative capture-phase pointerdown listener.
   * This is necessary for reliable bring-to-front behavior in Svelte 5, where the
   * `on:event|capture` directive syntax is legacy and may not work correctly.
   * The capture phase fires BEFORE any child element's bubble-phase handler,
   * so stopPropagation() in child interactable actions cannot suppress it.
   */
  function capturePointerDown(node: HTMLElement, onDown: () => void) {
    function handler(event: PointerEvent): void {
      // Respond to left click (button 0) and any touch/pen contact.
      // Middle click (button 1) and right click (button 2) do not activate.
      if (event.pointerType === "mouse" && event.button !== 0) return;
      onDown();
    }
    node.addEventListener("pointerdown", handler, { capture: true });
    return {
      update(newOnDown: () => void): void {
        onDown = newOnDown;
      },
      destroy(): void {
        node.removeEventListener("pointerdown", handler, { capture: true });
      },
    };
  }

  // ── Canvas Context Menu ──

  function openCanvasContextAt(clientX: number, clientY: number): void {
    // Convert viewport coords to world coords for widget placement
    const rect = canvasEl.getBoundingClientRect();
    const worldX = (clientX - rect.left - panX) / zoom;
    const worldY = (clientY - rect.top - panY) / zoom;

    const widgetSubItems: import("$lib/types/menu").ContextMenuItem[] = WIDGET_DEFINITIONS.map((def) => ({
      type: "action" as const,
      id: `ctx-add-${def.type}`,
      label: def.label,
      action: () => addCanvasItem(def.type, worldX, worldY),
    }));

    showContextMenu(clientX, clientY, [
      { type: "submenu", id: "ctx-add-widget", label: "Add Widget", icon: "+", children: widgetSubItems },
      { type: "separator", id: "sep-canvas" },
      { type: "action", id: "ctx-reset-zoom", label: "Reset Zoom", shortcut: "Click %", action: () => resetZoom() },
      { type: "action", id: "ctx-center-view", label: "Center View", action: () => { panX = 0; panY = 0; commitViewport(); } },
    ]);
  }

  function handleCanvasContextMenu(event: MouseEvent): void {
    // Shift + Right-click bypasses SPIRE and opens the native browser menu
    if (event.shiftKey) return;
    event.preventDefault();
    event.stopPropagation();
    openCanvasContextAt(event.clientX, event.clientY);
  }

  // ── Global Shortcut Event Handlers ──

  function handleDeleteSelected(): void {
    if (selectedWidgetId) {
      removeAllLinksForWidget(selectedWidgetId);
      removeCanvasItem(selectedWidgetId);
      if (connectSourceWidgetId === selectedWidgetId) {
        connectSourceWidgetId = null;
      }
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

    window.addEventListener("spire:canvas:focus-widget", handleCanvasFocusRequest as EventListener);
    nudgeCanvasRendering();
  });

  onDestroy(() => {
    for (const item of get(canvasItems)) {
      unregisterSource(item.id);
      unregisterSink(item.id);
    }

    unsubViewport();
    unsubCanvasItems();
    resizeObserver?.disconnect();
    detachWindowGrabListeners();
    if (repaintKickHandle !== null) {
      cancelAnimationFrame(repaintKickHandle);
    }
    window.removeEventListener("keydown", handleKeyDown);
    window.removeEventListener("keyup", handleKeyUp);
    window.removeEventListener("spire:canvas:delete-selected", handleDeleteSelected as EventListener);
    window.removeEventListener("spire:canvas:duplicate-selected", handleDuplicateSelected as EventListener);
    window.removeEventListener("spire:canvas:select-all", handleSelectAll as EventListener);
    window.removeEventListener("spire:canvas:focus-widget", handleCanvasFocusRequest as EventListener);
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
  on:pointerdown={handleCanvasPointerDown}
  on:wheel={handleWheel}
  on:dragover={handleCanvasDragOver}
  on:drop={handleCanvasDrop}
  on:contextmenu={handleCanvasContextMenu}
  use:longpress={{
    duration: 500,
    moveTolerance: 12,
    onLongPress: (detail) => {
      const target = detail.target;
      if (target instanceof HTMLElement && target.closest(".canvas-widget")) return;
      openCanvasContextAt(detail.x, detail.y);
    },
  }}
  data-tour-id="canvas-workspace"
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
    {#if automationEnabled}
      <svg class="automation-links" aria-hidden="true">
        {#each $pipelineLinks as link (link.id)}
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <path class="automation-link" d={linkPath(link.id)} on:dblclick={() => removeLink(link.id)}></path>
        {/each}
      </svg>
    {/if}

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
          data-canvas-item-id={item.id}
          use:capturePointerDown={() => bringToFrontById(item.id)}
          on:focusin={() => selectWidget(item)}
          on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
          use:longpress={{
            duration: 480,
            moveTolerance: 12,
            onLongPress: (detail) => openWidgetContextAt(detail.x, detail.y, item),
          }}
          role="group"
          aria-label="{WIDGET_LABELS[item.widgetType] ?? item.widgetType} widget"
        >
          <!-- Widget header (draggable) — hidden at minimal LOD -->
          {#if lodLevel !== "minimal"}
            <header
              class="cw-header"
              use:draggable={dragInteractableOptions(item)}
              on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
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
                class="cw-link"
                class:active={connectSourceWidgetId === item.id}
                disabled={!automationEnabled || !canAutomate(item)}
                on:click|stopPropagation={() => handleConnectClick(item)}
                on:pointerdown|stopPropagation
                on:mousedown|stopPropagation
                use:tooltip={{ text: connectButtonTitle(item) }}
              >↔</button>
              <button
                class="cw-close"
                on:click={() => handleClose(item)}
                on:pointerdown|stopPropagation
                on:mousedown|stopPropagation
                use:tooltip={{ text: "Close" }}
              >&times;</button>
            </header>
          {/if}

          <!-- Widget body -->
          <div
            class="cw-body"
            class:cw-body-minimal={lodLevel === "minimal"}
            use:draggable={dragInteractableOptions(item, lodLevel === "minimal")}
            on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
            role="region"
          >
            <CanvasLODWrapper widgetType={item.widgetType} {zoom}>
              <!-- ═══ FULL LOD (zoom ≥ 0.6): interactive widget ═══ -->
              <div slot="full" class="zoom-adaptive-text">
                {@const WidgetComponent = getWidgetComponent(item.widgetType)}
                {#if WidgetComponent}
                  <svelte:component this={WidgetComponent} />
                {:else}
                  <UnknownWidget widgetType={item.widgetType} />
                {/if}
              </div>

              <!-- ═══ SUMMARY LOD (0.3 ≤ zoom < 0.6): simplified card ═══ -->
              <div slot="summary">
                {@const SummaryComponent = getWidgetSummaryComponent(item.widgetType)}
                {#if SummaryComponent}
                  <svelte:component this={SummaryComponent} />
                {:else if WIDGET_LABELS[item.widgetType]}
                  <span style="color: var(--fg-secondary); font-size: 0.68rem; font-style: italic;">
                    {getWidgetLabel(item.widgetType)}
                  </span>
                {:else}
                  <UnknownWidget widgetType={item.widgetType} compact={true} />
                {/if}
              </div>
            </CanvasLODWrapper>
          </div>

          <!-- Resize handle (hidden at minimal LOD) -->
          {#if lodLevel !== "minimal"}
            <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
            <div class="cw-resize-layer" aria-hidden="true">
              <div class="cw-resize-handle cw-resize-n" role="separator" aria-label="Resize north" tabindex="-1" use:resizable={resizeInteractableOptions(item, "n")}></div>
              <div class="cw-resize-handle cw-resize-ne" role="separator" aria-label="Resize north-east" tabindex="-1" use:resizable={resizeInteractableOptions(item, "ne")}></div>
              <div class="cw-resize-handle cw-resize-e" role="separator" aria-label="Resize east" tabindex="-1" use:resizable={resizeInteractableOptions(item, "e")}></div>
              <div class="cw-resize-handle cw-resize-se" role="separator" aria-label="Resize south-east" tabindex="-1" use:resizable={resizeInteractableOptions(item, "se")}></div>
              <div class="cw-resize-handle cw-resize-s" role="separator" aria-label="Resize south" tabindex="-1" use:resizable={resizeInteractableOptions(item, "s")}></div>
              <div class="cw-resize-handle cw-resize-sw" role="separator" aria-label="Resize south-west" tabindex="-1" use:resizable={resizeInteractableOptions(item, "sw")}></div>
              <div class="cw-resize-handle cw-resize-w" role="separator" aria-label="Resize west" tabindex="-1" use:resizable={resizeInteractableOptions(item, "w")}></div>
              <div class="cw-resize-handle cw-resize-nw" role="separator" aria-label="Resize north-west" tabindex="-1" use:resizable={resizeInteractableOptions(item, "nw")}></div>
            </div>
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
  <button class="zoom-indicator" on:click={resetZoom} use:tooltip={{ text: "Reset zoom to 100%" }}>
    {Math.round(zoom * 100)}%
    {#if lodLevel !== "full"}
      <span class="lod-badge">{lodLevel === "summary" ? "SUM" : "MIN"}</span>
    {/if}
  </button>

  <button
    class="automation-toggle"
    class:active={automationEnabled}
    on:click={toggleAutomation}
    use:tooltip={{ text: automationEnabled ? "Disable widget automation controls" : "Enable widget automation controls" }}
  >
    {automationEnabled ? "Automation: On" : "Automation: Off"}
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
    touch-action: none;
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

  .automation-links {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: visible;
    pointer-events: none;
  }

  .automation-link {
    fill: none;
    stroke: color-mix(in oklab, var(--hl-symbol) 76%, var(--border));
    stroke-width: 2.2;
    stroke-dasharray: 8 6;
    pointer-events: auto;
    cursor: pointer;
  }

  .automation-link:hover {
    stroke-width: 2.8;
    stroke: var(--hl-symbol);
  }

  .canvas-widget {
    position: absolute;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    /* Do NOT set overflow: hidden here — it would clip the resize handles
       that extend 4-5 px outside the border-box. Clipping is applied to
       .cw-body instead so widget content is still contained. */
    overflow: visible;
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
    touch-action: none;
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

  .cw-link {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 0 0.16rem;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .cw-link:hover:not(:disabled) {
    color: var(--hl-symbol);
    border-color: var(--border-focus);
  }

  .cw-link.active {
    color: var(--hl-symbol);
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.1);
  }

  .cw-link:disabled {
    opacity: 0.35;
    cursor: not-allowed;
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
    touch-action: none;
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

  .cw-resize-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    /* overflow: visible ensures resize handles positioned outside the
       widget border-box are accessible. The parent .canvas-widget
       must also have overflow: visible for this to work. */
    overflow: visible;
  }

  .cw-resize-handle {
    position: absolute;
    pointer-events: auto;
    z-index: 10;
    touch-action: none;
    /* Transparent by default — only visible on hover */
    background: transparent;
  }

  /* Edge handles: thicker for easier touch targeting */
  .cw-resize-n,
  .cw-resize-s {
    left: 12px;
    right: 12px;
    height: 10px;
  }

  .cw-resize-n { top: -5px; cursor: n-resize; }
  .cw-resize-s { bottom: -5px; cursor: s-resize; }

  .cw-resize-e,
  .cw-resize-w {
    top: 12px;
    bottom: 12px;
    width: 10px;
  }

  .cw-resize-e { right: -5px; cursor: e-resize; }
  .cw-resize-w { left: -5px; cursor: w-resize; }

  /* Corner handles: larger for touch */
  .cw-resize-ne,
  .cw-resize-se,
  .cw-resize-sw,
  .cw-resize-nw {
    width: 16px;
    height: 16px;
  }

  .cw-resize-ne { top: -6px; right: -6px; cursor: ne-resize; }
  .cw-resize-se { bottom: -6px; right: -6px; cursor: se-resize; }
  .cw-resize-sw { bottom: -6px; left: -6px; cursor: sw-resize; }
  .cw-resize-nw { top: -6px; left: -6px; cursor: nw-resize; }

  .cw-resize-layer .cw-resize-handle:hover {
    background: color-mix(in srgb, var(--hl-symbol) 40%, transparent);
    border-radius: 2px;
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

  .automation-toggle {
    position: absolute;
    top: 8px;
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

  .automation-toggle:hover {
    border-color: var(--hl-symbol);
    color: var(--fg-primary);
  }

  .automation-toggle.active {
    border-color: var(--hl-symbol);
    color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.1);
  }
</style>
