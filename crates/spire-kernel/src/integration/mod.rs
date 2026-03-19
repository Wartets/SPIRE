//! # Integration - Monte Carlo Phase Space Integration Engine
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
//! - [`PhaseSpaceGenerator`]: Defines *how* phase-space points are generated.
//!   The integrator is agnostic to the generation algorithm.
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
//!
//! For weighted events, the implementation uses the empirical first and
//! second moments of $f$:
//!
//! $$\hat{\mu} = \frac{1}{N}\sum_i f_i,
//! \qquad
//! \widehat{\mathrm{Var}}(f)=\max\!\left(\frac{1}{N}\sum_i f_i^2-\hat{\mu}^2,0\right)$$
//!
//! and reports $\delta\hat{\mu}=\sqrt{\widehat{\mathrm{Var}}(f)/N}$.
//!
//! ## Relative Error and Convergence
//!
//! The reported relative error is
//! $\delta\hat{\mu}/|\hat{\mu}|$, exhibiting the expected Monte Carlo
//! scaling $\propto N^{-1/2}$ for finite-variance integrands.  Cut-aware
//! routines normalise by the total generated event count so acceptance losses
//! are physically reflected in the estimate.
//!
//! ## Hadronic Convolution Layer
//!
//! The hadronic helpers implement PDF convolution on top of partonic
//! integration:
//!
//! $$\sigma_{H_1H_2} = \sum_{a,b}\int dx_1 dx_2\;f_a^{H_1}(x_1,Q^2)
//! f_b^{H_2}(x_2,Q^2)\,\hat{\sigma}_{ab}(\hat{s})$$
//!
//! with $\hat{s}=x_1x_2S$.  This keeps matrix-element evaluation and PDF
//! sampling cleanly separated while sharing the same statistical machinery.

#[cfg(feature = "gpu")]
pub mod gpu;

use rayon::prelude::*;
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
// Integrator Backend - Unified CPU / GPU Abstraction
// ===========================================================================

/// The hardware backend for amplitude evaluation.
///
/// This enum selects whether the squared matrix element $|\mathcal{M}|^2$
/// is evaluated on the CPU (using Rayon thread-parallel loops) or on the
/// GPU (using a WGSL compute shader via `wgpu`).
///
/// # Fallback Behavior
///
/// If [`IntegratorBackend::Gpu`] is selected but no GPU adapter is
/// available (or the `gpu` feature is not compiled in), the integration
/// functions transparently fall back to [`IntegratorBackend::Cpu`] and
/// emit a diagnostic warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IntegratorBackend {
    /// Evaluate amplitudes on the CPU using Rayon parallelism.
    #[default]
    Cpu,
    /// Evaluate amplitudes on the GPU using a WGSL compute shader.
    ///
    /// Requires the `gpu` feature to be enabled at compile time and a
    /// compatible GPU adapter at runtime.
    Gpu,
}

impl IntegratorBackend {
    /// Returns `true` if GPU integration is available at runtime.
    ///
    /// This checks both compile-time feature availability and runtime
    /// adapter presence.
    pub fn gpu_available() -> bool {
        #[cfg(feature = "gpu")]
        {
            gpu::GpuIntegrator::new().is_some()
        }
        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }

    /// Return a human-readable description of the backend.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU (Rayon thread-parallel)",
            Self::Gpu => "GPU (WebGPU compute shader)",
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
/// - [`UniformIntegrator`]: Flat (unweighted) Monte Carlo - baseline.
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
    /// * `integrand` - A function mapping a phase-space point to a scalar
    ///   value (typically $|\mathcal{M}|^2$).
    /// * `generator` - The phase-space generator producing random events.
    /// * `cms_energy` - Centre-of-mass energy $\sqrt{s}$ in GeV.
    /// * `final_masses` - Rest masses of the final-state particles.
    /// * `num_events` - Number of Monte Carlo events to generate.
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

impl UniformIntegrator {
    /// Parallel variant of [`MonteCarloIntegrator::integrate`].
    ///
    /// Events are generated sequentially (the generator is not `Send`),
    /// then the integrand is evaluated in parallel using rayon's work-stealing
    /// thread pool. For expensive matrix elements this can provide near-linear
    /// speedup on multi-core machines.
    ///
    /// The integrand `F` must be `Send + Sync` to allow parallel evaluation.
    pub fn integrate_parallel<F>(
        &self,
        integrand: F,
        generator: &mut dyn PhaseSpaceGenerator,
        cms_energy: f64,
        final_masses: &[f64],
        num_events: usize,
    ) -> SpireResult<IntegrationResult>
    where
        F: Fn(&PhaseSpacePoint) -> f64 + Send + Sync,
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

