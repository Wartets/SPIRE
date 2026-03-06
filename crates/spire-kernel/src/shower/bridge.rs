//! # Bridge - External Parton Shower Provider Interface
//!
//! Implements the modular bridge between SPIRE's parton-level event
//! generation and external parton shower / hadronisation programs.
//!
//! ## Design Principles
//!
//! 1. **No hard-coded Pythia assumptions** - the `PartonShowerProvider`
//!    trait can be implemented for any external program (Herwig, Sherpa,
//!    Vincia, Dire, etc.).
//!
//! 2. **File-based interchange** - events are exchanged via standard HEP
//!    file formats (LHE in, HepMC out), ensuring compatibility with the
//!    entire MC ecosystem.
//!
//! 3. **CLI-first execution** - the default `PythiaCLIProvider` invokes
//!    Pythia as a subprocess, requiring only that the user has a working
//!    Pythia installation with a suitable run script.
//!
//! ## HepMC3 Parsing
//!
//! A minimal HepMC3 ASCII parser (`parse_hepmc3_events`) is provided to
//! read the showered output back into SPIRE's event model. This parser
//! handles the subset of HepMC3 needed for final-state particle readback.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::io::EventRecord;
use crate::{SpireError, SpireResult};

// ===========================================================================
// Configuration
// ===========================================================================

/// Configuration for a parton shower run.
///
/// Contains all parameters needed to configure an external shower
/// program. Additional key-value settings can be passed through the
/// `extra` map for program-specific options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowerConfig {
    /// Path to the shower executable or run script.
    pub executable: PathBuf,

    /// Working directory for the shower run. Temporary files (LHE
    /// input, HepMC output) will be written here.
    pub work_dir: PathBuf,

    /// Centre-of-mass energy $\sqrt{s}$ in GeV.
    pub cms_energy: f64,

    /// Maximum number of events to shower. If `None`, all input events
    /// are processed.
    pub max_events: Option<usize>,

    /// Random seed for the shower program (reproducibility).
    pub seed: u64,

    /// Whether to include hadronisation after the parton shower.
    pub hadronisation: bool,

    /// Whether to include QED radiation (photon emission).
    pub qed_radiation: bool,

    /// Whether to include multi-parton interactions (underlying event).
    pub mpi: bool,

    /// Additional program-specific settings as key-value pairs.
    /// For Pythia these might be strings like `"TimeShower:alphaSvalue"`.
    pub extra: HashMap<String, String>,
}

impl Default for ShowerConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("pythia8-main"),
            work_dir: PathBuf::from("."),
            cms_energy: 13000.0,
            max_events: None,
            seed: 42,
            hadronisation: true,
            qed_radiation: false,
            mpi: false,
            extra: HashMap::new(),
        }
    }
}

// ===========================================================================
// Shower Result
// ===========================================================================

/// A single particle in the showered event record.
///
/// Mirrors `EventParticle` but uses a dedicated type to distinguish
/// parton-level from showered particles at the type level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoweredParticle {
    /// PDG Monte Carlo particle ID.
    pub pdg_id: i32,
    /// Status code (1 = final state, 2 = intermediate, etc.).
    pub status: i32,
    /// Four-momentum components: $(p_x, p_y, p_z, E)$ in GeV.
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub energy: f64,
    /// Generated mass in GeV.
    pub mass: f64,
}

/// A single showered event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShoweredEvent {
    /// Event number / identifier.
    pub event_id: usize,
    /// All particles (including intermediates if available).
    pub particles: Vec<ShoweredParticle>,
    /// Event weight.
    pub weight: f64,
}

/// The result of running a parton shower over a set of events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowerResult {
    /// The showered events.
    pub events: Vec<ShoweredEvent>,

    /// Total number of input parton-level events.
    pub input_event_count: usize,

    /// Number of successfully showered events.
    pub showered_event_count: usize,

    /// Name of the shower program that produced these events.
    pub shower_program: String,

    /// Wall-clock time in seconds for the shower run.
    pub wall_time_seconds: f64,

    /// Any warnings or diagnostic messages from the shower program.
    pub diagnostics: Vec<String>,
}

