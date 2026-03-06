//! `AmplitudeResult` - Python wrapper around amplitude computation.

use pyo3::prelude::*;

use spire_kernel::algebra;
use spire_kernel::graph;

use crate::errors::to_py_err;

/// The symbolic amplitude expression for a single Feynman diagram.
///
/// ```python
/// from spire import AmplitudeResult
///
/// # Typically obtained from DiagramSet + Model pipeline:
/// amp = AmplitudeResult.from_diagram_json(diagram_json)
/// print(amp.expression)
/// print(amp.couplings)
/// ```
#[pyclass(name = "AmplitudeResult")]
#[derive(Debug, Clone)]
pub struct PyAmplitudeResult {
    /// Symbolic expression string.
    #[pyo3(get)]
    pub expression: String,
    /// Coupling constants appearing in the amplitude.
    #[pyo3(get)]
    pub couplings: Vec<String>,
    /// Momentum labels appearing in the amplitude.
    #[pyo3(get)]
    pub momenta_labels: Vec<String>,
    /// Full JSON serialization.
    json_cache: String,
}

#[pymethods]
impl PyAmplitudeResult {
    /// Compute the amplitude for a diagram specified as a JSON string.
    ///
    /// Parameters
    /// ----------
    /// diagram_json : str
    ///     JSON serialization of a single ``FeynmanGraph``.
    #[staticmethod]
    fn from_diagram_json(diagram_json: &str) -> PyResult<Self> {
        let diagram: graph::FeynmanGraph = serde_json::from_str(diagram_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let amplitude = algebra::generate_amplitude(&diagram).map_err(to_py_err)?;

        let json = serde_json::to_string(&amplitude)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self {
            expression: amplitude.expression.clone(),
            couplings: amplitude.couplings.clone(),
            momenta_labels: amplitude.momenta_labels.clone(),
            json_cache: json,
        })
    }

    /// Serialised JSON of the amplitude.
    fn to_json(&self) -> String {
        self.json_cache.clone()
    }

    fn __repr__(&self) -> String {
        format!("AmplitudeResult(expression='{}')", self.expression)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">📐 Amplitude</h4>\
             <p style=\"font-family:monospace;\">{}</p>\
             <p>Couplings: {}</p>\
             <p>Momenta: {}</p>\
             </div>",
            self.expression,
            self.couplings.join(", "),
            self.momenta_labels.join(", "),
        )
    }
}
