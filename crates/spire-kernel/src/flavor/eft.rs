//! # Flavor Effective Field Theory Engine
//!
//! Implements the computational framework for precision flavor observables
//! using the Weak Effective Theory (WET) approach. Combines perturbative
//! Wilson coefficient functions with non-perturbative Lattice QCD inputs
//! to predict:
//!
//! - **Neutral meson mixing** ($\Delta M_d$, $\Delta M_s$): Mass differences
//!   in the $B_d$–$\bar{B}_d$ and $B_s$–$\bar{B}_s$ systems.
//! - **Rare semi-leptonic decays** ($B \to K \ell^+ \ell^-$): Differential
//!   and integrated branching fractions as functions of the dilepton
//!   invariant mass $q^2$.
//!
//! ## Wilson Coefficient Basis
//!
//! The effective Hamiltonian for $b \to s \ell^+ \ell^-$ transitions is:
//!
//! $$\mathcal{H}_{\text{eff}} = -\frac{4 G_F}{\sqrt{2}} V_{tb} V_{ts}^*
//!   \sum_i (C_i O_i + C_i' O_i')$$
//!
//! The relevant operators and their SM Wilson coefficients at $\mu = m_b$:
//!
//! | Coefficient | SM Value | Operator |
//! |-------------|----------|----------|
//! | $C_7^{\text{eff}}$ | $-0.304$ | EM dipole |
//! | $C_9^{\text{eff}}$ | $+4.211$ | Vector semileptonic |
//! | $C_{10}$ | $-4.103$ | Axial semileptonic |
//!
//! BSM contributions enter as additive shifts $\Delta C_i$.
//!
//! ## Usage
//!
//! ```rust
//! use spire_kernel::flavor::eft::{WilsonCoefficients, compute_delta_m_s};
//! use spire_kernel::flavor::lattice::LatticeInputs;
//!
//! let wc = WilsonCoefficients::sm_defaults();
//! let lattice = LatticeInputs::flag_defaults();
//! let dm_s = compute_delta_m_s(&lattice);
//! // Experimental value: ΔMs ≈ 17.765 ps⁻¹
//! assert!(dm_s > 10.0 && dm_s < 25.0);
//! ```

use serde::{Deserialize, Serialize};

use super::lattice::{BToKFormFactors, LatticeInputs, M_B, M_K};

// ===========================================================================
// Physical Constants
// ===========================================================================

/// Fermi constant $G_F = 1.1663788 \times 10^{-5}$ GeV$^{-2}$.
pub const G_FERMI: f64 = 1.1663788e-5;

/// Fine-structure constant $\alpha_{em} = 1/137.036$.
pub const ALPHA_EM: f64 = 1.0 / 137.036;

/// $W$ boson mass in GeV.
pub const M_W: f64 = 80.379;

/// Top quark pole mass in GeV.
pub const M_TOP: f64 = 173.1;

/// Bottom quark $\overline{MS}$ mass in GeV.
pub const M_B_QUARK: f64 = 4.18;

/// Muon mass in GeV.
pub const M_MU: f64 = 0.10566;

/// Electron mass in GeV.
pub const M_ELECTRON: f64 = 0.000511;

/// $B_d$ meson mass in GeV.
pub const M_BD: f64 = 5.27965;

/// $B_s$ meson mass in GeV.
pub const M_BS: f64 = 5.36688;

/// $B^+$ lifetime in seconds.
pub const TAU_B_PLUS: f64 = 1.638e-12;

/// $\hbar$ in GeV·s.
pub const HBAR: f64 = 6.582119569e-25;

/// Conversion factor: $\hbar$ in ps·GeV ($6.582 \times 10^{-13}$ ps·GeV).
pub const HBAR_PS_GEV: f64 = 6.582119569e-13;

/// QCD correction factor $\eta_B$ for meson mixing.
pub const ETA_B: f64 = 0.5510;

/// CKM matrix element $|V_{td}|$.
pub const V_TD: f64 = 0.00854;

/// CKM matrix element $|V_{ts}|$.
pub const V_TS: f64 = 0.0404;

/// CKM matrix element $|V_{tb}|$.
pub const V_TB: f64 = 0.999;

// ===========================================================================
// Wilson Coefficients
// ===========================================================================

