<!--
  SPIRE - Infinite Canvas

  Whiteboard-mode workspace.  Widgets are placed at arbitrary (x, y)
  positions on a pannable, zoomable 2D plane.

  Controls:
  - Left-click + drag on background → Pan the viewport
  - Middle-click + drag             → Pan the viewport
  - Scroll wheel on background      → Zoom to cursor
  - Left-click on widget header     → Drag widget to reposition
  - Resize handle (8 directions)    → Resize widget
  - Click anywhere on a widget      → Bring to front
  - Click on zoom indicator         → Reset to 100%

  All canvas interactions (pan, drag, resize, bring-to-front) are driven
  by a SINGLE capture-phase pointer handler on the canvas root.
  No per-widget Svelte actions are used for dragging or resizing.

  The canvas state (items, viewport) lives in `layoutStore`.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { longpress } from "$lib/actions/longpress";
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
    zoomToLod,
    zoomToUiPercent,
    computeViewport,
    expandByOverscan,
    isVisible,
    WIDGET_ICONS,
    WIDGET_ACCENT,
  } from "$lib/components/canvas/lodUtils";

  let canvasEl: HTMLDivElement;
  let minimapEl: HTMLDivElement;
  let repaintKickHandle: number | null = null;

  // ── Viewport ──

  let panX = 0;
  let panY = 0;
  let zoom = 1;

  const unsubViewport = canvasViewport.subscribe((vp) => {
    panX = vp.panX;
    panY = vp.panY;
    zoom = vp.zoom;
  });

  function nudgeCanvasRendering(): void {
    if (typeof window === "undefined") return;
    if (repaintKickHandle !== null) cancelAnimationFrame(repaintKickHandle);
    repaintKickHandle = requestAnimationFrame(() => {
      repaintKickHandle = requestAnimationFrame(() => {
        screenW = canvasEl?.clientWidth ?? screenW;
        screenH = canvasEl?.clientHeight ?? screenH;
        window.dispatchEvent(new Event("resize"));
        repaintKickHandle = null;
      });
    });
  }

  function clampPanToBounds(nextPanX: number, nextPanY: number, nextZoom: number): { panX: number; panY: number } {
    if (!boundsEnabled) return { panX: nextPanX, panY: nextPanY };

    const z = Math.max(nextZoom, 1e-6);
    const halfW = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfWidth, BOUNDS_MAX_HALF_SIZE));
    const halfH = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfHeight, BOUNDS_MAX_HALF_SIZE));

    let minPanX = screenW - halfW * z;
    let maxPanX = halfW * z;
    let minPanY = screenH - halfH * z;
    let maxPanY = halfH * z;

    if (minPanX > maxPanX) {
      const center = (minPanX + maxPanX) / 2;
      minPanX = center;
      maxPanX = center;
    }
    if (minPanY > maxPanY) {
      const center = (minPanY + maxPanY) / 2;
      minPanY = center;
      maxPanY = center;
    }

    return {
      panX: Math.min(maxPanX, Math.max(minPanX, nextPanX)),
      panY: Math.min(maxPanY, Math.max(minPanY, nextPanY)),
    };
  }

  function commitViewport(): void {
    const clamped = clampPanToBounds(panX, panY, zoom);
    panX = clamped.panX;
    panY = clamped.panY;
    canvasViewport.set({ panX, panY, zoom });
  }

  // ── Multi-Level Dot Grid ──
  const MINOR_GRID = 20;
  const MAJOR_GRID = 100;

  // ── Finite Bounds & Minimap ──
  const BOUNDS_MIN_HALF_SIZE = 800;
  const BOUNDS_MAX_HALF_SIZE = 20000;
  let boundsEnabled = true;
  let boundsHalfWidth = 4200;
  let boundsHalfHeight = 3200;
  let minimapVisible = true;
  let minimapInteractive = true;
  let minimapSettingsOpen = false;

  interface MinimapViewportDragState {
    active: boolean;
    pointerId: number;
    grabDx: number;
    grabDy: number;
  }

  let minimapViewportDrag: MinimapViewportDragState = {
    active: false,
    pointerId: -1,
    grabDx: 0,
    grabDy: 0,
  };

  $: minorSize    = MINOR_GRID * zoom;
  $: majorSize    = MAJOR_GRID * zoom;
  $: minorOpacity = Math.min(1, Math.max(0, (zoom - 0.3) / 0.4));
  $: minorDotR    = Math.max(0.5, 0.8 * zoom);
  $: majorDotR    = Math.max(1,   1.4 * zoom);

  $: gridStyle = `
    background-position: ${panX}px ${panY}px;
    background-size:
      ${minorSize}px ${minorSize}px,
      ${majorSize}px ${majorSize}px;
    background-image:
      radial-gradient(circle, rgba(136,136,136,${minorOpacity * 0.45}) ${minorDotR}px, transparent ${minorDotR}px),
      radial-gradient(circle, rgba(170,170,170,0.55) ${majorDotR}px, transparent ${majorDotR}px);
  `;

  // ── Frustum Culling & LOD ──
  let screenW = 900;
  let screenH = 600;

  $: worldViewport = expandByOverscan(computeViewport(panX, panY, zoom, screenW, screenH));
  $: lodLevel = zoomToLod(zoom);

  function itemIsVisible(item: CanvasItem): boolean {
    // While viewport geometry is stabilising (mode switch / first paint),
    // skip culling so widgets do not appear only after interaction.
    if (screenW <= 2 || screenH <= 2) return true;
    // Always show selected or actively transformed widgets.
    if (item.id === selectedWidgetId) return true;
    if ((gesture.mode === "drag" || gesture.mode === "resize") && gesture.wid === item.id) return true;
    return isVisible(item.x, item.y, item.width, item.height, worldViewport);
  }

  // ── Edge Snapping ──
  const SNAP_THRESHOLD  = 8;
  const MIN_WIDGET_WIDTH  = 280;
  const MIN_WIDGET_HEIGHT = 200;

  interface SnapResult { x: number; y: number; }

  function clampWidgetToBounds(x: number, y: number, w: number, h: number): { x: number; y: number; width: number; height: number } {
    if (!boundsEnabled) return { x, y, width: w, height: h };

    const halfW = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfWidth, BOUNDS_MAX_HALF_SIZE));
    const halfH = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfHeight, BOUNDS_MAX_HALF_SIZE));
    const maxWidth = Math.max(MIN_WIDGET_WIDTH, halfW * 2);
    const maxHeight = Math.max(MIN_WIDGET_HEIGHT, halfH * 2);
    const width = Math.max(MIN_WIDGET_WIDTH, Math.min(w, maxWidth));
    const height = Math.max(MIN_WIDGET_HEIGHT, Math.min(h, maxHeight));

    return {
      x: Math.min(halfW - width, Math.max(-halfW, x)),
      y: Math.min(halfH - height, Math.max(-halfH, y)),
      width,
      height,
    };
  }

  function snapToEdges(dragId: string, x: number, y: number, w: number, h: number): SnapResult {
    let sx = x, sy = y;
    let bestDx = SNAP_THRESHOLD + 1;
    let bestDy = SNAP_THRESHOLD + 1;

    const rect    = canvasEl?.getBoundingClientRect();
    const vpLeft  = -panX / zoom;
    const vpTop   = -panY / zoom;
    const vpRight  = vpLeft + (rect?.width  ?? 900) / zoom;
    const vpBottom = vpTop  + (rect?.height ?? 600) / zoom;

    const xEdges = [vpLeft, vpRight];
    const yEdges = [vpTop, vpBottom];

    for (const sib of get(canvasItems)) {
      if (sib.id === dragId) continue;
      xEdges.push(sib.x, sib.x + sib.width);
      yEdges.push(sib.y, sib.y + sib.height);
    }

    for (const edge of xEdges) {
      const dL = Math.abs(x - edge), dR = Math.abs(x + w - edge);
      if (dL < bestDx) { bestDx = dL; sx = edge; }
      if (dR < bestDx) { bestDx = dR; sx = edge - w; }
    }
    for (const edge of yEdges) {
      const dT = Math.abs(y - edge), dB = Math.abs(y + h - edge);
      if (dT < bestDy) { bestDy = dT; sy = edge; }
      if (dB < bestDy) { bestDy = dB; sy = edge - h; }
    }

    return { x: sx, y: sy };
  }

  // ── Spacebar hold (grab-to-pan) ──

  let spaceHeld   = false;
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
      spaceHeld    = false;
      spacePanning = false;
    }
  }

  // ══════════════════════════════════════════════════════════════════════════
  // UNIFIED CANVAS GESTURE STATE MACHINE
  // ══════════════════════════════════════════════════════════════════════════
  //
  // One capture-phase pointerdown handler on the canvas root classifies
  // every press as: pan / drag / resize / focus-only.
  //
  // Benefits over the old per-widget use:draggable / use:resizable approach:
  //   • No closure-over-loop-variable hazards in {#each} blocks
  //   • No competing per-element event listeners (was 10+ per widget)
  //   • z-index binding uses {zIndexById[item.id]} directly — Svelte tracks
  //     this dependency and updates the style when zIndexById is reassigned
  //   • Touch and pen work correctly (no button-check false negatives)
  //   • Single onGestureMove / onGestureEnd drives all three operations

  type ResizeDir = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw";

  type Gesture =
    | { mode: "idle" }
    | { mode: "pan";    pid: number; sx: number; sy: number; spx: number; spy: number }
    | { mode: "drag";   pid: number; wid: string; sx: number; sy: number; ox: number; oy: number }
    | { mode: "resize"; pid: number; wid: string; dir: ResizeDir;
        sx: number; sy: number; ox: number; oy: number; ow: number; oh: number };

  let gesture: Gesture = { mode: "idle" };
  let gestureWindowActive = false;

  const RESIZE_DIRS: ResizeDir[] = ["nw", "ne", "sw", "se", "n", "s", "e", "w"];

  /**
   * Inspect the event's composedPath (innermost → outermost, stop at canvasEl)
   * and return:
   *   widgetId     – data-canvas-item-id on the .canvas-widget ancestor
   *   resizeDir    – direction encoded in the first cw-resize-{dir} class found
   *   isDragHandle – true when the hit element is .cw-header or .cw-body-minimal
   *                  and NOT a button/input inside the header
   */
  function classifyPointerDown(event: PointerEvent): {
    widgetId: string | null;
    resizeDir: ResizeDir | null;
    isDragHandle: boolean;
  } {
    const path = event.composedPath?.() ?? [];
    let widgetId: string | null = null;
    let resizeDir: ResizeDir | null = null;
    let isDragHandle = false;
    let hitInteractive = false;

    for (const raw of path) {
      if (!(raw instanceof HTMLElement)) continue;
      if (raw === canvasEl) break;

      // Widget identity
      if (!widgetId && raw.dataset.canvasItemId) {
        widgetId = raw.dataset.canvasItemId;
      }

      // Resize handle direction
      if (!resizeDir) {
        for (const dir of RESIZE_DIRS) {
          if (raw.classList.contains(`cw-resize-${dir}`)) { resizeDir = dir; break; }
        }
      }

      // Buttons/inputs come before .cw-header in the path (inner → outer).
      // When found, they suppress drag so clicking cw-close / cw-link is safe.
      if (!hitInteractive &&
          (raw.tagName === "BUTTON" || raw.tagName === "INPUT" ||
           raw.tagName === "TEXTAREA" || raw.tagName === "SELECT")) {
        hitInteractive = true;
      }

      // Drag handle: the header bar, or the body in minimal-LOD mode
      if (!isDragHandle && !hitInteractive) {
        if (raw.classList.contains("cw-header") ||
            raw.classList.contains("cw-body-minimal")) {
          isDragHandle = true;
        }
      }
    }

    return { widgetId, resizeDir, isDragHandle };
  }

  function attachGestureWindowListeners(): void {
    if (gestureWindowActive) return;
    gestureWindowActive = true;
    window.addEventListener("pointermove",   onGestureMove,   { passive: false });
    window.addEventListener("pointerup",     onGestureEnd,    { passive: false });
    window.addEventListener("pointercancel", onGestureCancel, { passive: false });
  }

  function detachGestureWindowListeners(): void {
    if (!gestureWindowActive) return;
    gestureWindowActive = false;
    window.removeEventListener("pointermove",   onGestureMove);
    window.removeEventListener("pointerup",     onGestureEnd);
    window.removeEventListener("pointercancel", onGestureCancel);
  }

  /**
   * Unified capture-phase pointerdown handler.
   *
   * Fires before any bubble-phase handler, including the isolateEvents action
   * that widget bodies use to guard against accidental canvas panning.
   *
   * Responsibilities:
   *  1. Always bring the clicked widget to the front.
   *  2. Classify the press (pan / resize / drag / focus-only).
   *  3. Start the gesture and attach window move/up listeners.
   *
   * event.preventDefault() is called ONLY when starting an actual gesture,
   * so buttons, inputs, and other interactive elements inside widget bodies
   * keep receiving focus and click events normally.
   */
  function onCanvasPointerDown(event: PointerEvent): void {
    // Ignore non-primary touch contacts and all but left/middle mouse buttons
    if (event.pointerType === "mouse"  && event.button > 1)    return;
    if (event.pointerType !== "mouse"  && event.isPrimary === false) return;
    // Ignore if a gesture is already in progress
    if (gesture.mode !== "idle") return;

    // Allow widgets tagged data-pointer-passthrough (e.g. Three.js canvas in
    // EventDisplay) to receive pointer events natively so OrbitControls works.
    const hitTarget = event.target as HTMLElement | null;

    // Minimap interactions must never be captured as canvas gestures.
    if (hitTarget?.closest(".minimap-shell") || hitTarget?.closest(".minimap-restore")) {
      return;
    }

    if (hitTarget?.closest("[data-pointer-passthrough]")) {
      const widgetEl = hitTarget.closest("[data-canvas-item-id]") as HTMLElement | null;
      if (widgetEl?.dataset.canvasItemId) bringToFrontById(widgetEl.dataset.canvasItemId);
      return;
    }

    const { widgetId, resizeDir, isDragHandle } = classifyPointerDown(event);

    // ── Bring-to-front on ANY widget press (before determining gesture type)
    if (widgetId) bringToFrontById(widgetId);

    // ── Middle-click → pan
    if (event.button === 1) {
      gesture = { mode: "pan", pid: event.pointerId,
        sx: event.clientX, sy: event.clientY, spx: panX, spy: panY };
      event.preventDefault();
      attachGestureWindowListeners();
      return;
    }

    // Left-click, primary touch, or pen from here
    if (event.button !== 0 && event.pointerType !== "touch" && event.pointerType !== "pen") return;

    // ── Alt key or Spacebar → pan
    if (event.altKey || spaceHeld) {
      if (spaceHeld) spacePanning = true;
      gesture = { mode: "pan", pid: event.pointerId,
        sx: event.clientX, sy: event.clientY, spx: panX, spy: panY };
      event.preventDefault();
      attachGestureWindowListeners();
      return;
    }

    // ── Resize handle
    if (resizeDir && widgetId) {
      const item = get(canvasItems).find(i => i.id === widgetId);
      if (item) {
        gesture = {
          mode: "resize", pid: event.pointerId, wid: widgetId, dir: resizeDir,
          sx: event.clientX, sy: event.clientY,
          ox: item.x, oy: item.y, ow: item.width, oh: item.height,
        };
        event.preventDefault();
        attachGestureWindowListeners();
        return;
      }
    }

    // ── Drag handle (header title area, or minimal-LOD body — not a button)
    if (isDragHandle && widgetId) {
      const item = get(canvasItems).find(i => i.id === widgetId);
      if (item) {
        gesture = {
          mode: "drag", pid: event.pointerId, wid: widgetId,
          sx: event.clientX, sy: event.clientY,
          ox: item.x, oy: item.y,
        };
        event.preventDefault();
        attachGestureWindowListeners();
        return;
      }
    }

    // ── No widget → pan the canvas background
    if (!widgetId) {
      gesture = { mode: "pan", pid: event.pointerId,
        sx: event.clientX, sy: event.clientY, spx: panX, spy: panY };
      event.preventDefault();
      attachGestureWindowListeners();
      return;
    }

    // ── Widget body click (not header / resize handle): bring-to-front only.
    //    No gesture started. No preventDefault(). Widget content works normally.
  }

  function onGestureMove(event: PointerEvent): void {
    if (gesture.mode === "idle")              return;
    if (event.pointerId !== gesture.pid)     return;
    event.preventDefault();

    if (gesture.mode === "pan") {
      const nextPanX = gesture.spx + (event.clientX - gesture.sx);
      const nextPanY = gesture.spy + (event.clientY - gesture.sy);
      const clamped = clampPanToBounds(nextPanX, nextPanY, zoom);
      panX = clamped.panX;
      panY = clamped.panY;
      return;
    }

    const activeGesture = gesture;
    const safeZoom = Math.max(zoom, 1e-6);
    const dx = (event.clientX - activeGesture.sx) / safeZoom;
    const dy = (event.clientY - activeGesture.sy) / safeZoom;

    if (activeGesture.mode === "drag" && "wid" in activeGesture) {
      const existing = get(canvasItems).find((item) => item.id === activeGesture.wid);
      if (!existing) return;
      const bounded = clampWidgetToBounds(activeGesture.ox + dx, activeGesture.oy + dy, existing.width, existing.height);
      queueDragPatch(activeGesture.wid, bounded.x, bounded.y);
      return;
    }

    if (activeGesture.mode === "resize" && "wid" in activeGesture) {
      const { ox, oy, ow, oh, dir } = activeGesture;
      let nx = ox, ny = oy, nw = ow, nh = oh;
      if (dir.includes("e")) nw = Math.max(MIN_WIDGET_WIDTH,  ow + dx);
      if (dir.includes("s")) nh = Math.max(MIN_WIDGET_HEIGHT, oh + dy);
      if (dir.includes("w")) { nw = Math.max(MIN_WIDGET_WIDTH,  ow - dx); nx = ox + (ow - nw); }
      if (dir.includes("n")) { nh = Math.max(MIN_WIDGET_HEIGHT, oh - dy); ny = oy + (oh - nh); }
      const bounded = clampWidgetToBounds(nx, ny, nw, nh);
      queueResizePatch(activeGesture.wid, bounded);
    }
  }

  function onGestureEnd(event: PointerEvent): void {
    if (gesture.mode === "idle")          return;
    if (event.pointerId !== gesture.pid) return;
    event.preventDefault();

    // Apply final position
    onGestureMove(event);

    if (gesture.mode === "pan") {
      spacePanning = false;
      commitViewport();
    } else if (gesture.mode === "drag") {
      if (_rafPending) flushCanvasUpdates();
      const wid = gesture.wid;
      const current = get(canvasItems).find(i => i.id === wid);
      if (current) {
        const snapped = snapToEdges(current.id, current.x, current.y, current.width, current.height);
        const bounded = clampWidgetToBounds(snapped.x, snapped.y, current.width, current.height);
        updateCanvasItem(current.id, { x: bounded.x, y: bounded.y });
      }
    } else if (gesture.mode === "resize") {
      if (_rafPending) flushCanvasUpdates();
    }

    gesture = { mode: "idle" };
    detachGestureWindowListeners();
  }

  function onGestureCancel(event: PointerEvent): void {
    if (gesture.mode === "idle")          return;
    if (event.pointerId !== gesture.pid) return;
    gesture      = { mode: "idle" };
    spacePanning = false;
    detachGestureWindowListeners();
  }

  // ── RAF-throttled store updates ──
  // During drag/resize the pointer fires faster than the display refresh rate.
  // We accumulate the latest desired geometry and flush once per animation frame
  // to avoid re-rendering ALL widgets on every pointermove.

  let _rafPending = false;
  let _pendingDragPatch: { id: string; x: number; y: number } | null = null;
  let _pendingResizePatch: { id: string; x: number; y: number; width: number; height: number } | null = null;

  function flushCanvasUpdates(): void {
    _rafPending = false;
    if (_pendingDragPatch) {
      updateCanvasItem(_pendingDragPatch.id, { x: _pendingDragPatch.x, y: _pendingDragPatch.y });
      _pendingDragPatch = null;
    }
    if (_pendingResizePatch) {
      updateCanvasItem(_pendingResizePatch.id, {
        x: _pendingResizePatch.x, y: _pendingResizePatch.y,
        width: _pendingResizePatch.width, height: _pendingResizePatch.height,
      });
      _pendingResizePatch = null;
    }
  }

  function scheduleCanvasFlush(): void {
    if (!_rafPending) { _rafPending = true; requestAnimationFrame(flushCanvasUpdates); }
  }

  function queueDragPatch(itemId: string, x: number, y: number): void {
    _pendingDragPatch = { id: itemId, x, y };
    scheduleCanvasFlush();
  }

  function queueResizePatch(itemId: string, patch: { x: number; y: number; width: number; height: number }): void {
    _pendingResizePatch = { id: itemId, ...patch };
    scheduleCanvasFlush();
  }

  // ── Zoom (scroll wheel) ──

  function elementCapturesWheel(el: HTMLElement | null, boundary: HTMLElement): boolean {
    let cur = el;
    while (cur && cur !== boundary) {
      if (cur.hasAttribute("data-wheel-capture")) return true;
      cur = cur.parentElement;
    }
    return false;
  }

  function canScrollInDirection(el: HTMLElement, deltaY: number): boolean {
    const tol = 1;
    if (deltaY < 0) return el.scrollTop > tol;
    if (deltaY > 0) return el.scrollTop + el.clientHeight < el.scrollHeight - tol;
    return false;
  }

  function canScrollInDirectionX(el: HTMLElement, deltaX: number): boolean {
    const tol = 1;
    if (deltaX < 0) return el.scrollLeft > tol;
    if (deltaX > 0) return el.scrollLeft + el.clientWidth < el.scrollWidth - tol;
    return false;
  }

  function isScrollableElement(el: HTMLElement): boolean {
    const style = getComputedStyle(el);
    const overflowY = style.overflowY;
    const overflowX = style.overflowX;
    const scrollableY = (overflowY === "auto" || overflowY === "scroll" || overflowY === "overlay")
      && el.scrollHeight > el.clientHeight + 1;
    const scrollableX = (overflowX === "auto" || overflowX === "scroll" || overflowX === "overlay")
      && el.scrollWidth > el.clientWidth + 1;
    return scrollableX || scrollableY;
  }

  function canScrollableAncestorConsumeWheel(start: HTMLElement | null, boundary: HTMLElement, deltaX: number, deltaY: number): boolean {
    let cur = start;
    while (cur && cur !== boundary) {
      if (cur.hasAttribute("data-wheel-capture")) {
        return true;
      }
      if (isScrollableElement(cur)) {
        const consumeY = Math.abs(deltaY) >= Math.abs(deltaX) && canScrollInDirection(cur, deltaY);
        const consumeX = Math.abs(deltaX) > Math.abs(deltaY) && canScrollInDirectionX(cur, deltaX);
        if (consumeY || consumeX) {
          return true;
        }
      }
      cur = cur.parentElement;
    }
    return false;
  }

  function handleWheel(event: WheelEvent): void {
    const target = event.target as HTMLElement | null;
    const cwBody = target?.closest(".cw-body") as HTMLElement | null;

    if (target && cwBody && elementCapturesWheel(target, cwBody)) return;
    if (target && canvasEl && canScrollableAncestorConsumeWheel(target, canvasEl, event.deltaX, event.deltaY)) {
      return;
    }

    event.preventDefault();
    const factor  = event.deltaY > 0 ? 0.92 : 1.08;
    const newZoom = Math.max(0.15, Math.min(5, zoom * factor));
    const rect    = canvasEl.getBoundingClientRect();
    const mouseX  = event.clientX - rect.left;
    const mouseY  = event.clientY - rect.top;
    const worldX  = (mouseX - panX) / zoom;
    const worldY  = (mouseY - panY) / zoom;
    const unclampedPanX = mouseX - worldX * newZoom;
    const unclampedPanY = mouseY - worldY * newZoom;
    const clamped = clampPanToBounds(unclampedPanX, unclampedPanY, newZoom);
    panX = clamped.panX;
    panY = clamped.panY;
    zoom = newZoom;
    commitViewport();
  }

  // ── Widget Selection & Z-Index ──

  const zIndexManager = createZIndexManager();
  let selectedWidgetId: string | null = null;
  // zIndexById is referenced DIRECTLY in the template as {zIndexById[item.id] ?? 1}.
  // Svelte's compiler tracks the assignment `zIndexById = {...}` and re-evaluates
  // that template expression every time syncZOrderState() runs — even when
  // $canvasItems itself has not changed (e.g. pure bring-to-front click).
  let zIndexById: Record<string, number> = {};
  let automationEnabled        = false;
  let connectSourceWidgetId: string | null = null;

  interface WidgetAutomationPorts { source?: PipelineDataType; sink?: PipelineDataType; }

  const WIDGET_AUTOMATION_PORTS: Partial<Record<WidgetType, WidgetAutomationPorts>> = {
    model:            { source: "model" },
    reaction:         { sink: "model",           source: "reaction" },
    diagram:          { sink: "reaction",         source: "diagram" },
    amplitude:        { sink: "diagram",          source: "amplitude" },
    kinematics:       { sink: "reaction",         source: "kinematics" },
    dalitz:           { sink: "kinematics" },
    event_display:    { sink: "kinematics" },
    decay_calculator: { sink: "model",            source: "decay_table" },
    analysis:         { sink: "amplitude",        source: "analysis_result" },
    notebook:         { sink: "analysis_result" },
    log:              { sink: "analysis_result" },
  };

  function itemCenter(item: CanvasItem): { x: number; y: number } {
    return { x: item.x + item.width / 2, y: item.y + item.height / 2 };
  }

  function linkPath(linkId: string): string {
    const link   = $pipelineLinks.find((l) => l.id === linkId);
    if (!link) return "";
    const source = $canvasItems.find((item) => item.id === link.source.widgetId);
    const sink   = $canvasItems.find((item) => item.id === link.sink.widgetId);
    if (!source || !sink) return "";
    const from   = itemCenter(source);
    const to     = itemCenter(sink);
    const dx     = Math.abs(to.x - from.x);
    const offset = Math.max(70, Math.min(240, dx * 0.45));
    return `M ${from.x} ${from.y} C ${from.x + offset} ${from.y}, ${to.x - offset} ${to.y}, ${to.x} ${to.y}`;
  }

  function canAutomate(item: CanvasItem): boolean {
    return Boolean(WIDGET_AUTOMATION_PORTS[item.widgetType]);
  }

  function connectButtonTitle(item: CanvasItem): string {
    const ports = WIDGET_AUTOMATION_PORTS[item.widgetType];
    if (!ports?.source && !ports?.sink) return "No automation ports on this widget";
    if (!automationEnabled) return "Enable automation to link widgets";
    if (connectSourceWidgetId === item.id) return "Cancel source selection";
    if (!connectSourceWidgetId) {
      return ports?.source ? "Mark as source" : "This widget accepts input only";
    }
    return "Create link from selected source";
  }

  function handleConnectClick(item: CanvasItem): void {
    if (!automationEnabled || !canAutomate(item)) return;
    if (connectSourceWidgetId === item.id) { connectSourceWidgetId = null; return; }
    if (!connectSourceWidgetId) {
      if (!WIDGET_AUTOMATION_PORTS[item.widgetType]?.source) return;
      connectSourceWidgetId = item.id;
      selectWidget(item);
      return;
    }
    createLink(connectSourceWidgetId, item.id);
    connectSourceWidgetId = null;
  }

  function toggleAutomation(): void {
    automationEnabled = !automationEnabled;
    if (!automationEnabled) connectSourceWidgetId = null;
  }

  function setBoundsHalfWidth(value: number): void {
    boundsHalfWidth = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(BOUNDS_MAX_HALF_SIZE, Number.isFinite(value) ? value : BOUNDS_MIN_HALF_SIZE));
  }

  function setBoundsHalfHeight(value: number): void {
    boundsHalfHeight = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(BOUNDS_MAX_HALF_SIZE, Number.isFinite(value) ? value : BOUNDS_MIN_HALF_SIZE));
  }

  const MINIMAP_WIDTH = 190;
  const MINIMAP_HEIGHT = 120;

  function minimapDomain(): { minX: number; maxX: number; minY: number; maxY: number } {
    const halfW = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfWidth, BOUNDS_MAX_HALF_SIZE));
    const halfH = Math.max(BOUNDS_MIN_HALF_SIZE, Math.min(boundsHalfHeight, BOUNDS_MAX_HALF_SIZE));

    if (boundsEnabled) {
      return { minX: -halfW, maxX: halfW, minY: -halfH, maxY: halfH };
    }

    const items = get(canvasItems);
    const viewport = computeViewport(panX, panY, zoom, screenW, screenH);
    const viewportWidth = viewport.right - viewport.left;
    const viewportHeight = viewport.bottom - viewport.top;
    if (items.length === 0) {
      return {
        minX: viewport.left - viewportWidth,
        maxX: viewport.right + viewportWidth,
        minY: viewport.top - viewportHeight,
        maxY: viewport.bottom + viewportHeight,
      };
    }

    const minX = Math.min(viewport.left, ...items.map((i) => i.x));
    const maxX = Math.max(viewport.right, ...items.map((i) => i.x + i.width));
    const minY = Math.min(viewport.top, ...items.map((i) => i.y));
    const maxY = Math.max(viewport.bottom, ...items.map((i) => i.y + i.height));
    const padX = Math.max(120, (maxX - minX) * 0.1);
    const padY = Math.max(120, (maxY - minY) * 0.1);
    return { minX: minX - padX, maxX: maxX + padX, minY: minY - padY, maxY: maxY + padY };
  }

  $: miniDomain = minimapDomain();
  $: miniSpanX = Math.max(1, miniDomain.maxX - miniDomain.minX);
  $: miniSpanY = Math.max(1, miniDomain.maxY - miniDomain.minY);

  function miniX(worldX: number): number {
    return ((worldX - miniDomain.minX) / miniSpanX) * MINIMAP_WIDTH;
  }

  function miniY(worldY: number): number {
    return ((worldY - miniDomain.minY) / miniSpanY) * MINIMAP_HEIGHT;
  }

  $: viewportWorld = computeViewport(panX, panY, zoom, screenW, screenH);
  $: minimapViewportRect = {
    left: miniX(viewportWorld.left),
    top: miniY(viewportWorld.top),
    width: Math.max(8, ((viewportWorld.right - viewportWorld.left) / miniSpanX) * MINIMAP_WIDTH),
    height: Math.max(8, ((viewportWorld.bottom - viewportWorld.top) / miniSpanY) * MINIMAP_HEIGHT),
  };

  function updateViewportFromMinimap(clientX: number, clientY: number, grabDx: number, grabDy: number): void {
    if (!minimapEl || !minimapInteractive) return;

    const rect = minimapEl.getBoundingClientRect();
    const localX = clientX - rect.left;
    const localY = clientY - rect.top;

    const maxLeft = Math.max(0, MINIMAP_WIDTH - minimapViewportRect.width);
    const maxTop = Math.max(0, MINIMAP_HEIGHT - minimapViewportRect.height);
    const nextLeft = Math.max(0, Math.min(maxLeft, localX - grabDx));
    const nextTop = Math.max(0, Math.min(maxTop, localY - grabDy));

    const worldLeft = miniDomain.minX + (nextLeft / MINIMAP_WIDTH) * miniSpanX;
    const worldTop = miniDomain.minY + (nextTop / MINIMAP_HEIGHT) * miniSpanY;

    const clamped = clampPanToBounds(-worldLeft * zoom, -worldTop * zoom, zoom);
    panX = clamped.panX;
    panY = clamped.panY;
    commitViewport();
  }

  function handleMinimapViewportPointerDown(event: PointerEvent): void {
    if (!minimapInteractive) return;
    if (event.pointerType === "mouse" && event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();

    if (!minimapEl) return;
    const rect = minimapEl.getBoundingClientRect();
    const localX = event.clientX - rect.left;
    const localY = event.clientY - rect.top;

    minimapViewportDrag = {
      active: true,
      pointerId: event.pointerId,
      grabDx: localX - Math.max(0, minimapViewportRect.left),
      grabDy: localY - Math.max(0, minimapViewportRect.top),
    };

    const target = event.currentTarget as HTMLElement | null;
    target?.setPointerCapture?.(event.pointerId);

    window.addEventListener("pointermove", handleMinimapViewportPointerMove, { passive: false });
    window.addEventListener("pointerup", handleMinimapViewportPointerEnd, true);
    window.addEventListener("pointercancel", handleMinimapViewportPointerEnd, true);
  }

  function handleMinimapViewportPointerMove(event: PointerEvent): void {
    if (!minimapViewportDrag.active || event.pointerId !== minimapViewportDrag.pointerId) return;
    event.preventDefault();
    updateViewportFromMinimap(event.clientX, event.clientY, minimapViewportDrag.grabDx, minimapViewportDrag.grabDy);
  }

  function handleMinimapViewportPointerEnd(event: PointerEvent): void {
    if (!minimapViewportDrag.active || event.pointerId !== minimapViewportDrag.pointerId) return;
    event.stopPropagation();

    const target = event.target as HTMLElement | null;
    target?.releasePointerCapture?.(event.pointerId);

    minimapViewportDrag = {
      active: false,
      pointerId: -1,
      grabDx: 0,
      grabDy: 0,
    };

    window.removeEventListener("pointermove", handleMinimapViewportPointerMove);
    window.removeEventListener("pointerup", handleMinimapViewportPointerEnd, true);
    window.removeEventListener("pointercancel", handleMinimapViewportPointerEnd, true);
  }

  function handleMinimapViewportKeydown(event: KeyboardEvent): void {
    if (!minimapInteractive) return;

    const stepX = Math.max(24, (viewportWorld.right - viewportWorld.left) * 0.12);
    const stepY = Math.max(24, (viewportWorld.bottom - viewportWorld.top) * 0.12);

    let dx = 0;
    let dy = 0;

    if (event.key === "ArrowLeft") dx = -stepX;
    else if (event.key === "ArrowRight") dx = stepX;
    else if (event.key === "ArrowUp") dy = -stepY;
    else if (event.key === "ArrowDown") dy = stepY;
    else return;

    event.preventDefault();
    const clamped = clampPanToBounds(panX - dx * zoom, panY - dy * zoom, zoom);
    panX = clamped.panX;
    panY = clamped.panY;
    commitViewport();
  }

  function selectWidget(item: CanvasItem): void { bringToFrontById(item.id); }

  function syncZOrderState(): void {
    const snapshot = zIndexManager.snapshot();
    selectedWidgetId = snapshot.selectedId;
    zIndexById = snapshot.order.reduce<Record<string, number>>((acc, id, i) => {
      acc[id] = i + 1;
      return acc;
    }, {});
  }

  function bringToFrontById(id: string): void {
    zIndexManager.bringToFront(id);
    syncZOrderState();
    nudgeCanvasRendering();
  }

  // ── Auto-select newly added widgets ──
  let _prevItemCount      = 0;
  let _previousCanvasIds  = new Set<string>();

  const unsubCanvasItems = canvasItems.subscribe((items) => {
    const ids = new Set(items.map((item) => item.id));
    zIndexManager.sync(items.map((item) => item.id));
    syncZOrderState();

    if (items.length > _prevItemCount && items.length > 0) {
      bringToFrontById(items[items.length - 1].id);
    } else if (_prevItemCount === 0 && items.length > 0) {
      nudgeCanvasRendering();
    }
    _prevItemCount = items.length;

    for (const removedId of _previousCanvasIds) {
      if (!ids.has(removedId)) { unregisterSource(removedId); unregisterSink(removedId); }
    }
    for (const link of get(pipelineLinks)) {
      if (!ids.has(link.source.widgetId) || !ids.has(link.sink.widgetId)) removeLink(link.id);
    }
    for (const item of items) {
      const ports = WIDGET_AUTOMATION_PORTS[item.widgetType];
      if (ports?.source) registerSource(item.id, ports.source, `${WIDGET_LABELS[item.widgetType] ?? item.widgetType} output`);
      if (ports?.sink)   registerSink(item.id, ports.sink, `${WIDGET_LABELS[item.widgetType] ?? item.widgetType} input`, () => {});
    }
    _previousCanvasIds = ids;
  });

  function resetZoom(): void {
    zoom = 1;
    const clamped = clampPanToBounds(panX, panY, zoom);
    panX = clamped.panX;
    panY = clamped.panY;
    commitViewport();
  }

  function duplicateCanvasWidget(item: CanvasItem): void {
    addCanvasItem(item.widgetType, item.x + 36, item.y + 36);
  }

  function resetCanvasWidgetSize(item: CanvasItem): void {
    const def    = WIDGET_DEFINITIONS.find((e) => e.type === item.widgetType);
    const width  = (def?.defaultColSpan ?? 2) * 320;
    const height = (def?.defaultRowSpan ?? 2) * 220;
    updateCanvasItem(item.id, { width, height });
  }

  function handleClose(item: CanvasItem): void {
    removeAllLinksForWidget(item.id);
    removeCanvasItem(item.id);
    if (connectSourceWidgetId === item.id) connectSourceWidgetId = null;
  }

  function openWidgetContextAt(x: number, y: number, item: CanvasItem): void {
    const widgetItems = getWidgetContextItems(item.widgetType);
    const menuItems: import("$lib/types/menu").ContextMenuItem[] = [
      ...widgetItems,
      ...(widgetItems.length > 0 ? [{ type: "separator" as const, id: "sep-cw" }] : []),
      {
        type: "action" as const,
        id: "cw-toggle-automation",
        label: automationEnabled ? "Disable Automation" : "Enable Automation",
        icon: "auto",
        action: () => toggleAutomation(),
      },
      {
        type: "action" as const,
        id: "cw-clear-automation-selection",
        label: "Clear Automation Source",
        icon: "x",
        disabled: !connectSourceWidgetId,
        action: () => {
          connectSourceWidgetId = null;
        },
      },
      { type: "separator" as const, id: "sep-cw-automation" },
      { type: "action" as const, id: "cw-front",      label: "Bring to Front",    icon: "⇡", action: () => selectWidget(item) },
      { type: "action" as const, id: "cw-duplicate",  label: "Duplicate Widget",  icon: "⧉", action: () => duplicateCanvasWidget(item) },
      { type: "action" as const, id: "cw-reset-size", label: "Reset Widget Size", icon: "□", action: () => resetCanvasWidgetSize(item) },
      { type: "separator" as const, id: "sep-cw-layout" },
      { type: "action" as const, id: "cw-close",      label: "Close Widget",      icon: "✕", action: () => handleClose(item) },
    ];
    showContextMenu(x, y, menuItems);
  }

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
    const rect   = canvasEl.getBoundingClientRect();
    const worldX = (event.clientX - rect.left - panX) / zoom;
    const worldY = (event.clientY - rect.top  - panY) / zoom;
    addCanvasItem(widgetType as WidgetType, worldX, worldY);
    nudgeCanvasRendering();
  }

  function handleCanvasFocusRequest(event: Event): void {
    const detail = (event as CustomEvent<{ widgetId?: string }>).detail;
    if (detail?.widgetId) bringToFrontById(detail.widgetId);
  }

  // ── Canvas Context Menu ──

  function openCanvasContextAt(clientX: number, clientY: number): void {
    const rect   = canvasEl.getBoundingClientRect();
    const worldX = (clientX - rect.left - panX) / zoom;
    const worldY = (clientY - rect.top  - panY) / zoom;

    const widgetSubItems: import("$lib/types/menu").ContextMenuItem[] = WIDGET_DEFINITIONS.map((def) => ({
      type: "action" as const,
      id: `ctx-add-${def.type}`,
      label: def.label,
      action: () => addCanvasItem(def.type, worldX, worldY),
    }));

    showContextMenu(clientX, clientY, [
      { type: "submenu", id: "ctx-add-widget", label: "Add Widget", icon: "+", children: widgetSubItems },
      { type: "separator", id: "sep-canvas" },
      {
        type: "action",
        id: "ctx-toggle-automation",
        label: automationEnabled ? "Disable Automation" : "Enable Automation",
        icon: "auto",
        action: () => toggleAutomation(),
      },
      {
        type: "action",
        id: "ctx-canvas-toggle-minimap",
        label: minimapVisible ? "Hide Minimap" : "Show Minimap",
        icon: "map",
        action: () => {
          minimapVisible = !minimapVisible;
          if (minimapVisible) minimapSettingsOpen = false;
        },
      },
      {
        type: "toggle",
        id: "ctx-canvas-toggle-bounds",
        label: "Finite Bounds",
        checked: boundsEnabled,
        action: (checked) => {
          boundsEnabled = checked;
          if (checked) {
            setBoundsHalfWidth(boundsHalfWidth);
            setBoundsHalfHeight(boundsHalfHeight);
          }
          commitViewport();
        },
      },
      { type: "separator", id: "sep-canvas-view" },
      { type: "action", id: "ctx-reset-zoom",  label: "Reset Zoom",   shortcut: "Click %", action: () => resetZoom() },
      { type: "action", id: "ctx-center-view", label: "Center View",  action: () => { panX = 0; panY = 0; commitViewport(); } },
    ]);
  }

  function handleCanvasContextMenu(event: MouseEvent): void {
    if (event.shiftKey) return; // allow native browser menu with Shift+right-click
    event.preventDefault();
    event.stopPropagation();
    openCanvasContextAt(event.clientX, event.clientY);
  }

  // ── Global Shortcut Handlers ──

  function handleDeleteSelected(): void {
    if (!selectedWidgetId) return;
    removeAllLinksForWidget(selectedWidgetId);
    removeCanvasItem(selectedWidgetId);
    if (connectSourceWidgetId === selectedWidgetId) connectSourceWidgetId = null;
    selectedWidgetId = null;
  }

  function handleDuplicateSelected(): void {
    if (!selectedWidgetId) return;
    const src = get(canvasItems).find((i) => i.id === selectedWidgetId);
    if (src) addCanvasItem(src.widgetType, src.x + 30, src.y + 30);
  }

  function handleSelectAll(): void {
    const items = get(canvasItems);
    if (items.length > 0) selectedWidgetId = items[0].id;
  }

  function handleCanvasAutomationToggleEvent(): void {
    toggleAutomation();
  }

  $: {
    const clamped = clampPanToBounds(panX, panY, zoom);
    if (clamped.panX !== panX || clamped.panY !== panY) {
      panX = clamped.panX;
      panY = clamped.panY;
      commitViewport();
    }
  }

  // ── Lifecycle ──

  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    window.addEventListener("keydown",  handleKeyDown);
    window.addEventListener("keyup",    handleKeyUp);
    window.addEventListener("spire:canvas:delete-selected",    handleDeleteSelected    as EventListener);
    window.addEventListener("spire:canvas:duplicate-selected", handleDuplicateSelected as EventListener);
    window.addEventListener("spire:canvas:select-all",         handleSelectAll         as EventListener);
    window.addEventListener("spire:canvas:focus-widget",       handleCanvasFocusRequest as EventListener);
    window.addEventListener("spire:canvas:toggle-automation",  handleCanvasAutomationToggleEvent as EventListener);

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

    nudgeCanvasRendering();
  });

  onDestroy(() => {
    for (const item of get(canvasItems)) { unregisterSource(item.id); unregisterSink(item.id); }
    unsubViewport();
    unsubCanvasItems();
    resizeObserver?.disconnect();
    detachGestureWindowListeners();
    if (repaintKickHandle !== null) cancelAnimationFrame(repaintKickHandle);
    window.removeEventListener("keydown",  handleKeyDown);
    window.removeEventListener("keyup",    handleKeyUp);
    window.removeEventListener("spire:canvas:delete-selected",    handleDeleteSelected    as EventListener);
    window.removeEventListener("spire:canvas:duplicate-selected", handleDuplicateSelected as EventListener);
    window.removeEventListener("spire:canvas:select-all",         handleSelectAll         as EventListener);
    window.removeEventListener("spire:canvas:focus-widget",       handleCanvasFocusRequest as EventListener);
    window.removeEventListener("spire:canvas:toggle-automation",  handleCanvasAutomationToggleEvent as EventListener);
    window.removeEventListener("pointermove", handleMinimapViewportPointerMove);
    window.removeEventListener("pointerup", handleMinimapViewportPointerEnd, true);
    window.removeEventListener("pointercancel", handleMinimapViewportPointerEnd, true);
  });
