//! # Relic Density - Cosmological Freeze-Out Calculator
//!
//! Computes the present-day cosmological relic abundance $\Omega h^2$ of a
//! dark matter (DM) candidate by solving the Boltzmann equation through the
//! thermal freeze-out epoch of the early universe.
//!
//! ## Physical Framework
//!
//! In the standard cosmological picture, DM particles were initially in
//! thermal equilibrium with the primordial plasma. As the universe expanded
//! and cooled, the DM annihilation rate dropped below the Hubble expansion
//! rate, causing the DM number density to "freeze out" at a constant
//! comoving value.
//!
//! The evolution of the DM yield $Y = n/s$ (number density normalised to
//! entropy density) is governed by the Boltzmann equation:
//!
//! $$\frac{dY}{dx} = -\frac{\lambda(x)}{x^2} \left(Y^2 - Y_{\text{eq}}^2\right)$$
//!
//! where $x = m_\chi / T$ and $\lambda$ is the dimensionless annihilation
//! strength:
//!
//! $$\lambda = \sqrt{\frac{\pi}{45}}\; M_{\text{Pl}}\; m_\chi\;
//!   \frac{g_{*s}}{\sqrt{g_*}}\; \langle\sigma v\rangle$$
//!
//! ## Solver Strategy
//!
//! The equation is extremely stiff at early times ($\lambda \sim 10^{12}$)
//! when $Y \approx Y_{\text{eq}}$. We employ the standard semi-analytic
//! method (Kolb & Turner, *The Early Universe*):
//!
//! 1. **Freeze-out temperature** $x_f$ is found iteratively from
//!    $n_{\text{eq}} \langle\sigma v\rangle = H(T)$.
//! 2. **Pre-freeze-out** ($x < x_f$): $Y$ tracks $Y_{\text{eq}}$ exactly.
//! 3. **Post-freeze-out** ($x > x_f$): $Y \gg Y_{\text{eq}}$, so the
//!    equation reduces to $dY/dx \approx -\lambda Y^2 / x^2$, which has
//!    the analytic solution $1/Y(x) = 1/Y_f + \lambda(1/x_f - 1/x)$.
//! 4. The relic density is $\Omega h^2 = m_\chi s_0 Y_\infty / (\rho_c/h^2)$.
//!
//! For detailed evolution curves, a numerical integration with the adaptive
//! Dormand-Prince solver is also available starting from $x_f$.
//!
//! ## Usage
//!
//! ```rust
//! use spire_kernel::cosmology::relic::{RelicConfig, compute_relic_density};
//!
//! let config = RelicConfig {
//!     dm_mass: 100.0,           // 100 GeV dark matter candidate
//!     sigma_v_a: 3e-26,         // s-wave coefficient (cm³/s)
//!     sigma_v_b: 0.0,           // p-wave coefficient
//!     g_dm: 2.0,                // spin degrees of freedom
//!     g_star: 86.25,            // SM effective d.o.f. at freeze-out
//!     g_star_s: 86.25,          // entropic d.o.f.
//!     x_start: 1.0,             // plot start
//!     x_end: 1000.0,            // plot end (well past freeze-out)
//! };
//!
//! let report = compute_relic_density(&config);
//! // Planck observation: Omega_c h^2 ≈ 0.120
//! assert!(report.omega_h2 > 0.0);
//! assert!(report.x_freeze_out > 1.0);
//! ```

use serde::{Deserialize, Serialize};

// ===========================================================================
// Physical Constants
// ===========================================================================

/// Planck mass $M_{\text{Pl}} = 1.221 \times 10^{19}$ GeV.
pub const M_PLANCK: f64 = 1.221e19;

/// Reduced Planck mass $\bar{M}_{\text{Pl}} = M_{\text{Pl}} / \sqrt{8\pi}
/// \approx 2.435 \times 10^{18}$ GeV.
pub const M_PLANCK_REDUCED: f64 = 2.435e18;

/// Conversion factor from natural units (GeV$^{-2}$) to cm$^3$/s for
/// thermally averaged cross-sections.
///
/// $1\;\text{GeV}^{-2} \approx 1.167 \times 10^{-17}\;\text{cm}^3/\text{s}$.
pub const GEV_M2_TO_CM3_PER_S: f64 = 1.167e-17;

