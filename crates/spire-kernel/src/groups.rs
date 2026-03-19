//! # Symmetry Groups and Conservation Laws
//!
//! This module implements the mathematical structures of gauge symmetry groups.
//!
//! ## Generalized Lie Algebra Engine (Phase 14)
//!
//! SPIRE supports **arbitrary gauge groups** at runtime. The [`LieGroup`] enum
//! can represent any $U(1)$, $SU(N)$, or $SO(N)$ group. Particles carry a
//! dynamic vector of [`LieGroupRepresentation`]s instead of (or in addition to)
//! the hardcoded SM quantum numbers.
//!
//! Conservation law validation uses two complementary systems:
//! - **Legacy**: Additive quantum number checks (electric charge, baryon number, etc.)
//! - **Generalized**: $N$-ality checks for $SU(N)$ and additive $U(1)$ charges.
//!
//! Discrete symmetries (C, P, T and their combinations) are evaluated depending
//! on the interaction type.

use serde::{Deserialize, Serialize};

use crate::ontology::{InteractionType, QuantumState};
use crate::SpireError;
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Symmetry Group Representations
// ---------------------------------------------------------------------------

/// Abstract representation of a Lie group element used in gauge symmetry checks.
///
/// Each variant encodes the dimensionality and structure constants of the
/// corresponding gauge group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GaugeGroup {
    /// $U(1)_Y$ - Abelian hypercharge symmetry.
    U1Y,
    /// $SU(2)_L$ - Non-Abelian weak isospin symmetry.
    SU2L,
    /// $SU(3)_C$ - Non-Abelian colour symmetry (QCD).
    SU3C,
}

/// Representation of a particle under a specific gauge group.
///
/// Encodes the dimension and any Casimir invariants needed for
/// coupling-constant computations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaugeRepresentation {
    /// Which gauge group this representation belongs to.
    pub group: GaugeGroup,
    /// Dimension of the representation (1 = singlet, 2 = doublet, 3 = triplet, etc.).
    pub dimension: u8,
    /// Label string for display (e.g., `"3"`, `"\\bar{3}"`, `"8"`).
    pub label: String,
}

/// Representation of the Poincaré group characterizing a particle's spacetime
/// transformation behaviour.
///
/// A particle is classified by its mass $m$ (Casimir $P^\mu P_\mu = m^2$) and
/// spin $s$ (Casimir $W^\mu W_\mu = -m^2 s(s+1)$), corresponding to an
/// irreducible unitary representation of the Poincaré group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoincareRepresentation {
    /// Invariant mass $m$ in GeV.
    pub mass: f64,
    /// Spin quantum number $s$ (twice the value, stored as integer).
    pub twice_spin: u8,
    /// Whether the representation is massive ($m > 0$) or massless ($m = 0$).
    pub is_massless: bool,
}

// ---------------------------------------------------------------------------
// Generalized Lie Algebra Engine (Phase 14)
// ---------------------------------------------------------------------------

/// A Lie group that can appear as a gauge symmetry factor.
///
/// Supports the three families relevant to particle physics:
/// - $U(1)$ - Abelian groups (hypercharge, dark photon, etc.)
/// - $SU(N)$ - Special unitary groups (colour, weak isospin, GUT groups)
/// - $SO(N)$ - Special orthogonal groups (Lorentz group, $SO(10)$ GUT)
///
/// The `label` field is a human-readable identifier for display and TOML parsing
/// (e.g., `"Y"`, `"C"`, `"L"`, `"'"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LieGroup {
    /// Abelian group $U(1)$. The label distinguishes multiple $U(1)$ factors
    /// (e.g., `"Y"` for hypercharge, `"EM"` for electromagnetism, `"'"` for
    /// dark photon).
    U1 {
        /// Human-readable subgroup label.
        label: String,
    },
    /// Special unitary group $SU(N)$. `n` is the rank parameter.
    SU {
        /// Group size parameter in $SU(N)$.
        n: u8,
        /// Human-readable subgroup label.
        label: String,
    },
    /// Special orthogonal group $SO(N)$. `n` is the dimension.
    SO {
        /// Group size parameter in $SO(N)$.
        n: u8,
        /// Human-readable subgroup label.
        label: String,
    },
}

impl LieGroup {
    /// Return the rank of the Lie algebra (number of Cartan generators).
    ///
    /// - $U(1)$: rank 1
    /// - $SU(N)$: rank $N-1$
    /// - $SO(N)$: rank $\lfloor N/2 \rfloor$
    pub fn rank(&self) -> u32 {
        match self {
            LieGroup::U1 { .. } => 1,
            LieGroup::SU { n, .. } => (*n as u32).saturating_sub(1),
            LieGroup::SO { n, .. } => (*n as u32) / 2,
        }
    }

    /// Return the dimension of the adjoint representation.
    ///
    /// - $U(1)$: 1
    /// - $SU(N)$: $N^2 - 1$
    /// - $SO(N)$: $N(N-1)/2$
    pub fn adjoint_dimension(&self) -> u32 {
        match self {
            LieGroup::U1 { .. } => 1,
            LieGroup::SU { n, .. } => {
                let n = *n as u32;
                n * n - 1
            }
            LieGroup::SO { n, .. } => {
                let n = *n as u32;
                n * (n - 1) / 2
            }
        }
    }

    /// Return a human-readable display string, e.g. `"SU(3)_C"`.
    pub fn display_name(&self) -> String {
        match self {
            LieGroup::U1 { label } => format!("U(1)_{}", label),
            LieGroup::SU { n, label } => format!("SU({})_{}", n, label),
            LieGroup::SO { n, label } => format!("SO({})_{}", n, label),
        }
    }
}

impl std::fmt::Display for LieGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A representation of a particle under a specific [`LieGroup`].
///
/// Encodes the dimension, $U(1)$ charge (if applicable), conjugation flag,
/// and Dynkin labels for higher representations of non-Abelian groups.
///
/// # Examples
/// - Electron under $SU(2)_L$: `dimension=2, dynkin_labels=[1], charge=None`
/// - Up quark under $SU(3)_C$: `dimension=3, dynkin_labels=[1,0], charge=None`
/// - Electron under $U(1)_Y$: `dimension=1, charge=Some(-1.0), dynkin_labels=[]`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LieGroupRepresentation {
    /// Which gauge group this representation belongs to.
    pub group: LieGroup,
    /// Dimension of the representation (1 = singlet, 2 = doublet, 3 = triplet, etc.).
    pub dimension: u32,
    /// For $U(1)$ groups: the charge under this $U(1)$ factor.
    /// For non-Abelian groups this is `None`.
    pub charge: Option<f64>,
    /// Whether this is the conjugate representation ($\bar{R}$).
    /// E.g., $\bar{3}$ of $SU(3)$ has `is_conjugate = true`.
    pub is_conjugate: bool,
    /// Dynkin labels for the highest weight of this representation.
    ///
    /// For $SU(N)$: an $(N-1)$-element vector.
    /// Examples for $SU(3)$: `[1,0]` = **3**, `[0,1]` = **3̄**, `[1,1]` = **8**.
    /// For $U(1)$: empty (charge is stored in the `charge` field).
    #[serde(default)]
    pub dynkin_labels: Vec<u32>,
    /// Human-readable label (e.g., `"3"`, `"3̄"`, `"8"`, `"1"`, `"-1/3"`).
    pub label: String,
}