/// Wilson coefficients for $b \to s \ell^+ \ell^-$ transitions.
///
/// Stores the effective coefficients $C_i(\mu)$ and their chirality-flipped
/// counterparts $C_i'(\mu)$. BSM effects enter as additive shifts on top
/// of the SM values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WilsonCoefficients {
    /// Electromagnetic dipole coefficient $C_7^{\text{eff}}$.
    pub c7_eff: f64,
    /// Chirality-flipped $C_7'^{\text{eff}}$.
    pub c7_prime: f64,
    /// Vector semileptonic coefficient $C_9^{\text{eff}}$.
    pub c9_eff: f64,
    /// Chirality-flipped $C_9'$.
    pub c9_prime: f64,
    /// Axial semileptonic coefficient $C_{10}$.
    pub c10: f64,
    /// Chirality-flipped $C_{10}'$.
    pub c10_prime: f64,
    /// Renormalization scale $\mu$ in GeV.
    pub scale_mu: f64,
}

impl WilsonCoefficients {
    /// Standard Model default values at $\mu = m_b \approx 4.18$ GeV.
    ///
    /// All primed coefficients vanish in the SM.
    pub fn sm_defaults() -> Self {
        Self {
            c7_eff: -0.304,
            c7_prime: 0.0,
            c9_eff: 4.211,
            c9_prime: 0.0,
            c10: -4.103,
            c10_prime: 0.0,
            scale_mu: M_B_QUARK,
        }
    }

    /// Create Wilson coefficients with BSM shifts on top of SM values.
    ///
    /// # Arguments
    /// * `delta_c9` — New Physics shift $\Delta C_9$.
    /// * `delta_c10` — New Physics shift $\Delta C_{10}$.
    /// * `delta_c7` — New Physics shift $\Delta C_7$.
    pub fn with_bsm_shifts(delta_c7: f64, delta_c9: f64, delta_c10: f64) -> Self {
        let mut wc = Self::sm_defaults();
        wc.c7_eff += delta_c7;
        wc.c9_eff += delta_c9;
        wc.c10 += delta_c10;
        wc
    }
}

// ===========================================================================
// Inami–Lim Function
// ===========================================================================

/// Inami–Lim function $S_0(x_t)$ for box diagram contributions to
/// neutral meson mixing.
///
/// $$S_0(x) = \frac{4x - 11x^2 + x^3}{4(1-x)^2}
///          - \frac{3x^3 \ln x}{2(1-x)^3}$$
///
/// where $x_t = (m_t / m_W)^2$.
pub fn inami_lim_s0(x: f64) -> f64 {
    let x2 = x * x;
    let x3 = x2 * x;
    let one_minus_x = 1.0 - x;
    let one_minus_x_sq = one_minus_x * one_minus_x;
    let one_minus_x_cb = one_minus_x_sq * one_minus_x;

    let term1 = (4.0 * x - 11.0 * x2 + x3) / (4.0 * one_minus_x_sq);
    let term2 = -(3.0 * x3 * x.ln()) / (2.0 * one_minus_x_cb);

    term1 + term2
}

// ===========================================================================
// B-Meson Mixing
// ===========================================================================

/// Input configuration for $B$-meson mixing computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BMixingInputs {
    /// Decay constant $f_{B_q}$ in GeV.
    pub f_bq: f64,
    /// Bag parameter $\hat{B}_q$ (dimensionless).
    pub b_hat_q: f64,
    /// $B_q$ meson mass in GeV.
    pub m_bq: f64,
    /// CKM factor $|V_{tq}^* V_{tb}|$.
    pub ckm_factor: f64,
}

impl BMixingInputs {
    /// Default inputs for $B_d$ mixing.
    pub fn b_d_defaults(lattice: &LatticeInputs) -> Self {
        Self {
            f_bq: lattice.f_b.value,
            b_hat_q: lattice.b_hat_d.value,
            m_bq: M_BD,
            ckm_factor: V_TD * V_TB,
        }
    }

    /// Default inputs for $B_s$ mixing.
    pub fn b_s_defaults(lattice: &LatticeInputs) -> Self {
        Self {
            f_bq: lattice.f_bs.value,
            b_hat_q: lattice.b_hat_s.value,
            m_bq: M_BS,
            ckm_factor: V_TS * V_TB,
        }
    }
}

