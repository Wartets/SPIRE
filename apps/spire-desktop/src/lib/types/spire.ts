/**
 * SPIRE TypeScript Type Definitions
 *
 * These interfaces exactly mirror the serialized data structures returned
 * by the Rust kernel (spire-kernel) and bindings (spire-bindings).
 *
 * The Tauri IPC commands and WASM API serialize Rust structs to JSON,
 * and these TypeScript interfaces define the shape of that JSON on the
 * frontend side.
 */

// ---------------------------------------------------------------------------
// Quantum Numbers & Enumerations
// ---------------------------------------------------------------------------

/** Spin stored as twice the spin value (e.g., spin-½ → 1, spin-1 → 2). */
export type Spin = number;

/** Chirality projection of a fermion. */
export type Chirality = "Left" | "Right";

/** Helicity — spin projection along the direction of motion. */
export type Helicity = "Plus" | "Minus";

/** Electric charge in units of e, numerator over 3 for quarks. */
export type ElectricCharge = number;

/** Weak isospin T₃, stored as twice the value. */
export type WeakIsospin = number;

/** Hypercharge Y under U(1)_Y, stored as integer multiple of 1/3. */
export type Hypercharge = number;

/** Baryon number B, stored as three times the actual value. */
export type BaryonNumber = number;

/** Lepton family numbers. */
export interface LeptonNumbers {
  electron: number;
  muon: number;
  tau: number;
}

/** Intrinsic parity under spatial inversion. */
export type Parity = "Even" | "Odd";

/** Charge conjugation eigenvalue. */
export type ChargeConjugation = "Even" | "Odd" | "Undefined";

/** SU(3)_C colour representation. */
export type ColorRepresentation =
  | "Singlet"
  | "Triplet"
  | "AntiTriplet"
  | "Octet";

/** Position within a weak isospin multiplet. */
export type WeakMultiplet =
  | "Singlet"
  | "DoubletUp"
  | "DoubletDown"
  | { Triplet: number };

/** Interaction type classification. */
export type InteractionType =
  | "Electromagnetic"
  | "WeakCC"
  | "WeakNC"
  | "Strong"
  | "Yukawa"
  | "ScalarSelf";

/** On-shell vs off-shell condition. */
export type ShellCondition = "OnShell" | "OffShell";

// ---------------------------------------------------------------------------
// Core Entity Interfaces
// ---------------------------------------------------------------------------

/** Complete set of quantum numbers for a particle. */
export interface QuantumNumbers {
  electric_charge: ElectricCharge;
  weak_isospin: WeakIsospin;
  hypercharge: Hypercharge;
  baryon_number: BaryonNumber;
  lepton_numbers: LeptonNumbers;
  spin: Spin;
  parity: Parity;
  charge_conjugation: ChargeConjugation;
  color: ColorRepresentation;
  weak_multiplet: WeakMultiplet;
  /** Generalized gauge-group representations */
  representations?: LieGroupRepresentation[];
}

/** A quantum field definition. */
export interface Field {
  id: string;
  name: string;
  symbol: string;
  mass: number;
  width: number;
  quantum_numbers: QuantumNumbers;
  interactions: InteractionType[];
}

/** A concrete particle (on-shell or off-shell excitation of a Field). */
export interface Particle {
  field: Field;
  shell: ShellCondition;
  helicity: Helicity | null;
  chirality: Chirality | null;
  is_anti: boolean;
}

/** A quantum state vector for an asymptotic particle. */
export interface QuantumState {
  particle: Particle;
  four_momentum: [number, number, number, number];
  normalization: number;
}

// ---------------------------------------------------------------------------
// Symmetry Groups & Conservation Laws
// ---------------------------------------------------------------------------

/** Gauge group classification (legacy SM-specific). */
export type GaugeGroup = "U1Y" | "SU2L" | "SU3C";

/** Representation of a particle under a gauge group (legacy SM-specific). */
export interface GaugeRepresentation {
  group: GaugeGroup;
  dimension: number;
  label: string;
}

// ---------------------------------------------------------------------------
// Generalized Lie Algebra Types
// ---------------------------------------------------------------------------

/**
 * A Lie group that can appear as a gauge symmetry factor.
 *
 * Supports U(1), SU(N), and SO(N) families.
 */
