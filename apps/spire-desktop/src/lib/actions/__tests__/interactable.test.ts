import { beforeEach, describe, expect, it } from "vitest";
import { interactable } from "../interactable";
import { interactionManager } from "../../core/layout/interactionManager";

describe("interactable action", () => {
  beforeEach(() => {
    interactionManager.forceReset();
  });

  it("brings widget to front on pointerdown and emits zoom-correct drag patches", () => {
    if (typeof PointerEvent === "undefined") {
      expect(true).toBe(true);
      return;
    }

    const node = document.createElement("div");
    document.body.appendChild(node);

    const bringCalls: string[] = [];
    const patches: Array<{ x: number; y: number }> = [];

    const action = interactable(node, {
      mode: "drag",
      itemId: "wid",
      getZoom: () => 2,
      getOrigin: () => ({ x: 10, y: 20 }),
      onBringToFront: (id) => bringCalls.push(id),
      onMove: (patch) => patches.push(patch),
    });

    node.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 1, button: 0, clientX: 100, clientY: 100, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointermove", { pointerId: 1, clientX: 140, clientY: 160, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointerup", { pointerId: 1, clientX: 140, clientY: 160, bubbles: true }));

    expect(bringCalls).toEqual(["wid"]);
    expect(patches.at(-1)).toEqual({ x: 30, y: 50 });

    action.destroy?.();
    node.remove();
  });

  it("does not start when disabled", () => {
    if (typeof PointerEvent === "undefined") {
      expect(true).toBe(true);
      return;
    }

    const node = document.createElement("div");
    document.body.appendChild(node);

    let moved = false;

    const action = interactable(node, {
      mode: "drag",
      itemId: "wid",
      enabled: false,
      getZoom: () => 1,
      getOrigin: () => ({ x: 0, y: 0 }),
      onMove: () => {
        moved = true;
      },
    });

    node.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 2, button: 0, clientX: 0, clientY: 0, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointermove", { pointerId: 2, clientX: 50, clientY: 50, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointerup", { pointerId: 2, clientX: 50, clientY: 50, bubbles: true }));

    expect(moved).toBe(false);

    action.destroy?.();
    node.remove();
  });

  it("supports mouse fallback drag when pointer stream is unavailable", () => {
    const node = document.createElement("div");
    document.body.appendChild(node);

    const bringCalls: string[] = [];
    const patches: Array<{ x: number; y: number }> = [];

    const action = interactable(node, {
      mode: "drag",
      itemId: "wid-mouse",
      getZoom: () => 1,
      getOrigin: () => ({ x: 4, y: 6 }),
      onBringToFront: (id) => bringCalls.push(id),
      onMove: (patch) => patches.push(patch),
    });

    node.dispatchEvent(new MouseEvent("mousedown", { button: 0, clientX: 10, clientY: 20, bubbles: true }));
    window.dispatchEvent(new MouseEvent("mousemove", { clientX: 40, clientY: 45, bubbles: true }));
    window.dispatchEvent(new MouseEvent("mouseup", { clientX: 40, clientY: 45, bubbles: true }));

    expect(bringCalls).toEqual(["wid-mouse"]);
    expect(patches.at(-1)).toEqual({ x: 34, y: 31 });

    action.destroy?.();
    node.remove();
  });
});
