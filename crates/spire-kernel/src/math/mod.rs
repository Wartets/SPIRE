//! # Mathematical Utilities
//!
//! Generic numerical methods for use throughout the SPIRE kernel.
//!
//! ## Modules
//!
//! - [`ode`] - Generic Ordinary Differential Equation (ODE) solvers.
//!   Provides classical Runge-Kutta 4th order and an adaptive
//!   Dormand-Prince (RK45) solver with error-controlled step sizing.
//! - [`mcmc`] - Markov Chain Monte Carlo engine.  Affine-Invariant
//!   Ensemble Sampler (Goodman & Weare, 2010) with `rayon` parallelism.
//! - [`precision`] - Numerical stability and precision management.
//!   Kahan compensated summation, catastrophic cancellation detection,
//!   and adaptive precision escalation for critical paths.

pub mod mcmc;
pub mod ode;
pub mod precision;
