//! # Kinematics - Phase Space and Relativistic Calculus
//!
//! This module implements the relativistic kinematics of scattering and decay
//! processes in quantum field theory:
//!
//! - **Mandelstam variables** ($s$, $t$, $u$) for $2 \to 2$ scattering.
//! - **Källén triangle function** $\lambda(x, y, z)$ - the universal kinematic
//!   kernel for 2-body relativistic phase space.
//! - **CM-frame momentum** $p^*$ from invariant mass and rest masses.
//! - **2-body phase space volume** $\Phi_2$.
//! - **Dalitz plot boundaries** for 3-body decays.
//! - **Reaction thresholds** - minimum $\sqrt{s}$ for particle production.
//! - **Lorentz boosts** between reference frames.
//!
//! # Metric Convention
//!
//! All four-momentum contractions use the $(+,-,-,-)$ metric inherited from
//! [`crate::algebra::FourMomentum`].
//!
//! # Key Formulas
//!
//! **Källén function:**
//! $$\lambda(x, y, z) = x^2 + y^2 + z^2 - 2xy - 2xz - 2yz$$
//!
//! **CM 3-momentum for $a + b$ at invariant mass squared $s$:**
//! $$p^* = \frac{\sqrt{\lambda(s,\, m_a^2,\, m_b^2)}}{2\sqrt{s}}$$
//!
//! **Integrated 2-body phase space (Lorentz-invariant):**
//! $$\Phi_2(s;\, m_3, m_4) = \frac{p^*_f}{8\pi\sqrt{s}}$$
//!
//! **Mandelstam sum rule:**
//! $$s + t + u = m_1^2 + m_2^2 + m_3^2 + m_4^2$$

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::algebra::{FourMomentum, MetricSignature, SpacetimeVector};
use crate::SpireResult;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

// ---------------------------------------------------------------------------
// Core Data Structures
// ---------------------------------------------------------------------------

/// The **Mandelstam variables** $(s, t, u)$ for a $2 \to 2$ scattering process.
///
/// Defined from the external 4-momenta $p_1, p_2$ (incoming) and $p_3, p_4$
/// (outgoing):
/// - $s = (p_1 + p_2)^2$
/// - $t = (p_1 - p_3)^2$
/// - $u = (p_1 - p_4)^2$
///
/// They satisfy the constraint $s + t + u = \sum_i m_i^2$.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MandelstamVars {
    /// $s = (p_1 + p_2)^2$: centre-of-mass energy squared (GeV²).
    pub s: f64,
    /// $t = (p_1 - p_3)^2$: momentum transfer squared (GeV²).
    pub t: f64,
    /// $u = (p_1 - p_4)^2$: crossed momentum transfer squared (GeV²).
    pub u: f64,
}

/// Physical boundaries for Mandelstam variables in a $2 \to 2$ process.
///
/// Defines the kinematically allowed region at a given $s$, including the
/// $t$-channel boundaries computed from the Källén function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandelstamBoundaries {
    /// Minimum $s$ (threshold): $(m_3 + m_4)^2$.
    pub s_min: f64,
    /// Minimum $t$ at the specified $s$.
    pub t_min: f64,
    /// Maximum $t$ at the specified $s$.
    pub t_max: f64,
    /// Masses of the four external particles $[m_1, m_2, m_3, m_4]$.
    pub masses: [f64; 4],
}

/// Description of the relativistic phase space for an $N$-body final state.
///
/// Contains the Lorentz-invariant phase space (LIPS) measure and the
/// dimensionality of the integration domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSpace {
    /// Number of final-state particles.
    pub n_final: usize,
    /// Number of independent integration variables ($3N - 4$ for $N$-body).
    pub n_variables: usize,
    /// Total available energy in the CM frame (GeV).
    pub total_energy_cm: f64,
    /// Masses of the final-state particles.
    pub final_masses: Vec<f64>,
    /// Symbolic expression for the phase-space measure.
    pub measure_expression: String,
}

/// Boundaries of a **Dalitz plot** for a 3-body decay $M \to a + b + c$.
///
/// The Dalitz plot is parametrised by two invariant mass-squared variables:
/// $s_{ab} = (p_a + p_b)^2$ and $s_{bc} = (p_b + p_c)^2$.
///
/// The absolute kinematic limits are:
/// - $s_{ab}^{\min} = (m_a + m_b)^2$, $\quad s_{ab}^{\max} = (M - m_c)^2$
/// - $s_{bc}^{\min} = (m_b + m_c)^2$, $\quad s_{bc}^{\max} = (M - m_a)^2$
///
/// Within these ranges, the boundary of the physical region is an elliptical
/// curve determined by the Källén function evaluated in the $(a,b)$ rest frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DalitzBoundaries {
    /// Mother particle mass in GeV.
    pub mother_mass: f64,
    /// Daughter masses $[m_a, m_b, m_c]$ in GeV.
    pub daughter_masses: [f64; 3],
    /// Minimum of $s_{ab} = (m_a + m_b)^2$ (GeV²).
    pub m_ab_sq_min: f64,
    /// Maximum of $s_{ab} = (M - m_c)^2$ (GeV²).
    pub m_ab_sq_max: f64,
    /// Minimum of $s_{bc} = (m_b + m_c)^2$ (GeV²).
    pub m_bc_sq_min: f64,
    /// Maximum of $s_{bc} = (M - m_a)^2$ (GeV²).
    pub m_bc_sq_max: f64,
}

/// The result of computing threshold energy for a specific final state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdResult {
    /// Minimum CM energy $\sqrt{s_{\min}}$ required (GeV).
    pub threshold_energy: f64,
    /// Minimum lab-frame beam energy for a fixed-target setup (GeV).
    /// Computed as $E_{\text{lab}} = \frac{s_{\min} - m_b^2 - m_t^2}{2 m_t}$
    /// where $m_t$ is the target mass. `None` if no target specified.
    pub lab_energy: Option<f64>,
    /// Masses of the final-state particles used in the calculation.
    pub final_masses: Vec<f64>,
}

/// A Lorentz boost specification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LorentzBoost {
    /// Boost velocity $\beta = v/c$ along each spatial axis.
    pub beta: [f64; 3],
    /// Lorentz factor $\gamma = 1/\sqrt{1-\beta^2}$.
    pub gamma: f64,
}

/// Reference frame for kinematic calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceFrame {
    /// Centre-of-mass (CM) frame.
    CentreOfMass,
    /// Laboratory (fixed-target) frame.
    Lab,
}

// ---------------------------------------------------------------------------
// Fundamental Kinematic Functions
// ---------------------------------------------------------------------------

/// The **Källén triangle function** (kinematic triangle function):
///
/// $$\lambda(x, y, z) = x^2 + y^2 + z^2 - 2xy - 2xz - 2yz$$
///
/// This is the universal kernel of 2-body relativistic kinematics. It appears
/// in the CM-frame 3-momentum, phase-space volumes, and Dalitz plot boundaries.
///
/// Equivalent forms:
/// - $\lambda(x, y, z) = (x - y - z)^2 - 4yz$
/// - $\lambda(x, y, z) = [x - (\sqrt{y} + \sqrt{z})^2][x - (\sqrt{y} - \sqrt{z})^2]$
///   (when $y, z \ge 0$)
///
/// # Examples
///
/// ```
/// # use spire_kernel::kinematics::kallen_lambda;
/// // λ(s, m₁², m₂²) for equal masses m at √s = 2m:
/// // λ(4m², m², m²) = 16m⁴ + m⁴ + m⁴ - 8m⁴ - 8m⁴ - 2m⁴ = 0 (at threshold)
/// let m_sq = 1.0;
/// assert!((kallen_lambda(4.0 * m_sq, m_sq, m_sq)).abs() < 1e-12);
/// ```
pub fn kallen_lambda(x: f64, y: f64, z: f64) -> f64 {
    x * x + y * y + z * z - 2.0 * x * y - 2.0 * x * z - 2.0 * y * z
}

/// Compute the **CM-frame 3-momentum** $p^*$ for a 2-body system at
/// invariant mass squared $s$:
///
/// $$p^* = \frac{\sqrt{\lambda(s,\, m_a^2,\, m_b^2)}}{2\sqrt{s}}$$
///
/// Returns `0.0` when the configuration is unphysical ($\lambda < 0$ or
/// $s \le 0$), i.e. below threshold.
///
/// # Arguments
/// * `s` - Invariant mass squared $s$ of the 2-body system (GeV²).
/// * `m_a` - Mass of particle $a$ (GeV).
/// * `m_b` - Mass of particle $b$ (GeV).
pub fn cm_momentum(s: f64, m_a: f64, m_b: f64) -> f64 {
    if s <= 0.0 {
        return 0.0;
    }

    let lam = kallen_lambda(s, m_a * m_a, m_b * m_b);

    if lam < 0.0 {
        return 0.0;
    }

    lam.sqrt() / (2.0 * s.sqrt())
}

/// Compute the **integrated 2-body Lorentz-invariant phase space** volume:
///
/// $$\Phi_2(s;\, m_3, m_4) = \frac{p^*_f}{8\pi\sqrt{s}}$$
///
/// where $p^*_f$ is the CM-frame 3-momentum of either final-state particle.
///
/// This is the standard PDG convention (see PDG Reviews §49 "Kinematics"):
///
/// $$\Phi_2 = \frac{1}{8\pi} \frac{|\vec{p}^{\,*}|}{\sqrt{s}}$$
///
/// which when multiplied by $|\mathcal{M}|^2$ and divided by the flux factor
/// $4 p^*_i \sqrt{s}$ gives the total cross section.
///
/// Returns `0.0` below threshold.
///
/// # Arguments
/// * `s` - Invariant mass squared of the system (GeV²).
/// * `m3` - Mass of final-state particle 3 (GeV).
/// * `m4` - Mass of final-state particle 4 (GeV).
pub fn two_body_phase_space(s: f64, m3: f64, m4: f64) -> f64 {
    let p_star = cm_momentum(s, m3, m4);
    if p_star <= 0.0 || s <= 0.0 {
        return 0.0;
    }
    p_star / (8.0 * PI * s.sqrt())
}

// ---------------------------------------------------------------------------
// Mandelstam Variables
// ---------------------------------------------------------------------------

