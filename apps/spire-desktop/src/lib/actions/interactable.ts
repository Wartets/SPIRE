import {
  interactionManager,
  type DragOrigin,
  type ResizeDirection,
  type ResizeOrigin,
} from "$lib/core/layout/interactionManager";

interface BaseInteractableOptions {
  itemId: string;
  getZoom: () => number;
  onBringToFront?: (itemId: string) => void;
  enabled?: boolean;
}

export interface DragInteractableOptions extends BaseInteractableOptions {
  mode: "drag";
  getOrigin: () => DragOrigin;
  onMove: (patch: DragOrigin) => void;
  onEnd?: (patch: DragOrigin) => void;
}

export interface ResizeInteractableOptions extends BaseInteractableOptions {
  mode: "resize";
  direction: ResizeDirection;
  minWidth: number;
  minHeight: number;
  getOrigin: () => ResizeOrigin;
  onMove: (patch: ResizeOrigin) => void;
  onEnd?: (patch: ResizeOrigin) => void;
}

export type InteractableOptions = DragInteractableOptions | ResizeInteractableOptions;

type ActionReturn = {
  update?: (newOptions: InteractableOptions) => void;
  destroy?: () => void;
};

/**
 * Unified pointer-based drag/resize action for infinite canvas widgets.
 */
