import {
  interactable,
  type DragInteractableOptions,
} from "$lib/actions/interactable";

export interface DraggableOptions extends Omit<DragInteractableOptions, "mode"> {}

/**
 * Drag-only interaction action for canvas widgets.
 */
export function draggable(node: HTMLElement, options: DraggableOptions) {
  return interactable(node, {
    ...options,
    mode: "drag",
  });
}
