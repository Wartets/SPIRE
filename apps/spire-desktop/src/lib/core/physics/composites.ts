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

  // ── Baryon octet extensions ────────────────────────────────────────────────
  { particleIds: ["xi0", "xi_0", "xi_zero"], label: "Ξ⁰", valence: ["u", "s", "s"], kind: "baryon" },
  { particleIds: ["xi-", "xi_minus"], label: "Ξ⁻", valence: ["d", "s", "s"], kind: "baryon" },

  // ── Baryon decuplet ────────────────────────────────────────────────────────
  { particleIds: ["delta++", "delta_pp"], label: "Δ++", valence: ["u", "u", "u"], kind: "baryon" },
  { particleIds: ["delta+", "delta_p"], label: "Δ+", valence: ["u", "u", "d"], kind: "baryon" },
  { particleIds: ["delta0", "delta_0"], label: "Δ⁰", valence: ["u", "d", "d"], kind: "baryon" },
  { particleIds: ["delta-", "delta_m"], label: "Δ⁻", valence: ["d", "d", "d"], kind: "baryon" },
  { particleIds: ["sigma*+", "sigmastar+"], label: "Σ*+", valence: ["u", "u", "s"], kind: "baryon" },
  { particleIds: ["sigma*0", "sigmastar0"], label: "Σ*⁰", valence: ["u", "d", "s"], kind: "baryon" },
  { particleIds: ["sigma*-", "sigmastar-"], label: "Σ*⁻", valence: ["d", "d", "s"], kind: "baryon" },
  { particleIds: ["xi*0", "xistar0"], label: "Ξ*⁰", valence: ["u", "s", "s"], kind: "baryon" },
  { particleIds: ["xi*-", "xistar-"], label: "Ξ*⁻", valence: ["d", "s", "s"], kind: "baryon" },
  { particleIds: ["omega-", "omega_baryon"], label: "Ω⁻", valence: ["s", "s", "s"], kind: "baryon" },

  // ── Pseudoscalar meson nonet extensions ───────────────────────────────────
  { particleIds: ["eta", "eta0"], label: "η", valence: ["u", "u_bar"], kind: "meson" },
  { particleIds: ["eta_prime", "eta'", "eta_p"], label: "η′", valence: ["s", "s_bar"], kind: "meson" },

  // ── Vector meson nonet ────────────────────────────────────────────────────
  { particleIds: ["rho+", "rho_plus"], label: "ρ+", valence: ["u", "d_bar"], kind: "meson" },
  { particleIds: ["rho-", "rho_minus"], label: "ρ⁻", valence: ["d", "u_bar"], kind: "meson" },
  { particleIds: ["rho0", "rho_zero"], label: "ρ⁰", valence: ["u", "u_bar"], kind: "meson" },
  { particleIds: ["omega_meson", "omega770"], label: "ω(782)", valence: ["u", "u_bar"], kind: "meson" },
  { particleIds: ["phi_meson", "phi1020"], label: "φ(1020)", valence: ["s", "s_bar"], kind: "meson" },
  { particleIds: ["kstar+", "k*+"], label: "K*+", valence: ["u", "s_bar"], kind: "meson" },
  { particleIds: ["kstar-", "k*-"], label: "K*⁻", valence: ["s", "u_bar"], kind: "meson" },
  { particleIds: ["kstar0", "k*0"], label: "K*⁰", valence: ["d", "s_bar"], kind: "meson" },
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

  // ── Lambda octet decays ────────────────────────────────────────────────────
  { parentIds: ["lambda0", "lambda", "lambda_0"], finalStateIds: ["p", "pi-"], branchingRatio: 0.639, interaction: "weak" },
  { parentIds: ["lambda0", "lambda", "lambda_0"], finalStateIds: ["n", "pi0"], branchingRatio: 0.358, interaction: "weak" },

  // ── Sigma octet decays ─────────────────────────────────────────────────────
  { parentIds: ["sigma+"], finalStateIds: ["p", "pi0"], branchingRatio: 0.516, interaction: "weak" },
  { parentIds: ["sigma+"], finalStateIds: ["n", "pi+"], branchingRatio: 0.484, interaction: "weak" },
  { parentIds: ["sigma0"], finalStateIds: ["lambda0", "gamma"], branchingRatio: 1.0, interaction: "em" },
  { parentIds: ["sigma-"], finalStateIds: ["n", "pi-"], branchingRatio: 0.9998, interaction: "weak" },

  // ── Xi octet decays ────────────────────────────────────────────────────────
  { parentIds: ["xi0", "xi_0", "xi_zero"], finalStateIds: ["lambda0", "pi0"], branchingRatio: 0.998, interaction: "weak" },
  { parentIds: ["xi-", "xi_minus"], finalStateIds: ["lambda0", "pi-"], branchingRatio: 0.9986, interaction: "weak" },

  // ── Delta decuplet decays ──────────────────────────────────────────────────
  { parentIds: ["delta++", "delta_pp"], finalStateIds: ["p", "pi+"], branchingRatio: 1.0, interaction: "strong" },
  { parentIds: ["delta+", "delta_p"], finalStateIds: ["p", "pi0"], branchingRatio: 0.667, interaction: "strong" },
  { parentIds: ["delta+", "delta_p"], finalStateIds: ["n", "pi+"], branchingRatio: 0.333, interaction: "strong" },
  { parentIds: ["delta0", "delta_0"], finalStateIds: ["n", "pi0"], branchingRatio: 0.667, interaction: "strong" },
  { parentIds: ["delta0", "delta_0"], finalStateIds: ["p", "pi-"], branchingRatio: 0.333, interaction: "strong" },
  { parentIds: ["delta-", "delta_m"], finalStateIds: ["n", "pi-"], branchingRatio: 1.0, interaction: "strong" },
  { parentIds: ["sigma*+", "sigmastar+"], finalStateIds: ["sigma+", "pi0"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["sigma*+", "sigmastar+"], finalStateIds: ["sigma0", "pi+"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["sigma*0", "sigmastar0"], finalStateIds: ["sigma+", "pi-"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["sigma*0", "sigmastar0"], finalStateIds: ["sigma-", "pi+"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["sigma*-", "sigmastar-"], finalStateIds: ["sigma0", "pi-"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["sigma*-", "sigmastar-"], finalStateIds: ["sigma-", "pi0"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["xi*0", "xistar0"], finalStateIds: ["xi0", "pi0"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["xi*0", "xistar0"], finalStateIds: ["xi-", "pi+"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["xi*-", "xistar-"], finalStateIds: ["xi-", "pi0"], branchingRatio: 0.5, interaction: "strong" },
  { parentIds: ["xi*-", "xistar-"], finalStateIds: ["xi0", "pi-"], branchingRatio: 0.5, interaction: "strong" },

  // ── Omega decays ───────────────────────────────────────────────────────────
  { parentIds: ["omega-", "omega_baryon"], finalStateIds: ["lambda0", "k-"], branchingRatio: 0.678, interaction: "weak" },
  { parentIds: ["omega-", "omega_baryon"], finalStateIds: ["xi0", "pi-"], branchingRatio: 0.236, interaction: "weak" },
  { parentIds: ["omega-", "omega_baryon"], finalStateIds: ["xi-", "pi0"], branchingRatio: 0.086, interaction: "weak" },

  // ── Eta / eta-prime decays ─────────────────────────────────────────────────
  { parentIds: ["eta", "eta0"], finalStateIds: ["gamma", "gamma"], branchingRatio: 0.392, interaction: "em" },
  { parentIds: ["eta", "eta0"], finalStateIds: ["pi0", "pi0", "pi0"], branchingRatio: 0.326, interaction: "strong" },
  { parentIds: ["eta", "eta0"], finalStateIds: ["pi+", "pi-", "pi0"], branchingRatio: 0.228, interaction: "strong" },
  { parentIds: ["eta_prime", "eta'", "eta_p"], finalStateIds: ["rho0", "gamma"], branchingRatio: 0.291, interaction: "em" },
  { parentIds: ["eta_prime", "eta'", "eta_p"], finalStateIds: ["pi+", "pi-", "eta"], branchingRatio: 0.429, interaction: "strong" },

  // ── Vector meson decays ────────────────────────────────────────────────────
  { parentIds: ["rho+", "rho_plus"], finalStateIds: ["pi+", "pi0"], branchingRatio: 1.0, interaction: "strong" },
  { parentIds: ["rho-", "rho_minus"], finalStateIds: ["pi-", "pi0"], branchingRatio: 1.0, interaction: "strong" },
  { parentIds: ["rho0", "rho_zero"], finalStateIds: ["pi+", "pi-"], branchingRatio: 1.0, interaction: "strong" },
  { parentIds: ["omega_meson", "omega770"], finalStateIds: ["pi+", "pi-", "pi0"], branchingRatio: 0.892, interaction: "strong" },
  { parentIds: ["omega_meson", "omega770"], finalStateIds: ["pi0", "gamma"], branchingRatio: 0.084, interaction: "em" },
  { parentIds: ["phi_meson", "phi1020"], finalStateIds: ["k+", "k-"], branchingRatio: 0.492, interaction: "strong" },
  { parentIds: ["phi_meson", "phi1020"], finalStateIds: ["k0", "k0"], branchingRatio: 0.340, interaction: "strong" },
  { parentIds: ["phi_meson", "phi1020"], finalStateIds: ["rho0", "pi0"], branchingRatio: 0.153, interaction: "strong" },
  { parentIds: ["kstar+", "k*+"], finalStateIds: ["k0", "pi+"], branchingRatio: 0.667, interaction: "strong" },
  { parentIds: ["kstar+", "k*+"], finalStateIds: ["k+", "pi0"], branchingRatio: 0.333, interaction: "strong" },
  { parentIds: ["kstar-", "k*-"], finalStateIds: ["k0", "pi-"], branchingRatio: 0.667, interaction: "strong" },
  { parentIds: ["kstar-", "k*-"], finalStateIds: ["k-", "pi0"], branchingRatio: 0.333, interaction: "strong" },
  { parentIds: ["kstar0", "k*0"], finalStateIds: ["k+", "pi-"], branchingRatio: 0.667, interaction: "strong" },
  { parentIds: ["kstar0", "k*0"], finalStateIds: ["k0", "pi0"], branchingRatio: 0.333, interaction: "strong" },
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

export function listCompositeReferenceLibrary(): CompositeQuarkContent[] {
  return [...COMPOSITE_LIBRARY].sort((a, b) => a.label.localeCompare(b.label));
}
