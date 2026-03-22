/**
 * Zod Runtime Validation Schemas
 *
 * Composable schemas for every data structure that crosses the Rust ↔ TS
 * boundary via Tauri IPC or WASM.  These guard against silent data‐shape
 * drift when the Rust kernel evolves independently of the frontend.
 *
 * Usage:
 *   import { TheoreticalModelSchema } from "$lib/core/domain/schemas";
 *   const model = TheoreticalModelSchema.parse(raw);
 */

import { z } from "zod";

// ===========================================================================
// Primitives & Enumerations
// ===========================================================================

export const SpinSchema = z.number();
export const ChiralitySchema = z.enum(["Left", "Right"]);
export const HelicitySchema = z.enum(["Plus", "Minus"]);
export const ElectricChargeSchema = z.number();
export const WeakIsospinSchema = z.number();
export const HyperchargeSchema = z.number();
export const BaryonNumberSchema = z.number();

export const LeptonNumbersSchema = z.object({
  electron: z.number(),
  muon: z.number(),
  tau: z.number(),
});

export const ParitySchema = z.enum(["Even", "Odd"]);
export const ChargeConjugationSchema = z.enum(["Even", "Odd", "Undefined"]);

export const ColorRepresentationSchema = z.enum([
  "Singlet",
  "Triplet",
  "AntiTriplet",
  "Octet",
]);

export const WeakMultipletSchema = z.union([
  z.literal("Singlet"),
  z.literal("DoubletUp"),
  z.literal("DoubletDown"),
  z.object({ Triplet: z.number() }),
]);

export const InteractionTypeSchema = z.enum([
  "Electromagnetic",
  "WeakCC",
  "WeakNC",
  "Strong",
  "Yukawa",
  "ScalarSelf",
]);

export const ShellConditionSchema = z.enum(["OnShell", "OffShell"]);

// ===========================================================================
// Gauge Groups & Lie Algebra
// ===========================================================================

export const LieGroupSchema = z.union([
  z.object({ U1: z.object({ label: z.string() }) }),
  z.object({ SU: z.object({ n: z.number(), label: z.string() }) }),
  z.object({ SO: z.object({ n: z.number(), label: z.string() }) }),
]);

export const LieGroupRepresentationSchema = z.object({
  group: LieGroupSchema,
  dimension: z.number(),
  charge: z.number().nullable(),
  is_conjugate: z.boolean(),
  dynkin_labels: z.array(z.number()),
  label: z.string(),
});

export const GaugeSymmetrySchema = z.object({
  groups: z.array(LieGroupSchema),
  label: z.string(),
});

// ===========================================================================
// Core Entity Schemas
// ===========================================================================

export const QuantumNumbersSchema = z.object({
  electric_charge: ElectricChargeSchema,
  weak_isospin: WeakIsospinSchema,
  hypercharge: HyperchargeSchema,
  baryon_number: BaryonNumberSchema,
  lepton_numbers: LeptonNumbersSchema,
  spin: SpinSchema,
  parity: ParitySchema,
  charge_conjugation: ChargeConjugationSchema,
  color: ColorRepresentationSchema,
  weak_multiplet: WeakMultipletSchema,
  representations: z.array(LieGroupRepresentationSchema).optional(),
});

export const FieldSchema = z.object({
  id: z.string(),
  name: z.string(),
  symbol: z.string(),
  mass: z.number(),
  width: z.number(),
  quantum_numbers: QuantumNumbersSchema,
  interactions: z.array(InteractionTypeSchema),
});

export const ParticleSchema = z.object({
  field: FieldSchema,
  shell: ShellConditionSchema,
  helicity: HelicitySchema.nullable(),
  chirality: ChiralitySchema.nullable(),
  is_anti: z.boolean(),
});

export const FourMomentumTupleSchema = z.tuple([
  z.number(),
  z.number(),
  z.number(),
  z.number(),
]);

export const QuantumStateSchema = z.object({
  particle: ParticleSchema,
  four_momentum: FourMomentumTupleSchema,
  normalization: z.number(),
});

// ===========================================================================
// Spacetime & Geometry
// ===========================================================================

export const MetricSignSchema = z.enum(["Plus", "Minus"]);

export const MetricSignatureSchema = z.object({
  signs: z.array(MetricSignSchema),
});

export const FlatMetricSchema = z.object({
  signature: MetricSignatureSchema,
});

export const SpacetimeDimensionSchema = z.union([
  z.object({ Fixed: z.number() }),
  z.object({ DimReg: z.object({ base: z.number() }) }),
]);

