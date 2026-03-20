//! # Python API - PyO3 Bindings for SPIRE
//!
//! This module exposes the SPIRE kernel to Python via PyO3, compiling the
//! kernel into a native Python extension module (`spire_hep`).
//!
//! ## Architecture
//!
//! The Python API is a thin, high-performance FFI wrapper over the generic
//! traits and functions in `spire-kernel`.  No physics logic is duplicated
//! here - every computation delegates to the kernel.
//!
//! ## Data Transport
//!
//! Two strategies are used depending on complexity:
//!
//! - **Native PyO3 classes** (`#[pyclass]`): For core numeric types
//!   (`SpacetimeVector`, `Complex`, `IntegrationResult`, `CrossSectionResult`,
//!   `HadronicCrossSectionResult`) that Python users interact with directly.
//! - **JSON serialization**: For deep nested structures (`TheoreticalModel`,
//!   `Reaction`, `FeynmanGraph`, `AmplitudeExpression`) that are better
//!   consumed as dictionaries via `json.loads()`.
//!
//! ## NumPy Interoperability
//!
//! High-volume data (phase-space events) is transferred via zero-copy NumPy
//! arrays using `rust-numpy`.  The `generate_phase_space_events` function
//! returns a `(momenta, weights)` tuple where `momenta` has shape
//! `(num_events, num_particles, 4)` and `weights` has shape `(num_events,)`.
//!
//! ## Usage
//!
//! ```python
//! import spire_hep as spire
//! import numpy as np
//!
//! # Load a model from TOML files.
//! model_json = spire.load_model(particles_toml, vertices_toml, "SM")
//!
//! # Construct and validate a reaction.
//! result = spire.construct_reaction(["e-", "e+"], ["mu-", "mu+"], 10.0, model_json)
//! print(result.is_valid, result.json)
//!
//! # Generate phase-space events as NumPy arrays.
//! momenta, weights = spire.generate_phase_space_events(100.0, [0.0, 0.0], 100000)
//! print(momenta.shape)  # (100000, 2, 4)
//! print(weights.shape)  # (100000,)
//!
//! # Calculate a partonic cross-section.
//! xsec = spire.calculate_cross_section(100.0, [0.0, 0.0], 50000)
//! print(f"σ = {xsec.cross_section_pb} ± {xsec.uncertainty_pb} pb")
//!
//! # Calculate a hadronic cross-section (proton-proton, 13 TeV).
//! had = spire.calculate_hadronic_cross_section(13000.0**2, [0.0, 0.0], 10000)
//! print(f"σ_pp = {had.cross_section_pb} ± {had.uncertainty_pb} pb")
//! ```
//!
//! Gated behind the `python` cargo feature to avoid pulling in PyO3/NumPy
//! dependencies when building for WASM or desktop-only targets.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use numpy::{PyArray1, PyArray3};

use spire_kernel::algebra;
use spire_kernel::data_loader;
use spire_kernel::graph::{self, LoopOrder};
use spire_kernel::kinematics;
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::ontology;
use spire_kernel::s_matrix;
use spire_kernel::SpireError;

// ===========================================================================
// Error Mapping
// ===========================================================================

