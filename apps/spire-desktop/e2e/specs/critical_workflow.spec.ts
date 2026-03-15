import { expect, test } from "@playwright/test";
import { WorkbenchPage } from "../page-objects/workbench.page";

test.describe("critical workflow", () => {
  test("model -> reaction -> diagram -> analysis -> compute grid", async ({ page }) => {
    const wb = new WorkbenchPage(page);
    const consoleErrors: string[] = [];

    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });

    await wb.goto();
    await wb.ensureDockingMode();

    // 1) Load model.
    await page.getByRole("button", { name: "Load Model" }).first().click();
    await expect(page.getByText(/loaded - \d+ fields, \d+ vertices/i).first()).toBeVisible();

    // 2) Run reaction pipeline.
    await page.getByRole("button", { name: "Run Full Pipeline" }).first().click();

    await expect
      .poll(async () => (await page.locator(".diagram-tabs .tab-btn").count()) > 0, {
        timeout: 30_000,
      })
      .toBeTruthy();

    // 3) Add analysis widget and run histogram job.
    await wb.addWidgetByLabel("Analysis");
    const eventInput = page.locator("#num-events").first();
    await eventInput.fill("500");
    await page.getByRole("button", { name: /Run Analysis/i }).first().click();
    await expect(page.getByText(/Events:/).first()).toBeVisible({ timeout: 30_000 });

    // 4) Add compute grid and launch a small job.
    await wb.addWidgetByLabel("Compute Grid");
    const gridEventsInput = page.locator("#grid-events").first();
    await gridEventsInput.fill("1000");
    await page.getByRole("button", { name: /Launch Grid/i }).first().click();

    await expect
      .poll(async () => {
        const complete = await page.getByText("✓ Complete").count();
        const progress = await page.locator(".progress-panel").count();
        return complete > 0 || progress > 0;
      }, { timeout: 45_000 })
      .toBeTruthy();

    expect(consoleErrors.filter((e) => e.includes("__TAURI_IPC__ is not a function"))).toEqual([]);
  });
});
