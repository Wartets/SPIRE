import { expect, test } from "@playwright/test";
import { ALL_WIDGET_LABELS } from "../fixtures/widgetLabels";
import { WorkbenchPage } from "../page-objects/workbench.page";

test.describe("cross-mode widget matrix", () => {
  test("all widgets can be spawned in docking and canvas; state survives reload", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();

    const initialCount = await wb.getWidgetCountValue();

    await wb.ensureDockingMode();
    for (const label of ALL_WIDGET_LABELS) {
      await wb.addWidgetByLabel(label);
    }

    await wb.ensureCanvasMode();
    for (const label of ALL_WIDGET_LABELS) {
      await wb.addWidgetByLabel(label);
    }

    const afterAddCount = await wb.getWidgetCountValue();
    expect(afterAddCount).toBeGreaterThan(initialCount);

    await page.reload();
    await expect(wb.widgetCountBadge()).toBeVisible();

    const afterReloadCount = await wb.getWidgetCountValue();
    expect(afterReloadCount).toBeGreaterThan(initialCount);
  });
});
