//! # LHE — Les Houches Event File Writer
//!
//! Implements the Les Houches Event (LHE) file format version 3.0 as defined
//! in [hep-ph/0609017](https://arxiv.org/abs/hep-ph/0609017).
//!
//! The LHE format is the *de facto* standard for exchanging parton-level event
//! samples between matrix-element generators (MadGraph, Sherpa, SPIRE) and
//! parton-shower / hadronization codes (Pythia, Herwig).
//!
//! ## Format Overview
//!
//! ```xml
//! <LesHouchesEvents version="3.0">
//! <header>
//! <!-- Generator: SPIRE vX.Y.Z -->
//! </header>
//! <init>
//!   IDBMUP(1) IDBMUP(2) EBMUP(1) EBMUP(2) PDFGUP(1) PDFGUP(2) ...
//!   XSECUP XERRUP XMAXUP LPRUP
//! </init>
//! <event>
//!   NUP IDPRUP XWGTUP SCALUP AQEDUP AQCDUP
//!   IDUP ISTUP MOTHUP1 MOTHUP2 ICOLUP1 ICOLUP2 PX PY PZ E M VTIMUP SPINUP
//!   ...
//! </event>
//! ...
//! </LesHouchesEvents>
//! ```
//!
//! ## Performance
//!
//! The writer wraps any `std::io::Write` in a `BufWriter` and formats numeric
//! values directly to the buffer via `write!` macros, avoiding intermediate
//! `String` allocations. This is suitable for use inside Monte Carlo
//! integration loops generating millions of events.

use std::io::{BufWriter, Write};

use crate::io::{EventRecord, EventWriter, InitConfig};
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// LHE Writer
// ---------------------------------------------------------------------------

/// A buffered writer that emits events in Les Houches Event format v3.0.
///
/// # Type Parameter
///
/// `W` — any `std::io::Write` target (file, `Vec<u8>`, stdout, etc.).
///
/// # Usage
///
/// ```no_run
/// use std::fs::File;
/// use spire_kernel::io::lhe::LheWriter;
/// use spire_kernel::io::{EventWriter, InitConfig, ProcessInfo};
///
/// let file = File::create("output.lhe").unwrap();
/// let mut writer = LheWriter::new(file);
///
/// let config = InitConfig {
///     beam1_id: 11, beam2_id: -11,
///     beam1_energy: 100.0, beam2_energy: 100.0,
///     pdf_group1: 0, pdf_group2: 0,
///     pdf_set1: 0, pdf_set2: 0,
///     weight_strategy: -1,
///     processes: vec![ProcessInfo {
///         cross_section: 1.23e-3,
///         error: 4.56e-5,
///         max_weight: 1.0,
///         process_id: 1,
///     }],
///     generator: "SPIRE".into(),
/// };
///
/// writer.write_header(&config).unwrap();
/// // ... writer.write_event(&event) in loop ...
/// writer.finish().unwrap();
/// ```
pub struct LheWriter<W: Write> {
    /// Buffered output stream.
    buf: BufWriter<W>,
}

impl<W: Write> LheWriter<W> {
    /// Create a new LHE writer wrapping the given output stream.
    ///
    /// The stream is wrapped in a `BufWriter` with the default buffer
    /// size (8 KiB) for efficient sequential writes.
    pub fn new(writer: W) -> Self {
        Self {
            buf: BufWriter::new(writer),
        }
    }

    /// Create a new LHE writer with a custom buffer capacity.
    pub fn with_capacity(capacity: usize, writer: W) -> Self {
        Self {
            buf: BufWriter::with_capacity(capacity, writer),
        }
    }

    /// Get a reference to the underlying writer (inside the BufWriter).
    pub fn get_ref(&self) -> &W {
        self.buf.get_ref()
    }
}

impl<W: Write + Send> EventWriter for LheWriter<W> {
    fn write_header(&mut self, config: &InitConfig) -> SpireResult<()> {
        let buf = &mut self.buf;

        // XML preamble.
        writeln!(buf, "<LesHouchesEvents version=\"3.0\">")?;

        // Header block with generator information.
        writeln!(buf, "<header>")?;
        writeln!(buf, "<!-- Generator: {} -->", config.generator)?;
        writeln!(buf, "</header>")?;

        // Init block: beam parameters + process cross-sections.
        writeln!(buf, "<init>")?;

        // Line 1: beam IDs, energies, PDF info, weight strategy, number of processes.
        writeln!(
            buf,
            " {} {} {:E} {:E} {} {} {} {} {} {}",
            config.beam1_id,
            config.beam2_id,
            config.beam1_energy,
            config.beam2_energy,
            config.pdf_group1,
            config.pdf_group2,
            config.pdf_set1,
            config.pdf_set2,
            config.weight_strategy,
            config.processes.len(),
        )?;

        // One line per subprocess.
        for proc in &config.processes {
            writeln!(
                buf,
                " {:E} {:E} {:E} {}",
                proc.cross_section, proc.error, proc.max_weight, proc.process_id,
            )?;
        }

        writeln!(buf, "</init>")?;

        Ok(())
    }