/// Present-day entropy density $s_0 = 2891.2\;\text{cm}^{-3}$.
pub const S0_ENTROPY_DENSITY: f64 = 2891.2;

/// Critical density divided by $h^2$:
/// $\rho_c / h^2 \approx 1.054 \times 10^{-5}\;\text{GeV}\;\text{cm}^{-3}$.
pub const RHO_CRIT_OVER_H2: f64 = 1.054e-5;

/// Planck satellite measurement of cold dark matter relic density.
pub const PLANCK_OMEGA_CDM_H2: f64 = 0.120;

// ===========================================================================
// Configuration
// ===========================================================================

/// Configuration for a relic density computation.
///
/// Specifies the DM candidate properties and the thermally averaged
/// annihilation cross-section in the velocity expansion
/// $\langle\sigma v\rangle \approx a + b \langle v^2 \rangle$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicConfig {
    /// Mass of the dark matter candidate in GeV.
    pub dm_mass: f64,

    /// $s$-wave annihilation coefficient $a$ in cm$^3$/s.
    ///
    /// The canonical thermal WIMP cross-section is $\sim 3 \times 10^{-26}$ cm$^3$/s.
    pub sigma_v_a: f64,

    /// $p$-wave annihilation coefficient $b$ in cm$^3$/s.
    ///
    /// The velocity-dependent part: $\langle\sigma v\rangle \approx a + 6b/x$.
    /// For $s$-wave dominated processes, set to 0.
    pub sigma_v_b: f64,

    /// Internal degrees of freedom of the DM particle.
    ///
    /// Dirac fermion: 4.  Majorana fermion: 2.  Real scalar: 1.  Complex scalar: 2.
    pub g_dm: f64,

    /// Effective number of relativistic degrees of freedom $g_*$
    /// (energy density). Full SM above the top quark: 106.75.
    pub g_star: f64,

    /// Effective entropic degrees of freedom $g_{*s}$.
    /// Often equal to $g_*$ at high temperatures.
    pub g_star_s: f64,

    /// Plot/evolution start point $x_0 = m_\chi / T_0$. Typically 1.
    pub x_start: f64,

    /// Plot/evolution end point $x_f = m_\chi / T_f$. Typically 500-1000.
    pub x_end: f64,
}

impl Default for RelicConfig {
    /// Default: 100 GeV WIMP with canonical thermal cross-section.
    fn default() -> Self {
        Self {
            dm_mass: 100.0,
            sigma_v_a: 3.0e-26,
            sigma_v_b: 0.0,
            g_dm: 2.0,
            g_star: 86.25,
            g_star_s: 86.25,
            x_start: 1.0,
            x_end: 1000.0,
        }
    }
}

// ===========================================================================
// Report
// ===========================================================================

/// A single data point on the freeze-out evolution curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeOutPoint {
    /// $x = m_\chi / T$
    pub x: f64,
    /// Actual yield $Y(x)$
    pub y: f64,
    /// Equilibrium yield $Y_{\text{eq}}(x)$
    pub y_eq: f64,
}

/// Complete report from a relic density computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicDensityReport {
    /// Present-day relic abundance $\Omega h^2$.
    pub omega_h2: f64,

    /// Freeze-out temperature parameter $x_f = m_\chi / T_f$.
    pub x_freeze_out: f64,

    /// Final asymptotic yield $Y_\infty$.
    pub y_infinity: f64,

    /// Evolution curve data points for plotting.
    pub evolution: Vec<FreezeOutPoint>,

    /// Comparison with the Planck measurement.
    pub planck_omega_h2: f64,

    /// Abundance classification: "under-abundant", "compatible", or "over-closes".
    pub classification: String,

    /// The DM candidate mass used (GeV).
    pub dm_mass: f64,

    /// The thermally averaged cross-section at freeze-out (cm$^3$/s).
    pub sigma_v: f64,
}

// ===========================================================================
// Equilibrium Yield
// ===========================================================================

