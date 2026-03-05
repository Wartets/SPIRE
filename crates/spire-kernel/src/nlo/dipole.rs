//! # Dipole Subtraction — Catani–Seymour Formalism
//!
//! Implements the **final-final (FF)** Catani–Seymour dipole subtraction
//! for NLO QCD calculations, as described in:
//!
//! > S. Catani and M.H. Seymour, *"A General Algorithm for Calculating
//! > Jet Cross Sections in NLO QCD"*, Nucl. Phys. B **485** (1997) 291,
//! > [hep-ph/9605323](https://arxiv.org/abs/hep-ph/9605323).
//!
//! ## Kinematic Mapping (Final-Final)
//!
//! For an emitter $i$, emitted parton $j$, and spectator $k$, the
//! Catani–Seymour FF mapping produces reduced momenta:
//!
//! $$y_{ij,k} = \frac{p_i \cdot p_j}{p_i \cdot p_j + p_j \cdot p_k + p_i \cdot p_k}$$
//!
//! $$z_i = \frac{p_i \cdot p_k}{(p_i + p_j) \cdot p_k}$$
//!
//! $$\tilde{p}_{ij} = p_i + p_j - \frac{y_{ij,k}}{1 - y_{ij,k}} p_k$$
//!
//! $$\tilde{p}_k = \frac{1}{1 - y_{ij,k}} p_k$$
//!
//! ## Splitting Functions
//!
//! The universal spin-averaged splitting kernels:
//!
//! - $\langle V_{q \to qg} \rangle = C_F \left[ \frac{2}{1-z(1-y)} - (1+z) \right]$
//! - $\langle V_{g \to gg} \rangle = 2 C_A \left[ \frac{1}{1-z(1-y)} + \frac{1}{1-(1-z)(1-y)} - 2 + z(1-z) \right]$
//! - $\langle V_{g \to q\bar{q}} \rangle = T_R \left[ 1 - 2z(1-z) \right]$
//!
//! where the QCD colour factors are $C_F = 4/3$, $C_A = 3$, $T_R = 1/2$.

use serde::{Deserialize, Serialize};

use crate::algebra::FourMomentum;
use crate::{SpireError, SpireResult};

// ===========================================================================
// QCD Colour Factors
// ===========================================================================

/// Quadratic Casimir of the fundamental representation: $C_F = (N_c^2 - 1)/(2 N_c) = 4/3$.
pub const CF: f64 = 4.0 / 3.0;

/// Quadratic Casimir of the adjoint representation: $C_A = N_c = 3$.
pub const CA: f64 = 3.0;

/// Dynkin index of the fundamental: $T_R = 1/2$.
pub const TR: f64 = 0.5;

// ===========================================================================
// Splitting Function Identifiers
// ===========================================================================

/// Identifies the type of parton splitting in a dipole.
///
/// Each variant corresponds to a distinct universal splitting kernel
/// in the Catani–Seymour formalism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplittingType {
    /// Quark emitting a gluon: $q \to q + g$.
    QuarkToQuarkGluon,

    /// Gluon splitting into two gluons: $g \to g + g$.
    GluonToGluonGluon,

    /// Gluon splitting into a quark-antiquark pair: $g \to q + \bar{q}$.
    GluonToQuarkAntiquark,
}

// ===========================================================================
// Dipole Kinematics
// ===========================================================================

/// Result of the Catani–Seymour final-final kinematic mapping.
///
/// Given the real-emission momenta $(p_i, p_j, p_k)$ — emitter, emitted,
/// spectator — this struct holds the mapped Born-level momenta
/// $(\tilde{p}_{ij}, \tilde{p}_k)$ and the kinematic invariants $(y, z)$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipoleKinematics {
    /// The reduced emitter momentum $\tilde{p}_{ij}$.
    pub p_tilde_ij: FourMomentum,

    /// The rescaled spectator momentum $\tilde{p}_k$.
    pub p_tilde_k: FourMomentum,

    /// The energy-sharing variable:
    /// $y_{ij,k} = (p_i \cdot p_j) / (p_i \cdot p_j + p_j \cdot p_k + p_i \cdot p_k)$
    pub y: f64,

    /// The splitting variable:
    /// $z_i = (p_i \cdot p_k) / ((p_i + p_j) \cdot p_k)$
    pub z: f64,
}

