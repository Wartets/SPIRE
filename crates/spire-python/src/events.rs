//! `EventGenerator` - Zero-copy NumPy phase-space event bridge.
//!
//! Wraps the RAMBO phase-space generator and returns events as contiguous
//! NumPy arrays with zero Python-side allocation overhead.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use numpy::{PyArray1, PyArray2, PyArray3};

use spire_kernel::kinematics::{PhaseSpaceGenerator, RamboGenerator};

use crate::errors::to_py_err;

/// A phase-space event generator backed by the RAMBO algorithm.
///
/// Generates Lorentz-invariant N-body phase-space points and returns them
/// as zero-copy NumPy arrays.
///
/// ```python
/// from spire import EventGenerator
/// import numpy as np
///
/// gen = EventGenerator(cms_energy=91.2, final_masses=[0.1057, 0.1057], seed=42)
/// momenta, weights = gen.generate(100_000)
/// print(momenta.shape)   # (100000, 2, 4)
/// print(weights.shape)   # (100000,)
///
/// # Flat arrays for ML pipelines:
/// flat = gen.generate_flat(100_000)
/// print(flat.shape)      # (100000, 8)
/// ```
#[pyclass(name = "EventGenerator")]
pub struct PyEventGenerator {
    cms_energy: f64,
    final_masses: Vec<f64>,
    generator: RamboGenerator,
}

#[pymethods]
impl PyEventGenerator {
    /// Create a new event generator.
    ///
    /// Parameters
    /// ----------
    /// cms_energy : float
    ///     Centre-of-mass energy √s in GeV.
    /// final_masses : list[float]
    ///     Final-state particle rest masses in GeV.
    /// seed : int, optional
    ///     RNG seed for reproducible generation.
    #[new]
    #[pyo3(signature = (cms_energy, final_masses, seed=None))]
    fn new(cms_energy: f64, final_masses: Vec<f64>, seed: Option<u64>) -> PyResult<Self> {
        if final_masses.is_empty() {
            return Err(PyValueError::new_err(
                "At least one final-state particle mass is required",
            ));
        }
        if cms_energy <= 0.0 {
            return Err(PyValueError::new_err(
                "Centre-of-mass energy must be positive",
            ));
        }

        let generator = match seed {
            Some(s) => RamboGenerator::with_seed(s),
            None => RamboGenerator::new(),
        };

        Ok(Self {
            cms_energy,
            final_masses,
            generator,
        })
    }

    // ── Properties ───────────────────────────────────────────────────

    /// Centre-of-mass energy in GeV.
    #[getter]
    fn cms_energy(&self) -> f64 {
        self.cms_energy
    }

    /// Final-state particle masses in GeV.
    #[getter]
    fn final_masses(&self) -> Vec<f64> {
        self.final_masses.clone()
    }

    /// Number of final-state particles.
    #[getter]
    fn num_particles(&self) -> usize {
        self.final_masses.len()
    }

    // ── Generation ───────────────────────────────────────────────────

