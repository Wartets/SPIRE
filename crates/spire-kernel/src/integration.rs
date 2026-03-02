//! # Integration — Monte Carlo Phase Space Integration Engine
//!
//! This module provides a modular framework for numerical integration of
//! matrix elements over Lorentz-invariant phase space. It is the engine
//! that transforms symbolic scattering amplitudes into measurable
//! observables: total cross-sections and differential distributions.
//!
//! ## Architecture
//!
//! The design is built on two orthogonal trait abstractions:
//!
//! - [`MonteCarloIntegrator`]: Defines *how* the integration is performed
//!   (sampling strategy, variance reduction, etc.). Implementations include
//!   flat uniform sampling ([`UniformIntegrator`]) and future adaptive
//!   algorithms (VEGAS, multi-channel, etc.).
//!
//! - [`PhaseSpaceGenerator`](crate::kinematics::PhaseSpaceGenerator): Defines
//!   *how* phase-space points are generated. The integrator is agnostic to the
//!   generation algorithm.
//!
//! ## Cross-Section Formula
//!
//! For a $2 \to N$ scattering process at centre-of-mass energy $\sqrt{s}$:
//!
//! $$\sigma = \frac{1}{2s} \int |\mathcal{M}|^2 \, d\Phi_N$$
//!
//! The Monte Carlo estimate with $N_\text{ev}$ events is:
//!
//! $$\sigma \approx \frac{1}{2s} \cdot \frac{1}{N_\text{ev}}
//!   \sum_{i=1}^{N_\text{ev}} |\mathcal{M}(p_i)|^2 \cdot w_i$$
//!
//! where $w_i$ is the LIPS weight from the phase-space generator.
//!
//! ## Statistical Uncertainty
//!
//! The standard error on the Monte Carlo estimate is:
//!
//! $$\delta\sigma = \frac{1}{2s} \sqrt{\frac{\langle f^2 \rangle - \langle f \rangle^2}{N_\text{ev}}}$$
//!
//! where $f_i = |\mathcal{M}(p_i)|^2 \cdot w_i$.

use serde::{Deserialize, Serialize};

use crate::kinematics::{PhaseSpaceGenerator, PhaseSpacePoint};
use crate::SpireResult;

// ===========================================================================
// Integration Result
// ===========================================================================

/// The result of a Monte Carlo integration.
///
/// Contains the estimated integral value (e.g., cross-section), its
/// statistical uncertainty, and diagnostic information about the
/// integration run.
///
/// # Units
///
/// When used for cross-section calculations, the `value` and `error` are
/// in natural units (GeV⁻²). To convert to picobarns:
///
/// $$1 \text{ GeV}^{-2} = 0.3894 \times 10^9 \text{ pb}$$
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationResult {
    /// The estimated integral value (e.g., cross-section in GeV⁻²).
    pub value: f64,
    /// The statistical uncertainty (standard error) on the estimate.
    pub error: f64,
    /// The number of phase-space events evaluated.
    pub events_evaluated: usize,
    /// The fraction of events that contributed (efficiency).
    /// For flat Monte Carlo this is always 1.0.
    pub efficiency: f64,
    /// The relative error $\delta\sigma / \sigma$.
    pub relative_error: f64,
}

impl IntegrationResult {
    /// Convert the result from natural units (GeV⁻²) to picobarns.
    ///
    /// Uses the conversion factor $1 \text{ GeV}^{-2} = 0.3894 \times 10^9 \text{ pb}$.
    pub fn to_picobarns(&self) -> IntegrationResult {
        let conversion = 0.3894e9; // GeV^{-2} → pb
        IntegrationResult {
            value: self.value * conversion,
            error: self.error * conversion,
            events_evaluated: self.events_evaluated,
            efficiency: self.efficiency,
            relative_error: self.relative_error,
        }
    }
}

// ===========================================================================
// Monte Carlo Integrator Trait
// ===========================================================================