// ===========================================================================
// PartonShowerProvider Trait
// ===========================================================================

/// Trait for external parton shower program integrations.
///
/// Each implementation wraps a specific shower program (Pythia, Herwig,
/// Sherpa, etc.) and handles:
///
/// 1. Writing the input events to a format the program understands (LHE).
/// 2. Generating a configuration / command card for the program.
/// 3. Invoking the program as a subprocess.
/// 4. Parsing the output (HepMC) back into SPIRE's event model.
///
/// # Object Safety
///
/// This trait is object-safe to allow runtime selection via
/// `Box<dyn PartonShowerProvider>`.
pub trait PartonShowerProvider: Send + Sync {
    /// Apply the parton shower to a set of input events.
    ///
    /// # Arguments
    ///
    /// * `events` - Parton-level event records (from SPIRE's MC integration).
    /// * `config` - Shower configuration parameters.
    ///
    /// # Returns
    ///
    /// A `ShowerResult` containing the showered events and diagnostics.
    fn apply_shower(
        &self,
        events: &[EventRecord],
        config: &ShowerConfig,
    ) -> SpireResult<ShowerResult>;

    /// Human-readable name of the shower program.
    fn provider_name(&self) -> &str;

    /// Check whether the shower executable is available on the system.
    fn is_available(&self, config: &ShowerConfig) -> bool;
}

// ===========================================================================
// PythiaCLIProvider
// ===========================================================================

/// Parton shower provider that invokes Pythia 8 as a CLI subprocess.
///
/// ## Workflow
///
/// 1. Writes the input events to an LHE file in the work directory.
/// 2. Generates a Pythia command card (`.cmnd` file) with the requested
///    settings.
/// 3. Runs the Pythia executable, passing the command card.
/// 4. Parses the HepMC3 output file produced by Pythia.
///
/// ## Requirements
///
/// - A working Pythia 8 installation with a main program that accepts
///   a command card and writes HepMC3 output.
/// - The executable path is specified in `ShowerConfig::executable`.
#[derive(Debug, Clone, Default)]
pub struct PythiaCLIProvider;

impl PythiaCLIProvider {
    /// Generate a Pythia command card as a string.
    fn generate_command_card(
        &self,
        lhe_path: &Path,
        hepmc_path: &Path,
        config: &ShowerConfig,
    ) -> String {
        let mut card = String::new();

        card.push_str("! SPIRE auto-generated Pythia8 command card\n");
        card.push_str("! ─────────────────────────────────────────\n\n");

        // Beam configuration (LHE mode)
        card.push_str("Beams:frameType = 4\n");
        card.push_str(&format!("Beams:LHEF = {}\n", lhe_path.display()));

        // Event limit
        if let Some(max) = config.max_events {
            card.push_str(&format!("Main:numberOfEvents = {}\n", max));
        }

        // Random seed
        card.push_str("Random:setSeed = on\n");
        card.push_str(&format!("Random:seed = {}\n", config.seed));

        // Shower and hadronisation switches
        card.push_str(&format!(
            "HadronLevel:Hadronize = {}\n",
            if config.hadronisation { "on" } else { "off" }
        ));
        card.push_str(&format!(
            "TimeShower:QEDshowerByQ = {}\n",
            if config.qed_radiation { "on" } else { "off" }
        ));
        card.push_str(&format!(
            "PartonLevel:MPI = {}\n",
            if config.mpi { "on" } else { "off" }
        ));

        // HepMC output
        card.push_str(&format!(
            "Main:HepMCoutput:file = {}\n",
            hepmc_path.display()
        ));

        // Extra user-defined settings
        for (key, val) in &config.extra {
            card.push_str(&format!("{} = {}\n", key, val));
        }

        card
    }
}

impl PartonShowerProvider for PythiaCLIProvider {
    fn provider_name(&self) -> &str {
        "Pythia 8 (CLI)"
    }

    fn is_available(&self, config: &ShowerConfig) -> bool {
        Command::new(&config.executable)
            .arg("--help")
            .output()
            .is_ok()
    }