/// A complete gauge symmetry specification for a theoretical model.
///
/// Describes the full product group (e.g., $SU(3)_C \times SU(2)_L \times U(1)_Y$)
/// as an ordered list of simple factors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaugeSymmetry {
    /// The simple group factors comprising this gauge symmetry.
    pub groups: Vec<LieGroup>,
    /// Human-readable label for the full product group.
    pub label: String,
}

impl GaugeSymmetry {
    /// Construct the Standard Model gauge symmetry $SU(3)_C \times SU(2)_L \times U(1)_Y$.
    pub fn standard_model() -> Self {
        GaugeSymmetry {
            groups: vec![
                LieGroup::SU {
                    n: 3,
                    label: "C".into(),
                },
                LieGroup::SU {
                    n: 2,
                    label: "L".into(),
                },
                LieGroup::U1 { label: "Y".into() },
            ],
            label: "SU(3)_C × SU(2)_L × U(1)_Y".into(),
        }
    }

    /// Construct an $SU(5)$ GUT gauge symmetry.
    pub fn su5_gut() -> Self {
        GaugeSymmetry {
            groups: vec![LieGroup::SU {
                n: 5,
                label: "GUT".into(),
            }],
            label: "SU(5)".into(),
        }
    }

    /// Construct an $SO(10)$ GUT gauge symmetry.
    pub fn so10_gut() -> Self {
        GaugeSymmetry {
            groups: vec![LieGroup::SO {
                n: 10,
                label: "GUT".into(),
            }],
            label: "SO(10)".into(),
        }
    }