/// Compute the mass difference $\Delta M_q$ for neutral $B_q$ meson mixing.
///
/// $$\Delta M_q = \frac{G_F^2 m_W^2}{6\pi^2}\,m_{B_q}\,f_{B_q}^2\,
///   \hat{B}_q\,\eta_B\,S_0(x_t)\,|V_{tq}^* V_{tb}|^2$$
///
/// # Returns
/// $\Delta M_q$ in ps$^{-1}$.
pub fn compute_delta_m(inputs: &BMixingInputs) -> f64 {
    let x_t = (M_TOP / M_W).powi(2);
    let s0 = inami_lim_s0(x_t);

    // Prefactor: G_F² m_W² / (6 π²)  in GeV⁰ units
    let prefactor = G_FERMI.powi(2) * M_W.powi(2) / (6.0 * std::f64::consts::PI.powi(2));

    // ΔM in GeV
    let delta_m_gev = prefactor
        * inputs.m_bq
        * inputs.f_bq.powi(2)
        * inputs.b_hat_q
        * ETA_B
        * s0
        * inputs.ckm_factor.powi(2);

    // Convert GeV to ps⁻¹: ΔM [ps⁻¹] = ΔM [GeV] / ℏ [GeV·ps]
    delta_m_gev / HBAR_PS_GEV
}

/// Compute $\Delta M_s$ using FLAG defaults.
pub fn compute_delta_m_s(lattice: &LatticeInputs) -> f64 {
    let inputs = BMixingInputs::b_s_defaults(lattice);
    compute_delta_m(&inputs)
}

/// Compute $\Delta M_d$ using FLAG defaults.
pub fn compute_delta_m_d(lattice: &LatticeInputs) -> f64 {
    let inputs = BMixingInputs::b_d_defaults(lattice);
    compute_delta_m(&inputs)
}

// ===========================================================================
// Rare Decay: B → K ℓ⁺ℓ⁻
// ===========================================================================

/// Källén triangle function $\lambda(a, b, c) = a^2 + b^2 + c^2 - 2ab - 2ac - 2bc$.
fn kallen(a: f64, b: f64, c: f64) -> f64 {
    a * a + b * b + c * c - 2.0 * a * b - 2.0 * a * c - 2.0 * b * c
}

/// Configuration for a $B \to K \ell^+ \ell^-$ computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BToKllConfig {
    /// Wilson coefficients.
    pub wilson: WilsonCoefficients,
    /// $B \to K$ form factors.
    pub form_factors: BToKFormFactors,
    /// Lepton mass in GeV (default: muon).
    pub m_lepton: f64,
    /// $B$ meson mass in GeV.
    pub m_b: f64,
    /// $K$ meson mass in GeV.
    pub m_k: f64,
}

impl BToKllConfig {
    /// Standard configuration for $B \to K \mu^+ \mu^-$.
    pub fn b_to_k_mumu_defaults() -> Self {
        Self {
            wilson: WilsonCoefficients::sm_defaults(),
            form_factors: BToKFormFactors::flag_defaults(),
            m_lepton: M_MU,
            m_b: M_B,
            m_k: M_K,
        }
    }

    /// Configuration with custom Wilson coefficients.
    pub fn with_wilson(mut self, wilson: WilsonCoefficients) -> Self {
        self.wilson = wilson;
        self
    }
}

