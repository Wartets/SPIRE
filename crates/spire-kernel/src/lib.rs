//! # SPIRE Kernel
//!
//! The **spire-kernel** crate is the computational heart of SPIRE (Structured Particle
//! Interaction and Reaction Engine). It is a pure, headless physics engine containing the
//! entirety of the QFT and HEP logic, with no dependencies on UI or platform-specific code.
//!
//! ## Module Overview
//!
//! - [`ontology`] — Core traits and structs for all physical entities (particles, fields,
//!   quantum states). Defines the formal type system grounded in Poincaré and gauge group
//!   representations.
//! - [`groups`] — Mathematical structures for symmetry groups ($SU(3)_C \times SU(2)_L
//!   \times U(1)_Y$ and Poincaré). Conservation law validation via group invariants.
//! - [`s_matrix`] — High-level process construction and validation. Manages transitions
//!   between asymptotic states, reaction reconstruction, and mediating boson identification.
//! - [`lagrangian`] — Parses theoretical models from data files and derives vertex rules,
//!   propagators, and Feynman rules from Lagrangian terms.
//! - [`graph`] — Feynman diagram generation, manipulation, and validation as directed
//!   graphs. Topological enumeration, channel identification, symmetry factor calculation.
//! - [`algebra`] — Symbolic mathematics engine for 4-momenta, gamma matrices, spinors,
//!   polarization vectors. Trace evaluation and index contraction. Numerical evaluation
//!   of CAS expressions via `NumericalContext`.
//! - [`kinematics`] — Phase space calculations, Mandelstam variables, threshold energies,
//!   Lorentz boosts, Dalitz plot boundary generation, and N-body phase space generation
//!   via the RAMBO algorithm.
//! - [`integration`] — Monte Carlo integration engine for computing cross-sections and
//!   differential distributions from squared matrix elements over phase space.
//! - [`pdf`] — Parton Distribution Function framework. Generic `PdfProvider` trait
//!   with analytical toy implementations for proton parton densities.
//! - [`scripting`] — Embedded Rhai scripting engine for runtime-defined observables,
//!   kinematic cuts, and form factors.
//! - [`io`] — Event serialization framework. Generic `EventWriter` trait with
//!   Les Houches Event (LHE) format v3.0 implementation.
//! - [`interface`] — External solver communication. Generic `ExternalSolver` trait
//!   with CLI subprocess wrapper for invoking tools like FIRE and Kira.
//! - [`analysis`] — Integrated histogramming and analysis pipeline. High-performance
//!   `Histogram1D` and `Histogram2D` structures with thread-local merge strategy
//!   for parallel Monte Carlo, connected to the Rhai scripting engine for
//!   runtime-defined observables.
//! - [`reco`] — Reconstruction and phenomenological detector simulation.
//!   Anti-$k_t$ jet clustering, configurable energy smearing and efficiency
//!   maps, missing transverse energy computation, and `ReconstructedEvent`
//!   objects for experimentally comparable observables.
pub mod algebra;
pub mod analysis;
pub mod data_loader;
pub mod graph;
pub mod groups;
pub mod integration;
pub mod interface;
pub mod io;
pub mod kinematics;
pub mod lagrangian;
pub mod ontology;
pub mod pdf;
pub mod reco;
pub mod s_matrix;
pub mod scripting;
pub mod theory;

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Core Error Types
// ---------------------------------------------------------------------------

/// Top-level error type for the SPIRE kernel.
///
/// Every fallible operation in the kernel returns a `Result<T, SpireError>`.
/// Variants are designed to give precise theoretical diagnostics when a
/// physical process is rejected (e.g., "Forbidden by Lepton Number conservation").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpireError {
    /// A proposed reaction violates one or more conservation laws.
    /// Contains a human-readable description of which symmetry was violated.
    ConservationViolation(String),

    /// The requested process is kinematically forbidden (e.g., insufficient
    /// centre-of-mass energy to produce the final state).
    KinematicsForbidden(String),

    /// An invalid or unrecognized particle identifier was supplied.
    UnknownParticle(String),

    /// A Feynman diagram topology is invalid under the active Lagrangian
    /// (no corresponding vertex rule exists).
    InvalidVertex(String),

    /// An error occurred while parsing a model file (particles.toml,
    /// sm_vertices.toml, or a BSM extension).
    ModelParseError(String),

    /// A group-theory operation failed (e.g., representation mismatch).
    GroupTheoryError(String),

    /// A symbolic algebra operation failed (e.g., index contraction mismatch).
    AlgebraError(String),

    /// A generic internal error not covered by the specific variants above.
    InternalError(String),
}

impl fmt::Display for SpireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpireError::ConservationViolation(msg) => {
                write!(f, "Conservation violation: {}", msg)
            }
            SpireError::KinematicsForbidden(msg) => {
                write!(f, "Kinematics forbidden: {}", msg)
            }
            SpireError::UnknownParticle(msg) => write!(f, "Unknown particle: {}", msg),
            SpireError::InvalidVertex(msg) => write!(f, "Invalid vertex: {}", msg),
            SpireError::ModelParseError(msg) => write!(f, "Model parse error: {}", msg),
            SpireError::GroupTheoryError(msg) => write!(f, "Group theory error: {}", msg),
            SpireError::AlgebraError(msg) => write!(f, "Algebra error: {}", msg),
            SpireError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for SpireError {}

/// Convenience alias used throughout the kernel.
pub type SpireResult<T> = Result<T, SpireError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_format() {
        let err = SpireError::UnknownParticle("foo".into());
        assert!(format!("{}", err).contains("foo"));
    }

    #[test]
    fn error_is_clone() {
        let err = SpireError::InternalError("test".into());
        let _ = err.clone();
    }
}
