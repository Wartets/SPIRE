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

/** Gauge group classification. */
export type GaugeGroup = "U1Y" | "SU2L" | "SU3C";

/** Representation of a particle under a gauge group. */
export interface GaugeRepresentation {
  group: GaugeGroup;
  dimension: number;
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
}

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

/** Polarization state of a vector boson. */
export type PolarizationState =
  | "TransversePlus"
  | "TransverseMinus"
  | "Longitudinal";

/** A symbolic amplitude expression. */
export interface AmplitudeResult {
  diagram_id: number;
  expression: string;
  couplings: string[];
  momenta_labels: string[];
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