/// Compute the **final-final Catani–Seymour kinematic mapping**.
///
/// # Arguments
///
/// * `p_i` — Four-momentum of the emitter parton.
/// * `p_j` — Four-momentum of the emitted (unresolved) parton.
/// * `p_k` — Four-momentum of the spectator parton.
///
/// # Returns
///
/// A `DipoleKinematics` struct containing the mapped momenta and invariants.
///
/// # Errors
///
/// Returns `SpireError::KinematicsForbidden` if the denominator vanishes
/// (degenerate three-body configuration).
///
/// # Physics
///
/// The mapping preserves total four-momentum:
/// $$\tilde{p}_{ij} + \tilde{p}_k = p_i + p_j + p_k$$
///
/// and puts the reduced momenta on shell for massless partons:
/// $$\tilde{p}_{ij}^2 = 0, \quad \tilde{p}_k^2 = 0$$
pub fn catani_seymour_ff_map(
    p_i: &FourMomentum,
    p_j: &FourMomentum,
    p_k: &FourMomentum,
) -> SpireResult<DipoleKinematics> {
    let pi_pj = p_i.dot(p_j);
    let pj_pk = p_j.dot(p_k);
    let pi_pk = p_i.dot(p_k);

    let denom = pi_pj + pj_pk + pi_pk;
    if denom.abs() < 1e-30 {
        return Err(SpireError::KinematicsForbidden(
            "Degenerate dipole configuration: (p_i·p_j + p_j·p_k + p_i·p_k) ≈ 0".into(),
        ));
    }

    let y = pi_pj / denom;

    // z_i = (p_i · p_k) / ((p_i + p_j) · p_k)
    let pi_plus_pj_dot_pk = pi_pk + pj_pk;
    if pi_plus_pj_dot_pk.abs() < 1e-30 {
        return Err(SpireError::KinematicsForbidden(
            "Degenerate dipole: (p_i + p_j)·p_k ≈ 0".into(),
        ));
    }
    let z = pi_pk / pi_plus_pj_dot_pk;

    // Guard against y == 1 (division by zero in mapping)
    if (1.0 - y).abs() < 1e-30 {
        return Err(SpireError::KinematicsForbidden(
            "Dipole invariant y = 1: fully collinear limit".into(),
        ));
    }

    let y_over_1my = y / (1.0 - y);

    // p̃_{ij} = p_i + p_j - y/(1-y) * p_k
    let p_tilde_ij = *p_i + *p_j - p_k.scale(y_over_1my);

    // p̃_k = 1/(1-y) * p_k
    let p_tilde_k = p_k.scale(1.0 / (1.0 - y));

    Ok(DipoleKinematics {
        p_tilde_ij,
        p_tilde_k,
        y,
        z,
    })
}

// ===========================================================================
// Splitting Functions
// ===========================================================================