export type LieGroup =
  | { U1: { label: string } }
  | { SU: { n: number; label: string } }
  | { SO: { n: number; label: string } };

/**
 * A representation of a particle under a specific LieGroup.
 *
 * Encodes dimension, U(1) charge (if applicable), conjugation flag,
 * and Dynkin labels for non-Abelian groups.
 */
export interface LieGroupRepresentation {
  group: LieGroup;
  dimension: number;
  charge: number | null;
  is_conjugate: boolean;
  dynkin_labels: number[];
  label: string;
}

/**
 * A complete gauge symmetry specification for a theoretical model.
 *
 * Describes the full product group (e.g., SU(3)_C × SU(2)_L × U(1)_Y)
 * as an ordered list of simple factors.
 */
export interface GaugeSymmetry {
  groups: LieGroup[];
  label: string;
}

/** Poincaré group representation (mass + spin). */
export interface PoincareRepresentation {
  mass: number;
  twice_spin: number;
  is_massless: boolean;
}

/** Result of validating conservation laws for a proposed reaction. */
export interface ConservationResult {
  is_valid: boolean;
  violations: string[];
}

/** Result of discrete symmetry (C, P, T) checks. */
export interface DiscreteSymmetryResult {
  c_parity_conserved: boolean | null;
  p_parity_conserved: boolean | null;
  t_symmetry_conserved: boolean | null;
  cp_conserved: boolean | null;
  diagnostics: string[];
}

// ---------------------------------------------------------------------------
// S-Matrix & Reaction
// ---------------------------------------------------------------------------

/** An asymptotic state (initial or final). */
export interface AsymptoticState {
  states: QuantumState[];
  invariant_mass_sq: number;
}

/** A fully specified and validated reaction. */
export interface Reaction {
  initial: AsymptoticState;
  final_state: AsymptoticState;
  is_valid: boolean;
  violation_diagnostics: string[];
  interaction_types: InteractionType[];
  mediating_bosons: Particle[];
  perturbative_order: number;
}

/** A reconstructed possible final state (from incomplete process). */
export interface ReconstructedFinalState {
  particles: Particle[];
  weight: number;
  interaction_chain: InteractionType[];
}

// ---------------------------------------------------------------------------
// Lagrangian & Feynman Rules
// ---------------------------------------------------------------------------

/** Classification of a Lagrangian term. */
export type LagrangianTermKind =
  | "Kinetic"
  | "Mass"
  | "Interaction"
  | "GaugeFixing"
  | "Ghost"
  | "ContactInteraction";

/** A single term in the Lagrangian density. */
export interface LagrangianTerm {
  id: string;
  description: string;
  coupling_symbol: string;
  coupling_value: number | null;
  field_ids: string[];
  lorentz_structure: string;
  interaction_type: InteractionType;
  term_kind: LagrangianTermKind;
  /** Mass dimension of the operator (4 = renormalisable, 5/6 = EFT). Null defaults to 4. */
  operator_dimension: number | null;
}

/** A Feynman rule vertex factor derived from a Lagrangian term. */
export interface VertexFactor {
  term_id: string;
  field_ids: string[];
  expression: string;
  coupling_value: number | null;
  n_legs: number;
}

/** A propagator Feynman rule for an internal line. */
export interface Propagator {
  field_id: string;
  spin: Spin;
  mass: number;
  width: number;
  expression: string;
  gauge_parameter: number | null;
}

// ---------------------------------------------------------------------------
// Feynman Diagram Topology
// ---------------------------------------------------------------------------

/** Mandelstam channel classification. */
export type Channel = "S" | "T" | "U";

/** Loop order of a diagram. */
export type LoopOrder = "Tree" | "OneLoop" | "TwoLoop" | { NLoop: number };

/** Classification of a node in the Feynman graph. */
export type NodeKind =
  | { ExternalIncoming: Particle }
  | { ExternalOutgoing: Particle }
  | { Vertex: VertexFactor };

/** A node in the Feynman diagram. */
export interface FeynmanNode {
  id: number;
  kind: NodeKind;
  position: [number, number] | null;
}

