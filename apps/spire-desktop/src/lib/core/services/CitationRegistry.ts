/**
 * SPIRE - Citation Registry
 *
 * Data-driven bibliography engine for theoretical justification.
 * Maintains a database of academic citations (BibTeX-style) and
 * tracks which references are currently "active" based on the
 * calculations the user has performed.
 *
 * Architecture:
 *   - `BIBLIOGRAPHY`: static database of Citation objects, keyed by ID.
 *   - `activeCitations`: reactive store of citation IDs accumulated
 *     as the user exercises different computational pathways.
 *   - Components (e.g. ReferencesPanel) subscribe to `activeCitations`
 *     and render properly formatted academic references.
 *   - The Rust kernel can append `citation_keys` to result payloads,
 *     which the frontend propagates via `addCitations()`.
 *
 * This registry is intentionally decoupled from all UI and backend
 * modules so that external models can inject their own references.
 */

import { writable, derived } from "svelte/store";

// ---------------------------------------------------------------------------
// Citation Data Model
// ---------------------------------------------------------------------------

export interface Citation {
  /** Unique identifier (e.g. "peskin1995", "passarino1979"). */
  id: string;
  /** List of authors (surname, initials). */
  authors: string[];
  /** Full title of the work. */
  title: string;
  /** Publication year. */
  year: number;
  /** Journal or publisher (e.g. "Nucl. Phys. B", "Addison-Wesley"). */
  journal?: string;
  /** Volume number. */
  volume?: string;
  /** Page range or article number. */
  pages?: string;
  /** arXiv identifier (e.g. "hep-ph/9506380"). */
  arxiv?: string;
  /** DOI. */
  doi?: string;
  /** Short description of why this reference is relevant. */
  context?: string;
}

// ---------------------------------------------------------------------------
// Bibliography Database
// ---------------------------------------------------------------------------

const _bibliographyMap = new Map<string, Citation>();

function _register(c: Citation): void {
  _bibliographyMap.set(c.id, c);
}

// ── Foundational Textbooks ───────────────────────────────────────────────

_register({
  id: "peskin1995",
  authors: ["Peskin, M.E.", "Schroeder, D.V."],
  title: "An Introduction to Quantum Field Theory",
  year: 1995,
  journal: "Addison-Wesley",
  context: "Standard reference for perturbative QFT, Feynman rules, and cross-section calculations.",
});

_register({
  id: "weinberg1995",
  authors: ["Weinberg, S."],
  title: "The Quantum Theory of Fields, Vol. I: Foundations",
  year: 1995,
  journal: "Cambridge University Press",
  doi: "10.1017/CBO9781139644167",
  context: "Foundational treatment of quantum field theory and the S-matrix.",
});

_register({
  id: "schwartz2014",
  authors: ["Schwartz, M.D."],
  title: "Quantum Field Theory and the Standard Model",
  year: 2014,
  journal: "Cambridge University Press",
  doi: "10.1017/9781139540940",
  context: "Modern QFT textbook covering the Standard Model and beyond.",
});

// ── Feynman Diagram Topology ─────────────────────────────────────────────

_register({
  id: "hahn2001",
  authors: ["Hahn, T."],
  title: "Generating Feynman Diagrams and Amplitudes with FeynArts 3",
  year: 2001,
  journal: "Comput. Phys. Commun.",
  volume: "140",
  pages: "153–168",
  arxiv: "hep-ph/0012260",
  doi: "10.1016/S0010-4655(01)00290-9",
  context: "Algorithm for automated generation of Feynman diagram topologies.",
});

_register({
  id: "alwall2014",
  authors: ["Alwall, J.", "Frederix, R.", "Frixione, S.", "et al."],
  title: "The Automated Computation of Tree-Level and Next-to-Leading Order Differential Cross Sections, and Their Matching to Parton Shower Simulations",
  year: 2014,
  journal: "JHEP",
  volume: "07",
  pages: "079",
  arxiv: "1405.0301",
  doi: "10.1007/JHEP07(2014)079",
  context: "Core reference for MadGraph5_aMC@NLO style automated perturbative workflows.",
});

// ── Passarino–Veltman Reduction ──────────────────────────────────────────