export const SpacetimeConfigSchema = z.object({
  metric: FlatMetricSchema,
  regularization: SpacetimeDimensionSchema,
});

export const PhysicalConstantsSchema = z.object({
  speed_of_light: z.number(),
  hbar: z.number(),
  gravitational_constant: z.number(),
  boltzmann_constant: z.number(),
});

// ===========================================================================
// S-Matrix & Reaction
// ===========================================================================

export const AsymptoticStateSchema = z.object({
  states: z.array(QuantumStateSchema),
  invariant_mass_sq: z.number(),
});

export const ReactionSchema = z.object({
  initial: AsymptoticStateSchema,
  final_state: AsymptoticStateSchema,
  is_valid: z.boolean(),
  violation_diagnostics: z.array(z.string()),
  interaction_types: z.array(InteractionTypeSchema),
  mediating_bosons: z.array(ParticleSchema),
  perturbative_order: z.number(),
});

export const ReconstructedFinalStateSchema = z.object({
  particles: z.array(ParticleSchema),
  weight: z.number(),
  interaction_chain: z.array(InteractionTypeSchema),
});

// ===========================================================================
// Lagrangian & Feynman Rules
// ===========================================================================

export const LagrangianTermKindSchema = z.enum([
  "Kinetic",
  "Mass",
  "Interaction",
  "GaugeFixing",
  "Ghost",
  "ContactInteraction",
]);

export const LagrangianTermSchema = z.object({
  id: z.string(),
  description: z.string(),
  coupling_symbol: z.string(),
  coupling_value: z.number().nullable(),
  field_ids: z.array(z.string()),
  lorentz_structure: z.string(),
  interaction_type: InteractionTypeSchema,
  term_kind: LagrangianTermKindSchema,
  operator_dimension: z.number().nullable(),
});

export const VertexFactorSchema = z.object({
  term_id: z.string(),
  field_ids: z.array(z.string()),
  expression: z.string(),
  coupling_value: z.number().nullable(),
  n_legs: z.number(),
});

export const PropagatorFormSchema = z.enum([
  "Scalar",
  "DiracFermion",
  "MasslessVector",
  "MassiveVector",
  "RaritaSchwinger",
  "MasslessSpin2",
  "MassiveSpin2",
]);

export const PropagatorSchema = z.object({
  field_id: z.string(),
  spin: SpinSchema,
  mass: z.number(),
  width: z.number(),
  expression: z.string(),
  gauge_parameter: z.number().nullable(),
  form: PropagatorFormSchema,
});

// ===========================================================================
// Feynman Diagram Topology
// ===========================================================================

export const ChannelSchema = z.enum(["S", "T", "U"]);

export const LoopOrderSchema = z.union([
  z.literal("Tree"),
  z.literal("OneLoop"),
  z.literal("TwoLoop"),
  z.object({ NLoop: z.number() }),
]);

export const NodeKindSchema = z.union([
  z.object({ ExternalIncoming: ParticleSchema }),
  z.object({ ExternalOutgoing: ParticleSchema }),
  z.object({ InternalVertex: VertexFactorSchema }),
]);

export const FeynmanNodeSchema = z.object({
  id: z.number(),
  kind: NodeKindSchema,
  position: z.tuple([z.number(), z.number()]).nullable(),
});

export const FeynmanEdgeSchema = z.object({
  field: FieldSchema,
  propagator: PropagatorSchema.nullable(),
  momentum_label: z.string(),
  is_external: z.boolean(),
});

export const OneLoopTopologyKindSchema = z.union([
  z.literal("Tadpole"),
  z.literal("Bubble"),
  z.literal("Triangle"),
  z.literal("Box"),
  z.object({ NPoint: z.number() }),
]);

export const MomentumExpressionSchema = z.object({
  loop_coefficients: z.array(z.tuple([z.string(), z.number()])),
  external_coefficients: z.array(z.tuple([z.string(), z.number()])),
});

export const LoopMomentumRoutingSchema = z.object({
  loop_momenta: z.array(z.string()),
  edge_momenta: z.array(
    z.tuple([z.number(), z.number(), MomentumExpressionSchema]),
  ),
});

// ===========================================================================
// Telemetry & Performance Profiling
// ===========================================================================

export const ComputeProfileSchema = z.object({
  stage_timings: z.record(z.string(), z.number()),
  total_time_ms: z.number(),
  peak_memory_mb: z.number(),
  threads_used: z.number().int(),
  convergence_data: z.array(z.tuple([z.number(), z.number()])),
});

