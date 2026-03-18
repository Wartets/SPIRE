import { derived, get, writable } from "svelte/store";
import {
  loadAllIsotopes,
  loadElements,
  type ElementData,
  type IsotopeData,
} from "$lib/core/physics/nuclearDataLoader";
import { broadcastSelection } from "$lib/stores/selectionBus";

export type AtlasPeriodicCategoryFilter = "all" | ElementData["category"];

export interface AtlasPeriodicIsotopeSelection {
  Z: number;
  A: number;
  symbol: string;
  name: string;
  isotopeData: IsotopeData;
  element: ElementData;
}

interface IsotopeStats {
  count: number;
  stable: number;
  naturalA: number | null;
  heavyA: number | null;
}

interface AtlasPeriodicState {
  ready: boolean;
  loading: boolean;
  elements: ElementData[];
  isotopeRegistry: Record<string, IsotopeData[]>;
  isotopeStats: Record<number, IsotopeStats>;
  query: string;
  categoryFilter: AtlasPeriodicCategoryFilter;
  selectedElement: ElementData | null;
  selectedIsotope: AtlasPeriodicIsotopeSelection | null;
  recentElements: ElementData[];
  recentIsotopes: AtlasPeriodicIsotopeSelection[];
}

const initialState: AtlasPeriodicState = {
  ready: false,
  loading: false,
  elements: [],
  isotopeRegistry: {},
  isotopeStats: {},
  query: "",
  categoryFilter: "all",
  selectedElement: null,
  selectedIsotope: null,
  recentElements: [],
  recentIsotopes: [],
};

const periodicState = writable<AtlasPeriodicState>(initialState);

function buildIsotopeStats(
  elements: ElementData[],
  isotopeRegistry: Record<string, IsotopeData[]>,
): Record<number, IsotopeStats> {
  return Object.fromEntries(
    elements.map((element) => {
      const isotopes = isotopeRegistry[String(element.Z)] ?? [];
      const stable = isotopes.filter((entry) => entry.half_life_s === null).length;
      const natural = [...isotopes].sort((a, b) => (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0))[0] ?? null;
      const heavy = [...isotopes].sort((a, b) => b.A - a.A)[0] ?? null;
      return [element.Z, {
        count: isotopes.length,
        stable,
        naturalA: natural?.A ?? null,
        heavyA: heavy?.A ?? null,
      }];
    }),
  );
}

function pushRecentElement(
  entries: ElementData[],
  element: ElementData,
): ElementData[] {
  return [element, ...entries.filter((entry) => entry.Z !== element.Z)].slice(0, 8);
}

function pushRecentIsotope(
  entries: AtlasPeriodicIsotopeSelection[],
  isotope: AtlasPeriodicIsotopeSelection,
): AtlasPeriodicIsotopeSelection[] {
  return [
    isotope,
    ...entries.filter((entry) => !(entry.Z === isotope.Z && entry.A === isotope.A)),
  ].slice(0, 8);
}

function toIsotopeSelection(
  element: ElementData,
  isotopeData: IsotopeData,
): AtlasPeriodicIsotopeSelection {
  return {
    Z: element.Z,
    A: isotopeData.A,
    symbol: element.symbol,
    name: element.name,
    isotopeData,
    element,
  };
}

function mostProbableIsotope(
  element: ElementData,
  isotopeRegistry: Record<string, IsotopeData[]>,
  isotopeStats: Record<number, IsotopeStats>,
): IsotopeData | null {
  const isotopes = isotopeRegistry[String(element.Z)] ?? [];
  if (isotopes.length === 0) return null;

  const naturalA = isotopeStats[element.Z]?.naturalA ?? null;
  if (naturalA != null) {
    const natural = isotopes.find((entry) => entry.A === naturalA) ?? null;
    if (natural) return natural;
  }

  let best: IsotopeData | null = null;
  let bestScore = Number.POSITIVE_INFINITY;

  for (const isotope of isotopes) {
    const abundancePenalty = 1 - Math.min(1, Math.max(0, (isotope.abundance_percent ?? 0) / 100));
    const stabilityPenalty = isotope.half_life_s === null ? 0 : 0.25;
    const massPenalty = Math.abs(isotope.A - element.atomic_mass) * 0.02;
    const score = abundancePenalty + stabilityPenalty + massPenalty;
    if (score < bestScore) {
      best = isotope;
      bestScore = score;
    }
  }

  return best;
}

export const atlasPeriodicState = derived(periodicState, ($state) => $state);

