import { expect, test } from "@playwright/test";
import { WorkbenchPage } from "../page-objects/workbench.page";

test.describe("widget internet + interop", () => {
  test("reaction context propagates to event display and parameter scanner", async ({ page }) => {
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

    await wb.addWidgetByLabel("Event Display");
    await wb.addWidgetByLabel("Parameter Scanner");

    await expect(page.locator("#ed-energy")).toHaveValue("250");
    await expect(page.locator("#scan-min")).toHaveValue("50");
    await expect(page.locator("#scan-max")).toHaveValue("500");
  });

  test("widget body context menu exposes internet actions for selected content", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    await wb.goto();
    await wb.ensureDockingMode();

    await wb.addWidgetByLabel("Plugin Manager");

    const pluginWidget = page
      .locator(".widget-container")
      .filter({ has: page.locator(".wc-title", { hasText: "Plugin Manager" }) })
      .last();

    await expect(pluginWidget).toBeVisible();

    const hint = pluginWidget.locator(".pm-empty-hint").first();
    await expect(hint).toBeVisible();

    await hint.evaluate((el) => {
      const range = document.createRange();
      range.selectNodeContents(el);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);

      el.dispatchEvent(
        new MouseEvent("contextmenu", {
          bubbles: true,
          cancelable: true,
          clientX: 300,
          clientY: 260,
        }),
      );
    });

    await expect(page.locator(".ctx-item .ctx-label", { hasText: "Search Web for" })).toBeVisible();
    await expect(page.locator(".ctx-item .ctx-label", { hasText: "Search arXiv for Selection" })).toBeVisible();
  });
});
