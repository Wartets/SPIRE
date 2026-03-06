//! # Mathematical Utilities
//!
//! Generic numerical methods for use throughout the SPIRE kernel.
//!
//! ## Modules
//!
//! - [`ode`] - Generic Ordinary Differential Equation (ODE) solvers.
//!   Provides classical Runge-Kutta 4th order and an adaptive
//!   Dormand-Prince (RK45) solver with error-controlled step sizing.

pub mod ode;
