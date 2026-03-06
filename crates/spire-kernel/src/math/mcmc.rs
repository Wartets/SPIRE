//! # Markov Chain Monte Carlo (MCMC) Engine
//!
//! Implements an **Affine-Invariant Ensemble Sampler** following the
//! Goodman & Weare (2010) stretch-move algorithm.  This sampler is
//! ideally suited for multi-dimensional posterior exploration because
//! its performance is independent of the coordinate system — it adapts
//! automatically to anisotropic and correlated distributions.
//!
//! ## Algorithm Overview
//!
//! The ensemble consists of $N_w$ "walkers" exploring a $D$-dimensional
//! parameter space.  At each step, for each walker $k$:
//!
//! 1. Choose a complementary walker $j \neq k$ uniformly at random.
//! 2. Draw a stretch factor $z$ from the distribution
//!    $g(z) \propto z^{-1/2}$ on $[1/a, a]$ (where $a = 2$ by default).
//! 3. Propose $\mathbf{y} = \mathbf{x}_j + z (\mathbf{x}_k - \mathbf{x}_j)$.
//! 4. Accept with probability
//!    $\min\!\bigl(1,\; z^{D-1} \exp[\ln p(\mathbf{y}) - \ln p(\mathbf{x}_k)]\bigr)$.
//!
//! ## Parallelisation
//!
//! The ensemble is split into two complementary halves.  Within each
//! half the walkers are updated independently (proposals use only the
//! other half), enabling embarrassingly parallel evaluation of the
//! log-probability function via `rayon`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use spire_kernel::math::mcmc::{EnsembleSampler, McmcConfig};
//!
//! let config = McmcConfig {
//!     n_walkers: 32,
//!     n_dim: 2,
//!     n_steps: 1000,
//!     stretch_factor: 2.0,
//! };
//!
//! // Define a log-probability function (e.g., a 2D Gaussian)
//! let log_prob = |params: &[f64]| -> f64 {
//!     -0.5 * (params[0] * params[0] + params[1] * params[1])
//! };
//!
//! let mut sampler = EnsembleSampler::new(config, initial_positions);
//! sampler.run(&log_prob);
//! let chain = sampler.chain();
//! ```
//!
//! ## References
//!
//! - Goodman, J. & Weare, J. (2010). "Ensemble samplers with affine
//!   invariance." *Communications in Applied Mathematics and Computational
//!   Science*, 5(1), 65–80.

use rand::distributions::Distribution;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ===========================================================================
// Configuration
// ===========================================================================

/// Configuration for the ensemble MCMC sampler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McmcConfig {
    /// Number of walkers in the ensemble.  Must be even and ≥ 2 × n_dim.
    pub n_walkers: usize,
    /// Dimensionality of the parameter space.
    pub n_dim: usize,
    /// Number of MCMC steps to execute.
    pub n_steps: usize,
    /// Stretch-move scale parameter $a$ (default: 2.0).
    /// The proposal distribution samples $z \in [1/a, a]$.
    pub stretch_factor: f64,
}

impl Default for McmcConfig {
    fn default() -> Self {
        Self {
            n_walkers: 32,
            n_dim: 2,
            n_steps: 1000,
            stretch_factor: 2.0,
        }
    }
}

// ===========================================================================
// Status Reporting
// ===========================================================================

/// Live status of a running or completed MCMC chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McmcStatus {
    /// Current step number (0-indexed).
    pub current_step: usize,
    /// Total number of steps configured.
    pub total_steps: usize,
    /// Fraction of proposals accepted so far (across all walkers and steps).
    pub acceptance_fraction: f64,
    /// Whether the sampler is still running.
    pub running: bool,
    /// Whether the sampler was stopped externally.
    pub stopped: bool,
}

// ===========================================================================
// Chain Data
// ===========================================================================

/// The complete MCMC chain output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McmcChain {
    /// Configuration used.
    pub config: McmcConfig,
    /// The chain of walker positions: shape `[n_steps][n_walkers][n_dim]`.
    pub positions: Vec<Vec<Vec<f64>>>,
    /// Log-probability at each position: shape `[n_steps][n_walkers]`.
    pub log_probs: Vec<Vec<f64>>,
    /// Overall acceptance fraction.
    pub acceptance_fraction: f64,
}

