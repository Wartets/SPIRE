//! # Theory Module — Lagrangian Workbench
//!
//! This module provides the infrastructure for defining quantum field theories
//! at the Lagrangian level and automatically deriving the operational Feynman
//! rules from that definition.
//!
//! ## Sub-modules
//!
//! - [`ast`] — Abstract Syntax Tree for Lagrangian density terms. Defines the
//!   symbolic nodes (field operators, gamma matrices, derivatives, coupling
//!   constants, tensor indices) and implements string parsing + index
//!   contraction validation.
//!
//! - [`derivation`] — Functional differentiation engine. Computes Feynman
//!   vertex rules by successively differentiating a Lagrangian term with
//!   respect to external fields, accounting for Grassmann sign factors,
//!   momentum-space replacement of derivatives, and combinatorial symmetry
//!   factors.
//!
//! - [`validation`] — Theoretical consistency checks. Verifies Lorentz
//!   invariance (all indices contracted), gauge singlet condition
//!   ($N$-ality / charge sum), and Hermiticity of proposed interaction terms.
//!
//! - [`rge`] — Renormalization Group Equation solver. Integrates user-defined
//!   β-functions (via Rhai scripts) to compute the running of couplings as a
//!   function of the energy scale μ.

pub mod ast;
pub mod derivation;
pub mod rge;
pub mod validation;
