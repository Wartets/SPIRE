//! # SPIRE Desktop — Tauri Backend
//!
//! This is the Tauri application entry point. It defines IPC commands that the
//! SvelteKit frontend invokes via `@tauri-apps/api`. Each command delegates to
//! the `spire-kernel` for all physics computation.
//!
//! ## Stateless Isomorphism
//!
//! Following the architecture's "Isomorphic State Representation" principle,
//! the frontend holds all state. Complex objects like `TheoreticalModel` and
//! `Reaction` are passed **by value** (as JSON) across the IPC bridge rather
//! than being stored behind `Mutex` locks in the Tauri process.
//!
//! ## Error Handling
//!
//! All kernel errors (`SpireError`) are stringified at the boundary via
//! `.map_err(|e| e.to_string())` so that the frontend receives human-readable
//! error messages.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use spire_kernel::algebra::{self, AmplitudeExpression};
use spire_kernel::analysis::{AnalysisConfig, AnalysisResult};
use spire_kernel::data_loader;
use spire_kernel::graph::{self, FeynmanGraph, LoopOrder, TopologySet};
use spire_kernel::kinematics::{self, DalitzPlotData, MandelstamBoundaries, PhaseSpace, ThresholdResult};
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::ontology;
use spire_kernel::s_matrix::{self, Reaction, ReconstructedFinalState};

// ---------------------------------------------------------------------------
// KinematicsReport — aggregate return type
// ---------------------------------------------------------------------------

/// Aggregate result for the `compute_kinematics` command.
///
/// Bundles threshold information, 2-body phase space data, and
/// Mandelstam boundaries into a single serializable response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicsReport {
    /// Threshold energy results.
    pub threshold: ThresholdResult,
    /// Whether the process is kinematically allowed at the given energy.
    pub is_allowed: bool,
    /// Phase space description (if energy is above threshold).
    pub phase_space: Option<PhaseSpace>,
    /// Mandelstam boundaries (only for 2→2, i.e. exactly 4 masses with s provided).
    pub mandelstam_boundaries: Option<MandelstamBoundaries>,
}

// ---------------------------------------------------------------------------
// Tauri IPC Commands
// ---------------------------------------------------------------------------

/// Load a theoretical model from TOML data.
///
/// The frontend passes the raw contents of `particles.toml` and
/// `sm_vertices.toml` (or a custom BSM model). The kernel parses them
/// into a fully constructed `TheoreticalModel`.
///
/// # Returns
/// The complete `TheoreticalModel` as a serialized JSON object.
#[tauri::command]
fn load_theoretical_model(
    particles_toml: String,
    vertices_toml: String,
    model_name: Option<String>,
) -> Result<TheoreticalModel, String> {
    let name = model_name.unwrap_or_else(|| "Standard Model".into());
    data_loader::build_model(&particles_toml, &vertices_toml, &name).map_err(|e| e.to_string())
}

