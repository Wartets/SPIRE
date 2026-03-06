//! `DecayTable` and `DecayChannel` - Python wrappers for particle decays.

use pyo3::prelude::*;

use spire_kernel::decay;

use crate::errors::to_py_err;
use crate::model::PyModel;

/// A single decay channel with partial width and branching ratio.
#[pyclass(name = "DecayChannel")]
#[derive(Debug, Clone)]
pub struct PyDecayChannel {
    pub(crate) inner: decay::DecayChannel,
}

#[pymethods]
impl PyDecayChannel {
    /// Final-state particle identifiers.
    #[getter]
    fn final_state(&self) -> Vec<String> {
        self.inner.final_state.clone()
    }

    /// Final-state particle display names.
    #[getter]
    fn final_state_names(&self) -> Vec<String> {
        self.inner.final_state_names.clone()
    }

    /// Partial width Γᵢ in GeV.
    #[getter]
    fn partial_width(&self) -> f64 {
        self.inner.partial_width
    }

    /// Branching ratio BRᵢ = Γᵢ / Γ_tot.
    #[getter]
    fn branching_ratio(&self) -> f64 {
        self.inner.branching_ratio
    }

    /// The vertex factor ID that mediates this channel.
    #[getter]
    fn vertex_id(&self) -> &str {
        &self.inner.vertex_id
    }

    fn __repr__(&self) -> String {
        format!(
            "DecayChannel({} → {}, BR={:.4e})",
            self.inner.vertex_id,
            self.inner.final_state_names.join(" "),
            self.inner.branching_ratio,
        )
    }
}

/// Complete decay table for a parent particle.
///
/// ```python
/// from spire import Model, DecayTable
///
/// sm = Model.standard_model()
/// dt = DecayTable.calculate(sm, "Z0")
/// print(dt.total_width)
/// for ch in dt.channels:
///     print(f"  {ch.final_state_names}  BR={ch.branching_ratio:.4e}")
/// ```
#[pyclass(name = "DecayTable")]
#[derive(Debug, Clone)]
pub struct PyDecayTable {
    pub(crate) inner: decay::DecayTable,
}

#[pymethods]
impl PyDecayTable {
    /// Calculate the decay table for a particle in a given model.
    ///
    /// Parameters
    /// ----------
    /// model : Model
    ///     The theoretical model.
    /// particle_id : str
    ///     The identifier of the parent particle (e.g. ``"Z0"``, ``"H0"``).
    #[staticmethod]
    fn calculate(model: &PyModel, particle_id: &str) -> PyResult<Self> {
        let table =
            decay::calculate_decay_table(&model.inner, particle_id).map_err(to_py_err)?;
        Ok(Self { inner: table })
    }

    /// Parent particle identifier.
    #[getter]
    fn parent_id(&self) -> &str {
        &self.inner.parent_id
    }

    /// Parent particle display name.
    #[getter]
    fn parent_name(&self) -> &str {
        &self.inner.parent_name
    }

    /// Parent particle mass in GeV.
    #[getter]
    fn parent_mass(&self) -> f64 {
        self.inner.parent_mass
    }

    /// Total decay width Γ_tot in GeV.
    #[getter]
    fn total_width(&self) -> f64 {
        self.inner.total_width
    }

    /// Mean lifetime τ in seconds.
    #[getter]
    fn lifetime_seconds(&self) -> f64 {
        self.inner.lifetime_seconds
    }

    /// Number of decay channels.
    #[getter]
    fn num_channels(&self) -> usize {
        self.inner.channels.len()
    }

    /// All kinematically allowed decay channels.
    #[getter]
    fn channels(&self) -> Vec<PyDecayChannel> {
        self.inner
            .channels
            .iter()
            .map(|c| PyDecayChannel { inner: c.clone() })
            .collect()
    }

    /// Export the decay table in SLHA format.
    ///
    /// Parameters
    /// ----------
    /// pdg_code : int
    ///     The PDG Monte Carlo particle number.
    fn to_slha(&self, pdg_code: i32) -> String {
        self.inner.to_slha_string(pdg_code)
    }

    fn __repr__(&self) -> String {
        format!(
            "DecayTable(parent='{}', width={:.4e} GeV, channels={})",
            self.inner.parent_name,
            self.inner.total_width,
            self.inner.channels.len(),
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __len__(&self) -> usize {
        self.inner.channels.len()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let mut html = format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">💥 Decay Table - {}</h4>\
             <table style=\"border-collapse:collapse; margin-bottom:4px;\">\
             <tr><td style=\"padding:2px 8px;\">Mass</td>\
             <td style=\"padding:2px 8px;\">{:.4} GeV</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Γ_tot</td>\
             <td style=\"padding:2px 8px;\">{:.4e} GeV</td></tr>\
             <tr><td style=\"padding:2px 8px;\">τ</td>\
             <td style=\"padding:2px 8px;\">{:.4e} s</td></tr>\
             </table>\n",
            self.inner.parent_name,
            self.inner.parent_mass,
            self.inner.total_width,
            self.inner.lifetime_seconds,
        );

        if !self.inner.channels.is_empty() {
            html.push_str(
                "<table style=\"border-collapse:collapse; font-size:12px;\">\
                 <tr style=\"background:#e8e8e8;\">\
                 <th style=\"padding:3px 8px; border:1px solid #ccc;\">Final State</th>\
                 <th style=\"padding:3px 8px; border:1px solid #ccc;\">Γᵢ (GeV)</th>\
                 <th style=\"padding:3px 8px; border:1px solid #ccc;\">BR</th>\
                 </tr>\n",
            );

            for ch in &self.inner.channels {
                html.push_str(&format!(
                    "<tr>\
                     <td style=\"padding:3px 8px; border:1px solid #eee;\">{}</td>\
                     <td style=\"padding:3px 8px; border:1px solid #eee; text-align:right;\">{:.4e}</td>\
                     <td style=\"padding:3px 8px; border:1px solid #eee; text-align:right;\">{:.4e}</td>\
                     </tr>\n",
                    ch.final_state_names.join(" + "),
                    ch.partial_width,
                    ch.branching_ratio,
                ));
            }

            html.push_str("</table>\n");
        }

        html.push_str("</div>");
        html
    }
}
