import {
  interactable,
  type ResizeInteractableOptions,
} from "$lib/actions/interactable";

export interface ResizableOptions extends Omit<ResizeInteractableOptions, "mode"> {}

/**
 * Resize-only interaction action for canvas widgets.
 */
export function resizable(node: HTMLElement, options: ResizableOptions) {
  return interactable(node, {
    ...options,
    mode: "resize",
  });
}