</script>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
  class="infinite-canvas"
  class:space-held={spaceHeld && !spacePanning}
  class:space-panning={spacePanning}
  class:is-panning={gesture.mode === "pan"}
  style="--canvas-zoom: {zoom};"
  bind:this={canvasEl}
  on:pointerdown|capture={onCanvasPointerDown}
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
            z-index: {zIndexById[item.id] ?? 1};
            --lod-accent: {WIDGET_ACCENT[item.widgetType] ?? '#5eb8ff'};
          "
          data-canvas-item-id={item.id}
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
          <!-- Widget header — acts as drag handle; hidden at minimal LOD -->
          {#if lodLevel !== "minimal"}
            <header
              class="cw-header"
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
                aria-label="Automation link"
                on:click|stopPropagation={() => handleConnectClick(item)}
                on:pointerdown|stopPropagation
                on:mousedown|stopPropagation
                use:tooltip={{ text: connectButtonTitle(item) }}
              >L</button>
              <button
                class="cw-close"
                on:click={() => handleClose(item)}
                on:pointerdown|stopPropagation
                on:mousedown|stopPropagation
                use:tooltip={{ text: "Close" }}
              >&times;</button>
            </header>
          {/if}

          <!-- Widget body — scrollable interactive content -->
          <div
            class="cw-body"
            class:cw-body-minimal={lodLevel === "minimal"}
            on:contextmenu={(e) => handleWidgetBodyContext(e, item)}
            role="region"
          >
            <CanvasLODWrapper widgetType={item.widgetType} {zoom}>
              <!-- FULL LOD: interactive widget component -->
              <div slot="full" class="zoom-adaptive-text">
                {@const WidgetComponent = getWidgetComponent(item.widgetType)}
                {#if WidgetComponent}
                  <svelte:component this={WidgetComponent} />
                {:else}
                  <UnknownWidget widgetType={item.widgetType} />
                {/if}
              </div>

              <!-- SUMMARY LOD: simplified card -->
              <div slot="summary" class="cw-summary-content">
                {@const SummaryComponent = getWidgetSummaryComponent(item.widgetType)}
                {#if SummaryComponent}
                  <svelte:component this={SummaryComponent} />
                {:else if WIDGET_LABELS[item.widgetType]}
                  <span style="color: var(--fg-secondary); font-size: 2.9rem; font-style: italic;">
                    {getWidgetLabel(item.widgetType)}
                  </span>
                {:else}
                  <UnknownWidget widgetType={item.widgetType} compact={true} />
                {/if}
              </div>
            </CanvasLODWrapper>
          </div>

          <!-- 8-direction resize handles — hidden at minimal LOD -->
          {#if lodLevel !== "minimal"}
            <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
            <div class="cw-resize-layer" aria-hidden="true">
              <div class="cw-resize-handle cw-resize-n"  role="separator" aria-label="Resize north"      tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-ne" role="separator" aria-label="Resize north-east" tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-e"  role="separator" aria-label="Resize east"       tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-se" role="separator" aria-label="Resize south-east" tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-s"  role="separator" aria-label="Resize south"      tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-sw" role="separator" aria-label="Resize south-west" tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-w"  role="separator" aria-label="Resize west"       tabindex="-1"></div>
              <div class="cw-resize-handle cw-resize-nw" role="separator" aria-label="Resize north-west" tabindex="-1"></div>
            </div>
          {/if}
        </div>
      {:else}
        <!-- Off-screen placeholder: maintains layout geometry without DOM cost -->
        <div
          class="cw-placeholder"
          style="left: {item.x}px; top: {item.y}px; width: {item.width}px; height: {item.height}px;"
          aria-hidden="true"
        ></div>
      {/if}
    {/each}
  </div>

  <!-- Zoom indicator -->
  <button class="zoom-indicator" on:click={resetZoom} use:tooltip={{ text: "Reset zoom to 100%" }}>
    {zoomToUiPercent(zoom)}%
    {#if lodLevel !== "full"}
      <span class="lod-badge">{lodLevel === "summary" ? "SUM" : "MIN"}</span>
    {/if}
  </button>

  <div class="minimap-shell" class:minimap-hidden={!minimapVisible}>
    <div class="minimap-header">
      <span>Minimap</span>
      <div class="minimap-actions">
        <button
          class="minimap-action"
          on:click={() => {
            minimapInteractive = !minimapInteractive;
          }}
          use:tooltip={{ text: minimapInteractive ? "Lock minimap pointer interactions" : "Enable minimap pointer interactions" }}
          aria-label={minimapInteractive ? "Lock minimap interactions" : "Enable minimap interactions"}
        >
          <span class="minimap-action-icon">{minimapInteractive ? "⊙" : "⊘"}</span>
        </button>
        <button
          class="minimap-action"
          on:click={() => (minimapSettingsOpen = !minimapSettingsOpen)}
          use:tooltip={{ text: minimapSettingsOpen ? "Hide minimap settings" : "Show minimap settings" }}
          aria-label={minimapSettingsOpen ? "Hide minimap settings" : "Show minimap settings"}
        ><span class="minimap-action-icon">⚙</span></button>
        <button class="minimap-action" on:click={() => (minimapVisible = false)} use:tooltip={{ text: "Minimize minimap" }}><span class="minimap-action-icon">−</span></button>
      </div>
    </div>
    {#if minimapSettingsOpen}
      <div class="minimap-settings">
        <label class="bounds-toggle">
          <input
            type="checkbox"
            checked={boundsEnabled}
            on:change={(event) => {
              boundsEnabled = (event.currentTarget as HTMLInputElement).checked;
              commitViewport();
            }}
          />
          <span>Finite bounds</span>
        </label>
        {#if boundsEnabled}
          <div class="bounds-inputs">
            <label>
              Width
              <input
                type="number"
                min={BOUNDS_MIN_HALF_SIZE}
                max={BOUNDS_MAX_HALF_SIZE}
                step="100"
                value={boundsHalfWidth}
                on:change={(event) => setBoundsHalfWidth((event.currentTarget as HTMLInputElement).valueAsNumber)}
              />
            </label>
            <label>
              Height
              <input
                type="number"
                min={BOUNDS_MIN_HALF_SIZE}
                max={BOUNDS_MAX_HALF_SIZE}
                step="100"
                value={boundsHalfHeight}
                on:change={(event) => setBoundsHalfHeight((event.currentTarget as HTMLInputElement).valueAsNumber)}
              />
            </label>
          </div>
        {/if}
      </div>
    {/if}
    <div class="minimap" bind:this={minimapEl} aria-hidden={!minimapVisible}>
      {#if boundsEnabled}
        <div class="minimap-boundary"></div>
      {/if}
      {#each $canvasItems as miniItem (miniItem.id)}
        <button
          class="minimap-item"
          class:selected={miniItem.id === selectedWidgetId}
          style="left: {miniX(miniItem.x)}px; top: {miniY(miniItem.y)}px; width: {Math.max(3, (miniItem.width / miniSpanX) * MINIMAP_WIDTH)}px; height: {Math.max(3, (miniItem.height / miniSpanY) * MINIMAP_HEIGHT)}px;"
          tabindex={minimapInteractive ? 0 : -1}
          on:pointerdown|stopPropagation={() => minimapInteractive && bringToFrontById(miniItem.id)}
          on:keydown={(event) => {
            if (!minimapInteractive) return;
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              bringToFrontById(miniItem.id);
            }
          }}
          aria-label="Focus widget on minimap"
        ></button>
      {/each}
      <div
        role="button"
        tabindex={minimapInteractive ? 0 : -1}
        aria-label="Move viewport"
        class="minimap-viewport"
        class:dragging={minimapViewportDrag.active}
        on:pointerdown|stopPropagation={handleMinimapViewportPointerDown}
        on:keydown={handleMinimapViewportKeydown}
        style="left: {Math.max(0, minimapViewportRect.left)}px; top: {Math.max(0, minimapViewportRect.top)}px; width: {Math.max(2, Math.min(MINIMAP_WIDTH - Math.max(0, minimapViewportRect.left), minimapViewportRect.width))}px; height: {Math.max(2, Math.min(MINIMAP_HEIGHT - Math.max(0, minimapViewportRect.top), minimapViewportRect.height))}px;"
      ></div>
    </div>
  </div>

  {#if !minimapVisible}
    <button class="minimap-restore" on:click={() => (minimapVisible = true)} aria-label="Restore minimap">MINIMAP</button>
  {/if}
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

  .infinite-canvas.space-held    { cursor: grab; }
  .infinite-canvas.space-panning,
  .infinite-canvas.is-panning    { cursor: grabbing; }

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
    pointer-events: auto;
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
    --cw-accent-fusion: color-mix(in oklab, var(--lod-accent, var(--color-accent)) 62%, var(--color-accent) 38%);
    position: absolute;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    overflow: visible; /* must stay visible — resize handles extend outside */
    pointer-events: auto;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    transition: box-shadow 0.12s, border-color 0.12s;
    isolation: isolate;
    opacity: 1;
    contain: layout;
  }

  .canvas-widget.cw-selected {
    border-color: color-mix(in oklab, var(--cw-accent-fusion) 56%, var(--border) 44%);
    box-shadow:
      0 0 0 1px color-mix(in oklab, var(--cw-accent-fusion) 30%, transparent),
      0 2px 8px color-mix(in oklab, var(--cw-accent-fusion) 18%, #000 82%);
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

  .cw-header:active { cursor: grabbing; }

  .cw-header-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }

  .cw-header-icon :global(svg) { width: 13px; height: 13px; }

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

  .cw-link:hover:not(:disabled) { color: var(--hl-symbol); border-color: var(--border-focus); }
  .cw-link.active { color: var(--hl-symbol); border-color: var(--hl-symbol); background: rgba(var(--color-accent-rgb), 0.1); }
  .cw-link:disabled { opacity: 0.35; cursor: not-allowed; }
  .cw-close:hover   { color: var(--hl-error); border-color: var(--hl-error); }

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

  .cw-body-minimal:active { cursor: grabbing; }

  .cw-placeholder {
    position: absolute;
    pointer-events: none;
  }

  .cw-lod-summary { box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3); }
  .cw-lod-minimal { box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2); border-width: 1px; }

  .canvas-widget.cw-lod-summary .cw-header {
    min-height: 2.2rem;
    padding: 0.32rem 0.62rem;
  }

  .canvas-widget.cw-lod-summary .cw-title {
    font-size: 2rem;
    letter-spacing: 0.05em;
  }

  .canvas-widget.cw-lod-summary .cw-body {
    font-size: 3.7rem;
    line-height: 1.22;
  }

  .canvas-widget.cw-lod-minimal .cw-body {
    font-size: 5.3rem;
    line-height: 1.16;
  }

  .canvas-widget.cw-lod-minimal .cw-body,
  .canvas-widget.cw-lod-summary .cw-summary-content {
    font-weight: 600;
  }

  .cw-summary-content {
    width: 100%;
    height: 100%;
  }

  .zoom-adaptive-text {
    width: 100%;
    height: 100%;
  }

  .cw-resize-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: visible;
  }

  /* Resize handles: thin edge rails + directional hover accents */
  .cw-resize-handle {
    position: absolute;
    pointer-events: auto;
    z-index: 2;
    touch-action: none;
    background: transparent;
    transition: background-color 0.14s ease;
  }

  /* Edge handles */
  .cw-resize-n,
  .cw-resize-s {
    left: -1px;
    right: -1px;
    height: 3px;
    cursor: ns-resize;
  }

  .cw-resize-e,
  .cw-resize-w {
    top: -1px;
    bottom: -1px;
    width: 3px;
    cursor: ew-resize;
  }

  .cw-resize-n { top: -2px; }
  .cw-resize-s { bottom: -2px; }
  .cw-resize-e { right: -2px; }
  .cw-resize-w { left: -2px; }

  /* Corner handles */
  .cw-resize-ne,
  .cw-resize-se,
  .cw-resize-sw,
  .cw-resize-nw {
    width: 7px;
    height: 7px;
  }

  .cw-resize-ne { top: -3px; right: -3px; cursor: nesw-resize; }
  .cw-resize-se { bottom: -3px; right: -3px; cursor: nwse-resize; }
  .cw-resize-sw { bottom: -3px; left: -3px; cursor: nesw-resize; }
  .cw-resize-nw { top: -3px; left: -3px; cursor: nwse-resize; }

  .cw-resize-n:hover {
    background: color-mix(in srgb, var(--hl-symbol) 42%, transparent);
    border-radius: 3px 3px 0 0;
  }

  .cw-resize-s:hover {
    background: color-mix(in srgb, var(--hl-symbol) 42%, transparent);
    border-radius: 0 0 3px 3px;
  }

  .cw-resize-e:hover {
    background: color-mix(in srgb, var(--hl-symbol) 42%, transparent);
    border-radius: 0 3px 3px 0;
  }

  .cw-resize-w:hover {
    background: color-mix(in srgb, var(--hl-symbol) 42%, transparent);
    border-radius: 3px 0 0 3px;
  }

  .cw-resize-ne:hover,
  .cw-resize-se:hover,
  .cw-resize-sw:hover,
  .cw-resize-nw:hover {
    background: color-mix(in srgb, var(--hl-symbol) 42%, transparent);
    border-radius: 3px;
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

  .zoom-indicator:hover { border-color: var(--hl-symbol); color: var(--fg-primary); }

  .lod-badge {
    font-size: 0.5rem;
    padding: 0 0.2rem;
    border: 1px solid var(--hl-value);
    color: var(--hl-value);
    letter-spacing: 0.04em;
  }

  .bounds-toggle {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.58rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .bounds-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.35rem;
  }

  .bounds-inputs label {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.14rem;
    font-size: 0.56rem;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .bounds-inputs input[type="number"] {
    width: 100%;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.6rem;
    padding: 0.12rem 0.18rem;
  }

  .minimap-shell {
    position: absolute;
    left: 8px;
    bottom: 8px;
    z-index: 11;
    width: calc(190px + 0.45rem);
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-inset) 90%, transparent);
    backdrop-filter: blur(2px);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.22rem;
  }

  .minimap-settings {
    border: 1px solid color-mix(in srgb, var(--border) 80%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 72%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.28rem;
  }

  .minimap-shell.minimap-hidden {
    display: none;
  }

  .minimap-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.58rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0 0.05rem;
  }

  .minimap-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
  }

  .minimap-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.24rem;
    height: 1.24rem;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    cursor: pointer;
    font-size: 0.72rem;
    padding: 0;
    line-height: 1;
  }

  .minimap-action-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1em;
    height: 1em;
    line-height: 1;
    transform: translateY(-0.02em);
    pointer-events: none;
  }

  .minimap-action:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  .minimap {
    position: relative;
    width: 190px;
    height: 120px;
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    background: color-mix(in srgb, var(--bg-primary) 82%, transparent);
    overflow: hidden;
  }

  .minimap-boundary {
    position: absolute;
    inset: 0;
    border: 1px dashed color-mix(in srgb, var(--hl-symbol) 60%, transparent);
    pointer-events: none;
  }

  .minimap-item {
    position: absolute;
    z-index: 1;
    border: 1px solid color-mix(in srgb, var(--fg-secondary) 70%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 78%, transparent);
    min-width: 2px;
    min-height: 2px;
    padding: 0;
    margin: 0;
    appearance: none;
    cursor: pointer;
  }

  .minimap-item.selected {
    border-color: var(--hl-symbol);
    background: color-mix(in srgb, var(--hl-symbol) 24%, transparent);
  }

  .minimap-viewport {
    position: absolute;
    z-index: 2;
    border: 1px solid color-mix(in srgb, var(--hl-success, #3ddc97) 72%, white 28%);
    background: color-mix(in srgb, var(--hl-success, #3ddc97) 10%, transparent);
    pointer-events: auto;
    cursor: grab;
  }

  .minimap-viewport.dragging {
    cursor: grabbing;
  }

  .minimap-restore {
    position: absolute;
    left: 8px;
    bottom: 8px;
    z-index: 11;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    font-size: 0.5rem;
    font-family: var(--font-mono);
    padding: 0.09rem 0.3rem;
    min-width: 4.2rem;
    min-height: 1.05rem;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .minimap-restore:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }
</style>
