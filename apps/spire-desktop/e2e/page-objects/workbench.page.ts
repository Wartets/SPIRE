import { expect, type Locator, type Page } from "@playwright/test";

export class WorkbenchPage {
  constructor(private readonly page: Page) {}

  private escapeRegex(input: string): string {
    return input.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  async goto(): Promise<void> {
    await this.page.goto("/");
    await this.page.evaluate(() => {
      window.localStorage.removeItem("spire.toolbox.prefs.v2");
      window.localStorage.removeItem("spire_workspace_state");
    });
    await this.page.reload();
    await expect(this.page.getByRole("button", { name: "+ Add Widget" })).toBeVisible();
  }

  async openAddWidgetMenu(): Promise<void> {
    const addWidgetButton = this.page.getByRole("button", { name: "+ Add Widget" });
    await addWidgetButton.click();
    await expect(this.page.locator(".toolbox-menu")).toBeVisible();
  }

  async addWidgetByLabel(label: string): Promise<void> {
    await this.openAddWidgetMenu();
    const search = this.page.locator(".toolbox-search");
    await expect(search).toBeVisible();
    await search.fill(label);

    const escaped = this.escapeRegex(label.trim());
    const widgetButton = this.page
      .getByRole("button", { name: new RegExp(`^${escaped}(\\s+Drag)?$`) })
      .first();
    await expect(widgetButton).toBeVisible();
    await widgetButton.click();
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