/** An edge (propagator line) in the Feynman diagram. */
export interface FeynmanEdge {
  particle: Particle;
  propagator: Propagator | null;
  momentum_label: string;
  is_external: boolean;
}

/** A complete Feynman diagram. */
export interface FeynmanDiagram {
  id: number;
  nodes: FeynmanNode[];
  edges: [number, number, FeynmanEdge][];
  channels: Channel[];
  loop_order: LoopOrder;
  symmetry_factor: number;
  is_connected: boolean;
  /** Whether this diagram is 1-particle irreducible */
  is_one_particle_irreducible?: boolean | null;
  /** Loop topology classification for 1-loop diagrams */
  loop_topology_kind?: OneLoopTopologyKind | null;
  /** Momentum routing assignment for loop diagrams */
  momentum_routing?: LoopMomentumRouting | null;
}

// ---------------------------------------------------------------------------
// Loop Calculus & Dimensional Regularization Types
// ---------------------------------------------------------------------------

/** Classification of 1-loop integral topologies. */
export type OneLoopTopologyKind =
  | "Tadpole"
  | "Bubble"
  | "Triangle"
  | "Box"
  | { NPoint: number };

/** A symbolic linear combination of loop and external momenta. */
export interface MomentumExpression {
  /** Coefficients for loop momenta, e.g., [["l", 1]]. */
  loop_coefficients: [string, number][];
  /** Coefficients for external momenta, e.g., [["p1", 1], ["p2", -1]]. */
  external_coefficients: [string, number][];
}

/** Momentum routing for a loop diagram. */
export interface LoopMomentumRouting {
  /** Labels for the independent loop momenta (e.g., ["l"] for 1-loop). */
  loop_momenta: string[];
  /** Edge momentum assignments: [source_node, target_node, momentum_expression]. */
  edge_momenta: [number, number, MomentumExpression][];
}

/**
 * Spacetime dimension for loop calculations.
 *
 * - `{ Fixed: 4 }` for tree-level (d = 4 exactly).
 * - `{ DimReg: { base: 4 } }` for dimensional regularization (d = 4 - 2ε).
 */
export type SpacetimeDimension =
  | { Fixed: number }
  | { DimReg: { base: number } };

/**
 * Classification of standard scalar 1-loop integrals
 * in the Passarino-Veltman decomposition scheme.
 */
export type ScalarIntegralType =
  | { A0: { mass_sq: number } }
  | { B0: { p_sq: string; m1_sq: number; m2_sq: number } }
  | { C0: { external_momenta_sq: string[]; masses_sq: number[] } }
  | { D0: { external_momenta_sq: string[]; mandelstam_invariants: string[]; masses_sq: number[] } }
  | { NPoint: { n: number; external_momenta: string[]; masses_sq: number[] } };

/** Set of all diagrams generated for a reaction at a given order. */
export interface TopologySet {
  reaction_id: string;
  max_loop_order: LoopOrder;
  diagrams: FeynmanDiagram[];
  count: number;
}

// ---------------------------------------------------------------------------
// Algebra & Amplitudes
// ---------------------------------------------------------------------------

/** A relativistic four-momentum (E, px, py, pz). */
export interface FourMomentum {
  e: number;
  px: number;
  py: number;
  pz: number;
}

// ---------------------------------------------------------------------------
// Generalized Spacetime Geometry Types
// ---------------------------------------------------------------------------

/** Sign of a single metric tensor diagonal component. */
export type MetricSign = "Plus" | "Minus";

/** Diagonal metric signature for an N-dimensional spacetime. */
export interface MetricSignature {
  signs: MetricSign[];
}

/** A flat diagonal metric tensor. */
export interface FlatMetric {
  signature: MetricSignature;
}

/** Complete spacetime geometry configuration. */
export interface SpacetimeConfig {
  metric: FlatMetric;
  regularization: SpacetimeDimension;
}

/** N-dimensional spacetime vector. */
export interface SpacetimeVector {
  components: number[];
}

/** Fundamental physical constants. */
export interface PhysicalConstants {
  speed_of_light: number;
  hbar: number;
  gravitational_constant: number;
  boltzmann_constant: number;
}

/** Polarization state of a vector boson. */
export type PolarizationState =
  | "TransversePlus"
  | "TransverseMinus"
  | "Longitudinal";

