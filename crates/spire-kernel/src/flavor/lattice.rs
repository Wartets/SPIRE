//! # Lattice QCD Data Structures
//!
//! Provides types and methods for ingesting non-perturbative hadronic
//! quantities computed on the lattice, such as meson decay constants and
//! transition form factors.
//!
//! ## Form Factor Parameterization
//!
//! Semi-leptonic $B \to K$ (and similar) transitions are described by
//! form factors $f_+(q^2)$, $f_0(q^2)$, and $f_T(q^2)$ that encode
//! the hadronic matrix element of the quark current between the initial
//! and final meson states.
//!
//! These form factors are parameterized using the BCL (Bourrely–Caprini–
//! Lellouch) $z$-expansion, which maps the momentum transfer $q^2$ to a
//! conformal variable $z$ that lies inside the unit disc:
//!
//! $$z(q^2, t_0) = \frac{\sqrt{t_+ - q^2} - \sqrt{t_+ - t_0}}
//!                       {\sqrt{t_+ - q^2} + \sqrt{t_+ - t_0}}$$
//!
//! The form factor is then expanded as a convergent power series in $z$:
//!
//! $$f(q^2) = \frac{1}{1 - q^2 / m_{\text{pole}}^2}
//!            \sum_{k=0}^{K} a_k \, z^k$$
//!
//! where $m_{\text{pole}}$ is the mass of the nearest sub-threshold
//! resonance and the $a_k$ are fit coefficients determined from lattice
//! simulations (e.g., by the FLAG collaboration).
//!
//! ## Usage
//!
//! ```rust
//! use spire_kernel::flavor::lattice::{
//!     DecayConstant, FormFactorZExpansion, BToKFormFactors,
//! };
//!
//! let f_b = DecayConstant::new(0.1903, 0.0013, 4.18);
//! assert!((f_b.value - 0.1903).abs() < 1e-10);
//!
//! let ff = BToKFormFactors::flag_defaults();
//! let f_plus_0 = ff.f_plus.evaluate(0.0);
//! assert!(f_plus_0 > 0.0);
//! ```

use serde::{Deserialize, Serialize};

// ===========================================================================
// Decay Constants
// ===========================================================================

/// A hadronic decay constant from Lattice QCD.
///
/// Encapsulates the central value, statistical+systematic uncertainty,
/// and the renormalization scale $\mu$ at which the constant is defined.
///
/// # Examples
///
/// ```rust
/// use spire_kernel::flavor::lattice::DecayConstant;
///
/// // FLAG 2023 average for f_Bs
/// let f_bs = DecayConstant::new(0.2303, 0.0013, 4.18);
/// assert!((f_bs.value - 0.2303).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConstant {
    /// Central value in GeV.
    pub value: f64,
    /// Combined uncertainty (1σ) in GeV.
    pub error: f64,
    /// Renormalization scale μ in GeV.
    pub scale_mu: f64,
}

impl DecayConstant {
    /// Create a new decay constant.
    pub fn new(value: f64, error: f64, scale_mu: f64) -> Self {
        Self {
            value,
            error,
            scale_mu,
        }
    }

    /// FLAG 2023 average for $f_B$ (charged $B$ meson).
    pub fn f_b_default() -> Self {
        Self::new(0.1903, 0.0013, 4.18)
    }

    /// FLAG 2023 average for $f_{B_s}$.
    pub fn f_bs_default() -> Self {
        Self::new(0.2303, 0.0013, 4.18)
    }

    /// FLAG 2023 average for $f_K$.
    pub fn f_k_default() -> Self {
        Self::new(0.1557, 0.0003, 2.0)
    }

    /// FLAG 2023 average for $f_\pi$.
    pub fn f_pi_default() -> Self {
        Self::new(0.1304, 0.0002, 2.0)
    }
}

// ===========================================================================
// BCL z-Expansion
// ===========================================================================

/// BCL $z$-expansion parameterization for a single form factor.
///
/// Stores the expansion coefficients $a_k$, the pole mass $m_{\text{pole}}$,
/// and the threshold parameters $t_+$ and $t_0$.
///
/// The conformal mapping is:
///
/// $$z(q^2) = \frac{\sqrt{t_+ - q^2} - \sqrt{t_+ - t_0}}
///                  {\sqrt{t_+ - q^2} + \sqrt{t_+ - t_0}}$$
///
/// and the form factor evaluates as:
///
/// $$f(q^2) = \frac{1}{1 - q^2 / m_{\text{pole}}^2}
///            \sum_{k} a_k \, z(q^2)^k$$
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFactorZExpansion {
    /// BCL expansion coefficients $[a_0, a_1, a_2, \ldots]$.
    pub coefficients: Vec<f64>,
    /// Pole mass in GeV for the Blaschke factor $1/(1 - q^2/m_{pole}^2)$.
    pub m_pole: f64,
    /// Production threshold $t_+ = (m_B + m_K)^2$ in GeV².
    pub t_plus: f64,
    /// Optimal expansion point $t_0$ in GeV².
    pub t_0: f64,
}