impl McmcChain {
    /// Extract flat samples after burn-in, optionally thinned.
    ///
    /// Returns a vector of `[n_dim]` parameter vectors.
    pub fn flat_samples(&self, burn_in: usize, thin: usize) -> Vec<Vec<f64>> {
        let thin = thin.max(1);
        let mut samples = Vec::new();
        for step in (burn_in..self.positions.len()).step_by(thin) {
            for walker in &self.positions[step] {
                samples.push(walker.clone());
            }
        }
        samples
    }

    /// Compute the mean of each parameter across flat samples.
    pub fn parameter_means(&self, burn_in: usize) -> Vec<f64> {
        let samples = self.flat_samples(burn_in, 1);
        if samples.is_empty() {
            return vec![0.0; self.config.n_dim];
        }
        let n = samples.len() as f64;
        let mut means = vec![0.0; self.config.n_dim];
        for s in &samples {
            for (i, val) in s.iter().enumerate() {
                means[i] += val;
            }
        }
        for m in &mut means {
            *m /= n;
        }
        means
    }

    /// Compute the covariance matrix of the flat samples after burn-in.
    /// Returns a `[n_dim × n_dim]` matrix (row-major).
    pub fn covariance_matrix(&self, burn_in: usize) -> Vec<Vec<f64>> {
        let samples = self.flat_samples(burn_in, 1);
        let n_dim = self.config.n_dim;
        if samples.is_empty() {
            return vec![vec![0.0; n_dim]; n_dim];
        }
        let means = self.parameter_means(burn_in);
        let n = samples.len() as f64;
        let mut cov = vec![vec![0.0; n_dim]; n_dim];
        for s in &samples {
            for i in 0..n_dim {
                for j in 0..n_dim {
                    cov[i][j] += (s[i] - means[i]) * (s[j] - means[j]);
                }
            }
        }
        for row in &mut cov {
            for val in row.iter_mut() {
                *val /= n - 1.0;
            }
        }
        cov
    }
}

// ===========================================================================
// Stretch-move Distribution
// ===========================================================================

/// Samples the stretch factor $z$ from $g(z) \propto z^{-1/2}$ on $[1/a, a]$.
///
/// Using the inverse CDF method:
/// $z = \bigl[u (a^{1/2} - a^{-1/2}) + a^{-1/2}\bigr]^2$
/// where $u \sim \mathrm{Uniform}(0, 1)$.
struct StretchDistribution {
    sqrt_a: f64,
    inv_sqrt_a: f64,
}

impl StretchDistribution {
    fn new(a: f64) -> Self {
        Self {
            sqrt_a: a.sqrt(),
            inv_sqrt_a: 1.0 / a.sqrt(),
        }
    }
}

impl Distribution<f64> for StretchDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let u: f64 = rng.gen();
        let x = u * (self.sqrt_a - self.inv_sqrt_a) + self.inv_sqrt_a;
        x * x
    }
}

// ===========================================================================
// Ensemble Sampler
// ===========================================================================

/// Affine-Invariant Ensemble MCMC Sampler.
///
/// Manages an ensemble of walkers exploring a parameter space using
/// the Goodman & Weare (2010) stretch-move algorithm.
pub struct EnsembleSampler {
    config: McmcConfig,
    /// Current positions of all walkers: `[n_walkers][n_dim]`.
    positions: Vec<Vec<f64>>,
    /// Current log-probability for each walker.
    log_probs: Vec<f64>,
    /// Recorded chain: `[step][walker][dim]`.
    chain_positions: Vec<Vec<Vec<f64>>>,
    /// Recorded log-probs: `[step][walker]`.
    chain_log_probs: Vec<Vec<f64>>,
    /// Total accepted proposals.
    n_accepted: usize,
    /// Total proposals made.
    n_proposed: usize,
    /// Current step counter.
    current_step: usize,
    /// External stop signal.
    stop_flag: Arc<AtomicBool>,
}

