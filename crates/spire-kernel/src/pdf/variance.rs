//! PDF uncertainty variance engine.
//!
//! This module provides reusable, observable-agnostic uncertainty
//! primitives for PDF replica and Hessian sets, plus quadrature
//! composition with other uncertainty sources.

use serde::{Deserialize, Serialize};

use crate::{SpireError, SpireResult};

/// PDF uncertainty model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PdfUncertaintyMode {
    /// Monte-Carlo replica sets (e.g. NNPDF).
    MonteCarloReplicas,
    /// Hessian eigenvector sets (e.g. CT-style sets).
    Hessian,
}

/// Output of a PDF uncertainty calculation for one observable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfUncertaintyBand {
    /// Central prediction per bin.
    pub central: Vec<f64>,
    /// Symmetric PDF standard deviation per bin.
    pub sigma_pdf: Vec<f64>,
    /// Upper envelope `central + sigma_pdf`.
    pub up: Vec<f64>,
    /// Lower envelope `max(0, central - sigma_pdf)`.
    pub down: Vec<f64>,
    /// Model used for uncertainty extraction.
    pub mode: PdfUncertaintyMode,
    /// Number of sets used (replicas or Hessian pairs).
    pub set_count: usize,
}

/// Compute PDF uncertainty from Monte-Carlo replicas.
///
/// `replicas[replica_idx][bin_idx]`.
pub fn mc_replica_uncertainty(replicas: &[Vec<f64>]) -> SpireResult<PdfUncertaintyBand> {
    if replicas.is_empty() {
        return Err(SpireError::DataParseError(
            "MC replica set is empty".to_string(),
        ));
    }

    let n_bins = replicas[0].len();
    if n_bins == 0 {
        return Err(SpireError::DataParseError(
            "MC replica bins are empty".to_string(),
        ));
    }

    if replicas.iter().any(|r| r.len() != n_bins) {
        return Err(SpireError::DataParseError(
            "Inconsistent bin count across MC replicas".to_string(),
        ));
    }

    let n_rep = replicas.len() as f64;
    let mut central = vec![0.0; n_bins];
    for rep in replicas {
        for (i, &v) in rep.iter().enumerate() {
            central[i] += v;
        }
    }
    for c in &mut central {
        *c /= n_rep;
    }

    let mut sigma_pdf = vec![0.0; n_bins];
    for rep in replicas {
        for (i, &v) in rep.iter().enumerate() {
            let d = v - central[i];
            sigma_pdf[i] += d * d;
        }
    }
    for s in &mut sigma_pdf {
        *s = (*s / n_rep.max(1.0)).sqrt();
    }

    let up: Vec<f64> = central
        .iter()
        .zip(sigma_pdf.iter())
        .map(|(c, s)| c + s)
        .collect();
    let down: Vec<f64> = central
        .iter()
        .zip(sigma_pdf.iter())
        .map(|(c, s)| (c - s).max(0.0))
        .collect();

    Ok(PdfUncertaintyBand {
        central,
        sigma_pdf,
        up,
        down,
        mode: PdfUncertaintyMode::MonteCarloReplicas,
        set_count: replicas.len(),
    })
}

/// Compute PDF uncertainty from Hessian eigenvector pairs.
///
/// Implements:
/// `ΔX = 1/2 * sqrt(sum_i (X_i^+ - X_i^-)^2)`
pub fn hessian_uncertainty(
    central: &[f64],
    eigenvector_pairs: &[(Vec<f64>, Vec<f64>)],
) -> SpireResult<PdfUncertaintyBand> {
    if central.is_empty() {
        return Err(SpireError::DataParseError(
            "Hessian central prediction is empty".to_string(),
        ));
    }
    if eigenvector_pairs.is_empty() {
        return Err(SpireError::DataParseError(
            "Hessian eigenvector set is empty".to_string(),
        ));
    }

    let n_bins = central.len();
    if eigenvector_pairs
        .iter()
        .any(|(plus, minus)| plus.len() != n_bins || minus.len() != n_bins)
    {
        return Err(SpireError::DataParseError(
            "Inconsistent bin count in Hessian eigenvectors".to_string(),
        ));
    }

    let mut sigma_pdf = vec![0.0; n_bins];
    for (plus, minus) in eigenvector_pairs {
        for i in 0..n_bins {
            let d = plus[i] - minus[i];
            sigma_pdf[i] += d * d;
        }
    }

    for s in &mut sigma_pdf {
        *s = 0.5 * s.sqrt();
    }

    let up: Vec<f64> = central
        .iter()
        .zip(sigma_pdf.iter())
        .map(|(c, s)| c + s)
        .collect();
    let down: Vec<f64> = central
        .iter()
        .zip(sigma_pdf.iter())
        .map(|(c, s)| (c - s).max(0.0))
        .collect();

    Ok(PdfUncertaintyBand {
        central: central.to_vec(),
        sigma_pdf,
        up,
        down,
        mode: PdfUncertaintyMode::Hessian,
        set_count: eigenvector_pairs.len(),
    })
}

/// Combine independent uncertainty components in quadrature.
///
/// `σ_total = sqrt(σ_scale² + σ_pdf² + σ_mc_stats²)`
pub fn combine_uncertainties_quadrature(
    central: &[f64],
    sigma_scale: &[f64],
    sigma_pdf: &[f64],
    sigma_mc_stats: &[f64],
) -> SpireResult<(Vec<f64>, Vec<f64>)> {
    let n = central.len();
    if sigma_scale.len() != n || sigma_pdf.len() != n || sigma_mc_stats.len() != n {
        return Err(SpireError::DataParseError(
            "Quadrature inputs have inconsistent bin lengths".to_string(),
        ));
    }

    let mut up = Vec::with_capacity(n);
    let mut down = Vec::with_capacity(n);

    for i in 0..n {
        let st = sigma_scale[i].max(0.0);
        let sp = sigma_pdf[i].max(0.0);
        let sm = sigma_mc_stats[i].max(0.0);
        let sigma = (st * st + sp * sp + sm * sm).sqrt();
        up.push(central[i] + sigma);
        down.push((central[i] - sigma).max(0.0));
    }

    Ok((up, down))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mc_replica_sigma_nonzero() {
        let reps = vec![vec![10.0, 20.0], vec![11.0, 21.0], vec![9.0, 19.0]];
        let out = mc_replica_uncertainty(&reps).unwrap();
        assert_eq!(out.central.len(), 2);
        assert!(out.sigma_pdf[0] > 0.0);
        assert!(out.up[0] >= out.central[0]);
        assert!(out.down[0] <= out.central[0]);
    }

    #[test]
    fn hessian_formula_matches_expected() {
        let central = vec![100.0, 200.0];
        let pairs = vec![
            (vec![102.0, 198.0], vec![98.0, 202.0]),
            (vec![101.0, 201.0], vec![99.0, 199.0]),
        ];
        let out = hessian_uncertainty(&central, &pairs).unwrap();

        let expected0 = 0.5 * ((4.0_f64).powi(2) + (2.0_f64).powi(2)).sqrt();
        assert!((out.sigma_pdf[0] - expected0).abs() < 1e-12);
    }

    #[test]
    fn quadrature_combines_components() {
        let central = vec![10.0];
        let (up, down) =
            combine_uncertainties_quadrature(&central, &[3.0], &[4.0], &[12.0]).unwrap();
        // sqrt(9 + 16 + 144) = 13
        assert!((up[0] - 23.0).abs() < 1e-12);
        assert!((down[0] - 0.0).abs() < 1e-12);
    }
}