// ---------------------------------------------------------------------------
// Computer Algebra System (CAS) Types
// ---------------------------------------------------------------------------

/** A symbolic Lorentz index (named, numeric, or dummy). */
export type LorentzIndex =
  | { Named: string }
  | { Numeric: number }
  | { Dummy: number };

/** Classification of spacetime tensor symmetry/structure. */
export type SpacetimeTensorKind =
  | "Scalar"
  | "Vector"
  | "VectorSpinor"
  | "SymmetricRank2"
  | "AntiSymmetricRank2"
  | { GeneralRank: number };

/**
 * The recursive CAS expression tree.
 *
 * Every mathematical object in an amplitude derivation is a CasExpr node.
 * The frontend can render these into LaTeX via `to_cas_latex()` on the Rust side,
 * or traverse the tree for interactive display.
 */
export type CasExpr =
  | { Scalar: number }
  | "ImaginaryUnit"
  | { Symbol: { name: string; indices: LorentzIndex[] } }
  | { MetricTensor: { mu: LorentzIndex; nu: LorentzIndex } }
  | { LeviCivita: { indices: [LorentzIndex, LorentzIndex, LorentzIndex, LorentzIndex] } }
  | { GammaMat: { index: LorentzIndex } }
  | "Gamma5"
  | { Momentum: { label: string; index: LorentzIndex } }
  | { SlashedMomentum: { label: string } }
  | { DotProduct: { left: string; right: string } }
  | { SpinorU: { label: string; momentum: string } }
  | { SpinorUBar: { label: string; momentum: string } }
  | { SpinorV: { label: string; momentum: string } }
  | { SpinorVBar: { label: string; momentum: string } }
  | { Tensor: { label: string; kind: SpacetimeTensorKind; indices: LorentzIndex[]; momentum: string | null; is_conjugate: boolean } }
  | { Add: CasExpr[] }
  | { Mul: CasExpr[] }
  | { Neg: CasExpr }
  | { Fraction: { numerator: CasExpr; denominator: CasExpr } }
  | { Trace: CasExpr }
  | { Commutator: [CasExpr, CasExpr] }
  | { AntiCommutator: [CasExpr, CasExpr] }
  | { KroneckerDelta: { mu: LorentzIndex; nu: LorentzIndex } }
  | { PropagatorDenom: { momentum: string; mass_sq: number } };

/**
 * A single step in a structured amplitude derivation
 *
 * The `derive_amplitude_steps` function returns an ordered array of these,
 * each representing one mathematical operation applied to the amplitude.
 */
export interface DerivationStep {
  /** Short label for the step (e.g., "Feynman Rules", "Dirac Equation"). */
  label: string;
  /** Human-readable description of the transformation. */
  description: string;
  /** The CAS expression after this step has been applied. */
  expression: CasExpr;
  /** LaTeX rendering of the expression at this step. */
  latex: string;
}

/** Propagator form classification */
export type PropagatorForm =
  | "Scalar"
  | "DiracFermion"
  | "MasslessVector"
  | "MassiveVector"
  | "RaritaSchwinger"
  | "MasslessSpin2"
  | "MassiveSpin2";

/** A symbolic amplitude expression. */
export interface AmplitudeResult {
  diagram_id: number;
  expression: string;
  couplings: string[];
  momenta_labels: string[];
}

/**
 * A symbolic loop integral term in the Passarino-Veltman basis
 *
 * This represents an unevaluated scalar loop integral that can be
 * exported to external libraries (LoopTools, Package-X) for evaluation.
 */
export interface LoopIntegralTerm {
  integral_type: ScalarIntegralType;
  spacetime_dim: SpacetimeDimension;
  loop_momentum_label: string;
  external_momenta: string[];
  description: string;
}

/** Result of a gamma-matrix trace evaluation. */
export interface TraceResult {
  input: string;
  result: string;
  steps: string[];
}

/** Result of contracting Lorentz indices in an amplitude expression. */
export interface ContractionResult {
  input: string;
  result: string;
}

// ---------------------------------------------------------------------------
// Kinematics
// ---------------------------------------------------------------------------

/** Mandelstam variables (s, t, u). */
export interface MandelstamVars {
  s: number;
  t: number;
  u: number;
}

