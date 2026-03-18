import { beforeEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import {
  atlasPeriodicState,
  atlasPeriodicVisibleElements,
  initializeAtlasPeriodicStore,
  resetAtlasPeriodicStore,
  selectAtlasPeriodicElement,
  selectAtlasPeriodicIsotope,
  setAtlasPeriodicCategoryFilter,
  setAtlasPeriodicQuery,
} from "./atlasPeriodicStore";

describe("atlasPeriodicStore", () => {
  beforeEach(() => {
    resetAtlasPeriodicStore();
  });

  it("loads the periodic atlas registries and exposes filtered elements", async () => {
    await initializeAtlasPeriodicStore();

    expect(get(atlasPeriodicState).ready).toBe(true);
    expect(get(atlasPeriodicVisibleElements).length).toBeGreaterThan(100);

    setAtlasPeriodicQuery("oxygen");
    expect(get(atlasPeriodicVisibleElements).some((element) => element.symbol === "O")).toBe(true);

    setAtlasPeriodicCategoryFilter("noble-gas");
    expect(get(atlasPeriodicVisibleElements).every((element) => element.category === "noble-gas")).toBe(true);
  });

  it("keeps element and isotope selection in one shared state", async () => {
    await initializeAtlasPeriodicStore();

    const state = get(atlasPeriodicState);
    const hydrogen = state.elements.find((element) => element.Z === 1);
    const deuterium = state.isotopeRegistry["1"]?.find((isotope) => isotope.A === 2);

    expect(hydrogen).toBeTruthy();
    expect(deuterium).toBeTruthy();

    selectAtlasPeriodicElement(hydrogen!);
    expect(get(atlasPeriodicState).selectedElement?.symbol).toBe("H");
    expect(get(atlasPeriodicState).selectedIsotope).toBeNull();

    selectAtlasPeriodicIsotope(hydrogen!, deuterium!);
    expect(get(atlasPeriodicState).selectedIsotope?.A).toBe(2);
    expect(get(atlasPeriodicState).recentElements[0]?.symbol).toBe("H");
    expect(get(atlasPeriodicState).recentIsotopes[0]?.A).toBe(2);
  });
});