/// Convert a `SpireError` into the appropriate Python exception.
///
/// Physics-facing errors (conservation violations, kinematics, unknown
/// particles, invalid vertices, model parsing, group theory) map to
/// `ValueError` since they represent invalid input.  Internal/algebra
/// errors map to `RuntimeError` since they indicate computational failures.
fn to_py_err(e: SpireError) -> PyErr {
    match e {
        SpireError::ConservationViolation(msg) => {
            PyValueError::new_err(format!("Conservation violation: {}", msg))
        }
        SpireError::KinematicsForbidden(msg) => {
            PyValueError::new_err(format!("Kinematics forbidden: {}", msg))
        }
        SpireError::UnknownParticle(msg) => {
            PyValueError::new_err(format!("Unknown particle: {}", msg))
        }
        SpireError::InvalidVertex(msg) => PyValueError::new_err(format!("Invalid vertex: {}", msg)),
        SpireError::ModelParseError(msg) => {
            PyValueError::new_err(format!("Model parse error: {}", msg))
        }
        SpireError::GroupTheoryError(msg) => {
            PyValueError::new_err(format!("Group theory error: {}", msg))
        }
        SpireError::DataParseError(msg) => {
            PyValueError::new_err(format!("Data parse error: {}", msg))
        }
        SpireError::DataMismatch(msg) => PyValueError::new_err(format!("Data mismatch: {}", msg)),
        SpireError::AlgebraError(msg) => PyRuntimeError::new_err(format!("Algebra error: {}", msg)),
        SpireError::InternalError(msg) => {
            PyRuntimeError::new_err(format!("Internal error: {}", msg))
        }
    }
}

// ===========================================================================
// Core Physics Type Wrappers
// ===========================================================================

/// A spacetime vector $v^\mu$ with $D$ components.
///
/// Wraps the kernel's `SpacetimeVector`. In 4D the components are
/// $(E, p_x, p_y, p_z)$ for a 4-momentum.
#[pyclass(name = "SpacetimeVector")]
#[derive(Debug, Clone)]
pub struct PySpacetimeVector {
    inner: algebra::SpacetimeVector,
}

#[pymethods]
impl PySpacetimeVector {
    /// Create a new spacetime vector from a list of components.
    #[new]
    #[pyo3(signature = (components))]
    fn new(components: Vec<f64>) -> PyResult<Self> {
        if components.is_empty() {
            return Err(PyValueError::new_err(
                "SpacetimeVector must have at least 1 component",
            ));
        }
        Ok(Self {
            inner: algebra::SpacetimeVector::new(components),
        })
    }

    /// Create a 4-vector $(E, p_x, p_y, p_z)$.
    #[staticmethod]
    fn new_4d(e: f64, px: f64, py: f64, pz: f64) -> Self {
        Self {
            inner: algebra::SpacetimeVector::new_4d(e, px, py, pz),
        }
    }

    /// The contravariant components $v^\mu$ as a Python list.
    #[getter]
    fn components(&self) -> Vec<f64> {
        self.inner.components().to_vec()
    }

    /// Number of spacetime dimensions.
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    /// Compute the Minkowski dot product $g_{\mu\nu} v^\mu w^\nu$ with
    /// signature $(+,-,-,-)$.
    fn dot(&self, other: &PySpacetimeVector) -> PyResult<f64> {
        let metric = algebra::MetricSignature::minkowski(self.inner.dimension());
        self.inner
            .dot(&other.inner, &metric)
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Invariant norm squared $v^\mu v_\mu$ (Minkowski metric).
    fn norm_sq(&self) -> PyResult<f64> {
        let metric = algebra::MetricSignature::minkowski(self.inner.dimension());
        self.inner
            .norm_sq(&metric)
            .map_err(|e| PyValueError::new_err(e))
    }

    /// Spatial magnitude $|\vec{v}| = \sqrt{v_1^2 + v_2^2 + v_3^2}$.
    fn spatial_magnitude(&self) -> f64 {
        self.inner.spatial_magnitude()
    }

    /// Scale the vector by a scalar factor.
    fn scale(&self, factor: f64) -> Self {
        Self {
            inner: self.inner.scale(factor),
        }
    }

    fn __repr__(&self) -> String {
        format!("SpacetimeVector({:?})", self.inner.components())
    }

    fn __add__(&self, other: &PySpacetimeVector) -> PyResult<Self> {
        if self.inner.dimension() != other.inner.dimension() {
            return Err(PyValueError::new_err("Dimension mismatch"));
        }
        let result = self.inner.clone() + other.inner.clone();
        Ok(Self { inner: result })
    }

    fn __sub__(&self, other: &PySpacetimeVector) -> PyResult<Self> {
        if self.inner.dimension() != other.inner.dimension() {
            return Err(PyValueError::new_err("Dimension mismatch"));
        }
        let result = self.inner.clone() - other.inner.clone();
        Ok(Self { inner: result })
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: -self.inner.clone(),
        }
    }
}