/// Compute the equilibrium yield $Y_{\text{eq}}(x)$ for a non-relativistic species.
///
/// $$Y_{\text{eq}}(x) = \frac{45}{4\pi^4} \frac{g}{g_{*s}} x^2 K_2(x)$$
///
/// where $K_2(x)$ is the modified Bessel function of the second kind,
/// approximated for $x \ge 0.01$ by:
///
/// $$K_2(x) \approx \sqrt{\frac{\pi}{2x}}\; e^{-x}
///   \left(1 + \frac{15}{8x} + \frac{105}{128 x^2}\right)$$
pub fn equilibrium_yield(x: f64, g_dm: f64, g_star_s: f64) -> f64 {
    if x < 0.01 {
        // Ultra-relativistic regime: Y_eq approaches a constant
        return 0.278 * g_dm / g_star_s;
    }

    let prefactor = 45.0 / (4.0 * std::f64::consts::PI.powi(4));

    // Modified Bessel K_2 approximation
    let k2_approx = (std::f64::consts::PI / (2.0 * x)).sqrt()
        * (-x).exp()
        * (1.0 + 15.0 / (8.0 * x) + 105.0 / (128.0 * x * x));

    prefactor * (g_dm / g_star_s) * x * x * k2_approx
}

// ===========================================================================
// Thermally Averaged Cross-Section
// ===========================================================================

/// Compute the thermally averaged annihilation cross-section.
///
/// $$\langle\sigma v\rangle(x) = a + \frac{6b}{x}$$
pub fn thermal_cross_section(x: f64, sigma_v_a: f64, sigma_v_b: f64) -> f64 {
    sigma_v_a + 6.0 * sigma_v_b / x
}

// ===========================================================================
// Dimensionless Annihilation Strength
// ===========================================================================

/// Compute the dimensionless annihilation prefactor $\lambda(x)$.
///
/// $$\lambda(x) = \sqrt{\frac{\pi}{45}}\; \bar{M}_{\text{Pl}}\; m_\chi\;
///   \frac{g_{*s}}{\sqrt{g_*}}\; \langle\sigma v\rangle_{\text{nat}}(x)$$
fn lambda_prefactor(x: f64, config: &RelicConfig) -> f64 {
    let sigma_v_cm3 = thermal_cross_section(x, config.sigma_v_a, config.sigma_v_b);
    let sigma_v_nat = sigma_v_cm3 / GEV_M2_TO_CM3_PER_S;

    (std::f64::consts::PI / 45.0).sqrt()
        * M_PLANCK_REDUCED
        * config.dm_mass
        * (config.g_star_s / config.g_star.sqrt())
        * sigma_v_nat
}

// ===========================================================================
// Freeze-Out Temperature
// ===========================================================================

/// Find the freeze-out point $x_f$ iteratively (Kolb & Turner method).
///
/// Solves $x_f = \ln(c\,(c+2)\,\alpha) - \tfrac{1}{2}\ln(x_f)$
/// where $\alpha = 0.038\,g\,m\,M_{\text{Pl}}\,\langle\sigma v\rangle / g_*^{1/2}$.
fn find_freeze_out(config: &RelicConfig) -> f64 {
    let sigma_v_cm3 = thermal_cross_section(20.0, config.sigma_v_a, config.sigma_v_b);
    let sigma_v_nat = sigma_v_cm3 / GEV_M2_TO_CM3_PER_S;

    let alpha =
        0.038 * config.g_dm * config.dm_mass * M_PLANCK * sigma_v_nat / config.g_star.sqrt();

    let c = 0.5_f64;
    let ln_arg = c * (c + 2.0) * alpha;

    if ln_arg <= 1.0 {
        return 3.0;
    }

    let mut x_f = ln_arg.ln();
    for _ in 0..50 {
        let x_f_new = ln_arg.ln() - 0.5 * x_f.ln();
        if (x_f_new - x_f).abs() < 0.01 {
            break;
        }
        x_f = x_f_new;
    }

    x_f.max(3.0)
}

// ===========================================================================
// Main Solver - Semi-Analytic
// ===========================================================================

