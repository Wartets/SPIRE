//! # SPIRE CLI — Headless HPC Daemon
//!
//! A command-line interface for running SPIRE physics computations without
//! a graphical frontend. Designed for High-Performance Computing (HPC)
//! clusters (e.g., CERN LXBATCH, SLURM-managed nodes) where no display
//! server is available.
//!
//! ## Usage
//!
//! ```bash
//! spire-cli --config run_card.toml
//! spire-cli --config run_card.toml --events 1000000 --output events.lhe
//! spire-cli --info  # Print hardware capabilities
//! ```
//!
//! ## Run Card Format
//!
//! The run card is a TOML file specifying the theoretical model, reaction
//! process, integration parameters, and output format. See `data/run_card.toml`
//! for a complete example.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

use spire_kernel::data_loader;
use spire_kernel::integration::{compute_cross_section, IntegrationResult, UniformIntegrator};
use spire_kernel::io::lhe::LheWriter;
use spire_kernel::io::{EventParticle, EventRecord, EventWriter as _, InitConfig, ProcessInfo};
use spire_kernel::kinematics::{PhaseSpaceGenerator as _, RamboGenerator};
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::s_matrix;
use spire_kernel::SpireError;

// ===========================================================================
// CLI Argument Parser
// ===========================================================================

/// SPIRE CLI — Headless Monte Carlo event generation and cross-section
/// computation for High-Performance Computing environments.
#[derive(Parser, Debug)]
#[command(name = "spire-cli")]
#[command(version, about, long_about = None)]
struct CliArgs {
    /// Path to the TOML run card configuration file.
    #[arg(short, long)]
    config: PathBuf,

    /// Override the number of Monte Carlo events.
    #[arg(short = 'n', long)]
    events: Option<usize>,

    /// Override the output file path.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Override the random seed.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Print hardware capabilities and exit.
    #[arg(long)]
    info: bool,

    /// Suppress progress output (quiet mode for batch jobs).
    #[arg(short, long)]
    quiet: bool,
}

// ===========================================================================
// Run Card Configuration Schema
// ===========================================================================

/// Complete run card configuration for a headless SPIRE computation.
///
/// This struct is deserialized from a TOML file and contains all parameters
/// needed to execute a Monte Carlo integration without user interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpireRunConfig {
    /// Process definition: model, initial/final states, energy.
    pub process: ProcessConfig,
    /// Integration parameters: event count, seed, backend selection.
    pub integration: IntegrationConfig,
    /// Output configuration: format, file path, verbosity.
    pub output: OutputConfig,
}

/// Process definition within the run card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// Path to the particles TOML file.
    pub particles_toml: PathBuf,
    /// Path to the vertices TOML file.
    pub vertices_toml: PathBuf,
    /// Name of the theoretical model.
    #[serde(default = "default_model_name")]
    pub model_name: String,
    /// Initial-state particle IDs (e.g., `["e-", "e+"]`).
    pub initial_state: Vec<String>,
    /// Final-state particle IDs (e.g., `["mu-", "mu+"]`).
    pub final_state: Vec<String>,
    /// Centre-of-mass energy $\sqrt{s}$ in GeV.
    pub sqrt_s: f64,
}

/// Integration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Number of Monte Carlo events to generate.
    #[serde(default = "default_num_events")]
    pub num_events: usize,
    /// Random number generator seed (for reproducibility).
    #[serde(default = "default_seed")]
    pub seed: u64,
    /// Whether to use GPU acceleration (requires `gpu` feature).
    #[serde(default)]
    pub use_gpu: bool,
}

/// Output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output format: `"lhe"`, `"json"`, or `"stdout"`.
    #[serde(default = "default_format")]
    pub format: String,
    /// Output file path. If absent, writes to stdout.
    pub file: Option<PathBuf>,
}

fn default_model_name() -> String {
    "Standard Model".into()
}
fn default_num_events() -> usize {
    10000
}
fn default_seed() -> u64 {
    42
}
fn default_format() -> String {
    "lhe".into()
}

// ===========================================================================
// Hardware Information
// ===========================================================================