    fn apply_shower(
        &self,
        events: &[EventRecord],
        config: &ShowerConfig,
    ) -> SpireResult<ShowerResult> {
        let start = std::time::Instant::now();

        // 1. Ensure work directory exists
        std::fs::create_dir_all(&config.work_dir)
            .map_err(|e| SpireError::InternalError(format!("Cannot create work dir: {}", e)))?;

        let lhe_path = config.work_dir.join("spire_input.lhe");
        let hepmc_path = config.work_dir.join("spire_showered.hepmc");
        let cmnd_path = config.work_dir.join("spire_run.cmnd");

        // 2. Write input events to LHE
        write_minimal_lhe(&lhe_path, events, config.cms_energy)?;

        // 3. Write command card
        let card = self.generate_command_card(&lhe_path, &hepmc_path, config);
        std::fs::write(&cmnd_path, &card)
            .map_err(|e| SpireError::InternalError(format!("Cannot write command card: {}", e)))?;

        // 4. Invoke Pythia
        let output = Command::new(&config.executable)
            .arg(cmnd_path.to_string_lossy().as_ref())
            .current_dir(&config.work_dir)
            .output()
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Failed to execute shower program '{}': {}",
                    config.executable.display(),
                    e
                ))
            })?;

        let mut diagnostics = Vec::new();
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.push(format!("Shower exit code: {}", output.status));
            if !stderr.is_empty() {
                diagnostics.push(format!(
                    "stderr: {}",
                    stderr.chars().take(2000).collect::<String>()
                ));
            }
        }

        // 5. Parse HepMC output
        let showered_events = if hepmc_path.exists() {
            parse_hepmc3_events(&hepmc_path)?
        } else {
            diagnostics.push("No HepMC output file found".into());
            Vec::new()
        };

        let wall_time = start.elapsed().as_secs_f64();

        Ok(ShowerResult {
            showered_event_count: showered_events.len(),
            events: showered_events,
            input_event_count: events.len(),
            shower_program: "Pythia 8".into(),
            wall_time_seconds: wall_time,
            diagnostics,
        })
    }
}

// ===========================================================================
// Minimal LHE Writer (for shower input)
// ===========================================================================

/// Write a minimal LHE file from event records for shower input.
///
/// This is a simplified writer that produces a valid LHE v1.0 file
/// sufficient for Pythia/Herwig to read. For full-featured LHE output,
/// use `crate::io::lhe::LheWriter`.
fn write_minimal_lhe(path: &Path, events: &[EventRecord], cms_energy: f64) -> SpireResult<()> {
    use std::io::Write;

    let file = std::fs::File::create(path)
        .map_err(|e| SpireError::InternalError(format!("Cannot create LHE file: {}", e)))?;
    let mut w = std::io::BufWriter::new(file);

    // Header
    writeln!(w, "<LesHouchesEvents version=\"1.0\">").map_err(write_err)?;
    writeln!(w, "<!--  Generated by SPIRE for shower input  -->").map_err(write_err)?;

    // Init block (minimal)
    let beam_e = cms_energy / 2.0;
    writeln!(w, "<init>").map_err(write_err)?;
    writeln!(w, "  2212 2212 {0:.6E} {0:.6E} 0 0 0 0 1 1", beam_e).map_err(write_err)?;
    writeln!(w, "  1.0000E+00 0.0000E+00 1.0000E+00 1").map_err(write_err)?;
    writeln!(w, "</init>").map_err(write_err)?;

    // Events
    for event in events {
        let np = event.particles.len();
        writeln!(
            w,
            "<event>\n {} {} {:.6E} {:.6E} {:.6E} {:.6E}",
            np, event.process_id, event.weight, event.scale, event.alpha_qed, event.alpha_qcd
        )
        .map_err(write_err)?;

        for p in &event.particles {
            writeln!(
                w,
                " {} {} {} {} {} {} {:.10E} {:.10E} {:.10E} {:.10E} {:.10E} {:.4E} {:.1}",
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
                p.spin
            )
            .map_err(write_err)?;
        }

        writeln!(w, "</event>").map_err(write_err)?;
    }

    writeln!(w, "</LesHouchesEvents>").map_err(write_err)?;

    Ok(())
}

