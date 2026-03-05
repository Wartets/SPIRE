//! # Shower — External Parton Shower Bridge
//!
//! This module provides a **modular interface** for piping SPIRE's
//! parton-level events through external parton shower and hadronisation
//! programs (Pythia, Herwig, Sherpa, etc.).
//!
//! ## Architecture
//!
//! The design follows the same trait-based abstraction used throughout the
//! kernel: the [`bridge::PartonShowerProvider`] trait defines *what* a
//! shower does, while concrete implementations handle *how* to invoke a
//! specific external program.
//!
//! No shower physics is hard-coded — SPIRE acts as a thin orchestrator
//! that writes LHE files, invokes the external tool, and parses the
//! showered output (currently HepMC format).
//!
//! ## Submodules
//!
//! - [`bridge`] — `PartonShowerProvider` trait, `PythiaCLIProvider`
//!   implementation, `ShowerConfig` / `ShowerResult` data types,
//!   and a minimal HepMC3 event parser.

pub mod bridge;
