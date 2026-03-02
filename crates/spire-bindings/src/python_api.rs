//! # Python API — PyO3 Bindings for SPIRE
//!
//! This module exposes the SPIRE kernel to Python via PyO3, compiling the
//! kernel into a native Python extension module (`spire`).
//!
//! ## Serialization Strategy
//!
//! Complex nested structures (e.g., `TheoreticalModel`, `Reaction`) are
//! transported as JSON strings. Python callers use `json.loads()` to convert
//! them into native dictionaries. This avoids the need for exhaustive
//! `#[pyclass]` wrappers for every kernel struct while still providing a
//! complete API.
//!
//! Simpler results (validity booleans, counts, expressions) are returned as
//! native Python types via `#[pyo3(get)]` attributes.
//!
//! ## Usage
//!
//! ```python
//! import spire
//!
//! # Load a model from TOML files.
//! model_json = spire.load_model(particles_toml, vertices_toml, "SM")
//!
//! # Construct and validate a reaction.
//! result = spire.construct_reaction(["e-", "e+"], ["mu-", "mu+"], 10.0, model_json)
//! print(result.is_valid, result.json)
//!
//! # Generate diagrams.
//! diagrams = spire.generate_diagrams(result.json, model_json, 0)
//! print(diagrams.count)
//!
//! # Compute threshold energy.
//! threshold = spire.calculate_threshold([173.0, 173.0])
//! print(threshold)
//! ```
//!
//! Gated behind the `python` cargo feature to avoid pulling in PyO3
//! dependencies when building for WASM or desktop-only targets.

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

use spire_kernel::algebra;
use spire_kernel::data_loader;
use spire_kernel::graph::{self, LoopOrder};
use spire_kernel::kinematics;
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::ontology;
use spire_kernel::s_matrix;

// ---------------------------------------------------------------------------
// PyO3 Class Wrappers
// ---------------------------------------------------------------------------

/// Python-visible wrapper around a `Reaction` result.
///
/// Exposes methods for querying validity, retrieving diagnostics, and
/// accessing the full reaction data as JSON.
#[pyclass(name = "ReactionResult")]
#[derive(Debug, Clone)]
pub struct PyReactionResult {
    /// Whether the reaction is valid.
    #[pyo3(get)]
    pub is_valid: bool,
    /// Violation diagnostics (empty list if valid).
    #[pyo3(get)]
    pub violations: Vec<String>,
    /// Interaction types as string labels.
    #[pyo3(get)]
    pub interaction_types: Vec<String>,
    /// Serialized JSON representation of the full reaction.
    #[pyo3(get)]
    pub json: String,
}

#[pymethods]
impl PyReactionResult {
    /// Return a Python dictionary representation of the reaction.
    ///
    /// Uses `json.loads()` internally via returning the JSON string.
    /// The caller can do `import json; d = json.loads(result.json)`.
    fn to_dict(&self) -> PyResult<String> {
        Ok(self.json.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "ReactionResult(is_valid={}, interactions={:?}, violations={:?})",
            self.is_valid, self.interaction_types, self.violations
        )
    }
}

/// Python-visible wrapper around a diagram topology set.
#[pyclass(name = "DiagramSet")]
#[derive(Debug, Clone)]
pub struct PyDiagramSet {
    /// Number of diagrams generated.
    #[pyo3(get)]
    pub count: usize,
    /// Serialized JSON of all diagrams.
    #[pyo3(get)]
    pub json: String,
}

#[pymethods]
impl PyDiagramSet {
    /// Return the JSON string for `json.loads()`.
    fn to_dict(&self) -> PyResult<String> {
        Ok(self.json.clone())
    }

    fn __repr__(&self) -> String {
        format!("DiagramSet(count={})", self.count)
    }
}

/// Python-visible wrapper around an amplitude expression.
#[pyclass(name = "AmplitudeResult")]
#[derive(Debug, Clone)]
pub struct PyAmplitudeResult {
    /// Symbolic expression string for the amplitude.
    #[pyo3(get)]
    pub expression: String,
    /// Coupling constants in the amplitude.
    #[pyo3(get)]
    pub couplings: Vec<String>,
    /// Momentum labels in the amplitude.
    #[pyo3(get)]
    pub momenta_labels: Vec<String>,
    /// Full JSON of the AmplitudeExpression.
    #[pyo3(get)]
    pub json: String,
}