/// Compute the **Mandelstam variables** for a $2 \to 2$ process from the
/// four-momenta of all external particles.
///
/// Uses the `FourMomentum` arithmetic from the algebra module:
/// - $s = (p_1 + p_2)^2$
/// - $t = (p_1 - p_3)^2$
/// - $u = (p_1 - p_4)^2$
///
/// # Arguments
/// * `p1` - Four-momentum of incoming particle 1.
/// * `p2` - Four-momentum of incoming particle 2.
/// * `p3` - Four-momentum of outgoing particle 3.
/// * `p4` - Four-momentum of outgoing particle 4.
pub fn compute_mandelstam(
    p1: &FourMomentum,
    p2: &FourMomentum,
    p3: &FourMomentum,
    p4: &FourMomentum,
) -> MandelstamVars {
    let s = (*p1 + *p2).invariant_mass_sq();
    let t = (*p1 - *p3).invariant_mass_sq();
    let u = (*p1 - *p4).invariant_mass_sq();

    MandelstamVars { s, t, u }
}

impl MandelstamVars {
    /// Verify the **Mandelstam sum rule**:
    ///
    /// $$s + t + u = m_1^2 + m_2^2 + m_3^2 + m_4^2$$
    ///
    /// Returns `true` if the identity holds within floating-point tolerance
    /// (relative error $< 10^{-9}$ or absolute error $< 10^{-12}$).
    ///
    /// # Arguments
    /// * `masses` - Rest masses of the four external particles $[m_1, m_2, m_3, m_4]$.
    pub fn verify_sum_rule(&self, masses: &[f64; 4]) -> bool {
        let sum_stu = self.s + self.t + self.u;
        let sum_m_sq: f64 = masses.iter().map(|m| m * m).sum();
        let diff = (sum_stu - sum_m_sq).abs();

        // Use both absolute and relative tolerance.
        let scale = sum_m_sq.abs().max(sum_stu.abs()).max(1.0);
        diff < 1e-9 * scale || diff < 1e-12
    }
}

/// Compute the physical **boundaries** of the Mandelstam variables for a
/// $2 \to 2$ process at a given $s$.
///
/// The $t$-channel boundaries at fixed $s$ are:
///
/// $$t_{\pm} = m_1^2 + m_3^2 - \frac{1}{2s}\bigl[(s + m_1^2 - m_2^2)(s + m_3^2 - m_4^2)
///   \mp \sqrt{\lambda(s, m_1^2, m_2^2)\,\lambda(s, m_3^2, m_4^2)}\bigr]$$
///
/// The minimum $s$ is the production threshold: $s_{\min} = (m_3 + m_4)^2$.
///
/// # Arguments
/// * `masses` - The four external particle masses $[m_1, m_2, m_3, m_4]$ in GeV.
/// * `s` - The centre-of-mass energy squared at which to evaluate the $t$ boundaries.
pub fn compute_mandelstam_boundaries(
    masses: [f64; 4],
    s: f64,
) -> SpireResult<MandelstamBoundaries> {
    let [m1, m2, m3, m4] = masses;
    let s_min = (m3 + m4) * (m3 + m4);

    let m1_sq = m1 * m1;
    let m2_sq = m2 * m2;
    let m3_sq = m3 * m3;
    let m4_sq = m4 * m4;

    // If s is below threshold, return the threshold with degenerate t boundaries.
    if s < s_min - 1e-12 {
        return Ok(MandelstamBoundaries {
            s_min,
            t_min: 0.0,
            t_max: 0.0,
            masses,
        });
    }

    let lam_12 = kallen_lambda(s, m1_sq, m2_sq);
    let lam_34 = kallen_lambda(s, m3_sq, m4_sq);

    let sqrt_lam_product = if lam_12 >= 0.0 && lam_34 >= 0.0 {
        (lam_12 * lam_34).sqrt()
    } else {
        0.0
    };

    let common = (s + m1_sq - m2_sq) * (s + m3_sq - m4_sq) / (2.0 * s);

    let t_minus = m1_sq + m3_sq - common - sqrt_lam_product / (2.0 * s);
    let t_plus = m1_sq + m3_sq - common + sqrt_lam_product / (2.0 * s);

    Ok(MandelstamBoundaries {
        s_min,
        t_min: t_minus,
        t_max: t_plus,
        masses,
    })
}

// ---------------------------------------------------------------------------
// Dalitz Plot Boundaries
// ---------------------------------------------------------------------------

/// Generate the **Dalitz plot kinematic boundaries** for a 3-body decay
/// $M \to a + b + c$.
///
/// Computes the absolute kinematic limits on the invariant mass-squared
/// variables $s_{ab} = (p_a + p_b)^2$ and $s_{bc} = (p_b + p_c)^2$:
///
/// - $s_{ab}^{\min} = (m_a + m_b)^2$, $\quad s_{ab}^{\max} = (M - m_c)^2$
/// - $s_{bc}^{\min} = (m_b + m_c)^2$, $\quad s_{bc}^{\max} = (M - m_a)^2$
///
/// The physical region within these limits is bounded by an elliptical curve
/// determined by momentum conservation and the Källén function.
///
/// # Errors
///
/// Returns `SpireError::KinematicsError` if $M < m_a + m_b + m_c$ (decay
/// is kinematically forbidden).
///
/// # Arguments
/// * `mother_mass` - Mass of the decaying particle in GeV.
/// * `daughter_masses` - Masses of the three daughter particles $[m_a, m_b, m_c]$.
pub fn generate_dalitz_boundaries(
    mother_mass: f64,
    daughter_masses: [f64; 3],
) -> SpireResult<DalitzBoundaries> {
    let [m_a, m_b, m_c] = daughter_masses;
    let mass_sum = m_a + m_b + m_c;

    if mother_mass < mass_sum - 1e-12 {
        return Err(crate::SpireError::KinematicsForbidden(format!(
            "Decay kinematically forbidden: M = {:.6} GeV < m_a + m_b + m_c = {:.6} GeV",
            mother_mass, mass_sum
        )));
    }

    Ok(DalitzBoundaries {
        mother_mass,
        daughter_masses,
        m_ab_sq_min: (m_a + m_b) * (m_a + m_b),
        m_ab_sq_max: (mother_mass - m_c) * (mother_mass - m_c),
        m_bc_sq_min: (m_b + m_c) * (m_b + m_c),
        m_bc_sq_max: (mother_mass - m_a) * (mother_mass - m_a),
    })
}

/// Compute the **Dalitz boundary for $s_{bc}$** at a given value of $s_{ab}$.
///
/// For fixed $s_{ab}$, the kinematically allowed range of $s_{bc}$ is:
///
/// $$s_{bc}^{\pm} = (E_b^* + E_c^*)^2 - \left(\sqrt{E_b^{*2} - m_b^2} \mp \sqrt{E_c^{*2} - m_c^2}\right)^2$$
///
/// where the starred energies are evaluated in the $(a,b)$ rest frame:
/// - $E_b^* = (s_{ab} - m_a^2 + m_b^2) / (2\sqrt{s_{ab}})$
/// - $E_c^* = (M^2 - s_{ab} - m_c^2) / (2\sqrt{s_{ab}})$
///
/// Returns `(s_bc_min, s_bc_max)` for the given $s_{ab}$, or `None` if
/// $s_{ab}$ is outside its allowed range.
pub fn dalitz_s_bc_limits_at_s_ab(
    mother_mass: f64,
    m_a: f64,
    m_b: f64,
    m_c: f64,
    s_ab: f64,
) -> Option<(f64, f64)> {
    let m_sq = mother_mass * mother_mass;
    let s_ab_sqrt = s_ab.sqrt();

    if s_ab_sqrt < 1e-15 {
        return None;
    }

    // Energies of b and c in the (a,b) rest frame
    let e_b_star = (s_ab - m_a * m_a + m_b * m_b) / (2.0 * s_ab_sqrt);
    let e_c_star = (m_sq - s_ab - m_c * m_c) / (2.0 * s_ab_sqrt);

    // Check that both energies allow physical momenta
    let p_b_star_sq = e_b_star * e_b_star - m_b * m_b;
    let p_c_star_sq = e_c_star * e_c_star - m_c * m_c;

    if p_b_star_sq < -1e-12 || p_c_star_sq < -1e-12 {
        return None;
    }

    let p_b_star = p_b_star_sq.max(0.0).sqrt();
    let p_c_star = p_c_star_sq.max(0.0).sqrt();

    let e_sum = e_b_star + e_c_star;
    let s_bc_max = e_sum * e_sum - (p_b_star - p_c_star) * (p_b_star - p_c_star);
    let s_bc_min = e_sum * e_sum - (p_b_star + p_c_star) * (p_b_star + p_c_star);

    Some((s_bc_min, s_bc_max))
}

// ---------------------------------------------------------------------------
// Dalitz Plot Data Generation
// ---------------------------------------------------------------------------

/// A collection of phase-space points for rendering a Dalitz plot.
///
/// Each point $(x, y) = (s_{ab}, s_{bc})$ lies within the kinematically
/// allowed region for the 3-body decay $M \to a + b + c$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DalitzPlotData {
    /// The Dalitz plot boundaries (absolute kinematic limits).
    pub boundaries: DalitzBoundaries,
    /// Pairs of $(s_{ab}, s_{bc})$ values in the physical region (GeV²).
    pub points: Vec<(f64, f64)>,
    /// Number of grid divisions used along the $s_{ab}$ axis.
    pub n_grid: usize,
}

