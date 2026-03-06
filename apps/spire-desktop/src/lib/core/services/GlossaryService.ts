/**
 * SPIRE - Glossary Service
 *
 * Data-driven registry of physics terms, constants, and concepts.
 * Each entry carries a human-readable definition, optional formula
 * (LaTeX), and an optional Particle Data Group (PDG) reference.
 *
 * The glossary is deliberately decoupled from UI components:
 *   - Core Standard Model terms are populated at import time.
 *   - BSM or UFO models can inject custom entries at runtime via
 *     `registerTerm()` / `registerTerms()`.
 *   - The `<HoverDef>` tooltip component queries this service by key.
 *
 * Keys follow a flat namespace: lowercase, underscores, no dots.
 * Example keys: "alpha_s", "m_z", "gamma_w", "ckm_matrix".
 */

import { writable, derived } from "svelte/store";

// ---------------------------------------------------------------------------
// Data Model
// ---------------------------------------------------------------------------

export interface GlossaryEntry {
  /** The canonical display name (e.g. "Strong Coupling Constant"). */
  term: string;
  /** Plain-text scientific definition (1–3 sentences). */
  definition: string;
  /** Optional LaTeX formula for the tooltip (e.g. "\\alpha_s(M_Z)"). */
  formula?: string;
  /** Optional current numerical value and units. */
  value?: string;
  /** Optional PDG reference string (e.g. "PDG 2024, §9.4"). */
  pdgRef?: string;
  /** Optional category for grouping (e.g. "Coupling", "Mass", "Concept"). */
  category?: string;
}

// ---------------------------------------------------------------------------
// Internal Store
// ---------------------------------------------------------------------------

const _glossaryMap = writable<Map<string, GlossaryEntry>>(new Map());

// ---------------------------------------------------------------------------
// Public Derived Stores
// ---------------------------------------------------------------------------

/**
 * All glossary entries as a read-only map.
 */
export const glossary = derived(_glossaryMap, ($map) => $map);

/**
 * Number of registered terms.
 */
export const glossarySize = derived(_glossaryMap, ($map) => $map.size);

// ---------------------------------------------------------------------------
// Registry API
// ---------------------------------------------------------------------------

/**
 * Register or replace a single glossary term.
 */
export function registerTerm(key: string, entry: GlossaryEntry): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    next.set(key, entry);
    return next;
  });
}

/**
 * Register multiple glossary terms at once.
 */
export function registerTerms(entries: Record<string, GlossaryEntry>): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    for (const [key, entry] of Object.entries(entries)) {
      next.set(key, entry);
    }
    return next;
  });
}

/**
 * Remove a glossary term by key.
 */
export function unregisterTerm(key: string): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    next.delete(key);
    return next;
  });
}

/**
 * Look up a single glossary entry by key.
 * Returns `undefined` if the key is not registered.
 */
export function lookupTerm(key: string): GlossaryEntry | undefined {
  let result: GlossaryEntry | undefined;
  _glossaryMap.subscribe((map) => {
    result = map.get(key);
  })();
  return result;
}

// ---------------------------------------------------------------------------
// Standard Model Core Glossary
// ---------------------------------------------------------------------------