/** Physical boundaries of Mandelstam variables. */
export interface MandelstamBoundaries {
  s_min: number;
  t_min: number;
  t_max: number;
  masses: [number, number, number, number];
}

/** Dalitz plot boundaries for a 3-body decay. */
export interface DalitzBoundaries {
  mother_mass: number;
  daughter_masses: [number, number, number];
  m_ab_sq_min: number;
  m_ab_sq_max: number;
  m_bc_sq_min: number;
  m_bc_sq_max: number;
}

/** Dalitz plot data: kinematic boundaries + phase-space points. */
export interface DalitzPlotData {
  boundaries: DalitzBoundaries;
  /** Array of [s_ab, s_bc] coordinate pairs in GeV². */
  points: [number, number][];
  /** Number of grid divisions used along the s_ab axis. */
  n_grid: number;
}

/** Threshold energy computation result. */
export interface ThresholdResult {
  threshold_energy: number;
  lab_energy: number | null;
  final_masses: number[];
}

/** Phase space description. */
export interface PhaseSpace {
  n_final: number;
  n_variables: number;
  total_energy_cm: number;
  final_masses: number[];
  measure_expression: string;
}

/** Aggregate kinematics report returned by the `compute_kinematics` command. */
export interface KinematicsReport {
  threshold: ThresholdResult;
  is_allowed: boolean;
  phase_space: PhaseSpace | null;
  mandelstam_boundaries: MandelstamBoundaries | null;
}

// ---------------------------------------------------------------------------
// Export Types
// ---------------------------------------------------------------------------

/** Result of a UFO model export — maps filenames to Python source content. */
export interface UfoExportResult {
  /** Map of filename → file content, e.g. { "particles.py": "...", "parameters.py": "..." } */
  [filename: string]: string;
}

// ---------------------------------------------------------------------------
// Cross-Section Calculation
// ---------------------------------------------------------------------------

/** Result of a Monte Carlo cross-section calculation. */
export interface CrossSectionResult {
  /** Total cross-section in natural units (GeV^-2). */
  cross_section: number;
  /** Statistical uncertainty on the cross-section (GeV^-2). */
  uncertainty: number;
  /** Cross-section converted to picobarns. */
  cross_section_pb: number;
  /** Uncertainty converted to picobarns. */
  uncertainty_pb: number;
  /** Number of phase-space events evaluated. */
  events_evaluated: number;
  /** Relative error (uncertainty / cross_section). */
  relative_error: number;
  /** Centre-of-mass energy used (GeV). */
  cms_energy: number;
  /** Description of the squared matrix element model used. */
  amplitude_model: string;
}

// ---------------------------------------------------------------------------
// Hadronic Cross-Section Calculation
// ---------------------------------------------------------------------------

/** Result of a hadronic (PDF-convoluted) cross-section calculation. */
export interface HadronicCrossSectionResult {
  /** Total hadronic cross-section in natural units (GeV^-2). */
  cross_section: number;
  /** Statistical uncertainty in natural units (GeV^-2). */
  uncertainty: number;
  /** Cross-section in picobarns. */
  cross_section_pb: number;
  /** Uncertainty in picobarns. */
  uncertainty_pb: number;
  /** Number of events evaluated. */
  events_evaluated: number;
  /** Relative error (uncertainty / cross_section). */
  relative_error: number;
  /** Squared beam energy S = (p1 + p2)^2 in GeV^2. */
  beam_energy_sq: number;
  /** Name of the PDF set used. */
  pdf_name: string;
  /** Description of the initial-state hadrons. */
  beam_description: string;
}

/** Hadron species for initial-state beams. */
export type Hadron = "Proton" | "AntiProton" | "PionPlus" | "PionMinus";

/**
 * Form factor parametrisation for off-shell vertex suppression.
 *
 * - `PointLike`: No suppression (F = 1).
 * - `Monopole`: F(Q²) = (1 + Q²/Λ²)^{-1}
 * - `Dipole`: F(Q²) = (1 + Q²/Λ²)^{-2}
 * - `Exponential`: F(Q²) = exp(-Q²/Λ²)
 */
