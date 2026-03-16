import { describe, expect, it, vi } from "vitest";
import { CanvasInteractionManager } from "./interactionManager";

describe("CanvasInteractionManager", () => {
  it("computes drag patches with zoom compensation", () => {
    const manager = new CanvasInteractionManager();
    const started = manager.startDrag({
      itemId: "w-1",
      pointerId: 11,
      clientX: 100,
      clientY: 120,
      origin: { x: 10, y: 20 },
    });

    expect(started).toBe(true);

    const patch = manager.move(11, 140, 160, 2);
    expect(patch).toEqual({
      mode: "drag",
      itemId: "w-1",
      patch: { x: 30, y: 40 },
    });
  });

  it("computes north-west resize with anchoring and clamps to minimums", () => {
    const manager = new CanvasInteractionManager();
    const started = manager.startResize({
      itemId: "w-2",
      pointerId: 22,
      clientX: 300,
      clientY: 300,
      direction: "nw",
      origin: { x: 100, y: 100, width: 500, height: 400 },
      minWidth: 280,
      minHeight: 200,
    });

    expect(started).toBe(true);

    const patch = manager.move(22, 900, 950, 1);
    expect(patch).toEqual({
      mode: "resize",
      itemId: "w-2",
      direction: "nw",
      patch: {
        x: 320,
        y: 300,
        width: 280,
        height: 200,
      },
    });
  });

  it("computes all eight resize directions correctly", () => {
    const cases = [
      {
        direction: "n",
        moveTo: { x: 100, y: 70 },
        expected: { x: 10, y: -10, width: 120, height: 130 },
      },
      {
        direction: "ne",
        moveTo: { x: 145, y: 70 },
        expected: { x: 10, y: -10, width: 165, height: 130 },
      },
      {
        direction: "e",
        moveTo: { x: 145, y: 100 },
        expected: { x: 10, y: 20, width: 165, height: 100 },
      },
      {
        direction: "se",
        moveTo: { x: 145, y: 140 },
        expected: { x: 10, y: 20, width: 165, height: 140 },
      },
      {
        direction: "s",
        moveTo: { x: 100, y: 140 },
        expected: { x: 10, y: 20, width: 120, height: 140 },
      },
      {
        direction: "sw",
        moveTo: { x: 70, y: 140 },
        expected: { x: -20, y: 20, width: 150, height: 140 },
      },
      {
        direction: "w",
        moveTo: { x: 70, y: 100 },
        expected: { x: -20, y: 20, width: 150, height: 100 },
      },
      {
        direction: "nw",
        moveTo: { x: 70, y: 70 },
        expected: { x: -20, y: -10, width: 150, height: 130 },
      },
    ] as const;

    for (const testCase of cases) {
      const manager = new CanvasInteractionManager();
      const started = manager.startResize({
        itemId: `resize-${testCase.direction}`,
        pointerId: 5,
        clientX: 100,
        clientY: 100,
        direction: testCase.direction,
        origin: { x: 10, y: 20, width: 120, height: 100 },
        minWidth: 60,
        minHeight: 50,
      });

      expect(started).toBe(true);

      const patch = manager.move(5, testCase.moveTo.x, testCase.moveTo.y, 1);
      expect(patch).toEqual({
        mode: "resize",
        itemId: `resize-${testCase.direction}`,
        direction: testCase.direction,
        patch: testCase.expected,
      });
    }
  });

  it("recovers when a different pointer starts while a session is active", () => {
    const manager = new CanvasInteractionManager();
    expect(
      manager.startDrag({
        itemId: "a",
        pointerId: 1,
        clientX: 0,
        clientY: 0,
        origin: { x: 0, y: 0 },
      }),
    ).toBe(true);

    expect(
      manager.startResize({
        itemId: "b",
        pointerId: 2,
        clientX: 0,
        clientY: 0,
        direction: "se",
        origin: { x: 0, y: 0, width: 100, height: 100 },
        minWidth: 10,
        minHeight: 10,
      }),
    ).toBe(true);

    const patch = manager.move(2, 30, 40, 1);
    expect(patch).toEqual({
      mode: "resize",
      itemId: "b",
      direction: "se",
      patch: { x: 0, y: 0, width: 130, height: 140 },
    });

    // Original pointer should no longer own the active session.
    expect(manager.move(1, 10, 10, 1)).toBeNull();
  });

  it("recovers from stale same-pointer session and starts new interaction", () => {
    const manager = new CanvasInteractionManager();
    expect(
      manager.startDrag({
        itemId: "a",
        pointerId: 7,
        clientX: 10,
        clientY: 10,
        origin: { x: 1, y: 1 },
      }),
    ).toBe(true);

    expect(
      manager.startResize({
        itemId: "b",
        pointerId: 7,
        clientX: 50,
        clientY: 50,
        direction: "se",
        origin: { x: 10, y: 10, width: 100, height: 100 },
        minWidth: 20,
        minHeight: 20,
      }),
    ).toBe(true);

    const patch = manager.move(7, 60, 70, 1);
    expect(patch?.mode).toBe("resize");
    expect(patch?.itemId).toBe("b");
  });

  it("supports force reset and active pointer introspection", () => {
    const manager = new CanvasInteractionManager();
    manager.startDrag({
      itemId: "x",
      pointerId: 99,
      clientX: 0,
      clientY: 0,
      origin: { x: 0, y: 0 },
    });

    expect(manager.hasActiveSession()).toBe(true);
    expect(manager.activePointerId()).toBe(99);

    manager.forceReset();

    expect(manager.hasActiveSession()).toBe(false);
    expect(manager.activePointerId()).toBeNull();
  });

  it("recovers from stale foreign-pointer sessions", () => {
    const manager = new CanvasInteractionManager();

    const nowSpy = vi.spyOn(Date, "now");
    nowSpy.mockReturnValue(1_000);

    expect(
      manager.startDrag({
        itemId: "first",
        pointerId: 1,
        clientX: 0,
        clientY: 0,
        origin: { x: 0, y: 0 },
      }),
    ).toBe(true);

    nowSpy.mockReturnValue(20_000);
    expect(
      manager.startResize({
        itemId: "second",
        pointerId: 2,
        clientX: 10,
        clientY: 10,
        direction: "se",
        origin: { x: 1, y: 2, width: 100, height: 100 },
        minWidth: 50,
        minHeight: 40,
      }),
    ).toBe(true);

    const patch = manager.move(2, 20, 20, 1);
    expect(patch?.itemId).toBe("second");

    nowSpy.mockRestore();
  });

  it("does not end session for non-owner pointer", () => {
    const manager = new CanvasInteractionManager();
    manager.startDrag({
      itemId: "keep",
      pointerId: 31,
      clientX: 0,
      clientY: 0,
      origin: { x: 5, y: 6 },
    });

    expect(manager.end(99)).toBeNull();
    expect(manager.hasActiveSession()).toBe(true);
    expect(manager.activePointerId()).toBe(31);
  });
});
