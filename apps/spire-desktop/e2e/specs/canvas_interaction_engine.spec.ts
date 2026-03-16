import { expect, test } from "@playwright/test";
import { WorkbenchPage } from "../page-objects/workbench.page";

function parsePx(value: string): number {
  const parsed = Number.parseFloat(value.replace("px", ""));
  return Number.isFinite(parsed) ? parsed : 0;
}

test.describe("canvas interaction engine", () => {
  test("supports bring-to-front, drag and resize in canvas mode", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureCanvasMode();

    const widgets = page.locator(".canvas-widget");
    await expect(widgets).toHaveCount(6);

    const first = widgets.nth(0);
    const second = widgets.nth(1);

    const firstHeader = first.locator(".cw-header");
    const firstStartBox = await first.boundingBox();
    const secondStartBox = await second.boundingBox();

    expect(firstStartBox).not.toBeNull();
    expect(secondStartBox).not.toBeNull();

    const firstHandleX = (firstStartBox?.x ?? 0) + 40;
    const firstHandleY = (firstStartBox?.y ?? 0) + 14;
    const targetX = (secondStartBox?.x ?? 0) + 30;
    const targetY = (secondStartBox?.y ?? 0) + 30;

    await firstHeader.hover({ position: { x: 20, y: 8 } });
    await page.mouse.move(firstHandleX, firstHandleY);
    await page.mouse.down();
    await page.mouse.move(targetX, targetY);
    await page.mouse.up();

    const firstAfterDrag = await first.boundingBox();
    expect(firstAfterDrag).not.toBeNull();
    expect((firstAfterDrag?.x ?? 0)).toBeGreaterThan((firstStartBox?.x ?? 0) + 40);

    const zAfterDragFirst = await first.evaluate((el) => getComputedStyle(el).zIndex);
    const zAfterDragSecond = await second.evaluate((el) => getComputedStyle(el).zIndex);
    expect(Number.parseInt(zAfterDragFirst, 10)).toBeGreaterThan(Number.parseInt(zAfterDragSecond, 10));

    // Pointer down in widget body must bring target to front.
    await second.locator(".cw-body").click({ position: { x: 30, y: 30 } });

    const zAfterBodyClickFirst = await first.evaluate((el) => getComputedStyle(el).zIndex);
    const zAfterBodyClickSecond = await second.evaluate((el) => getComputedStyle(el).zIndex);
    expect(Number.parseInt(zAfterBodyClickSecond, 10)).toBeGreaterThan(Number.parseInt(zAfterBodyClickFirst, 10));

    // Resize the now-foreground second widget from south-east handle.
    const secondResizeHandle = second.locator(".cw-resize-se");
    const secondBeforeResize = await second.evaluate((el) => {
      const style = (el as HTMLElement).style;
      return {
        width: style.width,
        height: style.height,
      };
    });

    const resizeBox = await secondResizeHandle.boundingBox();
    expect(resizeBox).not.toBeNull();

    const rx = (resizeBox?.x ?? 0) + (resizeBox?.width ?? 0) / 2;
    const ry = (resizeBox?.y ?? 0) + (resizeBox?.height ?? 0) / 2;

    await page.mouse.move(rx, ry);
    await page.mouse.down();
    await page.mouse.move(rx + 100, ry + 80);
    await page.mouse.up();

    const secondAfterResize = await second.evaluate((el) => {
      const style = (el as HTMLElement).style;
      return {
        width: style.width,
        height: style.height,
      };
    });

    expect(parsePx(secondAfterResize.width)).toBeGreaterThan(parsePx(secondBeforeResize.width));
    expect(parsePx(secondAfterResize.height)).toBeGreaterThan(parsePx(secondBeforeResize.height));
  });

  test("handles touch pointer drag without viewport pan interference", async ({ browser }) => {
    const context = await browser.newContext({
      viewport: { width: 430, height: 920 },
      hasTouch: true,
      isMobile: true,
    });
    const page = await context.newPage();

    await page.addInitScript(() => {
      window.localStorage.removeItem("spire_workspace_state");
      window.localStorage.removeItem("spire.toolbox.prefs.v2");
      window.sessionStorage.removeItem("spire.e2e.bootstrap.cleared");
    });

    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureCanvasMode();

    const first = page.locator(".canvas-widget").first();
    const firstHeader = first.locator(".cw-header");
    await expect(first).toBeVisible();
    await expect(firstHeader).toBeVisible();

    const before = await first.boundingBox();
    expect(before).not.toBeNull();

    const gestureX = (before?.x ?? 0) + 40;
    const gestureY = (before?.y ?? 0) + 16;

    await firstHeader.dispatchEvent("pointerdown", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      clientX: gestureX,
      clientY: gestureY,
      bubbles: true,
    });

    await firstHeader.dispatchEvent("pointermove", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      clientX: gestureX + 80,
      clientY: gestureY + 60,
      bubbles: true,
    });

    await firstHeader.dispatchEvent("pointerup", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      clientX: gestureX + 80,
      clientY: gestureY + 60,
      bubbles: true,
    });

    const after = await first.boundingBox();
    expect(after).not.toBeNull();
    expect((after?.x ?? 0)).toBeGreaterThan((before?.x ?? 0) + 20);

    await context.close();
  });
});
