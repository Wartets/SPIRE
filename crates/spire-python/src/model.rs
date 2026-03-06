//! `Model` - Python wrapper around [`TheoreticalModel`].
//!
//! Provides ergonomic model loading with built-in Standard Model preset,
//! field lookup, and Jupyter `_repr_html_` rendering.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use spire_kernel::data_loader;
use spire_kernel::lagrangian::TheoreticalModel;

use crate::errors::to_py_err;
use crate::particle::PyField;

// ── Embedded SM data files ───────────────────────────────────────────────
const SM_PARTICLES_TOML: &str = include_str!("../../../data/particles.toml");
const SM_VERTICES_TOML: &str = include_str!("../../../data/sm_vertices.toml");

/// A theoretical model (e.g. the Standard Model) that defines all fields,
/// Lagrangian terms, vertex factors, and propagators.
///
/// Create via the class methods `Model.standard_model()` or
/// `Model.from_toml(particles, vertices, name)`.
///
/// ```python
/// from spire import Model
///
/// sm = Model.standard_model()
/// print(sm.name)            # "Standard Model"
/// print(len(sm.fields))     # number of fields
/// electron = sm.get_field("e-")
/// ```
#[pyclass(name = "Model")]
#[derive(Debug, Clone)]
pub struct PyModel {
    pub(crate) inner: TheoreticalModel,
}

#[pymethods]
impl PyModel {
    // ── Constructors ─────────────────────────────────────────────────

    /// Load the built-in Standard Model.
    ///
    /// Returns a fully-constructed `Model` with all SM fields, vertices,
    /// and propagators.
    #[staticmethod]
    fn standard_model() -> PyResult<Self> {
        let model =
            data_loader::build_model(SM_PARTICLES_TOML, SM_VERTICES_TOML, "Standard Model")
                .map_err(to_py_err)?;
        Ok(Self { inner: model })
    }

    /// Load a model from custom TOML strings.
    ///
    /// Parameters
    /// ----------
    /// particles_toml : str
    ///     Contents of the particles definition file.
    /// vertices_toml : str
    ///     Contents of the vertices / interactions definition file.
    /// name : str, optional
    ///     Name to assign to the model (default: ``"Custom Model"``).
    #[staticmethod]
    #[pyo3(signature = (particles_toml, vertices_toml, name="Custom Model"))]
    fn from_toml(particles_toml: &str, vertices_toml: &str, name: &str) -> PyResult<Self> {
        let model = data_loader::build_model(particles_toml, vertices_toml, name).map_err(to_py_err)?;
        Ok(Self { inner: model })
    }

    // ── Properties ───────────────────────────────────────────────────

    /// The model name (e.g. ``"Standard Model"``).
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// The model description.
    #[getter]
    fn description(&self) -> &str {
        &self.inner.description
    }

    /// Number of fields defined in the model.
    #[getter]
    fn num_fields(&self) -> usize {
        self.inner.fields.len()
    }

    /// Number of vertex factors in the model.
    #[getter]
    fn num_vertices(&self) -> usize {
        self.inner.vertex_factors.len()
    }

    /// List of all field wrapper objects.
    #[getter]
    fn fields(&self) -> Vec<PyField> {
        self.inner
            .fields
            .iter()
            .map(|f| PyField {
                inner: f.clone(),
            })
            .collect()
    }

    // ── Lookup ───────────────────────────────────────────────────────