    /// Construct a dark sector $U(1)'$ gauge symmetry.
    pub fn dark_u1() -> Self {
        GaugeSymmetry {
            groups: vec![LieGroup::U1 { label: "'".into() }],
            label: "U(1)'".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// N-ality computation for SU(N)
// ---------------------------------------------------------------------------

/// Compute the $N$-ality of an $SU(N)$ representation from its Dynkin labels.
///
/// For a representation with Dynkin labels $[a_1, a_2, \ldots, a_{N-1}]$,
/// the $N$-ality is:
///
/// $$\text{N-ality} = \left(\sum_{k=1}^{N-1} k \cdot a_k\right) \bmod N$$
///
/// # Special cases
/// - $SU(2)$: 2-ality (doublet=1, singlet=0, triplet=0)
/// - $SU(3)$: triality (triplet `[1,0]`=1, anti-triplet `[0,1]`=2, octet `[1,1]`=0)
/// - $SU(5)$: 5-ality (fundamental `[1,0,0,0]`=1, anti-fundamental `[0,0,0,1]`=4)
///
/// # Arguments
/// * `n` - The $N$ in $SU(N)$.
/// * `dynkin_labels` - The Dynkin labels of the representation (length $N-1$).
///
/// # Returns
/// The $N$-ality as an integer in $[0, N)$.
pub fn compute_n_ality(n: u8, dynkin_labels: &[u32]) -> u32 {
    let n = n as u32;
    if n == 0 {
        return 0;
    }
    let mut total: u32 = 0;
    for (k_minus_1, &a_k) in dynkin_labels.iter().enumerate() {
        let k = (k_minus_1 as u32) + 1;
        total += k * a_k;
    }
    total % n
}

/// Compute the $N$-ality of the conjugate representation.
///
/// If a representation has $N$-ality $k$, its conjugate has $N$-ality $(N - k) \bmod N$.
pub fn conjugate_n_ality(n: u8, n_ality: u32) -> u32 {
    let n = n as u32;
    if n == 0 || n_ality == 0 {
        return 0;
    }
    (n - n_ality) % n
}

// ---------------------------------------------------------------------------
// Generalized gauge conservation validation
// ---------------------------------------------------------------------------

/// Validate gauge invariance for a set of representations under a specific
/// [`LieGroup`].
///
/// - For $U(1)$: checks that the sum of charges is zero (additive conservation).
/// - For $SU(N)$: checks that the total $N$-ality is zero modulo $N$.
/// - For $SO(N)$: checks a $\mathbb{Z}_2$ parity (representations are real
///   or pseudoreal, so we check the overall "spinorial" parity).
///
/// # Arguments
/// * `group` - The Lie group to check.
/// * `representations` - The representations of all particles at the vertex/reaction.
///
/// # Returns
/// `Ok(true)` if gauge invariance is satisfied, `Ok(false)` otherwise.
pub fn validate_gauge_invariance(
    group: &LieGroup,
    representations: &[LieGroupRepresentation],
) -> SpireResult<bool> {
    // Filter to only representations under this group.
    let relevant: Vec<&LieGroupRepresentation> = representations
        .iter()
        .filter(|r| r.group == *group)
        .collect();

    if relevant.is_empty() {
        // No representations under this group - trivially satisfied.
        return Ok(true);
    }

    match group {
        LieGroup::U1 { .. } => {
            // Additive: sum of charges must be zero.
            let charge_sum: f64 = relevant.iter().map(|r| r.charge.unwrap_or(0.0)).sum();
            // Use a tolerance for floating-point comparison.
            Ok(charge_sum.abs() < 1e-10)
        }
        LieGroup::SU { n, .. } => {
            // Check total N-ality is zero mod N.
            // N-ality is computed directly from the Dynkin labels, which
            // already encode whether a representation is conjugate or not.
            // (e.g., [0,1] for 3̄ of SU(3) gives triality 2.)
            let n_val = *n;
            let mut total_n_ality: u32 = 0;
            for rep in &relevant {
                let na = compute_n_ality(n_val, &rep.dynkin_labels);
                total_n_ality += na;
            }
            Ok(total_n_ality.is_multiple_of(n_val as u32))
        }
        LieGroup::SO { n, .. } => {
            // For SO(N), representations are classified by a Z_2 grading:
            // spinor representations are "odd", tensor representations are "even".
            // A gauge-invariant vertex requires even total spinorial parity.
            //
            // Heuristic: spinorial reps of SO(2k) have dimension 2^(k-1).
            // For SO(10): spinor = 16, for SO(6) ≅ SU(4): spinor = 4.
            // We use a simple parity based on the dimension being a power of 2
            // and less than 2^(n/2).
            let n_val = *n as u32;
            let spinor_dim = 1u32 << (n_val / 2 - 1); // 2^(n/2-1) for SO(2k)
            let mut spinor_count = 0u32;
            for rep in &relevant {
                if rep.dimension == spinor_dim {
                    spinor_count += 1;
                }
            }
            Ok(spinor_count.is_multiple_of(2))
        }
    }
}

/// Validate gauge conservation for a full set of representations under all
/// groups in a [`GaugeSymmetry`].
///
/// Returns a [`ConservationResult`] with diagnostics for each violated group.
pub fn validate_generalized_conservation(
    gauge_symmetry: &GaugeSymmetry,
    initial_reps: &[LieGroupRepresentation],
    final_reps: &[LieGroupRepresentation],
) -> SpireResult<ConservationResult> {
    let mut violations = Vec::new();

    for group in &gauge_symmetry.groups {
        match group {
            LieGroup::U1 { label } => {
                // For U(1): sum of initial charges must equal sum of final charges.
                let sum_charge = |reps: &[LieGroupRepresentation]| -> f64 {
                    reps.iter()
                        .filter(|r| r.group == *group)
                        .map(|r| r.charge.unwrap_or(0.0))
                        .sum::<f64>()
                };
                let q_i = sum_charge(initial_reps);
                let q_f = sum_charge(final_reps);
                if (q_i - q_f).abs() > 1e-10 {
                    violations.push(format!(
                        "U(1)_{} charge violated: initial={:.4}, final={:.4}",
                        label, q_i, q_f
                    ));
                }
            }
            LieGroup::SU { n, label } => {
                // For SU(N): total N-ality must match between initial and final.
                // Dynkin labels already encode conjugation, so we compute
                // N-ality directly without applying conjugate_n_ality.
                let sum_n_ality = |reps: &[LieGroupRepresentation]| -> u32 {
                    reps.iter()
                        .filter(|r| r.group == *group)
                        .map(|r| compute_n_ality(*n, &r.dynkin_labels))
                        .sum::<u32>()
                };
                let na_i = sum_n_ality(initial_reps);
                let na_f = sum_n_ality(final_reps);
                if na_i % (*n as u32) != na_f % (*n as u32) {
                    violations.push(format!(
                        "SU({})_{} {}-ality violated: initial={}, final={}",
                        n,
                        label,
                        n,
                        na_i % (*n as u32),
                        na_f % (*n as u32)
                    ));
                }
            }
            LieGroup::SO { n, label } => {
                // For SO(N): check spinorial parity balance.
                let spinor_dim = 1u32 << ((*n as u32) / 2 - 1);
                let count_spinors = |reps: &[LieGroupRepresentation]| -> u32 {
                    reps.iter()
                        .filter(|r| r.group == *group && r.dimension == spinor_dim)
                        .count() as u32
                };
                let sp_i = count_spinors(initial_reps);
                let sp_f = count_spinors(final_reps);
                if (sp_i + sp_f) % 2 != 0 {
                    violations.push(format!(
                        "SO({})_{} spinorial parity violated: initial spinors={}, final spinors={}",
                        n, label, sp_i, sp_f
                    ));
                }
            }
        }
    }

    Ok(ConservationResult {
        is_valid: violations.is_empty(),
        violations,
    })
}

/// The result of validating a set of conservation laws for a proposed process.
///
/// If the process is forbidden, each violation is recorded with a detailed
/// theoretical diagnostic string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationResult {
    /// `true` if all conservation laws are satisfied.
    pub is_valid: bool,
    /// List of violations, each containing a human-readable explanation
    /// (e.g., "Lepton family number $L_e$ is violated: initial = +1, final = 0").
    pub violations: Vec<String>,
}

/// Result of a discrete-symmetry (C, P, T) check for a given process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscreteSymmetryResult {
    /// Whether C-parity is conserved (or N/A for charged states).
    pub c_parity_conserved: Option<bool>,
    /// Whether P-parity is conserved.
    pub p_parity_conserved: Option<bool>,
    /// Whether T-symmetry is conserved.
    pub t_symmetry_conserved: Option<bool>,
    /// Whether CP is conserved (relevant for weak interactions with CKM phases).
    pub cp_conserved: Option<bool>,
    /// Diagnostic messages for any violations.
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API - Conservation and Symmetry Validation
// ---------------------------------------------------------------------------

/// Validate all additive conservation laws for a proposed reaction.
///
/// Checks conservation of: electric charge, baryon number, lepton family
/// numbers ($L_e$, $L_\mu$, $L_\tau$), weak isospin $T_3$, and hypercharge $Y$.
/// Validation is performed using group-theoretic invariant sums, not simple
/// arithmetic.
///
/// # Arguments
/// * `initial` - Slice of initial-state `QuantumState` objects.
/// * `final_states` - Slice of final-state `QuantumState` objects.
/// * `interaction` - The type of interaction governing the process (determines
///   which quantum numbers must be strictly conserved).
pub fn validate_conservation_laws(
    initial: &[QuantumState],
    final_states: &[QuantumState],
    interaction: InteractionType,
) -> SpireResult<ConservationResult> {
    let mut violations = Vec::new();

    // Helper: sum an additive quantum number across a set of states.
    let sum_q = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.electric_charge.0 as i32)
            .sum()
    };
    let sum_b = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.baryon_number.0 as i32)
            .sum()
    };
    let sum_le = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.lepton_numbers.electron as i32)
            .sum()
    };
    let sum_lmu = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.lepton_numbers.muon as i32)
            .sum()
    };
    let sum_ltau = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.lepton_numbers.tau as i32)
            .sum()
    };
    let sum_t3 = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.weak_isospin.0 as i32)
            .sum()
    };
    let sum_y = |states: &[QuantumState]| -> i32 {
        states
            .iter()
            .map(|s| s.particle.field.quantum_numbers.hypercharge.0 as i32)
            .sum()
    };

    // Electric charge is ALWAYS conserved.
    let q_i = sum_q(initial);
    let q_f = sum_q(final_states);
    if q_i != q_f {
        violations.push(format!(
            "Electric charge violated: initial 3Q={}, final 3Q={}",
            q_i, q_f
        ));
    }

    // Baryon number is ALWAYS conserved (in the SM).
    let b_i = sum_b(initial);
    let b_f = sum_b(final_states);
    if b_i != b_f {
        violations.push(format!(
            "Baryon number violated: initial 3B={}, final 3B={}",
            b_i, b_f
        ));
    }

    // Lepton family numbers are conserved in all SM interactions at tree level.
    let le_i = sum_le(initial);
    let le_f = sum_le(final_states);
    if le_i != le_f {
        violations.push(format!(
            "Electron lepton number Le violated: initial={}, final={}",
            le_i, le_f
        ));
    }

    let lmu_i = sum_lmu(initial);
    let lmu_f = sum_lmu(final_states);
    if lmu_i != lmu_f {
        violations.push(format!(
            "Muon lepton number Lmu violated: initial={}, final={}",
            lmu_i, lmu_f
        ));
    }

    let ltau_i = sum_ltau(initial);
    let ltau_f = sum_ltau(final_states);
    if ltau_i != ltau_f {
        violations.push(format!(
            "Tau lepton number Ltau violated: initial={}, final={}",
            ltau_i, ltau_f
        ));
    }

    // Weak isospin T3 and hypercharge Y are conserved in EM and Strong
    // interactions, but NOT in weak CC/NC interactions (W/Z carry T3 and Y).
    match interaction {
        InteractionType::Electromagnetic | InteractionType::Strong => {
            let t3_i = sum_t3(initial);
            let t3_f = sum_t3(final_states);
            if t3_i != t3_f {
                violations.push(format!(
                    "Weak isospin T3 violated: initial 2T3={}, final 2T3={}",
                    t3_i, t3_f
                ));
            }
            let y_i = sum_y(initial);
            let y_f = sum_y(final_states);
            if y_i != y_f {
                violations.push(format!(
                    "Hypercharge Y violated: initial 3Y={}, final 3Y={}",
                    y_i, y_f
                ));
            }
        }
        _ => {} // T3 and Y are not conserved in weak/Yukawa/scalar interactions.
    }

    Ok(ConservationResult {
        is_valid: violations.is_empty(),
        violations,
    })
}

