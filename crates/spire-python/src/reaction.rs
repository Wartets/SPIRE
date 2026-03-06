//! `Reaction` - Python wrapper around [`s_matrix::Reaction`].
//!
//! Provides process construction, validation, and rich Jupyter display.

use pyo3::prelude::*;

use spire_kernel::s_matrix;

use crate::errors::to_py_err;
use crate::model::PyModel;
use crate::particle::PyParticle;

/// A scattering or decay process: initial state → final state.
///
/// Bundles validity, conservation-law diagnostics, interaction types,
/// and mediating bosons identified by the SPIRE engine.
///
/// ```python
/// from spire import Model, Reaction
///
/// sm = Model.standard_model()
/// rxn = Reaction.create(sm, ["e-", "e+"], ["mu-", "mu+"], cms_energy=91.2)
/// print(rxn.is_valid)
/// print(rxn.interaction_types)
/// ```
#[pyclass(name = "Reaction")]
#[derive(Debug, Clone)]
pub struct PyReaction {
    pub(crate) inner: s_matrix::Reaction,
    /// Keep the JSON serialization cached for `to_json()`.
    json_cache: String,
}

#[pymethods]
impl PyReaction {
    /// Construct and validate a reaction.
    ///
    /// Parameters
    /// ----------
    /// model : Model
    ///     The theoretical model providing field definitions.
    /// initial : list[str]
    ///     Particle identifiers for the initial state (e.g. ``["e-", "e+"]``).
    /// final_state : list[str]
    ///     Particle identifiers for the final state (e.g. ``["mu-", "mu+"]``).
    /// cms_energy : float, optional
    ///     Centre-of-mass energy in GeV (default: 0).
    #[staticmethod]
    #[pyo3(signature = (model, initial, final_state, cms_energy=None))]
    fn create(
        model: &PyModel,
        initial: Vec<String>,
        final_state: Vec<String>,
        cms_energy: Option<f64>,
    ) -> PyResult<Self> {
        let init_refs: Vec<&str> = initial.iter().map(|s| s.as_str()).collect();
        let final_refs: Vec<&str> = final_state.iter().map(|s| s.as_str()).collect();

        let reaction =
            s_matrix::construct_reaction(&init_refs, &final_refs, &model.inner, cms_energy)
                .map_err(to_py_err)?;

        let json = serde_json::to_string(&reaction)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(Self {
            inner: reaction,
            json_cache: json,
        })
    }

    // ── Properties ───────────────────────────────────────────────────

    /// Whether the reaction passes all conservation-law checks.
    #[getter]
    fn is_valid(&self) -> bool {
        self.inner.is_valid
    }

    /// Violation diagnostics (empty list if valid).
    #[getter]
    fn violations(&self) -> Vec<String> {
        self.inner.violation_diagnostics.clone()
    }

    /// Interaction types that can mediate this process (e.g. ``["Electromagnetic"]``).
    #[getter]
    fn interaction_types(&self) -> Vec<String> {
        self.inner
            .interaction_types
            .iter()
            .map(|it| format!("{:?}", it))
            .collect()
    }

    /// Mediating gauge bosons.
    #[getter]
    fn mediating_bosons(&self) -> Vec<PyParticle> {
        self.inner
            .mediating_bosons
            .iter()
            .map(|p| PyParticle { inner: p.clone() })
            .collect()
    }

    /// Number of initial-state particles.
    #[getter]
    fn num_initial(&self) -> usize {
        self.inner.initial.states.len()
    }

    /// Number of final-state particles.
    #[getter]
    fn num_final(&self) -> usize {
        self.inner.final_state.states.len()
    }

    /// Serialised JSON of the full reaction.
    fn to_json(&self) -> String {
        self.json_cache.clone()
    }

    // ── Display ──────────────────────────────────────────────────────

    fn __repr__(&self) -> String {
        let initial: Vec<&str> = self
            .inner
            .initial
            .states
            .iter()
            .map(|s| s.particle.field.symbol.as_str())
            .collect();
        let final_st: Vec<&str> = self
            .inner
            .final_state
            .states
            .iter()
            .map(|s| s.particle.field.symbol.as_str())
            .collect();
        format!(
            "Reaction({} → {}, valid={})",
            initial.join(" "),
            final_st.join(" "),
            self.inner.is_valid,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let initial: Vec<&str> = self
            .inner
            .initial
            .states
            .iter()
            .map(|s| s.particle.field.symbol.as_str())
            .collect();
        let final_st: Vec<&str> = self
            .inner
            .final_state
            .states
            .iter()
            .map(|s| s.particle.field.symbol.as_str())
            .collect();

        let validity = if self.inner.is_valid {
            "<span style=\"color:green;\">✅ Valid</span>"
        } else {
            "<span style=\"color:red;\">❌ Invalid</span>"
        };

        let interactions: Vec<String> = self
            .inner
            .interaction_types
            .iter()
            .map(|i| format!("{:?}", i))
            .collect();

        let mut html = format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">⚛️ {} → {}</h4>\
             <p style=\"margin:2px 0;\">Status: {}</p>\
             <p style=\"margin:2px 0;\">Interactions: {}</p>",
            initial.join(" "),
            final_st.join(" "),
            validity,
            interactions.join(", "),
        );

        if !self.inner.violation_diagnostics.is_empty() {
            html.push_str("<p style=\"margin:2px 0; color:red;\">Violations:<ul>");
            for v in &self.inner.violation_diagnostics {
                html.push_str(&format!("<li>{}</li>", v));
            }
            html.push_str("</ul></p>");
        }

        if !self.inner.mediating_bosons.is_empty() {
            html.push_str("<p style=\"margin:2px 0;\">Mediators: ");
            let bosons: Vec<&str> = self
                .inner
                .mediating_bosons
                .iter()
                .map(|b| b.field.symbol.as_str())
                .collect();
            html.push_str(&bosons.join(", "));
            html.push_str("</p>");
        }

        html.push_str("</div>");
        html
    }
}