_register({
  id: "passarino1979",
  authors: ["Passarino, G.", "Veltman, M.J.G."],
  title: "One-Loop Corrections for e⁺e⁻ Annihilation into μ⁺μ⁻ in the Weinberg Model",
  year: 1979,
  journal: "Nucl. Phys. B",
  volume: "160",
  pages: "151–207",
  doi: "10.1016/0550-3213(79)90234-7",
  context: "Defines the standard tensor-integral reduction scheme for one-loop calculations.",
});

// ── Dirac Algebra & Trace Technology ─────────────────────────────────────

_register({
  id: "kennedy1982",
  authors: ["Kennedy, A.D."],
  title: "Clifford Algebras in 2ω Dimensions",
  year: 1982,
  journal: "J. Math. Phys.",
  volume: "22",
  pages: "1330",
  doi: "10.1063/1.525069",
  context: "Dirac trace evaluation in dimensional regularisation.",
});

_register({
  id: "thooft1972",
  authors: ["'t Hooft, G.", "Veltman, M."],
  title: "Regularization and Renormalization of Gauge Fields",
  year: 1972,
  journal: "Nucl. Phys. B",
  volume: "44",
  pages: "189–213",
  doi: "10.1016/0550-3213(72)90279-9",
  context: "Foundational dimensional regularisation and renormalisation framework.",
});

// ── Phase Space & Monte Carlo Integration ────────────────────────────────

_register({
  id: "kleiss1986",
  authors: ["Kleiss, R.", "Stirling, W.J.", "Ellis, S.D."],
  title: "A New Monte Carlo Treatment of Multiparticle Phase Space at High Energies",
  year: 1986,
  journal: "Comput. Phys. Commun.",
  volume: "40",
  pages: "359–373",
  doi: "10.1016/0010-4655(86)90119-0",
  context: "RAMBO algorithm for democratic multi-body phase-space generation.",
});

_register({
  id: "james1980",
  authors: ["James, F."],
  title: "Monte Carlo Theory and Practice",
  year: 1980,
  journal: "Rep. Prog. Phys.",
  volume: "43",
  pages: "1145–1189",
  doi: "10.1088/0034-4885/43/9/002",
  context: "Foundational reference for Monte Carlo integration in particle physics.",
});

_register({
  id: "vegas1978",
  authors: ["Lepage, G.P."],
  title: "A New Algorithm for Adaptive Multidimensional Integration",
  year: 1978,
  journal: "J. Comput. Phys.",
  volume: "27",
  pages: "192–203",
  doi: "10.1016/0021-9991(78)90004-9",
  context: "Canonical VEGAS adaptive Monte Carlo algorithm for high-dimensional integrals.",
});

// ── PDG Reference ────────────────────────────────────────────────────────

_register({
  id: "pdg2024",
  authors: ["Particle Data Group", "Navas, S. et al."],
  title: "Review of Particle Physics",
  year: 2024,
  journal: "Phys. Rev. D",
  volume: "110",
  pages: "030001",
  doi: "10.1103/PhysRevD.110.030001",
  context: "Comprehensive review of particle properties, coupling constants, and experimental measurements.",
});

// ── Electroweak Theory ───────────────────────────────────────────────────

_register({
  id: "glashow1961",
  authors: ["Glashow, S.L."],
  title: "Partial-Symmetries of Weak Interactions",
  year: 1961,
  journal: "Nucl. Phys.",
  volume: "22",
  pages: "579–588",
  doi: "10.1016/0029-5582(61)90469-2",
  context: "Original proposal for the electroweak unification via SU(2) × U(1).",
});

_register({
  id: "weinberg1967",
  authors: ["Weinberg, S."],
  title: "A Model of Leptons",
  year: 1967,
  journal: "Phys. Rev. Lett.",
  volume: "19",
  pages: "1264–1266",
  doi: "10.1103/PhysRevLett.19.1264",
  context: "Landmark paper on the electroweak theory with spontaneous symmetry breaking.",
});

// ── QCD ──────────────────────────────────────────────────────────────────

_register({
  id: "gross1973",
  authors: ["Gross, D.J.", "Wilczek, F."],
  title: "Ultraviolet Behavior of Non-Abelian Gauge Theories",
  year: 1973,
  journal: "Phys. Rev. Lett.",
  volume: "30",
  pages: "1343–1346",
  doi: "10.1103/PhysRevLett.30.1343",
  context: "Discovery of asymptotic freedom in QCD.",
});

