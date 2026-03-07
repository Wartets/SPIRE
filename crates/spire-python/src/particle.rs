//! `Field` and `Particle` - Python wrappers around ontology types.

use pyo3::prelude::*;

use spire_kernel::ontology;

// ===========================================================================
// PyField
// ===========================================================================

/// A quantum field definition - the fundamental building block of a model.
///
/// Fields carry intrinsic properties (mass, spin, gauge representations)
/// and serve as the building blocks of the Lagrangian.
///
/// Obtain `Field` objects from a `Model`:
///
/// ```python
/// from spire import Model
/// sm = Model.standard_model()
/// electron = sm.get_field("e-")
/// print(electron.mass, electron.spin)
/// ```
#[pyclass(name = "Field")]
#[derive(Debug, Clone)]
pub struct PyField {
    pub(crate) inner: ontology::Field,
}

#[pymethods]
impl PyField {
    /// Unique string identifier (e.g. ``"e-"``, ``"photon"``).
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Human-readable name (e.g. ``"Electron"``).
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// LaTeX symbol (e.g. ``"e^-"``).
    #[getter]
    fn symbol(&self) -> &str {
        &self.inner.symbol
    }

    /// Rest mass in GeV/c².
    #[getter]
    fn mass(&self) -> f64 {
        self.inner.mass
    }

    /// Total decay width in GeV.
    #[getter]
    fn width(&self) -> f64 {
        self.inner.width
    }

    /// Spin as a float (0, 0.5, 1, 1.5, 2, …).
    #[getter]
    fn spin(&self) -> f64 {
        self.inner.quantum_numbers.spin.0 as f64 / 2.0
    }

    /// Electric charge in units of the positron charge *e*.
    #[getter]
    fn electric_charge(&self) -> f64 {
        self.inner.quantum_numbers.electric_charge.0 as f64 / 3.0
    }

    /// Baryon number *B*.
    #[getter]
    fn baryon_number(&self) -> f64 {
        self.inner.quantum_numbers.baryon_number.0 as f64 / 3.0
    }

    /// Lepton numbers ``(L_e, L_mu, L_tau)``.
    #[getter]
    fn lepton_numbers(&self) -> (i8, i8, i8) {
        let ln = &self.inner.quantum_numbers.lepton_numbers;
        (ln.electron, ln.muon, ln.tau)
    }

    /// Color representation as a string (``"Singlet"``, ``"Triplet"``, etc.).
    #[getter]
    fn color(&self) -> String {
        format!("{:?}", self.inner.quantum_numbers.color)
    }

    /// Interaction types this field participates in.
    #[getter]
    fn interactions(&self) -> Vec<String> {
        self.inner
            .interactions
            .iter()
            .map(|i| format!("{:?}", i))
            .collect()
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Field(id='{}', name='{}', mass={:.6} GeV, spin={})",
            self.inner.id,
            self.inner.name,
            self.inner.mass,
            self.spin(),
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let spin_str = if self.spin().fract() == 0.0 {
            format!("{}", self.spin() as i32)
        } else {
            format!("{}/2", self.inner.quantum_numbers.spin.0)
        };

        format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <b>{}</b> ({}) - <i>{}</i>\
             <table style=\"border-collapse:collapse; margin-top:4px;\">\
             <tr><td style=\"padding:2px 8px;\">Mass</td>\
             <td style=\"padding:2px 8px;\">{:.6} GeV</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Spin</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Charge</td>\
             <td style=\"padding:2px 8px;\">{:+.2}e</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Color</td>\
             <td style=\"padding:2px 8px;\">{:?}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Width</td>\
             <td style=\"padding:2px 8px;\">{:.4e} GeV</td></tr>\
             </table></div>",
            self.inner.name,
            self.inner.symbol,
            self.inner.id,
            self.inner.mass,
            spin_str,
            self.electric_charge(),
            self.inner.quantum_numbers.color,
            self.inner.width,
        )
    }
}

// ===========================================================================
// PyParticle
// ===========================================================================

/// A concrete particle - an on-shell or off-shell excitation of a `Field`.
///
/// Carries kinematic (shell condition, helicity, chirality) and
/// anti-particle information on top of the underlying `Field`.
#[pyclass(name = "Particle")]
#[derive(Debug, Clone)]
pub struct PyParticle {
    pub(crate) inner: ontology::Particle,
}

#[pymethods]
impl PyParticle {
    /// The underlying field definition.
    #[getter]
    fn field(&self) -> PyField {
        PyField {
            inner: self.inner.field.clone(),
        }
    }

    /// Whether the particle is on-shell (``True``) or off-shell (``False``).
    #[getter]
    fn is_on_shell(&self) -> bool {
        matches!(self.inner.shell, ontology::ShellCondition::OnShell)
    }

    /// Whether this is an antiparticle.
    #[getter]
    fn is_anti(&self) -> bool {
        self.inner.is_anti
    }

    /// Helicity as a string (``"Plus"``, ``"Minus"``, or ``None``).
    #[getter]
    fn helicity(&self) -> Option<String> {
        self.inner.helicity.map(|h| format!("{:?}", h))
    }

    /// Chirality as a string (``"Left"``, ``"Right"``, or ``None``).
    #[getter]
    fn chirality(&self) -> Option<String> {
        self.inner.chirality.map(|c| format!("{:?}", c))
    }

    fn __repr__(&self) -> String {
        let anti_str = if self.inner.is_anti { "anti-" } else { "" };
        format!("Particle({}{})", anti_str, self.inner.field.name,)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let anti = if self.inner.is_anti { "anti-" } else { "" };
        let shell = if self.is_on_shell() {
            "on-shell"
        } else {
            "off-shell"
        };
        format!(
            "<span style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <b>{}{}</b> ({}) - {} | mass = {:.6} GeV\
             </span>",
            anti, self.inner.field.name, self.inner.field.symbol, shell, self.inner.field.mass,
        )
    }
}