export const FeynmanDiagramSchema = z.object({
  id: z.number(),
  nodes: z.array(FeynmanNodeSchema),
  edges: z.array(z.tuple([z.number(), z.number(), FeynmanEdgeSchema])),
  channels: z.array(ChannelSchema),
  loop_order: LoopOrderSchema,
  symmetry_factor: z.number(),
  is_connected: z.boolean(),
  is_one_particle_irreducible: z.boolean().nullable().optional(),
  loop_topology_kind: OneLoopTopologyKindSchema.nullable().optional(),
  momentum_routing: LoopMomentumRoutingSchema.nullable().optional(),
});

export const TopologySetSchema = z.object({
  reaction_id: z.string(),
  max_loop_order: LoopOrderSchema,
  diagrams: z.array(FeynmanDiagramSchema),
  count: z.number(),
  profile: ComputeProfileSchema.optional(),
});

// ===========================================================================
// Amplitudes & CAS
// ===========================================================================

export const FourMomentumSchema = z.object({
  e: z.number(),
  px: z.number(),
  py: z.number(),
  pz: z.number(),
});

export const AmplitudeResultSchema = z.object({
  diagram_id: z.number(),
  expression: z.string(),
  couplings: z.array(z.string()),
  momenta_labels: z.array(z.string()),
  profile: ComputeProfileSchema.optional(),
}).passthrough();

// CasExpr is deeply recursive - validate structurally without full recursion
// to avoid Zod stack overflows on deeply nested expressions.
export const CasExprSchema: z.ZodType = z.lazy(() =>
  z.union([
    z.object({ Scalar: z.number() }),
    z.literal("ImaginaryUnit"),
    z.object({
      Symbol: z.object({ name: z.string(), indices: z.array(z.unknown()) }),
    }),
    z.object({
      MetricTensor: z.object({ mu: z.unknown(), nu: z.unknown() }),
    }),
    z.object({ GammaMat: z.object({ index: z.unknown() }) }),
    z.literal("Gamma5"),
    z.object({
      Momentum: z.object({ label: z.string(), index: z.unknown() }),
    }),
    z.object({ SlashedMomentum: z.object({ label: z.string() }) }),
    z.object({
      DotProduct: z.object({ left: z.string(), right: z.string() }),
    }),
    z.object({
      SpinorU: z.object({ label: z.string(), momentum: z.string() }),
    }),
    z.object({
      SpinorUBar: z.object({ label: z.string(), momentum: z.string() }),
    }),
    z.object({
      SpinorV: z.object({ label: z.string(), momentum: z.string() }),
    }),
    z.object({
      SpinorVBar: z.object({ label: z.string(), momentum: z.string() }),
    }),
    z.object({ Add: z.array(z.lazy(() => CasExprSchema)) }),
    z.object({ Mul: z.array(z.lazy(() => CasExprSchema)) }),
    z.object({ Neg: z.lazy(() => CasExprSchema) }),
    z.object({
      Fraction: z.object({
        numerator: z.lazy(() => CasExprSchema),
        denominator: z.lazy(() => CasExprSchema),
      }),
    }),
    z.object({ Trace: z.lazy(() => CasExprSchema) }),
    z.object({
      PropagatorDenom: z.object({ momentum: z.string(), mass_sq: z.number() }),
    }),
    // Catch-all for less common variants
    z.record(z.string(), z.unknown()),
  ]),
);

export const DerivationStepSchema = z.object({
  label: z.string(),
  description: z.string(),
  expression: CasExprSchema,
  latex: z.string(),
});

export const ObservableKindSchema = z.enum([
  "Amplitude",
  "CrossSection",
  "DecayWidth",
  "BranchingRatio",
]);

export const DimensionalCheckReportSchema = z.object({
  observable: ObservableKindSchema,
  expected_mass_dimension: z.number(),
  inferred_mass_dimension: z.number(),
  is_consistent: z.boolean(),
  message: z.string(),
  diagnostics: z.array(z.string()),
});

export const SimplifiedExpressionResultSchema = z.object({
  original_latex: z.string(),
  simplified_expression: CasExprSchema,
  simplified_latex: z.string(),
  applied_rules: z.array(z.string()),
  dimension_check: DimensionalCheckReportSchema,
});

// ===========================================================================
// Kinematics
// ===========================================================================

export const MandelstamVarsSchema = z.object({
  s: z.number(),
  t: z.number(),
  u: z.number(),
});