        // Phase 1: sequential event generation.
        let mut events = Vec::with_capacity(num_events);
        for _ in 0..num_events {
            events.push(generator.generate_event(cms_energy, final_masses)?);
        }

        // Phase 2: parallel integrand evaluation & reduction.
        let (sum_fw, sum_fw2, n_success) = events
            .par_iter()
            .map(|event| {
                let f_val = integrand(event);
                if !f_val.is_finite() || !event.weight.is_finite() {
                    (0.0, 0.0, 0_usize)
                } else {
                    let fw = f_val * event.weight;
                    (fw, fw * fw, 1_usize)
                }
            })
            .reduce(
                || (0.0, 0.0, 0_usize),
                |(a_fw, a_fw2, a_n), (b_fw, b_fw2, b_n)| (a_fw + b_fw, a_fw2 + b_fw2, a_n + b_n),
            );

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

    /// Integrate with kinematic cuts applied before the integrand.
    ///
    /// Events that fail **any** cut are discarded (zero contribution).
    /// The efficiency metric reports the fraction of events passing all cuts.
    ///
    /// # Arguments
    ///
    /// * `integrand` - Squared matrix element $|\mathcal{M}|^2$.
    /// * `cuts` - Kinematic cuts to apply sequentially.
    /// * `generator` - Phase-space generator.
    /// * `cms_energy` - Centre-of-mass energy in GeV.
    /// * `final_masses` - Final-state particle masses.
    /// * `num_events` - Number of phase-space events to generate.
    pub fn integrate_with_cuts<F>(
        &self,
        integrand: F,
        cuts: &[&dyn crate::scripting::KinematicCut],
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
        let mut n_passed_cuts = 0usize;

        for _ in 0..num_events {
            let event = generator.generate_event(cms_energy, final_masses)?;

            // Apply all cuts.
            let passed = cuts.iter().all(|cut| cut.is_passed(&event));
            if !passed {
                continue;
            }
            n_passed_cuts += 1;

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
                efficiency: n_passed_cuts as f64 / num_events as f64,
                relative_error: f64::INFINITY,
            });
        }

        // Normalise by the *total* number of events (not just those passing
        // cuts) so that the integral correctly accounts for the cut efficiency.
        let n_f = num_events as f64;
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();

        let efficiency = n_passed_cuts as f64 / num_events as f64;
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

    /// Parallel integration with kinematic cuts.
    ///
    /// Combines the parallel evaluation strategy of
    /// [`integrate_parallel`](Self::integrate_parallel) with per-event
    /// kinematic cut filtering. Events failing any cut contribute zero.
    pub fn integrate_with_cuts_parallel<F>(
        &self,
        integrand: F,
        cuts: &[&dyn crate::scripting::KinematicCut],
        generator: &mut dyn PhaseSpaceGenerator,
        cms_energy: f64,
        final_masses: &[f64],
        num_events: usize,
    ) -> SpireResult<IntegrationResult>
    where
        F: Fn(&PhaseSpacePoint) -> f64 + Send + Sync,
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

        // Phase 1: sequential event generation.
        let mut events = Vec::with_capacity(num_events);
        for _ in 0..num_events {
            events.push(generator.generate_event(cms_energy, final_masses)?);
        }

        // Phase 2: parallel evaluation with cut filtering.
        let (sum_fw, sum_fw2, n_success, n_passed) = events
            .par_iter()
            .map(|event| {
                let passed = cuts.iter().all(|cut| cut.is_passed(event));
                if !passed {
                    return (0.0, 0.0, 0_usize, 0_usize);
                }
                let f_val = integrand(event);
                if !f_val.is_finite() || !event.weight.is_finite() {
                    (0.0, 0.0, 0_usize, 1_usize)
                } else {
                    let fw = f_val * event.weight;
                    (fw, fw * fw, 1_usize, 1_usize)
                }
            })
            .reduce(
                || (0.0, 0.0, 0_usize, 0_usize),
                |(a0, a1, a2, a3), (b0, b1, b2, b3)| (a0 + b0, a1 + b1, a2 + b2, a3 + b3),
            );

