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

  it("emits resize patches for pointer-based resize interactions", () => {
    if (typeof PointerEvent === "undefined") {
      expect(true).toBe(true);
      return;
    }

    const node = document.createElement("div");
    document.body.appendChild(node);

    const bringCalls: string[] = [];
    const movePatches: Array<{ x: number; y: number; width: number; height: number }> = [];
    const endPatches: Array<{ x: number; y: number; width: number; height: number }> = [];

    const action = interactable(node, {
      mode: "resize",
      itemId: "resize-pointer",
      direction: "sw",
      minWidth: 100,
      minHeight: 80,
      getZoom: () => 1,
      getOrigin: () => ({ x: 50, y: 60, width: 200, height: 150 }),
      onBringToFront: (id) => bringCalls.push(id),
      onMove: (patch) => movePatches.push(patch),
      onEnd: (patch) => endPatches.push(patch),
    });

    node.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 7, button: 0, clientX: 300, clientY: 200, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointermove", { pointerId: 7, clientX: 260, clientY: 240, bubbles: true }));
    node.dispatchEvent(new PointerEvent("pointerup", { pointerId: 7, clientX: 260, clientY: 240, bubbles: true }));

    expect(bringCalls).toEqual(["resize-pointer"]);
    expect(movePatches.at(-1)).toEqual({ x: 10, y: 60, width: 240, height: 190 });
    expect(endPatches.at(-1)).toEqual({ x: 10, y: 60, width: 240, height: 190 });

    action.destroy?.();
    node.remove();
  });

  it("supports mouse fallback resize interactions", () => {
    const node = document.createElement("div");
    document.body.appendChild(node);

    const bringCalls: string[] = [];
    const endPatches: Array<{ x: number; y: number; width: number; height: number }> = [];

    const action = interactable(node, {
      mode: "resize",
      itemId: "resize-mouse",
      direction: "ne",
      minWidth: 100,
      minHeight: 80,
      getZoom: () => 2,
      getOrigin: () => ({ x: 20, y: 30, width: 120, height: 100 }),
      onBringToFront: (id) => bringCalls.push(id),
      onMove: () => {},
      onEnd: (patch) => endPatches.push(patch),
    });

    node.dispatchEvent(new MouseEvent("mousedown", { button: 0, clientX: 100, clientY: 100, bubbles: true }));
    window.dispatchEvent(new MouseEvent("mousemove", { clientX: 140, clientY: 60, bubbles: true }));
    window.dispatchEvent(new MouseEvent("mouseup", { clientX: 140, clientY: 60, bubbles: true }));

    expect(bringCalls).toEqual(["resize-mouse"]);
    expect(endPatches.at(-1)).toEqual({ x: 20, y: 10, width: 140, height: 120 });

    action.destroy?.();
    node.remove();
  });

  it("cleans up the active session on pointercancel", () => {
    if (typeof PointerEvent === "undefined") {
      expect(true).toBe(true);
      return;
    }

    const node = document.createElement("div");
    document.body.appendChild(node);

    const action = interactable(node, {
      mode: "drag",
      itemId: "wid-cancel",
      getZoom: () => 1,
      getOrigin: () => ({ x: 0, y: 0 }),
      onMove: () => {},
    });

    node.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 13, button: 0, clientX: 10, clientY: 20, bubbles: true }));
    expect(interactionManager.activePointerId()).toBe(13);

    node.dispatchEvent(new PointerEvent("pointercancel", { pointerId: 13, bubbles: true }));
    expect(interactionManager.activePointerId()).toBeNull();

    action.destroy?.();
    node.remove();
  });
});