export const MandelstamBoundariesSchema = z.object({
  s_min: z.number(),
  t_min: z.number(),
  t_max: z.number(),
  masses: z.tuple([z.number(), z.number(), z.number(), z.number()]),
});

export const DalitzBoundariesSchema = z.object({
  mother_mass: z.number(),
  daughter_masses: z.tuple([z.number(), z.number(), z.number()]),
  m_ab_sq_min: z.number(),
  m_ab_sq_max: z.number(),
  m_bc_sq_min: z.number(),
  m_bc_sq_max: z.number(),
});

export const DalitzPlotDataSchema = z.object({
  boundaries: DalitzBoundariesSchema,
  points: z.array(z.tuple([z.number(), z.number()])),
  n_grid: z.number(),
});

export const ThresholdResultSchema = z.object({
  threshold_energy: z.number(),
  lab_energy: z.number().nullable(),
  final_masses: z.array(z.number()),
});

export const PhaseSpaceSchema = z.object({
  n_final: z.number(),
  n_variables: z.number(),
  total_energy_cm: z.number(),
  final_masses: z.array(z.number()),
  measure_expression: z.string(),
});

export const KinematicsReportSchema = z.object({
  threshold: ThresholdResultSchema,
  is_allowed: z.boolean(),
  phase_space: PhaseSpaceSchema.nullable(),
  mandelstam_boundaries: MandelstamBoundariesSchema.nullable(),
});

// ===========================================================================
// Analysis & Histogramming
// ===========================================================================

export const HistogramDataSchema = z.object({
  name: z.string(),
  bin_edges: z.array(z.number()),
  bin_contents: z.array(z.number()),
  bin_errors: z.array(z.number()),
  error_band_up: z.array(z.number()).nullable().optional(),
  error_band_down: z.array(z.number()).nullable().optional(),
  pdf_uncertainty: z.array(z.number()).nullable().optional(),
  total_uncertainty: z.array(z.number()).nullable().optional(),
  underflow: z.number(),
  overflow: z.number(),
  entries: z.number(),
  mean: z.number(),
});

export const Histogram2DDataSchema = z.object({
  name: z.string(),
  x_bin_edges: z.array(z.number()),
  y_bin_edges: z.array(z.number()),
  bin_contents: z.array(z.number()),
  nx: z.number(),
  ny: z.number(),
  entries: z.number(),
  total_weight: z.number(),
});

export const AnalysisResultSchema = z.object({
  histograms: z.array(HistogramDataSchema),
  histograms_2d: z.array(Histogram2DDataSchema),
  cross_section: z.number(),
  cross_section_error: z.number(),
  events_generated: z.number(),
  events_passed: z.number(),
  profile: ComputeProfileSchema.optional(),
});

export const GoodnessOfFitResultSchema = z.object({
  chi_square: z.number(),
  ndf: z.number().int(),
  chi_square_reduced: z.number(),
  p_value: z.number(),
  pulls: z.array(z.number()),
});

export const ObservableFitInputSchema = z.object({
  observable: z.string(),
  theory_bin_contents: z.array(z.number()),
  theory_bin_edges: z.array(z.number()),
  exp_csv: z.string(),
});

export const ObservableFitSummarySchema = z.object({
  observable: z.string(),
  result: GoodnessOfFitResultSchema,
});

export const GlobalObservableFitResultSchema = z.object({
  observables: z.array(ObservableFitSummarySchema),
  chi_square_global: z.number(),
  ndf_total: z.number().int(),
  n_params: z.number().int(),
  chi_square_reduced: z.number(),
  p_value: z.number(),
});

// ===========================================================================
// Cross-Section
// ===========================================================================

export const CrossSectionResultSchema = z.object({
  cross_section: z.number(),
  uncertainty: z.number(),
  cross_section_pb: z.number(),
  uncertainty_pb: z.number(),
  events_evaluated: z.number(),
  relative_error: z.number(),
  cms_energy: z.number(),
  amplitude_model: z.string(),
  profile: ComputeProfileSchema.optional(),
});

// ===========================================================================
// 3D Event Display
// ===========================================================================

export const Vec3Schema = z.object({
  x: z.number(),
  y: z.number(),
  z: z.number(),
});

export const DisplayJetSchema = z.object({
  direction: Vec3Schema,
  energy: z.number(),
  pt: z.number(),
  eta: z.number(),
  phi: z.number(),
  n_constituents: z.number(),
});