        if n_success == 0 {
            return Ok(IntegrationResult {
                value: 0.0,
                error: 0.0,
                events_evaluated: num_events,
                efficiency: n_passed as f64 / num_events as f64,
                relative_error: f64::INFINITY,
            });
        }

        let n_f = num_events as f64;
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();

        let efficiency = n_passed as f64 / num_events as f64;
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
/// * `amplitude_sq_fn` - A closure returning $|\mathcal{M}|^2$ for a given
///   phase-space point.
/// * `integrator` - The Monte Carlo integration algorithm.
/// * `generator` - The phase-space generator.
/// * `cms_energy` - Centre-of-mass energy in GeV.
/// * `final_masses` - Final-state particle masses in GeV.
/// * `num_events` - Number of events to sample.
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
// Hadronic (PDF-Convoluted) Cross-Section Integration
// ===========================================================================

/// Configuration for a hadronic cross-section calculation.
///
/// This bundles the two initial-state hadrons, their associated PDFs,
/// and the beam energy. The convolution integral is:
///
/// $$\sigma_{H_1 H_2} = \sum_{a,b} \int_0^1 dx_1 \int_0^1 dx_2 \;
///   f_a^{H_1}(x_1, Q^2) \, f_b^{H_2}(x_2, Q^2) \;
///   \hat{\sigma}_{ab}(\hat{s})$$
///
/// where $\hat{s} = x_1 x_2 S$ and $S$ is the hadronic centre-of-mass
/// energy squared.
pub struct HadronicIntegratorConfig<'a> {
    /// PDF provider for beam 1.
    pub pdf1: &'a dyn crate::pdf::PdfProvider,
    /// PDF provider for beam 2.
    pub pdf2: &'a dyn crate::pdf::PdfProvider,
    /// Hadron species for beam 1.
    pub beam1: crate::pdf::Hadron,
    /// Hadron species for beam 2.
    pub beam2: crate::pdf::Hadron,
    /// Hadronic centre-of-mass energy squared $S$ in GeV².
    pub s_hadronic: f64,
    /// Factorisation scale $Q^2$ in GeV². If `None`, uses $\hat{s}$.
    pub q2_fixed: Option<f64>,
}

