//! # WASM API — WebAssembly Bindings for SPIRE
//!
//! This module provides `#[wasm_bindgen]`-annotated functions that expose the
//! SPIRE kernel to JavaScript/TypeScript consumers running in a browser or
//! Tauri's webview.
//!
//! ## Serialization Strategy
//!
//! All data crosses the WASM boundary via `serde_wasm_bindgen`:
//! - **Input**: `JsValue` → `serde_wasm_bindgen::from_value` → Rust struct
//! - **Output**: Rust struct → `serde_wasm_bindgen::to_value` → `JsValue`
//!
//! This produces native JS objects (not JSON strings), matching the TypeScript
//! interfaces defined in `lib/types/spire.ts`.
//!
//! ## Error Handling
//!
//! Kernel errors (`SpireError`) are stringified via `.map_err(|e| e.to_string())`
//! and converted to `JsValue` so that the JS caller receives a rejected Promise
//! with a human-readable error message.

use wasm_bindgen::prelude::*;

use spire_kernel::algebra::{self, AmplitudeExpression};
use spire_kernel::data_loader;
use spire_kernel::graph::{self, FeynmanGraph, LoopOrder};
use spire_kernel::kinematics::{self, MandelstamBoundaries, PhaseSpace, ThresholdResult};
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::ontology;
use spire_kernel::s_matrix::{self, Reaction, ReconstructedFinalState};

// ---------------------------------------------------------------------------
// KinematicsReport (mirrors the Tauri-side struct)
// ---------------------------------------------------------------------------

/// Aggregate kinematics result for the WASM boundary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KinematicsReport {
    pub threshold: ThresholdResult,
    pub is_allowed: bool,
    pub phase_space: Option<PhaseSpace>,
    pub mandelstam_boundaries: Option<MandelstamBoundaries>,
}

// ---------------------------------------------------------------------------
// Helper: Convert a stringified error into a JsValue.
// ---------------------------------------------------------------------------

fn to_js_err(msg: String) -> JsValue {
    JsValue::from_str(&msg)
}

// ---------------------------------------------------------------------------
// Public WASM Functions
// ---------------------------------------------------------------------------

/// Load a theoretical model from TOML strings.
///
/// # Arguments
/// * `particles_toml` — Raw contents of `particles.toml`.
/// * `vertices_toml` — Raw contents of `sm_vertices.toml` (or BSM model).
/// * `model_name` — Optional model name (defaults to `"Standard Model"`).
///
/// # Returns
/// A JS object matching the `TheoreticalModel` TypeScript interface.
#[wasm_bindgen(js_name = "loadModel")]
pub fn wasm_load_model(
    particles_toml: &str,
    vertices_toml: &str,
    model_name: Option<String>,
) -> Result<JsValue, JsValue> {
    let name = model_name.unwrap_or_else(|| "Standard Model".into());
    let model =
        data_loader::build_model(particles_toml, vertices_toml, &name).map_err(|e| to_js_err(e.to_string()))?;
    serde_wasm_bindgen::to_value(&model).map_err(|e| to_js_err(e.to_string()))
}

/// Construct and validate a specific reaction.
///
/// # Arguments
/// * `initial_ids` — JS array of initial-state particle ID strings.
/// * `final_ids` — JS array of final-state particle ID strings.
/// * `cms_energy` — Centre-of-mass energy in GeV.
/// * `model` — The `TheoreticalModel` JS object.
///
/// # Returns
/// A JS object matching the `Reaction` TypeScript interface.
#[wasm_bindgen(js_name = "constructReaction")]
pub fn wasm_construct_reaction(
    initial_ids: JsValue,
    final_ids: JsValue,
    cms_energy: f64,
    model: JsValue,
) -> Result<JsValue, JsValue> {
    let init: Vec<String> =
        serde_wasm_bindgen::from_value(initial_ids).map_err(|e| to_js_err(e.to_string()))?;
    let finals: Vec<String> =
        serde_wasm_bindgen::from_value(final_ids).map_err(|e| to_js_err(e.to_string()))?;
    let the_model: TheoreticalModel =
        serde_wasm_bindgen::from_value(model).map_err(|e| to_js_err(e.to_string()))?;

    let init_refs: Vec<&str> = init.iter().map(|s| s.as_str()).collect();
    let final_refs: Vec<&str> = finals.iter().map(|s| s.as_str()).collect();

    let reaction = s_matrix::construct_reaction(&init_refs, &final_refs, &the_model, Some(cms_energy))
        .map_err(|e| to_js_err(e.to_string()))?;

    serde_wasm_bindgen::to_value(&reaction).map_err(|e| to_js_err(e.to_string()))
}