// ---------------------------------------------------------------------------

/// A complex number $z = \mathrm{Re}(z) + i \, \mathrm{Im}(z)$.
///
/// Used in numerical evaluation of scattering amplitudes.
#[pyclass(name = "Complex")]
#[derive(Debug, Clone, Copy)]
pub struct PyComplex {
    inner: algebra::Complex,
}

#[pymethods]
impl PyComplex {
    /// Create a complex number from real and imaginary parts.
    #[new]
    #[pyo3(signature = (re, im=0.0))]
    fn new(re: f64, im: f64) -> Self {
        Self {
            inner: algebra::Complex::new(re, im),
        }
    }

    /// Real part.
    #[getter]
    fn re(&self) -> f64 {
        self.inner.re
    }

    /// Imaginary part.
    #[getter]
    fn im(&self) -> f64 {
        self.inner.im
    }

    /// Squared modulus $|z|^2$.
    fn norm_sq(&self) -> f64 {
        self.inner.norm_sq()
    }

    /// Modulus $|z|$.
    fn norm(&self) -> f64 {
        self.inner.norm()
    }

    /// Complex conjugate $z^*$.
    fn conj(&self) -> Self {
        Self {
            inner: self.inner.conj(),
        }
    }

    fn __repr__(&self) -> String {
        if self.inner.im >= 0.0 {
            format!("Complex({} + {}i)", self.inner.re, self.inner.im)
        } else {
            format!("Complex({} - {}i)", self.inner.re, -self.inner.im)
        }
    }

    fn __add__(&self, other: &PyComplex) -> Self {
        Self {
            inner: self.inner + other.inner,
        }
    }

    fn __sub__(&self, other: &PyComplex) -> Self {
        Self {
            inner: self.inner - other.inner,
        }
    }

    fn __mul__(&self, other: &PyComplex) -> Self {
        Self {
            inner: self.inner * other.inner,
        }
    }
}

// ---------------------------------------------------------------------------

/// Result of a Monte Carlo integration.
///
/// Contains the estimated integral value, statistical uncertainty,
/// and diagnostic information.
#[pyclass(name = "IntegrationResult")]
#[derive(Debug, Clone)]
pub struct PyIntegrationResult {
    inner: spire_kernel::integration::IntegrationResult,
}

#[pymethods]
impl PyIntegrationResult {
    /// The estimated integral value (e.g., cross-section in GeV⁻²).
    #[getter]
    fn value(&self) -> f64 {
        self.inner.value
    }

    /// Statistical uncertainty (standard error).
    #[getter]
    fn error(&self) -> f64 {
        self.inner.error
    }

    /// Number of phase-space events evaluated.
    #[getter]
    fn events_evaluated(&self) -> usize {
        self.inner.events_evaluated
    }

    /// Fraction of events that contributed (efficiency).
    #[getter]
    fn efficiency(&self) -> f64 {
        self.inner.efficiency
    }

    /// Relative error $\delta\sigma / \sigma$.
    #[getter]
    fn relative_error(&self) -> f64 {
        self.inner.relative_error
    }

    /// Convert from natural units (GeV⁻²) to picobarns.
    fn to_picobarns(&self) -> Self {
        Self {
            inner: self.inner.to_picobarns(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "IntegrationResult(value={:.6e}, error={:.6e}, events={})",
            self.inner.value, self.inner.error, self.inner.events_evaluated
        )
    }
}

// ---------------------------------------------------------------------------

/// Result of a partonic Monte Carlo cross-section calculation.
///
/// Bundles the cross-section in both natural units (GeV⁻²) and picobarns,
/// with statistical uncertainties and process metadata.
#[pyclass(name = "CrossSectionResult")]
#[derive(Debug, Clone)]
pub struct PyCrossSectionResult {
    inner: s_matrix::CrossSectionResult,
}

#[pymethods]
impl PyCrossSectionResult {
    /// Total cross-section in natural units (GeV⁻²).
    #[getter]
    fn cross_section(&self) -> f64 {
        self.inner.cross_section
    }