fn write_err(e: std::io::Error) -> SpireError {
    SpireError::InternalError(format!("LHE write error: {}", e))
}

// ===========================================================================
// HepMC3 ASCII Parser (Minimal)
// ===========================================================================

/// Parse a HepMC3 ASCII event file into `ShoweredEvent` records.
///
/// This parser handles the minimal subset of the HepMC3 format needed
/// for reading final-state particles:
///
/// - `E` lines: event header (event number, n_vertices, n_particles)
/// - `P` lines: particle records (id, pdg, px, py, pz, e, m, status)
/// - `W` lines: event weights
///
/// Lines starting with other prefixes are silently ignored.
///
/// # Format Reference
///
/// HepMC3 ASCII format specification:
/// <https://gitlab.cern.ch/hepmc/HepMC3>
pub fn parse_hepmc3_events(path: &Path) -> SpireResult<Vec<ShoweredEvent>> {
    let file = std::fs::File::open(path).map_err(|e| {
        SpireError::InternalError(format!(
            "Cannot open HepMC file '{}': {}",
            path.display(),
            e
        ))
    })?;

    let reader = BufReader::new(file);
    let mut events: Vec<ShoweredEvent> = Vec::new();
    let mut current_event: Option<ShoweredEvent> = None;

    for line_result in reader.lines() {
        let line = line_result
            .map_err(|e| SpireError::InternalError(format!("HepMC read error: {}", e)))?;

        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("E ") {
            // Flush previous event
            if let Some(ev) = current_event.take() {
                events.push(ev);
            }

            // Parse event header: E <event_number> <n_vertices> <n_particles>
            let parts: Vec<&str> = line.split_whitespace().collect();
            let event_id = parts
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(events.len());

            current_event = Some(ShoweredEvent {
                event_id,
                particles: Vec::new(),
                weight: 1.0,
            });
        } else if line.starts_with("P ") {
            // Parse particle line:
            // P <id> <vertex_id> <pdg_id> <px> <py> <pz> <e> <m> <status>
            if let Some(ref mut ev) = current_event {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let pdg_id = parts[3].parse::<i32>().unwrap_or(0);
                    let px = parts[4].parse::<f64>().unwrap_or(0.0);
                    let py = parts[5].parse::<f64>().unwrap_or(0.0);
                    let pz = parts[6].parse::<f64>().unwrap_or(0.0);
                    let energy = parts[7].parse::<f64>().unwrap_or(0.0);
                    let mass = parts[8].parse::<f64>().unwrap_or(0.0);
                    let status = parts[9].parse::<i32>().unwrap_or(0);

                    ev.particles.push(ShoweredParticle {
                        pdg_id,
                        status,
                        px,
                        py,
                        pz,
                        energy,
                        mass,
                    });
                }
            }
        } else if line.starts_with("W ") {
            // Weight line: W <weight>
            if let Some(ref mut ev) = current_event {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(w) = parts.get(1).and_then(|s| s.parse::<f64>().ok()) {
                    ev.weight = w;
                }
            }
        }
        // All other line prefixes (V, A, U, etc.) are silently skipped
    }

    // Flush last event
    if let Some(ev) = current_event.take() {
        events.push(ev);
    }

    Ok(events)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::EventParticle;
    use std::io::Write;

    // ── ShowerConfig tests ──────────────────────────────────────────────

    #[test]
    fn default_config_has_sensible_values() {
        let cfg = ShowerConfig::default();
        assert_eq!(cfg.cms_energy, 13000.0);
        assert_eq!(cfg.seed, 42);
        assert!(cfg.hadronisation);
        assert!(!cfg.qed_radiation);
        assert!(!cfg.mpi);
        assert!(cfg.extra.is_empty());
    }

    // ── PythiaCLIProvider tests ─────────────────────────────────────────

    #[test]
    fn pythia_provider_name() {
        let p = PythiaCLIProvider;
        assert_eq!(p.provider_name(), "Pythia 8 (CLI)");
    }

    #[test]
    fn pythia_command_card_generation() {
        let provider = PythiaCLIProvider;
        let config = ShowerConfig {
            seed: 123,
            cms_energy: 14000.0,
            max_events: Some(1000),
            hadronisation: false,
            qed_radiation: true,
            mpi: false,
            ..Default::default()
        };

        let card = provider.generate_command_card(
            Path::new("input.lhe"),
            Path::new("output.hepmc"),
            &config,
        );

        assert!(card.contains("Beams:frameType = 4"));
        assert!(card.contains("Beams:LHEF = input.lhe"));
        assert!(card.contains("Main:numberOfEvents = 1000"));
        assert!(card.contains("Random:seed = 123"));
        assert!(card.contains("HadronLevel:Hadronize = off"));
        assert!(card.contains("TimeShower:QEDshowerByQ = on"));
        assert!(card.contains("PartonLevel:MPI = off"));
    }

    #[test]
    fn pythia_command_card_extra_settings() {
        let provider = PythiaCLIProvider;
        let mut config = ShowerConfig::default();
        config
            .extra
            .insert("TimeShower:alphaSvalue".into(), "0.118".into());

        let card =
            provider.generate_command_card(Path::new("in.lhe"), Path::new("out.hepmc"), &config);

        assert!(card.contains("TimeShower:alphaSvalue = 0.118"));
    }

    #[test]
    fn pythia_not_available_with_fake_path() {
        let provider = PythiaCLIProvider;
        let config = ShowerConfig {
            executable: PathBuf::from("/nonexistent/path/to/pythia"),
            ..Default::default()
        };
        assert!(!provider.is_available(&config));
    }

    // ── HepMC3 parser tests ────────────────────────────────────────────

    #[test]
    fn parse_hepmc3_single_event() {
        let content = r#"HepMC::Version 3.02.05
HepMC::Asciiv3-START_EVENT_LISTING
E 0 1 4
U GEV MM
W 1.0
P 1 0 2212 0.0 0.0 6500.0 6500.0 0.938 4
P 2 0 2212 0.0 0.0 -6500.0 6500.0 0.938 4
P 3 -1 11 10.5 -3.2 45.6 47.1 0.000511 1
P 4 -1 -11 -10.5 3.2 -45.6 47.1 0.000511 1
HepMC::Asciiv3-END_EVENT_LISTING
"#;

        // Write to temp file
        let dir = std::env::temp_dir().join("spire_test_hepmc");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_single.hepmc");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }

        let events = parse_hepmc3_events(&path).unwrap();
        assert_eq!(events.len(), 1);

        let ev = &events[0];
        assert_eq!(ev.event_id, 0);
        assert_eq!(ev.particles.len(), 4);
        assert!((ev.weight - 1.0).abs() < 1e-10);

        // Check electron (3rd particle)
        let electron = &ev.particles[2];
        assert_eq!(electron.pdg_id, 11);
        assert_eq!(electron.status, 1);
        assert!((electron.px - 10.5).abs() < 1e-10);
        assert!((electron.energy - 47.1).abs() < 1e-10);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_hepmc3_multiple_events() {
        let content = r#"HepMC::Version 3.02.05
HepMC::Asciiv3-START_EVENT_LISTING
E 0 1 2
U GEV MM
W 0.5
P 1 0 22 1.0 0.0 0.0 1.0 0.0 1
P 2 0 22 -1.0 0.0 0.0 1.0 0.0 1
E 1 1 2
U GEV MM
W 1.5
P 1 0 11 0.0 1.0 0.0 1.0 0.000511 1
P 2 0 -11 0.0 -1.0 0.0 1.0 0.000511 1
HepMC::Asciiv3-END_EVENT_LISTING
"#;

        let dir = std::env::temp_dir().join("spire_test_hepmc_multi");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_multi.hepmc");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(content.as_bytes()).unwrap();
        }

        let events = parse_hepmc3_events(&path).unwrap();
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].event_id, 0);
        assert!((events[0].weight - 0.5).abs() < 1e-10);
        assert_eq!(events[0].particles.len(), 2);
        assert_eq!(events[0].particles[0].pdg_id, 22); // photon

        assert_eq!(events[1].event_id, 1);
        assert!((events[1].weight - 1.5).abs() < 1e-10);
        assert_eq!(events[1].particles.len(), 2);
        assert_eq!(events[1].particles[0].pdg_id, 11); // electron

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_hepmc3_empty_file() {
        let dir = std::env::temp_dir().join("spire_test_hepmc_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.hepmc");
        std::fs::write(&path, "# just a comment\n").unwrap();

        let events = parse_hepmc3_events(&path).unwrap();
        assert_eq!(events.len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_hepmc3_missing_file() {
        let result = parse_hepmc3_events(Path::new("/nonexistent/file.hepmc"));
        assert!(result.is_err());
    }

    // ── Minimal LHE writer tests ────────────────────────────────────────

    #[test]
    fn write_minimal_lhe_creates_valid_file() {
        let dir = std::env::temp_dir().join("spire_test_lhe_min");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.lhe");

        let events = vec![EventRecord {
            particles: vec![
                EventParticle {
                    pdg_id: 2,
                    status: -1,
                    mother1: 0,
                    mother2: 0,
                    color1: 501,
                    color2: 0,
                    px: 0.0,
                    py: 0.0,
                    pz: 500.0,
                    energy: 500.0,
                    mass: 0.0,
                    lifetime: 0.0,
                    spin: 9.0,
                },
                EventParticle {
                    pdg_id: -2,
                    status: -1,
                    mother1: 0,
                    mother2: 0,
                    color1: 0,
                    color2: 501,
                    px: 0.0,
                    py: 0.0,
                    pz: -500.0,
                    energy: 500.0,
                    mass: 0.0,
                    lifetime: 0.0,
                    spin: 9.0,
                },
                EventParticle {
                    pdg_id: 11,
                    status: 1,
                    mother1: 1,
                    mother2: 2,
                    color1: 0,
                    color2: 0,
                    px: 100.0,
                    py: 200.0,
                    pz: 300.0,
                    energy: 500.0,
                    mass: 0.000511,
                    lifetime: 0.0,
                    spin: 9.0,
                },
            ],
            weight: 1.0,
            scale: 91.2,
            alpha_qed: 1.0 / 128.0,
            alpha_qcd: 0.118,
            process_id: 1,
        }];

        write_minimal_lhe(&path, &events, 1000.0).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<LesHouchesEvents"));
        assert!(content.contains("<init>"));
        assert!(content.contains("</init>"));
        assert!(content.contains("<event>"));
        assert!(content.contains("</event>"));
        assert!(content.contains("</LesHouchesEvents>"));
        assert!(content.contains("2212 2212")); // proton beams
        assert!(content.contains("11")); // electron PDG ID

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_minimal_lhe_empty_events() {
        let dir = std::env::temp_dir().join("spire_test_lhe_empty");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.lhe");

        write_minimal_lhe(&path, &[], 1000.0).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<LesHouchesEvents"));
        assert!(content.contains("</LesHouchesEvents>"));
        // No event blocks
        assert!(!content.contains("<event>"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── ShoweredParticle tests ──────────────────────────────────────────

    #[test]
    fn showered_particle_serialization() {
        let p = ShoweredParticle {
            pdg_id: 13,
            status: 1,
            px: 10.0,
            py: 20.0,
            pz: 30.0,
            energy: 40.0,
            mass: 0.10566,
        };

        let json = serde_json::to_string(&p).unwrap();
        let deser: ShoweredParticle = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.pdg_id, 13);
        assert!((deser.mass - 0.10566).abs() < 1e-10);
    }

    #[test]
    fn shower_result_empty() {
        let r = ShowerResult {
            events: Vec::new(),
            input_event_count: 0,
            showered_event_count: 0,
            shower_program: "test".into(),
            wall_time_seconds: 0.0,
            diagnostics: Vec::new(),
        };
        assert_eq!(r.events.len(), 0);
    }
}