const SM_GLOSSARY: Record<string, GlossaryEntry> = {
  alpha_s: {
    term: "Strong Coupling Constant",
    definition:
      "The dimensionless coupling constant of Quantum Chromodynamics (QCD), governing the strength of the strong interaction between quarks and gluons. Runs with energy scale due to asymptotic freedom.",
    formula: "\\alpha_s(M_Z) \\approx 0.1180",
    value: "0.1180 ± 0.0009 at M_Z",
    pdgRef: "PDG 2024, §9.4 - QCD",
    category: "Coupling",
  },
  alpha_em: {
    term: "Fine-Structure Constant",
    definition:
      "The dimensionless coupling constant of Quantum Electrodynamics (QED), characterising the strength of the electromagnetic interaction. Defined as e²/(4πε₀ℏc).",
    formula: "\\alpha_{\\text{em}} = \\frac{e^2}{4\\pi} \\approx \\frac{1}{137.036}",
    value: "1/137.035999084(21)",
    pdgRef: "PDG 2024, §1 - Physical Constants",
    category: "Coupling",
  },
  g_fermi: {
    term: "Fermi Coupling Constant",
    definition:
      "The effective coupling constant of the weak interaction in the four-fermion (Fermi) theory. Related to the W boson mass and the weak mixing angle by G_F/√2 = g²/(8M_W²).",
    formula: "G_F = 1.1663788(6) \\times 10^{-5}\\;\\text{GeV}^{-2}",
    value: "1.1663788 × 10⁻⁵ GeV⁻²",
    pdgRef: "PDG 2024, §10 - Electroweak Model",
    category: "Coupling",
  },
  m_z: {
    term: "Z Boson Mass",
    definition:
      "The pole mass of the Z⁰ boson, the electrically neutral mediator of the weak neutral current. Precisely measured at LEP via the Z lineshape scan.",
    formula: "M_Z = 91.1876 \\pm 0.0021\\;\\text{GeV}",
    value: "91.1876 ± 0.0021 GeV",
    pdgRef: "PDG 2024, §10.2 - Z Boson",
    category: "Mass",
  },
  m_w: {
    term: "W Boson Mass",
    definition:
      "The pole mass of the W± boson, mediator of the charged weak current. Connected to M_Z through the weak mixing angle: M_W = M_Z cos(θ_W).",
    formula: "M_W = 80.3692 \\pm 0.0133\\;\\text{GeV}",
    value: "80.3692 ± 0.0133 GeV",
    pdgRef: "PDG 2024, §10.2 - W Boson",
    category: "Mass",
  },
  m_h: {
    term: "Higgs Boson Mass",
    definition:
      "The pole mass of the Higgs scalar boson, the excitation of the Brout–Englert–Higgs field responsible for electroweak symmetry breaking.",
    formula: "M_H = 125.20 \\pm 0.11\\;\\text{GeV}",
    value: "125.20 ± 0.11 GeV",
    pdgRef: "PDG 2024, §11.6 - Higgs Boson",
    category: "Mass",
  },
  m_top: {
    term: "Top Quark Mass",
    definition:
      "The pole mass of the top quark, the heaviest known fermion. Its large Yukawa coupling (y_t ≈ 1) makes it central to electroweak precision constraints and Higgs physics.",
    formula: "m_t = 172.57 \\pm 0.29\\;\\text{GeV}",
    value: "172.57 ± 0.29 GeV",
    pdgRef: "PDG 2024, §66 - Top Quark",
    category: "Mass",
  },
  gamma_w: {
    term: "W Boson Total Width",
    definition:
      "The total decay width of the W± boson, corresponding to its inverse lifetime. Dominated by hadronic (qq̄′) and leptonic (ℓν) channels.",
    formula: "\\Gamma_W = 2.085 \\pm 0.042\\;\\text{GeV}",
    value: "2.085 ± 0.042 GeV",
    pdgRef: "PDG 2024, §10.2 - W Boson",
    category: "Width",
  },
  gamma_z: {
    term: "Z Boson Total Width",
    definition:
      "The total decay width of the Z⁰ boson. The invisible width constrains the number of light neutrino generations to N_ν = 2.9963 ± 0.0074.",
    formula: "\\Gamma_Z = 2.4955 \\pm 0.0023\\;\\text{GeV}",
    value: "2.4955 ± 0.0023 GeV",
    pdgRef: "PDG 2024, §10.2 - Z Boson",
    category: "Width",
  },
  sin2_theta_w: {
    term: "Weak Mixing Angle",
    definition:
      "The electroweak mixing (Weinberg) angle θ_W that parametrises the mixing between the SU(2)_L and U(1)_Y gauge bosons into the physical Z and photon. Defined via sin²θ_W = 1 − M_W²/M_Z².",
    formula: "\\sin^2\\theta_W^{\\text{eff}} = 0.23155 \\pm 0.00004",
    value: "0.23155 ± 0.00004",
    pdgRef: "PDG 2024, §10 - Electroweak Model",
    category: "Parameter",
  },
  ckm_matrix: {
    term: "CKM Quark Mixing Matrix",
    definition:
      "The Cabibbo–Kobayashi–Maskawa matrix is a 3×3 unitary matrix describing the mixing between quark mass eigenstates and weak-interaction eigenstates. Its off-diagonal elements govern flavour-changing charged currents.",
    formula: "V_{\\text{CKM}} = \\begin{pmatrix} V_{ud} & V_{us} & V_{ub} \\\\ V_{cd} & V_{cs} & V_{cb} \\\\ V_{td} & V_{ts} & V_{tb} \\end{pmatrix}",
    pdgRef: "PDG 2024, §12 - CKM Matrix",
    category: "Parameter",
  },
  cross_section: {
    term: "Cross Section",
    definition:
      "The effective target area for a scattering process, quantifying the probability of a specific reaction occurring. Measured in barns (1 b = 10⁻²⁴ cm²). Computed from the squared amplitude integrated over phase space.",
    formula: "\\sigma = \\frac{1}{2s} \\int |\\mathcal{M}|^2 \\, d\\Phi_n",
    category: "Concept",
  },
  mandelstam_s: {
    term: "Mandelstam Variable s",
    definition:
      "The square of the total four-momentum of the colliding system, equal to the centre-of-mass energy squared. For a 2→2 process: s = (p₁ + p₂)².",
    formula: "s = (p_1 + p_2)^2",
    category: "Concept",
  },
  mandelstam_t: {
    term: "Mandelstam Variable t",
    definition:
      "The square of the four-momentum transfer between an initial-state and a final-state particle. For a 2→2 process: t = (p₁ − p₃)².",
    formula: "t = (p_1 - p_3)^2",
    category: "Concept",
  },
  phase_space: {
    term: "Phase Space",
    definition:
      "The kinematically allowed region of final-state momenta for a given reaction. The n-body Lorentz-invariant phase space measure dΦ_n encodes energy-momentum conservation and on-shell constraints.",
    formula: "d\\Phi_n = \\prod_{i=1}^{n} \\frac{d^3 p_i}{(2\\pi)^3 2E_i} \\, (2\\pi)^4 \\delta^{(4)}\\!\\left(P - \\sum p_i\\right)",
    category: "Concept",
  },
  feynman_diagram: {
    term: "Feynman Diagram",
    definition:
      "A graphical representation of a term in the perturbative expansion of a scattering amplitude. Each vertex corresponds to a coupling constant, each internal line to a propagator, and each external line to an asymptotic particle state.",
    category: "Concept",
  },
  su3_color: {
    term: "SU(3) Colour Symmetry",
    definition:
      "The non-Abelian gauge symmetry group of QCD. Quarks transform in the fundamental (triplet) representation, antiquarks in the antitriplet, and gluons in the adjoint (octet) representation.",
    formula: "SU(3)_C",
    pdgRef: "PDG 2024, §9 - QCD",
    category: "Symmetry",
  },
};

// Populate the glossary at module load time
registerTerms(SM_GLOSSARY);