    /// Statistical uncertainty (GeV⁻²).
    #[getter]
    fn uncertainty(&self) -> f64 {
        self.inner.uncertainty
    }

    /// Cross-section in picobarns.
    #[getter]
    fn cross_section_pb(&self) -> f64 {
        self.inner.cross_section_pb
    }

    /// Uncertainty in picobarns.
    #[getter]
    fn uncertainty_pb(&self) -> f64 {
        self.inner.uncertainty_pb
    }

    /// Number of phase-space events evaluated.
    #[getter]
    fn events_evaluated(&self) -> usize {
        self.inner.events_evaluated
    }

    /// Relative error $\delta\sigma / \sigma$.
    #[getter]
    fn relative_error(&self) -> f64 {
        self.inner.relative_error
    }

    /// Centre-of-mass energy used (GeV).
    #[getter]
    fn cms_energy(&self) -> f64 {
        self.inner.cms_energy
    }

    /// Description of the squared matrix element model used.
    #[getter]
    fn amplitude_model(&self) -> String {
        self.inner.amplitude_model.clone()
    }

    /// Serialise to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CrossSectionResult(cross_section={:.6e} GeV^-2, cross_section_pb={:.6e} pb, events={})",
            self.inner.cross_section, self.inner.cross_section_pb, self.inner.events_evaluated
        )
    }
}

// ---------------------------------------------------------------------------

/// Result of a hadronic (PDF-convoluted) cross-section calculation.
///
/// Bundles the convolution result with beam metadata and PDF information.
#[pyclass(name = "HadronicCrossSectionResult")]
#[derive(Debug, Clone)]
pub struct PyHadronicResult {
    inner: spire_kernel::pdf::HadronicCrossSectionResult,
}

#[pymethods]
impl PyHadronicResult {
    /// Total hadronic cross-section in natural units (GeV⁻²).
    #[getter]
    fn cross_section(&self) -> f64 {
        self.inner.cross_section
    }

    /// Statistical uncertainty (GeV⁻²).
    #[getter]
    fn uncertainty(&self) -> f64 {
        self.inner.uncertainty
    }

    /// Cross-section in picobarns.
    #[getter]
    fn cross_section_pb(&self) -> f64 {
        self.inner.cross_section_pb
    }

    /// Uncertainty in picobarns.
    #[getter]
    fn uncertainty_pb(&self) -> f64 {
        self.inner.uncertainty_pb
    }

    /// Number of events evaluated.
    #[getter]
    fn events_evaluated(&self) -> usize {
        self.inner.events_evaluated
    }

    /// Relative error $\delta\sigma / \sigma$.
    #[getter]
    fn relative_error(&self) -> f64 {
        self.inner.relative_error
    }

    /// Squared beam energy $S$ in GeV².
    #[getter]
    fn beam_energy_sq(&self) -> f64 {
        self.inner.beam_energy_sq
    }

    /// Name of the PDF set used.
    #[getter]
    fn pdf_name(&self) -> String {
        self.inner.pdf_name.clone()
    }

    /// Description of the initial-state hadrons.
    #[getter]
    fn beam_description(&self) -> String {
        self.inner.beam_description.clone()
    }

    /// Serialise to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HadronicCrossSectionResult(cross_section_pb={:.6e} pb, beam='{}')",
            self.inner.cross_section_pb, self.inner.beam_description
        )
    }
}

// ===========================================================================
// Legacy Wrapper Classes (from Phase 2 - retained for API stability)
// ===========================================================================

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
    /// Return the JSON string (for use with `json.loads()`).
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

// ===========================================================================
// PyO3 Module Registration
// ===========================================================================