#[pymethods]
impl PyAmplitudeResult {
    /// Return the JSON string for `json.loads()`.
    fn to_dict(&self) -> PyResult<String> {
        Ok(self.json.clone())
    }

    fn __repr__(&self) -> String {
        format!("AmplitudeResult(expression='{}')", self.expression)
    }
}

/// Python-visible wrapper for kinematics results.
#[pyclass(name = "KinematicsResult")]
#[derive(Debug, Clone)]
pub struct PyKinematicsResult {
    /// Minimum CM energy required (GeV).
    #[pyo3(get)]
    pub threshold_energy: f64,
    /// Lab-frame beam energy for fixed-target (None if not applicable).
    #[pyo3(get)]
    pub lab_energy: Option<f64>,
    /// Full JSON for detailed results.
    #[pyo3(get)]
    pub json: String,
}

#[pymethods]
impl PyKinematicsResult {
    fn to_dict(&self) -> PyResult<String> {
        Ok(self.json.clone())
    }

    fn __repr__(&self) -> String {
        format!(
            "KinematicsResult(threshold={:.4} GeV, lab_energy={:?})",
            self.threshold_energy, self.lab_energy
        )
    }
}

// ---------------------------------------------------------------------------
// PyO3 Module Functions
// ---------------------------------------------------------------------------

/// The top-level Python module `spire`.
///
/// Registers all Python-visible functions and classes.
#[pymodule]
fn spire(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyReactionResult>()?;
    m.add_class::<PyDiagramSet>()?;
    m.add_class::<PyAmplitudeResult>()?;
    m.add_class::<PyKinematicsResult>()?;
    m.add_function(wrap_pyfunction!(load_model, m)?)?;
    m.add_function(wrap_pyfunction!(construct_reaction, m)?)?;
    m.add_function(wrap_pyfunction!(reconstruct_reaction, m)?)?;
    m.add_function(wrap_pyfunction!(generate_diagrams, m)?)?;
    m.add_function(wrap_pyfunction!(compute_amplitude, m)?)?;
    m.add_function(wrap_pyfunction!(calculate_threshold, m)?)?;
    m.add_function(wrap_pyfunction!(is_kinematically_allowed, m)?)?;
    Ok(())
}