/// Evaluate the spin-averaged Catani–Seymour splitting function for the
/// specified splitting type.
///
/// # Arguments
///
/// * `splitting` — Which parton splitting to evaluate.
/// * `z` — The splitting variable from the dipole mapping.
/// * `y` — The energy-sharing variable from the dipole mapping.
///
/// # Returns
///
/// The value of the splitting kernel $\langle V \rangle / (p_i \cdot p_j)$
/// (the denominator is *not* included — the caller multiplies by
/// $1 / (2 \, p_i \cdot p_j)$ when constructing the full dipole).
///
/// # Splitting Kernels
///
/// - **Quark → quark + gluon:**
///   $\langle V_{q \to qg} \rangle = C_F \left[ \frac{2}{1 - z(1 - y)} - (1 + z) \right]$
///
/// - **Gluon → gluon + gluon:**
///   $\langle V_{g \to gg} \rangle = 2 C_A \left[ \frac{1}{1 - z(1 - y)}
///     + \frac{1}{1 - (1-z)(1-y)} - 2 + z(1-z) \right]$
///
/// - **Gluon → quark + antiquark:**
///   $\langle V_{g \to q\bar{q}} \rangle = T_R \left[ 1 - 2z(1-z) \right]$
pub fn splitting_function(splitting: SplittingType, z: f64, y: f64) -> f64 {
    match splitting {
        SplittingType::QuarkToQuarkGluon => {
            // V_{q→qg} = C_F [ 2 / (1 - z(1-y)) - (1+z) ]
            let denom = 1.0 - z * (1.0 - y);
            CF * (2.0 / denom - (1.0 + z))
        }
        SplittingType::GluonToGluonGluon => {
            // V_{g→gg} = 2 C_A [ 1/(1 - z(1-y)) + 1/(1 - (1-z)(1-y)) - 2 + z(1-z) ]
            let denom1 = 1.0 - z * (1.0 - y);
            let denom2 = 1.0 - (1.0 - z) * (1.0 - y);
            2.0 * CA * (1.0 / denom1 + 1.0 / denom2 - 2.0 + z * (1.0 - z))
        }
        SplittingType::GluonToQuarkAntiquark => {
            // V_{g→qq̄} = T_R [ 1 - 2z(1-z) ]
            TR * (1.0 - 2.0 * z * (1.0 - z))
        }
    }
}

// ===========================================================================
// Dipole Contribution
// ===========================================================================

/// A single dipole subtraction term for a real-emission process.
///
/// Combines the kinematic mapping with the splitting function evaluation
/// and the colour factor to form a complete subtraction term:
///
/// $$\mathcal{D}_{ij,k} = -\frac{1}{2 \, p_i \cdot p_j}
///     \langle V_{ij,k}(z, y) \rangle \otimes |\mathcal{M}_{\text{Born}}(\tilde{p})|^2$$
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipoleContribution {
    /// The splitting type of this dipole.
    pub splitting: SplittingType,

    /// Kinematic invariants and mapped momenta.
    pub kinematics: DipoleKinematics,

    /// The dot product $p_i \cdot p_j$ (dipole invariant mass over 2).
    pub pi_dot_pj: f64,

    /// The splitting function value $\langle V \rangle$.
    pub splitting_value: f64,

    /// The full dipole prefactor: $-\langle V \rangle / (2 \, p_i \cdot p_j)$.
    pub dipole_factor: f64,
}

/// Build a complete dipole contribution from the three parton momenta
/// and the splitting type.
///
/// # Arguments
///
/// * `p_i` — Emitter four-momentum.
/// * `p_j` — Emitted (unresolved) parton four-momentum.
/// * `p_k` — Spectator four-momentum.
/// * `splitting` — Which splitting kernel to apply.
///
/// # Returns
///
/// A `DipoleContribution` ready to be multiplied by the Born matrix element
/// evaluated at the mapped momenta.
pub fn build_dipole(
    p_i: &FourMomentum,
    p_j: &FourMomentum,
    p_k: &FourMomentum,
    splitting: SplittingType,
) -> SpireResult<DipoleContribution> {
    let kinematics = catani_seymour_ff_map(p_i, p_j, p_k)?;

    let pi_dot_pj = p_i.dot(p_j);

    let splitting_value = splitting_function(splitting, kinematics.z, kinematics.y);

    // The dipole factor absorbs the 1/(2 p_i·p_j) propagator and sign:
    //   D = -V / (2 * p_i·p_j)
    let dipole_factor = if pi_dot_pj.abs() < 1e-30 {
        0.0 // Exact soft limit — dipole diverges; regulated by slicing or cutoff
    } else {
        -splitting_value / (2.0 * pi_dot_pj)
    };

    Ok(DipoleContribution {
        splitting,
        kinematics,
        pi_dot_pj,
        splitting_value,
        dipole_factor,
    })
}