/// Trait for Monte Carlo integration algorithms.
///
/// Defines the contract for integrating a scalar function over $N$-body
/// phase space using a [`PhaseSpaceGenerator`]. The integrand is typically
/// $|\mathcal{M}|^2$ (the squared matrix element).
///
/// # Extensibility
///
/// This trait enables swapping integration algorithms without changing the
/// physics code:
///
/// - [`UniformIntegrator`]: Flat (unweighted) Monte Carlo — baseline.
/// - Future `VegasIntegrator`: Adaptive importance sampling for peaked
///   integrands (e.g., near resonances).
/// - Future `MultiChannelIntegrator`: Multiple phase-space mappings for
///   complex topologies.
///
/// # Type Parameters
///
/// The integrand is passed as a closure `Fn(&PhaseSpacePoint) -> f64`,
/// enabling maximum flexibility in what quantity is being integrated.
pub trait MonteCarloIntegrator: Send + Sync {
    /// Perform the Monte Carlo integration.
    ///
    /// # Arguments
    ///
    /// * `integrand` — A function mapping a phase-space point to a scalar
    ///   value (typically $|\mathcal{M}|^2$).
    /// * `generator` — The phase-space generator producing random events.
    /// * `cms_energy` — Centre-of-mass energy $\sqrt{s}$ in GeV.
    /// * `final_masses` — Rest masses of the final-state particles.
    /// * `num_events` — Number of Monte Carlo events to generate.
    ///
    /// # Returns
    ///
    /// An [`IntegrationResult`] containing the integral estimate and
    /// statistical uncertainty.
    fn integrate<F>(
        &self,
        integrand: F,
        generator: &mut dyn PhaseSpaceGenerator,
        cms_energy: f64,
        final_masses: &[f64],
        num_events: usize,
    ) -> SpireResult<IntegrationResult>
    where
        F: Fn(&PhaseSpacePoint) -> f64;

    /// The name of this integration algorithm.
    fn name(&self) -> &str;
}

// ===========================================================================
// Uniform (Flat) Monte Carlo Integrator
// ===========================================================================

/// A flat (uniform) Monte Carlo integrator.
///
/// The simplest possible Monte Carlo integration strategy: evaluate the
/// integrand at uniformly-distributed phase-space points and compute the
/// weighted average.
///
/// $$I = \frac{1}{N} \sum_{i=1}^{N} f(p_i) \cdot w_i$$
///
/// The statistical error is:
///
/// $$\delta I = \sqrt{\frac{\langle (fw)^2 \rangle - \langle fw \rangle^2}{N}}$$
///
/// # Performance
///
/// For smooth integrands this converges as $1/\sqrt{N}$. For peaked
/// integrands (near resonances or threshold), the VEGAS algorithm
/// (implementing the same [`MonteCarloIntegrator`] trait) provides
/// significantly better convergence.
pub struct UniformIntegrator;

impl UniformIntegrator {
    /// Create a new uniform integrator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for UniformIntegrator {
    fn default() -> Self {
        Self::new()
    }
}

impl MonteCarloIntegrator for UniformIntegrator {
    fn integrate<F>(
        &self,
        integrand: F,
        generator: &mut dyn PhaseSpaceGenerator,
        cms_energy: f64,
        final_masses: &[f64],
        num_events: usize,
    ) -> SpireResult<IntegrationResult>
    where
        F: Fn(&PhaseSpacePoint) -> f64,
    {
        if num_events == 0 {
            return Ok(IntegrationResult {
                value: 0.0,
                error: 0.0,
                events_evaluated: 0,
                efficiency: 0.0,
                relative_error: f64::INFINITY,
            });
        }

        let mut sum_fw = 0.0;
        let mut sum_fw2 = 0.0;
        let mut n_success = 0usize;

        for _ in 0..num_events {
            let event = generator.generate_event(cms_energy, final_masses)?;
            let f_val = integrand(&event);

            if !f_val.is_finite() || !event.weight.is_finite() {
                continue;
            }

            let fw = f_val * event.weight;
            sum_fw += fw;
            sum_fw2 += fw * fw;
            n_success += 1;
        }

        if n_success == 0 {
            return Ok(IntegrationResult {
                value: 0.0,
                error: 0.0,
                events_evaluated: num_events,
                efficiency: 0.0,
                relative_error: f64::INFINITY,
            });
        }

        let n_f = n_success as f64;

        // Mean and variance of fw.
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();

        let efficiency = n_success as f64 / num_events as f64;
        let relative_error = if mean.abs() > 1e-300 {
            std_error / mean.abs()
        } else {
            f64::INFINITY
        };

        Ok(IntegrationResult {
            value: mean,
            error: std_error,
            events_evaluated: n_success,
            efficiency,
            relative_error,
        })
    }

