import { expect, type Locator, type Page } from "@playwright/test";

export class WorkbenchPage {
  constructor(private readonly page: Page) {}

  async goto(): Promise<void> {
    await this.page.goto("/");
    await expect(this.page.getByRole("button", { name: "+ Add Widget" })).toBeVisible();
  }

  async openAddWidgetMenu(): Promise<void> {
    const addWidgetButton = this.page.getByRole("button", { name: "+ Add Widget" });
    await addWidgetButton.click();
    await expect(this.page.locator(".toolbox-menu")).toBeVisible();
  }

  async addWidgetByLabel(label: string): Promise<void> {
    await this.openAddWidgetMenu();
    await this.page.getByRole("button", { name: label, exact: true }).click();
    await expect(this.page.locator(".toolbox-menu")).toBeHidden();
  }

  async ensureDockingMode(): Promise<void> {
    const modeButton = this.modeToggleButton();
    const text = (await modeButton.innerText()).trim();
    if (text.includes("Canvas")) {
      await modeButton.click();
      await expect(modeButton).toContainText("Docking");
    }
  }

  async ensureCanvasMode(): Promise<void> {
    const modeButton = this.modeToggleButton();
    const text = (await modeButton.innerText()).trim();
    if (text.includes("Docking")) {
      await modeButton.click();
      await expect(modeButton).toContainText("Canvas");
    }
  }

  modeToggleButton(): Locator {
    return this.page.locator(".toolbox-mode-toggle");
  }

  widgetCountBadge(): Locator {
    return this.page.locator(".widget-count");
  }

  async getWidgetCountValue(): Promise<number> {
    const txt = (await this.widgetCountBadge().innerText()).trim();
    const match = txt.match(/(\d+)/);
    return match ? Number.parseInt(match[1], 10) : 0;
  }
}
