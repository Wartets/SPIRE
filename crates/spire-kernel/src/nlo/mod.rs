//! # NLO — Next-to-Leading Order Corrections
//!
//! This module implements the theoretical infrastructure for computing NLO
//! QCD corrections to scattering cross-sections using the **Catani–Seymour
//! dipole subtraction** method.
//!
//! ## Architecture
//!
//! The subtraction method factorises soft and collinear singularities of
//! real-emission matrix elements into universal dipole terms that can be
//! analytically integrated over the unresolved one-particle phase space.
//!
//! $$\sigma^{\text{NLO}} = \int_{m+1} \left[ d\sigma^R - d\sigma^A \right]
//!     + \int_m \left[ d\sigma^V + \int_1 d\sigma^A \right]$$
//!
//! where $d\sigma^A$ is built from a sum of dipole contributions, each
//! constructed from a **kinematic mapping** and a **splitting function**.
//!
//! ## Submodules
//!
//! - [`dipole`] — Catani–Seymour kinematic mappings, splitting functions,
//!   dipole contributions, and the `SubtractionScheme` trait.

pub mod dipole;
