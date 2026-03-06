//! # Flavor Physics
//!
//! Modules for precision flavor observables, combining non-perturbative
//! Lattice QCD inputs with perturbative Wilson coefficient functions from
//! Effective Field Theory (EFT).
//!
//! ## Modules
//!
//! - [`lattice`] - Lattice QCD data structures: hadronic decay constants
//!   and form factor parameterizations via the BCL $z$-expansion.
//! - [`eft`] - Flavor EFT engine: Wilson coefficients, $B$-meson mixing,
//!   and rare semi-leptonic decay rate computations.

pub mod eft;
pub mod lattice;