/// Generate a uniform grid of **Dalitz plot phase-space points** for a 3-body
/// decay $M \to a + b + c$.
///
/// The algorithm scans a regular grid in $s_{ab}$ (from $(m_a + m_b)^2$ to
/// $(M - m_c)^2$) and, for each slice, distributes points uniformly in
/// $s_{bc}$ between its kinematic minimum and maximum.
///
/// # Arguments
/// * `mother_mass` - Mass of the decaying particle $M$ (GeV).
/// * `m_a`, `m_b`, `m_c` - Daughter particle masses (GeV).
/// * `n_points` - Approximate number of points to generate. The function
///   uses $\sqrt{n}$ grid divisions along each axis.
///
/// # Errors
/// Returns `SpireError::KinematicsForbidden` if $M < m_a + m_b + m_c$.
///
/// # Example
///
/// ```rust,no_run
/// # use spire_kernel::kinematics::generate_dalitz_plot_data;
/// let data = generate_dalitz_plot_data(1.865, 0.494, 0.140, 0.135, 5000).unwrap();
/// assert!(!data.points.is_empty());
/// ```
pub fn generate_dalitz_plot_data(
    mother_mass: f64,
    m_a: f64,
    m_b: f64,
    m_c: f64,
    n_points: usize,
) -> SpireResult<DalitzPlotData> {
    let boundaries = generate_dalitz_boundaries(mother_mass, [m_a, m_b, m_c])?;

    // Use √n_points grid divisions along each axis for a roughly uniform fill.
    let n_grid = (n_points as f64).sqrt().ceil().max(2.0) as usize;

    let s_ab_min = boundaries.m_ab_sq_min;
    let s_ab_max = boundaries.m_ab_sq_max;
    let s_ab_step = (s_ab_max - s_ab_min) / (n_grid as f64);

    let mut points = Vec::with_capacity(n_points);

    for i in 0..n_grid {
        let s_ab = s_ab_min + (i as f64 + 0.5) * s_ab_step;

        // Get the allowed s_bc range at this s_ab slice.
        let (s_bc_lo, s_bc_hi) = match dalitz_s_bc_limits_at_s_ab(mother_mass, m_a, m_b, m_c, s_ab)
        {
            Some(limits) => limits,
            None => continue, // outside physical region
        };

        if s_bc_hi - s_bc_lo < 1e-14 {
            // Degenerate slice - emit a single point.
            points.push((s_ab, (s_bc_lo + s_bc_hi) / 2.0));
            continue;
        }

        // Distribute n_grid points uniformly in s_bc for this slice.
        let s_bc_step = (s_bc_hi - s_bc_lo) / (n_grid as f64);
        for j in 0..n_grid {
            let s_bc = s_bc_lo + (j as f64 + 0.5) * s_bc_step;
            points.push((s_ab, s_bc));
        }
    }

    Ok(DalitzPlotData {
        boundaries,
        points,
        n_grid,
    })
}

/// Calculate the **threshold (minimum) CM energy** required to produce a given
/// set of final-state particles.
///
/// $$\sqrt{s_{\min}} = \sum_i m_i$$
///
/// For a fixed-target experiment with beam hitting a target of mass $m_t$,
/// the minimum lab-frame beam energy is:
///
/// $$E_{\text{lab}} = \frac{s_{\min} - m_b^2 - m_t^2}{2\, m_t}$$
///
/// where $m_b$ is the beam particle mass and $s_{\min} = (\sum_i m_i)^2$.
///
/// # Arguments
/// * `final_masses` - Rest masses of the desired final-state particles (GeV).
/// * `target_mass` - If computing the lab-frame threshold, provide `Some((beam_mass, target_mass))`.
pub fn calculate_thresholds(
    final_masses: &[f64],
    target_mass: Option<f64>,
) -> SpireResult<ThresholdResult> {
    let threshold_energy: f64 = final_masses.iter().sum();
    let s_min = threshold_energy * threshold_energy;

    let lab_energy = target_mass.map(|m_t| {
        // In fixed-target: s = m_b² + m_t² + 2 m_t E_lab
        // At threshold: E_lab = (s_min - m_b² - m_t²) / (2 m_t)
        // We approximate beam mass as the lightest final-state particle or 0
        // For a general threshold, assume the beam is massless (worst case).
        // A more refined version would take beam_mass as an argument.
        if m_t > 0.0 {
            (s_min - m_t * m_t) / (2.0 * m_t)
        } else {
            f64::INFINITY
        }
    });

    Ok(ThresholdResult {
        threshold_energy,
        lab_energy,
        final_masses: final_masses.to_vec(),
    })
}

/// Check whether a reaction is **kinematically allowed** at a given CM energy.
///
/// Returns `true` if $\sqrt{s} \ge \sum_i m_i$ (the available energy exceeds
/// the production threshold).
///
/// # Arguments
/// * `available_cms_energy` - Available centre-of-mass energy $\sqrt{s}$ in GeV.
/// * `final_masses` - Rest masses of the final-state particles in GeV.
pub fn is_kinematically_allowed(available_cms_energy: f64, final_masses: &[f64]) -> bool {
    let threshold: f64 = final_masses.iter().sum();
    available_cms_energy >= threshold - 1e-12
}

// ---------------------------------------------------------------------------
// Lorentz Boost
// ---------------------------------------------------------------------------

/// Apply a **Lorentz boost** to a four-momentum.
///
/// For a boost with velocity $\vec{\beta}$ and Lorentz factor $\gamma$,
/// the general transformation is:
///
/// $$E' = \gamma(E - \vec{\beta} \cdot \vec{p})$$
/// $$\vec{p}\,' = \vec{p} + (\gamma - 1) \frac{(\vec{\beta} \cdot \vec{p})}{|\vec{\beta}|^2} \vec{\beta} - \gamma E \vec{\beta}$$
///
/// # Arguments
/// * `momentum` - The four-momentum to transform.
/// * `boost` - The Lorentz boost specification.
pub fn apply_lorentz_boost(
    momentum: &FourMomentum,
    boost: &LorentzBoost,
) -> SpireResult<FourMomentum> {
    let [bx, by, bz] = boost.beta;
    let beta_sq = bx.mul_add(bx, by.mul_add(by, bz * bz));

    // If no boost, return unchanged.
    if beta_sq < 1e-30 {
        return Ok(*momentum);
    }

    if !beta_sq.is_finite() || beta_sq >= 1.0 {
        return Err(crate::SpireError::KinematicsForbidden(format!(
            "Invalid Lorentz boost: β² = {:.16} (must satisfy 0 ≤ β² < 1)",
            beta_sq
        )));
    }

    // Recompute γ from β to enforce internal consistency and reduce drift
    // when callers provide a rounded/inconsistent gamma value.
    let gamma = 1.0 / (1.0 - beta_sq).sqrt();

    let beta_dot_p = bx.mul_add(momentum.px, by.mul_add(momentum.py, bz * momentum.pz));

    let e_prime = gamma * (momentum.e - beta_dot_p);

    // Use (γ - 1) / β² = γ² / (γ + 1) to avoid subtractive cancellation for small boosts.
    let parallel_coeff = (gamma * gamma / (gamma + 1.0)) * beta_dot_p;
    let factor = parallel_coeff - gamma * momentum.e;

    let px_prime = bx.mul_add(factor, momentum.px);
    let py_prime = by.mul_add(factor, momentum.py);
    let pz_prime = bz.mul_add(factor, momentum.pz);

    Ok(FourMomentum::new(e_prime, px_prime, py_prime, pz_prime))
}

/// Compute the **boost parameters** to transform from the lab frame to the
/// centre-of-mass frame for a fixed-target collision.
///
/// In a fixed-target setup, the beam has momentum $p_{\text{beam}}$ and the
/// target is at rest with mass $m_t$. The CM frame has velocity:
///
/// $$\vec{\beta}_{\text{CM}} = \frac{\vec{p}_{\text{beam}}}{E_{\text{beam}} + m_t}$$
///
/// # Arguments
/// * `beam_momentum` - Four-momentum of the beam particle.
/// * `target_mass` - Rest mass of the stationary target particle.
pub fn compute_cm_boost(
    beam_momentum: &FourMomentum,
    target_mass: f64,
) -> SpireResult<LorentzBoost> {
    let e_total = beam_momentum.e + target_mass;

    if e_total.abs() < 1e-15 {
        return Err(crate::SpireError::KinematicsForbidden(
            "Total energy is zero - cannot compute CM boost".into(),
        ));
    }

    let bx = beam_momentum.px / e_total;
    let by = beam_momentum.py / e_total;
    let bz = beam_momentum.pz / e_total;

    let beta_sq = bx * bx + by * by + bz * bz;

    if beta_sq >= 1.0 {
        return Err(crate::SpireError::KinematicsForbidden(format!(
            "Superluminal boost: β² = {:.6} ≥ 1",
            beta_sq
        )));
    }

    let gamma = 1.0 / (1.0 - beta_sq).sqrt();

    Ok(LorentzBoost {
        beta: [bx, by, bz],
        gamma,
    })
}

// ---------------------------------------------------------------------------
// Phase Space Generation
// ---------------------------------------------------------------------------

/// Generate the **Lorentz-invariant phase space** (LIPS) description for an
/// $N$-body final state.
///
/// For a 2-body final state the integrated LIPS is:
///
/// $$\Phi_2 = \frac{p^*_f}{8\pi\sqrt{s}}$$
///
/// For general $N$-body, the number of independent integration variables is
/// $3N - 4$ (after removing 4 constraints from energy-momentum conservation).
///
/// # Arguments
/// * `total_energy_cm` - Total CM energy in GeV.
/// * `final_masses` - Masses of the $N$ final-state particles.
pub fn generate_phase_space(total_energy_cm: f64, final_masses: &[f64]) -> SpireResult<PhaseSpace> {
    let n = final_masses.len();
    if n == 0 {
        return Err(crate::SpireError::KinematicsForbidden(
            "Phase space requires at least 1 final-state particle".into(),
        ));
    }

    let mass_sum: f64 = final_masses.iter().sum();
    if total_energy_cm < mass_sum - 1e-12 {
        return Err(crate::SpireError::KinematicsForbidden(format!(
            "Insufficient energy: √s = {:.6} GeV < Σm = {:.6} GeV",
            total_energy_cm, mass_sum
        )));
    }

    let s = total_energy_cm * total_energy_cm;

    // Number of independent variables: 3N - 4 for N ≥ 2, 0 for N = 1.
    let n_variables = if n >= 2 { 3 * n - 4 } else { 0 };

    let measure_expression = if n == 2 {
        let p_star = cm_momentum(s, final_masses[0], final_masses[1]);
        format!(
            "Φ₂ = p*/(8π√s) = {:.6e} GeV⁻¹ (p* = {:.6} GeV)",
            two_body_phase_space(s, final_masses[0], final_masses[1]),
            p_star
        )
    } else {
        format!(
            "dΦ_{} - {}-body LIPS with {} integration variables",
            n, n, n_variables
        )
    };

    Ok(PhaseSpace {
        n_final: n,
        n_variables,
        total_energy_cm,
        final_masses: final_masses.to_vec(),
        measure_expression,
    })
}

// ===========================================================================
// Phase Space Generator - Trait & RAMBO Implementation
// ===========================================================================

// ---------------------------------------------------------------------------
// Phase Space Point
// ---------------------------------------------------------------------------