/// Compute the cosmological relic density for a dark matter candidate.
///
/// Uses the semi-analytic freeze-out method:
/// 1. Find $x_f$ iteratively.
/// 2. Pre-freeze-out: $Y = Y_{\text{eq}}$.
/// 3. Post-freeze-out: analytic $1/Y_\infty = 1/Y_f + \lambda / x_f$.
/// 4. Full evolution curve with ~300 log-spaced points for plotting.
pub fn compute_relic_density(config: &RelicConfig) -> RelicDensityReport {
    let x_f = find_freeze_out(config);
    let y_f = equilibrium_yield(x_f, config.g_dm, config.g_star_s);
    let lambda_f = lambda_prefactor(x_f, config);

    // Post-freeze-out: 1/Y_∞ = 1/Y_f + lambda_f / x_f
    let inv_y_inf = 1.0 / y_f + lambda_f / x_f;
    let y_infinity = 1.0 / inv_y_inf;

    let omega_h2 = config.dm_mass * S0_ENTROPY_DENSITY * y_infinity / RHO_CRIT_OVER_H2;

    let classification = if omega_h2 < 0.10 {
        "under-abundant".to_string()
    } else if omega_h2 > 0.14 {
        "over-closes".to_string()
    } else {
        "compatible".to_string()
    };

    // Build evolution curve
    let n_points = 300;
    let mut evolution = Vec::with_capacity(n_points);
    let log_x_start = config.x_start.max(0.5).ln();
    let log_x_end = config.x_end.ln();

    for i in 0..n_points {
        let frac = i as f64 / (n_points - 1) as f64;
        let x = (log_x_start + frac * (log_x_end - log_x_start)).exp();
        let y_eq = equilibrium_yield(x, config.g_dm, config.g_star_s);

        let y = if x <= x_f {
            y_eq
        } else {
            let inv_y = 1.0 / y_f + lambda_f * (1.0 / x_f - 1.0 / x);
            if inv_y > 0.0 {
                1.0 / inv_y
            } else {
                y_eq
            }
        };

        evolution.push(FreezeOutPoint { x, y, y_eq });
    }

    let sigma_v = thermal_cross_section(x_f, config.sigma_v_a, config.sigma_v_b);

    RelicDensityReport {
        omega_h2,
        x_freeze_out: x_f,
        y_infinity,
        evolution,
        planck_omega_h2: PLANCK_OMEGA_CDM_H2,
        classification,
        dm_mass: config.dm_mass,
        sigma_v,
    }
}

// ===========================================================================
// Numerical Post-Freeze-Out Integration
// ===========================================================================