/// Print system hardware capabilities relevant to SPIRE computation.
fn print_hardware_info() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║           SPIRE — Hardware Capability Report        ║");
    println!("╠══════════════════════════════════════════════════════╣");
    println!(
        "║  CPU threads (logical):  {:>4}                        ║",
        rayon_threads()
    );
    println!(
        "║  Rayon thread pool:      {:>4}                        ║",
        rayon_threads()
    );

    #[cfg(feature = "gpu")]
    {
        println!("║  GPU backend:            WebGPU (wgpu)              ║");
        println!("║  GPU status:             Feature enabled             ║");
    }
    #[cfg(not(feature = "gpu"))]
    {
        println!("║  GPU backend:            Not compiled                ║");
        println!("║  GPU status:             Build with --features gpu   ║");
    }

    println!("║  SPIRE kernel version:   {}                    ║", env!("CARGO_PKG_VERSION"));
    println!("╚══════════════════════════════════════════════════════╝");
}

fn rayon_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

// ===========================================================================
// Main Execution Engine
// ===========================================================================

/// Execute a full Monte Carlo run from a parsed run card.
fn execute_run(config: &SpireRunConfig, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    let t_start = Instant::now();

    // -----------------------------------------------------------------------
    // Step 1: Load the theoretical model
    // -----------------------------------------------------------------------
    if !quiet {
        eprintln!(
            "[SPIRE] Loading model '{}' from {:?} + {:?}",
            config.process.model_name,
            config.process.particles_toml,
            config.process.vertices_toml
        );
    }

    let particles_toml = fs::read_to_string(&config.process.particles_toml).map_err(|e| {
        format!(
            "Failed to read particles file {:?}: {}",
            config.process.particles_toml, e
        )
    })?;
    let vertices_toml = fs::read_to_string(&config.process.vertices_toml).map_err(|e| {
        format!(
            "Failed to read vertices file {:?}: {}",
            config.process.vertices_toml, e
        )
    })?;

    let model = data_loader::build_model(&particles_toml, &vertices_toml, &config.process.model_name)?;

    if !quiet {
        eprintln!(
            "[SPIRE] Model loaded: {} fields, {} vertices",
            model.fields.len(),
            model.terms.len()
        );
    }

    // -----------------------------------------------------------------------
    // Step 2: Construct and validate the reaction
    // -----------------------------------------------------------------------
    let initial_refs: Vec<&str> = config.process.initial_state.iter().map(|s| s.as_str()).collect();
    let final_refs: Vec<&str> = config.process.final_state.iter().map(|s| s.as_str()).collect();
    let reaction = s_matrix::construct_reaction(
        &initial_refs,
        &final_refs,
        &model,
        Some(config.process.sqrt_s),
    )?;

    if !reaction.is_valid {
        let diag = reaction.violation_diagnostics.join("; ");
        return Err(format!("Reaction validation failed: {}", diag).into());
    }

    if !quiet {
        eprintln!(
            "[SPIRE] Reaction validated: {} → {}  at √s = {:.2} GeV",
            config.process.initial_state.join(" "),
            config.process.final_state.join(" "),
            config.process.sqrt_s
        );
    }

    // -----------------------------------------------------------------------
    // Step 3: Resolve final-state masses
    // -----------------------------------------------------------------------
    let final_masses: Vec<f64> = resolve_final_masses(&config.process.final_state, &model)?;

    // -----------------------------------------------------------------------
    // Step 4: Run Monte Carlo integration
    // -----------------------------------------------------------------------
    let num_events = config.integration.num_events;
    let mut generator = RamboGenerator::with_seed(config.integration.seed);
    let integrator = UniformIntegrator::new();

    if !quiet {
        eprintln!(
            "[SPIRE] Running {} events (seed={}, backend=CPU/rayon)",
            num_events, config.integration.seed
        );
    }

    let t_integration = Instant::now();

    // Use a constant |M|² = 1.0 for the generic headless mode.
    // In a complete workflow, the user would provide an amplitude
    // via the scripting engine or the compiled WGSL shader pipeline.
    let result = compute_cross_section(
        |_| 1.0,
        &integrator,
        &mut generator,
        config.process.sqrt_s,
        &final_masses,
        num_events,
    )?;

    let t_integration_elapsed = t_integration.elapsed();

    if !quiet {
        let result_pb = result.to_picobarns();
        eprintln!("[SPIRE] ─────────────────────────────────────────────");
        eprintln!(
            "[SPIRE] Cross-section:  {:.6e} GeV⁻²  ({:.6e} pb)",
            result.value, result_pb.value
        );
        eprintln!(
            "[SPIRE] Uncertainty:    {:.6e} GeV⁻²  ({:.6e} pb)",
            result.error, result_pb.error
        );
        eprintln!("[SPIRE] Relative error: {:.4}%", result.relative_error * 100.0);
        eprintln!("[SPIRE] Events:         {}", result.events_evaluated);
        eprintln!("[SPIRE] Efficiency:     {:.4}%", result.efficiency * 100.0);
        eprintln!(
            "[SPIRE] Integration:    {:.3}s",
            t_integration_elapsed.as_secs_f64()
        );
    }

    // -----------------------------------------------------------------------
    // Step 5: Write output
    // -----------------------------------------------------------------------
    write_output(config, &result, &final_masses, &mut generator, quiet)?;

    let t_total = t_start.elapsed();
    if !quiet {
        eprintln!(
            "[SPIRE] Total elapsed:  {:.3}s",
            t_total.as_secs_f64()
        );
        eprintln!("[SPIRE] Done.");
    }

    Ok(())
}