/// Compute the differential decay rate $d\Gamma/dq^2$ for
/// $B \to K \ell^+ \ell^-$ at a given $q^2$.
///
/// $$\frac{d\Gamma}{dq^2} = N(q^2) \left[
///   \frac{2}{3}\lambda\,\beta_\ell^2\,(|F_V|^2 + |F_A|^2)
///   + q^2\,|F_S|^2 \right]$$
///
/// with:
/// - $F_V = (C_9 + C_9') f_+ + \frac{2m_b}{m_B + m_K} C_7 f_T$
/// - $F_A = (C_{10} + C_{10}') f_+$
/// - $F_S = \frac{m_B^2 - m_K^2}{2 m_B} (C_{10} + C_{10}') f_0$
///
/// # Returns
/// $d\Gamma/dq^2$ in GeV$^{-1}$. Returns 0 if $q^2$ is outside the
/// kinematically allowed region.
pub fn differential_decay_rate(config: &BToKllConfig, q2: f64) -> f64 {
    let m_b_sq = config.m_b * config.m_b;
    let m_k_sq = config.m_k * config.m_k;
    let m_l_sq = config.m_lepton * config.m_lepton;

    // Kinematic bounds
    let q2_min = 4.0 * m_l_sq;
    let q2_max = (config.m_b - config.m_k).powi(2);

    if q2 < q2_min || q2 > q2_max {
        return 0.0;
    }

    // Källén function
    let lam = kallen(m_b_sq, m_k_sq, q2);
    if lam <= 0.0 {
        return 0.0;
    }

    // Lepton velocity β_ℓ = √(1 - 4m_ℓ²/q²)
    let beta_l = (1.0 - 4.0 * m_l_sq / q2).sqrt();

    // Form factors at q²
    let wc = &config.wilson;
    let ff = &config.form_factors;

    let f_plus = ff.f_plus.evaluate(q2);
    let f_zero = ff.f_zero.evaluate(q2);
    let f_t = ff.f_tensor.evaluate(q2);

    // Effective amplitudes
    // F_V = (C9 + C9') f+ + (2 m_b / (m_B + m_K)) C7_eff f_T
    let f_v = (wc.c9_eff + wc.c9_prime) * f_plus
        + (2.0 * M_B_QUARK / (config.m_b + config.m_k)) * wc.c7_eff * f_t;

    // F_A = (C10 + C10') f+
    let f_a = (wc.c10 + wc.c10_prime) * f_plus;

    // F_S = (m_B² - m_K²) / (2 m_B) · (C10 + C10') · f_0
    // (scalar contribution, relevant for massive leptons)
    let f_s = (m_b_sq - m_k_sq) / (2.0 * config.m_b)
        * (wc.c10 + wc.c10_prime) * f_zero;

    // Normalization
    // N(q²) = G_F² α² |V_tb V_ts*|² / (512 π⁵ m_B³) · √λ · β_ℓ
    let ckm_sq = (V_TB * V_TS).powi(2);
    let norm = G_FERMI.powi(2) * ALPHA_EM.powi(2) * ckm_sq
        / (512.0 * std::f64::consts::PI.powi(5) * config.m_b.powi(3))
        * lam.sqrt()
        * beta_l;

    // Full differential rate
    let rate = norm * (
        (2.0 / 3.0) * lam * beta_l * beta_l * (f_v * f_v + f_a * f_a)
        + q2 * f_s * f_s
    );

    rate.max(0.0)
}

/// A single data point on the $d\Gamma/dq^2$ distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferentialPoint {
    /// Dilepton invariant mass squared $q^2$ in GeV².
    pub q2: f64,
    /// Differential decay rate $d\Gamma/dq^2$ in GeV$^{-1}$.
    pub dgamma_dq2: f64,
}

/// Complete report from a flavor physics computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavorObservableReport {
    /// $\Delta M_d$ in ps$^{-1}$.
    pub delta_m_d: f64,
    /// $\Delta M_s$ in ps$^{-1}$.
    pub delta_m_s: f64,
    /// Integrated branching ratio for $B \to K \mu^+ \mu^-$
    /// in the specified $q^2$ window.
    pub branching_ratio: f64,
    /// Differential decay rate distribution.
    pub differential_spectrum: Vec<DifferentialPoint>,
    /// The $q^2$ integration window lower bound in GeV².
    pub q2_min: f64,
    /// The $q^2$ integration window upper bound in GeV².
    pub q2_max: f64,
    /// Experimental $\Delta M_d$ for comparison (ps$^{-1}$).
    pub exp_delta_m_d: f64,
    /// Experimental $\Delta M_s$ for comparison (ps$^{-1}$).
    pub exp_delta_m_s: f64,
}

/// Experimental values for comparison.
pub const EXP_DELTA_M_D: f64 = 0.5065; // ps⁻¹
pub const EXP_DELTA_M_S: f64 = 17.765; // ps⁻¹

