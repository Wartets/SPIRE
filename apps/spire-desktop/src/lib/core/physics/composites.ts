import type { Field } from "$lib/types/spire";

export type CompositeKind = "baryon" | "meson" | "nucleus" | "other";
export type InteractionClass = "strong" | "weak" | "em";
export type ColorState = "red" | "green" | "blue" | "anti-red" | "anti-green" | "anti-blue";

export interface CompositeQuarkContent {
  particleIds: string[];
  label: string;
  valence: string[];
  kind: CompositeKind;
}

export interface ColoredValenceQuark {
  flavor: string;
  color: ColorState;
  isAnti: boolean;
  symbol: string;
}

export interface DecayChannelPreset {
  parentIds: string[];
  finalStateIds: string[];
  branchingRatio: number;
  interaction: InteractionClass;
}

const COMPOSITE_LIBRARY: CompositeQuarkContent[] = [
  { particleIds: ["p", "proton", "p+"], label: "Proton", valence: ["u", "u", "d"], kind: "baryon" },
  { particleIds: ["n", "neutron", "n0"], label: "Neutron", valence: ["u", "d", "d"], kind: "baryon" },
  { particleIds: ["lambda0", "lambda", "lambda_0"], label: "Lambda", valence: ["u", "d", "s"], kind: "baryon" },
  { particleIds: ["sigma+"], label: "Sigma+", valence: ["u", "u", "s"], kind: "baryon" },
  { particleIds: ["sigma0"], label: "Sigma0", valence: ["u", "d", "s"], kind: "baryon" },
  { particleIds: ["sigma-"], label: "Sigma-", valence: ["d", "d", "s"], kind: "baryon" },
  { particleIds: ["pi+", "pion+"], label: "Pion+", valence: ["u", "d_bar"], kind: "meson" },
  { particleIds: ["pi-", "pion-"], label: "Pion-", valence: ["d", "u_bar"], kind: "meson" },
  { particleIds: ["pi0", "pion0"], label: "Pion0", valence: ["u", "u_bar"], kind: "meson" },
  { particleIds: ["k+", "kaon+"], label: "Kaon+", valence: ["u", "s_bar"], kind: "meson" },
  { particleIds: ["k-", "kaon-"], label: "Kaon-", valence: ["s", "u_bar"], kind: "meson" },
  { particleIds: ["k0", "k0_short", "k0_long"], label: "Kaon0", valence: ["d", "s_bar"], kind: "meson" },
  { particleIds: ["b0", "b0_bar", "b_d0"], label: "B0", valence: ["d", "b_bar"], kind: "meson" },
  { particleIds: ["b+", "b_plus"], label: "B+", valence: ["u", "b_bar"], kind: "meson" },
  { particleIds: ["deuteron", "d2"], label: "Deuteron", valence: ["uud", "udd"], kind: "nucleus" },
];

const DECAY_LIBRARY: DecayChannelPreset[] = [
  { parentIds: ["z", "z0"], finalStateIds: ["e-", "e+"], branchingRatio: 0.03363, interaction: "weak" },
  { parentIds: ["z", "z0"], finalStateIds: ["mu-", "mu+"], branchingRatio: 0.03366, interaction: "weak" },
  { parentIds: ["z", "z0"], finalStateIds: ["tau-", "tau+"], branchingRatio: 0.03370, interaction: "weak" },
  { parentIds: ["z", "z0"], finalStateIds: ["nu_e", "nu_e_bar"], branchingRatio: 0.067, interaction: "weak" },
  { parentIds: ["z", "z0"], finalStateIds: ["u", "u_bar"], branchingRatio: 0.116, interaction: "weak" },
  { parentIds: ["z", "z0"], finalStateIds: ["d", "d_bar"], branchingRatio: 0.152, interaction: "weak" },
  { parentIds: ["w+"], finalStateIds: ["e+", "nu_e"], branchingRatio: 0.108, interaction: "weak" },
  { parentIds: ["w+"], finalStateIds: ["mu+", "nu_mu"], branchingRatio: 0.106, interaction: "weak" },
  { parentIds: ["w+"], finalStateIds: ["u", "d_bar"], branchingRatio: 0.326, interaction: "weak" },
  { parentIds: ["w-"], finalStateIds: ["e-", "nu_e_bar"], branchingRatio: 0.108, interaction: "weak" },
  { parentIds: ["w-"], finalStateIds: ["mu-", "nu_mu_bar"], branchingRatio: 0.106, interaction: "weak" },
  { parentIds: ["w-"], finalStateIds: ["d", "u_bar"], branchingRatio: 0.326, interaction: "weak" },
  { parentIds: ["h"], finalStateIds: ["b", "b_bar"], branchingRatio: 0.5824, interaction: "weak" },
  { parentIds: ["h"], finalStateIds: ["w+", "w-"], branchingRatio: 0.215, interaction: "weak" },
  { parentIds: ["h"], finalStateIds: ["z", "z"], branchingRatio: 0.026, interaction: "weak" },
  { parentIds: ["n", "neutron", "n0"], finalStateIds: ["p", "e-", "nu_e_bar"], branchingRatio: 0.999, interaction: "weak" },
  { parentIds: ["pi+"], finalStateIds: ["mu+", "nu_mu"], branchingRatio: 0.9999, interaction: "weak" },
  { parentIds: ["pi-"], finalStateIds: ["mu-", "nu_mu_bar"], branchingRatio: 0.9999, interaction: "weak" },
  { parentIds: ["k+"], finalStateIds: ["mu+", "nu_mu"], branchingRatio: 0.6356, interaction: "weak" },
  { parentIds: ["k+"], finalStateIds: ["pi+", "pi0"], branchingRatio: 0.2067, interaction: "weak" },
  { parentIds: ["k-"], finalStateIds: ["mu-", "nu_mu_bar"], branchingRatio: 0.6356, interaction: "weak" },
  { parentIds: ["b0", "b_d0"], finalStateIds: ["d", "d_bar"], branchingRatio: 0.35, interaction: "weak" },
  { parentIds: ["b0", "b_d0"], finalStateIds: ["c", "c_bar"], branchingRatio: 0.25, interaction: "weak" },
  { parentIds: ["b0", "b_d0"], finalStateIds: ["k+", "pi-"], branchingRatio: 0.08, interaction: "weak" },
];