    fn name(&self) -> &str {
        "Uniform Monte Carlo"
    }
}

// ===========================================================================
// Cross-Section Calculation Utilities
// ===========================================================================

/// Compute a total cross-section via Monte Carlo integration.
///
/// Combines the integrator and phase-space generator to estimate:
///
/// $$\sigma = \frac{1}{2s} \int |\mathcal{M}|^2 \, d\Phi_N$$
///
/// # Arguments
///
/// * `amplitude_sq_fn` — A closure returning $|\mathcal{M}|^2$ for a given
///   phase-space point.
/// * `integrator` — The Monte Carlo integration algorithm.
/// * `generator` — The phase-space generator.
/// * `cms_energy` — Centre-of-mass energy in GeV.
/// * `final_masses` — Final-state particle masses in GeV.
/// * `num_events` — Number of events to sample.
///
/// # Returns
///
/// An [`IntegrationResult`] with the cross-section in natural units (GeV⁻²).
pub fn compute_cross_section<F, I>(
    amplitude_sq_fn: F,
    integrator: &I,
    generator: &mut dyn PhaseSpaceGenerator,
    cms_energy: f64,
    final_masses: &[f64],
    num_events: usize,
) -> SpireResult<IntegrationResult>
where
    F: Fn(&PhaseSpacePoint) -> f64,
    I: MonteCarloIntegrator,
{
    let s = cms_energy * cms_energy;
    let flux_factor = 2.0 * s;

    if flux_factor < 1e-300 {
        return Err(crate::SpireError::KinematicsForbidden(
            "Centre-of-mass energy too small for cross-section calculation".into(),
        ));
    }

    // Integrate |M|² over phase space.
    let raw_result = integrator.integrate(
        &amplitude_sq_fn,
        generator,
        cms_energy,
        final_masses,
        num_events,
    )?;

    // Divide by the flux factor: σ = (1/2s) ∫ |M|² dΦ
    Ok(IntegrationResult {
        value: raw_result.value / flux_factor,
        error: raw_result.error / flux_factor,
        events_evaluated: raw_result.events_evaluated,
        efficiency: raw_result.efficiency,
        relative_error: raw_result.relative_error,
    })
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinematics::RamboGenerator;

    #[test]
    fn integration_result_serde() {
        let result = IntegrationResult {
            value: 1.23e-6,
            error: 4.56e-8,
            events_evaluated: 10000,
            efficiency: 1.0,
            relative_error: 0.037,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: IntegrationResult = serde_json::from_str(&json).unwrap();
        assert!((back.value - 1.23e-6).abs() < 1e-15);
        assert_eq!(back.events_evaluated, 10000);
    }

    #[test]
    fn integration_result_to_picobarns() {
        let result = IntegrationResult {
            value: 1.0,
            error: 0.1,
            events_evaluated: 1000,
            efficiency: 1.0,
            relative_error: 0.1,
        };
        let pb = result.to_picobarns();
        assert!((pb.value - 0.3894e9).abs() < 1e3);
    }

    #[test]
    fn uniform_integrator_constant_integrand() {
        // Integrate f(x) = 1.0 over 2-body massless phase space.
        // The result should be approximately the RAMBO weight average.
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];

        let result = integrator.integrate(
            |_event| 1.0,
            &mut gen,
            cms,
            &masses,
            5000,
        ).unwrap();

        // For constant integrand, value = average weight.
        assert!(result.value > 0.0, "Integral should be positive");
        assert!(result.error >= 0.0, "Error should be non-negative");
        assert!(result.events_evaluated > 0);
        assert!((result.efficiency - 1.0).abs() < 1e-12);
    }

    #[test]
    fn uniform_integrator_zero_events() {
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let result = integrator.integrate(
            |_| 1.0,
            &mut gen,
            100.0,
            &[0.0, 0.0],
            0,
        ).unwrap();
        assert_eq!(result.events_evaluated, 0);
        assert_eq!(result.value, 0.0);
    }

    #[test]
    fn uniform_integrator_name() {
        let integrator = UniformIntegrator::new();
        assert_eq!(integrator.name(), "Uniform Monte Carlo");
    }

    #[test]
    fn cross_section_massless_two_body() {
        // Compute a cross-section with a constant |M|² = 1.
        // This tests that the flux factor (1/2s) is applied correctly.
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];

        let result = compute_cross_section(
            |_| 1.0,
            &integrator,
            &mut gen,
            cms,
            &masses,
            10000,
        ).unwrap();

        // σ = (1/2s) * ∫ |M|² dΦ₂ ≈ (1/2s) * <w>
        let s = cms * cms;
        assert!(result.value > 0.0);
        assert!(result.value < 1.0 / (2.0 * s) * 10.0); // Sanity bound
        assert!(result.error >= 0.0);
    }

    #[test]
    fn cross_section_statistical_convergence() {
        // Verify that error decreases with more events (1/√N scaling).
        let integrator = UniformIntegrator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];

        let mut gen1 = RamboGenerator::new();
        let r1 = compute_cross_section(
            |_| 1.0,
            &integrator,
            &mut gen1,
            cms,
            &masses,
            1000,
        ).unwrap();

        let mut gen2 = RamboGenerator::new();
        let r2 = compute_cross_section(
            |_| 1.0,
            &integrator,
            &mut gen2,
            cms,
            &masses,
            100000,
        ).unwrap();

        // With 100x more events, relative error should decrease by ~10x.
        // Allow generous margins for statistical fluctuations.
        assert!(
            r2.relative_error < r1.relative_error,
            "More events should reduce relative error: {} vs {}",
            r2.relative_error, r1.relative_error
        );
    }

    #[test]
    fn cross_section_massive_particles() {
        // Cross-section for massive final-state particles.
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 200.0;
        let masses = [0.10566, 0.10566]; // muon masses

        let result = compute_cross_section(
            |_| 1.0,
            &integrator,
            &mut gen,
            cms,
            &masses,
            5000,
        ).unwrap();

        assert!(result.value > 0.0);
        assert!(result.events_evaluated > 0);
    }

    #[test]
    fn cross_section_three_body() {
        // 3-body cross-section.
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 10.0;
        let masses = [0.0, 0.0, 0.0];

        let result = compute_cross_section(
            |_| 1.0,
            &integrator,
            &mut gen,
            cms,
            &masses,
            5000,
        ).unwrap();

        assert!(result.value > 0.0);
        assert!(result.error >= 0.0);
    }

    #[test]
    fn monte_carlo_integrator_concrete_dispatch() {
        // Verify the trait works through a generic function (static dispatch).
        fn run_integration<I: MonteCarloIntegrator>(integrator: &I) -> IntegrationResult {
            let mut gen = RamboGenerator::new();
            integrator.integrate(
                |_| 42.0,
                &mut gen,
                100.0,
                &[0.0, 0.0],
                100,
            ).unwrap()
        }

        let integrator = UniformIntegrator::new();
        let result = run_integration(&integrator);
        assert!(result.value > 0.0);
    }
}