/// The top-level Python module `spire_hep`.
///
/// Registers all Python-visible functions and classes.
#[pymodule]
fn spire_hep(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Core physics types
    m.add_class::<PySpacetimeVector>()?;
    m.add_class::<PyComplex>()?;
    m.add_class::<PyIntegrationResult>()?;
    m.add_class::<PyCrossSectionResult>()?;
    m.add_class::<PyHadronicResult>()?;

    // Legacy wrapper classes
    m.add_class::<PyReactionResult>()?;
    m.add_class::<PyDiagramSet>()?;
    m.add_class::<PyAmplitudeResult>()?;
    m.add_class::<PyKinematicsResult>()?;

    // NumPy event generation + integrator wrappers
    m.add_wrapped(wrap_pyfunction!(generate_phase_space_events))?;
    m.add_wrapped(wrap_pyfunction!(calculate_cross_section))?;
    m.add_wrapped(wrap_pyfunction!(calculate_hadronic_cross_section))?;

    // Legacy model/reaction/diagram functions
    m.add_wrapped(wrap_pyfunction!(load_model))?;
    m.add_wrapped(wrap_pyfunction!(construct_reaction))?;
    m.add_wrapped(wrap_pyfunction!(reconstruct_reaction))?;
    m.add_wrapped(wrap_pyfunction!(generate_diagrams))?;
    m.add_wrapped(wrap_pyfunction!(compute_amplitude))?;
    m.add_wrapped(wrap_pyfunction!(calculate_threshold))?;
    m.add_wrapped(wrap_pyfunction!(is_kinematically_allowed))?;

    Ok(())
}

// ===========================================================================
// NumPy Event Generation
// ===========================================================================

/// Generate $N$-body phase-space events as contiguous NumPy arrays.
///
/// Uses the RAMBO algorithm to produce Lorentz-invariant phase-space points
/// for $2 \to N$ scattering at the specified centre-of-mass energy.
///
/// Returns a tuple `(momenta, weights)` where:
/// - `momenta` is a `numpy.ndarray` of shape `(num_events, num_particles, 4)`
///   with dtype `float64`. The 4-momentum components are ordered $(E, p_x, p_y, p_z)$.
/// - `weights` is a `numpy.ndarray` of shape `(num_events,)` containing the
///   Lorentz-invariant phase space weight for each event.
///
/// # Arguments
/// * `cms_energy` - Centre-of-mass energy $\sqrt{s}$ in GeV.
/// * `final_masses` - List of final-state particle masses in GeV.
/// * `num_events` - Number of events to generate.
/// * `seed` - Optional RNG seed for reproducible generation.
///
/// # Example
/// ```python
/// momenta, weights = spire.generate_phase_space_events(100.0, [0.0, 0.0], 100000)
/// energies = momenta[:, :, 0]  # All particle energies
/// ```
#[pyfunction]
#[pyo3(signature = (cms_energy, final_masses, num_events, seed=None))]
fn generate_phase_space_events<'py>(
    py: Python<'py>,
    cms_energy: f64,
    final_masses: Vec<f64>,
    num_events: usize,
    seed: Option<u64>,
) -> PyResult<(&'py PyArray3<f64>, &'py PyArray1<f64>)> {
    use spire_kernel::kinematics::{PhaseSpaceGenerator, RamboGenerator};

    if final_masses.is_empty() {
        return Err(PyValueError::new_err(
            "At least one final-state particle mass is required",
        ));
    }
    if num_events == 0 {
        return Err(PyValueError::new_err("Number of events must be positive"));
    }

    let n_particles = final_masses.len();

    let mut generator = match seed {
        Some(s) => RamboGenerator::with_seed(s),
        None => RamboGenerator::new(),
    };

    // Allocate contiguous NumPy arrays.
    let momenta_array = PyArray3::<f64>::zeros(py, [num_events, n_particles, 4], false);
    let weights_array = PyArray1::<f64>::zeros(py, [num_events], false);

    // Fill the arrays event-by-event.
    // SAFETY: We have exclusive access (just allocated) and indices are in-bounds.
    unsafe {
        let momenta_slice = momenta_array
            .as_slice_mut()
            .map_err(|e| PyRuntimeError::new_err(format!("NumPy buffer error: {}", e)))?;
        let weights_slice = weights_array
            .as_slice_mut()
            .map_err(|e| PyRuntimeError::new_err(format!("NumPy buffer error: {}", e)))?;

        for i in 0..num_events {
            let event = generator
                .generate_event(cms_energy, &final_masses)
                .map_err(to_py_err)?;

            weights_slice[i] = event.weight;

            for (j, momentum) in event.momenta.iter().enumerate() {
                let base = i * n_particles * 4 + j * 4;
                let comps = momentum.components();
                // Copy (E, px, py, pz) - at most 4 components.
                let n_copy = comps.len().min(4);
                momenta_slice[base..base + n_copy].copy_from_slice(&comps[..n_copy]);
                // Zero-pad if fewer than 4 components (shouldn't happen for 4D).
                for k in n_copy..4 {
                    momenta_slice[base + k] = 0.0;
                }
            }
        }
    }

    Ok((momenta_array, weights_array))
}