    /// Look up a field by its identifier string (e.g. ``"e-"``, ``"photon"``).
    ///
    /// Raises ``ValueError`` if the field is not found in the model.
    fn get_field(&self, field_id: &str) -> PyResult<PyField> {
        self.inner
            .fields
            .iter()
            .find(|f| f.id == field_id)
            .map(|f| PyField {
                inner: f.clone(),
            })
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Unknown field '{}'. Available: [{}]",
                    field_id,
                    self.inner
                        .fields
                        .iter()
                        .map(|f| f.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }

    /// Serialize the entire model to a JSON string.
    ///
    /// Useful for interoperability with other tools or for saving / loading
    /// model snapshots.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    // ── Display ──────────────────────────────────────────────────────

    fn __repr__(&self) -> String {
        format!(
            "Model(name='{}', fields={}, vertices={})",
            self.inner.name,
            self.inner.fields.len(),
            self.inner.vertex_factors.len(),
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    ///
    /// Renders an HTML summary table of the model's fields grouped by type
    /// (leptons, quarks, gauge bosons, scalar bosons).
    fn _repr_html_(&self) -> String {
        let mut html = String::from(
            "<div style=\"font-family: 'Segoe UI', sans-serif;\">\n\
             <h3 style=\"margin:0 0 8px 0;\">🔬 SPIRE Model</h3>\n\
             <table style=\"border-collapse:collapse; font-size:13px;\">\n\
             <tr style=\"background:#f4f4f4;\">\
             <th style=\"padding:4px 10px; border:1px solid #ddd;\">Name</th>\
             <td style=\"padding:4px 10px; border:1px solid #ddd;\">",
        );
        html.push_str(&self.inner.name);
        html.push_str(
            "</td></tr>\n\
             <tr><th style=\"padding:4px 10px; border:1px solid #ddd;\">Fields</th>\
             <td style=\"padding:4px 10px; border:1px solid #ddd;\">",
        );
        html.push_str(&self.inner.fields.len().to_string());
        html.push_str(
            "</td></tr>\n\
             <tr style=\"background:#f4f4f4;\">\
             <th style=\"padding:4px 10px; border:1px solid #ddd;\">Vertices</th>\
             <td style=\"padding:4px 10px; border:1px solid #ddd;\">",
        );
        html.push_str(&self.inner.vertex_factors.len().to_string());
        html.push_str(
            "</td></tr>\n\
             <tr><th style=\"padding:4px 10px; border:1px solid #ddd;\">Propagators</th>\
             <td style=\"padding:4px 10px; border:1px solid #ddd;\">",
        );
        html.push_str(&self.inner.propagators.len().to_string());
        html.push_str("</td></tr>\n</table>\n");

        // ── Field summary table ──────────────────────────────────────
        html.push_str(
            "<details open><summary style=\"cursor:pointer; margin-top:8px; font-weight:600;\">\
             Fields</summary>\n\
             <table style=\"border-collapse:collapse; font-size:12px; margin-top:4px;\">\n\
             <tr style=\"background:#e8e8e8;\">\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">ID</th>\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">Name</th>\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">Symbol</th>\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">Mass (GeV)</th>\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">Spin</th>\
             <th style=\"padding:3px 8px; border:1px solid #ccc;\">Q</th>\
             </tr>\n",
        );

        for f in &self.inner.fields {
            let spin_val = f.quantum_numbers.spin.0 as f64 / 2.0;
            let charge_val = f.quantum_numbers.electric_charge.0 as f64 / 3.0;
            html.push_str(&format!(
                "<tr>\
                 <td style=\"padding:3px 8px; border:1px solid #eee; font-family:monospace;\">{}</td>\
                 <td style=\"padding:3px 8px; border:1px solid #eee;\">{}</td>\
                 <td style=\"padding:3px 8px; border:1px solid #eee;\">{}</td>\
                 <td style=\"padding:3px 8px; border:1px solid #eee; text-align:right;\">{:.4}</td>\
                 <td style=\"padding:3px 8px; border:1px solid #eee; text-align:center;\">{}</td>\
                 <td style=\"padding:3px 8px; border:1px solid #eee; text-align:center;\">{:+.2}</td>\
                 </tr>\n",
                f.id,
                f.name,
                f.symbol,
                f.mass,
                if spin_val.fract() == 0.0 {
                    format!("{}", spin_val as i32)
                } else {
                    format!("{}/2", f.quantum_numbers.spin.0)
                },
                charge_val,
            ));
        }

        html.push_str("</table>\n</details>\n</div>");
        html
    }
}