/// Resolve final-state particle masses from the theoretical model.
fn resolve_final_masses(
    final_ids: &[String],
    model: &TheoreticalModel,
) -> Result<Vec<f64>, SpireError> {
    final_ids
        .iter()
        .map(|id| {
            model
                .fields
                .iter()
                .find(|f| f.id == *id)
                .map(|f| f.mass)
                .ok_or_else(|| SpireError::UnknownParticle(id.clone()))
        })
        .collect()
}

/// Write the integration result and/or events to the configured output.
fn write_output(
    config: &SpireRunConfig,
    result: &IntegrationResult,
    final_masses: &[f64],
    generator: &mut RamboGenerator,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match config.output.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(result)?;
            match &config.output.file {
                Some(path) => {
                    fs::write(path, &json)?;
                    if !quiet {
                        eprintln!("[SPIRE] JSON output written to {:?}", path);
                    }
                }
                None => {
                    println!("{}", json);
                }
            }
        }
        "lhe" => {
            let path = config
                .output
                .file
                .as_ref()
                .ok_or("LHE output requires a file path")?;

            let file = fs::File::create(path)?;
            let buf = BufWriter::new(file);
            let mut writer = LheWriter::new(buf);

            // Build the LHE init configuration.
            let beam_energy = config.process.sqrt_s / 2.0;
            let result_pb = result.to_picobarns();

            let init_config = InitConfig {
                beam1_id: particle_id_to_pdg(&config.process.initial_state[0]),
                beam2_id: if config.process.initial_state.len() > 1 {
                    particle_id_to_pdg(&config.process.initial_state[1])
                } else {
                    0
                },
                beam1_energy: beam_energy,
                beam2_energy: beam_energy,
                pdf_group1: 0,
                pdf_group2: 0,
                pdf_set1: 0,
                pdf_set2: 0,
                weight_strategy: -1,
                processes: vec![ProcessInfo {
                    cross_section: result_pb.value,
                    error: result_pb.error,
                    max_weight: 1.0,
                    process_id: 1,
                }],
                generator: "SPIRE".into(),
            };

            writer.write_header(&init_config)?;

            // Generate and write events.
            let n_write = config.integration.num_events.min(result.events_evaluated);
            for _ in 0..n_write {
                if let Ok(ps_point) = generator.generate_event(config.process.sqrt_s, final_masses)
                {
                    let particles = build_event_particles(
                        &config.process.initial_state,
                        &config.process.final_state,
                        &ps_point,
                        config.process.sqrt_s,
                    );

                    let record = EventRecord {
                        particles,
                        weight: ps_point.weight,
                        scale: config.process.sqrt_s,
                        alpha_qed: 1.0 / 137.036,
                        alpha_qcd: 0.118,
                        process_id: 1,
                    };

                    writer.write_event(&record)?;
                }
            }

            writer.finish()?;

            if !quiet {
                eprintln!("[SPIRE] LHE output written to {:?} ({} events)", path, n_write);
            }
        }
        "stdout" => {
            let result_pb = result.to_picobarns();
            println!("cross_section_gev2 = {:.10e}", result.value);
            println!("uncertainty_gev2   = {:.10e}", result.error);
            println!("cross_section_pb   = {:.10e}", result_pb.value);
            println!("uncertainty_pb     = {:.10e}", result_pb.error);
            println!("relative_error     = {:.10e}", result.relative_error);
            println!("events_evaluated   = {}", result.events_evaluated);
            println!("efficiency         = {:.10e}", result.efficiency);
        }
        other => {
            return Err(format!("Unknown output format '{}'. Use 'lhe', 'json', or 'stdout'.", other).into());
        }
    }
    Ok(())
}

