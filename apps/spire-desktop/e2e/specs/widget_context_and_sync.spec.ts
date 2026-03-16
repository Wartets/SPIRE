import { expect, test } from "@playwright/test";
import { ALL_WIDGET_LABELS } from "../fixtures/widgetLabels";
import { WorkbenchPage } from "../page-objects/workbench.page";

test.describe("widget context + interop", () => {
  test("each widget exposes documentation in header context menu", async ({ page }) => {
    test.setTimeout(240_000);

    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureDockingMode();

    for (const label of ALL_WIDGET_LABELS) {
      await wb.addWidgetByLabel(label);

      const widget = page
        .locator(".widget-container")
        .filter({ has: page.locator(".wc-title", { hasText: label }) })
        .last();

      await expect(widget).toHaveCount(1);
      await widget.locator(".wc-title").evaluate((el) => {
        el.dispatchEvent(
          new MouseEvent("contextmenu", {
            bubbles: true,
            cancelable: true,
            clientX: 280,
            clientY: 220,
          }),
        );
      });
      await expect(page.locator(".ctx-item .ctx-label", { hasText: "Open Documentation" })).toBeVisible();
      await page.keyboard.press("Escape");
    }
  });

  test("reaction settings propagate to analysis and compute grid", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureDockingMode();

    const reactionWidget = page
      .locator(".widget-container")
      .filter({ has: page.locator(".wc-title", { hasText: "Reaction Workspace" }) })
      .first();
    await expect(reactionWidget).toBeVisible();
    await reactionWidget.locator(".wc-title").click({ button: "right", force: true });
    await page.locator(".ctx-item .ctx-label", { hasText: "√s = 250 GeV" }).click();

    await wb.addWidgetByLabel("Analysis");
    await wb.addWidgetByLabel("Compute Grid");

    await expect(page.locator("#cms-energy")).toHaveValue("250");
    await expect(page.locator("#grid-cms")).toHaveValue("250");
  });
});