/// A single generated phase-space point with $N$-body final-state momenta.
///
/// Stores the 4-momenta of all final-state particles along with the
/// Lorentz-invariant phase space (LIPS) weight required for Monte Carlo
/// integration.
///
/// # Invariant
///
/// Energy-momentum conservation is strictly enforced:
/// $$\sum_{i=1}^{N} p_i^\mu = P_{\text{total}}^\mu = (E_\text{cms}, 0, 0, 0)$$
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSpacePoint {
    /// The 4-momenta of all final-state particles.
    pub momenta: Vec<SpacetimeVector>,
    /// The LIPS weight $w$ of this phase-space configuration.
    ///
    /// For flat Monte Carlo integration:
    /// $$\int d\Phi_N \approx \frac{1}{N_{\text{events}}} \sum_i w_i \cdot f(p_i)$$
    pub weight: f64,
}

// ---------------------------------------------------------------------------
// PhaseSpaceGenerator Trait
// ---------------------------------------------------------------------------

/// Trait for generating random $N$-body phase space configurations.
///
/// Implementations must produce 4-momentum configurations that strictly
/// conserve energy-momentum: $\sum_i p_i^\mu = (E_\text{cms}, 0, 0, 0)$.
///
/// Each generated event carries a weight that encodes the Lorentz-invariant
/// phase space measure, ensuring correct Monte Carlo integration.
///
/// # Implementations
///
/// - [`RamboGenerator`]: Flat $N$-body phase space using the RAMBO algorithm.
/// - Future: Adaptive importance sampling, multi-channel generators.
pub trait PhaseSpaceGenerator: Send + Sync {
    /// Generate a single phase-space event.
    ///
    /// # Arguments
    /// * `cms_energy` - Centre-of-mass energy $\sqrt{s}$ in GeV.
    /// * `final_masses` - Rest masses of the $N$ final-state particles in GeV.
    ///
    /// # Returns
    /// A `PhaseSpacePoint` containing the momenta and LIPS weight.
    fn generate_event(
        &mut self,
        cms_energy: f64,
        final_masses: &[f64],
    ) -> SpireResult<PhaseSpacePoint>;