/// Build LHE event particles from a phase-space point.
fn build_event_particles(
    initial_ids: &[String],
    final_ids: &[String],
    ps_point: &spire_kernel::kinematics::PhaseSpacePoint,
    sqrt_s: f64,
) -> Vec<EventParticle> {
    let mut particles = Vec::new();

    // Initial-state particles (back-to-back along z-axis).
    let beam_energy = sqrt_s / 2.0;
    for (i, id) in initial_ids.iter().enumerate() {
        let pz = if i == 0 { beam_energy } else { -beam_energy };
        particles.push(EventParticle {
            pdg_id: particle_id_to_pdg(id),
            status: -1,
            mother1: 0,
            mother2: 0,
            color1: 0,
            color2: 0,
            px: 0.0,
            py: 0.0,
            pz,
            energy: beam_energy,
            mass: 0.0,
            lifetime: 0.0,
            spin: 9.0,
        });
    }

    // Final-state particles from phase-space momenta.
    for (i, id) in final_ids.iter().enumerate() {
        if i < ps_point.momenta.len() {
            let p = &ps_point.momenta[i];
            let comps = p.components();
            particles.push(EventParticle {
                pdg_id: particle_id_to_pdg(id),
                status: 1,
                mother1: 1,
                mother2: 2,
                color1: 0,
                color2: 0,
                px: if comps.len() > 1 { comps[1] } else { 0.0 },
                py: if comps.len() > 2 { comps[2] } else { 0.0 },
                pz: if comps.len() > 3 { comps[3] } else { 0.0 },
                energy: comps[0],
                mass: 0.0,
                lifetime: 0.0,
                spin: 9.0,
            });
        }
    }

    particles
}

/// Map common particle string IDs to PDG Monte Carlo codes.
///
/// This is a simplified mapping for the most common Standard Model particles.
/// A complete mapping would use the full PDG MC numbering scheme.
fn particle_id_to_pdg(id: &str) -> i32 {
    match id {
        "e-" | "electron" => 11,
        "e+" | "positron" => -11,
        "mu-" | "muon" => 13,
        "mu+" | "antimuon" => -13,
        "tau-" => 15,
        "tau+" => -15,
        "nu_e" => 12,
        "nu_e_bar" => -12,
        "nu_mu" => 14,
        "nu_mu_bar" => -14,
        "nu_tau" => 16,
        "nu_tau_bar" => -16,
        "u" | "up" => 2,
        "u_bar" => -2,
        "d" | "down" => 1,
        "d_bar" => -1,
        "c" | "charm" => 4,
        "c_bar" => -4,
        "s" | "strange" => 3,
        "s_bar" => -3,
        "t" | "top" => 6,
        "t_bar" => -6,
        "b" | "bottom" => 5,
        "b_bar" => -5,
        "photon" | "gamma" | "a" => 22,
        "Z" | "Z0" => 23,
        "W+" => 24,
        "W-" => -24,
        "gluon" | "g" => 21,
        "H" | "higgs" => 25,
        _ => 0, // Unknown
    }
}