function normalizeKey(v: string): string {
  return v.trim().toLowerCase().replace(/\s+/g, "").replace(/[{}\\^]/g, "");
}

function toFlavorSymbol(flavor: string): string {
  const base = flavor.replace(/_bar$/, "");
  if (flavor.endsWith("_bar")) {
    return `${base}\u0305`;
  }
  return base;
}

function hashFlavor(flavor: string): number {
  let h = 0;
  for (let i = 0; i < flavor.length; i += 1) {
    h = (h * 33 + flavor.charCodeAt(i)) >>> 0;
  }
  return h;
}

const TRIPLET: ColorState[] = ["red", "green", "blue"];
function deterministicTripletColor(seed: string): ColorState {
  return TRIPLET[hashFlavor(seed) % TRIPLET.length];
}

function antiColor(color: ColorState): ColorState {
  if (color === "red") return "anti-red";
  if (color === "green") return "anti-green";
  if (color === "blue") return "anti-blue";
  if (color === "anti-red") return "red";
  if (color === "anti-green") return "green";
  return "blue";
}

export function resolveCompositeState(particle: Field | string): CompositeQuarkContent | null {
  const key = normalizeKey(typeof particle === "string" ? particle : particle.id);
  for (const item of COMPOSITE_LIBRARY) {
    if (item.particleIds.some((id) => normalizeKey(id) === key)) {
      return item;
    }
  }
  return null;
}

export function resolveDecayChannels(particle: Field | string): DecayChannelPreset[] {
  const key = normalizeKey(typeof particle === "string" ? particle : particle.id);
  return DECAY_LIBRARY
    .filter((ch) => ch.parentIds.some((id) => normalizeKey(id) === key))
    .sort((a, b) => b.branchingRatio - a.branchingRatio);
}

export function assignColorSinglet(valence: string[]): ColoredValenceQuark[] {
  if (valence.length === 0) return [];

  if (valence.length === 3) {
    return valence.map((flavor, idx) => ({
      flavor,
      color: TRIPLET[idx % 3],
      isAnti: flavor.endsWith("_bar"),
      symbol: toFlavorSymbol(flavor),
    }));
  }

  if (valence.length === 2) {
    const q = valence[0];
    const qb = valence[1];
    const qColor = deterministicTripletColor(q);
    return [
      { flavor: q, color: qColor, isAnti: q.endsWith("_bar"), symbol: toFlavorSymbol(q) },
      { flavor: qb, color: antiColor(qColor), isAnti: qb.endsWith("_bar"), symbol: toFlavorSymbol(qb) },
    ];
  }

  return valence.map((flavor, idx) => {
    const raw = deterministicTripletColor(`${flavor}_${idx}`);
    const color = flavor.endsWith("_bar") ? antiColor(raw) : raw;
    return {
      flavor,
      color,
      isAnti: flavor.endsWith("_bar"),
      symbol: toFlavorSymbol(flavor),
    };
  });
}

export function isCompositeParticle(particle: Field | string): boolean {
  return resolveCompositeState(particle) !== null;
}