    /// Generate events and return structured NumPy arrays.
    ///
    /// Returns
    /// -------
    /// momenta : numpy.ndarray, shape ``(num_events, num_particles, 4)``
    ///     4-momenta in order ``(E, px, py, pz)`` with ``dtype=float64``.
    /// weights : numpy.ndarray, shape ``(num_events,)``
    ///     Lorentz-invariant phase-space weight for each event.
    #[pyo3(signature = (num_events))]
    fn generate<'py>(
        &mut self,
        py: Python<'py>,
        num_events: usize,
    ) -> PyResult<(&'py PyArray3<f64>, &'py PyArray1<f64>)> {
        if num_events == 0 {
            return Err(PyValueError::new_err("Number of events must be positive"));
        }

        let n_particles = self.final_masses.len();
        let momenta_array = PyArray3::<f64>::zeros(py, [num_events, n_particles, 4], false);
        let weights_array = PyArray1::<f64>::zeros(py, [num_events], false);

        // SAFETY: Arrays just allocated; exclusive access guaranteed; indices in-bounds.
        unsafe {
            let momenta_slice = momenta_array
                .as_slice_mut()
                .map_err(|e| PyRuntimeError::new_err(format!("NumPy buffer error: {e}")))?;
            let weights_slice = weights_array
                .as_slice_mut()
                .map_err(|e| PyRuntimeError::new_err(format!("NumPy buffer error: {e}")))?;

            for i in 0..num_events {
                let event = self
                    .generator
                    .generate_event(self.cms_energy, &self.final_masses)
                    .map_err(to_py_err)?;

                weights_slice[i] = event.weight;

                for (j, momentum) in event.momenta.iter().enumerate() {
                    let base = i * n_particles * 4 + j * 4;
                    let comps = momentum.components();
                    let n_copy = comps.len().min(4);
                    momenta_slice[base..base + n_copy].copy_from_slice(&comps[..n_copy]);
                    for k in n_copy..4 {
                        momenta_slice[base + k] = 0.0;
                    }
                }
            }
        }

        Ok((momenta_array, weights_array))
    }

    /// Generate events and return a flat 2-D NumPy array.
    ///
    /// Each row contains all 4-momentum components concatenated:
    /// ``[E1, px1, py1, pz1, E2, px2, py2, pz2, …]``.
    ///
    /// This layout is ideal for ML pipelines and DataFrames.
    ///
    /// Returns
    /// -------
    /// events : numpy.ndarray, shape ``(num_events, num_particles * 4)``
    ///     Flat event matrix with ``dtype=float64``.
    #[pyo3(signature = (num_events))]
    fn generate_flat<'py>(
        &mut self,
        py: Python<'py>,
        num_events: usize,
    ) -> PyResult<&'py PyArray2<f64>> {
        if num_events == 0 {
            return Err(PyValueError::new_err("Number of events must be positive"));
        }

        let n_particles = self.final_masses.len();
        let cols = n_particles * 4;
        let flat_array = PyArray2::<f64>::zeros(py, [num_events, cols], false);

        unsafe {
            let slice = flat_array
                .as_slice_mut()
                .map_err(|e| PyRuntimeError::new_err(format!("NumPy buffer error: {e}")))?;

            for i in 0..num_events {
                let event = self
                    .generator
                    .generate_event(self.cms_energy, &self.final_masses)
                    .map_err(to_py_err)?;

                for (j, momentum) in event.momenta.iter().enumerate() {
                    let base = i * cols + j * 4;
                    let comps = momentum.components();
                    let n_copy = comps.len().min(4);
                    slice[base..base + n_copy].copy_from_slice(&comps[..n_copy]);
                    for k in n_copy..4 {
                        slice[base + k] = 0.0;
                    }
                }
            }
        }

        Ok(flat_array)
    }

    // ── Display ──────────────────────────────────────────────────────

    fn __repr__(&self) -> String {
        format!(
            "EventGenerator(√s={:.2} GeV, particles={}, masses={:?})",
            self.cms_energy,
            self.final_masses.len(),
            self.final_masses,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Jupyter rich HTML representation.
    fn _repr_html_(&self) -> String {
        let masses_str: Vec<String> = self
            .final_masses
            .iter()
            .map(|m| format!("{m:.4}"))
            .collect();
        format!(
            "<div style=\"font-family:'Segoe UI',sans-serif; font-size:13px;\">\
             <h4 style=\"margin:0 0 6px 0;\">🎲 Event Generator (RAMBO)</h4>\
             <table style=\"border-collapse:collapse;\">\
             <tr><td style=\"padding:2px 8px;\">√s</td>\
             <td style=\"padding:2px 8px;\">{:.4} GeV</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Particles</td>\
             <td style=\"padding:2px 8px;\">{}</td></tr>\
             <tr><td style=\"padding:2px 8px;\">Masses (GeV)</td>\
             <td style=\"padding:2px 8px;\">[{}]</td></tr>\
             </table></div>",
            self.cms_energy,
            self.final_masses.len(),
            masses_str.join(", "),
        )
    }
}