impl FormFactorZExpansion {
    /// Create a new BCL z-expansion form factor.
    pub fn new(coefficients: Vec<f64>, m_pole: f64, t_plus: f64, t_0: f64) -> Self {
        Self {
            coefficients,
            m_pole,
            t_plus,
            t_0,
        }
    }

    /// Compute the conformal variable $z(q^2)$.
    ///
    /// $$z = \frac{\sqrt{t_+ - q^2} - \sqrt{t_+ - t_0}}
    ///            {\sqrt{t_+ - q^2} + \sqrt{t_+ - t_0}}$$
    ///
    /// Returns 0 if $q^2 \geq t_+$ (above threshold, unphysical for
    /// the semi-leptonic region).
    pub fn z_variable(&self, q2: f64) -> f64 {
        if q2 >= self.t_plus {
            return 0.0;
        }
        let sqrt_diff = (self.t_plus - q2).sqrt();
        let sqrt_ref = (self.t_plus - self.t_0).sqrt();
        (sqrt_diff - sqrt_ref) / (sqrt_diff + sqrt_ref)
    }

    /// Evaluate the form factor at a given $q^2$ in GeV².
    ///
    /// $$f(q^2) = \frac{1}{1 - q^2/m_{\text{pole}}^2} \sum_k a_k z^k$$
    pub fn evaluate(&self, q2: f64) -> f64 {
        let z = self.z_variable(q2);
        let pole_factor = 1.0 / (1.0 - q2 / (self.m_pole * self.m_pole));

        let series: f64 = self
            .coefficients
            .iter()
            .enumerate()
            .fold(0.0, |acc, (k, &a_k)| acc + a_k * z.powi(k as i32));

        pole_factor * series
    }
}

// ===========================================================================
// B → K Form Factor Set
// ===========================================================================

/// Complete set of $B \to K$ transition form factors.
///
/// Contains the vector ($f_+$), scalar ($f_0$), and tensor ($f_T$) form
/// factors, each parameterized via the BCL $z$-expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BToKFormFactors {
    /// Vector form factor $f_+(q^2)$.
    pub f_plus: FormFactorZExpansion,
    /// Scalar form factor $f_0(q^2)$.
    pub f_zero: FormFactorZExpansion,
    /// Tensor form factor $f_T(q^2)$.
    pub f_tensor: FormFactorZExpansion,
}

/// Meson masses used in the $B \to K$ system (GeV).
pub const M_B: f64 = 5.27934;
/// Charged-kaon mass $m_K$ in GeV.
pub const M_K: f64 = 0.49368;
/// Vector-pole mass $m_{B_s^*}$ entering $f_+$ and $f_T$ in GeV.
pub const M_B_STAR_S: f64 = 5.3252;
/// Scalar-pole mass $m_{B_{s0}^*}$ entering $f_0$ in GeV.
pub const M_B_STAR_S0: f64 = 5.711;

impl BToKFormFactors {
    /// Default FLAG 2023 form factors for $B \to K$ transitions.
    ///
    /// Uses BCL $z$-expansion coefficients from lattice QCD averages.
    pub fn flag_defaults() -> Self {
        let t_plus = (M_B + M_K).powi(2);
        // Optimal t_0 minimising |z|_max
        let t_0 = (M_B + M_K) * (M_B.sqrt() - M_K.sqrt()).powi(2);

        Self {
            f_plus: FormFactorZExpansion::new(vec![0.466, -0.885, -0.213], M_B_STAR_S, t_plus, t_0),
            f_zero: FormFactorZExpansion::new(vec![0.292, 0.281, 0.150], M_B_STAR_S0, t_plus, t_0),
            f_tensor: FormFactorZExpansion::new(
                vec![0.460, -0.798, -0.470],
                M_B_STAR_S,
                t_plus,
                t_0,
            ),
        }
    }
}

// ===========================================================================
// Bag Parameters
// ===========================================================================

/// Bag parameter for neutral meson mixing.
///
/// The bag parameter $\hat{B}_q$ encodes the ratio of the full QCD matrix
/// element to its vacuum insertion approximation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BagParameter {
    /// Central value (dimensionless, renormalization-group invariant).
    pub value: f64,
    /// Combined uncertainty (1σ).
    pub error: f64,
}

impl BagParameter {
    /// Create a new bag parameter.
    pub fn new(value: f64, error: f64) -> Self {
        Self { value, error }
    }

    /// FLAG 2023 average for $\hat{B}_d$.
    pub fn b_d_default() -> Self {
        Self::new(1.222, 0.061)
    }

    /// FLAG 2023 average for $\hat{B}_s$.
    pub fn b_s_default() -> Self {
        Self::new(1.270, 0.054)
    }
}

// ===========================================================================
// Complete Lattice Input Configuration
// ===========================================================================