// ===========================================================================
// Integrator Wrappers
// ===========================================================================

/// Calculate a partonic cross-section via Monte Carlo integration.
///
/// Uses a constant $|\mathcal{M}|^2 = 1$ (phase-space only) with the RAMBO
/// algorithm.  For user-supplied matrix elements, use the
/// `generate_phase_space_events` function and compute the weighted average
/// in Python/NumPy.
///
/// # Arguments
/// * `cms_energy` - Centre-of-mass energy $\sqrt{s}$ in GeV.
/// * `final_masses` - List of final-state particle masses in GeV.
/// * `num_events` - Number of Monte Carlo samples.
///
/// # Returns
/// A `CrossSectionResult` object.
#[pyfunction]
#[pyo3(signature = (cms_energy, final_masses, num_events))]
fn calculate_cross_section(
    cms_energy: f64,
    final_masses: Vec<f64>,
    num_events: usize,
) -> PyResult<PyCrossSectionResult> {
    let result =
        s_matrix::calculate_phase_space_cross_section(cms_energy, &final_masses, num_events)
            .map_err(to_py_err)?;

    Ok(PyCrossSectionResult { inner: result })
}

/// Calculate a hadronic (PDF-convoluted) cross-section for proton–proton
/// collisions using the built-in toy proton PDF.
///
/// Performs the convolution:
/// $$\sigma_{pp} = \sum_{a,b} \int dx_1\,dx_2\;
///   f_a(x_1, Q^2)\,f_b(x_2, Q^2)\;\hat{\sigma}_{ab}(\hat{s})$$
///
/// with constant $|\mathcal{M}|^2 = 1$ (phase-space only).
///
/// # Arguments
/// * `beam_energy_sq` - Hadronic $S$ in GeV² (e.g., `13000.0**2` for 13 TeV LHC).
/// * `final_masses` - List of final-state particle masses in GeV.
/// * `num_events` - Number of Monte Carlo samples per parton channel.
///
/// # Returns
/// A `HadronicCrossSectionResult` object.
#[pyfunction]
#[pyo3(signature = (beam_energy_sq, final_masses, num_events))]
fn calculate_hadronic_cross_section(
    beam_energy_sq: f64,
    final_masses: Vec<f64>,
    num_events: usize,
) -> PyResult<PyHadronicResult> {
    let result = s_matrix::calculate_hadronic_phase_space_cross_section(
        beam_energy_sq,
        &final_masses,
        num_events,
    )
    .map_err(to_py_err)?;

    Ok(PyHadronicResult { inner: result })
}

// ===========================================================================
// Legacy Functions (from Phase 2 - retained for API stability)
// ===========================================================================

