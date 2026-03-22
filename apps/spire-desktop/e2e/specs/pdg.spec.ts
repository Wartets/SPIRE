import { expect, test } from "@playwright/test";
import { PdgWorkflowPage } from "../page-objects/pdg-workflow.page";

test.describe("pdg integration workflows", () => {
  test("atlas selection propagates to decay calculator workflow", async ({ page }) => {
    const pdg = new PdgWorkflowPage(page);
    await pdg.boot();

    await pdg.loadModelFromModelLoader();
    await pdg.ensureWidget("Decay Calculator");
    await pdg.openAtlasPdgCatalog();

    await pdg.searchAtlas("top");
    await pdg.selectAtlasParticleByText(/top/i);

    const atlas = pdg.widget("Particle Atlas");
    await expect(atlas.locator(".particle-row").filter({ hasText: /top/i }).first()).toBeVisible();

    await pdg.dragAtlasSelectionToDecayCalculator(/top/i);

    const decay = pdg.widget("Decay Calculator");
    await expect(decay.locator("#decay-particle option:checked")).toContainText(/t|top/i);

    await decay.getByRole("button", { name: /Compute Decays/i }).click();
    await expect(decay.locator(".summary-banner")).toBeVisible({ timeout: 45_000 });
    await expect(decay.getByText(/Channels/i)).toBeVisible();
  });

  test("decay calculator toggles analytical and pdg modes", async ({ page }) => {
    const pdg = new PdgWorkflowPage(page);
    await pdg.boot();

    await pdg.loadModelFromModelLoader();
    const decay = await pdg.ensureWidget("Decay Calculator");

    const particleSelect = decay.locator("#decay-particle");
    const particleOptions = await particleSelect.locator("option").allTextContents();
    const zOption = particleOptions.find((option) => /(^|\s)z(\s|$)|z boson/i.test(option));
    if (zOption) {
      await particleSelect.selectOption({ label: zOption });
    }

    await decay.getByRole("button", { name: "PDG Experimental" }).click();
    await decay.locator("#decay-pdg-policy").selectOption("Catalog");
    await decay.getByRole("button", { name: /Compute Decays/i }).click();

    await expect(decay.getByRole("columnheader", { name: /PDG channel/i })).toBeVisible({
      timeout: 45_000,
    });

    const genericWarning = decay.getByText(/generic channels are shown/i).first();
    if (await genericWarning.count()) {
      await expect(genericWarning).toBeVisible();
    }

    await decay.locator("#decay-pdg-policy").selectOption("StrictPhysical");
    await decay.getByRole("button", { name: /Compute Decays/i }).click();
    await expect(decay.getByRole("columnheader", { name: /Type/i })).toBeVisible();

    await decay.getByRole("button", { name: "Side-by-Side" }).click();
    await expect(decay.getByRole("columnheader", { name: /Residual pull/i })).toBeVisible();

    await decay.getByRole("button", { name: /Analytical/i }).click();
    await expect(decay.getByRole("columnheader", { name: /Γpartial/i })).toBeVisible();
  });

  test("parameter scanner snaps to asymmetric pdg bounds", async ({ page }) => {
    const pdg = new PdgWorkflowPage(page);
    await pdg.boot();

    await pdg.loadModelFromModelLoader();
    const scanner = await pdg.ensureWidget("Parameter Scanner");

    await scanner.locator(".custom-toggle input[type='checkbox']").check();
    await scanner.locator("input.custom-input").fill("field.z.mass");

    const minInput = scanner.getByLabel("Minimum value").first();
    const maxInput = scanner.getByLabel("Maximum value").first();
    const beforeMin = await minInput.inputValue();
    const beforeMax = await maxInput.inputValue();

    await scanner.getByRole("button", { name: "PDG Bounds" }).click();

    await expect(scanner.getByText(/No PDG mapping|measurement unavailable/i)).toHaveCount(0);

    const afterMin = await minInput.inputValue();
    const afterMax = await maxInput.inputValue();

    await expect(scanner.getByText(/No PDG mapping|measurement unavailable|available for field mass\/width/i)).toHaveCount(0);

    const min = Number(afterMin);
    const max = Number(afterMax);
    expect(Number.isFinite(min)).toBeTruthy();
    expect(Number.isFinite(max)).toBeTruthy();
    expect(max).toBeGreaterThan(min);
    expect(Number(beforeMin)).toBeGreaterThanOrEqual(0);
    expect(Number(beforeMax)).toBeGreaterThan(Number(beforeMin));
  });
});