/// Compute the full flavor observable report.
///
/// Integrates the differential decay rate over the specified $q^2$
/// window using the trapezoidal rule, and computes both B-meson
/// mixing mass differences.
///
/// # Arguments
/// * `lattice` — Lattice QCD inputs (decay constants, bag parameters, form factors).
/// * `wilson` — Wilson coefficients (SM + optional BSM shifts).
/// * `q2_min` — Lower bound of the $q^2$ integration window in GeV².
/// * `q2_max` — Upper bound of the $q^2$ integration window in GeV².
/// * `n_points` — Number of evaluation points for the differential spectrum.
pub fn compute_flavor_observables(
    lattice: &LatticeInputs,
    wilson: &WilsonCoefficients,
    q2_min: f64,
    q2_max: f64,
    n_points: usize,
) -> FlavorObservableReport {
    // B-meson mixing
    let delta_m_d = compute_delta_m_d(lattice);
    let delta_m_s = compute_delta_m_s(lattice);

    // B → K μμ configuration
    let config = BToKllConfig::b_to_k_mumu_defaults().with_wilson(wilson.clone());

    // Kinematic limits
    let q2_lo = q2_min.max(4.0 * M_MU * M_MU);
    let q2_hi = q2_max.min((M_B - M_K).powi(2));

    let n = n_points.max(10);
    let step = (q2_hi - q2_lo) / (n as f64 - 1.0);

    // Evaluate differential spectrum
    let mut spectrum = Vec::with_capacity(n);
    for i in 0..n {
        let q2 = q2_lo + (i as f64) * step;
        let dgamma = differential_decay_rate(&config, q2);
        spectrum.push(DifferentialPoint {
            q2,
            dgamma_dq2: dgamma,
        });
    }

    // Integrate via trapezoidal rule to get partial width
    let mut integral = 0.0;
    for i in 0..(spectrum.len() - 1) {
        let dq2 = spectrum[i + 1].q2 - spectrum[i].q2;
        integral += 0.5 * (spectrum[i].dgamma_dq2 + spectrum[i + 1].dgamma_dq2) * dq2;
    }

    // Convert partial width to branching ratio
    // BR = Γ_partial · τ_B / ℏ
    let branching_ratio = integral * TAU_B_PLUS / HBAR;

    FlavorObservableReport {
        delta_m_d,
        delta_m_s,
        branching_ratio,
        differential_spectrum: spectrum,
        q2_min: q2_lo,
        q2_max: q2_hi,
        exp_delta_m_d: EXP_DELTA_M_D,
        exp_delta_m_s: EXP_DELTA_M_S,
    }
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavor::lattice::LatticeInputs;

    #[test]
    fn inami_lim_function_value() {
        // x_t = (m_t/m_W)² ≈ 4.64, S_0 ≈ 2.31
        let x_t = (M_TOP / M_W).powi(2);
        let s0 = inami_lim_s0(x_t);
        assert!(s0 > 2.0 && s0 < 3.0,
            "S_0(x_t) = {} expected in [2.0, 3.0]", s0);
    }

    #[test]
    fn wilson_coefficients_sm_defaults() {
        let wc = WilsonCoefficients::sm_defaults();
        assert!((wc.c7_eff - (-0.304)).abs() < 1e-10);
        assert!((wc.c9_eff - 4.211).abs() < 1e-10);
        assert!((wc.c10 - (-4.103)).abs() < 1e-10);
        assert!((wc.c7_prime).abs() < 1e-15);
        assert!((wc.c9_prime).abs() < 1e-15);
        assert!((wc.c10_prime).abs() < 1e-15);
    }

    #[test]
    fn wilson_coefficients_bsm_shift() {
        let wc = WilsonCoefficients::with_bsm_shifts(0.0, -1.0, 0.5);
        assert!((wc.c9_eff - 3.211).abs() < 1e-10);
        assert!((wc.c10 - (-3.603)).abs() < 1e-10);
    }

    #[test]
    fn delta_m_s_order_of_magnitude() {
        let lattice = LatticeInputs::flag_defaults();
        let dm_s = compute_delta_m_s(&lattice);
        // Experimental: 17.765 ps⁻¹; we expect the right ballpark
        assert!(dm_s > 5.0,
            "ΔMs = {} ps⁻¹ too small", dm_s);
        assert!(dm_s < 30.0,
            "ΔMs = {} ps⁻¹ too large", dm_s);
    }

    #[test]
    fn delta_m_d_order_of_magnitude() {
        let lattice = LatticeInputs::flag_defaults();
        let dm_d = compute_delta_m_d(&lattice);
        // Experimental: 0.5065 ps⁻¹
        assert!(dm_d > 0.1,
            "ΔMd = {} ps⁻¹ too small", dm_d);
        assert!(dm_d < 2.0,
            "ΔMd = {} ps⁻¹ too large", dm_d);
    }

    #[test]
    fn delta_m_ratio_vs_ckm() {
        let lattice = LatticeInputs::flag_defaults();
        let dm_s = compute_delta_m_s(&lattice);
        let dm_d = compute_delta_m_d(&lattice);
        // ΔMs/ΔMd ≈ |Vts/Vtd|² × (f_Bs/f_Bd)² × (B̂s/B̂d) × (m_Bs/m_Bd) ≈ 35
        let ratio = dm_s / dm_d;
        assert!(ratio > 15.0 && ratio < 60.0,
            "ΔMs/ΔMd = {} expected in [15, 60]", ratio);
    }

    #[test]
    fn differential_rate_kinematic_bounds() {
        let config = BToKllConfig::b_to_k_mumu_defaults();
        // Below kinematic threshold
        assert_eq!(differential_decay_rate(&config, 0.01), 0.0);
        // Above kinematic maximum
        assert_eq!(differential_decay_rate(&config, 25.0), 0.0);
    }

    #[test]
    fn differential_rate_positive_in_physical_region() {
        let config = BToKllConfig::b_to_k_mumu_defaults();
        for q2 in [1.0, 3.0, 6.0, 10.0, 15.0, 20.0] {
            let rate = differential_decay_rate(&config, q2);
            assert!(rate > 0.0,
                "dΓ/dq² should be positive at q² = {} GeV², got {}", q2, rate);
        }
    }

    #[test]
    fn bsm_shift_changes_decay_rate() {
        let ff = BToKFormFactors::flag_defaults();
        let sm_config = BToKllConfig {
            wilson: WilsonCoefficients::sm_defaults(),
            form_factors: ff.clone(),
            m_lepton: M_MU,
            m_b: M_B,
            m_k: M_K,
        };
        let bsm_config = BToKllConfig {
            wilson: WilsonCoefficients::with_bsm_shifts(0.0, -1.0, 0.0),
            form_factors: ff,
            m_lepton: M_MU,
            m_b: M_B,
            m_k: M_K,
        };

        let sm_rate = differential_decay_rate(&sm_config, 6.0);
        let bsm_rate = differential_decay_rate(&bsm_config, 6.0);

        // ΔC9 = -1 should significantly alter the rate
        let rel_change = ((bsm_rate - sm_rate) / sm_rate).abs();
        assert!(rel_change > 0.05,
            "BSM shift ΔC9=-1 should change rate by >5%, got {}%",
            rel_change * 100.0);
    }

    #[test]
    fn c10_modification_impacts_rate() {
        let ff = BToKFormFactors::flag_defaults();
        let sm_config = BToKllConfig {
            wilson: WilsonCoefficients::sm_defaults(),
            form_factors: ff.clone(),
            m_lepton: M_MU,
            m_b: M_B,
            m_k: M_K,
        };
        let bsm_config = BToKllConfig {
            wilson: WilsonCoefficients::with_bsm_shifts(0.0, 0.0, 1.0),
            form_factors: ff,
            m_lepton: M_MU,
            m_b: M_B,
            m_k: M_K,
        };

        let sm_rate = differential_decay_rate(&sm_config, 6.0);
        let bsm_rate = differential_decay_rate(&bsm_config, 6.0);

        assert!((sm_rate - bsm_rate).abs() > 1e-30,
            "Modifying C10 should impact the decay width");
    }

    #[test]
    fn full_observable_report() {
        let lattice = LatticeInputs::flag_defaults();
        let wilson = WilsonCoefficients::sm_defaults();
        let report = compute_flavor_observables(&lattice, &wilson, 1.0, 6.0, 50);

        assert!(report.delta_m_s > 0.0);
        assert!(report.delta_m_d > 0.0);
        assert!(report.branching_ratio > 0.0);
        assert_eq!(report.differential_spectrum.len(), 50);
        assert!(report.q2_min >= 4.0 * M_MU * M_MU);
    }

    #[test]
    fn observable_report_serialization() {
        let lattice = LatticeInputs::flag_defaults();
        let wilson = WilsonCoefficients::sm_defaults();
        let report = compute_flavor_observables(&lattice, &wilson, 1.0, 6.0, 20);
        let json = serde_json::to_string(&report).unwrap();
        let report2: FlavorObservableReport = serde_json::from_str(&json).unwrap();
        assert!((report.delta_m_s - report2.delta_m_s).abs() < 1e-15);
        assert_eq!(report.differential_spectrum.len(), report2.differential_spectrum.len());
    }

    #[test]
    fn kallen_function_known_values() {
        // λ(s, 0, 0) = s²
        assert!((kallen(4.0, 0.0, 0.0) - 16.0).abs() < 1e-12);
        // λ(s, m², m²) = s(s - 4m²) for equal masses
        let s = 10.0;
        let m2 = 1.0;
        let expected = s * (s - 4.0 * m2);
        assert!((kallen(s, m2, m2) - expected).abs() < 1e-12);
    }
}