export const atlasPeriodicVisibleElements = derived(periodicState, ($state) => {
  const query = $state.query.trim().toLowerCase();
  return $state.elements.filter((element) => {
    const categoryMatch = $state.categoryFilter === "all" || element.category === $state.categoryFilter;
    if (!categoryMatch) return false;
    if (!query) return true;
    return (
      element.name.toLowerCase().includes(query) ||
      element.symbol.toLowerCase().includes(query) ||
      String(element.Z).includes(query)
    );
  });
});

export const atlasPeriodicSelectedIsotopes = derived(periodicState, ($state) => {
  if (!$state.selectedElement) return [];
  return $state.isotopeRegistry[String($state.selectedElement.Z)] ?? [];
});

export async function initializeAtlasPeriodicStore(): Promise<void> {
  const state = get(periodicState);
  if (state.ready || state.loading) return;

  periodicState.update((current) => ({ ...current, loading: true }));

  const [elements, isotopeRegistry] = await Promise.all([
    loadElements(),
    loadAllIsotopes(),
  ]);

  periodicState.update((current) => ({
    ...current,
    ready: true,
    loading: false,
    elements,
    isotopeRegistry,
    isotopeStats: buildIsotopeStats(elements, isotopeRegistry),
  }));
}

export function setAtlasPeriodicQuery(query: string): void {
  periodicState.update((state) => ({ ...state, query }));
}

export function setAtlasPeriodicCategoryFilter(categoryFilter: AtlasPeriodicCategoryFilter): void {
  periodicState.update((state) => ({ ...state, categoryFilter }));
}

export function clearAtlasPeriodicSelection(): void {
  periodicState.update((state) => ({
    ...state,
    selectedElement: null,
    selectedIsotope: null,
  }));
}

export function resetAtlasPeriodicStore(): void {
  periodicState.set(initialState);
}

export function selectAtlasPeriodicElement(element: ElementData): void {
  periodicState.update((state) => {
    const probable = mostProbableIsotope(element, state.isotopeRegistry, state.isotopeStats);
    const isotopeSelection = probable ? toIsotopeSelection(element, probable) : null;

    return {
      ...state,
      selectedElement: element,
      selectedIsotope: isotopeSelection,
      recentElements: pushRecentElement(state.recentElements, element),
      recentIsotopes: isotopeSelection
        ? pushRecentIsotope(state.recentIsotopes, isotopeSelection)
        : state.recentIsotopes,
    };
  });

  broadcastSelection({ type: "ELEMENT_SELECTED", data: element });

  const state = get(periodicState);
  if (state.selectedIsotope) {
    broadcastSelection({
      type: "ISOTOPE_SELECTED",
      data: {
        Z: state.selectedIsotope.Z,
        A: state.selectedIsotope.A,
        symbol: state.selectedIsotope.symbol,
        name: state.selectedIsotope.name,
        isotope: state.selectedIsotope.isotopeData,
      },
    });
  }
}

export function selectAtlasPeriodicElementByZ(Z: number): void {
  const state = get(periodicState);
  const element = state.elements.find((entry) => entry.Z === Z) ?? null;
  if (!element) return;
  selectAtlasPeriodicElement(element);
}

export function selectAtlasPeriodicIsotope(
  element: ElementData,
  isotopeData: IsotopeData,
): void {
  const isotopeSelection = toIsotopeSelection(element, isotopeData);
  periodicState.update((state) => ({
    ...state,
    selectedElement: element,
    selectedIsotope: isotopeSelection,
    recentElements: pushRecentElement(state.recentElements, element),
    recentIsotopes: pushRecentIsotope(state.recentIsotopes, isotopeSelection),
  }));

  broadcastSelection({
    type: "ISOTOPE_SELECTED",
    data: {
      Z: isotopeSelection.Z,
      A: isotopeSelection.A,
      symbol: isotopeSelection.symbol,
      name: isotopeSelection.name,
      isotope: isotopeSelection.isotopeData,
    },
  });
}

export function selectAtlasPeriodicIsotopeByZA(Z: number, A: number): void {
  const state = get(periodicState);
  const element = state.elements.find((entry) => entry.Z === Z) ?? null;
  const isotopeData = state.isotopeRegistry[String(Z)]?.find((entry) => entry.A === A) ?? null;
  if (!element || !isotopeData) return;
  selectAtlasPeriodicIsotope(element, isotopeData);
}

export function setAtlasPeriodicSelectionFromIsotope(
  isotope: AtlasPeriodicIsotopeSelection,
): void {
  selectAtlasPeriodicIsotope(isotope.element, isotope.isotopeData);
}
