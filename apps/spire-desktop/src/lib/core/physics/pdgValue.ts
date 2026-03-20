import type { PdgValue } from "$lib/types/spire";

export interface PdgErrorPair {
  minus: number;
  plus: number;
}

export interface PdgBounds {
  central: number;
  min: number;
  max: number;
  minus: number;
  plus: number;
}

export function pdgCentral(value: PdgValue | undefined): number | null {
  if (!value) return null;
  return value.value;
}

export function pdgErrors(value: PdgValue | undefined): PdgErrorPair | null {
  if (!value) return null;
  if (value.kind === "exact") return { minus: 0, plus: 0 };
  if (value.kind === "symmetric") {
    return { minus: Math.abs(value.error), plus: Math.abs(value.error) };
  }
  return {
    minus: Math.abs(value.error.minus),
    plus: Math.abs(value.error.plus),
  };
}

export function pdgBounds(value: PdgValue | undefined, sigmaMultiplier = 1): PdgBounds | null {
  if (!value) return null;
  const central = value.value;
  const errors = pdgErrors(value);
  if (!errors) return null;
  const minus = errors.minus * sigmaMultiplier;
  const plus = errors.plus * sigmaMultiplier;
  return {
    central,
    minus,
    plus,
    min: central - minus,
    max: central + plus,
  };
}

/**
 * Pull definition using directional asymmetric uncertainty:
 * (Theory - Central) / sigma_dir where sigma_dir = sigma_plus for positive residual,
 * sigma_minus for negative residual. Falls back to average when directional sigma is 0.
 */
export function pdgPull(theoryValue: number, reference: PdgValue | undefined): number | null {
  if (!reference) return null;
  const central = reference.value;
  const errors = pdgErrors(reference);
  if (!errors) return null;

  const delta = theoryValue - central;
  const directional = delta >= 0 ? errors.plus : errors.minus;
  const fallback = (errors.plus + errors.minus) / 2;
  const sigma = directional > 0 ? directional : fallback;
  if (sigma <= 0) return null;
  return delta / sigma;
}