// ===========================================================================
// Subtraction Scheme Trait
// ===========================================================================

/// Configuration for NLO subtraction parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtractionConfig {
    /// Technical cut on the dipole invariant $y$ to regularise the
    /// soft/collinear limit.  Dipoles with $y < y_{\min}$ are discarded.
    /// Typical value: $10^{-7}$.
    pub y_min: f64,

    /// Upper bound on $y$.  Dipoles with $y > y_{\max}$ are discarded.
    /// Used in phase-space slicing variants.  Set to 1.0 to disable.
    pub y_max: f64,

    /// Alpha parameter for the $\alpha$-dipole restriction
    /// (Catani–Seymour $\alpha$ parameter).  Set to 1.0 for the
    /// unrestricted scheme.
    pub alpha: f64,
}

impl Default for SubtractionConfig {
    fn default() -> Self {
        Self {
            y_min: 1e-7,
            y_max: 1.0,
            alpha: 1.0,
        }
    }
}

/// A collection of dipole contributions forming the full subtraction
/// term for one real-emission phase-space point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtractionResult {
    /// All individual dipole contributions.
    pub dipoles: Vec<DipoleContribution>,

    /// Sum of all dipole factors: $\sum_D \mathcal{D}$.
    /// (To be multiplied by the Born matrix element at each dipole's
    /// mapped kinematics.)
    pub total_subtraction: f64,
}

/// Trait for NLO subtraction scheme implementations.
///
/// A subtraction scheme constructs the set of dipole subtraction terms
/// for a given real-emission phase-space configuration.  Different
/// implementations can provide Catani–Seymour, FKS, or antenna subtraction.
///
/// # Type Parameters
///
/// The trait is object-safe to allow runtime selection of the scheme.
pub trait SubtractionScheme: Send + Sync {
    /// Compute all dipole subtraction terms for a given set of final-state
    /// momenta and their parton identities.
    ///
    /// # Arguments
    ///
    /// * `momenta` — The $(m+1)$-body final-state four-momenta.
    /// * `parton_ids` — PDG-like identifiers for each parton (quark = 1..6,
    ///   antiquark = -1..-6, gluon = 21).
    /// * `config` — Subtraction parameters (cuts, alpha restriction).
    ///
    /// # Returns
    ///
    /// A `SubtractionResult` containing all non-vanishing dipole terms.
    fn compute_subtraction(
        &self,
        momenta: &[FourMomentum],
        parton_ids: &[i32],
        config: &SubtractionConfig,
    ) -> SpireResult<SubtractionResult>;

    /// Human-readable name of the subtraction scheme.
    fn scheme_name(&self) -> &str;
}

// ===========================================================================
// Catani–Seymour Implementation
// ===========================================================================

/// The standard Catani–Seymour dipole subtraction scheme for massless
/// final-state partons.
///
/// For each valid emitter–emitted–spectator triple in the final state,
/// constructs the corresponding dipole term using the appropriate
/// splitting function.
#[derive(Debug, Clone, Default)]
pub struct CataniSeymourScheme;

impl SubtractionScheme for CataniSeymourScheme {
    fn scheme_name(&self) -> &str {
        "Catani-Seymour (massless FF)"
    }