export const DisplayTrackSchema = z.object({
  direction: Vec3Schema,
  energy: z.number(),
  pt: z.number(),
  eta: z.number(),
  particle_type: z.string(),
});

export const DisplayMETSchema = z.object({
  direction: Vec3Schema,
  magnitude: z.number(),
});

export const EventDisplayDataSchema = z.object({
  jets: z.array(DisplayJetSchema),
  electrons: z.array(DisplayTrackSchema),
  muons: z.array(DisplayTrackSchema),
  photons: z.array(DisplayTrackSchema),
  met: DisplayMETSchema,
  cms_energy: z.number(),
});

// ===========================================================================
// Lagrangian AST (Phase 32)
// ===========================================================================

export const IndexKindSchema = z.union([
  z.literal("Lorentz"),
  z.object({ Gauge: z.string() }),
]);

export const IndexPositionSchema = z.enum(["Upper", "Lower"]);

export const IndexSlotSchema = z.object({
  name: z.string(),
  kind: IndexKindSchema,
  position: IndexPositionSchema,
});

export const FieldSpinSchema = z.enum([
  "Scalar",
  "Fermion",
  "Vector",
  "ThreeHalf",
  "Tensor2",
]);

// LagrangianExpr is recursive
export const LagrangianExprSchema: z.ZodType = z.lazy(() =>
  z.union([
    z.object({ RealConstant: z.number() }),
    z.object({
      CouplingConstant: z.object({
        name: z.string(),
        value: z.number().nullable(),
      }),
    }),
    z.object({
      FieldOp: z.object({
        field_id: z.string(),
        spin: FieldSpinSchema,
        is_adjoint: z.boolean(),
        lorentz_indices: z.array(IndexSlotSchema),
        gauge_indices: z.array(IndexSlotSchema),
      }),
    }),
    z.object({ GammaMat: z.object({ index: IndexSlotSchema }) }),
    z.literal("Gamma5"),
    z.object({ Derivative: z.object({ index: IndexSlotSchema }) }),
    z.object({
      CovariantDerivative: z.object({
        index: IndexSlotSchema,
        gauge_field_id: z.string().nullable(),
      }),
    }),
    z.object({
      Metric: z.object({ mu: IndexSlotSchema, nu: IndexSlotSchema }),
    }),
    z.object({
      FieldStrength: z.object({
        field_id: z.string(),
        mu: IndexSlotSchema,
        nu: IndexSlotSchema,
        gauge_index: IndexSlotSchema.nullable(),
      }),
    }),
    z.object({
      Product: z.array(z.lazy(() => LagrangianExprSchema)),
    }),
    z.object({ Sum: z.array(z.lazy(() => LagrangianExprSchema)) }),
    z.object({
      Scaled: z.object({
        factor: z.number(),
        inner: z.lazy(() => LagrangianExprSchema),
      }),
    }),
  ]),
);

export const ExternalFieldSchema = z.object({
  field_id: z.string(),
  is_adjoint: z.boolean(),
});

export const DerivedVertexRuleSchema = z.object({
  external_fields: z.array(ExternalFieldSchema),
  residual_expr: LagrangianExprSchema,
  latex: z.string(),
  symmetry_factor: z.number(),
  grassmann_sign: z.number(),
  n_legs: z.number(),
});

export const ValidationMessageSchema = z.object({
  severity: z.string(),
  check: z.string(),
  message: z.string(),
});

export const ValidationResultSchema = z.object({
  is_lorentz_scalar: z.boolean(),
  is_gauge_singlet: z.boolean(),
  is_hermitian: z.boolean(),
  is_renormalisable: z.boolean(),
  mass_dimension: z.number(),
  messages: z.array(ValidationMessageSchema),
});

export const RgeFlowResultSchema = z.object({
  coupling_name: z.string(),
  mu_values: z.array(z.number()),
  coupling_values: z.array(z.number()),
});

// ===========================================================================
// SLHA Spectrum Import
// ===========================================================================

export const DecayChannelSchema = z.object({
  branching_ratio: z.number(),
  n_daughters: z.number(),
  daughter_pdg_ids: z.array(z.number()),
});

export const SlhaBlockSchema = z.object({
  name: z.string(),
  scale: z.number().nullable(),
  entries: z.record(z.string(), z.number()),
  comments: z.array(z.string()),
});

export const SlhaDecaySchema = z.object({
  pdg_id: z.number(),
  total_width: z.number(),
  channels: z.array(DecayChannelSchema),
});

