import { describe, expect, it } from "vitest";
import { createZIndexManager } from "./zIndexManager";

describe("zIndexManager", () => {
  it("brings target widget to front deterministically", () => {
    const manager = createZIndexManager(["a", "b", "c"]);

    manager.bringToFront("b");

    expect(manager.snapshot().order).toEqual(["a", "c", "b"]);
    expect(manager.snapshot().selectedId).toBe("b");
    expect(manager.zIndexOf("b")).toBe(3);
  });

  it("sync keeps current order for retained ids and appends new ids", () => {
    const manager = createZIndexManager(["a", "b"]);
    manager.bringToFront("a");
    manager.sync(["b", "c", "a"]);

    expect(manager.snapshot().order).toEqual(["b", "a", "c"]);
    expect(manager.snapshot().currentMaxZ).toBe(3);
  });

  it("remove updates selection to top-most remaining widget", () => {
    const manager = createZIndexManager(["x", "y", "z"]);
    manager.bringToFront("y");

    manager.remove("y");

    expect(manager.snapshot().selectedId).toBe("z");
    expect(manager.snapshot().order).toEqual(["x", "z"]);
  });

  it("bringToFront accepts unknown ids and adds them", () => {
    const manager = createZIndexManager(["left"]);
    manager.bringToFront("new");

    expect(manager.snapshot().order).toEqual(["left", "new"]);
    expect(manager.zIndexOf("new")).toBe(2);
  });
});