    fn write_event(&mut self, event: &EventRecord) -> SpireResult<()> {
        let buf = &mut self.buf;
        let nup = event.particles.len();

        writeln!(buf, "<event>")?;

        // Event header line.
        writeln!(
            buf,
            " {} {} {:E} {:E} {:E} {:E}",
            nup,
            event.process_id,
            event.weight,
            event.scale,
            event.alpha_qed,
            event.alpha_qcd,
        )?;

        // One line per particle.
        for p in &event.particles {
            writeln!(
                buf,
                " {} {} {} {} {} {} {:E} {:E} {:E} {:E} {:E} {:E} {:E}",
                p.pdg_id,
                p.status,
                p.mother1,
                p.mother2,
                p.color1,
                p.color2,
                p.px,
                p.py,
                p.pz,
                p.energy,
                p.mass,
                p.lifetime,
                p.spin,
            )?;
        }

        writeln!(buf, "</event>")?;

        Ok(())
    }

    fn finish(&mut self) -> SpireResult<()> {
        writeln!(self.buf, "</LesHouchesEvents>")?;
        self.buf.flush()?;
        Ok(())
    }
}

// Map std::io::Error to SpireError for the `?` operator.
impl From<std::io::Error> for SpireError {
    fn from(e: std::io::Error) -> Self {
        SpireError::InternalError(format!("IO error: {}", e))
    }
}

// ---------------------------------------------------------------------------
// Helper: PhaseSpacePoint → EventRecord conversion
// ---------------------------------------------------------------------------