export const SlhaDocumentSchema = z.object({
  blocks: z.record(z.string(), SlhaBlockSchema),
  decays: z.record(z.string(), SlhaDecaySchema),
});

// ===========================================================================
// UFO Model Import
// ===========================================================================

export const UfoParticleSchema = z.object({
  pdg_code: z.number(),
  name: z.string(),
  antiname: z.string(),
  spin: z.number(),
  color: z.number(),
  mass_name: z.string(),
  width_name: z.string(),
  charge: z.number(),
  extra: z.record(z.string(), z.string()),
});

export const UfoCouplingSchema = z.object({
  name: z.string(),
  value: z.string(),
  order: z.record(z.string(), z.number()),
});

export const UfoLorentzSchema = z.object({
  name: z.string(),
  spins: z.array(z.number()),
  structure: z.string(),
});

export const UfoVertexSchema = z.object({
  name: z.string(),
  particles: z.array(z.string()),
  color: z.array(z.string()),
  lorentz: z.array(z.string()),
  couplings: z.record(z.string(), z.string()),
});

export const UfoParameterSchema = z.object({
  name: z.string(),
  nature: z.string(),
  param_type: z.string(),
  value: z.number().nullable(),
  expression: z.string().nullable(),
  texname: z.string(),
  lhablock: z.string().nullable(),
  lhacode: z.array(z.number()).nullable(),
});

export const UfoModelSchema = z.object({
  particles: z.array(UfoParticleSchema),
  vertices: z.array(UfoVertexSchema),
  couplings: z.array(UfoCouplingSchema),
  parameters: z.array(UfoParameterSchema),
  lorentz_structures: z.array(UfoLorentzSchema),
});

export const UfoExportResultSchema = z.record(z.string(), z.string());

// ===========================================================================
// NLO Counterterms
// ===========================================================================

export const CountertermKindSchema = z.enum([
  "FieldStrength",
  "Mass",
  "Coupling",
]);

export const RenormalizationConstantSchema = z.object({
  name: z.string(),
  original_parameter: z.string(),
  kind: CountertermKindSchema,
  value: z.number().nullable(),
});

export const CountertermEntrySchema = z.object({
  expression: LagrangianExprSchema,
  delta_parameters: z.array(z.string()),
  description: z.string(),
  latex: z.string(),
});

export const CountertermResultSchema = z.object({
  tree_level_expr: LagrangianExprSchema,
  counterterms: z.array(CountertermEntrySchema),
  counterterm_rules: z.array(DerivedVertexRuleSchema),
  renorm_constants: z.array(RenormalizationConstantSchema),
});

// ===========================================================================
// Theoretical Model (top-level aggregate)
// ===========================================================================

export const TheoreticalModelSchema = z.object({
  name: z.string(),
  description: z.string(),
  fields: z.array(FieldSchema),
  terms: z.array(LagrangianTermSchema),
  vertex_factors: z.array(VertexFactorSchema),
  propagators: z.array(PropagatorSchema),
  gauge_symmetry: GaugeSymmetrySchema.nullable().optional(),
  spacetime: SpacetimeConfigSchema.optional(),
  constants: PhysicalConstantsSchema.optional(),
}).passthrough();

// ===========================================================================
// Parameter Scanner (Phase 44)
// ===========================================================================

/** Linear or logarithmic spacing for scan points. */
export const ScanScaleSchema = z.enum(["Linear", "Logarithmic"]);
export type ScanScale = z.infer<typeof ScanScaleSchema>;

/** A single parameter to be swept. */
export const ScanVariableSchema = z.object({
  /** Dot-separated path, e.g. "field.Z.mass", "vertex.eeZ.coupling", "cms_energy". */
  target: z.string(),
  /** Lower bound. */
  min: z.number(),
  /** Upper bound. */
  max: z.number(),
  /** Number of evaluation points (≥ 2). */
  steps: z.number().int().min(2),
  /** Linear or logarithmic spacing. */
  scale: ScanScaleSchema,
});
export type ScanVariable = z.infer<typeof ScanVariableSchema>;

/** Configuration for a 1D parameter scan. */
export const ScanConfig1DSchema = z.object({
  variable: ScanVariableSchema,
  model: TheoreticalModelSchema,
  final_masses: z.array(z.number()),
  cms_energy: z.number(),
  events_per_point: z.number().int().min(1),
});
export type ScanConfig1D = z.infer<typeof ScanConfig1DSchema>;