    fn compute_subtraction(
        &self,
        momenta: &[FourMomentum],
        parton_ids: &[i32],
        config: &SubtractionConfig,
    ) -> SpireResult<SubtractionResult> {
        let n = momenta.len();
        if n < 3 {
            return Err(SpireError::KinematicsForbidden(
                "Need at least 3 final-state partons for dipole subtraction".into(),
            ));
        }
        if parton_ids.len() != n {
            return Err(SpireError::InternalError(
                "momenta and parton_ids must have the same length".into(),
            ));
        }

        let mut dipoles = Vec::new();
        let mut total = 0.0;

        // Iterate over all emitter-emitted-spectator triples
        for i in 0..n {
            for j in 0..n {
                if j == i {
                    continue;
                }
                for k in 0..n {
                    if k == i || k == j {
                        continue;
                    }

                    // Determine the splitting type from parton identities
                    let splitting = match identify_splitting(parton_ids[i], parton_ids[j]) {
                        Some(s) => s,
                        None => continue,
                    };

                    // Apply the kinematic mapping
                    let kin = match catani_seymour_ff_map(&momenta[i], &momenta[j], &momenta[k]) {
                        Ok(k) => k,
                        Err(_) => continue, // Skip degenerate configurations
                    };

                    // Apply y-cut from config
                    if kin.y < config.y_min || kin.y > config.y_max {
                        continue;
                    }

                    // Apply alpha restriction
                    if config.alpha < 1.0 && kin.y > config.alpha {
                        continue;
                    }

                    let pi_dot_pj = momenta[i].dot(&momenta[j]);
                    let splitting_value = splitting_function(splitting, kin.z, kin.y);

                    let dipole_factor = if pi_dot_pj.abs() < 1e-30 {
                        0.0
                    } else {
                        -splitting_value / (2.0 * pi_dot_pj)
                    };

                    total += dipole_factor;

                    dipoles.push(DipoleContribution {
                        splitting,
                        kinematics: kin,
                        pi_dot_pj,
                        splitting_value,
                        dipole_factor,
                    });
                }
            }
        }

        Ok(SubtractionResult {
            dipoles,
            total_subtraction: total,
        })
    }
}

// ===========================================================================
// Parton Identity Helpers
// ===========================================================================