/// Build an [`EventRecord`] from a [`PhaseSpacePoint`](crate::kinematics::PhaseSpacePoint) and process metadata.
///
/// This is the bridge between the kernel's internal phase-space representation
/// and the format-agnostic event record used by all writers.
///
/// # Arguments
///
/// * `event` — The phase-space point from Monte Carlo sampling.
/// * `initial_pdg_ids` — PDG IDs for the initial-state particles.
/// * `final_pdg_ids` — PDG IDs for the final-state particles.
/// * `final_masses` — Rest masses of the final-state particles (GeV).
/// * `beam_energy` — Per-beam energy in GeV (used for initial-state momenta).
/// * `process_id` — Subprocess identifier.
/// * `scale` — Factorisation scale $Q$ in GeV.
/// * `alpha_qed` — QED coupling at the event scale.
/// * `alpha_qcd` — QCD coupling at the event scale.
///
/// # Convention
///
/// Initial-state momenta are constructed as back-to-back along the $z$-axis
/// with the given beam energy. Final-state momenta come from the
/// `PhaseSpacePoint`.
pub fn event_record_from_phase_space(
    event: &crate::kinematics::PhaseSpacePoint,
    initial_pdg_ids: &[i32],
    final_pdg_ids: &[i32],
    final_masses: &[f64],
    beam_energy: f64,
    process_id: i32,
    scale: f64,
    alpha_qed: f64,
    alpha_qcd: f64,
) -> EventRecord {
    use crate::io::EventParticle;

    let mut particles = Vec::with_capacity(initial_pdg_ids.len() + final_pdg_ids.len());

    // Initial-state particles (back-to-back along z-axis).
    for (i, &pdg_id) in initial_pdg_ids.iter().enumerate() {
        let pz_sign = if i == 0 { 1.0 } else { -1.0 };
        particles.push(EventParticle {
            pdg_id,
            status: -1,
            mother1: 0,
            mother2: 0,
            color1: 0,
            color2: 0,
            px: 0.0,
            py: 0.0,
            pz: pz_sign * beam_energy,
            energy: beam_energy,
            mass: 0.0, // massless beam approximation
            lifetime: 0.0,
            spin: 9.0,
        });
    }

    // Final-state particles from the phase-space point.
    let n_initial = initial_pdg_ids.len();
    for (j, &pdg_id) in final_pdg_ids.iter().enumerate() {
        let mom = &event.momenta[j];
        let mass = if j < final_masses.len() {
            final_masses[j]
        } else {
            0.0
        };

        particles.push(EventParticle {
            pdg_id,
            status: 1,
            mother1: 1,
            mother2: n_initial,
            color1: 0,
            color2: 0,
            px: mom[1],
            py: mom[2],
            pz: mom[3],
            energy: mom[0],
            mass,
            lifetime: 0.0,
            spin: 9.0,
        });
    }

    EventRecord {
        particles,
        weight: event.weight,
        scale,
        alpha_qed,
        alpha_qcd,
        process_id,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::SpacetimeVector;
    use crate::io::{EventParticle, EventRecord, EventWriter, InitConfig, ProcessInfo};
    use crate::kinematics::PhaseSpacePoint;

    fn sample_init_config() -> InitConfig {
        InitConfig {
            beam1_id: 11,
            beam2_id: -11,
            beam1_energy: 100.0,
            beam2_energy: 100.0,
            pdf_group1: 0,
            pdf_group2: 0,
            pdf_set1: 0,
            pdf_set2: 0,
            weight_strategy: -1,
            processes: vec![ProcessInfo {
                cross_section: 1.23e-3,
                error: 4.56e-5,
                max_weight: 1.0,
                process_id: 1,
            }],
            generator: "SPIRE v0.1.0".into(),
        }
    }

    fn sample_event_record() -> EventRecord {
        EventRecord {
            particles: vec![
                EventParticle {
                    pdg_id: 11,
                    status: -1,
                    mother1: 0,
                    mother2: 0,
                    color1: 0,
                    color2: 0,
                    px: 0.0,
                    py: 0.0,
                    pz: 100.0,
                    energy: 100.0,
                    mass: 0.000511,
                    lifetime: 0.0,
                    spin: 9.0,
                },
                EventParticle {
                    pdg_id: -11,
                    status: -1,
                    mother1: 0,
                    mother2: 0,
                    color1: 0,
                    color2: 0,
                    px: 0.0,
                    py: 0.0,
                    pz: -100.0,
                    energy: 100.0,
                    mass: 0.000511,
                    lifetime: 0.0,
                    spin: 9.0,
                },
                EventParticle {
                    pdg_id: 13,
                    status: 1,
                    mother1: 1,
                    mother2: 2,
                    color1: 0,
                    color2: 0,
                    px: 30.0,
                    py: 40.0,
                    pz: 86.6,
                    energy: 100.0,
                    mass: 0.10566,
                    lifetime: 0.0,
                    spin: 9.0,
                },
                EventParticle {
                    pdg_id: -13,
                    status: 1,
                    mother1: 1,
                    mother2: 2,
                    color1: 0,
                    color2: 0,
                    px: -30.0,
                    py: -40.0,
                    pz: -86.6,
                    energy: 100.0,
                    mass: 0.10566,
                    lifetime: 0.0,
                    spin: 9.0,
                },
            ],
            weight: 0.42,
            scale: 200.0,
            alpha_qed: 7.297e-3,
            alpha_qcd: 0.118,
            process_id: 1,
        }
    }

    #[test]
    fn lhe_header_contains_version_tag() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("<LesHouchesEvents version=\"3.0\">"));
        assert!(output.contains("</LesHouchesEvents>"));
    }

    #[test]
    fn lhe_header_contains_init_block() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("<init>"));
        assert!(output.contains("</init>"));
        // Beam IDs should be present.
        assert!(output.contains("11 -11"));
    }

    #[test]
    fn lhe_header_contains_generator_comment() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("SPIRE v0.1.0"));
    }

    #[test]
    fn lhe_event_block_structure() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("<event>"));
        assert!(output.contains("</event>"));
    }

    #[test]
    fn lhe_event_contains_particle_count() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        // 4 particles in the event.
        // The event header line starts with " 4 "
        let event_start = output.find("<event>").unwrap();
        let event_body = &output[event_start..];
        let first_line = event_body.lines().nth(1).unwrap();
        assert!(first_line.trim().starts_with("4"));
    }

    #[test]
    fn lhe_event_contains_pdg_ids() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        // Electron (11), positron (-11), muon (13), anti-muon (-13).
        assert!(output.contains(" 11 -1"));   // electron, initial state
        assert!(output.contains(" -11 -1"));  // positron, initial state
        assert!(output.contains(" 13 1"));    // muon, final state
        assert!(output.contains(" -13 1"));   // anti-muon, final state
    }

    #[test]
    fn lhe_event_contains_momentum_values() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        // Check that the muon momentum values appear somewhere.
        // px=30 → should appear as "3E1" or "3.0E1" in scientific notation.
        assert!(output.contains("E1") || output.contains("e1") || output.contains("E+"));
    }

    #[test]
    fn lhe_multiple_events() {
        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.write_event(&sample_event_record()).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        let event_count = output.matches("<event>").count();
        assert_eq!(event_count, 3);
        assert_eq!(output.matches("</event>").count(), 3);
    }

    #[test]
    fn lhe_init_block_process_count() {
        let config = InitConfig {
            beam1_id: 2212,
            beam2_id: 2212,
            beam1_energy: 6500.0,
            beam2_energy: 6500.0,
            pdf_group1: 0,
            pdf_group2: 0,
            pdf_set1: 0,
            pdf_set2: 0,
            weight_strategy: -1,
            processes: vec![
                ProcessInfo {
                    cross_section: 1.0e-3,
                    error: 1.0e-5,
                    max_weight: 1.0,
                    process_id: 1,
                },
                ProcessInfo {
                    cross_section: 2.0e-3,
                    error: 2.0e-5,
                    max_weight: 1.0,
                    process_id: 2,
                },
            ],
            generator: "SPIRE".into(),
        };

        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&config).unwrap();
            writer.finish().unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        // The init header should end with "2" (number of processes).
        let init_start = output.find("<init>").unwrap();
        let init_body = &output[init_start..];
        let first_line = init_body.lines().nth(1).unwrap();
        assert!(first_line.trim().ends_with("2"));
    }

    #[test]
    fn event_record_from_phase_space_basic() {
        let psp = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 30.0, 40.0, 86.6),
                SpacetimeVector::new_4d(100.0, -30.0, -40.0, -86.6),
            ],
            weight: 0.5,
        };

        let record = event_record_from_phase_space(
            &psp,
            &[11, -11],
            &[13, -13],
            &[0.10566, 0.10566],
            100.0,
            1,
            200.0,
            7.297e-3,
            0.118,
        );

        // 2 initial + 2 final = 4 particles.
        assert_eq!(record.particles.len(), 4);
        // First two are initial state.
        assert_eq!(record.particles[0].status, -1);
        assert_eq!(record.particles[1].status, -1);
        // Last two are final state.
        assert_eq!(record.particles[2].status, 1);
        assert_eq!(record.particles[3].status, 1);
        // PDG IDs.
        assert_eq!(record.particles[0].pdg_id, 11);
        assert_eq!(record.particles[2].pdg_id, 13);
        // Final-state momenta come from the PhaseSpacePoint.
        assert!((record.particles[2].px - 30.0).abs() < 1e-10);
        assert!((record.particles[3].py - (-40.0)).abs() < 1e-10);
        // Weight.
        assert!((record.weight - 0.5).abs() < 1e-10);
    }

    #[test]
    fn full_lhe_roundtrip() {
        // Generate a phase-space event, convert to EventRecord, write LHE, verify.
        let psp = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 10.0, 20.0, 40.0),
                SpacetimeVector::new_4d(50.0, -10.0, -20.0, -40.0),
            ],
            weight: 1.0,
        };

        let record = event_record_from_phase_space(
            &psp,
            &[11, -11],
            &[13, -13],
            &[0.10566, 0.10566],
            50.0,
            1,
            100.0,
            7.297e-3,
            0.118,
        );

        let mut buf = Vec::new();
        {
            let mut writer = LheWriter::new(&mut buf);
            writer.write_header(&sample_init_config()).unwrap();
            writer.write_event(&record).unwrap();
            writer.finish().unwrap();
        }

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("<LesHouchesEvents"));
        assert!(output.contains("<event>"));
        assert!(output.contains("</event>"));
        assert!(output.contains("</LesHouchesEvents>"));
        // Event should have 4 particles.
        let event_start = output.find("<event>").unwrap();
        let event_body = &output[event_start..];
        let first_line = event_body.lines().nth(1).unwrap();
        assert!(first_line.trim().starts_with("4"));
    }

    #[test]
    fn lhe_writer_with_custom_capacity() {
        let buf: Vec<u8> = Vec::new();
        let mut writer = LheWriter::with_capacity(32 * 1024, buf);
        writer.write_header(&sample_init_config()).unwrap();
        writer.finish().unwrap();
        // Verify the underlying buffer has content after flush.
        let output = String::from_utf8(writer.get_ref().clone()).unwrap();
        assert!(output.contains("<LesHouchesEvents"));
    }
}