/** Result of a 1D parameter scan. */
export const ScanResult1DSchema = z.object({
  variable: ScanVariableSchema,
  x_values: z.array(z.number()),
  y_values: z.array(z.number()),
  y_errors: z.array(z.number()),
});
export type ScanResult1D = z.infer<typeof ScanResult1DSchema>;

/** Configuration for a 2D parameter scan. */
export const ScanConfig2DSchema = z.object({
  variable_x: ScanVariableSchema,
  variable_y: ScanVariableSchema,
  model: TheoreticalModelSchema,
  final_masses: z.array(z.number()),
  cms_energy: z.number(),
  events_per_point: z.number().int().min(1),
});
export type ScanConfig2D = z.infer<typeof ScanConfig2DSchema>;

/** Result of a 2D parameter scan. */
export const ScanResult2DSchema = z.object({
  variable_x: ScanVariableSchema,
  variable_y: ScanVariableSchema,
  x_values: z.array(z.number()),
  y_values: z.array(z.number()),
  z_values: z.array(z.number()),
  z_errors: z.array(z.number()),
});
export type ScanResult2D = z.infer<typeof ScanResult2DSchema>;

// ===========================================================================
// IPC Response Envelope
// ===========================================================================

export const SpireResponseSchema = z.object({
  success: z.boolean(),
  data: z.string(),
  error: z.string().nullable(),
});

// ===========================================================================
// Convenience: safeParse helper
// ===========================================================================

/**
 * Parse `data` with `schema`, returning the typed result or throwing a
 * descriptive `BackendValidationError`.
 */
export function validateResponse<T>(
  schema: z.ZodType<T>,
  data: unknown,
  context: string,
): T {
  const result = schema.safeParse(data);
  if (result.success) return result.data;

  const issues = result.error.issues
    .map((i) => `  • ${i.path.join(".")}: ${i.message}`)
    .join("\n");
  throw new Error(
    `[SPIRE] Response validation failed for "${context}":\n${issues}`,
  );
}

// ===========================================================================
// Decay Calculator
// ===========================================================================

export const CalcDecayChannelSchema = z.object({
  final_state: z.array(z.string()),
  final_state_names: z.array(z.string()),
  partial_width: z.number(),
  branching_ratio: z.number(),
  vertex_id: z.string(),
});

export const CalcDecayTableSchema = z.object({
  parent_id: z.string(),
  parent_name: z.string(),
  parent_mass: z.number(),
  total_width: z.number(),
  lifetime_seconds: z.number(),
  channels: z.array(CalcDecayChannelSchema),
});

// ===========================================================================
// NLO Dipole Subtraction Configuration (Phase 46)
// ===========================================================================

export const NloConfigSchema = z.object({
  enabled: z.boolean(),
  subtraction_scheme: z.enum(["CataniSeymour", "FKS", "Antenna"]),
  y_min: z.number(),
  y_max: z.number(),
  alpha: z.number(),
});
export type NloConfig = z.infer<typeof NloConfigSchema>;

// ===========================================================================
// Parton Shower Configuration (Phase 46)
// ===========================================================================

export const ShowerToggleConfigSchema = z.object({
  enabled: z.boolean(),
  provider: z.enum(["pythia8", "herwig7", "sherpa", "custom"]),
  executable_path: z.string(),
  hadronisation: z.boolean(),
  qed_radiation: z.boolean(),
  mpi: z.boolean(),
  seed: z.number().int(),
});
export type ShowerToggleConfig = z.infer<typeof ShowerToggleConfigSchema>;

// ===========================================================================
// Plugin System (Phase 54)
// ===========================================================================

export const PluginInfoSchema = z.object({
  name: z.string(),
  version: z.string(),
  description: z.string(),
  author: z.string(),
  capabilities: z.array(z.string()),
  enabled: z.boolean(),
});
export type PluginInfo = z.infer<typeof PluginInfoSchema>;

// ===========================================================================
// Global Fits & MCMC (Phase 55)
// ===========================================================================

export const FitPropertySchema = z.union([
  z.literal("Mass"),
  z.literal("Width"),
  z.object({ Coupling: z.string() }),
]);

export const FitParameterSchema = z.object({
  name: z.string(),
  min: z.number(),
  max: z.number(),
  field_id: z.string(),
  property: FitPropertySchema,
});

export const GaussianConstraintSchema = z.object({
  name: z.string(),
  observed: z.number(),
  sigma: z.number(),
  field_id: z.string(),
  property: FitPropertySchema,
});