/// Compute relic density using numerical integration from $x_f$ onwards.
///
/// Uses the [`DormandPrince45`](crate::math::ode::DormandPrince45) adaptive
/// solver for the post-freeze-out phase where the equation is well-conditioned.
pub fn compute_relic_density_numerical(config: &RelicConfig) -> RelicDensityReport {
    use crate::math::ode::{DormandPrince45, OdeSolver};

    let x_f = find_freeze_out(config);
    let y_f = equilibrium_yield(x_f, config.g_dm, config.g_star_s);

    let solver = DormandPrince45::with_config(1e-12, 1e-8, 100_000, 0.1, 1e-4, 50.0);

    let rhs = |x: f64, y: f64| -> f64 {
        let y_eq = equilibrium_yield(x, config.g_dm, config.g_star_s);
        let lam = lambda_prefactor(x, config);
        -lam / (x * x) * (y * y - y_eq * y_eq)
    };

    let trajectory = solver.integrate(rhs, x_f, y_f, config.x_end);

    let mut evolution = Vec::new();

    // Pre-freeze-out
    let n_pre = 100;
    let log_start = config.x_start.max(0.5).ln();
    let log_xf = x_f.ln();
    for i in 0..n_pre {
        let frac = i as f64 / n_pre as f64;
        let x = (log_start + frac * (log_xf - log_start)).exp();
        let y_eq = equilibrium_yield(x, config.g_dm, config.g_star_s);
        evolution.push(FreezeOutPoint { x, y: y_eq, y_eq });
    }

    // Post-freeze-out: subsample trajectory
    let max_post = 200;
    let stride = if trajectory.len() > max_post {
        trajectory.len() / max_post
    } else {
        1
    };
    for (i, &(x, y)) in trajectory.iter().enumerate() {
        if i % stride == 0 || i == trajectory.len() - 1 {
            let y_eq = equilibrium_yield(x, config.g_dm, config.g_star_s);
            evolution.push(FreezeOutPoint {
                x,
                y: y.max(0.0),
                y_eq,
            });
        }
    }

    let y_infinity = trajectory.last().map_or(y_f, |&(_, y)| y.max(0.0));
    let omega_h2 = config.dm_mass * S0_ENTROPY_DENSITY * y_infinity / RHO_CRIT_OVER_H2;

    let classification = if omega_h2 < 0.10 {
        "under-abundant".to_string()
    } else if omega_h2 > 0.14 {
        "over-closes".to_string()
    } else {
        "compatible".to_string()
    };

    let sigma_v = thermal_cross_section(x_f, config.sigma_v_a, config.sigma_v_b);

    RelicDensityReport {
        omega_h2,
        x_freeze_out: x_f,
        y_infinity,
        evolution,
        planck_omega_h2: PLANCK_OMEGA_CDM_H2,
        classification,
        dm_mass: config.dm_mass,
        sigma_v,
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equilibrium_yield_is_positive() {
        for &x in &[0.005, 0.5, 1.0, 5.0, 10.0, 20.0, 50.0] {
            let y_eq = equilibrium_yield(x, 2.0, 86.25);
            assert!(y_eq > 0.0, "Y_eq({}) = {} should be > 0", x, y_eq);
        }
    }

    #[test]
    fn equilibrium_yield_decreases_with_x() {
        let y1 = equilibrium_yield(5.0, 2.0, 86.25);
        let y2 = equilibrium_yield(10.0, 2.0, 86.25);
        let y3 = equilibrium_yield(20.0, 2.0, 86.25);
        assert!(
            y1 > y2 && y2 > y3,
            "Y_eq should decrease: {} > {} > {}",
            y1,
            y2,
            y3
        );
    }

    #[test]
    fn equilibrium_yield_boltzmann_suppressed_at_large_x() {
        let y_eq = equilibrium_yield(50.0, 2.0, 86.25);
        assert!(
            y_eq < 1e-15,
            "Y_eq(50) = {} should be extremely small",
            y_eq
        );
    }

    #[test]
    fn equilibrium_yield_relativistic_limit() {
        let y_eq = equilibrium_yield(0.001, 2.0, 86.25);
        let expected = 0.278 * 2.0 / 86.25;
        assert!(
            (y_eq - expected).abs() / expected < 0.01,
            "Relativistic Y_eq = {}, expected ~{}",
            y_eq,
            expected
        );
    }

    #[test]
    fn thermal_cross_section_s_wave_only() {
        let sigma = thermal_cross_section(20.0, 3e-26, 0.0);
        assert!((sigma - 3e-26).abs() < 1e-30);
    }

    #[test]
    fn thermal_cross_section_p_wave_contribution() {
        let sigma = thermal_cross_section(20.0, 3e-26, 1e-26);
        let expected = 3e-26 + 6.0 * 1e-26 / 20.0;
        assert!((sigma - expected).abs() / expected < 1e-10);
    }

    #[test]
    fn freeze_out_reasonable_for_wimp() {
        let config = RelicConfig::default();
        let x_f = find_freeze_out(&config);
        assert!(
            x_f > 10.0 && x_f < 50.0,
            "x_f = {} should be in 10-50 for a WIMP",
            x_f
        );
    }

    #[test]
    fn lambda_prefactor_is_large() {
        let config = RelicConfig::default();
        let lam = lambda_prefactor(20.0, &config);
        assert!(
            lam > 1e10 && lam < 1e15,
            "lambda = {} should be ~ 10^12",
            lam
        );
    }

    #[test]
    fn relic_density_default_config_positive() {
        let config = RelicConfig::default();
        let report = compute_relic_density(&config);
        assert!(report.omega_h2 > 0.0, "Omega h^2 = {}", report.omega_h2);
        assert!(report.y_infinity > 0.0, "Y_inf = {}", report.y_infinity);
    }

    #[test]
    fn relic_density_reasonable_order_of_magnitude() {
        let config = RelicConfig::default();
        let report = compute_relic_density(&config);
        assert!(
            report.omega_h2 > 0.001 && report.omega_h2 < 10.0,
            "Omega h^2 = {}",
            report.omega_h2
        );
    }

    #[test]
    fn relic_density_has_evolution_curve() {
        let config = RelicConfig::default();
        let report = compute_relic_density(&config);
        assert!(
            report.evolution.len() > 10,
            "Got {} points",
            report.evolution.len()
        );
    }

    #[test]
    fn relic_density_freeze_out_occurs() {
        let config = RelicConfig::default();
        let report = compute_relic_density(&config);
        assert!(
            report.x_freeze_out > 5.0 && report.x_freeze_out < 100.0,
            "x_f = {}",
            report.x_freeze_out
        );
    }

    #[test]
    fn relic_density_yield_plateaus() {
        let config = RelicConfig::default();
        let report = compute_relic_density(&config);
        let late: Vec<_> = report.evolution.iter().filter(|p| p.x > 200.0).collect();
        if late.len() >= 2 {
            let y0 = late.first().unwrap().y;
            let y1 = late.last().unwrap().y;
            let rel = (y1 - y0).abs() / y0.max(1e-30);
            assert!(rel < 0.5, "Yield should plateau: rel change = {}", rel);
        }
    }

    #[test]
    fn larger_cross_section_gives_smaller_omega() {
        let r_small = compute_relic_density(&RelicConfig {
            sigma_v_a: 1e-26,
            ..Default::default()
        });
        let r_large = compute_relic_density(&RelicConfig {
            sigma_v_a: 1e-25,
            ..Default::default()
        });
        assert!(
            r_small.omega_h2 > r_large.omega_h2,
            "small={}, large={}",
            r_small.omega_h2,
            r_large.omega_h2
        );
    }

    #[test]
    fn heavier_dm_both_positive() {
        let r_light = compute_relic_density(&RelicConfig {
            dm_mass: 50.0,
            ..Default::default()
        });
        let r_heavy = compute_relic_density(&RelicConfig {
            dm_mass: 500.0,
            ..Default::default()
        });
        assert!(r_light.omega_h2 > 0.0 && r_heavy.omega_h2 > 0.0);
    }

    #[test]
    fn classification_under_abundant() {
        let r = compute_relic_density(&RelicConfig {
            sigma_v_a: 1e-24,
            ..Default::default()
        });
        assert_eq!(
            r.classification, "under-abundant",
            "Omega h^2 = {}",
            r.omega_h2
        );
    }

    #[test]
    fn planck_value_included_in_report() {
        let r = compute_relic_density(&RelicConfig::default());
        assert!((r.planck_omega_h2 - PLANCK_OMEGA_CDM_H2).abs() < 1e-10);
    }

    #[test]
    fn evolution_y_exceeds_y_eq_after_freezeout() {
        let r = compute_relic_density(&RelicConfig::default());
        let p = r.evolution.iter().find(|p| p.x > 100.0);
        if let Some(p) = p {
            assert!(p.y > p.y_eq, "Y={}, Y_eq={}", p.y, p.y_eq);
        }
    }

    #[test]
    fn early_yield_tracks_equilibrium() {
        let r = compute_relic_density(&RelicConfig::default());
        let early: Vec<_> = r
            .evolution
            .iter()
            .filter(|p| p.x > 1.0 && p.x < r.x_freeze_out - 2.0)
            .collect();
        for p in &early {
            if p.y_eq > 0.0 {
                let ratio = p.y / p.y_eq;
                assert!((ratio - 1.0).abs() < 0.01, "x={}, Y/Y_eq={}", p.x, ratio);
            }
        }
    }

    #[test]
    fn numerical_solver_agrees_with_analytic() {
        let config = RelicConfig::default();
        let a = compute_relic_density(&config);
        let n = compute_relic_density_numerical(&config);
        let rel = (a.omega_h2 - n.omega_h2).abs() / a.omega_h2.max(1e-30);
        assert!(
            rel < 0.5,
            "analytic={}, numerical={}, diff={}",
            a.omega_h2,
            n.omega_h2,
            rel
        );
    }

    #[test]
    fn y_infinity_scaling_with_cross_section() {
        let r1 = compute_relic_density(&RelicConfig {
            sigma_v_a: 3e-26,
            ..Default::default()
        });
        let r2 = compute_relic_density(&RelicConfig {
            sigma_v_a: 6e-26,
            ..Default::default()
        });
        let ratio = r1.y_infinity / r2.y_infinity;
        assert!(ratio > 1.3 && ratio < 3.0, "Y_inf ratio = {}", ratio);
    }
}