// ===========================================================================
// Entry Point
// ===========================================================================

fn main() {
    let args = CliArgs::parse();

    // --info flag: print hardware report and exit.
    if args.info {
        print_hardware_info();
        return;
    }

    // Parse the run card.
    let config_str = match fs::read_to_string(&args.config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[SPIRE] Error: Failed to read config file {:?}: {}", args.config, e);
            std::process::exit(1);
        }
    };

    let mut config: SpireRunConfig = match toml::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SPIRE] Error: Failed to parse run card: {}", e);
            std::process::exit(1);
        }
    };

    // Apply CLI overrides.
    if let Some(n) = args.events {
        config.integration.num_events = n;
    }
    if let Some(seed) = args.seed {
        config.integration.seed = seed;
    }
    if let Some(ref path) = args.output {
        config.output.file = Some(path.clone());
    }

    // Execute the run.
    match execute_run(&config, args.quiet) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("[SPIRE] Error: {}", e);
            std::process::exit(1);
        }
    }
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_config_deserialize() {
        let toml_str = r#"
[process]
particles_toml = "data/particles.toml"
vertices_toml = "data/sm_vertices.toml"
model_name = "Test SM"
initial_state = ["e-", "e+"]
final_state = ["mu-", "mu+"]
sqrt_s = 91.2

[integration]
num_events = 1000
seed = 42
use_gpu = false

[output]
format = "json"
"#;
        let config: SpireRunConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.process.model_name, "Test SM");
        assert_eq!(config.process.initial_state, vec!["e-", "e+"]);
        assert_eq!(config.process.final_state, vec!["mu-", "mu+"]);
        assert!((config.process.sqrt_s - 91.2).abs() < 1e-10);
        assert_eq!(config.integration.num_events, 1000);
        assert_eq!(config.integration.seed, 42);
        assert!(!config.integration.use_gpu);
        assert_eq!(config.output.format, "json");
        assert!(config.output.file.is_none());
    }

    #[test]
    fn run_config_defaults() {
        let toml_str = r#"
[process]
particles_toml = "p.toml"
vertices_toml = "v.toml"
initial_state = ["e-", "e+"]
final_state = ["mu-", "mu+"]
sqrt_s = 100.0

[integration]

[output]
"#;
        let config: SpireRunConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.process.model_name, "Standard Model");
        assert_eq!(config.integration.num_events, 10000);
        assert_eq!(config.integration.seed, 42);
        assert!(!config.integration.use_gpu);
        assert_eq!(config.output.format, "lhe");
    }

    #[test]
    fn pdg_id_mapping() {
        assert_eq!(particle_id_to_pdg("e-"), 11);
        assert_eq!(particle_id_to_pdg("e+"), -11);
        assert_eq!(particle_id_to_pdg("mu-"), 13);
        assert_eq!(particle_id_to_pdg("mu+"), -13);
        assert_eq!(particle_id_to_pdg("photon"), 22);
        assert_eq!(particle_id_to_pdg("Z"), 23);
        assert_eq!(particle_id_to_pdg("W+"), 24);
        assert_eq!(particle_id_to_pdg("W-"), -24);
        assert_eq!(particle_id_to_pdg("gluon"), 21);
        assert_eq!(particle_id_to_pdg("H"), 25);
        assert_eq!(particle_id_to_pdg("u"), 2);
        assert_eq!(particle_id_to_pdg("t"), 6);
        assert_eq!(particle_id_to_pdg("unknown"), 0);
    }

    #[test]
    fn hardware_info_does_not_panic() {
        // Simply verify the function runs without panic.
        print_hardware_info();
    }

    #[test]
    fn run_config_with_output_file() {
        let toml_str = r#"
[process]
particles_toml = "p.toml"
vertices_toml = "v.toml"
initial_state = ["e-"]
final_state = ["e-"]
sqrt_s = 10.0

[integration]
num_events = 100

[output]
format = "lhe"
file = "output.lhe"
"#;
        let config: SpireRunConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.output.file.as_ref().unwrap().to_str().unwrap(), "output.lhe");
    }
}