/// Reconstruct all kinematically allowed two-body final states for a given
/// initial state and energy.
///
/// # Arguments
/// * `initial_ids` — JS array of initial-state particle ID strings.
/// * `cms_energy` — Centre-of-mass energy in GeV.
/// * `model` — The `TheoreticalModel` JS object.
///
/// # Returns
/// A JS array of `ReconstructedFinalState` objects.
#[wasm_bindgen(js_name = "reconstructReaction")]
pub fn wasm_reconstruct_reaction(
    initial_ids: JsValue,
    cms_energy: f64,
    model: JsValue,
) -> Result<JsValue, JsValue> {
    let init: Vec<String> =
        serde_wasm_bindgen::from_value(initial_ids).map_err(|e| to_js_err(e.to_string()))?;
    let the_model: TheoreticalModel =
        serde_wasm_bindgen::from_value(model).map_err(|e| to_js_err(e.to_string()))?;

    let particles: Vec<_> = init
        .iter()
        .map(|id| {
            the_model
                .fields
                .iter()
                .find(|f| f.id == *id)
                .ok_or_else(|| to_js_err(format!("Unknown particle: {}", id)))
                .map(|f| ontology::particle_from_field(f.clone()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let results: Vec<ReconstructedFinalState> =
        s_matrix::reconstruct_reaction(&particles, &the_model, cms_energy)
            .map_err(|e| to_js_err(e.to_string()))?;

    serde_wasm_bindgen::to_value(&results).map_err(|e| to_js_err(e.to_string()))
}

/// Generate all topologically distinct Feynman diagrams for a reaction.
///
/// # Arguments
/// * `reaction` — A `Reaction` JS object from a prior construction call.
/// * `model` — The `TheoreticalModel` JS object.
/// * `max_loop_order` — Maximum loop order (0 = tree-level).
///
/// # Returns
/// A JS object matching the `TopologySet` TypeScript interface.
#[wasm_bindgen(js_name = "generateDiagrams")]
pub fn wasm_generate_diagrams(
    reaction: JsValue,
    model: JsValue,
    max_loop_order: u32,
) -> Result<JsValue, JsValue> {
    let the_reaction: Reaction =
        serde_wasm_bindgen::from_value(reaction).map_err(|e| to_js_err(e.to_string()))?;
    let the_model: TheoreticalModel =
        serde_wasm_bindgen::from_value(model).map_err(|e| to_js_err(e.to_string()))?;

    let order = match max_loop_order {
        0 => LoopOrder::Tree,
        1 => LoopOrder::OneLoop,
        2 => LoopOrder::TwoLoop,
        n => LoopOrder::NLoop(n),
    };

    let topologies = graph::generate_topologies(&the_reaction, &the_model, order)
        .map_err(|e| to_js_err(e.to_string()))?;

    serde_wasm_bindgen::to_value(&topologies).map_err(|e| to_js_err(e.to_string()))
}

/// Compute the invariant amplitude for a specific Feynman diagram.
///
/// # Arguments
/// * `diagram` — A `FeynmanGraph` JS object from a prior diagram generation.
///
/// # Returns
/// A JS object matching the `AmplitudeResult` TypeScript interface.
#[wasm_bindgen(js_name = "deriveAmplitude")]
pub fn wasm_derive_amplitude(diagram: JsValue) -> Result<JsValue, JsValue> {
    let the_diagram: FeynmanGraph =
        serde_wasm_bindgen::from_value(diagram).map_err(|e| to_js_err(e.to_string()))?;

    let amplitude: AmplitudeExpression =
        algebra::generate_amplitude(&the_diagram).map_err(|e| to_js_err(e.to_string()))?;

    serde_wasm_bindgen::to_value(&amplitude).map_err(|e| to_js_err(e.to_string()))
}

/// Compute kinematic properties for a set of final-state masses at a given energy.
///
/// # Arguments
/// * `final_masses` — JS array of final-state particle masses in GeV.
/// * `cms_energy` — Available centre-of-mass energy in GeV.
/// * `target_mass` — Optional target mass for fixed-target kinematics.
/// * `external_masses` — Optional 4-element array for Mandelstam boundary computation.
///
/// # Returns
/// A JS object matching the `KinematicsReport` structure.
#[wasm_bindgen(js_name = "computeKinematics")]
pub fn wasm_compute_kinematics(
    final_masses: JsValue,
    cms_energy: f64,
    target_mass: JsValue,
    external_masses: JsValue,
) -> Result<JsValue, JsValue> {
    let masses: Vec<f64> =
        serde_wasm_bindgen::from_value(final_masses).map_err(|e| to_js_err(e.to_string()))?;

    let target: Option<f64> =
        serde_wasm_bindgen::from_value(target_mass).map_err(|e| to_js_err(e.to_string()))?;

    let ext_masses: Option<[f64; 4]> =
        serde_wasm_bindgen::from_value(external_masses).map_err(|e| to_js_err(e.to_string()))?;

    let threshold =
        kinematics::calculate_thresholds(&masses, target).map_err(|e| to_js_err(e.to_string()))?;

    let is_allowed = kinematics::is_kinematically_allowed(cms_energy, &masses);

    let phase_space = if is_allowed {
        Some(
            kinematics::generate_phase_space(cms_energy, &masses)
                .map_err(|e| to_js_err(e.to_string()))?,
        )
    } else {
        None
    };

    let mandelstam_boundaries = if let Some(ext) = ext_masses {
        let s = cms_energy * cms_energy;
        Some(
            kinematics::compute_mandelstam_boundaries(ext, s)
                .map_err(|e| to_js_err(e.to_string()))?,
        )
    } else {
        None
    };

    let report = KinematicsReport {
        threshold,
        is_allowed,
        phase_space,
        mandelstam_boundaries,
    };

    serde_wasm_bindgen::to_value(&report).map_err(|e| to_js_err(e.to_string()))
}

/// Generate Dalitz plot phase-space points for a 3-body decay.
///
/// # Arguments
/// * `mother_mass` — Mass of the decaying particle (GeV).
/// * `m_a`, `m_b`, `m_c` — Daughter masses (GeV).
/// * `n_points` — Approximate number of phase-space points.
///
/// # Returns
/// A JS object matching the `DalitzPlotData` TypeScript interface.
#[wasm_bindgen(js_name = "computeDalitzData")]
pub fn wasm_compute_dalitz_data(
    mother_mass: f64,
    m_a: f64,
    m_b: f64,
    m_c: f64,
    n_points: usize,
) -> Result<JsValue, JsValue> {
    let data = kinematics::generate_dalitz_plot_data(mother_mass, m_a, m_b, m_c, n_points)
        .map_err(|e| to_js_err(e.to_string()))?;
    serde_wasm_bindgen::to_value(&data).map_err(|e| to_js_err(e.to_string()))
}

/// Export a symbolic amplitude as a LaTeX string.
///
/// # Arguments
/// * `diagram` — A `FeynmanGraph` JS object.
///
/// # Returns
/// A JS string containing the LaTeX representation (e.g. `i\mathcal{M} = ...`).
#[wasm_bindgen(js_name = "exportAmplitudeLatex")]
pub fn wasm_export_amplitude_latex(diagram: JsValue) -> Result<JsValue, JsValue> {
    let the_diagram: FeynmanGraph =
        serde_wasm_bindgen::from_value(diagram).map_err(|e| to_js_err(e.to_string()))?;

    let amplitude: AmplitudeExpression =
        algebra::generate_amplitude(&the_diagram).map_err(|e| to_js_err(e.to_string()))?;

    Ok(JsValue::from_str(&amplitude.to_latex()))
}

/// Export a theoretical model in simplified UFO format.
///
/// # Arguments
/// * `model` — A `TheoreticalModel` JS object.
///
/// # Returns
/// A JS object mapping filenames to their content:
/// `{ "particles.py": "...", "parameters.py": "...", "__init__.py": "..." }`.
#[wasm_bindgen(js_name = "exportModelUfo")]
pub fn wasm_export_model_ufo(model: JsValue) -> Result<JsValue, JsValue> {
    let the_model: TheoreticalModel =
        serde_wasm_bindgen::from_value(model).map_err(|e| to_js_err(e.to_string()))?;

    let files = the_model.to_ufo();
    serde_wasm_bindgen::to_value(&files).map_err(|e| to_js_err(e.to_string()))
}

// ---------------------------------------------------------------------------
// Phase 16 — CAS Derivation Steps
// ---------------------------------------------------------------------------

/// Perform a step-by-step amplitude derivation using the CAS engine.
///
/// Returns an ordered array of `DerivationStep` objects, each representing
/// one transformation from the initial Feynman rules through simplification
/// to the final result.
///
/// # Arguments
/// * `diagram` — A `FeynmanGraph` JS object.
/// * `dim` — Spacetime dimension specification (e.g., `{ Fixed: 4 }` or `{ DimReg: { base: 4 } }`).
///
/// # Returns
/// A JS array of `DerivationStep` objects.
#[wasm_bindgen(js_name = "deriveAmplitudeSteps")]
pub fn wasm_derive_amplitude_steps(
    diagram: JsValue,
    dim: JsValue,
) -> Result<JsValue, JsValue> {
    let the_diagram: FeynmanGraph =
        serde_wasm_bindgen::from_value(diagram).map_err(|e| to_js_err(e.to_string()))?;
    let spacetime_dim: algebra::SpacetimeDimension =
        serde_wasm_bindgen::from_value(dim).map_err(|e| to_js_err(e.to_string()))?;

    let steps = algebra::derive_amplitude_steps(&the_diagram, spacetime_dim)
        .map_err(|e| to_js_err(e.to_string()))?;

    serde_wasm_bindgen::to_value(&steps).map_err(|e| to_js_err(e.to_string()))
}
