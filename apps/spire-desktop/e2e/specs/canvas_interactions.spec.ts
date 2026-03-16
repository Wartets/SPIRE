import { expect, test, type Locator, type Page } from "@playwright/test";
import { WorkbenchPage } from "../page-objects/workbench.page";

function widgetByTitle(page: Page, title: string): Locator {
  return page
    .locator(".canvas-widget")
    .filter({ has: page.locator(".cw-title", { hasText: title }) })
    .first();
}

async function readWidgetBox(widget: Locator): Promise<{ left: number; top: number; width: number; height: number; z: number }> {
  return widget.evaluate((el) => {
    const asNumber = (value: string | null): number => Number.parseFloat(value ?? "0");
    const style = el as HTMLElement;
    const cs = window.getComputedStyle(style);
    return {
      left: asNumber(style.style.left),
      top: asNumber(style.style.top),
      width: asNumber(style.style.width),
      height: asNumber(style.style.height),
      z: Number.parseInt(cs.zIndex || "0", 10),
    };
  });
}

test.describe("canvas interaction engine", () => {
  test("z-index focus, drag and resize are reliable on desktop", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureCanvasMode();

    await wb.addWidgetByLabel("Notebook");
    await wb.addWidgetByLabel("Parameter Scanner");

    const notebook = widgetByTitle(page, "Notebook");
    const scanner = widgetByTitle(page, "Parameter Scanner");

    await expect(notebook).toBeVisible();
    await expect(scanner).toBeVisible();

    const transformBefore = await page.locator(".canvas-transform").getAttribute("style");

    await notebook.locator(".cw-body").click({ position: { x: 20, y: 20 } });
    const zNotebookAfterClick = (await readWidgetBox(notebook)).z;
    const zScannerAfterNotebookClick = (await readWidgetBox(scanner)).z;
    expect(zNotebookAfterClick).toBeGreaterThan(zScannerAfterNotebookClick);

    await scanner.locator(".cw-body").click({ position: { x: 20, y: 20 } });
    const zScannerAfterClick = (await readWidgetBox(scanner)).z;
    const zNotebookAfterScannerClick = (await readWidgetBox(notebook)).z;
    expect(zScannerAfterClick).toBeGreaterThan(zNotebookAfterScannerClick);

    const beforeDrag = await readWidgetBox(notebook);
    const header = notebook.locator(".cw-header");
    const headerBox = await header.boundingBox();
    if (!headerBox) throw new Error("Notebook header not measurable");

    await page.mouse.move(headerBox.x + 20, headerBox.y + 10);
    await page.mouse.down();
    await page.mouse.move(headerBox.x + 170, headerBox.y + 95);
    await page.mouse.up();

    await page.waitForTimeout(80);
    const afterDrag = await readWidgetBox(notebook);
    expect(afterDrag.left).toBeGreaterThan(beforeDrag.left + 80);
    expect(afterDrag.top).toBeGreaterThan(beforeDrag.top + 50);

    const transformAfterDrag = await page.locator(".canvas-transform").getAttribute("style");
    expect(transformAfterDrag).toBe(transformBefore);

    const beforeResize = await readWidgetBox(notebook);
    const seHandle = notebook.locator(".cw-resize-se");
    const seBox = await seHandle.boundingBox();
    if (!seBox) throw new Error("Notebook resize handle not measurable");

    await page.mouse.move(seBox.x + seBox.width / 2, seBox.y + seBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(seBox.x + 90, seBox.y + 70);
    await page.mouse.up();

    await page.waitForTimeout(80);
    const afterResize = await readWidgetBox(notebook);
    expect(afterResize.width).toBeGreaterThan(beforeResize.width + 40);
    expect(afterResize.height).toBeGreaterThan(beforeResize.height + 30);

    const nwHandle = notebook.locator(".cw-resize-nw");
    const nwBox = await nwHandle.boundingBox();
    if (!nwBox) throw new Error("Notebook north-west resize handle not measurable");

    await page.mouse.move(nwBox.x + nwBox.width / 2, nwBox.y + nwBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(nwBox.x + 1500, nwBox.y + 1500);
    await page.mouse.up();

    await page.waitForTimeout(80);
    const afterClampResize = await readWidgetBox(notebook);
    expect(afterClampResize.width).toBeGreaterThanOrEqual(280);
    expect(afterClampResize.height).toBeGreaterThanOrEqual(200);
  });
});

test.describe("canvas interaction engine - touch", () => {
  test.use({ hasTouch: true, isMobile: true, viewport: { width: 900, height: 1200 } });

  test("touch drag updates widget position", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureCanvasMode();

    await wb.addWidgetByLabel("Notebook");

    const notebook = widgetByTitle(page, "Notebook");
    await expect(notebook).toBeVisible();

    const before = await readWidgetBox(notebook);
    const header = notebook.locator(".cw-header");
    const box = await header.boundingBox();
    if (!box) throw new Error("Notebook header not measurable for touch");

    const startX = box.x + 24;
    const startY = box.y + 10;

    await header.dispatchEvent("pointerdown", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      buttons: 1,
      isPrimary: true,
      pressure: 0.5,
      clientX: startX,
      clientY: startY,
      bubbles: true,
    });
    await header.dispatchEvent("pointermove", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      buttons: 1,
      isPrimary: true,
      pressure: 0.5,
      clientX: startX + 120,
      clientY: startY + 90,
      bubbles: true,
    });
    await header.dispatchEvent("pointerup", {
      pointerId: 41,
      pointerType: "touch",
      button: 0,
      buttons: 0,
      isPrimary: true,
      pressure: 0,
      clientX: startX + 120,
      clientY: startY + 90,
      bubbles: true,
    });

    await page.waitForTimeout(140);
    const after = await readWidgetBox(notebook);
    expect(after.left).toBeGreaterThan(before.left + 60);
    expect(after.top).toBeGreaterThan(before.top + 40);
  });
});
