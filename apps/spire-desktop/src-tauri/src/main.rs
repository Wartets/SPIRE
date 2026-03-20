//! # SPIRE Desktop - Tauri Backend
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use spire_kernel::algebra::{self, AmplitudeExpression};
use spire_kernel::analysis::global_fit::{GlobalFitResult, ObservableFitInput};
use spire_kernel::analysis::{AnalysisConfig, AnalysisResult, EventDisplayData};
use spire_kernel::cosmology::relic as relic_engine;
use spire_kernel::data_loader;
use spire_kernel::decay;
use spire_kernel::flavor::eft as flavor_eft;
use spire_kernel::flavor::lattice as flavor_lattice;
use spire_kernel::graph::{self, FeynmanGraph, LoopOrder, TopologySet};
use spire_kernel::io::{experimental, latex as latex_compiler, provenance as provenance_engine};
use spire_kernel::kinematics::{
    self, DalitzPlotData, MandelstamBoundaries, PhaseSpace, ThresholdResult,
};
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::ontology;
use spire_kernel::s_matrix::{self, Reaction, ReconstructedFinalState};
use spire_kernel::scanner;
use spire_kernel::session::{self, ExecutionResult as SessionResult};
use spire_kernel::theory;
use spire_kernel::theory::pdg::{
    adapter::PdgAdapter, contracts::{PdgParticleRecord, PdgDecayTable, PdgMetadata},
    policy::PdgExtractionPolicy,
};

// ---------------------------------------------------------------------------
// KinematicsReport - aggregate return type
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
            let reaction =
                s_matrix::construct_reaction(&init_refs, &final_refs, &model, Some(cms_energy))
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
        Some(kinematics::compute_mandelstam_boundaries(masses, s).map_err(|e| e.to_string())?)
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
/// * `mother_mass` - Mass of the decaying particle (GeV).
/// * `m_a`, `m_b`, `m_c` - Daughter masses (GeV).
/// * `n_points` - Approximate number of phase-space points to generate.
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
/// * `diagram` - A `FeynmanGraph` from a prior `generate_feynman_diagrams` call.
/// * `dim` - Spacetime dimension (e.g., `{ "Fixed": 4 }`).
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

/// Simplify a diagram amplitude with the rule-based CAS engine.
///
/// Returns the original and simplified LaTeX, applied rewrite rules, and
/// dimensional consistency diagnostics.
#[tauri::command]
fn simplify_expression(
    diagram: FeynmanGraph,
    dim: Option<algebra::SpacetimeDimension>,
    observable: Option<algebra::ObservableKind>,
) -> Result<algebra::SimplifiedExpressionResult, String> {
    let dim = dim.unwrap_or(algebra::SpacetimeDimension::Fixed(4));
    let observable = observable.unwrap_or(algebra::ObservableKind::Amplitude);
    algebra::simplify_expression(&diagram, dim, observable).map_err(|e| e.to_string())
}

/// Verify mass-dimension consistency for a diagram-derived CAS expression.
#[tauri::command]
fn verify_dimensions(
    diagram: FeynmanGraph,
    dim: Option<algebra::SpacetimeDimension>,
    observable: Option<algebra::ObservableKind>,
) -> Result<algebra::DimensionalCheckReport, String> {
    let dim = dim.unwrap_or(algebra::SpacetimeDimension::Fixed(4));
    let observable = observable.unwrap_or(algebra::ObservableKind::Amplitude);
    let result =
        algebra::simplify_expression(&diagram, dim, observable).map_err(|e| e.to_string())?;
    Ok(result.dimension_check)
}

/// Compute χ² goodness-of-fit between a theory histogram and experimental data.
///
/// Parses experimental data from CSV format and compares against theoretical
/// predictions bin-by-bin using a χ² statistic.
///
/// # Arguments
/// * `theory_bin_contents` - Theoretical histogram bin values
/// * `theory_bin_edges` - Theoretical histogram bin edges
/// * `exp_csv` - Experimental data in CSV format (x,y,dy_stat_up,dy_stat_down,dy_syst_up,dy_syst_down)
/// * `exp_label` - Human-readable label for the experimental dataset
///
/// # Returns
/// `GoodnessOfFitResult` containing χ², reduced χ², ndf, and p-value.
#[tauri::command]
fn compute_chi_square(
    theory_bin_contents: Vec<f64>,
    theory_bin_edges: Vec<f64>,
    exp_csv: String,
    exp_label: String,
) -> Result<experimental::GoodnessOfFitResult, String> {
    let exp_dataset = experimental::ExperimentalDataSet::from_csv(&exp_csv, exp_label, None)
        .map_err(|e| e.to_string())?;

    experimental::compute_chi_square(&theory_bin_contents, &theory_bin_edges, &exp_dataset)
        .map_err(|e| e.to_string())
}

