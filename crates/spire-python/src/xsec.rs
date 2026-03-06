//! Cross-section result wrappers.

use pyo3::prelude::*;

use spire_kernel::pdf;
use spire_kernel::s_matrix;

use crate::errors::to_py_err;

// ===========================================================================
// PyCrossSectionResult
// ===========================================================================

/// Result of a partonic cross-section calculation.
///
/// ```python
/// from spire import CrossSectionResult
/// # Typically obtained from the kernel:
/// # xsec = CrossSectionResult.calculate(100.0, [0.0, 0.0], 50000)
/// ```
#[pyclass(name = "CrossSectionResult")]
#[derive(Debug, Clone)]
pub struct PyCrossSectionResult {
    pub(crate) inner: s_matrix::CrossSectionResult,
}

#[pymethods]
impl PyCrossSectionResult {
    /// Calculate a partonic cross-section via flat Monte Carlo.
    ///
    /// Uses constant |M|² = 1 (phase-space only). For custom matrix elements,
    /// use `EventGenerator` and compute the weighted average in NumPy.
    ///
    /// Parameters
    /// ----------
    /// cms_energy : float
    ///     Centre-of-mass energy √s in GeV.
    /// final_masses : list[float]
    ///     Final-state particle masses in GeV.
    /// num_events : int
    ///     Number of Monte Carlo samples.
    #[staticmethod]
    fn calculate(cms_energy: f64, final_masses: Vec<f64>, num_events: usize) -> PyResult<Self> {
        let result =
            s_matrix::calculate_phase_space_cross_section(cms_energy, &final_masses, num_events)
                .map_err(to_py_err)?;
        Ok(Self { inner: result })
    }

    /// Cross-section in natural units (GeV⁻²).
    #[getter]
    fn cross_section(&self) -> f64 {
        self.inner.cross_section
    }

    /// Uncertainty in natural units (GeV⁻²).
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

    /// Number of Monte Carlo events evaluated.
    #[getter]
    fn events_evaluated(&self) -> usize {
        self.inner.events_evaluated
    }

    /// Relative error δσ/σ.
    #[getter]
    fn relative_error(&self) -> f64 {
        self.inner.relative_error
    }

    /// Centre-of-mass energy in GeV.
    #[getter]
    fn cms_energy(&self) -> f64 {
        self.inner.cms_energy
    }

    fn __repr__(&self) -> String {
        format!(
            "CrossSectionResult(σ={:.6e} pb ± {:.6e} pb, events={})",
            self.inner.cross_section_pb, self.inner.uncertainty_pb, self.inner.events_evaluated,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">📈 Cross Section</h4>\
             <table style=\"border-collapse:collapse;\">\
             <tr><td style=\"padding:2px 8px;\">σ</td>\
             <td style=\"padding:2px 8px;\">{:.6e} pb</td></tr>\
             <tr><td style=\"padding:2px 8px;\">δσ</td>\
             <td style=\"padding:2px 8px;\">± {:.6e} pb</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Rel. Error</td>\
             <td style=\"padding:2px 8px;\">{:.2}%</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Events</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">√s</td>\
             <td style=\"padding:2px 8px;\">{:.2} GeV</td></tr>\
             </table></div>",
            self.inner.cross_section_pb,
            self.inner.uncertainty_pb,
            self.inner.relative_error * 100.0,
            self.inner.events_evaluated,
            self.inner.cms_energy,
        )
    }
}

// ===========================================================================
// PyHadronicResult
// ===========================================================================

/// Result of a hadronic (PDF-convoluted) cross-section calculation.
#[pyclass(name = "HadronicCrossSectionResult")]
#[derive(Debug, Clone)]
pub struct PyHadronicResult {
    pub(crate) inner: pdf::HadronicCrossSectionResult,
}

#[pymethods]
impl PyHadronicResult {
    /// Calculate a hadronic cross-section for proton–proton collisions.
    ///
    /// Performs the PDF convolution with constant |M|² = 1.
    ///
    /// Parameters
    /// ----------
    /// beam_energy_sq : float
    ///     Hadronic S in GeV² (e.g. ``13000.0**2`` for 13 TeV LHC).
    /// final_masses : list[float]
    ///     Final-state particle masses in GeV.
    /// num_events : int
    ///     Number of Monte Carlo samples per parton channel.
    #[staticmethod]
    fn calculate(beam_energy_sq: f64, final_masses: Vec<f64>, num_events: usize) -> PyResult<Self> {
        let result = s_matrix::calculate_hadronic_phase_space_cross_section(
            beam_energy_sq,
            &final_masses,
            num_events,
        )
        .map_err(to_py_err)?;
        Ok(Self { inner: result })
    }

    #[getter]
    fn cross_section(&self) -> f64 {
        self.inner.cross_section
    }

    #[getter]
    fn uncertainty(&self) -> f64 {
        self.inner.uncertainty
    }

    #[getter]
    fn cross_section_pb(&self) -> f64 {
        self.inner.cross_section_pb
    }

    #[getter]
    fn uncertainty_pb(&self) -> f64 {
        self.inner.uncertainty_pb
    }

    #[getter]
    fn events_evaluated(&self) -> usize {
        self.inner.events_evaluated
    }

    #[getter]
    fn relative_error(&self) -> f64 {
        self.inner.relative_error
    }

    #[getter]
    fn beam_energy_sq(&self) -> f64 {
        self.inner.beam_energy_sq
    }

    #[getter]
    fn pdf_name(&self) -> &str {
        &self.inner.pdf_name
    }

    #[getter]
    fn beam_description(&self) -> &str {
        &self.inner.beam_description
    }

    fn __repr__(&self) -> String {
        format!(
            "HadronicCrossSectionResult(σ={:.6e} pb, beam='{}')",
            self.inner.cross_section_pb, self.inner.beam_description,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">📈 Hadronic Cross Section</h4>\
             <table style=\"border-collapse:collapse;\">\
             <tr><td style=\"padding:2px 8px;\">σ</td>\
             <td style=\"padding:2px 8px;\">{:.6e} pb</td></tr>\
             <tr><td style=\"padding:2px 8px;\">δσ</td>\
             <td style=\"padding:2px 8px;\">± {:.6e} pb</td></tr>\
             <tr><td style=\"padding:2px 8px;\">PDF</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Beam</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Events</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             </table></div>",
            self.inner.cross_section_pb,
            self.inner.uncertainty_pb,
            self.inner.pdf_name,
            self.inner.beam_description,
            self.inner.events_evaluated,
        )
    }
}