impl EnsembleSampler {
    /// Create a new ensemble sampler.
    ///
    /// # Arguments
    /// - `config` - Sampler configuration.
    /// - `initial_positions` - Starting positions `[n_walkers][n_dim]`.
    ///
    /// # Panics
    /// Panics if the number of walkers is odd, less than `2 * n_dim`,
    /// or if the initial positions array doesn't match the config.
    pub fn new(config: McmcConfig, initial_positions: Vec<Vec<f64>>) -> Self {
        assert!(
            config.n_walkers % 2 == 0,
            "Number of walkers must be even (got {})",
            config.n_walkers
        );
        assert!(
            config.n_walkers >= 2 * config.n_dim,
            "Need at least 2×n_dim walkers (got {} for dim {})",
            config.n_walkers,
            config.n_dim
        );
        assert_eq!(
            initial_positions.len(),
            config.n_walkers,
            "Initial positions count mismatch"
        );
        for (i, pos) in initial_positions.iter().enumerate() {
            assert_eq!(
                pos.len(),
                config.n_dim,
                "Walker {} has {} dims, expected {}",
                i,
                pos.len(),
                config.n_dim
            );
        }

        Self {
            config,
            positions: initial_positions,
            log_probs: Vec::new(),
            chain_positions: Vec::new(),
            chain_log_probs: Vec::new(),
            n_accepted: 0,
            n_proposed: 0,
            current_step: 0,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a handle to the stop flag for external cancellation.
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    /// Get the current MCMC status.
    pub fn status(&self) -> McmcStatus {
        McmcStatus {
            current_step: self.current_step,
            total_steps: self.config.n_steps,
            acceptance_fraction: if self.n_proposed > 0 {
                self.n_accepted as f64 / self.n_proposed as f64
            } else {
                0.0
            },
            running: self.current_step < self.config.n_steps
                && !self.stop_flag.load(Ordering::Relaxed),
            stopped: self.stop_flag.load(Ordering::Relaxed),
        }
    }

    /// Run the full MCMC chain.
    ///
    /// The `log_prob` function is called with a `&[f64]` parameter
    /// vector and must return the log-probability (unnormalised
    /// log-posterior).  It must be `Send + Sync` for parallel evaluation.
    ///
    /// Returns the complete chain data.
    pub fn run<F>(&mut self, log_prob: &F) -> McmcChain
    where
        F: Fn(&[f64]) -> f64 + Send + Sync,
    {
        let n_walkers = self.config.n_walkers;
        let n_dim = self.config.n_dim;
        let n_steps = self.config.n_steps;
        let a = self.config.stretch_factor;

        // Initialise log-probs for all walkers (parallel)
        self.log_probs = self
            .positions
            .par_iter()
            .map(|pos| log_prob(pos))
            .collect();

        // Pre-allocate chain storage
        self.chain_positions = Vec::with_capacity(n_steps);
        self.chain_log_probs = Vec::with_capacity(n_steps);

        // Record initial state
        self.chain_positions.push(self.positions.clone());
        self.chain_log_probs.push(self.log_probs.clone());

        let half = n_walkers / 2;

        for _step in 0..n_steps {
            // Check stop flag
            if self.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            // Update each half using the other half as the complement
            for batch in 0..2 {
                let (active_range, complement_range) = if batch == 0 {
                    (0..half, half..n_walkers)
                } else {
                    (half..n_walkers, 0..half)
                };

                // Collect complement positions (immutable snapshot)
                let complement: Vec<Vec<f64>> = complement_range
                    .clone()
                    .map(|i| self.positions[i].clone())
                    .collect();

                // Collect active walker data
                let active_data: Vec<(Vec<f64>, f64)> = active_range
                    .clone()
                    .map(|i| (self.positions[i].clone(), self.log_probs[i]))
                    .collect();

                // Parallel stretch-move proposals for active walkers
                let results: Vec<(Vec<f64>, f64, bool)> = active_data
                    .par_iter()
                    .map(|(current_pos, current_lp)| {
                        let mut rng = rand::thread_rng();
                        let stretch_dist = StretchDistribution::new(a);

                        // Choose a random complement walker
                        let j = rng.gen_range(0..complement.len());
                        let comp_pos = &complement[j];

                        // Draw stretch factor
                        let z = stretch_dist.sample(&mut rng);

                        // Propose new position: y = x_j + z * (x_k - x_j)
                        let mut proposal = vec![0.0; n_dim];
                        for d in 0..n_dim {
                            proposal[d] = comp_pos[d] + z * (current_pos[d] - comp_pos[d]);
                        }

                        // Evaluate log-probability at proposal
                        let new_lp = log_prob(&proposal);

                        // Acceptance criterion
                        let log_accept =
                            (n_dim as f64 - 1.0) * z.ln() + new_lp - current_lp;

                        let u: f64 = rng.gen();
                        if log_accept.is_finite() && u.ln() < log_accept {
                            (proposal, new_lp, true)
                        } else {
                            (current_pos.clone(), *current_lp, false)
                        }
                    })
                    .collect();

                // Apply results back to the sampler state
                for (idx, (new_pos, new_lp, accepted)) in results.into_iter().enumerate() {
                    let walker_idx = active_range.start + idx;
                    self.positions[walker_idx] = new_pos;
                    self.log_probs[walker_idx] = new_lp;
                    self.n_proposed += 1;
                    if accepted {
                        self.n_accepted += 1;
                    }
                }
            }

            self.current_step += 1;

            // Record state after this step
            self.chain_positions.push(self.positions.clone());
            self.chain_log_probs.push(self.log_probs.clone());
        }

        McmcChain {
            config: self.config.clone(),
            positions: self.chain_positions.clone(),
            log_probs: self.chain_log_probs.clone(),
            acceptance_fraction: if self.n_proposed > 0 {
                self.n_accepted as f64 / self.n_proposed as f64
            } else {
                0.0
            },
        }
    }

    /// Get a partial chain snapshot (for live status reporting).
    pub fn partial_chain(&self) -> McmcChain {
        McmcChain {
            config: self.config.clone(),
            positions: self.chain_positions.clone(),
            log_probs: self.chain_log_probs.clone(),
            acceptance_fraction: if self.n_proposed > 0 {
                self.n_accepted as f64 / self.n_proposed as f64
            } else {
                0.0
            },
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the stretch distribution samples in the correct range.
    #[test]
    fn stretch_distribution_range() {
        let dist = StretchDistribution::new(2.0);
        let mut rng = rand::thread_rng();
        for _ in 0..10_000 {
            let z = dist.sample(&mut rng);
            assert!(z >= 0.5, "z = {} < 1/a = 0.5", z);
            assert!(z <= 2.0, "z = {} > a = 2.0", z);
        }
    }

    /// Test the sampler on a 1D standard normal: $p(x) \propto \exp(-x^2/2)$.
    #[test]
    fn sample_1d_normal() {
        let config = McmcConfig {
            n_walkers: 16,
            n_dim: 1,
            n_steps: 2000,
            stretch_factor: 2.0,
        };

        let log_prob = |x: &[f64]| -> f64 { -0.5 * x[0] * x[0] };

        let mut rng = rand::thread_rng();
        let initial: Vec<Vec<f64>> = (0..config.n_walkers)
            .map(|_| vec![rng.gen_range(-2.0..2.0)])
            .collect();

        let mut sampler = EnsembleSampler::new(config, initial);
        let chain = sampler.run(&log_prob);

        let means = chain.parameter_means(500);
        assert!(
            means[0].abs() < 0.2,
            "Mean should be ~0, got {}",
            means[0]
        );

        let cov = chain.covariance_matrix(500);
        assert!(
            (cov[0][0] - 1.0).abs() < 0.3,
            "Variance should be ~1, got {}",
            cov[0][0]
        );

        // Acceptance fraction should be reasonable (typically 0.2–0.5)
        assert!(
            chain.acceptance_fraction > 0.1,
            "Acceptance too low: {}",
            chain.acceptance_fraction
        );
    }

    /// Regression test: sample a 2D correlated Gaussian and verify
    /// the chain recovers the true covariance structure.
    ///
    /// Target distribution:
    /// $p(x, y) \propto \exp\bigl(-\frac{1}{2} \mathbf{x}^T \Sigma^{-1} \mathbf{x}\bigr)$
    ///
    /// with $\Sigma = \begin{pmatrix} 1 & 0.8 \\ 0.8 & 1 \end{pmatrix}$
    /// and $\Sigma^{-1} = \frac{1}{0.36}\begin{pmatrix} 1 & -0.8 \\ -0.8 & 1 \end{pmatrix}$.
    #[test]
    fn sample_2d_correlated_gaussian() {
        let config = McmcConfig {
            n_walkers: 32,
            n_dim: 2,
            n_steps: 5000,
            stretch_factor: 2.0,
        };

        // Σ⁻¹ for Σ = [[1, 0.8], [0.8, 1]]
        // det(Σ) = 1 - 0.64 = 0.36
        // Σ⁻¹ = (1/0.36) * [[1, -0.8], [-0.8, 1]]
        let inv_det = 1.0 / 0.36;
        let log_prob = move |x: &[f64]| -> f64 {
            let dx = x[0]; // mean = 0
            let dy = x[1];
            -0.5 * inv_det * (dx * dx - 2.0 * 0.8 * dx * dy + dy * dy)
        };

        let mut rng = rand::thread_rng();
        let initial: Vec<Vec<f64>> = (0..config.n_walkers)
            .map(|_| vec![rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)])
            .collect();

        let mut sampler = EnsembleSampler::new(config, initial);
        let chain = sampler.run(&log_prob);

        let burn_in = 1000;
        let means = chain.parameter_means(burn_in);
        let cov = chain.covariance_matrix(burn_in);

        // Check means are near zero
        assert!(
            means[0].abs() < 0.15,
            "Mean[0] should be ~0, got {}",
            means[0]
        );
        assert!(
            means[1].abs() < 0.15,
            "Mean[1] should be ~0, got {}",
            means[1]
        );

        // Check diagonal (variances ≈ 1)
        assert!(
            (cov[0][0] - 1.0).abs() < 0.3,
            "Var[0] should be ~1, got {}",
            cov[0][0]
        );
        assert!(
            (cov[1][1] - 1.0).abs() < 0.3,
            "Var[1] should be ~1, got {}",
            cov[1][1]
        );

        // Check off-diagonal (covariance ≈ 0.8)
        assert!(
            (cov[0][1] - 0.8).abs() < 0.2,
            "Cov[0,1] should be ~0.8, got {}",
            cov[0][1]
        );

        assert!(
            chain.acceptance_fraction > 0.1,
            "Acceptance too low: {}",
            chain.acceptance_fraction
        );
    }

    /// Test flat_samples extraction with burn-in and thinning.
    #[test]
    fn flat_samples_burn_in_thin() {
        let config = McmcConfig {
            n_walkers: 8,
            n_dim: 1,
            n_steps: 100,
            stretch_factor: 2.0,
        };

        let log_prob = |x: &[f64]| -> f64 { -0.5 * x[0] * x[0] };

        let initial: Vec<Vec<f64>> = (0..8).map(|i| vec![i as f64 * 0.1]).collect();
        let mut sampler = EnsembleSampler::new(config, initial);
        let chain = sampler.run(&log_prob);

        // Total recorded steps = n_steps + 1 (initial)
        assert_eq!(chain.positions.len(), 101);

        // With burn_in=50, thin=2: steps 50..101 = 51 steps, step_by(2) = 26
        // Each step has 8 walkers → 26 × 8 = 208 samples
        let samples = chain.flat_samples(50, 2);
        assert_eq!(samples.len(), 26 * 8);
    }

    /// Ensure the stop flag halts execution early.
    #[test]
    fn stop_flag_halts_sampler() {
        let config = McmcConfig {
            n_walkers: 8,
            n_dim: 1,
            n_steps: 100_000, // Very large
            stretch_factor: 2.0,
        };

        let log_prob = |x: &[f64]| -> f64 { -0.5 * x[0] * x[0] };
        let initial: Vec<Vec<f64>> = (0..8).map(|i| vec![i as f64 * 0.1]).collect();
        let mut sampler = EnsembleSampler::new(config, initial);

        // Set stop flag immediately
        sampler.stop_flag().store(true, Ordering::Relaxed);

        let chain = sampler.run(&log_prob);

        // Should have stopped very early
        assert!(
            chain.positions.len() < 100,
            "Chain should have stopped early, got {} steps",
            chain.positions.len()
        );
    }
}
