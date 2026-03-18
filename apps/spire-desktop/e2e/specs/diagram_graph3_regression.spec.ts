import { expect, test } from "@playwright/test";
import { WorkbenchPage } from "../page-objects/workbench.page";

test.describe("diagram visualizer graph #3", () => {
  test("feynman view stays responsive on the fourth diagram tab", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    const consoleErrors: string[] = [];
    const pageErrors: string[] = [];

    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    page.on("pageerror", (error) => {
      pageErrors.push(error.message);
    });

    await wb.goto();
    await wb.ensureDockingMode();

    await page.getByRole("button", { name: "Load Model" }).first().click();
    await expect(page.getByText(/loaded - \d+ fields, \d+ vertices/i).first()).toBeVisible();

    await page.getByRole("button", { name: "Run Full Pipeline" }).first().click();

    await expect
      .poll(async () => await page.locator(".diagram-tabs .tab-btn").count(), {
        timeout: 30_000,
      })
      .toBeGreaterThanOrEqual(4);

    await page.locator(".diagram-tabs .tab-btn", { hasText: "#3" }).first().click();

    await expect(page.locator(".summary-bar")).toContainText("1-Loop");
    await expect(page.locator(".svg-container svg")).toBeVisible();
    await expect(page.locator(".edge-details summary")).toContainText("Propagators");

    const ping = await page.evaluate(() => window.performance.now());
    expect(typeof ping).toBe("number");

    expect(pageErrors).toEqual([]);
    expect(consoleErrors.filter((message) => /RangeError|Maximum call stack|too much recursion/i.test(message))).toEqual([]);
  });
});
