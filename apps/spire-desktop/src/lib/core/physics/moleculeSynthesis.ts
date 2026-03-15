import {
  loadElements,
  loadAllIsotopes,
  type ElementData,
  type IsotopeData,
} from "$lib/core/physics/nuclearDataLoader";

export interface MoleculeToken {
  symbol: string;
  count: number;
  element: ElementData;
}

export interface IsotopeRecommendation {
  symbol: string;
  count: number;
  element: ElementData;
  natural: IsotopeData | null;
  enriched: IsotopeData | null;
  allCandidates: IsotopeData[];
}

export interface MoleculeSynthesisResult {
  formula: string;
  tokens: MoleculeToken[];
  recommendations: IsotopeRecommendation[];
  estimatedNaturalMass: number;
  estimatedEnrichedMass: number;
}

function normalizeFormula(input: string): string {
  return input.replace(/\s+/g, "").trim();
}

function parseInteger(input: string, start: number): { value: number; next: number } {
  let i = start;
  while (i < input.length && /\d/.test(input[i])) i += 1;
  if (i === start) return { value: 1, next: start };
  return { value: Math.max(1, Number.parseInt(input.slice(start, i), 10)), next: i };
}

function parseSegment(formula: string, start: number): { counts: Map<string, number>; next: number } {
  const counts = new Map<string, number>();
  let i = start;

  while (i < formula.length) {
    const ch = formula[i];
    if (ch === ")") {
      return { counts, next: i + 1 };
    }

    if (ch === "(") {
      const nested = parseSegment(formula, i + 1);
      const mul = parseInteger(formula, nested.next);
      nested.counts.forEach((count, symbol) => {
        counts.set(symbol, (counts.get(symbol) ?? 0) + count * mul.value);
      });
      i = mul.next;
      continue;
    }

    if (!/[A-Z]/.test(ch)) {
      throw new Error(`Unexpected token '${ch}' at position ${i + 1}`);
    }

    let j = i + 1;
    while (j < formula.length && /[a-z]/.test(formula[j])) j += 1;
    const symbol = formula.slice(i, j);
    const amount = parseInteger(formula, j);
    counts.set(symbol, (counts.get(symbol) ?? 0) + amount.value);
    i = amount.next;
  }

  return { counts, next: i };
}

function preferredNatural(isotopes: IsotopeData[]): IsotopeData | null {
  if (isotopes.length === 0) return null;
  return [...isotopes].sort((a, b) => {
    const abundanceCmp = (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0);
    if (abundanceCmp !== 0) return abundanceCmp;
    const stableA = a.half_life_s === null ? Number.POSITIVE_INFINITY : a.half_life_s;
    const stableB = b.half_life_s === null ? Number.POSITIVE_INFINITY : b.half_life_s;
    return stableB - stableA;
  })[0];
}

function preferredEnriched(isotopes: IsotopeData[], targetA: number): IsotopeData | null {
  if (isotopes.length === 0) return null;
  return [...isotopes].sort((a, b) => {
    const dA = Math.abs(a.A - targetA) - Math.abs(b.A - targetA);
    if (dA !== 0) return dA;
    return (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0);
  })[0];
}

export async function synthesizeMoleculeIsotopes(formulaInput: string): Promise<MoleculeSynthesisResult> {
  const formula = normalizeFormula(formulaInput);
  if (!formula) {
    throw new Error("Formula is empty.");
  }

  const { counts, next } = parseSegment(formula, 0);
  if (next !== formula.length) {
    throw new Error("Formula has unmatched parentheses.");
  }

  const elements = await loadElements();
  const bySymbol = new Map(elements.map((el) => [el.symbol, el]));

  const tokens = [...counts.entries()].map(([symbol, count]) => {
    const element = bySymbol.get(symbol);
    if (!element) {
      throw new Error(`Unknown element symbol: ${symbol}`);
    }
    return { symbol, count, element } satisfies MoleculeToken;
  });

  const allIsotopes = await loadAllIsotopes();

  const recommendations: IsotopeRecommendation[] = tokens.map((token) => {
    const iso = allIsotopes[String(token.element.Z)] ?? [];
    const natural = preferredNatural(iso);
    const enriched = preferredEnriched(iso, Math.max(1, Math.round(token.element.atomic_mass)));
    return {
      symbol: token.symbol,
      count: token.count,
      element: token.element,
      natural,
      enriched,
      allCandidates: iso,
    };
  });

  const estimatedNaturalMass = recommendations.reduce((acc, rec) => {
    const chosenA = rec.natural?.A ?? Math.round(rec.element.atomic_mass);
    return acc + chosenA * rec.count;
  }, 0);

  const estimatedEnrichedMass = recommendations.reduce((acc, rec) => {
    const chosenA = rec.enriched?.A ?? Math.round(rec.element.atomic_mass);
    return acc + chosenA * rec.count;
  }, 0);

  return {
    formula,
    tokens,
    recommendations,
    estimatedNaturalMass,
    estimatedEnrichedMass,
  };
}