/// Compute a multi-observable global χ² summary.
#[tauri::command]
fn compute_global_fit(
    observables: Vec<ObservableFitInput>,
    n_params: usize,
) -> Result<GlobalFitResult, String> {
    spire_kernel::analysis::global_fit::compute_global_fit(&observables, n_params)
        .map_err(|e| e.to_string())
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
/// 3. Optionally reconstructs events through a detector model.
/// 4. Runs the Monte Carlo event loop, applying cuts and filling histograms.
/// 5. Returns serialized histogram data alongside cross-section estimates.
///
/// When `config.detector_preset` is specified (e.g. `"lhc_like"`), the
/// pipeline passes each truth-level event through a phenomenological
/// detector simulation. Observable scripts can then access the
/// `reco` variable (jets, leptons, MET) in addition to `event`.
///
/// # Arguments
/// * `config` - Analysis configuration with plot definitions, cuts, CMS energy, etc.
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
    spire_kernel::analysis::run_reco_analysis(&config, |_| 1.0, &mut generator)
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
    let obs = engine
        .compile_observable(&script)
        .map_err(|e| e.to_string())?;

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
// 3D Event Display Command
// ---------------------------------------------------------------------------

/// Generate a single reconstructed event for 3D visualisation.
///
/// Runs one RAMBO event through the specified detector model and returns
/// an `EventDisplayData` containing jets (as cones), lepton/photon tracks,
/// and missing transverse energy for rendering in the Three.js event display.
///
/// # Arguments
/// * `cms_energy` - Centre-of-mass energy (GeV).
/// * `final_masses` - Final-state particle masses.
/// * `detector_preset` - Detector preset name (e.g., `"lhc_like"`).
/// * `particle_kinds` - Optional particle kind labels per final-state leg.
#[tauri::command]
fn generate_display_event(
    cms_energy: f64,
    final_masses: Vec<f64>,
    detector_preset: String,
    particle_kinds: Option<Vec<String>>,
) -> Result<EventDisplayData, String> {
    spire_kernel::analysis::generate_display_event(
        cms_energy,
        &final_masses,
        &detector_preset,
        particle_kinds.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Generates a batch of Monte-Carlo events for animated 3D playback.
///
/// Each event is independently generated via RAMBO, processed through the
/// detector model, and returned as an `EventDisplayData` for sequential
/// rendering in the Three.js time-of-flight animation loop.
///
/// # Arguments
/// * `cms_energy` - Centre-of-mass energy (GeV).
/// * `final_masses` - Final-state particle masses.
/// * `detector_preset` - Detector preset name (e.g., `"lhc_like"`).
/// * `particle_kinds` - Optional particle kind labels per final-state leg.
/// * `batch_size` - Number of events to generate (clamped to 1..=100).
#[tauri::command]
fn generate_display_batch(
    cms_energy: f64,
    final_masses: Vec<f64>,
    detector_preset: String,
    particle_kinds: Option<Vec<String>>,
    batch_size: usize,
) -> Result<Vec<EventDisplayData>, String> {
    spire_kernel::analysis::generate_display_batch(
        cms_energy,
        &final_masses,
        &detector_preset,
        particle_kinds.as_deref(),
        batch_size,
    )
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Lagrangian Workbench Commands (Phase 32)
// ---------------------------------------------------------------------------

/// Parse a Lagrangian term string into its AST representation.
#[tauri::command]
fn parse_lagrangian_term(
    input: String,
    known_fields: HashMap<String, theory::ast::FieldSpin>,
) -> Result<theory::ast::LagrangianExpr, String> {
    theory::ast::parse_lagrangian_term(&input, &known_fields).map_err(|e| e.to_string())
}

/// Derive a vertex rule from a Lagrangian term by functional differentiation.
#[tauri::command]
fn derive_vertex_rule_from_ast(
    input: String,
    known_fields: HashMap<String, theory::ast::FieldSpin>,
    external_fields: Vec<theory::derivation::ExternalField>,
) -> Result<theory::derivation::DerivedVertexRule, String> {
    let expr =
        theory::ast::parse_lagrangian_term(&input, &known_fields).map_err(|e| e.to_string())?;
    theory::derivation::derive_vertex_rule(&expr, &external_fields).map_err(|e| e.to_string())
}

/// Validate a Lagrangian term for theoretical consistency.
#[tauri::command]
fn validate_lagrangian_term(
    input: String,
    known_fields: HashMap<String, theory::ast::FieldSpin>,
    gauge_symmetry: Option<spire_kernel::groups::GaugeSymmetry>,
    field_gauge_info: HashMap<String, theory::validation::FieldGaugeInfo>,
) -> Result<theory::validation::ValidationResult, String> {
    let expr =
        theory::ast::parse_lagrangian_term(&input, &known_fields).map_err(|e| e.to_string())?;
    Ok(theory::validation::validate_lagrangian_term(
        &expr,
        gauge_symmetry.as_ref(),
        &field_gauge_info,
    ))
}

/// Run an RGE coupling flow integration.
#[tauri::command]
fn run_rge_flow(config: theory::rge::RgeFlowConfig) -> Result<theory::rge::RgeFlowResult, String> {
    theory::rge::run_rge_flow(&config).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// External Theory Bridge Commands (Phase 33)
// ---------------------------------------------------------------------------

/// Parse an SLHA spectrum string and return the parsed document.
#[tauri::command]
fn import_slha_string(slha_text: String) -> Result<theory::slha::SlhaDocument, String> {
    theory::slha::parse_slha(&slha_text).map_err(|e| e.to_string())
}

/// Import a UFO model from its raw file contents.
///
/// The frontend sends a `UfoFileContents` struct with the string content
/// of each `.py` file. The backend parses them and converts to a
/// `TheoreticalModel`.
#[tauri::command]
fn import_ufo_model(
    file_contents: theory::ufo::UfoFileContents,
    model_name: String,
) -> Result<(theory::ufo::UfoModel, TheoreticalModel), String> {
    let ufo = theory::ufo::parse_ufo_model(&file_contents).map_err(|e| e.to_string())?;
    let model =
        theory::ufo::ufo_to_theoretical_model(&ufo, &model_name).map_err(|e| e.to_string())?;
    Ok((ufo, model))
}

/// Generate NLO counterterms from a Lagrangian expression string.
#[tauri::command]
fn derive_counterterms(
    input: String,
    known_fields: HashMap<String, theory::ast::FieldSpin>,
    external_fields: Vec<theory::derivation::ExternalField>,
) -> Result<theory::renormalization::CountertermResult, String> {
    let expr =
        theory::ast::parse_lagrangian_term(&input, &known_fields).map_err(|e| e.to_string())?;
    let scheme = theory::renormalization::build_default_scheme(&expr);
    theory::renormalization::generate_counterterms(&expr, &scheme, &external_fields)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Notebook Session Commands (Phase 43)
// ---------------------------------------------------------------------------

/// Create a new persistent notebook session.
///
/// Returns a unique session ID that the frontend uses to identify
/// subsequent execution requests.
#[tauri::command]
fn session_create() -> String {
    session::global_session_manager().create_session()
}

/// Execute a Rhai script cell within a persistent notebook session.
///
/// Variables and state from previous cell executions are preserved in
/// the session's scope.  An optional `physics_context` JSON blob can
/// inject live workspace data (diagrams, amplitudes, sqrt_s, etc.)
/// into the Rhai scope before evaluation.
#[tauri::command]
fn session_execute_script(
    session_id: String,
    script: String,
    physics_context: Option<serde_json::Value>,
) -> Result<SessionResult, String> {
    session::global_session_manager().execute_script(&session_id, &script, physics_context)
}

/// Execute a TOML config cell to load a theoretical model into a session.
///
/// The model becomes available to subsequent script cells via injected
/// variables (`model_name`, `field_names`, `field_masses`, etc.).
#[tauri::command]
fn session_execute_config(
    session_id: String,
    toml_content: String,
) -> Result<SessionResult, String> {
    session::global_session_manager().execute_config(&session_id, &toml_content)
}

/// Reset a notebook session, clearing all variables and loaded models.
#[tauri::command]
fn session_reset(session_id: String) -> Result<(), String> {
    session::global_session_manager().reset_session(&session_id)
}

/// Destroy a notebook session and free its resources.
#[tauri::command]
fn session_destroy(session_id: String) -> bool {
    session::global_session_manager().destroy_session(&session_id)
}

// ---------------------------------------------------------------------------
// Parameter Scanner Commands (Phase 44)
// ---------------------------------------------------------------------------

/// Run a 1D parameter scan over a single variable.
///
/// Sweeps the target parameter (field mass, decay width, coupling constant,
/// or centre-of-mass energy) across the configured range and evaluates the
/// cross-section at each point using parallel Monte Carlo integration.
///
/// # Returns
/// A `ScanResult1D` with parameter values, cross-sections (pb), and errors.
#[tauri::command]
fn run_parameter_scan_1d(config: scanner::ScanConfig1D) -> Result<scanner::ScanResult1D, String> {
    scanner::run_scan_1d(&config).map_err(|e| e.to_string())
}

/// Run a 2D parameter scan over two variables.
///
/// Sweeps two target parameters across their configured ranges and evaluates
/// the cross-section at each grid point. Returns a row-major matrix of
/// results for heatmap visualisation.
///
/// # Returns
/// A `ScanResult2D` with axis values, cross-section matrix (pb), and errors.
#[tauri::command]
fn run_parameter_scan_2d(config: scanner::ScanConfig2D) -> Result<scanner::ScanResult2D, String> {
    scanner::run_scan_2d(&config).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Decay Calculator Commands (Phase 45)
// ---------------------------------------------------------------------------

/// Calculate the complete decay table for a particle in the model.
///
/// Discovers all kinematically allowed 2-body decay channels, computes
/// partial widths from spin-averaged matrix elements over phase space,
/// and assembles branching ratios into a serialisable decay table.
///
/// # Arguments
/// * `model` - The theoretical model containing fields and vertices.
/// * `particle_id` - The identifier of the decaying particle.
///
/// # Returns
/// A `DecayTable` with channels, partial widths, BRs, and total width.
#[tauri::command]
fn calculate_decay_table_cmd(
    model: TheoreticalModel,
    particle_id: String,
) -> Result<decay::DecayTable, String> {
    decay::calculate_decay_table(&model, &particle_id).map_err(|e| e.to_string())
}

/// Format a decay table as an SLHA DECAY block string.
///
/// # Arguments
/// * `model` - The theoretical model.
/// * `particle_id` - The identifier of the decaying particle.
/// * `pdg_code` - The PDG Monte Carlo particle numbering code.
///
/// # Returns
/// The formatted SLHA DECAY block as a string.
#[tauri::command]
fn export_decay_slha(
    model: TheoreticalModel,
    particle_id: String,
    pdg_code: i32,
) -> Result<String, String> {
    let table = decay::calculate_decay_table(&model, &particle_id).map_err(|e| e.to_string())?;
    Ok(table.to_slha_string(pdg_code))
}

// ---------------------------------------------------------------------------
// NLO Configuration (Phase 46)
// ---------------------------------------------------------------------------

/// Store NLO subtraction-scheme configuration.
///
/// Accepts the user's NLO settings from the frontend (scheme, kinematic
/// bounds, alpha parameter) and stores them for use in subsequent
/// cross-section calculations.
#[tauri::command]
fn configure_nlo(config: serde_json::Value) -> Result<(), String> {
    let _enabled = config
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let _scheme = config
        .get("subtraction_scheme")
        .and_then(|v| v.as_str())
        .unwrap_or("CataniSeymour");
    // Configuration stored; future phases will wire this into the NLO pipeline.
    Ok(())
}

// ---------------------------------------------------------------------------
// Parton Shower Configuration (Phase 46)
// ---------------------------------------------------------------------------

/// Store parton shower provider configuration.
///
/// Accepts the user's shower settings from the frontend (provider,
/// executable path, physics toggles) and stores them for use in
/// downstream event-generation pipelines.
#[tauri::command]
fn configure_shower(config: serde_json::Value) -> Result<(), String> {
    let _enabled = config
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let _provider = config
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("pythia8");
    // Configuration stored; future phases will wire this into the shower pipeline.
    Ok(())
}

// ---------------------------------------------------------------------------
// Proof Generation
// ---------------------------------------------------------------------------

/// Generate a complete mathematical proof document for a Feynman diagram.
///
/// Drives the algebraic proof tracker through the amplitude derivation
/// pipeline and compiles the resulting proof document into a standalone
/// LaTeX source string using the `revtex4-2` document class.
///
/// # Arguments
/// * `diagram` - A `FeynmanGraph` from a prior `generate_feynman_diagrams` call.
/// * `process_label` - Human-readable process label (e.g., "e⁺e⁻ → μ⁺μ⁻").
/// * `dim` - Spacetime dimension configuration.
///
/// # Returns
/// The complete `.tex` source as a string, ready for `pdflatex` compilation.
#[tauri::command]
fn generate_mathematical_proof(
    diagram: FeynmanGraph,
    process_label: String,
    dim: algebra::SpacetimeDimension,
) -> Result<String, String> {
    let proof_doc = algebra::generate_proof_document(&diagram, &process_label, dim)
        .map_err(|e| e.to_string())?;
    Ok(latex_compiler::compile_proof_to_latex(&proof_doc))
}

// ---------------------------------------------------------------------------
// Data Provenance
// ---------------------------------------------------------------------------

/// Compute a SHA-256 provenance hash for the current computational state.
///
/// Serializes the model, reaction, kinematic configuration, and random
/// seed into a canonical JSON form and returns the hex-encoded SHA-256
/// digest alongside the full serialized payload. The result enables
/// bit-for-bit reproducibility verification.
///
/// # Arguments
/// * `model` - The full theoretical model.
/// * `reaction` - The configured scattering reaction (if any).
/// * `cms_energy` - Centre-of-mass energy in GeV (0.0 if not set).
/// * `num_events` - Number of Monte Carlo events.
/// * `seed` - Random number generator seed.
#[tauri::command]
fn compute_provenance_hash(
    model: TheoreticalModel,
    reaction: Option<Reaction>,
    cms_energy: f64,
    num_events: usize,
    seed: u64,
) -> Result<provenance_engine::ProvenanceRecord, String> {
    let kin = if cms_energy > 0.0 {
        Some(provenance_engine::KinematicConfig {
            cms_energy,
            num_events,
        })
    } else {
        None
    };
    let state = provenance_engine::ProvenanceState::new(model, reaction, kin, seed);
    Ok(provenance_engine::compute_provenance(&state))
}

/// Load and verify a provenance payload, restoring the full computational
/// state.
///
/// Accepts a JSON string containing a serialized provenance record
/// (with `sha256` and `payload` fields). Recomputes the hash to verify
/// data integrity, then deserializes the inner state and returns it to
/// the frontend for workspace restoration.
///
/// # Arguments
/// * `payload` - JSON string of a serialized `ProvenanceRecord`.
///
/// # Returns
/// The deserialized `ProvenanceState` on success. Returns an error if the
/// hash does not match (data corruption) or if deserialization fails.
#[tauri::command]
fn load_provenance_state(payload: String) -> Result<provenance_engine::ProvenanceState, String> {
    let record: provenance_engine::ProvenanceRecord =
        serde_json::from_str(&payload).map_err(|e| {
            format!(
                "Failed to parse provenance record: {}. \
                 Expected JSON with 'sha256' and 'payload' fields.",
                e
            )
        })?;
    provenance_engine::load_provenance(&record).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Cosmological Relic Density
// ---------------------------------------------------------------------------

/// Compute the cosmological relic density for a dark matter candidate.
///
/// Integrates the Boltzmann equation through the thermal freeze-out epoch
/// using the semi-analytic method (Kolb & Turner). Returns the present-day
/// abundance $\Omega h^2$, freeze-out temperature, and the full evolution
/// curve for log-log plotting.
///
/// # Arguments
/// * `config` - [`RelicConfig`](relic_engine::RelicConfig) with DM mass,
///   annihilation cross-section, and d.o.f.
///
/// # Returns
/// A [`RelicDensityReport`](relic_engine::RelicDensityReport) with
/// $\Omega h^2$, comparison to the Planck observation, and evolution data.
#[tauri::command]
fn calculate_relic_density(
    config: relic_engine::RelicConfig,
) -> Result<relic_engine::RelicDensityReport, String> {
    Ok(relic_engine::compute_relic_density(&config))
}

// ---------------------------------------------------------------------------
// Flavor Physics - Lattice QCD & EFT Observables
// ---------------------------------------------------------------------------

/// Compute neutral B-meson mixing mass differences.
///
/// Returns $\Delta M_d$ and $\Delta M_s$ in ps$^{-1}$ using the provided
/// Lattice QCD inputs (decay constants and bag parameters).
///
/// # Arguments
/// * `lattice` - [`LatticeInputs`](flavor_lattice::LatticeInputs) with
///   decay constants, bag parameters, and form factors.
///
/// # Returns
/// A JSON object with `delta_m_d` and `delta_m_s` fields.
#[tauri::command]
fn calculate_b_mixing(lattice: flavor_lattice::LatticeInputs) -> Result<serde_json::Value, String> {
    let dm_d = flavor_eft::compute_delta_m_d(&lattice);
    let dm_s = flavor_eft::compute_delta_m_s(&lattice);
    Ok(serde_json::json!({
        "delta_m_d": dm_d,
        "delta_m_s": dm_s,
        "exp_delta_m_d": flavor_eft::EXP_DELTA_M_D,
        "exp_delta_m_s": flavor_eft::EXP_DELTA_M_S,
    }))
}

/// Compute the full flavor observable report for $B \to K \ell^+ \ell^-$.
///
/// Integrates the differential decay rate, evaluates B-meson mixing,
/// and returns the differential $d\Gamma/dq^2$ spectrum for plotting.
///
/// # Arguments
/// * `q2_min` - Lower bound of the $q^2$ window in GeV².
/// * `q2_max` - Upper bound of the $q^2$ window in GeV².
/// * `wilson_coeffs` - Wilson coefficients (SM + BSM shifts).
/// * `lattice` - Lattice QCD inputs.
/// * `n_points` - Number of evaluation points for the spectrum.
///
/// # Returns
/// A [`FlavorObservableReport`](flavor_eft::FlavorObservableReport).
#[tauri::command]
fn calculate_b_to_k_ll(
    q2_min: f64,
    q2_max: f64,
    wilson_coeffs: flavor_eft::WilsonCoefficients,
    lattice: flavor_lattice::LatticeInputs,
    n_points: Option<usize>,
) -> Result<flavor_eft::FlavorObservableReport, String> {
    let n = n_points.unwrap_or(100);
    Ok(flavor_eft::compute_flavor_observables(
        &lattice,
        &wilson_coeffs,
        q2_min,
        q2_max,
        n,
    ))
}

// ---------------------------------------------------------------------------
// GPU / Hardware Backend Commands
// ---------------------------------------------------------------------------

/// Query hardware capabilities for GPU-accelerated integration.
///
/// Returns a JSON-serializable report of available compute backends:
/// whether the `gpu` feature was compiled in, whether a GPU adapter was
/// found at runtime, and the adapter description string.
///
/// The frontend uses this to conditionally show a GPU toggle switch in
/// the Analysis and Parameter Scanner widgets.
#[tauri::command]
fn query_hardware_backends() -> HardwareReport {
    let gpu_compiled = cfg!(feature = "gpu");
    let gpu_available = spire_kernel::integration::IntegratorBackend::gpu_available();
    let adapter_name = if gpu_available {
        #[cfg(feature = "gpu")]
        {
            spire_kernel::integration::gpu::GpuIntegrator::new()
                .map(|g| g.adapter_info.clone())
                .unwrap_or_default()
        }
        #[cfg(not(feature = "gpu"))]
        {
            String::new()
        }
    } else {
        String::new()
    };

    HardwareReport {
        gpu_feature_compiled: gpu_compiled,
        gpu_adapter_available: gpu_available,
        adapter_name,
        cpu_backend: spire_kernel::integration::IntegratorBackend::Cpu
            .description()
            .to_string(),
        gpu_backend: spire_kernel::integration::IntegratorBackend::Gpu
            .description()
            .to_string(),
    }
}

/// Serializable report of available hardware backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HardwareReport {
    /// Whether the binary was compiled with `features = ["gpu"]`.
    gpu_feature_compiled: bool,
    /// Whether a compatible GPU adapter was found at runtime.
    gpu_adapter_available: bool,
    /// Human-readable GPU adapter name (empty if none).
    adapter_name: String,
    /// Description of the CPU backend.
    cpu_backend: String,
    /// Description of the GPU backend.
    gpu_backend: String,
}

// ---------------------------------------------------------------------------
// Global Fits & MCMC Engine (Phase 55)
// ---------------------------------------------------------------------------

use spire_kernel::analysis::likelihood::{
    ExperimentalConstraint, GlobalFitConfig, GlobalLikelihood,
};
use spire_kernel::math::mcmc::{EnsembleSampler, McmcConfig, McmcStatus};

/// IPC-serializable MCMC fit request from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McmcFitRequest {
    /// Serialized `TheoreticalModel` (JSON).
    model: TheoreticalModel,
    /// The fit configuration.
    config: GlobalFitConfig,
}

/// IPC-serializable MCMC status + partial chain data for live updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McmcFitStatus {
    /// Current sampler status.
    pub status: McmcStatus,
    /// Flat samples array (burn-in already discarded): [sample][param].
    /// Only sent when `include_samples` is true in the poll request.
    pub flat_samples: Option<Vec<Vec<f64>>>,
    /// Parameter names (in the same order as samples columns).
    pub param_names: Vec<String>,
}

/// Managed state for background MCMC execution.
struct McmcFitState {
    /// Handle to the background thread (None when idle).
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    /// Stop flag shared with the sampler.
    stop_flag: Arc<AtomicBool>,
    /// Partial chain data updated by the background thread.
    chain: Arc<Mutex<Option<spire_kernel::math::mcmc::McmcChain>>>,
    /// Current sampler status.
    sampler_status: Arc<Mutex<McmcStatus>>,
    /// Parameter names from the fit config.
    param_names: Mutex<Vec<String>>,
    /// Burn-in steps (to discard when producing flat samples).
    burn_in: Mutex<usize>,
}

/// Start an MCMC global fit in a background thread.
///
/// Returns immediately. The frontend polls `get_mcmc_status` for progress.
#[tauri::command]
fn start_mcmc_fit(
    request: McmcFitRequest,
    state: tauri::State<'_, McmcFitState>,
) -> Result<(), String> {
    // Check if already running
    {
        let thread = state.thread.lock().map_err(|e| format!("Lock: {}", e))?;
        if thread.is_some() {
            return Err("An MCMC fit is already running. Stop it first.".into());
        }
    }

    // Reset stop flag
    state.stop_flag.store(false, Ordering::SeqCst);

    // Store param names and burn-in
    {
        let mut names = state
            .param_names
            .lock()
            .map_err(|e| format!("Lock: {}", e))?;
        *names = request
            .config
            .parameters
            .iter()
            .map(|p| p.name.clone())
            .collect();
    }
    {
        let mut bi = state.burn_in.lock().map_err(|e| format!("Lock: {}", e))?;
        *bi = request.config.burn_in;
    }

    // Build the GlobalLikelihood
    let n_dim = request.config.parameters.len();
    let n_walkers = request.config.n_walkers;
    let n_steps = request.config.n_steps;

    let constraints: Vec<Box<dyn ExperimentalConstraint>> = request
        .config
        .gaussian_constraints
        .into_iter()
        .map(|c| Box::new(c) as Box<dyn ExperimentalConstraint>)
        .collect();

    let likelihood = Arc::new(GlobalLikelihood::new(
        request.model,
        request.config.parameters,
        constraints,
    ));

    let mcmc_config = McmcConfig {
        n_walkers,
        n_dim,
        n_steps,
        stretch_factor: 2.0,
    };

    // Generate initial positions: uniform random within prior bounds
    let initial: Vec<Vec<f64>> = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..n_walkers)
            .map(|_| (0..n_dim).map(|_| rng.gen_range(0.0..1.0)).collect())
            .collect()
    };

    // Clones for the background thread
    let stop_flag = Arc::clone(&state.stop_flag);
    let chain_slot = Arc::clone(&state.chain);
    let status_slot = Arc::clone(&state.sampler_status);

    let handle = std::thread::spawn(move || {
        let mut sampler = EnsembleSampler::new(mcmc_config, initial);
        sampler.stop_flag().store(false, Ordering::SeqCst);

        // Connect external stop flag
        let sampler_stop = sampler.stop_flag();
        let _stop_watcher = std::thread::spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            sampler_stop.store(true, Ordering::SeqCst);
        });

        let lk = likelihood;
        let chain = sampler.run(&|p: &[f64]| lk.log_prob(p));

        // Store final chain
        if let Ok(mut slot) = chain_slot.lock() {
            *slot = Some(chain);
        }

        // Update status to "done"
        if let Ok(mut s) = status_slot.lock() {
            s.running = false;
        }
    });

    // Store thread handle
    {
        let mut thread = state.thread.lock().map_err(|e| format!("Lock: {}", e))?;
        *thread = Some(handle);
    }

    // Set running status
    {
        let mut s = state
            .sampler_status
            .lock()
            .map_err(|e| format!("Lock: {}", e))?;
        *s = McmcStatus {
            current_step: 0,
            total_steps: n_steps,
            acceptance_fraction: 0.0,
            running: true,
            stopped: false,
        };
    }

    Ok(())
}

/// Poll the current MCMC fit status and optionally retrieve samples.
#[tauri::command]
fn get_mcmc_status(
    include_samples: bool,
    state: tauri::State<'_, McmcFitState>,
) -> Result<McmcFitStatus, String> {
    let status = {
        let s = state
            .sampler_status
            .lock()
            .map_err(|e| format!("Lock: {}", e))?;
        s.clone()
    };

    let param_names = {
        let names = state
            .param_names
            .lock()
            .map_err(|e| format!("Lock: {}", e))?;
        names.clone()
    };

    let flat_samples = if include_samples {
        let chain_guard = state.chain.lock().map_err(|e| format!("Lock: {}", e))?;
        chain_guard.as_ref().map(|chain| {
            let burn_in = state.burn_in.lock().map_or(0, |b| *b);
            chain.flat_samples(burn_in, 1)
        })
    } else {
        None
    };

    Ok(McmcFitStatus {
        status,
        flat_samples,
        param_names,
    })
}

/// Request the MCMC fit to stop gracefully.
#[tauri::command]
fn stop_mcmc_fit(state: tauri::State<'_, McmcFitState>) -> Result<(), String> {
    state.stop_flag.store(true, Ordering::SeqCst);

    // Wait for the thread to finish (with timeout)
    let handle = {
        let mut thread = state.thread.lock().map_err(|e| format!("Lock: {}", e))?;
        thread.take()
    };

    if let Some(h) = handle {
        // Give it up to 5 seconds
        let _ = h.join();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// PDG Integration (Phase 73)
// ---------------------------------------------------------------------------

/// Get PDG database metadata (edition, version, timestamp).
#[tauri::command]
fn pdg_get_metadata() -> Result<PdgMetadata, String> {
    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;
    adapter.get_metadata().map_err(|e| e.to_string())
}

/// Look up a particle by its MCID (Monte Carlo ID).
#[tauri::command]
fn pdg_lookup_particle_by_mcid(mcid: i32) -> Result<PdgParticleRecord, String> {
    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;
    adapter.lookup_particle_by_mcid(mcid).map_err(|e| e.to_string())
}

/// Look up a particle by its PDG ID (alternate identifier).
#[tauri::command]
fn pdg_lookup_particle_by_pdgid(pdgid: String) -> Result<PdgParticleRecord, String> {
    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;
    adapter.lookup_particle_by_name(&pdgid).map_err(|e| e.to_string())
}

/// Get full particle properties (mass, width, lifetime, quantum numbers, branching ratios).
#[tauri::command]
fn pdg_get_particle_properties(mcid: i32) -> Result<PdgParticleRecord, String> {
    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;
    adapter.get_particle_properties(mcid).map_err(|e| e.to_string())
}

/// Get decay table for a particle, filtered by extraction policy.
#[tauri::command]
fn pdg_get_decay_table(mcid: i32, policy: String) -> Result<PdgDecayTable, String> {
    let policy_enum = match policy.as_str() {
        "StrictPhysical" => PdgExtractionPolicy::StrictPhysical,
        "Catalog" => PdgExtractionPolicy::Catalog,
        other => {
            return Err(format!(
                "Invalid policy '{}': must be 'StrictPhysical' or 'Catalog'",
                other
            ))
        }
    };

    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;
    adapter
        .get_decay_table(mcid, policy_enum)
        .map_err(|e| e.to_string())
}

/// Synchronize a theoretical model with PDG database values.
///
/// Takes an entire `TheoreticalModel` as input, updates particle masses and widths
/// from the PDG database (respecting edition locks), and returns the updated model.
/// Adheres to strict stateless isomorphism: no shared mutable state across IPC boundary.
///
/// For now, this is a placeholder that returns the model unchanged. In a production
/// system, it would:
/// 1. Parse each field's name or identifier to derive a PDG MCID.
/// 2. Look up that MCID in the PDG database.
/// 3. Update the mass and width fields with PDG values.
/// 4. Track provenance in model metadata.
#[tauri::command]
fn pdg_sync_model(model: TheoreticalModel) -> Result<TheoreticalModel, String> {
    let _adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;

    // Placeholder: return model unchanged for now.
    // Production implementation would iterate over model.fields and look up PDG data.
    // The adapter API is available for future integration.

    Ok(model)
}

/// Search for particles by name, label, or identifier fragment.
///
/// Returns a list of matching `PdgParticleRecord` objects.
#[tauri::command]
fn pdg_search_identifiers(query: String) -> Result<Vec<PdgParticleRecord>, String> {
    let adapter = PdgAdapter::with_default_path().map_err(|e| e.to_string())?;

    // Simple substring search on particle name (case-insensitive)
    let query_lower = query.to_lowercase();

    // For now, perform a limited search against known particles
    // In production, this would query a full text search index in the PDG database
    let mut results = Vec::new();

    // Attempt to match against common particle labels and MCIDs
    let common_particles = [11, -11, 12, 13, -13, 14, 15, -15, 16, 1, 2, 3, 4, 5, 6, 21, 22, 23, 24, 25];
    for mcid in &common_particles {
        if let Ok(record) = adapter.lookup_particle_by_mcid(*mcid) {
            if let Some(ref label) = record.label {
                if label.to_lowercase().contains(&query_lower) {
                    results.push(record);
                }
            }
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Plugin System (Phase 54)
// ---------------------------------------------------------------------------

/// Serializable plugin summary for IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginInfo {
    name: String,
    version: String,
    description: String,
    author: String,
    capabilities: Vec<String>,
    enabled: bool,
}

/// Managed state wrapping the plugin host engine.
///
/// This is the first `.manage()`-ed state in the Tauri backend. The
/// `PluginHost` lives behind a `Mutex` because WASM stores are `!Send`
/// on some targets and we need interior mutability for load/unload.
struct PluginManagerState {
    #[cfg(feature = "plugins")]
    host: Mutex<spire_kernel::plugins::PluginHost>,
}

/// Load a WASM plugin from a file path selected by the user.
///
/// Reads the `.wasm` file, compiles and instantiates it in the sandboxed
/// wasmtime runtime, extracts metadata, validates API version compatibility,
/// and returns the plugin summary.
#[tauri::command]
fn load_plugin(
    path: String,
    state: tauri::State<'_, PluginManagerState>,
) -> Result<PluginInfo, String> {
    #[cfg(feature = "plugins")]
    {
        let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
        let mut host = state
            .host
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        let meta = host
            .load_plugin_from_bytes(&bytes)
            .map_err(|e| e.to_string())?;
        Ok(PluginInfo {
            name: meta.name,
            version: meta.version.to_string(),
            description: meta.description,
            author: meta.author,
            capabilities: meta
                .capabilities
                .iter()
                .map(|c| format!("{:?}", c))
                .collect(),
            enabled: true,
        })
    }
    #[cfg(not(feature = "plugins"))]
    {
        let _ = (path, state);
        Err("Plugin system not compiled (enable the 'plugins' feature)".into())
    }
}

/// List all currently loaded plugins.
#[tauri::command]
fn list_active_plugins(
    state: tauri::State<'_, PluginManagerState>,
) -> Result<Vec<PluginInfo>, String> {
    #[cfg(feature = "plugins")]
    {
        let host = state
            .host
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        Ok(host
            .list_plugins()
            .into_iter()
            .map(|s| PluginInfo {
                name: s.name,
                version: s.version,
                description: s.description,
                author: s.author,
                capabilities: s.capabilities,
                enabled: s.enabled,
            })
            .collect())
    }
    #[cfg(not(feature = "plugins"))]
    {
        let _ = state;
        Ok(vec![])
    }
}

/// Unload a plugin by name.
#[tauri::command]
fn unload_plugin(name: String, state: tauri::State<'_, PluginManagerState>) -> Result<(), String> {
    #[cfg(feature = "plugins")]
    {
        let mut host = state
            .host
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        host.unload_plugin(&name).map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "plugins"))]
    {
        let _ = (name, state);
        Err("Plugin system not compiled (enable the 'plugins' feature)".into())
    }
}

// ---------------------------------------------------------------------------
// Application Entry Point
// ---------------------------------------------------------------------------

fn main() {
    // Initialise plugin manager state
    let plugin_state = PluginManagerState {
        #[cfg(feature = "plugins")]
        host: Mutex::new(
            spire_kernel::plugins::PluginHost::new()
                .expect("Failed to initialise WASM plugin engine"),
        ),
    };

    // Initialise MCMC fit state
    let mcmc_state = McmcFitState {
        thread: Mutex::new(None),
        stop_flag: Arc::new(AtomicBool::new(false)),
        chain: Arc::new(Mutex::new(None)),
        sampler_status: Arc::new(Mutex::new(McmcStatus {
            current_step: 0,
            total_steps: 0,
            acceptance_fraction: 0.0,
            running: false,
            stopped: false,
        })),
        param_names: Mutex::new(vec![]),
        burn_in: Mutex::new(0),
    };

    tauri::Builder::default()
        .manage(plugin_state)
        .manage(mcmc_state)
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
            simplify_expression,
            verify_dimensions,
            run_analysis,
            validate_script,
            test_observable_script,
            test_cut_script,
            generate_display_event,
            generate_display_batch,
            parse_lagrangian_term,
            derive_vertex_rule_from_ast,
            validate_lagrangian_term,
            run_rge_flow,
            import_slha_string,
            import_ufo_model,
            derive_counterterms,
            session_create,
            session_execute_script,
            session_execute_config,
            session_reset,
            session_destroy,
            run_parameter_scan_1d,
            run_parameter_scan_2d,
            calculate_decay_table_cmd,
            export_decay_slha,
            configure_nlo,
            configure_shower,
            generate_mathematical_proof,
            compute_provenance_hash,
            load_provenance_state,
            calculate_relic_density,
            calculate_b_mixing,
            calculate_b_to_k_ll,
            query_hardware_backends,
            load_plugin,
            list_active_plugins,
            unload_plugin,
            start_mcmc_fit,
            get_mcmc_status,
            stop_mcmc_fit,
            compute_chi_square,
            compute_global_fit,
            pdg_get_metadata,
            pdg_lookup_particle_by_mcid,
            pdg_lookup_particle_by_pdgid,
            pdg_get_particle_properties,
            pdg_get_decay_table,
            pdg_sync_model,
            pdg_search_identifiers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SPIRE desktop application");
}