export type FormFactor =
  | "PointLike"
  | { Monopole: { lambda_sq: number } }
  | { Dipole: { lambda_sq: number } }
  | { Exponential: { lambda_sq: number } };

// ---------------------------------------------------------------------------
// Theoretical Model
// ---------------------------------------------------------------------------

/** A complete theoretical model (SM, BSM extension, etc.). */
export interface TheoreticalModel {
  name: string;
  description: string;
  fields: Field[];
  terms: LagrangianTerm[];
  vertex_factors: VertexFactor[];
  propagators: Propagator[];
  /** The gauge symmetry of this model */
  gauge_symmetry?: GaugeSymmetry | null;
  /** Spacetime geometry configuration */
  spacetime?: SpacetimeConfig | null;
  /** Fundamental physical constants */
  constants?: PhysicalConstants | null;
}

// ---------------------------------------------------------------------------
// IPC Response Wrapper
// ---------------------------------------------------------------------------

/** Generic response envelope from Tauri IPC commands. */
export interface SpireResponse {
  success: boolean;
  data: string;
  error: string | null;
}

// ---------------------------------------------------------------------------
// Application-Level Types
// ---------------------------------------------------------------------------

/** Theoretical framework selection. */
export type TheoreticalFramework =
  | "StandardModel"
  | "QED"
  | "QCD"
  | "ElectroWeak"
  | "BSM"
  | "Custom";

/** The complete state of the physics workspace. */
export interface WorkspaceState {
  /** The active theoretical framework. */
  framework: TheoreticalFramework;
  /** The current reaction being analysed (null if none set). */
  active_reaction: Reaction | null;
  /** Generated diagrams for the active reaction. */
  diagrams: TopologySet | null;
  /** Computed amplitude results. */
  amplitudes: AmplitudeResult[];
  /** Kinematic analysis results. */
  kinematics: MandelstamVars | null;
}

// ---------------------------------------------------------------------------
// Scripting — Observables & Cuts
// ---------------------------------------------------------------------------

/** A user-defined observable script. */
export interface ObservableScript {
  /** Human-readable name (e.g., "Invariant mass m₁₂"). */
  name: string;
  /** Rhai script source that computes a scalar from a PhaseSpacePoint. */
  script: string;
  /** Whether this script compiles successfully. */
  isValid: boolean;
  /** Compilation error message, if any. */
  errorMessage?: string;
  /** Result from the last test evaluation (synthetic event). */
  testResult?: number;
}

/** A user-defined kinematic cut script. */
export interface CutScript {
  /** Human-readable name (e.g., "pT > 50 GeV"). */
  name: string;
  /** Rhai script source that returns a boolean. */
  script: string;
  /** Whether this script compiles successfully. */
  isValid: boolean;
  /** Compilation error message, if any. */
  errorMessage?: string;
  /** Whether the test event passed this cut. */
  testResult?: boolean;
}

// ---------------------------------------------------------------------------
// Analysis & Histogramming
// ---------------------------------------------------------------------------

/** Definition of a single plot to fill during analysis. */
export interface PlotDefinition {
  /** Human-readable name (e.g., "Muon pT"). */
  name: string;
  /** Rhai script returning a numeric observable value. */
  observable_script: string;
  /** Number of histogram bins. */
  n_bins: number;
  /** Lower edge of the histogram range. */
  min: number;
  /** Upper edge of the histogram range. */
  max: number;
}

/** Complete analysis configuration sent to the backend. */
export interface AnalysisConfig {
  /** Plot definitions with observable scripts and binning. */
  plots: PlotDefinition[];
  /** Optional kinematic cut scripts. */
  cut_scripts: string[];
  /** Number of Monte Carlo events to generate. */
  num_events: number;
  /** Centre-of-mass energy in GeV. */
  cms_energy: number;
  /** Final-state particle masses in GeV. */
  final_masses: number[];
  /**
   * Detector preset name for phenomenological simulation.
   *
   * Supported values: `"perfect"`, `"lhc_like"`, `"ilc_like"`.
   * When `undefined` or `null`, no detector simulation is applied.
   */
  detector_preset?: string | null;
  /**
   * Classification of each final-state particle by detector subsystem.
   *
   * Must have the same length as `final_masses` when `detector_preset` is set.
   * Accepted values: `"electron"`, `"muon"`, `"photon"`, `"hadron"`, `"invisible"`.
   * When `undefined`, all particles are treated as hadrons.
   */
  particle_kinds?: string[] | null;
  /**
   * Optional 2D correlation plot definitions.
   *
   * Each entry specifies two observable scripts (X and Y axes) and binning
   * parameters for a 2D heatmap.
   */
  plots_2d?: PlotDefinition2D[] | null;
}

