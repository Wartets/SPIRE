//! Kinematics utility functions exposed as module-level Python functions.

use pyo3::prelude::*;

use spire_kernel::kinematics;

use crate::errors::to_py_err;

/// Calculate the threshold energy for a set of final-state masses.
///
/// Parameters
/// ----------
/// final_masses : list[float]
///     Final-state particle masses in GeV.
/// target_mass : float, optional
///     Target mass for fixed-target lab energy calculation.
///
/// Returns
/// -------
/// dict
///     A dictionary with keys ``threshold_energy`` (float),
///     ``lab_energy`` (float or None), and ``final_masses`` (list[float]).
#[pyfunction]
#[pyo3(signature = (final_masses, target_mass=None))]
pub fn calculate_threshold(
    final_masses: Vec<f64>,
    target_mass: Option<f64>,
) -> PyResult<PyObject> {
    let result =
        kinematics::calculate_thresholds(&final_masses, target_mass).map_err(to_py_err)?;

    Python::with_gil(|py| {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("threshold_energy", result.threshold_energy)?;
        dict.set_item("lab_energy", result.lab_energy)?;
        dict.set_item("final_masses", result.final_masses)?;
        Ok(dict.into())
    })
}

/// Check whether a reaction is kinematically allowed at a given CM energy.
///
/// Parameters
/// ----------
/// cms_energy : float
///     Centre-of-mass energy √s in GeV.
/// final_masses : list[float]
///     Final-state particle masses in GeV.
///
/// Returns
/// -------
/// bool
///     ``True`` if the process is kinematically allowed.
#[pyfunction]
#[pyo3(signature = (cms_energy, final_masses))]
pub fn is_kinematically_allowed(cms_energy: f64, final_masses: Vec<f64>) -> bool {
    kinematics::is_kinematically_allowed(cms_energy, &final_masses)
}
