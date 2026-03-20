//! PDG integration primitives.

/// High-level PDG SQLite adapter API for MCID/name property lookup.
pub mod adapter;
/// Source abstraction and precedence arbitration for PDG-backed records.
pub mod arbiter;
/// Canonical PDG value/provenance contracts shared across transports.
pub mod contracts;
/// Database connection and query strategy layer for PDG SQLite datasets.
pub mod database;
/// Join-key-correct quantity extraction from normalized PDG tables.
pub mod extraction;
/// Edition mismatch guards used to protect model reproducibility.
pub mod guards;
/// Deterministic MCID/name resolution across particle and alias tables.
pub mod resolution;