/// Check discrete symmetries (C, P, T, CP) for a proposed process.
///
/// Evaluates whether the process respects or violates the discrete symmetries
/// given the interaction type. Weak interactions may violate P and CP; strong
/// and electromagnetic interactions conserve all discrete symmetries.
///
/// # Arguments
/// * `initial` - Slice of initial-state `QuantumState` objects.
/// * `final_states` - Slice of final-state `QuantumState` objects.
/// * `interaction` - The governing interaction type.
pub fn check_cpt(
    _initial: &[QuantumState],
    _final_states: &[QuantumState],
    interaction: InteractionType,
) -> SpireResult<DiscreteSymmetryResult> {
    // Discrete symmetry expectations depend on the interaction type.
    // CPT is always conserved (CPT theorem); individual C, P, T may not be.
    let (c_conserved, p_conserved, t_conserved, cp_conserved, diagnostics) = match interaction {
        InteractionType::Electromagnetic | InteractionType::Strong => {
            (Some(true), Some(true), Some(true), Some(true), vec![])
        }
        InteractionType::WeakCC | InteractionType::WeakNC => {
            (
                Some(false),
                Some(false),
                Some(true),
                // CP is approximately conserved; violations arise from CKM phases.
                Some(true),
                vec![
                    "P is maximally violated in weak interactions.".to_string(),
                    "C is violated in weak interactions.".to_string(),
                ],
            )
        }
        InteractionType::Yukawa => {
            // Yukawa couplings in the SM conserve C, P, T.
            (Some(true), Some(true), Some(true), Some(true), vec![])
        }
        InteractionType::ScalarSelf => (Some(true), Some(true), Some(true), Some(true), vec![]),
    };

    Ok(DiscreteSymmetryResult {
        c_parity_conserved: c_conserved,
        p_parity_conserved: p_conserved,
        t_symmetry_conserved: t_conserved,
        cp_conserved,
        diagnostics,
    })
}

/// Validate that an interaction vertex is a colour singlet under $SU(3)_C$.
///
/// Ensures local gauge invariance for QCD vertices by checking that the
/// tensor product of colour representations at the vertex contains the
/// trivial (singlet) representation.
pub fn validate_color_singlet(representations: &[GaugeRepresentation]) -> SpireResult<bool> {
    // Check SU(3)_C representations using triality.
    // Triality: singlet=0, triplet(3)=1, antitriplet(3̄)=2, octet(8)=0.
    // A colour-singlet state requires total triality ≡ 0 (mod 3).
    let mut triality_sum: i32 = 0;
    for rep in representations {
        match rep.dimension {
            1 => {} // singlet: triality 0
            3 => {
                // Distinguish triplet vs antitriplet by label.
                if rep.label.contains("bar") || rep.label.contains('\u{0304}') {
                    triality_sum += 2; // antitriplet
                } else {
                    triality_sum += 1; // triplet
                }
            }
            8 => {} // adjoint octet: triality 0
            other => {
                return Err(SpireError::GroupTheoryError(format!(
                    "Unsupported SU(3) representation dimension: {}",
                    other
                )));
            }
        }
    }

    Ok(triality_sum.rem_euclid(3) == 0)
}

/// Validate Poincaré invariance for an interaction term.
///
/// Ensures that the interaction Lagrangian term transforms as a Lorentz scalar,
/// i.e., respects the full 10-parameter spacetime symmetry.
pub fn validate_poincare_invariance(
    representations: &[PoincareRepresentation],
) -> SpireResult<bool> {
    if representations.is_empty() {
        return Ok(true);
    }

    // A Lagrangian interaction term must transform as a Lorentz scalar (J=0).
    // We check whether the spins can couple to zero total angular momentum
    // using the triangle inequality iteratively:
    //   J_min..J_max for pair, then fold with next spin.
    //
    // All spins stored as 2s (integer), so J ranges are in units of 1/2.
    let mut j_min: u32 = 0;
    let mut j_max: u32 = 0;

    for (i, rep) in representations.iter().enumerate() {
        let s = rep.twice_spin as u32;
        if i == 0 {
            j_min = s;
            j_max = s;
        } else {
            // Couple previous range [j_min, j_max] with spin s.
            // New max = j_max + s.
            // New min = absolute difference of closest approach.
            let new_max = j_max + s;
            let new_min = if j_min > s {
                j_min - s
            } else {
                s.saturating_sub(j_max)
            };
            j_min = new_min;
            j_max = new_max;
        }
    }

    // J=0 is achievable if 0 is in [j_min, j_max] (in steps of 1 in half-integer units).
    // Since j_min is the smallest achievable J and j_max the largest,
    // J=0 requires j_min == 0.
    Ok(j_min == 0)
}