/// Load a theoretical model from TOML file contents.
///
/// Returns a JSON string of the complete `TheoreticalModel`.
/// Use `json.loads()` to convert to a Python dictionary.
#[pyfunction]
#[pyo3(signature = (particles_toml, vertices_toml, model_name="Standard Model"))]
fn load_model(particles_toml: &str, vertices_toml: &str, model_name: &str) -> PyResult<String> {
    let model =
        data_loader::build_model(particles_toml, vertices_toml, model_name).map_err(to_py_err)?;
    serde_json::to_string(&model).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Construct and validate a specific reaction.
///
/// Returns a `ReactionResult` with validity, diagnostics, and full JSON.
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

    let reaction = s_matrix::construct_reaction(&init_refs, &final_refs, &model, Some(cms_energy))
        .map_err(to_py_err)?;

    let json =
        serde_json::to_string(&reaction).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

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
/// Returns a JSON string containing a list of `ReconstructedFinalState` objects.
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

    let results =
        s_matrix::reconstruct_reaction(&particles, &model, cms_energy).map_err(to_py_err)?;

    serde_json::to_string(&results).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Generate Feynman diagrams for a reaction (given as JSON).
///
/// Returns a `DiagramSet` with the count and full JSON.
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

    let topologies = graph::generate_topologies(&reaction, &model, order).map_err(to_py_err)?;

    let json =
        serde_json::to_string(&topologies).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyDiagramSet {
        count: topologies.count,
        json,
    })
}

/// Compute the invariant amplitude for a specific diagram (given as JSON).
///
/// Returns an `AmplitudeResult` with the expression and full JSON.
#[pyfunction]
fn compute_amplitude(diagram_json: String) -> PyResult<PyAmplitudeResult> {
    let diagram: graph::FeynmanGraph =
        serde_json::from_str(&diagram_json).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let amplitude = algebra::generate_amplitude(&diagram).map_err(to_py_err)?;

    let json =
        serde_json::to_string(&amplitude).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyAmplitudeResult {
        expression: amplitude.expression.clone(),
        couplings: amplitude.couplings.clone(),
        momenta_labels: amplitude.momenta_labels.clone(),
        json,
    })
}

/// Calculate the threshold energy for a given set of final-state masses.
///
/// Returns a `KinematicsResult` with threshold energy, optional lab energy, and JSON.
#[pyfunction]
#[pyo3(signature = (final_masses, target_mass=None))]
fn calculate_threshold(
    final_masses: Vec<f64>,
    target_mass: Option<f64>,
) -> PyResult<PyKinematicsResult> {
    let result = kinematics::calculate_thresholds(&final_masses, target_mass).map_err(to_py_err)?;

    let json =
        serde_json::to_string(&result).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyKinematicsResult {
        threshold_energy: result.threshold_energy,
        lab_energy: result.lab_energy,
        json,
    })
}