_register({
  id: "politzer1973",
  authors: ["Politzer, H.D."],
  title: "Reliable Perturbative Results for Strong Interactions?",
  year: 1973,
  journal: "Phys. Rev. Lett.",
  volume: "30",
  pages: "1346–1349",
  doi: "10.1103/PhysRevLett.30.1346",
  context: "Independent discovery of asymptotic freedom complementary to Gross-Wilczek.",
});

// ── Renormalisation Group ────────────────────────────────────────────────

_register({
  id: "machacek1984",
  authors: ["Machacek, M.E.", "Vaughn, M.T."],
  title: "Two-Loop Renormalization Group Equations in a General Quantum Field Theory",
  year: 1984,
  journal: "Nucl. Phys. B",
  volume: "236",
  pages: "221–232",
  doi: "10.1016/0550-3213(84)90533-9",
  context: "General two-loop beta function formulas for gauge-Yukawa theories.",
});

_register({
  id: "machacek1983",
  authors: ["Machacek, M.E.", "Vaughn, M.T."],
  title: "Two-Loop Renormalization Group Equations in a General Quantum Field Theory (I). Wave Function Renormalization",
  year: 1983,
  journal: "Nucl. Phys. B",
  volume: "222",
  pages: "83–103",
  doi: "10.1016/0550-3213(83)90610-7",
  context: "Companion one-/two-loop RG machinery for general field theories.",
});

_register({
  id: "machacek1985",
  authors: ["Machacek, M.E.", "Vaughn, M.T."],
  title: "Two-Loop Renormalization Group Equations in a General Quantum Field Theory (III). Scalar Quartic Couplings",
  year: 1985,
  journal: "Nucl. Phys. B",
  volume: "249",
  pages: "70–92",
  doi: "10.1016/0550-3213(85)90040-9",
  context: "Detailed two-loop RG treatment for scalar sectors and quartic operators.",
});

// ── UFO Model Format ─────────────────────────────────────────────────────

_register({
  id: "degrande2012",
  authors: ["Degrande, C.", "Duhr, C.", "Fuks, B.", "Grellscheid, D.", "Mattelaer, O.", "Reiter, T."],
  title: "UFO - The Universal FeynRules Output",
  year: 2012,
  journal: "Comput. Phys. Commun.",
  volume: "183",
  pages: "1201–1214",
  arxiv: "1108.2040",
  doi: "10.1016/j.cpc.2012.01.022",
  context: "Specification of the UFO model format for BSM physics.",
});

_register({
  id: "alloul2014",
  authors: ["Alloul, A.", "Christensen, N.D.", "Degrande, C.", "Duhr, C.", "Fuks, B."],
  title: "FeynRules 2.0 — A Complete Toolbox for Tree-Level Phenomenology",
  year: 2014,
  journal: "Comput. Phys. Commun.",
  volume: "185",
  pages: "2250–2300",
  arxiv: "1310.1921",
  doi: "10.1016/j.cpc.2014.04.012",
  context: "Model implementation and export ecosystem used for UFO generation workflows.",
});

// ── Cross-Section Formulae ───────────────────────────────────────────────

_register({
  id: "bardin1999",
  authors: ["Bardin, D.", "Passarino, G."],
  title: "The Standard Model in the Making",
  year: 1999,
  journal: "Oxford University Press",
  doi: "10.1093/acprof:oso/9780198502807.001.0001",
  context: "Precision electroweak calculations and cross-section formulas.",
});

_register({
  id: "catani1997",
  authors: ["Catani, S.", "Seymour, M.H."],
  title: "A General Algorithm for Calculating Jet Cross Sections in NLO QCD",
  year: 1997,
  journal: "Nucl. Phys. B",
  volume: "485",
  pages: "291–419",
  arxiv: "hep-ph/9605323",
  doi: "10.1016/S0550-3213(96)00589-5",
  context: "Dipole subtraction formalism for infrared-safe NLO predictions.",
});

_register({
  id: "buckley2011",
  authors: ["Buckley, A.", "Butterworth, J.", "Lonnblad, L.", "et al."],
  title: "General-Purpose Event Generators for LHC Physics",
  year: 2011,
  journal: "Phys. Rept.",
  volume: "504",
  pages: "145–233",
  arxiv: "1101.2599",
  doi: "10.1016/j.physrep.2011.03.005",
  context: "Survey of event-generator architecture and practical phenomenology workflows.",
});

