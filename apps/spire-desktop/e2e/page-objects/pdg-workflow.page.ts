import { expect, type Locator, type Page } from "@playwright/test";
import { WorkbenchPage } from "./workbench.page";

export class PdgWorkflowPage {
  readonly workbench: WorkbenchPage;

  constructor(private readonly page: Page) {
    this.workbench = new WorkbenchPage(page);
  }

  widget(title: string): Locator {
    return this.page
      .locator(".widget-container")
      .filter({ has: this.page.locator(".wc-title", { hasText: title }) })
      .first();
  }

  async boot(): Promise<void> {
    await this.workbench.goto();
    await this.workbench.ensureDockingMode();
  }

  async ensureWidget(title: string): Promise<Locator> {
    const existing = this.widget(title);
    if (await existing.count()) {
      return existing;
    }

    await this.workbench.addWidgetByLabel(title);
    const mounted = this.widget(title);
    await expect(mounted).toBeVisible();
    return mounted;
  }

  async loadModelFromModelLoader(): Promise<void> {
    const modelLoader = await this.ensureWidget("Model Loader");
    const loadButton = modelLoader.getByRole("button", { name: "Load Model" }).first();
    await loadButton.click();

    await expect
      .poll(async () => {
        const scopedStatus = await modelLoader
          .getByText(/-\s*\d+\s+fields,\s*\d+\s+vertices/i)
          .count();
        const globalStatus = await this.page
          .getByText(/loaded\s*-\s*\d+\s+fields,\s*\d+\s+vertices/i)
          .count();
        return scopedStatus > 0 || globalStatus > 0;
      }, { timeout: 45_000 })
      .toBeTruthy();
  }

  async openAtlasPdgCatalog(): Promise<Locator> {
    const atlas = await this.ensureWidget("Particle Atlas");
    await atlas.getByRole("button", { name: "PDG Catalog" }).click();
    await expect(atlas.locator(".pdg-shell")).toBeVisible();
    return atlas;
  }

  async searchAtlas(query: string): Promise<void> {
    const atlas = this.widget("Particle Atlas");
    const search = atlas.getByPlaceholder("Search particles by name, symbol, or PDG ID...");
    await expect(search).toBeVisible();
    await search.fill(query);
  }

  async selectAtlasParticleByText(text: RegExp): Promise<void> {
    const atlas = this.widget("Particle Atlas");
    const row = atlas.locator(".particle-row").filter({ hasText: text }).first();
    await expect(row).toBeVisible({ timeout: 20_000 });
    await row.click({ force: true });
  }

  async dragAtlasSelectionToDecayCalculator(text?: RegExp): Promise<void> {
    const atlas = this.widget("Particle Atlas");
    const decay = await this.ensureWidget("Decay Calculator");

    const dragChip = text
      ? atlas.locator(".particle-row").filter({ hasText: text }).first()
      : atlas.locator(".particle-row.selected").first();
    const dropZone = decay.locator(".atlas-dropzone").first();

    await expect(dragChip).toBeVisible();
    await expect(dropZone).toBeVisible();

    const pdgId = Number((await dragChip.locator(".pdg-id").first().innerText()).trim());
    const label = (await dragChip.locator(".name").first().innerText()).trim();

    await dropZone.evaluate(
      (node, payload) => {
        const dataTransfer = new DataTransfer();
        dataTransfer.setData("application/x-spire-particle", JSON.stringify(payload));
        dataTransfer.setData("text/plain", payload.label ?? String(payload.pdgId));

        const event = new DragEvent("drop", {
          bubbles: true,
          cancelable: true,
          dataTransfer,
        });

        node.dispatchEvent(event);
      },
      {
        type: "pdg-particle",
        pdgId,
        label,
      },
    );
  }
}