/** Available detector presets for phenomenological simulation. */
export type DetectorPreset = 'perfect' | 'lhc_like' | 'ilc_like';

/** Particle kind classification for detector subsystem routing. */
export type ParticleKind = 'electron' | 'muon' | 'photon' | 'hadron' | 'invisible';

/** Serialized histogram data from the backend. */
export interface HistogramData {
  /** Human-readable name. */
  name: string;
  /** Bin edges (length = n_bins + 1). */
  bin_edges: number[];
  /** Bin contents (weighted counts, length = n_bins). */
  bin_contents: number[];
  /** Statistical errors per bin. */
  bin_errors: number[];
  /** Underflow count. */
  underflow: number;
  /** Overflow count. */
  overflow: number;
  /** Total number of entries. */
  entries: number;
  /** Distribution mean. */
  mean: number;
}

/** Complete analysis result from the backend. */
export interface AnalysisResult {
  /** Filled histogram data for each requested plot. */
  histograms: HistogramData[];
  /** Filled 2D histogram data for each requested 2D correlation plot. */
  histograms_2d: Histogram2DData[];
  /** Estimated total cross-section (GeV⁻²). */
  cross_section: number;
  /** Statistical uncertainty on the cross-section. */
  cross_section_error: number;
  /** Total events generated. */
  events_generated: number;
  /** Events passing all kinematic cuts. */
  events_passed: number;
}

// ---------------------------------------------------------------------------
// 2D Histogramming
// ---------------------------------------------------------------------------

/** Serialized 2D histogram data for heatmap rendering. */
export interface Histogram2DData {
  /** Human-readable name. */
  name: string;
  /** X-axis bin edges (length = nx + 1). */
  x_bin_edges: number[];
  /** Y-axis bin edges (length = ny + 1). */
  y_bin_edges: number[];
  /** Bin contents in row-major order (length = nx * ny). */
  bin_contents: number[];
  /** Number of bins on the X axis. */
  nx: number;
  /** Number of bins on the Y axis. */
  ny: number;
  /** Total number of entries. */
  entries: number;
  /** Total accumulated weight. */
  total_weight: number;
}

/** Definition of a 2D correlation plot. */
export interface PlotDefinition2D {
  /** Human-readable name (e.g., "pT vs η"). */
  name: string;
  /** Rhai script returning the X-axis observable value. */
  x_observable_script: string;
  /** Rhai script returning the Y-axis observable value. */
  y_observable_script: string;
  /** Number of bins on the X axis. */
  nx: number;
  /** Lower edge of the X-axis range. */
  x_min: number;
  /** Upper edge of the X-axis range. */
  x_max: number;
  /** Number of bins on the Y axis. */
  ny: number;
  /** Lower edge of the Y-axis range. */
  y_min: number;
  /** Upper edge of the Y-axis range. */
  y_max: number;
}

// ---------------------------------------------------------------------------
// 3D Event Display
// ---------------------------------------------------------------------------

/** 3D vector for event display rendering. */
export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/** Jet representation for 3D event display. */
export interface DisplayJet {
  direction: Vec3;
  energy: number;
  pt: number;
  eta: number;
  phi: number;
  n_constituents: number;
}

/** Track representation for 3D event display (leptons, photons). */
export interface DisplayTrack {
  direction: Vec3;
  energy: number;
  pt: number;
  eta: number;
  particle_type: string;
}

/** Missing transverse energy for 3D event display. */
export interface DisplayMET {
  direction: Vec3;
  magnitude: number;
}

/** Complete event display data for the 3D visualiser. */
export interface EventDisplayData {
  jets: DisplayJet[];
  electrons: DisplayTrack[];
  muons: DisplayTrack[];
  photons: DisplayTrack[];
  met: DisplayMET;
  cms_energy: number;
}