export const GlobalFitConfigSchema = z.object({
  parameters: z.array(FitParameterSchema),
  gaussian_constraints: z.array(GaussianConstraintSchema),
  n_walkers: z.number().int(),
  n_steps: z.number().int(),
  burn_in: z.number().int(),
});

export const McmcFitRequestSchema = z.object({
  model: TheoreticalModelSchema,
  config: GlobalFitConfigSchema,
});

export const McmcStatusSchema = z.object({
  current_step: z.number().int(),
  total_steps: z.number().int(),
  acceptance_fraction: z.number(),
  running: z.boolean(),
  stopped: z.boolean(),
});

export const McmcFitStatusSchema = z.object({
  status: McmcStatusSchema,
  flat_samples: z.array(z.array(z.number())).nullable(),
  param_names: z.array(z.string()),
});

// ===========================================================================
// Particle Data Group (PDG) Integration (Phase 73)
// ===========================================================================

export const AsymmetricErrorSchema = z.object({
  minus: z.number(),
  plus: z.number(),
});

export const PdgValueSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("exact"),
    value: z.number(),
    is_limit: z.boolean().optional(),
  }),
  z.object({
    kind: z.literal("symmetric"),
    value: z.number(),
    error: z.number(),
    is_limit: z.boolean().optional(),
  }),
  z.object({
    kind: z.literal("asymmetric"),
    value: z.number(),
    error: AsymmetricErrorSchema,
    is_limit: z.boolean().optional(),
  }),
]);

export const PdgQuantumNumbersSchema = z.object({
  charge: z.number().optional(),
  spin_j: z.string().optional(),
  parity: z.string().optional(),
  c_parity: z.string().optional(),
});

export const PdgBranchingFractionSchema = z.object({
  pdgid: z.string(),
  description: z.string(),
  value: PdgValueSchema,
});

export const PdgProvenanceSchema = z.object({
  edition: z.string(),
  release_timestamp: z.string().optional(),
  source_id: z.string(),
  origin: z.string().optional(),
  source_path: z.string().optional(),
  extraction_policy: z.string().optional(),
  source_arbitration_outcome: z.string().optional(),
  local_file_fingerprint: z.string().optional(),
  fingerprint: z.string(),
});

export const PdgMetadataSchema = z.object({
  edition: z.string(),
  version: z.string(),
  timestamp: z.string(),
  source_files: z.array(z.string()),
});

export const PdgCacheBucketDiagnosticsSchema = z.object({
  hits: z.number(),
  misses: z.number(),
  evictions: z.number(),
  size: z.number().int(),
  capacity: z.number().int(),
  hit_rate: z.number(),
});

export const PdgCacheDiagnosticsSchema = z.object({
  particle_records: PdgCacheBucketDiagnosticsSchema,
  decay_tables: PdgCacheBucketDiagnosticsSchema,
  id_resolution: PdgCacheBucketDiagnosticsSchema,
  total_hits: z.number(),
  total_misses: z.number(),
  total_evictions: z.number(),
  total_entries: z.number().int(),
  db_queries: z.number(),
  db_average_latency_us: z.number(),
  db_last_latency_us: z.number(),
});

export const PdgParticleRecordSchema = z.object({
  pdg_id: z.number().int(),
  label: z.string().optional(),
  mass: PdgValueSchema.optional(),
  width: PdgValueSchema.optional(),
  lifetime: PdgValueSchema.optional(),
  branching_fractions: z.array(PdgBranchingFractionSchema).default([]),
  quantum_numbers: PdgQuantumNumbersSchema,
  provenance: PdgProvenanceSchema,
});

export const PdgDecayProductSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("concrete"), mcid: z.number().int() }),
  z.object({ kind: z.literal("generic"), description: z.string() }),
]);

export const PdgDecayChannelSchema = z.object({
  mode_id: z.number().int(),
  products: z.array(z.tuple([PdgDecayProductSchema, z.number().int()])),
  branching_ratio: PdgValueSchema.optional(),
  is_generic: z.boolean(),
  description: z.string().optional(),
});

export const PdgDecayTableSchema = z.object({
  parent_pdg_id: z.number().int(),
  channels: z.array(PdgDecayChannelSchema),
  edition: z.string(),
});

export const PdgCatalogChunkSchema = z.object({
  request_id: z.string().nullable().optional(),
  offset: z.number().int(),
  limit: z.number().int(),
  total: z.number().int(),
  done: z.boolean(),
  cancelled: z.boolean(),
  records: z.array(PdgParticleRecordSchema),
});

export const PdgExtractionPolicySchema = z.enum(["StrictPhysical", "Catalog"]);
