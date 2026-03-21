//! Error mapping from `SpireError` to Python exceptions.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::PyErr;

use spire_kernel::SpireError;

/// Convert a [`SpireError`] into the appropriate Python exception.
///
/// Physics-facing errors (conservation violations, kinematics, unknown
/// particles, invalid vertices, model parsing, group theory) map to
/// `ValueError` since they represent invalid user input.  Internal /
/// algebra errors map to `RuntimeError` since they indicate computational
/// failures.
pub fn to_py_err(e: SpireError) -> PyErr {
    match e {
        SpireError::ConservationViolation(msg) => {
            PyValueError::new_err(format!("Conservation violation: {msg}"))
        }
        SpireError::KinematicsForbidden(msg) => {
            PyValueError::new_err(format!("Kinematics forbidden: {msg}"))
        }
        SpireError::UnknownParticle(msg) => {
            PyValueError::new_err(format!("Unknown particle: {msg}"))
        }
        SpireError::InvalidVertex(msg) => PyValueError::new_err(format!("Invalid vertex: {msg}")),
        SpireError::ModelParseError(msg) => {
            PyValueError::new_err(format!("Model parse error: {msg}"))
        }
        SpireError::GroupTheoryError(msg) => {
            PyValueError::new_err(format!("Group theory error: {msg}"))
        }
        SpireError::DataParseError(msg) => {
            PyValueError::new_err(format!("Data parse error: {msg}"))
        }
        SpireError::DataMismatch(msg) => PyValueError::new_err(format!("Data mismatch: {msg}")),
        SpireError::DatabaseError(msg) => PyRuntimeError::new_err(format!("Database error: {msg}")),
        SpireError::EditionMismatch {
            locked_edition,
            incoming_edition,
            ..
        } => {
            PyValueError::new_err(format!(
                "Edition mismatch: expected {locked_edition}, found {incoming_edition}"
            ))
        }
        SpireError::ProvenanceMismatch {
            saved_edition,
            local_edition,
            saved_fingerprint,
            local_fingerprint,
            remediation_options,
            reason,
        } => PyValueError::new_err(format!(
            "Provenance mismatch: saved edition '{saved_edition}', local edition '{local_edition}', saved fingerprint {:?}, local fingerprint {:?}. {}. Options: {}",
            saved_fingerprint,
            local_fingerprint,
            reason,
            remediation_options.join(", ")
        )),
        SpireError::AlgebraError(msg) => PyRuntimeError::new_err(format!("Algebra error: {msg}")),
        SpireError::InternalError(msg) => PyRuntimeError::new_err(format!("Internal error: {msg}")),
    }
}