/// Load a theoretical model from TOML file contents.
///
/// # Arguments
/// * `particles_toml` — Contents of the particles definition file.
/// * `vertices_toml` — Contents of the vertices/interactions definition file.
/// * `model_name` — Name for the model (e.g., `"Standard Model"`).
///
/// # Returns
/// A JSON string of the complete `TheoreticalModel`. Use `json.loads()` to
/// convert to a Python dictionary.
#[pyfunction]
#[pyo3(signature = (particles_toml, vertices_toml, model_name="Standard Model"))]
fn load_model(particles_toml: &str, vertices_toml: &str, model_name: &str) -> PyResult<String> {
    let model = data_loader::build_model(particles_toml, vertices_toml, model_name)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    serde_json::to_string(&model).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Construct and validate a specific reaction.
///
/// # Arguments
/// * `initial` — List of initial-state particle ID strings.
/// * `final_state` — List of final-state particle ID strings.
/// * `cms_energy` — Centre-of-mass energy in GeV.
/// * `model_json` — JSON string of the TheoreticalModel.
///
/// # Returns
/// A `ReactionResult` with validity, diagnostics, and full JSON.
#[pyfunction]
#[pyo3(signature = (initial, final_state, cms_energy, model_json))]
fn construct_reaction(
    initial: Vec<String>,
    final_state: Vec<String>,
    cms_energy: f64,
    model_json: String,
) -> PyResult<PyReactionResult> {
    let model: TheoreticalModel =
        serde_json::from_str(&model_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let init_refs: Vec<&str> = initial.iter().map(|s| s.as_str()).collect();
    let final_refs: Vec<&str> = final_state.iter().map(|s| s.as_str()).collect();

    let reaction =
        s_matrix::construct_reaction(&init_refs, &final_refs, &model, Some(cms_energy))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json = serde_json::to_string(&reaction).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyReactionResult {
        is_valid: reaction.is_valid,
        violations: reaction.violation_diagnostics.clone(),
        interaction_types: reaction
            .interaction_types
            .iter()
            .map(|it| format!("{:?}", it))
            .collect(),
        json,
    })
}

/// Reconstruct all kinematically allowed two-body final states.
///
/// # Arguments
/// * `initial_ids` — List of initial-state particle ID strings.
/// * `cms_energy` — Centre-of-mass energy in GeV.
/// * `model_json` — JSON string of the TheoreticalModel.
///
/// # Returns
/// A JSON string containing a list of `ReconstructedFinalState` objects.
#[pyfunction]
#[pyo3(signature = (initial_ids, cms_energy, model_json))]
fn reconstruct_reaction(
    initial_ids: Vec<String>,
    cms_energy: f64,
    model_json: String,
) -> PyResult<String> {
    let model: TheoreticalModel =
        serde_json::from_str(&model_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let particles: Vec<_> = initial_ids
        .iter()
        .map(|id| {
            model
                .fields
                .iter()
                .find(|f| f.id == *id)
                .ok_or_else(|| PyValueError::new_err(format!("Unknown particle: {}", id)))
                .map(|f| ontology::particle_from_field(f.clone()))
        })
        .collect::<PyResult<Vec<_>>>()?;

    let results = s_matrix::reconstruct_reaction(&particles, &model, cms_energy)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    serde_json::to_string(&results).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Generate Feynman diagrams for a reaction (given as JSON).
///
/// # Arguments
/// * `reaction_json` — JSON string of the Reaction.
/// * `model_json` — JSON string of the TheoreticalModel.
/// * `max_loop_order` — Maximum loop order (0 = tree, 1 = one-loop, etc.).
///
/// # Returns
/// A `DiagramSet` with the count and full JSON.
#[pyfunction]
#[pyo3(signature = (reaction_json, model_json, max_loop_order=0))]
fn generate_diagrams(
    reaction_json: String,
    model_json: String,
    max_loop_order: u32,
) -> PyResult<PyDiagramSet> {
    let reaction: s_matrix::Reaction =
        serde_json::from_str(&reaction_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let model: TheoreticalModel =
        serde_json::from_str(&model_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let order = match max_loop_order {
        0 => LoopOrder::Tree,
        1 => LoopOrder::OneLoop,
        2 => LoopOrder::TwoLoop,
        n => LoopOrder::NLoop(n),
    };

    let topologies = graph::generate_topologies(&reaction, &model, order)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json =
        serde_json::to_string(&topologies).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyDiagramSet {
        count: topologies.count,
        json,
    })
}

/// Compute the invariant amplitude for a specific diagram (given as JSON).
///
/// # Arguments
/// * `diagram_json` — JSON string of a `FeynmanGraph`.
///
/// # Returns
/// An `AmplitudeResult` with the expression and full JSON.
#[pyfunction]
fn compute_amplitude(diagram_json: String) -> PyResult<PyAmplitudeResult> {
    let diagram: graph::FeynmanGraph =
        serde_json::from_str(&diagram_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let amplitude = algebra::generate_amplitude(&diagram)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json =
        serde_json::to_string(&amplitude).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyAmplitudeResult {
        expression: amplitude.expression.clone(),
        couplings: amplitude.couplings.clone(),
        momenta_labels: amplitude.momenta_labels.clone(),
        json,
    })
}

/// Calculate the threshold energy for a given set of final-state masses.
///
/// # Arguments
/// * `final_masses` — List of final-state particle masses in GeV.
/// * `target_mass` — Optional target mass for fixed-target kinematics.
///
/// # Returns
/// A `KinematicsResult` with threshold energy, optional lab energy, and JSON.
#[pyfunction]
#[pyo3(signature = (final_masses, target_mass=None))]
fn calculate_threshold(
    final_masses: Vec<f64>,
    target_mass: Option<f64>,
) -> PyResult<PyKinematicsResult> {
    let result = kinematics::calculate_thresholds(&final_masses, target_mass)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let json =
        serde_json::to_string(&result).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyKinematicsResult {
        threshold_energy: result.threshold_energy,
        lab_energy: result.lab_energy,
        json,
    })
}

/// Check whether a reaction is kinematically allowed at a given CM energy.
///
/// # Arguments
/// * `cms_energy` — Available centre-of-mass energy in GeV.
/// * `final_masses` — List of final-state particle masses in GeV.
///
/// # Returns
/// `True` if the process is kinematically allowed.
#[pyfunction]
#[pyo3(signature = (cms_energy, final_masses))]
fn is_kinematically_allowed(cms_energy: f64, final_masses: Vec<f64>) -> bool {
    kinematics::is_kinematically_allowed(cms_energy, &final_masses)
}