    /// The name of this generator (for diagnostics and logging).
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// RAMBO Algorithm
// ---------------------------------------------------------------------------

/// The **RAMBO** (RAndom Momenta BOoster) phase-space generator.
///
/// Generates flat $N$-body Lorentz-invariant phase space with correct weight.
/// The algorithm is:
///
/// 1. Generate $N$ massless isotropic 4-momenta from an exponential energy
///    distribution and uniform angular distribution.
/// 2. Boost and scale the system so that the total 4-momentum equals
///    $(E_\text{cms}, 0, 0, 0)$.
/// 3. Apply mass corrections iteratively (Newton's method) to achieve the
///    required final-state particle masses.
/// 4. Compute the exact LIPS weight including mass correction factors.
///
/// # Reference
///
/// R. Kleiss, W.J. Stirling, S.D. Ellis,
/// "A new Monte Carlo treatment of multiparticle phase space at high energies",
/// Computer Physics Communications 40 (1986) 359–373.
pub struct RamboGenerator {
    /// The random number generator (seeded, reproducible, thread-safe).
    rng: StdRng,
}

impl RamboGenerator {
    /// Create a new RAMBO generator with a random seed.
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a RAMBO generator with a fixed seed (for reproducible results).
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl Default for RamboGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseSpaceGenerator for RamboGenerator {
    fn generate_event(
        &mut self,
        cms_energy: f64,
        final_masses: &[f64],
    ) -> SpireResult<PhaseSpacePoint> {
        let n = final_masses.len();
        if n < 2 {
            return Err(crate::SpireError::KinematicsForbidden(
                "RAMBO requires at least 2 final-state particles".into(),
            ));
        }

        let mass_sum: f64 = final_masses.iter().sum();
        if cms_energy < mass_sum - 1e-12 {
            return Err(crate::SpireError::KinematicsForbidden(format!(
                "Insufficient energy: √s = {:.6} GeV < Σm = {:.6} GeV",
                cms_energy, mass_sum
            )));
        }

        // Step 1: Generate N massless isotropic 4-momenta.
        let q: Vec<SpacetimeVector> = (0..n).map(|_| self.generate_massless_isotropic()).collect();

        // Step 2: Compute total 4-vector Q = Σq_i.
        let q_total = sum_vectors(&q);
        let metric = MetricSignature::minkowski_4d();
        let q_sq = metric
            .inner_product(&q_total, &q_total)
            .map_err(crate::SpireError::InternalError)?;
        let m_q = q_sq.sqrt();

        if m_q < 1e-15 {
            return Err(crate::SpireError::InternalError(
                "RAMBO: degenerate total momentum (M_Q ≈ 0)".into(),
            ));
        }

        // Step 3: Boost + scale to the CM frame with total energy = cms_energy.
        let x = cms_energy / m_q;
        let mut p: Vec<SpacetimeVector> = q
            .iter()
            .map(|qi| rambo_boost_and_scale(qi, &q_total, m_q, x))
            .collect();

        // Step 4: Compute the massless RAMBO weight.
        let massless_weight = rambo_massless_weight(n, cms_energy);

        // Step 5: Apply mass corrections if any particle is massive.
        let all_massless = final_masses.iter().all(|m| m.abs() < 1e-15);
        let weight = if all_massless {
            massless_weight
        } else {
            // Iterative rescaling via Newton's method to achieve target masses.
            let xi = rambo_mass_rescale(&p, final_masses, cms_energy)?;

            // Apply the rescaling: E_i = sqrt(|p_i|² * ξ² + m_i²), p_i → ξ * p_i_spatial
            let mut massive_momenta = Vec::with_capacity(n);
            for (i, pi) in p.iter().enumerate() {
                let p_spatial_sq: f64 = pi.components()[1..].iter().map(|c| c * c).sum();
                let m_i = final_masses[i];
                let e_new = (p_spatial_sq * xi * xi + m_i * m_i).sqrt();
                massive_momenta.push(SpacetimeVector::new_4d(
                    e_new,
                    pi[1] * xi,
                    pi[2] * xi,
                    pi[3] * xi,
                ));
            }

            // Compute mass correction weight factor.
            let mass_weight =
                rambo_mass_weight_factor(&massive_momenta, final_masses, xi, n, cms_energy);
            p = massive_momenta;
            massless_weight * mass_weight
        };

        Ok(PhaseSpacePoint { momenta: p, weight })
    }

    fn name(&self) -> &str {
        "RAMBO"
    }
}

impl RamboGenerator {
    /// Generate a single massless isotropic 4-momentum.
    ///
    /// The energy follows $E = -\ln(\rho_3 \cdot \rho_4)$ and the direction
    /// is uniformly distributed on the unit sphere.
    fn generate_massless_isotropic(&mut self) -> SpacetimeVector {
        let rho1: f64 = self.rng.gen();
        let rho2: f64 = self.rng.gen();
        let rho3: f64 = self.rng.gen::<f64>().max(1e-300);
        let rho4: f64 = self.rng.gen::<f64>().max(1e-300);

        let cos_theta = 2.0 * rho1 - 1.0;
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let phi = 2.0 * PI * rho2;

        let energy = -(rho3 * rho4).ln();

        SpacetimeVector::new_4d(
            energy,
            energy * sin_theta * phi.cos(),
            energy * sin_theta * phi.sin(),
            energy * cos_theta,
        )
    }
}

/// Sum a slice of spacetime vectors component-wise.
fn sum_vectors(vecs: &[SpacetimeVector]) -> SpacetimeVector {
    let dim = vecs[0].dimension();
    let mut result = SpacetimeVector::zero(dim);
    for v in vecs {
        for i in 0..dim {
            result[i] += v[i];
        }
    }
    result
}

/// Apply the RAMBO boost-and-scale transformation to a single momentum.
///
/// Maps the generated massless momentum $q_i$ into the CM frame where the
/// total 4-momentum is $(E_\text{cms}, 0, 0, 0)$.
///
/// The transformation combines:
/// 1. A Lorentz boost from the $Q$ rest frame to the CM frame.
/// 2. A uniform energy rescaling by factor $x = E_\text{cms}/M_Q$.
fn rambo_boost_and_scale(
    qi: &SpacetimeVector,
    q_total: &SpacetimeVector,
    m_q: f64,
    x: f64,
) -> SpacetimeVector {
    let q0 = q_total[0];
    let qx = q_total[1];
    let qy = q_total[2];
    let qz = q_total[3];

    // Boost vector: b = -Q_spatial / M_Q
    let bx = -qx / m_q;
    let by = -qy / m_q;
    let bz = -qz / m_q;
    let gamma = q0 / m_q;
    let a = 1.0 / (1.0 + gamma);

    let qi_e = qi[0];
    let qi_x = qi[1];
    let qi_y = qi[2];
    let qi_z = qi[3];

    // b · q_spatial
    let bq = bx * qi_x + by * qi_y + bz * qi_z;

    // Boosted momentum
    let e_new = gamma * qi_e + bq;
    let px_new = qi_x + bx * (a * bq + qi_e);
    let py_new = qi_y + by * (a * bq + qi_e);
    let pz_new = qi_z + bz * (a * bq + qi_e);

    // Scale by x = E_cms / M_Q
    SpacetimeVector::new_4d(x * e_new, x * px_new, x * py_new, x * pz_new)
}

/// Compute the flat massless RAMBO weight.
///
/// For $N$ massless particles at CM energy $E_\text{cms}$:
///
/// $$w_0 = \frac{(2\pi)^{4-3N} \, \pi^{N-1}}{(N-1)!\,(N-2)!}
///          \left(\frac{E_\text{cms}}{2}\right)^{2N-4}$$
fn rambo_massless_weight(n: usize, cms_energy: f64) -> f64 {
    // Lorentz-invariant N-body massless phase-space volume (LIPS):
    //
    //   Φ_N = E_cm^{2(N-2)} / [2 × (4π)^{2N-3} × (N-1)! × (N-2)!]
    //
    // Reference: Kleiss, Stirling & Ellis, Comp. Phys. Comm. 40 (1986) 359, eq. 4.
    let energy_power = cms_energy.powi(2 * n as i32 - 4);
    let four_pi_power = (4.0 * PI).powi(2 * n as i32 - 3);
    let fact_n1 = factorial(n - 1) as f64;
    let fact_n2 = factorial(n - 2) as f64;

    energy_power / (2.0 * four_pi_power * fact_n1 * fact_n2)
}

/// Compute the factorial $n!$.
fn factorial(n: usize) -> u64 {
    (1..=n as u64).product()
}

/// Solve for the mass rescaling parameter $\xi$ using Newton's method.
///
/// Finds $\xi > 0$ such that:
/// $$\sum_{i=1}^{N} \sqrt{|\vec{p}_i|^2 \xi^2 + m_i^2} = E_\text{cms}$$
///
/// Starting value: $\xi_0 = \sqrt{1 - (\sum m_i / E_\text{cms})^2}$.
fn rambo_mass_rescale(
    momenta: &[SpacetimeVector],
    masses: &[f64],
    cms_energy: f64,
) -> SpireResult<f64> {
    let n = momenta.len();
    let mass_sum: f64 = masses.iter().sum();

    // Initial guess
    let ratio = mass_sum / cms_energy;
    let mut xi = (1.0 - ratio * ratio).max(1e-10).sqrt();

    // Spatial momentum magnitudes squared
    let p_sq: Vec<f64> = momenta
        .iter()
        .map(|p| p.components()[1..].iter().map(|c| c * c).sum())
        .collect();

    // Newton iterations
    for _ in 0..100 {
        let mut f_val = -cms_energy;
        let mut f_deriv = 0.0;
        for i in 0..n {
            let e_i = (p_sq[i] * xi * xi + masses[i] * masses[i]).sqrt();
            f_val += e_i;
            if e_i > 1e-300 {
                f_deriv += p_sq[i] * xi / e_i;
            }
        }

        if f_val.abs() < 1e-12 * cms_energy {
            return Ok(xi);
        }

        if f_deriv.abs() < 1e-300 {
            return Err(crate::SpireError::InternalError(
                "RAMBO mass rescaling: zero derivative in Newton iteration".into(),
            ));
        }

        xi -= f_val / f_deriv;
        xi = xi.max(1e-15);
    }

    // Should converge very quickly (typically 3–5 iterations).
    Err(crate::SpireError::InternalError(
        "RAMBO mass rescaling failed to converge in 100 iterations".into(),
    ))
}

/// Compute the mass correction factor for the RAMBO weight.
///
/// The massive RAMBO weight is:
/// $$w = w_0 \cdot \xi^{2N-3} \cdot \frac{E_\text{cms}}
///        {\sum_i |\vec{p}_i|^2 / E_i} \cdot \prod_i \frac{|\vec{p}_i|}{E_i}$$
fn rambo_mass_weight_factor(
    massive_momenta: &[SpacetimeVector],
    _masses: &[f64],
    xi: f64,
    n: usize,
    cms_energy: f64,
) -> f64 {
    let mut product = 1.0;
    let mut sum_p_sq_over_e = 0.0;

    for p in massive_momenta.iter() {
        let e_i = p[0];
        let p_spatial_sq: f64 = p.components()[1..].iter().map(|c| c * c).sum();
        let p_mag = p_spatial_sq.sqrt();

        if e_i > 1e-300 {
            product *= p_mag / e_i;
            sum_p_sq_over_e += p_spatial_sq / e_i;
        }
    }

    if sum_p_sq_over_e < 1e-300 {
        return 0.0;
    }

    let xi_power = xi.powi(2 * n as i32 - 3);
    xi_power * cms_energy / sum_p_sq_over_e * product
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Existing serde/structural tests (preserved from Task 01)
    // -----------------------------------------------------------------------

    #[test]
    fn mandelstam_vars_serde() {
        let m = MandelstamVars {
            s: 100.0,
            t: -25.0,
            u: -75.0,
        };
        let json = serde_json::to_string(&m).unwrap();
        let m2: MandelstamVars = serde_json::from_str(&json).unwrap();
        assert_eq!(m.s, m2.s);
    }

    #[test]
    fn reference_frame_equality() {
        assert_eq!(ReferenceFrame::CentreOfMass, ReferenceFrame::CentreOfMass);
        assert_ne!(ReferenceFrame::CentreOfMass, ReferenceFrame::Lab);
    }

    #[test]
    fn dalitz_boundaries_serialize() {
        let db = DalitzBoundaries {
            mother_mass: 1.0,
            daughter_masses: [0.1, 0.2, 0.3],
            m_ab_sq_min: 0.09,
            m_ab_sq_max: 0.49,
            m_bc_sq_min: 0.25,
            m_bc_sq_max: 0.64,
        };
        let json = serde_json::to_string(&db).unwrap();
        assert!(json.contains("mother_mass"));
    }

    // -----------------------------------------------------------------------
    // 7.1: Mandelstam Invariants
    // -----------------------------------------------------------------------

    #[test]
    fn mandelstam_ee_to_mumu_cm_frame() {
        // e⁺e⁻ → μ⁺μ⁻ in the CM frame at √s = 200 GeV
        // Massless approximation for simplicity:
        //   p1 = (100, 0, 0, 100)   - e⁻ along +z
        //   p2 = (100, 0, 0, -100)  - e⁺ along -z
        //   p3 = (100, 50, 0, 86.6) - μ⁻ at θ ≈ 30°
        //   p4 = (100, -50, 0, -86.6) - μ⁺ back-to-back
        let p1 = FourMomentum::new(100.0, 0.0, 0.0, 100.0);
        let p2 = FourMomentum::new(100.0, 0.0, 0.0, -100.0);
        let p3 = FourMomentum::new(100.0, 50.0, 0.0, 86.6025403784);
        let p4 = FourMomentum::new(100.0, -50.0, 0.0, -86.6025403784);

        let m = compute_mandelstam(&p1, &p2, &p3, &p4);

        // s = (200)² = 40000
        assert!((m.s - 40000.0).abs() < 0.01, "s = {}", m.s);

        // For massless: s + t + u = 0
        let sum = m.s + m.t + m.u;
        assert!(
            sum.abs() < 0.01,
            "s + t + u = {} (expected ≈ 0 for massless)",
            sum
        );

        // t = (p1 - p3)²  - should be negative (space-like)
        assert!(m.t < 0.0, "t should be negative for scattering");
        // u = (p1 - p4)²  - should also be negative
        assert!(m.u < 0.0, "u should be negative for scattering");
    }

    #[test]
    fn mandelstam_sum_rule_massless() {
        let p1 = FourMomentum::new(100.0, 0.0, 0.0, 100.0);
        let p2 = FourMomentum::new(100.0, 0.0, 0.0, -100.0);
        let p3 = FourMomentum::new(100.0, 100.0, 0.0, 0.0);
        let p4 = FourMomentum::new(100.0, -100.0, 0.0, 0.0);

        let m = compute_mandelstam(&p1, &p2, &p3, &p4);
        let masses = [0.0_f64; 4];
        assert!(
            m.verify_sum_rule(&masses),
            "sum rule failed for massless: s+t+u = {}",
            m.s + m.t + m.u
        );
    }

    #[test]
    fn mandelstam_sum_rule_massive() {
        // e⁻e⁺ → μ⁻μ⁺ in the CM frame at √s ≈ 200 GeV
        // Both initial and final particles must be on-shell: E² = |p|² + m²
        let m_e: f64 = 0.000511;
        let m_mu: f64 = 0.10566;

        // Initial state: electrons along z-axis with |p| = 100 GeV
        let p_beam: f64 = 100.0;
        let e_e = (p_beam * p_beam + m_e * m_e).sqrt();
        let p1 = FourMomentum::new(e_e, 0.0, 0.0, p_beam);
        let p2 = FourMomentum::new(e_e, 0.0, 0.0, -p_beam);

        // Final state: muons back-to-back with |p*| from CM kinematics
        let s = (p1 + p2).invariant_mass_sq();
        let p_star = cm_momentum(s, m_mu, m_mu);
        let e_mu = (p_star * p_star + m_mu * m_mu).sqrt();

        // Scatter at angle θ in xz-plane
        let cos_theta: f64 = 0.6;
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let p3 = FourMomentum::new(e_mu, p_star * sin_theta, 0.0, p_star * cos_theta);
        let p4 = FourMomentum::new(e_mu, -p_star * sin_theta, 0.0, -p_star * cos_theta);

        let m = compute_mandelstam(&p1, &p2, &p3, &p4);
        let masses = [m_e, m_e, m_mu, m_mu];
        assert!(
            m.verify_sum_rule(&masses),
            "sum rule failed: s+t+u = {:.6}, Σm² = {:.6}",
            m.s + m.t + m.u,
            masses.iter().map(|x| x * x).sum::<f64>()
        );
    }

    #[test]
    fn mandelstam_forward_scattering() {
        // Forward scattering (θ = 0): p3 = p1, p4 = p2
        // t = (p1 - p3)² = 0, u = (p1 - p4)² = (p1 - p2)²
        let p1 = FourMomentum::new(100.0, 0.0, 0.0, 100.0);
        let p2 = FourMomentum::new(100.0, 0.0, 0.0, -100.0);

        let m = compute_mandelstam(&p1, &p2, &p1, &p2);
        assert!(m.t.abs() < 1e-10, "forward: t = {} (expected 0)", m.t);
        assert!((m.s - 40000.0).abs() < 1e-8, "s = {}", m.s);
        assert!((m.u + 40000.0).abs() < 1e-8, "u = {} (expected -s)", m.u);
    }

    // -----------------------------------------------------------------------
    // 7.2: Källén Function and CM Momentum
    // -----------------------------------------------------------------------

    #[test]
    fn kallen_lambda_at_threshold() {
        // At threshold: √s = m_a + m_b → s = (m_a + m_b)²
        // λ((m_a + m_b)², m_a², m_b²) should be 0
        let m_a = 0.938; // proton
        let m_b = 0.140; // pion
        let s_thr = (m_a + m_b) * (m_a + m_b);
        let lam = kallen_lambda(s_thr, m_a * m_a, m_b * m_b);
        assert!(lam.abs() < 1e-10, "λ at threshold = {} (expected 0)", lam);
    }

    #[test]
    fn kallen_lambda_equal_masses() {
        // λ(s, m², m²) = s² + 2m⁴ - 2s·m² - 2s·m²  (simplifies)
        // = s² - 4s·m² + 4m⁴ = ... wait let's be precise:
        // λ(s, m², m²) = s² + m⁴ + m⁴ - 2s·m² - 2s·m² - 2m⁴ = s² - 4sm² + 4m⁴ = ...
        // Actually: x=s, y=z=m² → λ = s² + 2m⁴ - 4sm²
        // Wait: λ = s² + m⁴ + m⁴ - 2s·m² - 2s·m² - 2m²·m² = s² + 2m⁴ - 4sm² - 2m⁴ = s² - 4sm²
        // Hmm: s² + m⁴ + m⁴ - 2s·m² - 2s·m² - 2·m²·m² = s² + 2m⁴ - 4sm² - 2m⁴ = s² - 4sm²
        // = s(s - 4m²)
        let m = 1.0;
        let s = 10.0;
        let lam = kallen_lambda(s, m * m, m * m);
        let expected = s * (s - 4.0 * m * m); // s(s - 4m²)
        assert!(
            (lam - expected).abs() < 1e-10,
            "λ = {}, expected {}",
            lam,
            expected
        );
    }

    #[test]
    fn kallen_lambda_one_massless() {
        // λ(s, 0, m²) = s² + m⁴ - 2sm² = (s - m²)²
        let s = 100.0;
        let m_sq = 25.0;
        let lam = kallen_lambda(s, 0.0, m_sq);
        let expected = (s - m_sq) * (s - m_sq);
        assert!(
            (lam - expected).abs() < 1e-10,
            "λ = {}, expected {}",
            lam,
            expected
        );
    }

    #[test]
    fn kallen_lambda_symmetric() {
        // λ is symmetric under permutation of arguments
        let (x, y, z) = (100.0, 4.0, 9.0);
        let l1 = kallen_lambda(x, y, z);
        let l2 = kallen_lambda(y, z, x);
        let l3 = kallen_lambda(z, x, y);
        assert!((l1 - l2).abs() < 1e-10, "not symmetric: λ(x,y,z)≠λ(y,z,x)");
        assert!((l1 - l3).abs() < 1e-10, "not symmetric: λ(x,y,z)≠λ(z,x,y)");
    }

    #[test]
    fn cm_momentum_massless_decay() {
        // H → γγ: both massless → p* = M_H/2
        let m_h = 125.1;
        let s = m_h * m_h;
        let p_star = cm_momentum(s, 0.0, 0.0);
        assert!(
            (p_star - m_h / 2.0).abs() < 1e-10,
            "p* = {} (expected {})",
            p_star,
            m_h / 2.0
        );
    }

    #[test]
    fn cm_momentum_equal_masses() {
        // M → m + m: p* = √(M²/4 - m²)
        let big_m = 10.0;
        let m = 3.0;
        let s = big_m * big_m;
        let p_star = cm_momentum(s, m, m);
        let expected = (big_m * big_m / 4.0 - m * m).sqrt();
        assert!(
            (p_star - expected).abs() < 1e-10,
            "p* = {} (expected {})",
            p_star,
            expected
        );
    }

    #[test]
    fn cm_momentum_at_threshold() {
        // At threshold: √s = m_a + m_b → p* = 0
        let m_a = 0.938;
        let m_b = 0.140;
        let s_thr = (m_a + m_b) * (m_a + m_b);
        let p_star = cm_momentum(s_thr, m_a, m_b);
        assert!(
            p_star.abs() < 1e-10,
            "p* at threshold = {} (expected 0)",
            p_star
        );
    }

    #[test]
    fn cm_momentum_below_threshold() {
        // Below threshold: should return 0
        let m_a = 10.0;
        let m_b = 10.0;
        let s = 100.0; // √s = 10 < m_a + m_b = 20
        let p_star = cm_momentum(s, m_a, m_b);
        assert_eq!(p_star, 0.0, "below threshold should return 0.0");
    }

    #[test]
    fn cm_momentum_unequal_masses() {
        // General case: e⁺e⁻ → μ⁺μ⁻ at √s = 200 GeV
        // μ mass = 0.10566 GeV → negligible correction
        let s = 40000.0;
        let m_mu = 0.10566;
        let p_star = cm_momentum(s, m_mu, m_mu);
        // p* ≈ 200/2 = 100 GeV minus tiny mass correction
        let expected = (s / 4.0 - m_mu * m_mu).sqrt();
        assert!(
            (p_star - expected).abs() < 1e-6,
            "p* = {} (expected {})",
            p_star,
            expected
        );
    }

    // -----------------------------------------------------------------------
    // 7.2 (cont): 2-Body Phase Space
    // -----------------------------------------------------------------------

    #[test]
    fn two_body_phase_space_massless() {
        // H → γγ: Φ₂ = p*/(8π√s) = (M_H/2)/(8π·M_H) = 1/(16π)
        let m_h = 125.1;
        let s = m_h * m_h;
        let phi2 = two_body_phase_space(s, 0.0, 0.0);
        let expected = 1.0 / (16.0 * PI);
        assert!(
            (phi2 - expected).abs() < 1e-10,
            "Φ₂ = {} (expected {})",
            phi2,
            expected
        );
    }

    #[test]
    fn two_body_phase_space_below_threshold() {
        let phi2 = two_body_phase_space(1.0, 5.0, 5.0);
        assert_eq!(phi2, 0.0);
    }

    #[test]
    fn two_body_phase_space_at_threshold() {
        // At threshold: p* = 0 → Φ₂ = 0
        let m = 5.0;
        let s = (2.0 * m) * (2.0 * m);
        let phi2 = two_body_phase_space(s, m, m);
        assert!(
            phi2.abs() < 1e-10,
            "Φ₂ at threshold = {} (expected 0)",
            phi2
        );
    }

    // -----------------------------------------------------------------------
    // 7.3: Dalitz Plot Boundaries
    // -----------------------------------------------------------------------

    #[test]
    fn dalitz_boundaries_basic() {
        // D⁰ → K⁻π⁺π⁰  (approximate masses)
        // M_D ≈ 1.865, m_K ≈ 0.494, m_π⁺ ≈ 0.140, m_π⁰ ≈ 0.135
        let m_d = 1.865;
        let masses = [0.494, 0.140, 0.135];

        let db = generate_dalitz_boundaries(m_d, masses).unwrap();

        // s_ab_min = (m_K + m_π⁺)² = (0.634)²
        let expected_ab_min = (0.494 + 0.140) * (0.494 + 0.140);
        assert!(
            (db.m_ab_sq_min - expected_ab_min).abs() < 1e-6,
            "s_ab_min = {} (expected {})",
            db.m_ab_sq_min,
            expected_ab_min
        );

        // s_ab_max = (M_D - m_π⁰)² = (1.730)²
        let expected_ab_max = (m_d - 0.135) * (m_d - 0.135);
        assert!(
            (db.m_ab_sq_max - expected_ab_max).abs() < 1e-6,
            "s_ab_max = {} (expected {})",
            db.m_ab_sq_max,
            expected_ab_max
        );

        // s_bc_min = (m_π⁺ + m_π⁰)² = (0.275)²
        let expected_bc_min = (0.140 + 0.135) * (0.140 + 0.135);
        assert!((db.m_bc_sq_min - expected_bc_min).abs() < 1e-6);

        // s_bc_max = (M_D - m_K)² = (1.371)²
        let expected_bc_max = (m_d - 0.494) * (m_d - 0.494);
        assert!((db.m_bc_sq_max - expected_bc_max).abs() < 1e-6);
    }

    #[test]
    fn dalitz_boundaries_forbidden() {
        // M < m_a + m_b + m_c → should error
        let result = generate_dalitz_boundaries(0.5, [0.3, 0.3, 0.3]);
        assert!(result.is_err(), "should fail when M < sum of masses");
    }

    #[test]
    fn dalitz_equal_masses() {
        // M → 3 equal masses: τ → 3π (approximate)
        let m_tau = 1.777;
        let m_pi = 0.140;

        let db = generate_dalitz_boundaries(m_tau, [m_pi, m_pi, m_pi]).unwrap();

        // All min should be (2m_π)² = (0.280)²
        let expected_min = (2.0 * m_pi) * (2.0 * m_pi);
        assert!((db.m_ab_sq_min - expected_min).abs() < 1e-8);
        assert!((db.m_bc_sq_min - expected_min).abs() < 1e-8);

        // All max should be (M - m_π)²
        let expected_max = (m_tau - m_pi) * (m_tau - m_pi);
        assert!((db.m_ab_sq_max - expected_max).abs() < 1e-8);
        assert!((db.m_bc_sq_max - expected_max).abs() < 1e-8);
    }

    #[test]
    fn dalitz_s_bc_limits_at_midpoint() {
        // For M → a + b + c with equal masses, evaluate s_bc limits at
        // the midpoint of s_ab range.
        let big_m = 3.0;
        let m = 0.5;

        let db = generate_dalitz_boundaries(big_m, [m, m, m]).unwrap();
        let s_ab_mid = (db.m_ab_sq_min + db.m_ab_sq_max) / 2.0;

        let limits = dalitz_s_bc_limits_at_s_ab(big_m, m, m, m, s_ab_mid);
        assert!(limits.is_some(), "should return valid limits at midpoint");

        let (s_bc_min, s_bc_max) = limits.unwrap();
        // Physical region must satisfy s_bc_min < s_bc_max
        assert!(
            s_bc_min < s_bc_max + 1e-10,
            "s_bc_min = {} > s_bc_max = {}",
            s_bc_min,
            s_bc_max
        );
        // Both should be within absolute Dalitz boundaries
        assert!(s_bc_min >= db.m_bc_sq_min - 1e-8);
        assert!(s_bc_max <= db.m_bc_sq_max + 1e-8);
    }

    #[test]
    fn dalitz_s_bc_limits_at_threshold() {
        // At s_ab = s_ab_min = (m_a + m_b)², particles a and b are at rest
        // relative to each other. s_bc should have a single value (thin boundary).
        let big_m = 3.0;
        let m = 0.5;

        let s_ab_min = (m + m) * (m + m);
        let limits = dalitz_s_bc_limits_at_s_ab(big_m, m, m, m, s_ab_min);
        assert!(limits.is_some());

        let (s_bc_min, s_bc_max) = limits.unwrap();
        // At s_ab threshold, b has zero momentum in (a,b) frame → s_bc_min ≈ s_bc_max
        // (the Dalitz boundary collapses to a point at the corner)
        assert!(
            (s_bc_max - s_bc_min).abs() < 1e-6,
            "at s_ab threshold: s_bc range = [{}, {}] should be degenerate",
            s_bc_min,
            s_bc_max
        );
    }

    // -----------------------------------------------------------------------
    // 12.1: Dalitz Plot Data Generation
    // -----------------------------------------------------------------------

    #[test]
    fn dalitz_plot_data_basic() {
        // D⁰ → K⁻π⁺π⁰  (approximate masses)
        let m_d = 1.865;
        let data = generate_dalitz_plot_data(m_d, 0.494, 0.140, 0.135, 3000).unwrap();

        assert!(
            !data.points.is_empty(),
            "should produce non-empty point set"
        );
        assert!(data.n_grid > 1, "grid should have multiple divisions");
        assert!((data.boundaries.mother_mass - m_d).abs() < 1e-12);

        // Every point must lie within the absolute Dalitz boundaries.
        for &(s_ab, s_bc) in &data.points {
            assert!(
                s_ab >= data.boundaries.m_ab_sq_min - 1e-10,
                "s_ab = {} below min {}",
                s_ab,
                data.boundaries.m_ab_sq_min
            );
            assert!(
                s_ab <= data.boundaries.m_ab_sq_max + 1e-10,
                "s_ab = {} above max {}",
                s_ab,
                data.boundaries.m_ab_sq_max
            );
            assert!(
                s_bc >= data.boundaries.m_bc_sq_min - 1e-10,
                "s_bc = {} below min {}",
                s_bc,
                data.boundaries.m_bc_sq_min
            );
            assert!(
                s_bc <= data.boundaries.m_bc_sq_max + 1e-10,
                "s_bc = {} above max {}",
                s_bc,
                data.boundaries.m_bc_sq_max
            );
        }
    }

    #[test]
    fn dalitz_plot_data_points_within_local_boundary() {
        // Check that each point lies within the s_bc limits at its s_ab slice.
        let big_m = 3.0;
        let m = 0.5;
        let data = generate_dalitz_plot_data(big_m, m, m, m, 2000).unwrap();

        for &(s_ab, s_bc) in &data.points {
            let limits = dalitz_s_bc_limits_at_s_ab(big_m, m, m, m, s_ab);
            assert!(limits.is_some(), "s_ab = {} should have valid limits", s_ab);
            let (lo, hi) = limits.unwrap();
            assert!(
                s_bc >= lo - 1e-10 && s_bc <= hi + 1e-10,
                "s_bc = {} outside [{}, {}] at s_ab = {}",
                s_bc,
                lo,
                hi,
                s_ab
            );
        }
    }

    #[test]
    fn dalitz_plot_data_forbidden_decay() {
        // M < m_a + m_b + m_c → should error
        let result = generate_dalitz_plot_data(0.5, 0.3, 0.3, 0.3, 1000);
        assert!(result.is_err(), "forbidden decay should return error");
    }

    #[test]
    fn dalitz_plot_data_respects_n_points() {
        // With small n_points, we should still get a reasonable number of points
        let data = generate_dalitz_plot_data(1.865, 0.494, 0.140, 0.135, 100).unwrap();
        assert!(
            data.points.len() > 0 && data.points.len() <= 200,
            "100 requested → got {} points",
            data.points.len()
        );

        // With large n_points, we should get more
        let data_large = generate_dalitz_plot_data(1.865, 0.494, 0.140, 0.135, 10000).unwrap();
        assert!(
            data_large.points.len() > data.points.len(),
            "10000 requested ({}) should produce more than 100 requested ({})",
            data_large.points.len(),
            data.points.len()
        );
    }

    #[test]
    fn dalitz_plot_data_equal_masses_symmetric() {
        // For equal daughter masses, the Dalitz plot should be approximately
        // symmetric: the centroid of all points should be near the centre of the plot.
        let big_m = 3.0;
        let m = 0.5;
        let data = generate_dalitz_plot_data(big_m, m, m, m, 5000).unwrap();

        let n = data.points.len() as f64;
        let mean_s_ab: f64 = data.points.iter().map(|&(x, _)| x).sum::<f64>() / n;
        let mean_s_bc: f64 = data.points.iter().map(|&(_, y)| y).sum::<f64>() / n;

        // For equal masses, the centre of the Dalitz plot in s_ab and s_bc
        // should be identical (by relabelling symmetry a ↔ c).
        let s_ab_centre = (data.boundaries.m_ab_sq_min + data.boundaries.m_ab_sq_max) / 2.0;
        let s_bc_centre = (data.boundaries.m_bc_sq_min + data.boundaries.m_bc_sq_max) / 2.0;

        // The mean should be within ~10% of the midpoint.
        let range_ab = data.boundaries.m_ab_sq_max - data.boundaries.m_ab_sq_min;
        assert!(
            (mean_s_ab - s_ab_centre).abs() < 0.15 * range_ab,
            "mean s_ab = {:.4} far from centre {:.4}",
            mean_s_ab,
            s_ab_centre
        );
        let range_bc = data.boundaries.m_bc_sq_max - data.boundaries.m_bc_sq_min;
        assert!(
            (mean_s_bc - s_bc_centre).abs() < 0.15 * range_bc,
            "mean s_bc = {:.4} far from centre {:.4}",
            mean_s_bc,
            s_bc_centre
        );
    }

    // -----------------------------------------------------------------------
    // 7.4: Reaction Thresholds
    // -----------------------------------------------------------------------

    #[test]
    fn threshold_top_pair_production() {
        // tt̄ production threshold: √s_min = 2 × m_top ≈ 2 × 173.0 = 346.0 GeV
        let m_top = 173.0;
        let result = calculate_thresholds(&[m_top, m_top], None).unwrap();
        assert!((result.threshold_energy - 346.0).abs() < 1e-6);
    }

    #[test]
    fn threshold_too_low_energy() {
        // Trying to produce top quarks at 10 GeV → not allowed
        let m_top = 173.0;
        let allowed = is_kinematically_allowed(10.0, &[m_top, m_top]);
        assert!(!allowed, "10 GeV should not produce tt̄");
    }

    #[test]
    fn threshold_massless_final_state() {
        // γγ → threshold = 0
        let result = calculate_thresholds(&[0.0, 0.0], None).unwrap();
        assert!((result.threshold_energy).abs() < 1e-12);
    }

    #[test]
    fn threshold_with_fixed_target() {
        // pp collision: beam proton hitting stationary proton target
        // to produce pp + pp̄ (4 protons' worth of mass)
        // s_min = (4 × 0.938)² = (3.752)² ≈ 14.08
        // E_lab = (s_min - 2m_p²) / (2m_p) for m_beam = m_target = m_p
        // But our simplified formula: E_lab = (s_min - m_t²) / (2m_t)
        let m_p = 0.938;
        let result = calculate_thresholds(&[m_p, m_p, m_p, m_p], Some(m_p)).unwrap();

        let s_min = (4.0 * m_p) * (4.0 * m_p);
        let expected_lab = (s_min - m_p * m_p) / (2.0 * m_p);
        assert!(result.lab_energy.is_some());
        assert!(
            (result.lab_energy.unwrap() - expected_lab).abs() < 1e-6,
            "E_lab = {} (expected {})",
            result.lab_energy.unwrap(),
            expected_lab
        );
    }

    #[test]
    fn kinematically_allowed_muon_pair() {
        // e⁺e⁻ → μ⁺μ⁻: need √s ≥ 2m_μ ≈ 0.211 GeV
        let m_mu = 0.10566;
        assert!(is_kinematically_allowed(200.0, &[m_mu, m_mu]));
        assert!(is_kinematically_allowed(0.22, &[m_mu, m_mu]));
        assert!(!is_kinematically_allowed(0.10, &[m_mu, m_mu]));
    }

    // -----------------------------------------------------------------------
    // 7.2 (cont): Mandelstam Boundaries
    // -----------------------------------------------------------------------

    #[test]
    fn mandelstam_boundaries_massless() {
        // All massless: s_min = 0, t in [-s, 0]
        let bounds = compute_mandelstam_boundaries([0.0; 4], 100.0).unwrap();
        assert!((bounds.s_min).abs() < 1e-12, "s_min = {}", bounds.s_min);
        assert!(
            (bounds.t_min + 100.0).abs() < 1e-8,
            "t_min = {} (expected -100)",
            bounds.t_min
        );
        assert!(
            (bounds.t_max).abs() < 1e-8,
            "t_max = {} (expected 0)",
            bounds.t_max
        );
    }

    #[test]
    fn mandelstam_boundaries_at_threshold() {
        // At threshold s = (m3 + m4)²: p*_f = 0 → t_min = t_max (degenerate)
        let m3 = 0.938;
        let m4 = 0.140;
        let s_thr = (m3 + m4) * (m3 + m4);
        let bounds = compute_mandelstam_boundaries([0.0, 0.0, m3, m4], s_thr).unwrap();
        assert!(
            (bounds.t_max - bounds.t_min).abs() < 1e-6,
            "at threshold: t_min={} t_max={} should be degenerate",
            bounds.t_min,
            bounds.t_max
        );
    }

    #[test]
    fn mandelstam_boundaries_below_threshold() {
        // Below threshold: should return degenerate boundaries
        let bounds = compute_mandelstam_boundaries([0.0, 0.0, 5.0, 5.0], 1.0).unwrap();
        assert_eq!(bounds.t_min, 0.0);
        assert_eq!(bounds.t_max, 0.0);
    }

    // -----------------------------------------------------------------------
    // Lorentz Boost Tests
    // -----------------------------------------------------------------------

    #[test]
    fn lorentz_boost_identity() {
        // Zero boost should return the same momentum
        let p = FourMomentum::new(10.0, 1.0, 2.0, 3.0);
        let boost = LorentzBoost {
            beta: [0.0, 0.0, 0.0],
            gamma: 1.0,
        };
        let p_prime = apply_lorentz_boost(&p, &boost).unwrap();
        assert!((p_prime.e - p.e).abs() < 1e-12);
        assert!((p_prime.px - p.px).abs() < 1e-12);
        assert!((p_prime.py - p.py).abs() < 1e-12);
        assert!((p_prime.pz - p.pz).abs() < 1e-12);
    }

    #[test]
    fn lorentz_boost_preserves_invariant_mass() {
        // A Lorentz boost must preserve p² = m²
        let m: f64 = 0.938;
        let p = FourMomentum::new(10.0, 0.0, 0.0, (100.0_f64 - m * m).sqrt());
        let m_sq_original = p.invariant_mass_sq();

        let beta_z: f64 = 0.5;
        let gamma = 1.0 / (1.0 - beta_z * beta_z).sqrt();
        let boost = LorentzBoost {
            beta: [0.0, 0.0, beta_z],
            gamma,
        };

        let p_prime = apply_lorentz_boost(&p, &boost).unwrap();
        let m_sq_boosted = p_prime.invariant_mass_sq();

        assert!(
            (m_sq_original - m_sq_boosted).abs() < 1e-8,
            "p² changed: {:.6} → {:.6}",
            m_sq_original,
            m_sq_boosted
        );
    }

    #[test]
    fn lorentz_boost_preserves_small_invariant_under_near_collinear_cancellation() {
        // Regression-style stress case: tiny invariant mass with large energy/momentum.
        // This is numerically delicate because E² and |p|² are both O(1e6) while
        // their difference is O(1e-6).
        let m_sq: f64 = 1.0e-6;
        let pz: f64 = 1_000.0;
        let e = (pz * pz + m_sq).sqrt();
        let p = FourMomentum::new(e, 0.2, -0.15, pz);

        let beta: [f64; 3] = [0.31, -0.28, 0.44];
        let beta_sq = beta[0] * beta[0] + beta[1] * beta[1] + beta[2] * beta[2];
        let gamma = 1.0 / (1.0 - beta_sq).sqrt();
        let boost = LorentzBoost { beta, gamma };

        let p_sq = p.invariant_mass_sq();
        let boosted = apply_lorentz_boost(&p, &boost).unwrap();
        let boosted_sq = boosted.invariant_mass_sq();

        let scale = p_sq.abs().max(boosted_sq.abs()).max(1.0);
        let diff = (boosted_sq - p_sq).abs();

        assert!(
            diff < 1e-9 * scale,
            "small-invariant Lorentz drift: p²={:.12e}, p'²={:.12e}, diff={:.3e}",
            p_sq,
            boosted_sq,
            diff
        );
    }

    #[test]
    fn lorentz_boost_to_rest_frame() {
        // Boost a particle to its rest frame: β = p/E along momentum direction
        let m: f64 = 0.938;
        let pz: f64 = 5.0;
        let e = (m * m + pz * pz).sqrt();
        let p = FourMomentum::new(e, 0.0, 0.0, pz);

        let beta_z = pz / e;
        let gamma = 1.0 / (1.0 - beta_z * beta_z).sqrt();
        let boost = LorentzBoost {
            beta: [0.0, 0.0, beta_z],
            gamma,
        };

        let p_rest = apply_lorentz_boost(&p, &boost).unwrap();

        // In rest frame: E = m, p = 0
        assert!(
            (p_rest.e - m).abs() < 1e-8,
            "E_rest = {} (expected {})",
            p_rest.e,
            m
        );
        assert!(
            p_rest.pz.abs() < 1e-8,
            "pz_rest = {} (expected 0)",
            p_rest.pz
        );
    }

    #[test]
    fn compute_cm_boost_basic() {
        // 10 GeV proton beam on stationary proton target
        let m_p: f64 = 0.938;
        let pz: f64 = 10.0;
        let e = (m_p * m_p + pz * pz).sqrt();
        let beam = FourMomentum::new(e, 0.0, 0.0, pz);

        let boost = compute_cm_boost(&beam, m_p).unwrap();

        // β_CM = p_beam / (E_beam + m_target)
        let expected_beta = pz / (e + m_p);
        assert!((boost.beta[2] - expected_beta).abs() < 1e-10);
        assert!(boost.gamma > 1.0);
        assert!(boost.beta[0].abs() < 1e-15);
        assert!(boost.beta[1].abs() < 1e-15);
    }

    // -----------------------------------------------------------------------
    // Phase Space Generation
    // -----------------------------------------------------------------------

    #[test]
    fn phase_space_two_body() {
        let ps = generate_phase_space(200.0, &[0.10566, 0.10566]).unwrap();
        assert_eq!(ps.n_final, 2);
        assert_eq!(ps.n_variables, 2); // 3×2 - 4 = 2
        assert!(ps.measure_expression.contains("Φ₂"));
    }

    #[test]
    fn phase_space_three_body() {
        let ps = generate_phase_space(2.0, &[0.140, 0.140, 0.140]).unwrap();
        assert_eq!(ps.n_final, 3);
        assert_eq!(ps.n_variables, 5); // 3×3 - 4 = 5
    }

    #[test]
    fn phase_space_insufficient_energy() {
        let result = generate_phase_space(0.1, &[1.0, 1.0]);
        assert!(result.is_err());
    }

    #[test]
    fn phase_space_single_particle() {
        // 1-body: trivial (decay at rest)
        let ps = generate_phase_space(1.0, &[0.938]).unwrap();
        assert_eq!(ps.n_final, 1);
        assert_eq!(ps.n_variables, 0);
    }

    // -----------------------------------------------------------------------
    // RAMBO Phase Space Generator
    // -----------------------------------------------------------------------

    #[test]
    fn rambo_massless_two_body_momentum_conservation() {
        // Generate 200 events and verify 4-momentum conservation for each.
        let mut gen = RamboGenerator::new();
        let cms = 200.0;
        let masses = [0.0, 0.0];

        for _ in 0..200 {
            let event = gen.generate_event(cms, &masses).unwrap();
            assert_eq!(event.momenta.len(), 2);

            // Sum all final-state momenta.
            let total = sum_vectors(&event.momenta);

            // Should equal (E_cms, 0, 0, 0).
            assert!(
                (total[0] - cms).abs() < 1e-8,
                "Energy not conserved: E_total = {}, expected {}",
                total[0],
                cms
            );
            for j in 1..4 {
                assert!(
                    total[j].abs() < 1e-8,
                    "Spatial momentum not conserved: p[{}] = {}",
                    j,
                    total[j]
                );
            }

            // Weight should be positive.
            assert!(event.weight > 0.0, "Weight should be positive");
        }
    }

    #[test]
    fn rambo_massless_four_body_momentum_conservation() {
        // 4-body massless: 1000 events with strict conservation.
        let mut gen = RamboGenerator::new();
        let cms = 500.0;
        let masses = [0.0, 0.0, 0.0, 0.0];

        for _ in 0..1000 {
            let event = gen.generate_event(cms, &masses).unwrap();
            assert_eq!(event.momenta.len(), 4);

            let total = sum_vectors(&event.momenta);

            assert!(
                (total[0] - cms).abs() < 1e-8,
                "Energy not conserved: E = {}",
                total[0]
            );
            for j in 1..4 {
                assert!(
                    total[j].abs() < 1e-8,
                    "3-momentum not conserved: p[{}] = {}",
                    j,
                    total[j]
                );
            }

            // Each massless particle should satisfy E = |p|.
            for p in &event.momenta {
                let p_mag: f64 = p.components()[1..]
                    .iter()
                    .map(|c| c * c)
                    .sum::<f64>()
                    .sqrt();
                assert!(
                    (p[0] - p_mag).abs() < 1e-8,
                    "Massless particle not on-shell: E = {}, |p| = {}",
                    p[0],
                    p_mag
                );
            }
        }
    }

    #[test]
    fn rambo_massive_two_body_conservation_and_mass_shell() {
        // e⁺e⁻ → μ⁺μ⁻ at 200 GeV with m_μ = 0.10566 GeV.
        let mut gen = RamboGenerator::new();
        let cms = 200.0;
        let m_mu = 0.10566;
        let masses = [m_mu, m_mu];
        let metric = MetricSignature::minkowski_4d();

        for _ in 0..500 {
            let event = gen.generate_event(cms, &masses).unwrap();
            assert_eq!(event.momenta.len(), 2);

            let total = sum_vectors(&event.momenta);

            // Energy-momentum conservation.
            assert!(
                (total[0] - cms).abs() < 1e-6,
                "Energy: {} vs {}",
                total[0],
                cms
            );
            for j in 1..4 {
                assert!(total[j].abs() < 1e-6, "p[{}] = {}", j, total[j]);
            }

            // On-shell check: p² = m².
            for (i, p) in event.momenta.iter().enumerate() {
                let p_sq = metric.inner_product(p, p).unwrap();
                assert!(
                    (p_sq - masses[i] * masses[i]).abs() < 1e-4,
                    "Particle {} off-shell: p² = {}, m² = {}",
                    i,
                    p_sq,
                    masses[i] * masses[i]
                );
            }

            assert!(event.weight > 0.0);
        }
    }

    #[test]
    fn rambo_massive_three_body_conservation() {
        // 3-body decay at 2 GeV with pion-like masses.
        let mut gen = RamboGenerator::new();
        let cms = 2.0;
        let masses = [0.140, 0.140, 0.494];
        let metric = MetricSignature::minkowski_4d();

        for _ in 0..500 {
            let event = gen.generate_event(cms, &masses).unwrap();
            assert_eq!(event.momenta.len(), 3);

            let total = sum_vectors(&event.momenta);
            assert!((total[0] - cms).abs() < 1e-6, "E = {}", total[0]);
            for j in 1..4 {
                assert!(total[j].abs() < 1e-6);
            }

            // Mass shell.
            for (i, p) in event.momenta.iter().enumerate() {
                let p_sq = metric.inner_product(p, p).unwrap();
                assert!(
                    (p_sq - masses[i] * masses[i]).abs() < 1e-4,
                    "Particle {} off-shell: p²={}, m²={}",
                    i,
                    p_sq,
                    masses[i] * masses[i]
                );
            }
        }
    }

    #[test]
    fn rambo_insufficient_energy_error() {
        let mut gen = RamboGenerator::new();
        let result = gen.generate_event(0.1, &[1.0, 1.0]);
        assert!(result.is_err());
    }

    #[test]
    fn rambo_single_particle_error() {
        let mut gen = RamboGenerator::new();
        let result = gen.generate_event(1.0, &[0.5]);
        assert!(result.is_err());
    }

    #[test]
    fn rambo_weight_positive_and_finite() {
        let mut gen = RamboGenerator::new();
        for _ in 0..100 {
            let event = gen.generate_event(100.0, &[0.0, 0.0, 0.0]).unwrap();
            assert!(event.weight > 0.0);
            assert!(event.weight.is_finite());
        }
    }

    #[test]
    fn rambo_generator_name() {
        let gen = RamboGenerator::new();
        assert_eq!(gen.name(), "RAMBO");
    }

    #[test]
    fn rambo_massless_weight_formula_two_body() {
        // For N=2 massless particles:
        // Φ₂ = E^0 / [2 × (4π)^1 × 1! × 0!] = 1 / (8π)
        let w = rambo_massless_weight(2, 100.0);
        let expected = 1.0 / (8.0 * PI);
        assert!(
            (w - expected).abs() < 1e-10,
            "RAMBO 2-body weight: {} vs expected {}",
            w,
            expected
        );
    }

    #[test]
    fn rambo_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }

    #[test]
    fn phase_space_point_serde() {
        let point = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 30.0, 0.0, 40.0),
                SpacetimeVector::new_4d(50.0, -30.0, 0.0, -40.0),
            ],
            weight: 0.123,
        };
        let json = serde_json::to_string(&point).unwrap();
        let back: PhaseSpacePoint = serde_json::from_str(&json).unwrap();
        assert_eq!(back.momenta.len(), 2);
        assert!((back.weight - 0.123).abs() < 1e-12);
    }
}