/// Complete set of Lattice QCD inputs for flavor physics computations.
///
/// Bundles decay constants, bag parameters, and form factors into a
/// single configuration object that can be serialized to/from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatticeInputs {
    /// $B$-meson decay constant $f_B$.
    pub f_b: DecayConstant,
    /// $B_s$-meson decay constant $f_{B_s}$.
    pub f_bs: DecayConstant,
    /// Kaon decay constant $f_K$.
    pub f_k: DecayConstant,
    /// Bag parameter $\hat{B}_d$ for $B_d$ mixing.
    pub b_hat_d: BagParameter,
    /// Bag parameter $\hat{B}_s$ for $B_s$ mixing.
    pub b_hat_s: BagParameter,
    /// $B \to K$ transition form factors.
    pub b_to_k: BToKFormFactors,
}

impl LatticeInputs {
    /// Standard FLAG 2023 defaults.
    pub fn flag_defaults() -> Self {
        Self {
            f_b: DecayConstant::f_b_default(),
            f_bs: DecayConstant::f_bs_default(),
            f_k: DecayConstant::f_k_default(),
            b_hat_d: BagParameter::b_d_default(),
            b_hat_s: BagParameter::b_s_default(),
            b_to_k: BToKFormFactors::flag_defaults(),
        }
    }
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_constant_defaults() {
        let f_b = DecayConstant::f_b_default();
        assert!((f_b.value - 0.1903).abs() < 1e-10);
        assert!(f_b.error > 0.0);

        let f_bs = DecayConstant::f_bs_default();
        assert!((f_bs.value - 0.2303).abs() < 1e-10);
    }

    #[test]
    fn z_variable_at_q2_zero() {
        let ff = BToKFormFactors::flag_defaults();
        let z0 = ff.f_plus.z_variable(0.0);
        // z(0) should be small and inside the unit disc
        // Sign depends on t_0 relative to 0: since t_0 > 0, we get
        // √(t+ - 0) > √(t+ - t0), so z(0) > 0 but small.
        assert!(z0.abs() < 1.0, "z(0) outside unit disc: {}", z0);
    }

    #[test]
    fn z_variable_at_t0() {
        let ff = BToKFormFactors::flag_defaults();
        let z_t0 = ff.f_plus.z_variable(ff.f_plus.t_0);
        // z(t_0) = 0 by construction
        assert!(z_t0.abs() < 1e-12, "z(t_0) should be zero, got {}", z_t0);
    }

    #[test]
    fn z_variable_above_threshold() {
        let ff = BToKFormFactors::flag_defaults();
        let z_above = ff.f_plus.z_variable(ff.f_plus.t_plus + 1.0);
        assert!((z_above).abs() < 1e-12);
    }

    #[test]
    fn form_factor_f_plus_at_zero() {
        let ff = BToKFormFactors::flag_defaults();
        let f_plus_0 = ff.f_plus.evaluate(0.0);
        // f+(0) ≈ 0.33 for B → K (known lattice result)
        assert!(f_plus_0 > 0.2, "f+(0) too small: {}", f_plus_0);
        assert!(f_plus_0 < 0.6, "f+(0) too large: {}", f_plus_0);
    }

    #[test]
    fn form_factor_f_zero_at_zero() {
        let ff = BToKFormFactors::flag_defaults();
        let f_zero_0 = ff.f_zero.evaluate(0.0);
        // f_0(0) = f_+(0) at q² = 0 (kinematic constraint)
        // They should be approximately equal; small deviations from
        // truncated z-expansion are expected.
        let f_plus_0 = ff.f_plus.evaluate(0.0);
        let ratio = f_zero_0 / f_plus_0;
        assert!(
            ratio > 0.5 && ratio < 2.0,
            "f_0(0)/f_+(0) out of range: {}",
            ratio
        );
    }

    #[test]
    fn form_factor_increases_with_q2() {
        // Form factors should generally increase with q² due to the pole
        let ff = BToKFormFactors::flag_defaults();
        let f_low = ff.f_plus.evaluate(1.0);
        let f_high = ff.f_plus.evaluate(15.0);
        assert!(
            f_high > f_low,
            "f+(15) = {} should exceed f+(1) = {}",
            f_high,
            f_low
        );
    }

    #[test]
    fn form_factor_serialization_roundtrip() {
        let ff = BToKFormFactors::flag_defaults();
        let json = serde_json::to_string(&ff).unwrap();
        let ff2: BToKFormFactors = serde_json::from_str(&json).unwrap();
        assert!((ff.f_plus.evaluate(5.0) - ff2.f_plus.evaluate(5.0)).abs() < 1e-15);
    }

    #[test]
    fn bag_parameter_defaults() {
        let b_d = BagParameter::b_d_default();
        assert!((b_d.value - 1.222).abs() < 1e-10);
        let b_s = BagParameter::b_s_default();
        assert!((b_s.value - 1.270).abs() < 1e-10);
    }

    #[test]
    fn lattice_inputs_flag_defaults() {
        let inputs = LatticeInputs::flag_defaults();
        assert!((inputs.f_b.value - 0.1903).abs() < 1e-10);
        assert!((inputs.f_bs.value - 0.2303).abs() < 1e-10);
        // Verify form factors are populated
        let fp = inputs.b_to_k.f_plus.evaluate(0.0);
        assert!(fp > 0.0);
    }
}