/// Compute the hypercharge $Y$ from the Gell-Mann–Nishijima relation:
/// $Q = T_3 + \tfrac{Y}{2}$.
///
/// Given electric charge $Q$ and weak isospin $T_3$, derives $Y$.
pub fn derive_hypercharge(electric_charge: i8, weak_isospin_t3: i8) -> i8 {
    // Gell-Mann–Nishijima: Q = T3 + Y/2.
    // In scaled integers (Q_s = 3Q, T3_s = 2T3, Y_s = 3Y):
    //   2 * Q_s = 3 * T3_s + Y_s
    //   Y_s = 2 * Q_s - 3 * T3_s
    let q = electric_charge as i16;
    let t3 = weak_isospin_t3 as i16;
    (2 * q - 3 * t3) as i8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::*;

    /// Helper: create a minimal QuantumState from quantum numbers.
    fn make_state(q: i8, b: i8, le: i8, lmu: i8, ltau: i8, t3: i8, y: i8) -> QuantumState {
        let field = Field {
            id: "test".into(),
            name: "Test".into(),
            symbol: "T".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(q),
                weak_isospin: WeakIsospin(t3),
                hypercharge: Hypercharge(y),
                baryon_number: BaryonNumber(b),
                lepton_numbers: LeptonNumbers {
                    electron: le,
                    muon: lmu,
                    tau: ltau,
                },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![],
        };
        let particle = Particle {
            field,
            shell: ShellCondition::OnShell,
            helicity: None,
            chirality: None,
            is_anti: false,
        };
        QuantumState {
            particle,
            four_momentum: [1.0, 0.0, 0.0, 0.0],
            normalization: 2.0,
        }
    }

    #[test]
    fn conservation_result_serde() {
        let cr = ConservationResult {
            is_valid: true,
            violations: vec![],
        };
        let json = serde_json::to_string(&cr).unwrap();
        let cr2: ConservationResult = serde_json::from_str(&json).unwrap();
        assert!(cr2.is_valid);
    }

    #[test]
    fn gauge_group_clone() {
        let g = GaugeGroup::SU3C;
        let g2 = g.clone();
        assert!(format!("{:?}", g2).contains("SU3C"));
    }

    // --- Hypercharge ---

    #[test]
    fn hypercharge_electron() {
        // Electron: Q=-1 (3Q=-3), T3=-1/2 (2T3=-1), Y=-1 (3Y=-3)
        assert_eq!(derive_hypercharge(-3, -1), -3);
    }

    #[test]
    fn hypercharge_up_quark() {
        // Up quark: Q=+2/3 (3Q=+2), T3=+1/2 (2T3=+1), Y=+1/3 (3Y=+1)
        assert_eq!(derive_hypercharge(2, 1), 1);
    }

    #[test]
    fn hypercharge_down_quark() {
        // Down quark: Q=-1/3 (3Q=-1), T3=-1/2 (2T3=-1), Y=+1/3 (3Y=+1)
        assert_eq!(derive_hypercharge(-1, -1), 1);
    }

    #[test]
    fn hypercharge_neutrino() {
        // Neutrino: Q=0 (3Q=0), T3=+1/2 (2T3=+1), Y=-1 (3Y=-3)
        assert_eq!(derive_hypercharge(0, 1), -3);
    }

    // --- Conservation Laws ---

    #[test]
    fn conservation_valid_em_process() {
        // e- e+ -> gamma gamma (EM): all QNs should balance.
        // e-:  Q=-3, B=0, Le=1,  Lmu=0, Ltau=0, T3=-1, Y=-3
        // e+:  Q=+3, B=0, Le=-1, Lmu=0, Ltau=0, T3=+1, Y=+3
        // gamma: Q=0, B=0, Le=0, Lmu=0, Ltau=0, T3=0, Y=0
        let electron = make_state(-3, 0, 1, 0, 0, -1, -3);
        let positron = make_state(3, 0, -1, 0, 0, 1, 3);
        let photon1 = make_state(0, 0, 0, 0, 0, 0, 0);
        let photon2 = make_state(0, 0, 0, 0, 0, 0, 0);

        let result = validate_conservation_laws(
            &[electron, positron],
            &[photon1, photon2],
            InteractionType::Electromagnetic,
        )
        .unwrap();
        assert!(
            result.is_valid,
            "e+e- -> γγ should be valid: {:?}",
            result.violations
        );
    }

    #[test]
    fn conservation_charge_violated() {
        // e- -> gamma (impossible: charge not conserved)
        let electron = make_state(-3, 0, 1, 0, 0, -1, -3);
        let photon = make_state(0, 0, 0, 0, 0, 0, 0);

        let result =
            validate_conservation_laws(&[electron], &[photon], InteractionType::Electromagnetic)
                .unwrap();
        assert!(!result.is_valid);
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("Electric charge")));
    }

    #[test]
    fn conservation_lepton_number_violated() {
        // e- -> mu- (impossible: Le and Lmu violated)
        let electron = make_state(-3, 0, 1, 0, 0, -1, -3);
        let muon = make_state(-3, 0, 0, 1, 0, -1, -3);

        let result =
            validate_conservation_laws(&[electron], &[muon], InteractionType::Electromagnetic)
                .unwrap();
        assert!(!result.is_valid);
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("Electron lepton")));
        assert!(result.violations.iter().any(|v| v.contains("Muon lepton")));
    }

    #[test]
    fn conservation_weak_cc_ignores_t3() {
        // In weak CC, T3 is not conserved (W carries T3).
        // nu_e + d -> e- + u (simplified)
        // nu_e: Q=0, B=0, Le=1, T3=+1, Y=-3
        // d:    Q=-1, B=1, Le=0, T3=-1, Y=+1
        // e-:   Q=-3, B=0, Le=1, T3=-1, Y=-3
        // u:    Q=+2, B=1, Le=0, T3=+1, Y=+1
        let nu_e = make_state(0, 0, 1, 0, 0, 1, -3);
        let d = make_state(-1, 1, 0, 0, 0, -1, 1);
        let e = make_state(-3, 0, 1, 0, 0, -1, -3);
        let u = make_state(2, 1, 0, 0, 0, 1, 1);

        let result =
            validate_conservation_laws(&[nu_e, d], &[e, u], InteractionType::WeakCC).unwrap();
        // Charge: 0+(-1)=-1 vs -3+2=-1 ✓
        // B: 0+1=1 vs 0+1=1 ✓
        // Le: 1+0=1 vs 1+0=1 ✓
        // T3 not checked for WeakCC
        assert!(
            result.is_valid,
            "nu_e d -> e- u should be valid in CC: {:?}",
            result.violations
        );
    }

    // --- CPT ---

    #[test]
    fn cpt_em_all_conserved() {
        let result = check_cpt(&[], &[], InteractionType::Electromagnetic).unwrap();
        assert_eq!(result.c_parity_conserved, Some(true));
        assert_eq!(result.p_parity_conserved, Some(true));
        assert_eq!(result.t_symmetry_conserved, Some(true));
    }

    #[test]
    fn cpt_weak_violates_p_and_c() {
        let result = check_cpt(&[], &[], InteractionType::WeakCC).unwrap();
        assert_eq!(result.p_parity_conserved, Some(false));
        assert_eq!(result.c_parity_conserved, Some(false));
        assert!(!result.diagnostics.is_empty());
    }

    // --- Colour Singlet ---

    #[test]
    fn color_singlet_leptons() {
        // All singlets -> trivially singlet.
        let reps = vec![
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 1,
                label: "1".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 1,
                label: "1".into(),
            },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_quark_antiquark() {
        // 3 ⊗ 3̄ -> contains 1.
        let reps = vec![
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3bar".into(),
            },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_three_quarks() {
        // 3 ⊗ 3 ⊗ 3 -> contains 1 (baryon).
        let reps = vec![
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_non_singlet_two_quarks() {
        // 3 ⊗ 3 -> does NOT contain 1.
        let reps = vec![
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 3,
                label: "3".into(),
            },
        ];
        assert!(!validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_gluon_pair() {
        // 8 ⊗ 8 -> contains 1.
        let reps = vec![
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 8,
                label: "8".into(),
            },
            GaugeRepresentation {
                group: GaugeGroup::SU3C,
                dimension: 8,
                label: "8".into(),
            },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    // --- Poincaré Invariance ---

    #[test]
    fn poincare_ffv_vertex() {
        // Two spin-1/2 fermions + one spin-1 boson -> J=0 achievable.
        let reps = vec![
            PoincareRepresentation {
                mass: 0.000511,
                twice_spin: 1,
                is_massless: false,
            },
            PoincareRepresentation {
                mass: 0.000511,
                twice_spin: 1,
                is_massless: false,
            },
            PoincareRepresentation {
                mass: 0.0,
                twice_spin: 2,
                is_massless: true,
            },
        ];
        assert!(validate_poincare_invariance(&reps).unwrap());
    }

    #[test]
    fn poincare_ffs_vertex() {
        // Two spin-1/2 fermions + one spin-0 scalar -> J=0 achievable.
        let reps = vec![
            PoincareRepresentation {
                mass: 0.000511,
                twice_spin: 1,
                is_massless: false,
            },
            PoincareRepresentation {
                mass: 0.000511,
                twice_spin: 1,
                is_massless: false,
            },
            PoincareRepresentation {
                mass: 125.0,
                twice_spin: 0,
                is_massless: false,
            },
        ];
        assert!(validate_poincare_invariance(&reps).unwrap());
    }

    #[test]
    fn poincare_single_massive_vector_not_scalar() {
        // A single spin-1 particle alone cannot form a scalar.
        let reps = vec![PoincareRepresentation {
            mass: 91.2,
            twice_spin: 2,
            is_massless: false,
        }];
        assert!(!validate_poincare_invariance(&reps).unwrap());
    }

    // ===================================================================
    // Phase 14 - Generalized Lie Algebra Tests
    // ===================================================================

    // --- LieGroup basic properties ---

    #[test]
    fn lie_group_u1_rank() {
        let g = LieGroup::U1 { label: "Y".into() };
        assert_eq!(g.rank(), 1);
        assert_eq!(g.adjoint_dimension(), 1);
        assert_eq!(g.display_name(), "U(1)_Y");
    }

    #[test]
    fn lie_group_su2_rank() {
        let g = LieGroup::SU {
            n: 2,
            label: "L".into(),
        };
        assert_eq!(g.rank(), 1);
        assert_eq!(g.adjoint_dimension(), 3); // 2²-1
        assert_eq!(g.display_name(), "SU(2)_L");
    }

    #[test]
    fn lie_group_su3_rank() {
        let g = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        assert_eq!(g.rank(), 2);
        assert_eq!(g.adjoint_dimension(), 8); // 3²-1
    }

    #[test]
    fn lie_group_su5_rank() {
        let g = LieGroup::SU {
            n: 5,
            label: "GUT".into(),
        };
        assert_eq!(g.rank(), 4);
        assert_eq!(g.adjoint_dimension(), 24); // 5²-1
    }

    #[test]
    fn lie_group_so10_rank() {
        let g = LieGroup::SO {
            n: 10,
            label: "GUT".into(),
        };
        assert_eq!(g.rank(), 5); // ⌊10/2⌋
        assert_eq!(g.adjoint_dimension(), 45); // 10*9/2
    }

    #[test]
    fn lie_group_display_trait() {
        let g = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        assert_eq!(format!("{}", g), "SU(3)_C");
    }

    // --- N-ality computation ---

    #[test]
    fn su3_triality_fundamental() {
        // 3 of SU(3): Dynkin [1,0] → triality = 1*1 = 1 mod 3 = 1
        assert_eq!(compute_n_ality(3, &[1, 0]), 1);
    }

    #[test]
    fn su3_triality_antifundamental() {
        // 3̄ of SU(3): Dynkin [0,1] → triality = 2*1 = 2 mod 3 = 2
        assert_eq!(compute_n_ality(3, &[0, 1]), 2);
    }

    #[test]
    fn su3_triality_octet() {
        // 8 of SU(3): Dynkin [1,1] → triality = 1*1 + 2*1 = 3 mod 3 = 0
        assert_eq!(compute_n_ality(3, &[1, 1]), 0);
    }

    #[test]
    fn su3_triality_singlet() {
        // 1 of SU(3): Dynkin [0,0] → triality = 0
        assert_eq!(compute_n_ality(3, &[0, 0]), 0);
    }

    #[test]
    fn su2_duality_doublet() {
        // 2 of SU(2): Dynkin [1] → 2-ality = 1*1 = 1 mod 2 = 1
        assert_eq!(compute_n_ality(2, &[1]), 1);
    }

    #[test]
    fn su2_duality_triplet() {
        // 3 of SU(2): Dynkin [2] → 2-ality = 1*2 = 2 mod 2 = 0
        assert_eq!(compute_n_ality(2, &[2]), 0);
    }

    #[test]
    fn su5_five_ality_fundamental() {
        // 5 of SU(5): Dynkin [1,0,0,0] → 5-ality = 1*1 = 1 mod 5 = 1
        assert_eq!(compute_n_ality(5, &[1, 0, 0, 0]), 1);
    }

    #[test]
    fn su5_five_ality_antifundamental() {
        // 5̄ of SU(5): Dynkin [0,0,0,1] → 5-ality = 4*1 = 4 mod 5 = 4
        assert_eq!(compute_n_ality(5, &[0, 0, 0, 1]), 4);
    }

    #[test]
    fn su5_five_ality_10() {
        // 10 of SU(5): Dynkin [0,1,0,0] → 5-ality = 2*1 = 2 mod 5 = 2
        assert_eq!(compute_n_ality(5, &[0, 1, 0, 0]), 2);
    }

    #[test]
    fn su5_five_ality_adjoint() {
        // 24 of SU(5): Dynkin [1,0,0,1] → 5-ality = 1*1 + 4*1 = 5 mod 5 = 0
        assert_eq!(compute_n_ality(5, &[1, 0, 0, 1]), 0);
    }

    #[test]
    fn conjugate_n_ality_su3() {
        // If triality = 1, conjugate = 3-1 = 2
        assert_eq!(conjugate_n_ality(3, 1), 2);
        // If triality = 2, conjugate = 3-2 = 1
        assert_eq!(conjugate_n_ality(3, 2), 1);
        // If triality = 0, conjugate = 0
        assert_eq!(conjugate_n_ality(3, 0), 0);
    }

    #[test]
    fn conjugate_n_ality_su5() {
        // 5-ality 1 → conjugate = 4
        assert_eq!(conjugate_n_ality(5, 1), 4);
        // 5-ality 2 → conjugate = 3
        assert_eq!(conjugate_n_ality(5, 2), 3);
    }

    // --- GaugeSymmetry constructors ---

    #[test]
    fn gauge_symmetry_standard_model() {
        let sm = GaugeSymmetry::standard_model();
        assert_eq!(sm.groups.len(), 3);
        assert!(matches!(sm.groups[0], LieGroup::SU { n: 3, .. }));
        assert!(matches!(sm.groups[1], LieGroup::SU { n: 2, .. }));
        assert!(matches!(sm.groups[2], LieGroup::U1 { .. }));
        assert!(sm.label.contains("SU(3)"));
    }

    #[test]
    fn gauge_symmetry_su5_gut() {
        let su5 = GaugeSymmetry::su5_gut();
        assert_eq!(su5.groups.len(), 1);
        assert!(matches!(su5.groups[0], LieGroup::SU { n: 5, .. }));
    }

    #[test]
    fn gauge_symmetry_so10_gut() {
        let so10 = GaugeSymmetry::so10_gut();
        assert_eq!(so10.groups.len(), 1);
        assert!(matches!(so10.groups[0], LieGroup::SO { n: 10, .. }));
    }

    #[test]
    fn gauge_symmetry_dark_u1() {
        let dark = GaugeSymmetry::dark_u1();
        assert_eq!(dark.groups.len(), 1);
        assert!(matches!(dark.groups[0], LieGroup::U1 { .. }));
    }

    // --- validate_gauge_invariance ---

    #[test]
    fn u1_charge_conservation_valid() {
        let u1 = LieGroup::U1 { label: "EM".into() };
        let reps = vec![
            LieGroupRepresentation {
                group: u1.clone(),
                dimension: 1,
                charge: Some(-1.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "-1".into(),
            },
            LieGroupRepresentation {
                group: u1.clone(),
                dimension: 1,
                charge: Some(1.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "+1".into(),
            },
        ];
        assert!(validate_gauge_invariance(&u1, &reps).unwrap());
    }

    #[test]
    fn u1_charge_conservation_violated() {
        let u1 = LieGroup::U1 { label: "EM".into() };
        let reps = vec![
            LieGroupRepresentation {
                group: u1.clone(),
                dimension: 1,
                charge: Some(-1.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "-1".into(),
            },
            LieGroupRepresentation {
                group: u1.clone(),
                dimension: 1,
                charge: Some(-1.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "-1".into(),
            },
        ];
        assert!(!validate_gauge_invariance(&u1, &reps).unwrap());
    }

    #[test]
    fn su3_singlet_vertex_valid() {
        // Quark (3) + antiquark (3̄) → octet (or singlet), triality: 1+2=3≡0 mod 3
        let su3 = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: true,
                dynkin_labels: vec![0, 1],
                label: "3̄".into(),
            },
        ];
        assert!(validate_gauge_invariance(&su3, &reps).unwrap());
    }

    #[test]
    fn su3_non_singlet_vertex_invalid() {
        // Two quarks (3+3), triality: 1+1=2 mod 3 ≠ 0
        let su3 = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
        ];
        assert!(!validate_gauge_invariance(&su3, &reps).unwrap());
    }

    #[test]
    fn su3_three_quarks_valid_baryon() {
        // Three quarks (3+3+3), triality: 1+1+1=3≡0 mod 3 → colour singlet (baryon)
        let su3 = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
        ];
        assert!(validate_gauge_invariance(&su3, &reps).unwrap());
    }

    #[test]
    fn su5_5_and_5bar_valid() {
        // 5 + 5̄ of SU(5): 5-ality 1+4=5≡0 mod 5 → singlet
        let su5 = LieGroup::SU {
            n: 5,
            label: "GUT".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: su5.clone(),
                dimension: 5,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0, 0, 0],
                label: "5".into(),
            },
            LieGroupRepresentation {
                group: su5.clone(),
                dimension: 5,
                charge: None,
                is_conjugate: true,
                dynkin_labels: vec![0, 0, 0, 1],
                label: "5̄".into(),
            },
        ];
        assert!(validate_gauge_invariance(&su5, &reps).unwrap());
    }

    #[test]
    fn su5_10_and_5bar_and_5bar_valid() {
        // 10 + 5̄ + 5̄ of SU(5): 5-ality 2+4+4=10≡0 mod 5 → valid
        let su5 = LieGroup::SU {
            n: 5,
            label: "GUT".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: su5.clone(),
                dimension: 10,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0, 1, 0, 0],
                label: "10".into(),
            },
            LieGroupRepresentation {
                group: su5.clone(),
                dimension: 5,
                charge: None,
                is_conjugate: true,
                dynkin_labels: vec![0, 0, 0, 1],
                label: "5̄".into(),
            },
            LieGroupRepresentation {
                group: su5.clone(),
                dimension: 5,
                charge: None,
                is_conjugate: true,
                dynkin_labels: vec![0, 0, 0, 1],
                label: "5̄".into(),
            },
        ];
        assert!(validate_gauge_invariance(&su5, &reps).unwrap());
    }

    #[test]
    fn so10_even_spinors_valid() {
        // Two spinor 16's of SO(10): even spinorial parity → valid
        let so10 = LieGroup::SO {
            n: 10,
            label: "GUT".into(),
        };
        let reps = vec![
            LieGroupRepresentation {
                group: so10.clone(),
                dimension: 16,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "16".into(),
            },
            LieGroupRepresentation {
                group: so10.clone(),
                dimension: 16,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "16".into(),
            },
        ];
        assert!(validate_gauge_invariance(&so10, &reps).unwrap());
    }

    #[test]
    fn so10_odd_spinor_invalid() {
        // Single spinor 16 of SO(10): odd spinorial parity → invalid
        let so10 = LieGroup::SO {
            n: 10,
            label: "GUT".into(),
        };
        let reps = vec![LieGroupRepresentation {
            group: so10.clone(),
            dimension: 16,
            charge: None,
            is_conjugate: false,
            dynkin_labels: vec![],
            label: "16".into(),
        }];
        assert!(!validate_gauge_invariance(&so10, &reps).unwrap());
    }

    // --- validate_generalized_conservation ---

    #[test]
    fn generalized_conservation_sm_qqbar_to_ll() {
        // q + q̄ → l + l̄ under SM gauge symmetry
        // SU(3)_C: 3+3̄=0 vs 1+1=0 ✓
        // SU(2)_L: all singlet ✓
        // U(1)_Y: charges must balance
        let sm = GaugeSymmetry::standard_model();
        let su3 = LieGroup::SU {
            n: 3,
            label: "C".into(),
        };
        let su2 = LieGroup::SU {
            n: 2,
            label: "L".into(),
        };
        let u1y = LieGroup::U1 { label: "Y".into() };

        let initial = vec![
            // Quark: 3 under SU(3), 1 under SU(2), Y=1/6
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1, 0],
                label: "3".into(),
            },
            LieGroupRepresentation {
                group: su2.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: u1y.clone(),
                dimension: 1,
                charge: Some(1.0 / 6.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "1/6".into(),
            },
            // Antiquark: 3̄ under SU(3), 1 under SU(2), Y=-1/6
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 3,
                charge: None,
                is_conjugate: true,
                dynkin_labels: vec![0, 1],
                label: "3̄".into(),
            },
            LieGroupRepresentation {
                group: su2.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: u1y.clone(),
                dimension: 1,
                charge: Some(-1.0 / 6.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "-1/6".into(),
            },
        ];
        let final_state = vec![
            // Lepton: 1 under SU(3), 1 under SU(2), Y=-1/2
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0, 0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: su2.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: u1y.clone(),
                dimension: 1,
                charge: Some(-1.0 / 2.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "-1/2".into(),
            },
            // Antilepton: 1 under SU(3), 1 under SU(2), Y=+1/2
            LieGroupRepresentation {
                group: su3.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0, 0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: su2.clone(),
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0],
                label: "1".into(),
            },
            LieGroupRepresentation {
                group: u1y.clone(),
                dimension: 1,
                charge: Some(1.0 / 2.0),
                is_conjugate: false,
                dynkin_labels: vec![],
                label: "1/2".into(),
            },
        ];

        let result = validate_generalized_conservation(&sm, &initial, &final_state).unwrap();
        // SU(3) and SU(2) are satisfied; U(1)_Y: initial 0, final 0 ✓
        assert!(result.is_valid, "Violations: {:?}", result.violations);
    }

    #[test]
    fn generalized_conservation_u1_violation() {
        let dark = GaugeSymmetry::dark_u1();
        let u1 = LieGroup::U1 { label: "'".into() };
        let initial = vec![LieGroupRepresentation {
            group: u1.clone(),
            dimension: 1,
            charge: Some(1.0),
            is_conjugate: false,
            dynkin_labels: vec![],
            label: "1".into(),
        }];
        let final_state = vec![LieGroupRepresentation {
            group: u1.clone(),
            dimension: 1,
            charge: Some(2.0),
            is_conjugate: false,
            dynkin_labels: vec![],
            label: "2".into(),
        }];
        let result = validate_generalized_conservation(&dark, &initial, &final_state).unwrap();
        assert!(!result.is_valid);
        assert_eq!(result.violations.len(), 1);
        assert!(result.violations[0].contains("U(1)"));
    }

    // --- LieGroupRepresentation serde ---

    #[test]
    fn lie_group_representation_serde_roundtrip() {
        let rep = LieGroupRepresentation {
            group: LieGroup::SU {
                n: 3,
                label: "C".into(),
            },
            dimension: 3,
            charge: None,
            is_conjugate: false,
            dynkin_labels: vec![1, 0],
            label: "3".into(),
        };
        let json = serde_json::to_string(&rep).unwrap();
        let rep2: LieGroupRepresentation = serde_json::from_str(&json).unwrap();
        assert_eq!(rep, rep2);
    }

    #[test]
    fn gauge_symmetry_serde_roundtrip() {
        let sm = GaugeSymmetry::standard_model();
        let json = serde_json::to_string(&sm).unwrap();
        let sm2: GaugeSymmetry = serde_json::from_str(&json).unwrap();
        assert_eq!(sm, sm2);
    }

    #[test]
    fn lie_group_serde_roundtrip() {
        let groups = vec![
            LieGroup::U1 { label: "Y".into() },
            LieGroup::SU {
                n: 5,
                label: "GUT".into(),
            },
            LieGroup::SO {
                n: 10,
                label: "GUT".into(),
            },
        ];
        for g in &groups {
            let json = serde_json::to_string(g).unwrap();
            let g2: LieGroup = serde_json::from_str(&json).unwrap();
            assert_eq!(g, &g2);
        }
    }

    #[test]
    fn empty_representations_trivially_valid() {
        let u1 = LieGroup::U1 {
            label: "test".into(),
        };
        assert!(validate_gauge_invariance(&u1, &[]).unwrap());
    }
}
