export interface QuantumFormatOptions {
  signed?: boolean;
  maxDenominator?: number;
  tolerance?: number;
}

function bestRationalApproximation(value: number, maxDenominator: number): { numerator: number; denominator: number; error: number } {
  let bestNumerator = Math.round(value);
  let bestDenominator = 1;
  let bestError = Math.abs(value - bestNumerator);

  for (let denominator = 1; denominator <= maxDenominator; denominator += 1) {
    const numerator = Math.round(value * denominator);
    const approx = numerator / denominator;
    const error = Math.abs(value - approx);
    if (error < bestError) {
      bestNumerator = numerator;
      bestDenominator = denominator;
      bestError = error;
    }
  }

  return { numerator: bestNumerator, denominator: bestDenominator, error: bestError };
}

function gcd(a: number, b: number): number {
  let x = Math.abs(a);
  let y = Math.abs(b);
  while (y !== 0) {
    const r = x % y;
    x = y;
    y = r;
  }
  return x || 1;
}

export function formatQuantumFraction(value: number, options: QuantumFormatOptions = {}): string {
  const {
    signed = false,
    maxDenominator = 12,
    tolerance = 1e-6,
  } = options;

  if (!Number.isFinite(value)) return "n/a";
  if (Math.abs(value) < tolerance) return "0";

  const sign = value < 0 ? "-" : signed ? "+" : "";
  const absValue = Math.abs(value);

  if (Math.abs(absValue - Math.round(absValue)) <= tolerance) {
    return `${sign}${Math.round(absValue)}`;
  }

  const approx = bestRationalApproximation(absValue, Math.max(1, Math.floor(maxDenominator)));
  const relativeScale = Math.max(1, absValue);
  if (approx.error <= tolerance * relativeScale) {
    const common = gcd(approx.numerator, approx.denominator);
    const numerator = approx.numerator / common;
    const denominator = approx.denominator / common;
    if (denominator === 1) {
      return `${sign}${numerator}`;
    }
    return `${sign}${numerator}/${denominator}`;
  }

  const compact = absValue >= 1e3 || absValue < 1e-4
    ? absValue.toExponential(3)
    : absValue.toFixed(4).replace(/\.?0+$/, "");
  return `${sign}${compact}`;
}