/// Determine the splitting type from PDG IDs of emitter and emitted parton.
///
/// Conventions:
/// - Quarks: 1..6  (d, u, s, c, b, t)
/// - Antiquarks: -1..-6
/// - Gluon: 21
fn identify_splitting(emitter_id: i32, emitted_id: i32) -> Option<SplittingType> {
    let is_quark = |id: i32| (1..=6).contains(&id.abs());
    let is_gluon = |id: i32| id == 21;

    if is_quark(emitter_id) && is_gluon(emitted_id) {
        // q → q + g
        Some(SplittingType::QuarkToQuarkGluon)
    } else if is_gluon(emitter_id) && is_gluon(emitted_id) {
        // g → g + g
        Some(SplittingType::GluonToGluonGluon)
    } else if is_gluon(emitter_id) && is_quark(emitted_id) {
        // g → q + q̄  (emitted is the quark or antiquark)
        Some(SplittingType::GluonToQuarkAntiquark)
    } else {
        None
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Kinematic mapping tests ─────────────────────────────────────────

    #[test]
    fn ff_map_preserves_total_momentum() {
        let make_lightlike = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let p_i = make_lightlike(3.0, 4.0, 5.0);
        let p_j = make_lightlike(-2.0, 1.0, 3.0);
        let p_k = make_lightlike(1.0, -3.0, 7.0);

        let result = catani_seymour_ff_map(&p_i, &p_j, &p_k).unwrap();

        // Total momentum must be conserved:
        // p̃_{ij} + p̃_k = p_i + p_j + p_k
        let total_before = p_i + p_j + p_k;
        let total_after = result.p_tilde_ij + result.p_tilde_k;

        assert!(
            (total_before.e - total_after.e).abs() < 1e-10,
            "Energy not conserved: {} vs {}",
            total_before.e,
            total_after.e
        );
        assert!(
            (total_before.px - total_after.px).abs() < 1e-10,
            "px not conserved"
        );
        assert!(
            (total_before.py - total_after.py).abs() < 1e-10,
            "py not conserved"
        );
        assert!(
            (total_before.pz - total_after.pz).abs() < 1e-10,
            "pz not conserved"
        );
    }

    #[test]
    fn ff_map_spectator_on_shell() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let p_i = make_ll(5.0, 0.0, 0.0);
        let p_j = make_ll(0.0, 3.0, 0.0);
        let p_k = make_ll(0.0, 0.0, 4.0);

        let result = catani_seymour_ff_map(&p_i, &p_j, &p_k).unwrap();

        // p̃_k should be on-shell (massless): p̃_k² = 0
        let pk_mass_sq = result.p_tilde_k.invariant_mass_sq();
        assert!(
            pk_mass_sq.abs() < 1e-10,
            "Spectator not on-shell: p̃_k² = {}",
            pk_mass_sq
        );
    }

    #[test]
    fn ff_map_y_and_z_range() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let p_i = make_ll(10.0, 0.0, 0.0);
        let p_j = make_ll(0.5, 0.5, 0.0);
        let p_k = make_ll(0.0, 0.0, 8.0);

        let result = catani_seymour_ff_map(&p_i, &p_j, &p_k).unwrap();

        // y should be in (0, 1) for well-separated partons
        assert!(result.y > 0.0, "y should be positive: {}", result.y);
        assert!(result.y < 1.0, "y should be < 1: {}", result.y);

        // z should be in (0, 1)
        assert!(result.z > 0.0, "z should be positive: {}", result.z);
        assert!(result.z < 1.0, "z should be < 1: {}", result.z);
    }

    #[test]
    fn ff_map_degenerate_returns_error() {
        let zero = FourMomentum::zero();
        let result = catani_seymour_ff_map(&zero, &zero, &zero);
        assert!(result.is_err());
    }

    // ── Splitting function tests ────────────────────────────────────────

    #[test]
    fn splitting_qqg_soft_limit() {
        // In the soft limit z → 1, y → 0:
        // V_{q→qg} = C_F [ 2/(1-1*(1-0)) - (1+1) ] = C_F [2/0 - 2] → diverges
        // So at z slightly less than 1, the function should be large.
        let v = splitting_function(SplittingType::QuarkToQuarkGluon, 0.99, 0.01);
        assert!(v > 10.0, "Soft limit should give large splitting fn: {}", v);
    }

    #[test]
    fn splitting_qqg_at_z_half() {
        // V_{q→qg}(z=0.5, y=0.5) = C_F [ 2/(1-0.5*0.5) - 1.5 ]
        //                          = C_F [ 2/0.75 - 1.5 ]
        //                          = C_F [ 2.6667 - 1.5 ]
        //                          = C_F * 1.1667
        let v = splitting_function(SplittingType::QuarkToQuarkGluon, 0.5, 0.5);
        let expected = CF * (2.0 / 0.75 - 1.5);
        assert!(
            (v - expected).abs() < 1e-12,
            "V_qqg(0.5, 0.5) = {} (expected {})",
            v,
            expected
        );
    }

    #[test]
    fn splitting_ggg_symmetric() {
        // V_{g→gg} should have a symmetry under z ↔ (1-z) at fixed y
        let v1 = splitting_function(SplittingType::GluonToGluonGluon, 0.3, 0.4);
        let v2 = splitting_function(SplittingType::GluonToGluonGluon, 0.7, 0.4);
        assert!(
            (v1 - v2).abs() < 1e-12,
            "V_ggg should be z↔(1-z) symmetric: {} vs {}",
            v1,
            v2
        );
    }

    #[test]
    fn splitting_gqq_at_z_half() {
        // V_{g→qq̄}(z=0.5) = T_R [1 - 2*0.25] = T_R * 0.5
        let v = splitting_function(SplittingType::GluonToQuarkAntiquark, 0.5, 0.3);
        let expected = TR * 0.5;
        assert!(
            (v - expected).abs() < 1e-12,
            "V_gqq(0.5) = {} (expected {})",
            v,
            expected
        );
    }

    #[test]
    fn splitting_gqq_at_endpoints() {
        // At z=0: V = T_R [1 - 0] = T_R = 0.5
        let v0 = splitting_function(SplittingType::GluonToQuarkAntiquark, 0.0, 0.5);
        assert!((v0 - TR).abs() < 1e-12, "V_gqq(z=0) should be T_R");

        // At z=1: V = T_R [1 - 0] = T_R = 0.5
        let v1 = splitting_function(SplittingType::GluonToQuarkAntiquark, 1.0, 0.5);
        assert!((v1 - TR).abs() < 1e-12, "V_gqq(z=1) should be T_R");
    }

    // ── Build dipole tests ──────────────────────────────────────────────

    #[test]
    fn build_dipole_qqg() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let p_i = make_ll(10.0, 0.0, 0.0); // quark
        let p_j = make_ll(0.0, 5.0, 0.0); // gluon
        let p_k = make_ll(0.0, 0.0, 8.0); // spectator

        let d = build_dipole(&p_i, &p_j, &p_k, SplittingType::QuarkToQuarkGluon).unwrap();

        // Dipole factor should be negative (subtraction term)
        assert!(
            d.dipole_factor < 0.0,
            "Dipole factor should be negative: {}",
            d.dipole_factor
        );

        // pi·pj should be positive for well-separated light-like momenta
        assert!(
            d.pi_dot_pj > 0.0,
            "p_i·p_j should be positive: {}",
            d.pi_dot_pj
        );
    }

    // ── Identify splitting tests ────────────────────────────────────────

    #[test]
    fn identify_splitting_quark_gluon() {
        // up-quark (2) emitting a gluon (21) → QuarkToQuarkGluon
        assert_eq!(
            identify_splitting(2, 21),
            Some(SplittingType::QuarkToQuarkGluon)
        );
        // anti-down (-1) emitting gluon → QuarkToQuarkGluon
        assert_eq!(
            identify_splitting(-1, 21),
            Some(SplittingType::QuarkToQuarkGluon)
        );
    }

    #[test]
    fn identify_splitting_gluon_gluon() {
        assert_eq!(
            identify_splitting(21, 21),
            Some(SplittingType::GluonToGluonGluon)
        );
    }

    #[test]
    fn identify_splitting_gluon_quark() {
        // gluon emitting a quark → GluonToQuarkAntiquark
        assert_eq!(
            identify_splitting(21, 3),
            Some(SplittingType::GluonToQuarkAntiquark)
        );
        assert_eq!(
            identify_splitting(21, -5),
            Some(SplittingType::GluonToQuarkAntiquark)
        );
    }

    #[test]
    fn identify_splitting_invalid() {
        // quark-quark should not be a valid splitting
        assert_eq!(identify_splitting(1, 2), None);
        // lepton (11 = electron) should not match
        assert_eq!(identify_splitting(11, 21), None);
    }

    // ── CataniSeymourScheme tests ───────────────────────────────────────

    #[test]
    fn cs_scheme_three_gluons() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let momenta = vec![
            make_ll(10.0, 0.0, 0.0),
            make_ll(0.0, 8.0, 0.0),
            make_ll(0.0, 0.0, 6.0),
        ];
        let ids = vec![21, 21, 21]; // all gluons

        let scheme = CataniSeymourScheme;
        let config = SubtractionConfig::default();

        let result = scheme.compute_subtraction(&momenta, &ids, &config).unwrap();

        // Three gluons: 3 choices for emitter, 2 for emitted, 1 for spectator = 6 dipoles
        assert_eq!(
            result.dipoles.len(),
            6,
            "Three gluons should produce 6 dipoles"
        );

        // All dipoles should be g→gg type
        for d in &result.dipoles {
            assert_eq!(d.splitting, SplittingType::GluonToGluonGluon);
        }
    }

    #[test]
    fn cs_scheme_qqg() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let momenta = vec![
            make_ll(10.0, 0.0, 0.0),
            make_ll(0.0, 8.0, 0.0),
            make_ll(0.0, 0.0, 6.0),
        ];
        let ids = vec![2, -2, 21]; // u, ū, g

        let scheme = CataniSeymourScheme;
        let config = SubtractionConfig::default();

        let result = scheme.compute_subtraction(&momenta, &ids, &config).unwrap();

        // Expected dipoles:
        // (i=u, j=g, k=ū) → q→qg  ✓
        // (i=ū, j=g, k=u) → q→qg  ✓
        // (i=g, j=u, k=ū) → g→qq̄  ✓
        // (i=g, j=ū, k=u) → g→qq̄  ✓
        // (i=u, j=ū, k=g) → None (q+q̄ not a splitting)
        // (i=ū, j=u, k=g) → None (q̄+q not a splitting)
        assert_eq!(result.dipoles.len(), 4, "q q̄ g should give 4 dipoles");

        let n_qqg = result
            .dipoles
            .iter()
            .filter(|d| d.splitting == SplittingType::QuarkToQuarkGluon)
            .count();
        let n_gqq = result
            .dipoles
            .iter()
            .filter(|d| d.splitting == SplittingType::GluonToQuarkAntiquark)
            .count();

        assert_eq!(n_qqg, 2, "Should have 2 q→qg dipoles");
        assert_eq!(n_gqq, 2, "Should have 2 g→qq̄ dipoles");
    }

    #[test]
    fn cs_scheme_too_few_partons() {
        let momenta = vec![
            FourMomentum::new(10.0, 10.0, 0.0, 0.0),
            FourMomentum::new(10.0, -10.0, 0.0, 0.0),
        ];
        let ids = vec![21, 21];

        let scheme = CataniSeymourScheme;
        let result = scheme.compute_subtraction(&momenta, &ids, &SubtractionConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn cs_scheme_mismatched_lengths() {
        let momenta = vec![
            FourMomentum::new(10.0, 10.0, 0.0, 0.0),
            FourMomentum::new(10.0, -10.0, 0.0, 0.0),
            FourMomentum::new(10.0, 0.0, 10.0, 0.0),
        ];
        let ids = vec![21, 21]; // one too few

        let scheme = CataniSeymourScheme;
        let result = scheme.compute_subtraction(&momenta, &ids, &SubtractionConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn cs_scheme_y_cut_filters() {
        let make_ll = |px: f64, py: f64, pz: f64| -> FourMomentum {
            let e = (px * px + py * py + pz * pz).sqrt();
            FourMomentum::new(e, px, py, pz)
        };

        let momenta = vec![
            make_ll(10.0, 0.0, 0.0),
            make_ll(0.0, 8.0, 0.0),
            make_ll(0.0, 0.0, 6.0),
        ];
        let ids = vec![21, 21, 21];

        let scheme = CataniSeymourScheme;

        // With y_max = 0.001, all dipoles should be filtered out
        // since these well-separated partons have y >> 0.001
        let tight_config = SubtractionConfig {
            y_min: 1e-7,
            y_max: 0.001,
            alpha: 1.0,
        };

        let result = scheme
            .compute_subtraction(&momenta, &ids, &tight_config)
            .unwrap();
        assert_eq!(
            result.dipoles.len(),
            0,
            "Tight y_max should filter all dipoles"
        );
    }

    // ── Colour factor tests ─────────────────────────────────────────────

    #[test]
    fn colour_factors_values() {
        assert!((CF - 4.0 / 3.0).abs() < 1e-15);
        assert!((CA - 3.0).abs() < 1e-15);
        assert!((TR - 0.5).abs() < 1e-15);
    }
}