_register({
  id: "sjostrand2015",
  authors: ["Sjostrand, T.", "Ask, S.", "Christiansen, J.R.", "et al."],
  title: "An Introduction to PYTHIA 8.2",
  year: 2015,
  journal: "Comput. Phys. Commun.",
  volume: "191",
  pages: "159–177",
  arxiv: "1410.3012",
  doi: "10.1016/j.cpc.2015.01.024",
  context: "Parton showering, hadronisation, and event-record conventions.",
});

_register({
  id: "agostinelli2003",
  authors: ["Agostinelli, S.", "Allison, J.", "Amako, K.", "et al."],
  title: "GEANT4 — A Simulation Toolkit",
  year: 2003,
  journal: "Nucl. Instrum. Meth. A",
  volume: "506",
  pages: "250–303",
  doi: "10.1016/S0168-9002(03)01368-8",
  context: "Detector-level simulation reference for event interpretation and response studies.",
});

_register({
  id: "ball2022",
  authors: ["Ball, R.D.", "Bertone, V.", "Carrazza, S.", "et al."],
  title: "The Path to Proton Structure at 1% Accuracy",
  year: 2022,
  journal: "Eur. Phys. J. C",
  volume: "82",
  pages: "428",
  arxiv: "2109.02653",
  doi: "10.1140/epjc/s10052-022-10328-7",
  context: "Modern global PDF analyses relevant for hadronic cross-section predictions.",
});

// ---------------------------------------------------------------------------
// Active Citations Store
// ---------------------------------------------------------------------------

/**
 * Set of citation IDs that are currently relevant based on the user's
 * active calculations and operations.
 */
const _activeCitationIds = writable<Set<string>>(new Set());

/**
 * Derived: array of full Citation objects for all active references,
 * sorted by year (ascending).
 */
export const activeCitations = derived(_activeCitationIds, ($ids) => {
  const citations: Citation[] = [];
  for (const id of $ids) {
    const c = _bibliographyMap.get(id);
    if (c) citations.push(c);
  }
  citations.sort((a, b) => a.year - b.year);
  return citations;
});

/**
 * Number of currently active citations.
 */
export const activeCitationCount = derived(
  _activeCitationIds,
  ($ids) => $ids.size,
);

// ---------------------------------------------------------------------------
// Citation API
// ---------------------------------------------------------------------------

/**
 * Add one or more citation keys to the active set.
 * Called when the kernel returns results with `citation_keys`.
 */
export function addCitations(keys: string[]): void {
  _activeCitationIds.update((set) => {
    const next = new Set(set);
    for (const key of keys) {
      if (_bibliographyMap.has(key)) {
        next.add(key);
      }
    }
    return next;
  });
}

/**
 * Remove a citation from the active set.
 */
export function removeCitation(key: string): void {
  _activeCitationIds.update((set) => {
    const next = new Set(set);
    next.delete(key);
    return next;
  });
}

/**
 * Clear all active citations (e.g. on workspace reset).
 */
export function clearCitations(): void {
  _activeCitationIds.set(new Set());
}

/**
 * Register a custom citation (e.g. from a BSM model).
 */
export function registerCitation(citation: Citation): void {
  _bibliographyMap.set(citation.id, citation);
}

/**
 * Look up a citation by ID from the bibliography database.
 */
export function getCitation(id: string): Citation | undefined {
  return _bibliographyMap.get(id);
}

/**
 * Format a citation in Physical Review style.
 * Example: "Peskin, M.E. and Schroeder, D.V., An Introduction to Quantum Field Theory, Addison-Wesley (1995)."
 */
export function formatCitation(c: Citation): string {
  const authorStr =
    c.authors.length <= 2
      ? c.authors.join(" and ")
      : `${c.authors[0]} et al.`;

  let parts = [`${authorStr}, "${c.title}"`];

  if (c.journal) parts.push(c.journal);
  if (c.volume) parts.push(`**${c.volume}**`);
  if (c.pages) parts.push(c.pages);

  parts.push(`(${c.year})`);

  let result = parts.join(", ");

  if (c.arxiv) {
    result += ` [arXiv:${c.arxiv}]`;
  }

  return result + ".";
}

/**
 * Get all citations in the bibliography database.
 */
export function getAllCitations(): Citation[] {
  return Array.from(_bibliographyMap.values());
}
