//! Multi-observable global goodness-of-fit aggregation.

use serde::{Deserialize, Serialize};

use crate::io::experimental::{
    compute_chi_square, ExperimentalDataSet, GoodnessOfFitResult,
};
use crate::{SpireError, SpireResult};

/// One observable payload for global-fit aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableFitInput {
    /// Observable label (e.g. pT(Z), m_ll).
    pub observable: String,
    /// Theory histogram bin values.
    pub theory_bin_contents: Vec<f64>,
    /// Theory histogram bin edges.
    pub theory_bin_edges: Vec<f64>,
    /// Experimental dataset CSV text.
    pub exp_csv: String,
}

/// One observable fit summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservableFitSummary {
    /// Observable label.
    pub observable: String,
    /// Fit result for this observable.
    pub result: GoodnessOfFitResult,
}

/// Aggregated global-fit result over multiple observables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFitResult {
    /// Per-observable summaries.
    pub observables: Vec<ObservableFitSummary>,
    /// Sum of individual chi-square values.
    pub chi_square_global: f64,
    /// Total degrees of freedom after parameter subtraction.
    pub ndf_total: usize,
    /// Number of fitted parameters subtracted from bin count.
    pub n_params: usize,
    /// Global reduced chi-square.
    pub chi_square_reduced: f64,
    /// Approximate global p-value.
    pub p_value: f64,
}

/// Compute a global χ² aggregation over multiple observables.
///
/// `χ²_global = Σ_i χ²_i`
///
/// `N_dof = Σ_i N_bins,i - N_params`
pub fn compute_global_fit(
    inputs: &[ObservableFitInput],
    n_params: usize,
) -> SpireResult<GlobalFitResult> {
    if inputs.is_empty() {
        return Err(SpireError::DataParseError(
            "Global fit requires at least one observable".to_string(),
        ));
    }

    let mut observables = Vec::with_capacity(inputs.len());
    let mut chi_square_global = 0.0;
    let mut ndf_raw = 0usize;

    for input in inputs {
        let exp = ExperimentalDataSet::from_csv(&input.exp_csv, input.observable.clone(), None)?;
        let result = compute_chi_square(&input.theory_bin_contents, &input.theory_bin_edges, &exp)?;
        ndf_raw += result.ndf;
        chi_square_global += result.chi_square;
        observables.push(ObservableFitSummary {
            observable: input.observable.clone(),
            result,
        });
    }

    let ndf_total = ndf_raw.saturating_sub(n_params);
    let chi_square_reduced = if ndf_total > 0 {
        chi_square_global / (ndf_total as f64)
    } else {
        f64::NAN
    };

    // Keep p-value approximation consistent with single-observable path.
    let p_value = if chi_square_reduced.is_finite() && chi_square_reduced > 0.0 {
        1.0 / (1.0 + chi_square_reduced)
    } else {
        0.0
    };

    Ok(GlobalFitResult {
        observables,
        chi_square_global,
        ndf_total,
        n_params,
        chi_square_reduced,
        p_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_fit_aggregation() {
        let inputs = vec![
            ObservableFitInput {
                observable: "pt".to_string(),
                theory_bin_contents: vec![10.0, 20.0],
                theory_bin_edges: vec![0.0, 1.0, 2.0],
                exp_csv: "x,y,dy\n0.5,10.0,1.0\n1.5,20.0,1.0".to_string(),
            },
            ObservableFitInput {
                observable: "mll".to_string(),
                theory_bin_contents: vec![5.0, 7.0],
                theory_bin_edges: vec![0.0, 1.0, 2.0],
                exp_csv: "x,y,dy\n0.5,5.0,1.0\n1.5,7.0,1.0".to_string(),
            },
        ];

        let out = compute_global_fit(&inputs, 1).unwrap();
        assert_eq!(out.observables.len(), 2);
        assert_eq!(out.ndf_total, 3);
        assert!(out.chi_square_global.abs() < 1e-12);
    }
}