export function interactable(node: HTMLElement, initialOptions: InteractableOptions): ActionReturn {
  let options = initialOptions;
  let activePointerId: number | null = null;
  let usingWindowFallback = false;
  let usingMouseFallback = false;
  let lastPointerDownAt = 0;

  const MOUSE_POINTER_ID = -1;
  const POINTER_MOUSE_DEDUPE_MS = 80;

  function safeSetCapture(pointerId: number): boolean {
    try {
      node.setPointerCapture(pointerId);
      return node.hasPointerCapture(pointerId);
    } catch {
      // If capture cannot be set (e.g. transient detached node), continue gracefully.
      return false;
    }
  }

  function safeReleaseCapture(pointerId: number): void {
    try {
      if (node.hasPointerCapture(pointerId)) {
        node.releasePointerCapture(pointerId);
      }
    } catch {
      // Ignore capture release races.
    }
  }

  function addWindowFallbackListeners(): void {
    if (usingWindowFallback) return;
    usingWindowFallback = true;
    window.addEventListener("pointermove", handlePointerMove, { passive: false });
    window.addEventListener("pointerup", finishPointer, { passive: false });
    window.addEventListener("pointercancel", handlePointerCancel, { passive: false });
  }

  function removeWindowFallbackListeners(): void {
    if (!usingWindowFallback) return;
    usingWindowFallback = false;
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", finishPointer);
    window.removeEventListener("pointercancel", handlePointerCancel);
  }

  function addWindowMouseFallbackListeners(): void {
    if (usingMouseFallback) return;
    usingMouseFallback = true;
    window.addEventListener("mousemove", handleMouseMove, { passive: false });
    window.addEventListener("mouseup", finishMouse, { passive: false });
  }

  function removeWindowMouseFallbackListeners(): void {
    if (!usingMouseFallback) return;
    usingMouseFallback = false;
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", finishMouse);
  }

  function handlePointerDown(event: PointerEvent): void {
    if (options.enabled === false) return;
    if (event.button !== 0) return;
    lastPointerDownAt = Date.now();

    options.onBringToFront?.(options.itemId);

    const startInteraction = () =>
      options.mode === "drag"
        ? interactionManager.startDrag({
            itemId: options.itemId,
            pointerId: event.pointerId,
            clientX: event.clientX,
            clientY: event.clientY,
            origin: options.getOrigin(),
          })
        : interactionManager.startResize({
            itemId: options.itemId,
            pointerId: event.pointerId,
            clientX: event.clientX,
            clientY: event.clientY,
            origin: options.getOrigin(),
            direction: options.direction,
            minWidth: options.minWidth,
            minHeight: options.minHeight,
          });

    let started = startInteraction();
    if (!started) {
      // Defensive recovery: an interrupted pointer lifecycle can leave
      // a stale global session that blocks all future drags/resizes.
      interactionManager.forceReset();
      started = startInteraction();
    }

    if (!started) return;

    activePointerId = event.pointerId;
    safeSetCapture(event.pointerId);
    // Keep fallback listeners even when capture succeeds.
    // WebView capture delivery can be inconsistent across hosts.
    addWindowFallbackListeners();
    event.preventDefault();
    event.stopPropagation();
  }

  function handleMouseDown(event: MouseEvent): void {
    if (options.enabled === false) return;
    if (event.button !== 0) return;
    if (activePointerId !== null) return;

    // Ignore compatibility mouse events emitted immediately after pointerdown.
    if (Date.now() - lastPointerDownAt < POINTER_MOUSE_DEDUPE_MS) return;

    options.onBringToFront?.(options.itemId);

    const startInteraction = () =>
      options.mode === "drag"
        ? interactionManager.startDrag({
            itemId: options.itemId,
            pointerId: MOUSE_POINTER_ID,
            clientX: event.clientX,
            clientY: event.clientY,
            origin: options.getOrigin(),
          })
        : interactionManager.startResize({
            itemId: options.itemId,
            pointerId: MOUSE_POINTER_ID,
            clientX: event.clientX,
            clientY: event.clientY,
            origin: options.getOrigin(),
            direction: options.direction,
            minWidth: options.minWidth,
            minHeight: options.minHeight,
          });

    let started = startInteraction();
    if (!started) {
      interactionManager.forceReset();
      started = startInteraction();
    }

    if (!started) return;

    activePointerId = MOUSE_POINTER_ID;
    addWindowMouseFallbackListeners();
    event.preventDefault();
    event.stopPropagation();
  }

  function handlePointerMove(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;
    const update = interactionManager.move(
      event.pointerId,
      event.clientX,
      event.clientY,
      options.getZoom(),
    );
    if (!update) return;

    if (update.mode === "drag" && options.mode === "drag") {
      options.onMove(update.patch);
    }

    if (update.mode === "resize" && options.mode === "resize") {
      options.onMove(update.patch);
    }

    event.preventDefault();
    event.stopPropagation();
  }

  function handleMouseMove(event: MouseEvent): void {
    if (activePointerId !== MOUSE_POINTER_ID) return;
    const update = interactionManager.move(
      MOUSE_POINTER_ID,
      event.clientX,
      event.clientY,
      options.getZoom(),
    );
    if (!update) return;

    if (update.mode === "drag" && options.mode === "drag") {
      options.onMove(update.patch);
    }

    if (update.mode === "resize" && options.mode === "resize") {
      options.onMove(update.patch);
    }

    event.preventDefault();
    event.stopPropagation();
  }

  function finishPointer(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;

    const final = interactionManager.move(
      event.pointerId,
      event.clientX,
      event.clientY,
      options.getZoom(),
    );

    if (final?.mode === "drag" && options.mode === "drag") {
      options.onMove(final.patch);
      options.onEnd?.(final.patch);
    }

    if (final?.mode === "resize" && options.mode === "resize") {
      options.onMove(final.patch);
      options.onEnd?.(final.patch);
    }

    interactionManager.end(event.pointerId);
    safeReleaseCapture(event.pointerId);
    removeWindowFallbackListeners();
    activePointerId = null;

    event.preventDefault();
    event.stopPropagation();
  }

  function handlePointerCancel(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;
    interactionManager.cancel(event.pointerId);
    safeReleaseCapture(event.pointerId);
    removeWindowFallbackListeners();
    activePointerId = null;
    event.preventDefault();
    event.stopPropagation();
  }

  function finishMouse(event: MouseEvent): void {
    if (activePointerId !== MOUSE_POINTER_ID) return;

    const final = interactionManager.move(
      MOUSE_POINTER_ID,
      event.clientX,
      event.clientY,
      options.getZoom(),
    );

    if (final?.mode === "drag" && options.mode === "drag") {
      options.onMove(final.patch);
      options.onEnd?.(final.patch);
    }

    if (final?.mode === "resize" && options.mode === "resize") {
      options.onMove(final.patch);
      options.onEnd?.(final.patch);
    }

    interactionManager.end(MOUSE_POINTER_ID);
    removeWindowMouseFallbackListeners();
    activePointerId = null;

    event.preventDefault();
    event.stopPropagation();
  }

  function handleLostPointerCapture(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;
    interactionManager.cancel(event.pointerId);
    removeWindowFallbackListeners();
    activePointerId = null;
  }

  node.addEventListener("pointerdown", handlePointerDown);
  node.addEventListener("mousedown", handleMouseDown);
  node.addEventListener("pointermove", handlePointerMove);
  node.addEventListener("pointerup", finishPointer);
  node.addEventListener("pointercancel", handlePointerCancel);
  node.addEventListener("lostpointercapture", handleLostPointerCapture);

  return {
    update(newOptions: InteractableOptions): void {
      options = newOptions;
    },
    destroy(): void {
      node.removeEventListener("pointerdown", handlePointerDown);
      node.removeEventListener("mousedown", handleMouseDown);
      node.removeEventListener("pointermove", handlePointerMove);
      node.removeEventListener("pointerup", finishPointer);
      node.removeEventListener("pointercancel", handlePointerCancel);
      node.removeEventListener("lostpointercapture", handleLostPointerCapture);
      removeWindowFallbackListeners();
      removeWindowMouseFallbackListeners();
      if (activePointerId !== null) {
        interactionManager.cancel(activePointerId);
        safeReleaseCapture(activePointerId);
      }
    },
  };
}

export type { ResizeDirection };
