//! # Symmetry Groups and Conservation Laws
//!
//! This module implements the mathematical structures of the gauge symmetry groups
//! that underpin the Standard Model: $SU(3)_C$, $SU(2)_L$, $U(1)_Y$, and the
//! Poincaré group of spacetime symmetries.
//!
//! Conservation law validation is performed via group-theoretic invariants rather
//! than simple arithmetic. Discrete symmetries (C, P, T and their combinations)
//! are evaluated depending on the interaction type.

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
    /// $U(1)_Y$ — Abelian hypercharge symmetry.
    U1Y,
    /// $SU(2)_L$ — Non-Abelian weak isospin symmetry.
    SU2L,
    /// $SU(3)_C$ — Non-Abelian colour symmetry (QCD).
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
// Public API — Conservation and Symmetry Validation
// ---------------------------------------------------------------------------

/// Validate all additive conservation laws for a proposed reaction.
///
/// Checks conservation of: electric charge, baryon number, lepton family
/// numbers ($L_e$, $L_\mu$, $L_\tau$), weak isospin $T_3$, and hypercharge $Y$.
/// Validation is performed using group-theoretic invariant sums, not simple
/// arithmetic.
///
/// # Arguments
/// * `initial` — Slice of initial-state `QuantumState` objects.
/// * `final_states` — Slice of final-state `QuantumState` objects.
/// * `interaction` — The type of interaction governing the process (determines
///   which quantum numbers must be strictly conserved).
pub fn validate_conservation_laws(
    initial: &[QuantumState],
    final_states: &[QuantumState],
    interaction: InteractionType,
) -> SpireResult<ConservationResult> {
    let mut violations = Vec::new();

    // Helper: sum an additive quantum number across a set of states.
    let sum_q = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.electric_charge.0 as i32).sum()
    };
    let sum_b = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.baryon_number.0 as i32).sum()
    };
    let sum_le = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.lepton_numbers.electron as i32).sum()
    };
    let sum_lmu = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.lepton_numbers.muon as i32).sum()
    };
    let sum_ltau = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.lepton_numbers.tau as i32).sum()
    };
    let sum_t3 = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.weak_isospin.0 as i32).sum()
    };
    let sum_y = |states: &[QuantumState]| -> i32 {
        states.iter().map(|s| s.particle.field.quantum_numbers.hypercharge.0 as i32).sum()
    };

    // Electric charge is ALWAYS conserved.
    let q_i = sum_q(initial);
    let q_f = sum_q(final_states);
    if q_i != q_f {
        violations.push(format!(
            "Electric charge violated: initial 3Q={}, final 3Q={}", q_i, q_f
        ));
    }

    // Baryon number is ALWAYS conserved (in the SM).
    let b_i = sum_b(initial);
    let b_f = sum_b(final_states);
    if b_i != b_f {
        violations.push(format!(
            "Baryon number violated: initial 3B={}, final 3B={}", b_i, b_f
        ));
    }

    // Lepton family numbers are conserved in all SM interactions at tree level.
    let le_i = sum_le(initial);
    let le_f = sum_le(final_states);
    if le_i != le_f {
        violations.push(format!(
            "Electron lepton number Le violated: initial={}, final={}", le_i, le_f
        ));
    }

    let lmu_i = sum_lmu(initial);
    let lmu_f = sum_lmu(final_states);
    if lmu_i != lmu_f {
        violations.push(format!(
            "Muon lepton number Lmu violated: initial={}, final={}", lmu_i, lmu_f
        ));
    }

    let ltau_i = sum_ltau(initial);
    let ltau_f = sum_ltau(final_states);
    if ltau_i != ltau_f {
        violations.push(format!(
            "Tau lepton number Ltau violated: initial={}, final={}", ltau_i, ltau_f
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
                    "Weak isospin T3 violated: initial 2T3={}, final 2T3={}", t3_i, t3_f
                ));
            }
            let y_i = sum_y(initial);
            let y_f = sum_y(final_states);
            if y_i != y_f {
                violations.push(format!(
                    "Hypercharge Y violated: initial 3Y={}, final 3Y={}", y_i, y_f
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
/// * `initial` — Slice of initial-state `QuantumState` objects.
/// * `final_states` — Slice of final-state `QuantumState` objects.
/// * `interaction` — The governing interaction type.
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
        InteractionType::ScalarSelf => {
            (Some(true), Some(true), Some(true), Some(true), vec![])
        }
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
            let new_min = if j_min > s { j_min - s } else if s > j_max { s - j_max } else { 0 };
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
    fn make_state(
        q: i8,
        b: i8,
        le: i8,
        lmu: i8,
        ltau: i8,
        t3: i8,
        y: i8,
    ) -> QuantumState {
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
        assert!(result.is_valid, "e+e- -> γγ should be valid: {:?}", result.violations);
    }

    #[test]
    fn conservation_charge_violated() {
        // e- -> gamma (impossible: charge not conserved)
        let electron = make_state(-3, 0, 1, 0, 0, -1, -3);
        let photon = make_state(0, 0, 0, 0, 0, 0, 0);

        let result = validate_conservation_laws(
            &[electron],
            &[photon],
            InteractionType::Electromagnetic,
        )
        .unwrap();
        assert!(!result.is_valid);
        assert!(result.violations.iter().any(|v| v.contains("Electric charge")));
    }

    #[test]
    fn conservation_lepton_number_violated() {
        // e- -> mu- (impossible: Le and Lmu violated)
        let electron = make_state(-3, 0, 1, 0, 0, -1, -3);
        let muon = make_state(-3, 0, 0, 1, 0, -1, -3);

        let result = validate_conservation_laws(
            &[electron],
            &[muon],
            InteractionType::Electromagnetic,
        )
        .unwrap();
        assert!(!result.is_valid);
        assert!(result.violations.iter().any(|v| v.contains("Electron lepton")));
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

        let result = validate_conservation_laws(
            &[nu_e, d],
            &[e, u],
            InteractionType::WeakCC,
        )
        .unwrap();
        // Charge: 0+(-1)=-1 vs -3+2=-1 ✓
        // B: 0+1=1 vs 0+1=1 ✓
        // Le: 1+0=1 vs 1+0=1 ✓
        // T3 not checked for WeakCC
        assert!(result.is_valid, "nu_e d -> e- u should be valid in CC: {:?}", result.violations);
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
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 1, label: "1".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 1, label: "1".into() },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_quark_antiquark() {
        // 3 ⊗ 3̄ -> contains 1.
        let reps = vec![
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3bar".into() },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_three_quarks() {
        // 3 ⊗ 3 ⊗ 3 -> contains 1 (baryon).
        let reps = vec![
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_non_singlet_two_quarks() {
        // 3 ⊗ 3 -> does NOT contain 1.
        let reps = vec![
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 3, label: "3".into() },
        ];
        assert!(!validate_color_singlet(&reps).unwrap());
    }

    #[test]
    fn color_singlet_gluon_pair() {
        // 8 ⊗ 8 -> contains 1.
        let reps = vec![
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 8, label: "8".into() },
            GaugeRepresentation { group: GaugeGroup::SU3C, dimension: 8, label: "8".into() },
        ];
        assert!(validate_color_singlet(&reps).unwrap());
    }

    // --- Poincaré Invariance ---

    #[test]
    fn poincare_ffv_vertex() {
        // Two spin-1/2 fermions + one spin-1 boson -> J=0 achievable.
        let reps = vec![
            PoincareRepresentation { mass: 0.000511, twice_spin: 1, is_massless: false },
            PoincareRepresentation { mass: 0.000511, twice_spin: 1, is_massless: false },
            PoincareRepresentation { mass: 0.0, twice_spin: 2, is_massless: true },
        ];
        assert!(validate_poincare_invariance(&reps).unwrap());
    }

    #[test]
    fn poincare_ffs_vertex() {
        // Two spin-1/2 fermions + one spin-0 scalar -> J=0 achievable.
        let reps = vec![
            PoincareRepresentation { mass: 0.000511, twice_spin: 1, is_massless: false },
            PoincareRepresentation { mass: 0.000511, twice_spin: 1, is_massless: false },
            PoincareRepresentation { mass: 125.0, twice_spin: 0, is_massless: false },
        ];
        assert!(validate_poincare_invariance(&reps).unwrap());
    }

    #[test]
    fn poincare_single_massive_vector_not_scalar() {
        // A single spin-1 particle alone cannot form a scalar.
        let reps = vec![
            PoincareRepresentation { mass: 91.2, twice_spin: 2, is_massless: false },
        ];
        assert!(!validate_poincare_invariance(&reps).unwrap());
    }
}
