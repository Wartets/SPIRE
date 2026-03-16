import {
  interactionManager,
  type DragOrigin,
  type ResizeDirection,
  type ResizeOrigin,
} from "$lib/core/layout/CanvasInteractionManager";

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

  function handlePointerDown(event: PointerEvent): void {
    if (options.enabled === false) return;
    if (event.button !== 0) return;

    options.onBringToFront?.(options.itemId);

    const started =
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

    if (!started) return;

    activePointerId = event.pointerId;
    node.setPointerCapture(event.pointerId);
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
    if (node.hasPointerCapture(event.pointerId)) {
      node.releasePointerCapture(event.pointerId);
    }
    activePointerId = null;

    event.preventDefault();
    event.stopPropagation();
  }

  function handlePointerCancel(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;
    interactionManager.cancel(event.pointerId);
    if (node.hasPointerCapture(event.pointerId)) {
      node.releasePointerCapture(event.pointerId);
    }
    activePointerId = null;
    event.preventDefault();
    event.stopPropagation();
  }

  function handleLostPointerCapture(event: PointerEvent): void {
    if (activePointerId === null || event.pointerId !== activePointerId) return;
    interactionManager.cancel(event.pointerId);
    activePointerId = null;
  }

  node.addEventListener("pointerdown", handlePointerDown);
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
      node.removeEventListener("pointermove", handlePointerMove);
      node.removeEventListener("pointerup", finishPointer);
      node.removeEventListener("pointercancel", handlePointerCancel);
      node.removeEventListener("lostpointercapture", handleLostPointerCapture);
      if (activePointerId !== null) {
        interactionManager.cancel(activePointerId);
        if (node.hasPointerCapture(activePointerId)) {
          node.releasePointerCapture(activePointerId);
        }
      }
    },
  };
}

export type { ResizeDirection };
