import { describe, it, expect } from "vitest";
import { isolateEvents } from "../widgetEvents";

describe("isolateEvents action", () => {
  it("stops wheel propagation to parent listeners", () => {
    const parent = document.createElement("div");
    const child = document.createElement("div");
    parent.appendChild(child);

    let parentWheelCount = 0;
    parent.addEventListener("wheel", () => {
      parentWheelCount += 1;
    });

    const action = isolateEvents(child);

    const evt = new WheelEvent("wheel", { bubbles: true, cancelable: true });
    child.dispatchEvent(evt);

    expect(parentWheelCount).toBe(0);
    action.destroy();
  });

  it("honors per-event options", () => {
    const parent = document.createElement("div");
    const child = document.createElement("div");
    parent.appendChild(child);

    let parentMouseDownCount = 0;
    let parentWheelCount = 0;

    parent.addEventListener("mousedown", () => {
      parentMouseDownCount += 1;
    });
    parent.addEventListener("wheel", () => {
      parentWheelCount += 1;
    });

    const action = isolateEvents(child, {
      wheel: true,
      mouse: false,
      pointer: false,
      touch: false,
    });

    child.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    child.dispatchEvent(new WheelEvent("wheel", { bubbles: true }));

    expect(parentMouseDownCount).toBe(1);
    expect(parentWheelCount).toBe(0);
    action.destroy();
  });
});
