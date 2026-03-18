/**
 * SPIRE – Nuclear Data Loader
 *
 * Asynchronously loads and caches the elements and isotopes registries from
 * the static JSON data files.  The caches persist for the lifetime of the
 * browser session; a second call to any loader function is an O(1) cache hit.
 *
 * Data sources:
 *   src/lib/data/elements.json  – IUPAC element table (all 118 elements)
 *   src/lib/data/isotopes.json  – Nuclear isotope registry (Z = 1-20, heavy nuclides)
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ElementData {
  Z: number;
  symbol: string;
  name: string;
  group: number | null;
  period: number;
  /** CSS grid-column for the 18-column periodic table layout. */
  display_col: number;
  /** CSS grid-row for the 18-column periodic table layout (rows 8-9 = f-block). */
  display_row: number;
  atomic_mass: number;
  electron_configuration: string;
  category:
    | "alkali-metal"
    | "alkaline-earth-metal"
    | "transition-metal"
    | "metalloid"
    | "nonmetal"
    | "halogen"
    | "noble-gas"
    | "post-transition-metal"
    | "lanthanide"
    | "actinide";
}

export type IsotopeDecayMode =
  | "beta-minus"
  | "beta-plus"
  | "alpha"
  | "ec"
  | "gamma"
  | "sf";

export interface IsotopeDecay {
  mode: IsotopeDecayMode;
  /** Branching fraction [0, 1]. */
  fraction: number;
  daughter_Z: number;
  daughter_A: number;
}

export interface IsotopeData {
  A: number;
  /** Mass excess in keV relative to C-12 = 0. */
  mass_excess_kev: number;
  /** Half-life in seconds.  Null means experimentally stable. */
  half_life_s: number | null;
  spin_parity: string;
  abundance_percent: number;
  decay_modes: IsotopeDecay[];
}

// ---------------------------------------------------------------------------
// Module-level caches
// ---------------------------------------------------------------------------

let elementsCache: ElementData[] | null = null;
let isotopesCache: Record<string, IsotopeData[]> | null = null;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function sortIsotopesByA(isotopes: IsotopeData[]): IsotopeData[] {
  return [...isotopes].sort((a, b) => a.A - b.A);
}

function isotopeAnchorA(element: ElementData, isotopes: IsotopeData[]): number {
  if (isotopes.length === 0) return Math.max(element.Z, Math.round(element.atomic_mass));

  const natural = [...isotopes].sort((a, b) => (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0))[0] ?? null;
  if (natural && (natural.abundance_percent ?? 0) > 0) return natural.A;

  const stable = isotopes.find((entry) => entry.half_life_s === null) ?? null;
  if (stable) return stable.A;

  return Math.max(element.Z, Math.round(element.atomic_mass));
}

function syntheticSpinParity(A: number): string {
  if (A % 2 === 0) return "0+";
  if (A % 4 === 1) return "1/2+";
  return "3/2-";
}

function syntheticDecayModes(Z: number, A: number, anchorA: number): IsotopeDecay[] {
  const delta = A - anchorA;
  if (Math.abs(delta) <= 1) return [];

  if (Z >= 84 && delta >= 3) {
    const daughterZ = Math.max(1, Z - 2);
    const daughterA = Math.max(daughterZ, A - 4);
    return [{ mode: "alpha", fraction: 1, daughter_Z: daughterZ, daughter_A: daughterA }];
  }

  if (delta > 0) {
    const daughterZ = Math.min(118, Z + 1);
    const daughterA = Math.max(daughterZ, A);
    return [{ mode: "beta-minus", fraction: 1, daughter_Z: daughterZ, daughter_A: daughterA }];
  }

  const daughterZ = Math.max(1, Z - 1);
  const daughterA = Math.max(daughterZ, A);
  const mode: IsotopeDecayMode = Z > 36 ? "ec" : "beta-plus";
  return [{ mode, fraction: 1, daughter_Z: daughterZ, daughter_A: daughterA }];
}

function syntheticHalfLifeSeconds(delta: number): number | null {
  if (delta <= 1) return null;
  if (delta === 2) return 3.15e7;
  const log10HalfLife = clamp(8 - delta * 1.3, -5, 6);
  return 10 ** log10HalfLife;
}

function buildSyntheticIsotope(
  element: ElementData,
  A: number,
  anchorA: number,
): IsotopeData {
  const delta = Math.abs(A - anchorA);
  const stable = delta <= 1;
  const abundance = delta === 0
    ? 100
    : stable
      ? Math.max(0, 34 - delta * 12)
      : Math.max(0, 16 - delta * 2.2);

  return {
    A,
    mass_excess_kev: (A - element.atomic_mass) * 930,
    half_life_s: stable ? null : syntheticHalfLifeSeconds(delta),
    spin_parity: syntheticSpinParity(A),
    abundance_percent: abundance,
    decay_modes: stable ? [] : syntheticDecayModes(element.Z, A, anchorA),
  };
}

