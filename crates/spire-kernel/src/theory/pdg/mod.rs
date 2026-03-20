//! PDG integration primitives.

/// Source abstraction and precedence arbitration for PDG-backed records.
pub mod arbiter;
/// Canonical PDG value/provenance contracts shared across transports.
pub mod contracts;
/// Edition mismatch guards used to protect model reproducibility.
pub mod guards;