/// Construct and validate a particle reaction, then optionally reconstruct
/// permissible final states.
///
/// If `final_ids` is provided, constructs a specific reaction. Otherwise,
/// uses `reconstruct_reaction` to enumerate all kinematically and
/// dynamically allowed two-body final states.
///
/// # Returns
/// A serialized `Reaction` when `final_ids` is given, or a list of
/// `ReconstructedFinalState` when performing reconstruction.
#[tauri::command]
fn validate_and_reconstruct_reaction(
    initial_ids: Vec<String>,
    cms_energy: f64,
    model: TheoreticalModel,
    final_ids: Option<Vec<String>>,
) -> Result<serde_json::Value, String> {
    match final_ids {
        Some(finals) => {
            // Construct a specific reaction.
            let init_refs: Vec<&str> = initial_ids.iter().map(|s| s.as_str()).collect();
            let final_refs: Vec<&str> = finals.iter().map(|s| s.as_str()).collect();
            let reaction = s_matrix::construct_reaction(
                &init_refs,
                &final_refs,
                &model,
                Some(cms_energy),
            )
            .map_err(|e| e.to_string())?;

            serde_json::to_value(&reaction).map_err(|e| e.to_string())
        }
        None => {
            // Reconstruct all permissible two-body final states.
            let particles: Vec<_> = initial_ids
                .iter()
                .map(|id| {
                    model
                        .fields
                        .iter()
                        .find(|f| f.id == *id)
                        .ok_or_else(|| format!("Unknown particle: {}", id))
                        .map(|f| ontology::particle_from_field(f.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let results: Vec<ReconstructedFinalState> =
                s_matrix::reconstruct_reaction(&particles, &model, cms_energy)
                    .map_err(|e| e.to_string())?;

            serde_json::to_value(&results).map_err(|e| e.to_string())
        }
    }
}

/// Generate all topologically distinct Feynman diagrams for a reaction.
///
/// Takes a `Reaction` and `TheoreticalModel` (both passed by value from
/// the frontend's state) and returns the complete `TopologySet`.
#[tauri::command]
fn generate_feynman_diagrams(
    reaction: Reaction,
    model: TheoreticalModel,
    max_loop_order: Option<u32>,
) -> Result<TopologySet, String> {
    let order = match max_loop_order.unwrap_or(0) {
        0 => LoopOrder::Tree,
        1 => LoopOrder::OneLoop,
        2 => LoopOrder::TwoLoop,
        n => LoopOrder::NLoop(n),
    };
    graph::generate_topologies(&reaction, &model, order).map_err(|e| e.to_string())
}

/// Compute the invariant amplitude for a specific Feynman diagram.
///
/// Takes a `FeynmanGraph` (from a prior `generate_feynman_diagrams` call)
/// and returns the symbolic `AmplitudeExpression`.
#[tauri::command]
fn derive_amplitude(diagram: FeynmanGraph) -> Result<AmplitudeExpression, String> {
    algebra::generate_amplitude(&diagram).map_err(|e| e.to_string())
}

/// Compute kinematic properties for a set of final-state masses at a given
/// energy.
///
/// Returns a `KinematicsReport` containing threshold information, phase
/// space description, and (for 4 external masses) Mandelstam boundaries.
#[tauri::command]
fn compute_kinematics(
    final_masses: Vec<f64>,
    cms_energy: f64,
    target_mass: Option<f64>,
    external_masses: Option<[f64; 4]>,
) -> Result<KinematicsReport, String> {
    let threshold =
        kinematics::calculate_thresholds(&final_masses, target_mass).map_err(|e| e.to_string())?;

    let is_allowed = kinematics::is_kinematically_allowed(cms_energy, &final_masses);

    let phase_space = if is_allowed {
        Some(
            kinematics::generate_phase_space(cms_energy, &final_masses)
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    // Compute Mandelstam boundaries if 4 external masses and s are provided.
    let mandelstam_boundaries = if let Some(masses) = external_masses {
        let s = cms_energy * cms_energy;
        Some(
            kinematics::compute_mandelstam_boundaries(masses, s).map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    Ok(KinematicsReport {
        threshold,
        is_allowed,
        phase_space,
        mandelstam_boundaries,
    })
}

/// Generate Dalitz plot phase-space points for a 3-body decay.
///
/// Returns a `DalitzPlotData` containing the kinematic boundaries and
/// a vector of $(s_{ab}, s_{bc})$ coordinate pairs.
///
/// # Arguments
/// * `mother_mass` — Mass of the decaying particle (GeV).
/// * `m_a`, `m_b`, `m_c` — Daughter masses (GeV).
/// * `n_points` — Approximate number of phase-space points to generate.
#[tauri::command]
fn compute_dalitz_data(
    mother_mass: f64,
    m_a: f64,
    m_b: f64,
    m_c: f64,
    n_points: usize,
) -> Result<DalitzPlotData, String> {
    kinematics::generate_dalitz_plot_data(mother_mass, m_a, m_b, m_c, n_points)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Export Commands (LaTeX & UFO)
// ---------------------------------------------------------------------------

/// Export a symbolic amplitude as a LaTeX string.
///
/// The frontend passes a `FeynmanGraph`. The kernel derives the amplitude
/// and then calls `to_latex()` to produce the LaTeX representation.
///
/// # Returns
/// The LaTeX string for the amplitude, e.g. `i\mathcal{M} = ...`.
#[tauri::command]
fn export_amplitude_latex(diagram: FeynmanGraph) -> Result<String, String> {
    let amplitude = algebra::generate_amplitude(&diagram).map_err(|e| e.to_string())?;
    Ok(amplitude.to_latex())
}

/// Export the theoretical model in simplified UFO format.
///
/// Returns a `HashMap<String, String>` mapping filenames (e.g.
/// `"particles.py"`, `"parameters.py"`, `"__init__.py"`) to their content.
///
/// # Returns
/// A JSON-serializable map of `{ filename: content }`.
#[tauri::command]
fn export_model_ufo(model: TheoreticalModel) -> Result<HashMap<String, String>, String> {
    Ok(model.to_ufo())
}

// ---------------------------------------------------------------------------
// CAS Derivation Steps
// ---------------------------------------------------------------------------

/// Perform a step-by-step amplitude derivation using the CAS engine.
///
/// Returns an ordered array of `DerivationStep` objects, each showing one
/// algebraic transformation from the initial Feynman rules to the final result.
///
/// # Arguments
/// * `diagram` — A `FeynmanGraph` from a prior `generate_feynman_diagrams` call.
/// * `dim` — Spacetime dimension (e.g., `{ "Fixed": 4 }`).
///
/// # Returns
/// A JSON array of `DerivationStep` objects with `label`, `description`,
/// `expression`, and `latex` fields.
#[tauri::command]
fn derive_amplitude_steps(
    diagram: FeynmanGraph,
    dim: algebra::SpacetimeDimension,
) -> Result<Vec<algebra::DerivationStep>, String> {
    algebra::derive_amplitude_steps(&diagram, dim).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Analysis Pipeline Command
// ---------------------------------------------------------------------------

/// Run a complete analysis pipeline: compile observable/cut scripts,
/// generate Monte Carlo events, fill histograms, and return results.
///
/// This orchestrates the full analysis workflow in a single IPC call:
/// 1. Compiles all Rhai observable and cut scripts.
/// 2. Initialises histograms with the requested binning.
/// 3. Runs the Monte Carlo event loop, applying cuts and filling histograms.
/// 4. Returns serialized histogram data alongside cross-section estimates.
///
/// # Arguments
/// * `config` — Analysis configuration with plot definitions, cuts, CMS energy, etc.
///
/// # Returns
/// An `AnalysisResult` containing filled histogram data and integration diagnostics.
#[tauri::command]
fn run_analysis(config: AnalysisConfig) -> Result<AnalysisResult, String> {
    use spire_kernel::kinematics::RamboGenerator;

    let mut generator = RamboGenerator::new();

    // Use constant |M|² = 1 for phase-space distributions.
    // For realistic matrix elements, the frontend should provide the
    // process definition and the kernel would compute |M|² from diagrams.
    spire_kernel::analysis::run_analysis(&config, |_| 1.0, &mut generator)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Scripting IPC Commands
// ---------------------------------------------------------------------------

/// Validate a user-provided Rhai script for syntax correctness.
///
/// Returns `Ok(())` if the script compiles, or an error message describing
/// the syntax error. This powers the "lint as you type" feature in the
/// Observables & Cuts panel.
#[tauri::command]
fn validate_script(script: String) -> Result<(), String> {
    let engine = spire_kernel::scripting::SpireScriptEngine::new();
    engine.validate_script(&script)
}

/// Compile an observable script and evaluate it on a test event.
///
/// The frontend sends the script source; the backend compiles it, runs it
/// against a synthetic test event (two massless back-to-back particles at
/// 100 GeV CMS), and returns the numeric result. This allows immediate
/// feedback in the UI without running a full integration.
#[tauri::command]
fn test_observable_script(script: String) -> Result<f64, String> {
    let engine = spire_kernel::scripting::SpireScriptEngine::new();
    let obs = engine.compile_observable(&script).map_err(|e| e.to_string())?;

    // Synthetic 2→2 test event.
    use spire_kernel::algebra::SpacetimeVector;
    use spire_kernel::kinematics::PhaseSpacePoint;
    let test_event = PhaseSpacePoint {
        momenta: vec![
            SpacetimeVector::new_4d(50.0, 0.0, 0.0, 50.0),
            SpacetimeVector::new_4d(50.0, 0.0, 0.0, -50.0),
            SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0),
            SpacetimeVector::new_4d(50.0, -30.0, -40.0, 0.0),
        ],
        weight: 1.0,
    };

    use spire_kernel::scripting::Observable;
    let value = obs.evaluate(&test_event);
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Script returned non-finite value".into())
    }
}

/// Compile a kinematic cut script and test it against a synthetic event.
///
/// Returns `true` if the test event passes, `false` otherwise.
#[tauri::command]
fn test_cut_script(script: String) -> Result<bool, String> {
    let engine = spire_kernel::scripting::SpireScriptEngine::new();
    let cut = engine.compile_cut(&script).map_err(|e| e.to_string())?;

    use spire_kernel::algebra::SpacetimeVector;
    use spire_kernel::kinematics::PhaseSpacePoint;
    let test_event = PhaseSpacePoint {
        momenta: vec![
            SpacetimeVector::new_4d(50.0, 0.0, 0.0, 50.0),
            SpacetimeVector::new_4d(50.0, 0.0, 0.0, -50.0),
            SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0),
            SpacetimeVector::new_4d(50.0, -30.0, -40.0, 0.0),
        ],
        weight: 1.0,
    };

    use spire_kernel::scripting::KinematicCut;
    Ok(cut.is_passed(&test_event))
}

// ---------------------------------------------------------------------------
// Application Entry Point
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_theoretical_model,
            validate_and_reconstruct_reaction,
            generate_feynman_diagrams,
            derive_amplitude,
            compute_kinematics,
            compute_dalitz_data,
            export_amplitude_latex,
            export_model_ufo,
            derive_amplitude_steps,
            run_analysis,
            validate_script,
            test_observable_script,
            test_cut_script,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SPIRE desktop application");
}