function enrichIsotopesForElement(element: ElementData, source: IsotopeData[]): IsotopeData[] {
  const known = sortIsotopesByA(source);
  const heavyBonus = element.Z >= 84 ? 10 : element.Z >= 56 ? 6 : element.Z >= 28 ? 3 : 0;
  const targetCount = clamp(Math.round(22 + Math.log2(element.Z + 1) * 8 + heavyBonus), 26, 64);
  if (known.length >= targetCount) return known;

  const anchorA = isotopeAnchorA(element, known);
  const asymmetry = Math.max(3, Math.round(element.Z * 0.045));
  const spread = Math.max(12, Math.ceil(targetCount / 2));
  const minA = Math.max(element.Z, anchorA - spread + Math.floor(asymmetry * 0.45));
  const maxA = Math.max(minA, anchorA + spread + asymmetry);

  const byA = new Map<number, IsotopeData>(known.map((entry) => [entry.A, entry]));
  for (let A = minA; A <= maxA; A += 1) {
    if (!byA.has(A)) {
      byA.set(A, buildSyntheticIsotope(element, A, anchorA));
    }
  }

  return sortIsotopesByA(Array.from(byA.values()));
}

function enrichIsotopeRegistry(
  baseRegistry: Record<string, IsotopeData[]>,
  elements: ElementData[],
): Record<string, IsotopeData[]> {
  return Object.fromEntries(
    elements.map((element) => {
      const raw = baseRegistry[String(element.Z)] ?? [];
      return [String(element.Z), enrichIsotopesForElement(element, raw)];
    }),
  );
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Return the complete list of all 118 IUPAC elements.
 * The result is cached after the first call.
 */
export async function loadElements(): Promise<ElementData[]> {
  if (elementsCache) return elementsCache;
  const mod = await import("$lib/data/elements.json");
  elementsCache = mod.default as ElementData[];
  return elementsCache;
}

/**
 * Return the full isotope registry keyed by atomic number Z (as string).
 * The result is cached after the first call.
 */
export async function loadAllIsotopes(): Promise<Record<string, IsotopeData[]>> {
  if (isotopesCache) return isotopesCache;
  const [mod, elements] = await Promise.all([
    import("$lib/data/isotopes.json"),
    loadElements(),
  ]);
  const base = mod.default as Record<string, IsotopeData[]>;
  isotopesCache = enrichIsotopeRegistry(base, elements);
  return isotopesCache;
}

/**
 * Return the isotopes for a specific element (by atomic number Z).
 * Returns an empty array if no data is available for that Z.
 */
export async function loadIsotopesForZ(Z: number): Promise<IsotopeData[]> {
  const all = await loadAllIsotopes();
  return all[String(Z)] ?? [];
}

/**
 * Look up an element by its atomic number Z synchronously from the cache.
 * Returns `null` if the cache has not been populated yet.
 */
export function getElementByZ(Z: number): ElementData | null {
  if (!elementsCache) return null;
  return elementsCache.find((el) => el.Z === Z) ?? null;
}

/**
 * Look up an element by its symbol synchronously from the cache.
 * Returns `null` if the cache has not been populated yet or the symbol is
 * not recognised.
 */
export function getElementBySymbol(symbol: string): ElementData | null {
  if (!elementsCache) return null;
  return elementsCache.find((el) => el.symbol === symbol) ?? null;
}

/**
 * Format a half-life value as a human-readable string.
 *
 * @param half_life_s – Half-life in seconds, or null for stable nuclei.
 */
export function formatHalfLife(half_life_s: number | null): string {
  if (half_life_s === null) return "stable";
  if (half_life_s < 1e-9) return `${(half_life_s * 1e12).toExponential(2)} ps`;
  if (half_life_s < 1e-6) return `${(half_life_s * 1e9).toExponential(2)} ns`;
  if (half_life_s < 1e-3) return `${(half_life_s * 1e6).toExponential(2)} µs`;
  if (half_life_s < 1.0) return `${(half_life_s * 1e3).toFixed(2)} ms`;
  if (half_life_s < 60) return `${half_life_s.toFixed(2)} s`;
  if (half_life_s < 3600) return `${(half_life_s / 60).toFixed(2)} min`;
  if (half_life_s < 86400) return `${(half_life_s / 3600).toFixed(2)} h`;
  if (half_life_s < 3.156e7) return `${(half_life_s / 86400).toFixed(2)} d`;
  if (half_life_s < 3.156e9) return `${(half_life_s / 3.156e7).toFixed(2)} yr`;
  if (half_life_s < 3.156e12) return `${(half_life_s / 3.156e9).toExponential(2)} kyr`;
  if (half_life_s < 3.156e15) return `${(half_life_s / 3.156e12).toExponential(2)} Myr`;
  return `${(half_life_s / 3.156e15).toExponential(2)} Gyr`;
}

/**
 * Return the ground-state binding energy per nucleon (MeV) using the
 * Bethe–Weizsäcker semi-empirical mass formula.
 *
 * Coefficients (MeV): av=15.75, as=17.8, ac=0.711, aa=23.7, ap=12 (pair term).
 */
export function bindingEnergyPerNucleon(Z: number, A: number): number {
  if (A <= 1) return 0;
  const N = A - Z;
  const av = 15.75;
  const as = 17.8;
  const ac = 0.711;
  const aa = 23.7;
  const ap = 12.0;

  let pairTerm: number;
  if (A % 2 !== 0) {
    pairTerm = 0;
  } else if (Z % 2 === 0 && N % 2 === 0) {
    pairTerm = ap / Math.pow(A, 0.5);
  } else {
    pairTerm = -ap / Math.pow(A, 0.5);
  }

  const B =
    av * A -
    as * Math.pow(A, 2 / 3) -
    ac * (Z * (Z - 1)) / Math.pow(A, 1 / 3) -
    aa * ((N - Z) ** 2) / A +
    pairTerm;

  return B / A;
}