/// Check whether a reaction is kinematically allowed at a given CM energy.
///
/// Returns `True` if the process is kinematically allowed.
#[pyfunction]
#[pyo3(signature = (cms_energy, final_masses))]
fn is_kinematically_allowed(cms_energy: f64, final_masses: Vec<f64>) -> bool {
    kinematics::is_kinematically_allowed(cms_energy, &final_masses)
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_complex_arithmetic() {
        let a = algebra::Complex::new(1.0, 2.0);
        let b = algebra::Complex::new(3.0, -1.0);
        let sum = a + b;
        assert!((sum.re - 4.0).abs() < 1e-12);
        assert!((sum.im - 1.0).abs() < 1e-12);
    }

    #[test]
    fn py_spacetime_vector_dot() {
        let v = algebra::SpacetimeVector::new_4d(5.0, 1.0, 2.0, 3.0);
        let metric = algebra::MetricSignature::minkowski(4);
        let nsq = v.norm_sq(&metric).unwrap();
        // 5² - 1² - 2² - 3² = 25 - 14 = 11
        assert!((nsq - 11.0).abs() < 1e-12);
    }

    #[test]
    fn to_py_err_maps_conservation_violation() {
        let err = SpireError::ConservationViolation("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Conservation violation"));
    }

    #[test]
    fn to_py_err_maps_kinematics_forbidden() {
        let err = SpireError::KinematicsForbidden("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Kinematics forbidden"));
    }

    #[test]
    fn to_py_err_maps_algebra_error() {
        let err = SpireError::AlgebraError("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Algebra error"));
    }

    #[test]
    fn to_py_err_maps_internal_error() {
        let err = SpireError::InternalError("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Internal error"));
    }

    #[test]
    fn to_py_err_maps_unknown_particle() {
        let err = SpireError::UnknownParticle("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Unknown particle"));
    }

    #[test]
    fn to_py_err_maps_invalid_vertex() {
        let err = SpireError::InvalidVertex("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Invalid vertex"));
    }

    #[test]
    fn to_py_err_maps_model_parse_error() {
        let err = SpireError::ModelParseError("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Model parse error"));
    }

    #[test]
    fn to_py_err_maps_group_theory_error() {
        let err = SpireError::GroupTheoryError("test".into());
        let py_err = to_py_err(err);
        let msg = py_err.to_string();
        assert!(msg.contains("Group theory error"));
    }

    #[test]
    fn integration_result_wrapper() {
        let inner = spire_kernel::integration::IntegrationResult {
            value: 1.23e-6,
            error: 4.56e-8,
            events_evaluated: 10000,
            efficiency: 1.0,
            relative_error: 0.037,
        };
        let py = PyIntegrationResult {
            inner: inner.clone(),
        };
        assert!((py.inner.value - 1.23e-6).abs() < 1e-15);
        assert_eq!(py.inner.events_evaluated, 10000);

        let pb = py.inner.to_picobarns();
        let conv = 0.3894e9;
        assert!((pb.value - 1.23e-6 * conv).abs() < 1.0);
    }

    #[test]
    fn cross_section_result_wrapper() {
        let inner = s_matrix::CrossSectionResult {
            cross_section: 1.0e-8,
            uncertainty: 1.0e-10,
            cross_section_pb: 3.894e1,
            uncertainty_pb: 3.894e-1,
            events_evaluated: 50000,
            relative_error: 0.01,
            cms_energy: 100.0,
            amplitude_model: "test".into(),
        };
        let py = PyCrossSectionResult { inner };
        assert!((py.inner.cross_section - 1.0e-8).abs() < 1e-20);
        assert_eq!(py.inner.events_evaluated, 50000);
    }

    #[test]
    fn hadronic_result_wrapper() {
        let inner = spire_kernel::pdf::HadronicCrossSectionResult {
            cross_section: 2.0e-8,
            uncertainty: 1.0e-9,
            cross_section_pb: 7.788e1,
            uncertainty_pb: 3.894e0,
            events_evaluated: 10000,
            relative_error: 0.05,
            beam_energy_sq: 1.69e8,
            pdf_name: "ToyProtonPdf".into(),
            beam_description: "p p -> X".into(),
        };
        let py = PyHadronicResult { inner };
        assert!((py.inner.cross_section - 2.0e-8).abs() < 1e-20);
        assert_eq!(py.inner.pdf_name, "ToyProtonPdf");
    }

    #[test]
    fn spacetime_vector_operations() {
        let v1 = algebra::SpacetimeVector::new_4d(10.0, 1.0, 0.0, 0.0);
        let v2 = algebra::SpacetimeVector::new_4d(5.0, 0.0, 1.0, 0.0);

        let sum = v1.clone() + v2.clone();
        assert!((sum[0] - 15.0).abs() < 1e-12);
        assert!((sum[1] - 1.0).abs() < 1e-12);
        assert!((sum[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn complex_conjugate() {
        let z = algebra::Complex::new(3.0, 4.0);
        let zc = z.conj();
        assert!((zc.re - 3.0).abs() < 1e-12);
        assert!((zc.im + 4.0).abs() < 1e-12);
        assert!((z.norm() - 5.0).abs() < 1e-12);
    }
}
