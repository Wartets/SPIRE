//! # IO - Event Serialization and File Format Writers
//!
//! This module provides a generic framework for writing Monte Carlo event
//! records to standard HEP file formats. The design separates the
//! *what* (physics data) from the *how* (format encoding) via the
//! [`EventWriter`] trait.
//!
//! ## Supported Formats
//!
//! - [`lhe::LheWriter`] - Les Houches Event (LHE) format v3.0, the
//!   industry-standard text format for exchanging parton-level events
//!   between generators (MadGraph, Sherpa) and shower/hadronization
//!   codes (Pythia, Herwig).
//!
//! ## Architecture
//!
//! All writers implement the three-phase [`EventWriter`] protocol:
//!
//! 1. **`write_header`** - Emit format preamble, beam parameters, and
//!    process cross-sections.
//! 2. **`write_event`** - Emit one event record (called in the MC loop).
//! 3. **`finish`** - Flush buffers and write format epilogue.
//!
//! Writers are parameterised over `std::io::Write`, so they work equally
//! well with files, in-memory buffers, and network streams.

pub mod latex;
pub mod lhe;
pub mod provenance;

use crate::SpireResult;

// ---------------------------------------------------------------------------
// Common Data Structures
// ---------------------------------------------------------------------------

/// A single particle entry within an event record.
///
/// This is the format-agnostic representation of one line in an event
/// block. Individual writers translate this to their format's encoding.
#[derive(Debug, Clone)]
pub struct EventParticle {
    /// PDG Monte Carlo particle ID (e.g., 11 = electron, 22 = photon).
    pub pdg_id: i32,
    /// Status code:
    /// - `-1` = incoming (initial state)
    /// - `+1` = outgoing (final state, stable)
    /// - `+2` = intermediate (resonance / decay)
    pub status: i32,
    /// Index of first mother particle (1-based, 0 = no mother).
    pub mother1: usize,
    /// Index of second mother particle (1-based, 0 = no mother).
    pub mother2: usize,
    /// First colour-flow tag (0 for colour singlets).
    pub color1: i32,
    /// Second colour-flow tag (0 for colour singlets).
    pub color2: i32,
    /// $x$-component of 3-momentum $p_x$ in GeV.
    pub px: f64,
    /// $y$-component of 3-momentum $p_y$ in GeV.
    pub py: f64,
    /// $z$-component of 3-momentum $p_z$ in GeV.
    pub pz: f64,
    /// Energy $E$ in GeV.
    pub energy: f64,
    /// Generated (invariant) mass in GeV.
    pub mass: f64,
    /// Proper lifetime $c\tau$ in mm (0 = stable or prompt).
    pub lifetime: f64,
    /// Spin/helicity information (9.0 = not specified).
    pub spin: f64,
}

/// Data for a single event to be written.
///
/// Contains the list of particles, the event weight, and the
/// factorisation/renormalisation scales.
#[derive(Debug, Clone)]
pub struct EventRecord {
    /// The particles (initial + final state) in this event.
    pub particles: Vec<EventParticle>,
    /// Event weight (typically the MC weight × matrix element).
    pub weight: f64,
    /// Factorisation scale $Q$ in GeV.
    pub scale: f64,
    /// QED coupling $\alpha_\text{QED}$ at the event scale.
    pub alpha_qed: f64,
    /// QCD coupling $\alpha_s$ at the event scale.
    pub alpha_qcd: f64,
    /// Process ID (cross-reference to the `<init>` block).
    pub process_id: i32,
}

/// Configuration for the file header / initialisation block.
///
/// Contains beam parameters, process cross-sections, and generator
/// metadata. This is written once at the start of the output file.
#[derive(Debug, Clone)]
pub struct InitConfig {
    /// PDG ID of beam particle 1 (e.g., 2212 = proton, 11 = electron).
    pub beam1_id: i32,
    /// PDG ID of beam particle 2.
    pub beam2_id: i32,
    /// Energy of beam 1 in GeV.
    pub beam1_energy: f64,
    /// Energy of beam 2 in GeV.
    pub beam2_energy: f64,
    /// PDF author group ID for beam 1 (0 = not applicable).
    pub pdf_group1: i32,
    /// PDF author group ID for beam 2.
    pub pdf_group2: i32,
    /// PDF set ID for beam 1 (0 = not applicable).
    pub pdf_set1: i32,
    /// PDF set ID for beam 2.
    pub pdf_set2: i32,
    /// Event weight strategy:
    /// - `+1` = unit weight (unweighted events)
    /// - `-1` = weighted events
    /// - `+3` / `-3` = multi-weight variants
    pub weight_strategy: i32,
    /// Process entries: `(cross_section, error, max_weight, process_id)`.
    pub processes: Vec<ProcessInfo>,
    /// Optional generator name for the header comment.
    pub generator: String,
}

/// Cross-section information for a single subprocess.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Cross-section in pb.
    pub cross_section: f64,
    /// Statistical error on the cross-section in pb.
    pub error: f64,
    /// Maximum event weight (relevant for unweighting).
    pub max_weight: f64,
    /// Process ID tag.
    pub process_id: i32,
}

// ---------------------------------------------------------------------------
// EventWriter Trait
// ---------------------------------------------------------------------------

/// Generic interface for writing Monte Carlo event records.
///
/// Implementations handle the encoding details for specific file formats
/// (LHE, HepMC, ROOT NTuple, etc.) while the physics code works with
/// the format-agnostic [`EventRecord`] and [`InitConfig`] types.
///
/// # Three-Phase Protocol
///
/// 1. Call [`EventWriter::write_header`] once to emit the preamble.
/// 2. Call [`EventWriter::write_event`] for each MC event (inside the integration loop).
/// 3. Call [`EventWriter::finish`] to flush buffers and close the format.
///
/// # Performance
///
/// Writers should use buffered I/O internally. The `write_event` method
/// is called in the Monte Carlo hot loop - avoid allocations.
pub trait EventWriter: Send {
    /// Write the file header and initialisation block.
    fn write_header(&mut self, config: &InitConfig) -> SpireResult<()>;

    /// Write a single event record.
    fn write_event(&mut self, event: &EventRecord) -> SpireResult<()>;

    /// Flush all buffered data and write any format epilogue.
    fn finish(&mut self) -> SpireResult<()>;
}
