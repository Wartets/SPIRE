//! # SPIRE Python Extension Module
//!
//! The **spire-python** crate compiles the SPIRE physics kernel into a native
//! Python extension module via [PyO3](https://pyo3.rs) and
//! [rust-numpy](https://github.com/PyO3/rust-numpy).
//!
//! ## Design Philosophy
//!
//! This crate is a **pure FFI wrapper** - no physics logic is duplicated here.
//! Every computation delegates to `spire-kernel`.  The module is structured
//! around *ergonomic, stateful PyO3 classes* that carry their own data and
//! expose Jupyter-friendly `_repr_html_` methods.
//!
//! ## Module Layout
//!
//! - [`errors`] - `SpireError` → Python exception mapping.
//! - [`model`] - `Model` class: load Standard Model or BSM presets.
//! - [`particle`] - `Particle` and `Field` wrapper classes with rich display.
//! - [`reaction`] - `Reaction` class: process construction and validation.
//! - [`diagrams`] - `DiagramSet` class: Feynman diagram generation.
//! - [`amplitude`] - `AmplitudeResult` class: amplitude computation.
//! - [`events`] - `EventGenerator` class: zero-copy NumPy phase-space bridge.
//! - [`xsec`] - `CrossSectionResult` / `HadronicCrossSectionResult` wrappers.
//! - [`decay`] - `DecayTable` class with branching-ratio display.
//! - [`kinematics_mod`] - Threshold, Mandelstam, and boost utilities.

// PyO3 0.20 generates non-local impl blocks; allow until pyo3 0.22+ is adopted.
#![allow(non_local_definitions)]

mod amplitude;
mod decay;
mod diagrams;
mod errors;
mod events;
mod kinematics_mod;
mod model;
mod particle;
mod reaction;
mod xsec;

use pyo3::prelude::*;

// ===========================================================================
// PyO3 Module Registration
// ===========================================================================

/// The top-level Python module `spire._native`.
///
/// This is imported by the pure-Python `spire/__init__.py` and re-exported
/// under the public `spire` namespace.
#[pymodule]
fn _native(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // ── Model & ontology ─────────────────────────────────────────────
    m.add_class::<model::PyModel>()?;
    m.add_class::<particle::PyField>()?;
    m.add_class::<particle::PyParticle>()?;

    // ── Reaction pipeline ────────────────────────────────────────────
    m.add_class::<reaction::PyReaction>()?;
    m.add_class::<diagrams::PyDiagramSet>()?;
    m.add_class::<amplitude::PyAmplitudeResult>()?;

    // ── Cross-sections & integration ─────────────────────────────────
    m.add_class::<xsec::PyCrossSectionResult>()?;
    m.add_class::<xsec::PyHadronicResult>()?;
    m.add_class::<events::PyEventGenerator>()?;

    // ── Decay ────────────────────────────────────────────────────────
    m.add_class::<decay::PyDecayTable>()?;
    m.add_class::<decay::PyDecayChannel>()?;

    // ── Kinematics utilities ─────────────────────────────────────────
    m.add_function(wrap_pyfunction!(kinematics_mod::calculate_threshold, m)?)?;
    m.add_function(wrap_pyfunction!(kinematics_mod::is_kinematically_allowed, m)?)?;

    Ok(())
}