/// Perform the hadronic cross-section convolution via Monte Carlo sampling.
///
/// For each MC event the algorithm:
/// 1. Samples $x_1, x_2$ uniformly in $[\sqrt{\tau_0}, 1]$.
/// 2. Computes $\hat{s} = x_1 x_2 S$.
/// 3. Checks the kinematic threshold: $\hat{s} \geq (\sum m_f)^2$.
/// 4. Evaluates the PDF luminosity $\mathcal{L}_{ab} = f_a(x_1, Q^2) \cdot f_b(x_2, Q^2)$.
/// 5. Generates a partonic phase-space event at $\sqrt{\hat{s}}$ via
///    the supplied generator, and evaluates $|\mathcal{M}|^2 \cdot w_\text{PS}$.
/// 6. Accumulates with the Jacobian $(1 - \sqrt{\tau_0})^2$.
///
/// # Arguments
///
/// * `config` - Hadronic beam/PDF configuration.
/// * `final_masses` - Rest masses of the final-state partons in GeV.
/// * `amplitude_sq_fn` - Closure returning $|\mathcal{M}|^2$ for a given
///   phase-space point *and* the partonic invariant $\hat{s}$.
///   Signature: `(ps_point, s_hat) -> f64`.
/// * `parton_channels` - List of `(pdg_a, pdg_b)` pairs to sum over.
///   If empty, the Cartesian product of `beam1.parton_content()` and
///   `beam2.parton_content()` is used.
/// * `generator` - Phase-space generator (e.g., RAMBO).
/// * `num_events` - Number of MC samples *per channel*.
///
/// # Returns
///
/// A [`HadronicCrossSectionResult`](crate::pdf::HadronicCrossSectionResult)
/// containing the convolution result in both natural units and picobarns.
pub fn compute_hadronic_cross_section<F>(
    config: &HadronicIntegratorConfig<'_>,
    final_masses: &[f64],
    amplitude_sq_fn: F,
    parton_channels: &[(i32, i32)],
    generator: &mut dyn crate::kinematics::PhaseSpaceGenerator,
    num_events: usize,
) -> SpireResult<crate::pdf::HadronicCrossSectionResult>
where
    F: Fn(&PhaseSpacePoint, f64) -> f64,
{
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;

    let s_had = config.s_hadronic;
    if s_had <= 0.0 {
        return Err(crate::SpireError::KinematicsForbidden(
            "Hadronic centre-of-mass energy squared must be positive".into(),
        ));
    }

    let mass_threshold: f64 = final_masses.iter().sum();
    let tau_0 = (mass_threshold * mass_threshold) / s_had;
    if tau_0 >= 1.0 {
        return Err(crate::SpireError::KinematicsForbidden(format!(
            "Beam energy √S = {:.4} GeV below threshold {:.4} GeV",
            s_had.sqrt(),
            mass_threshold,
        )));
    }

    // Determine parton channels.
    let channels: Vec<(i32, i32)> = if parton_channels.is_empty() {
        let p1 = config.beam1.parton_content();
        let p2 = config.beam2.parton_content();
        let mut ch = Vec::with_capacity(p1.len() * p2.len());
        for &a in p1 {
            for &b in p2 {
                ch.push((a, b));
            }
        }
        ch
    } else {
        parton_channels.to_vec()
    };

    if channels.is_empty() {
        return Ok(crate::pdf::HadronicCrossSectionResult {
            cross_section: 0.0,
            uncertainty: 0.0,
            cross_section_pb: 0.0,
            uncertainty_pb: 0.0,
            events_evaluated: 0,
            relative_error: f64::INFINITY,
            beam_energy_sq: s_had,
            pdf_name: config.pdf1.name().to_string(),
            beam_description: format!("{} {} → X", config.beam1.label(), config.beam2.label()),
        });
    }

    let x_min = tau_0.sqrt();
    let jacobian = (1.0 - x_min) * (1.0 - x_min);

    let mut rng = StdRng::from_entropy();

    let mut total_sum = 0.0;
    let mut total_sum2 = 0.0;
    let mut total_events = 0usize;

    let gev2_to_pb = 0.3894e9;

    for &(pdg_a, pdg_b) in &channels {
        for _ in 0..num_events {
            // Sample x1, x2 uniformly in [x_min, 1].
            let x1 = x_min + (1.0 - x_min) * rng.gen::<f64>();
            let x2 = x_min + (1.0 - x_min) * rng.gen::<f64>();

            let s_hat = x1 * x2 * s_had;
            let sqrt_s_hat = s_hat.sqrt();

            // Kinematic threshold check.
            if sqrt_s_hat < mass_threshold {
                total_events += 1;
                continue;
            }

            // Factorisation scale: fixed or dynamic (= ŝ).
            let q2 = config.q2_fixed.unwrap_or(s_hat);

            // PDF luminosity.
            let f1 = config.pdf1.evaluate(pdg_a, x1, q2);
            let f2 = config.pdf2.evaluate(pdg_b, x2, q2);
            let lumi = f1 * f2;

            if lumi <= 0.0 {
                total_events += 1;
                continue;
            }

            // Generate partonic phase-space event.
            let ps_event = generator.generate_event(sqrt_s_hat, final_masses)?;

            // Partonic |M|² × PS weight.
            let msq = amplitude_sq_fn(&ps_event, s_hat);
            if !msq.is_finite() || !ps_event.weight.is_finite() {
                total_events += 1;
                continue;
            }

            // Full integrand:
            // σ_had = Σ_{a,b} ∫dx₁ dx₂ f_a(x₁) f_b(x₂) × (1/2ŝ) |M|² × w_PS
            // MC estimate: Jacobian × <lumi × (1/2ŝ) × |M|² × w_PS>
            let flux_partonic = 2.0 * s_hat;
            let contribution = jacobian * lumi * msq * ps_event.weight / flux_partonic;

            total_sum += contribution;
            total_sum2 += contribution * contribution;
            total_events += 1;
        }
    }

    let n_total = total_events as f64;
    if n_total < 1.0 {
        return Ok(crate::pdf::HadronicCrossSectionResult {
            cross_section: 0.0,
            uncertainty: 0.0,
            cross_section_pb: 0.0,
            uncertainty_pb: 0.0,
            events_evaluated: 0,
            relative_error: f64::INFINITY,
            beam_energy_sq: s_had,
            pdf_name: config.pdf1.name().to_string(),
            beam_description: format!("{} {} → X", config.beam1.label(), config.beam2.label()),
        });
    }

    let mean = total_sum / n_total;
    let variance = (total_sum2 / n_total - mean * mean).max(0.0);
    let std_error = (variance / n_total).sqrt();

    let relative_error = if mean.abs() > 1e-300 {
        std_error / mean.abs()
    } else {
        f64::INFINITY
    };

    Ok(crate::pdf::HadronicCrossSectionResult {
        cross_section: mean,
        uncertainty: std_error,
        cross_section_pb: mean * gev2_to_pb,
        uncertainty_pb: std_error * gev2_to_pb,
        events_evaluated: total_events,
        relative_error,
        beam_energy_sq: s_had,
        pdf_name: config.pdf1.name().to_string(),
        beam_description: format!("{} {} → X", config.beam1.label(), config.beam2.label()),
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

        let result = integrator
            .integrate(|_event| 1.0, &mut gen, cms, &masses, 5000)
            .unwrap();

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
        let result = integrator
            .integrate(|_| 1.0, &mut gen, 100.0, &[0.0, 0.0], 0)
            .unwrap();
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

        let result =
            compute_cross_section(|_| 1.0, &integrator, &mut gen, cms, &masses, 10000).unwrap();

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
        let r1 =
            compute_cross_section(|_| 1.0, &integrator, &mut gen1, cms, &masses, 1000).unwrap();

        let mut gen2 = RamboGenerator::new();
        let r2 =
            compute_cross_section(|_| 1.0, &integrator, &mut gen2, cms, &masses, 100000).unwrap();

        // With 100x more events, relative error should decrease by ~10x.
        // Allow generous margins for statistical fluctuations.
        assert!(
            r2.relative_error < r1.relative_error,
            "More events should reduce relative error: {} vs {}",
            r2.relative_error,
            r1.relative_error
        );
    }

    #[test]
    fn cross_section_massive_particles() {
        // Cross-section for massive final-state particles.
        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 200.0;
        let masses = [0.10566, 0.10566]; // muon masses

        let result =
            compute_cross_section(|_| 1.0, &integrator, &mut gen, cms, &masses, 5000).unwrap();

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

        let result =
            compute_cross_section(|_| 1.0, &integrator, &mut gen, cms, &masses, 5000).unwrap();

        assert!(result.value > 0.0);
        assert!(result.error >= 0.0);
    }

    #[test]
    fn monte_carlo_integrator_concrete_dispatch() {
        // Verify the trait works through a generic function (static dispatch).
        fn run_integration<I: MonteCarloIntegrator>(integrator: &I) -> IntegrationResult {
            let mut gen = RamboGenerator::new();
            integrator
                .integrate(|_| 42.0, &mut gen, 100.0, &[0.0, 0.0], 100)
                .unwrap()
        }

        let integrator = UniformIntegrator::new();
        let result = run_integration(&integrator);
        assert!(result.value > 0.0);
    }

    // -----------------------------------------------------------------------
    // Hadronic (PDF-convoluted) cross-section tests
    // -----------------------------------------------------------------------

    #[test]
    fn hadronic_cross_section_flat_pdf_positive() {
        use crate::pdf::{FlatPdf, Hadron};

        let pdf = FlatPdf::new(1.0);
        let config = HadronicIntegratorConfig {
            pdf1: &pdf,
            pdf2: &pdf,
            beam1: Hadron::Proton,
            beam2: Hadron::Proton,
            s_hadronic: 100.0 * 100.0, // 100 GeV
            q2_fixed: Some(100.0),
        };

        let mut gen = RamboGenerator::new();
        // Constant |M|² = 1, only one channel for simplicity.
        let result = compute_hadronic_cross_section(
            &config,
            &[0.0, 0.0],
            |_ps, _s_hat| 1.0,
            &[(2, -2)], // u ubar channel only
            &mut gen,
            5000,
        )
        .unwrap();

        assert!(
            result.cross_section > 0.0,
            "Hadronic xsec should be positive"
        );
        assert!(result.uncertainty > 0.0, "Should have non-zero error");
        assert!(
            result.cross_section_pb > 0.0,
            "pb conversion should be positive"
        );
        assert_eq!(result.beam_energy_sq, 10000.0);
        assert!(result.beam_description.contains("p"));
    }

    #[test]
    fn hadronic_cross_section_below_threshold() {
        use crate::pdf::{FlatPdf, Hadron};

        let pdf = FlatPdf::new(1.0);
        let config = HadronicIntegratorConfig {
            pdf1: &pdf,
            pdf2: &pdf,
            beam1: Hadron::Proton,
            beam2: Hadron::AntiProton,
            s_hadronic: 1.0, // √S = 1 GeV, below 2*80 GeV threshold
            q2_fixed: None,
        };

        let mut gen = RamboGenerator::new();
        let result = compute_hadronic_cross_section(
            &config,
            &[80.0, 80.0], // Two 80 GeV particles
            |_ps, _s_hat| 1.0,
            &[(2, -2)],
            &mut gen,
            100,
        );

        assert!(result.is_err(), "Should fail when below threshold");
    }

    #[test]
    fn hadronic_cross_section_empty_channels() {
        use crate::pdf::{FlatPdf, Hadron};

        let pdf = FlatPdf::new(1.0);
        let config = HadronicIntegratorConfig {
            pdf1: &pdf,
            pdf2: &pdf,
            beam1: Hadron::Proton,
            beam2: Hadron::Proton,
            s_hadronic: 10000.0,
            q2_fixed: Some(100.0),
        };

        let mut gen = RamboGenerator::new();
        let result = compute_hadronic_cross_section(
            &config,
            &[0.0, 0.0],
            |_ps, _s_hat| 1.0,
            &[], // No explicit channels → uses full Cartesian product
            &mut gen,
            100,
        )
        .unwrap();

        // With all channels from proton⊗proton and flat PDF, result should be positive.
        assert!(result.cross_section > 0.0);
        assert!(result.events_evaluated > 0);
    }

    #[test]
    fn hadronic_cross_section_toy_proton_pdf() {
        use crate::pdf::{Hadron, ToyProtonPdf};

        let pdf = ToyProtonPdf::new();
        let config = HadronicIntegratorConfig {
            pdf1: &pdf,
            pdf2: &pdf,
            beam1: Hadron::Proton,
            beam2: Hadron::Proton,
            s_hadronic: 13000.0_f64.powi(2), // 13 TeV
            q2_fixed: Some(1e4),             // Q² = 10⁴ GeV²
        };

        let mut gen = RamboGenerator::new();
        // Only u-ubar channel, constant |M|² = 1.
        let result = compute_hadronic_cross_section(
            &config,
            &[0.0, 0.0],
            |_ps, _s_hat| 1.0,
            &[(2, -2)],
            &mut gen,
            5000,
        )
        .unwrap();

        assert!(result.cross_section > 0.0);
        assert!(result.pdf_name.contains("Toy"));
    }

    #[test]
    fn hadronic_cross_section_serde_roundtrip() {
        use crate::pdf::HadronicCrossSectionResult;

        let result = HadronicCrossSectionResult {
            cross_section: 1.23e-8,
            uncertainty: 4.56e-10,
            cross_section_pb: 4.79e1,
            uncertainty_pb: 1.78e0,
            events_evaluated: 50000,
            relative_error: 0.037,
            beam_energy_sq: 1.69e8,
            pdf_name: "TestPDF".into(),
            beam_description: "p p → X".into(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let back: HadronicCrossSectionResult = serde_json::from_str(&json).unwrap();
        assert!((back.cross_section - 1.23e-8).abs() < 1e-20);
        assert_eq!(back.events_evaluated, 50000);
        assert_eq!(back.pdf_name, "TestPDF");
    }

    #[test]
    fn hadronic_cross_section_negative_s() {
        use crate::pdf::{FlatPdf, Hadron};

        let pdf = FlatPdf::new(1.0);
        let config = HadronicIntegratorConfig {
            pdf1: &pdf,
            pdf2: &pdf,
            beam1: Hadron::Proton,
            beam2: Hadron::Proton,
            s_hadronic: -100.0,
            q2_fixed: None,
        };

        let mut gen = RamboGenerator::new();
        let result = compute_hadronic_cross_section(
            &config,
            &[0.0, 0.0],
            |_ps, _s_hat| 1.0,
            &[(2, -2)],
            &mut gen,
            100,
        );

        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Cut-aware integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn integrate_with_no_cuts_matches_plain() {
        let integrator = UniformIntegrator::new();
        let mut gen1 = RamboGenerator::new();
        let mut gen2 = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];
        let n = 2000;

        let plain = integrator
            .integrate(|_| 1.0, &mut gen1, cms, &masses, n)
            .unwrap();
        let cuts: Vec<&dyn crate::scripting::KinematicCut> = vec![];
        let with_cuts = integrator
            .integrate_with_cuts(|_| 1.0, &cuts, &mut gen2, cms, &masses, n)
            .unwrap();

        // Same seed → same events → same result.
        assert!((plain.value - with_cuts.value).abs() < 1e-12);
    }

    #[test]
    fn integrate_with_cut_reduces_value() {
        use crate::scripting::SpireScriptEngine;

        let engine = SpireScriptEngine::new();
        // Tight pT cut: only events where final-state particle has pT > 49 GeV.
        // For 2-body massless at 100 GeV CMS, pT ≈ 50 sin(θ) ≤ 50 GeV.
        let cut = engine.compile_cut("event.momenta[0].pt() > 49.0").unwrap();
        let cuts: Vec<&dyn crate::scripting::KinematicCut> = vec![&cut];

        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];
        let n = 5000;

        let result = integrator
            .integrate_with_cuts(|_| 1.0, &cuts, &mut gen, cms, &masses, n)
            .unwrap();

        // The cut should discard most events.
        assert!(result.efficiency < 0.5, "Cut should reject most events");
    }

    #[test]
    fn integrate_with_always_true_cut_preserves_value() {
        use crate::scripting::SpireScriptEngine;

        let engine = SpireScriptEngine::new();
        let cut = engine.compile_cut("true").unwrap();
        let cuts: Vec<&dyn crate::scripting::KinematicCut> = vec![&cut];

        let integrator = UniformIntegrator::new();
        let mut gen1 = RamboGenerator::new();
        let mut gen2 = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];
        let n = 3000;

        let plain = integrator
            .integrate(|_| 1.0, &mut gen1, cms, &masses, n)
            .unwrap();
        let with_cuts = integrator
            .integrate_with_cuts(|_| 1.0, &cuts, &mut gen2, cms, &masses, n)
            .unwrap();

        // Same seed → identical results.
        assert!((plain.value - with_cuts.value).abs() < 1e-12);
        assert!((with_cuts.efficiency - 1.0).abs() < 1e-12);
    }

    #[test]
    fn integrate_with_always_false_cut_gives_zero() {
        use crate::scripting::SpireScriptEngine;

        let engine = SpireScriptEngine::new();
        let cut = engine.compile_cut("false").unwrap();
        let cuts: Vec<&dyn crate::scripting::KinematicCut> = vec![&cut];

        let integrator = UniformIntegrator::new();
        let mut gen = RamboGenerator::new();
        let result = integrator
            .integrate_with_cuts(|_| 1.0, &cuts, &mut gen, 100.0, &[0.0, 0.0], 1000)
            .unwrap();

        assert_eq!(result.value, 0.0);
        assert!((result.efficiency - 0.0).abs() < 1e-12);
    }

    #[test]
    fn integrate_with_cuts_parallel_consistency() {
        use crate::scripting::SpireScriptEngine;

        let engine = SpireScriptEngine::new();
        let cut = engine.compile_cut("true").unwrap();
        let cuts: Vec<&dyn crate::scripting::KinematicCut> = vec![&cut];

        let integrator = UniformIntegrator::new();
        let mut gen1 = RamboGenerator::new();
        let mut gen2 = RamboGenerator::new();
        let cms = 100.0;
        let masses = [0.0, 0.0];
        let n = 5000;

        let seq = integrator
            .integrate_with_cuts(|_| 1.0, &cuts, &mut gen1, cms, &masses, n)
            .unwrap();
        let par = integrator
            .integrate_with_cuts_parallel(|_| 1.0, &cuts, &mut gen2, cms, &masses, n)
            .unwrap();

        // Same seed, same events → results should be very close.
        assert!((seq.value - par.value).abs() / seq.value.max(1e-300) < 1e-10);
    }
}
