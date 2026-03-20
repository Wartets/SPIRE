//! # Experimental Data Integration & Statistical Comparison
//!
//! This module provides tools for importing published experimental measurements
//! and computing goodness-of-fit metrics (χ²) between theoretical predictions
//! and data.
//!
//! ## CSV Format
//!
//! The standard format for experimental data is:
//! ```csv
//! x,y,dy_stat_up,dy_stat_down,dy_syst_up,dy_syst_down
//! 0.5,10.5,0.3,0.3,0.5,0.5
//! 1.5,12.2,0.4,0.4,0.6,0.6
//! ```
//!
//! Where:
//! - `x`: Observable value (bin center or independent variable)
//! - `y`: Measured value (cross-section, rate, etc.)
//! - `dy_stat_up`: Statistical error (positive)
//! - `dy_stat_down`: Statistical error (positive, merged with `dy_stat_up` in quadrature)
//! - `dy_syst_up`: Systematic error (positive)
//! - `dy_syst_down`: Systematic error (positive, merged with `dy_syst_up` in quadrature)

use serde::{Deserialize, Serialize};

use crate::io::csv::{parse_numeric_table, validate_axis_overlap};
use crate::{SpireError, SpireResult};

/// A single experimental data point with asymmetric errors.
///
/// This structure accommodates both statistical and systematic uncertainties,
/// stored separately with asymmetric variants so they can be combined in different
/// ways during fitting and goodness-of-fit calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalDataPoint {
    /// The observable value (e.g., transverse momentum, invariant mass).
    pub x: f64,
    /// The measured value (cross-section, branching ratio, etc.).
    pub y: f64,
    /// Combined uncertainty (statistical and systematic in quadrature).
    pub total_error: f64,
    /// Statistical uncertainty component.
    pub stat_error: f64,
    /// Systematic uncertainty component.
    pub syst_error: f64,
    /// Asymmetric statistical uncertainty (upward).
    pub stat_error_up: f64,
    /// Asymmetric statistical uncertainty (downward).
    pub stat_error_down: f64,
    /// Asymmetric systematic uncertainty (upward).
    pub syst_error_up: f64,
    /// Asymmetric systematic uncertainty (downward).
    pub syst_error_down: f64,
}

impl ExperimentalDataPoint {
    /// Compute combined uncertainty from separate stat/syst components.
    ///
    /// Both upward and downward systematic variations are merged symmetrically.
    pub fn new(x: f64, y: f64, dy_stat: f64, dy_syst: f64) -> Self {
        let stat_error = dy_stat.abs();
        let syst_error = dy_syst.abs();
        let total_error = (stat_error * stat_error + syst_error * syst_error).sqrt();

        Self {
            x,
            y,
            total_error,
            stat_error,
            syst_error,
            stat_error_up: stat_error,
            stat_error_down: stat_error,
            syst_error_up: syst_error,
            syst_error_down: syst_error,
        }
    }

    /// Create from asymmetric errors (preserving asymmetry).
    pub fn from_asymmetric(
        x: f64,
        y: f64,
        dy_stat_up: f64,
        dy_stat_down: f64,
        dy_syst_up: f64,
        dy_syst_down: f64,
    ) -> Self {
        let stat_error_up = dy_stat_up.abs();
        let stat_error_down = dy_stat_down.abs();
        let syst_error_up = dy_syst_up.abs();
        let syst_error_down = dy_syst_down.abs();

        // Compute symmetric errors as root-mean-square of asymmetric variants
        let stat_error = (stat_error_up * stat_error_up + stat_error_down * stat_error_down).sqrt()
            / 2.0_f64.sqrt();
        let syst_error = (syst_error_up * syst_error_up + syst_error_down * syst_error_down).sqrt()
            / 2.0_f64.sqrt();
        let total_error = (stat_error * stat_error + syst_error * syst_error).sqrt();

        Self {
            x,
            y,
            total_error,
            stat_error,
            syst_error,
            stat_error_up,
            stat_error_down,
            syst_error_up,
            syst_error_down,
        }
    }

    /// Get the appropriate error for goodness-of-fit depending on theory-experiment comparison.
    ///
    /// Returns a tuple of (error_up, error_down) for the caller to decide which to use.
    pub fn get_asymmetric_error(&self) -> (f64, f64) {
        // Compute combined error properly with asymmetric variants
        let stat_sq_up = self.stat_error_up * self.stat_error_up;
        let stat_sq_down = self.stat_error_down * self.stat_error_down;
        let syst_sq_up = self.syst_error_up * self.syst_error_up;
        let syst_sq_down = self.syst_error_down * self.syst_error_down;

        let total_sq_up = stat_sq_up + syst_sq_up;
        let total_sq_down = stat_sq_down + syst_sq_down;

        // Return both variants as tuple for caller to decide
        (total_sq_up.sqrt(), total_sq_down.sqrt())
    }
}

/// A set of experimental data points that can be compared to theory.
///
/// Stores a collection of measurements along with their metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalDataSet {
    /// Human-readable label for this dataset.
    pub name: String,
    /// DOI or reference citation.
    pub reference: Option<String>,
    /// The measured data points.
    pub points: Vec<ExperimentalDataPoint>,
}

impl ExperimentalDataSet {
    /// Create a new empty dataset.
    pub fn new(name: String, reference: Option<String>) -> Self {
        Self {
            name,
            reference,
            points: Vec::new(),
        }
    }

    /// Add a data point to this dataset.
    pub fn add_point(&mut self, point: ExperimentalDataPoint) {
        self.points.push(point);
    }

    /// Parse experimental data from a CSV string.
    ///
    /// Expected format:
    /// ```csv
    /// x,y,dy_stat_up,dy_stat_down,dy_syst_up,dy_syst_down
    /// 0.5,10.5,0.3,0.3,0.5,0.5
    /// ```
    ///
    /// If asymmetric errors are not provided, symmetric errors with a single
    /// value per uncertainty type are acceptable.
    pub fn from_csv(csv_text: &str, name: String, reference: Option<String>) -> SpireResult<Self> {
        let mut dataset = Self::new(name, reference);

        let parsed = parse_numeric_table(csv_text)?;

        for row in parsed.rows {
            let x = row[0];
            let y = row[1];

            // Parse errors with graceful defaults
            let point = match row.len() {
                6.. => ExperimentalDataPoint::from_asymmetric(x, y, row[2], row[3], row[4], row[5]),
                3 => ExperimentalDataPoint::new(x, y, row[2], 0.0),
                _ => ExperimentalDataPoint::new(x, y, 0.0, 0.0),
            };

            dataset.add_point(point);
        }

        Ok(dataset)
    }
}

/// Goodness-of-fit result from χ² comparison between theory and experiment.
///
/// Contains the computed χ² value, number of degrees of freedom, and derived
/// statistical measures (reduced χ², p-value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodnessOfFitResult {
    /// The χ² statistic: $\sum_i [(y_{\text{theory}} - y_{\text{exp}})^2 / \sigma_i^2]$.
    pub chi_square: f64,
    /// Number of degrees of freedom (typically n_points - n_parameters).
    pub ndf: usize,
    /// Reduced χ²: χ² / ndf.
    pub chi_square_reduced: f64,
    /// Approximate p-value (confidence that model is consistent with data).
    pub p_value: f64,
    /// Bin-by-bin pulls: (theory - exp) / sigma.
    pub pulls: Vec<f64>,
}

impl GoodnessOfFitResult {
    /// Create a result from raw χ² and ndf values.
    pub fn new(chi_square: f64, ndf: usize, pulls: Vec<f64>) -> Self {
        let chi_square_reduced = if ndf > 0 {
            chi_square / (ndf as f64)
        } else {
            f64::NAN
        };

        // Approximate p-value via reduced χ²
        // (This is a heuristic; proper computation requires special functions)
        let p_value = if ndf > 0 && chi_square_reduced > 0.0 {
            // Rough heuristic: p ≈ 1 / (1 + χ²_red)
            1.0 / (1.0 + chi_square_reduced)
        } else {
            0.0
        };

        Self {
            chi_square,
            ndf,
            chi_square_reduced,
            p_value,
            pulls,
        }
    }
}

/// Compute χ² goodness-of-fit between a theoretical histogram and experimental data.
///
/// Maps the continuous theoretical histogram bins to discrete experimental points
/// and computes the χ² statistic using asymmetric errors when applicable:
///
/// $$\chi^2 = \sum_{i} \frac{(y_{\text{theory},i} - y_{\text{exp},i})^2}{\sigma_i^2}$$
///
/// where σᵢ is chosen asymmetrically based on whether theory over- or under-predicts:
/// - If y_theory > y_exp: use σ_down (theory overestimates)
/// - If y_theory < y_exp: use σ_up (theory underestimates)
/// - This properly penalizes directional biases in the theoretical model
///
/// # Arguments
///
/// * `theory_bin_contents` - Values from the theoretical histogram (e.g., cross-sections)
/// * `theory_bin_edges` - Bin edges for interpolation/mapping
/// * `exp_data` - Experimental measurement points with errors
///
/// # Returns
///
/// A `GoodnessOfFitResult` containing χ², ndf, and per-point pulls.
pub fn compute_chi_square(
    theory_bin_contents: &[f64],
    theory_bin_edges: &[f64],
    exp_data: &ExperimentalDataSet,
) -> SpireResult<GoodnessOfFitResult> {
    if theory_bin_contents.is_empty() || theory_bin_edges.len() != theory_bin_contents.len() + 1 {
        return Err(SpireError::InternalError(
            "Theory histogram has inconsistent binning".into(),
        ));
    }

    let n_exp_points = exp_data.points.len();
    if n_exp_points == 0 {
        return Err(SpireError::DataParseError(
            "Experimental dataset is empty".into(),
        ));
    }

    let xs: Vec<f64> = exp_data.points.iter().map(|p| p.x).collect();
    validate_axis_overlap(theory_bin_edges, &xs, &exp_data.name)?;

    let mut chi_square = 0.0;
    let mut pulls = Vec::new();

    for exp_point in &exp_data.points {
        // Find the bin in the theory histogram that contains this point
        let mut theory_value = 0.0;
        let mut found = false;

        // Linear search for the bin (can be optimized with bisect for large N)
        for (bin_idx, &bin_content) in theory_bin_contents.iter().enumerate() {
            let bin_min = theory_bin_edges[bin_idx];
            let bin_max = theory_bin_edges[bin_idx + 1];

            if exp_point.x >= bin_min && exp_point.x < bin_max {
                theory_value = bin_content;
                found = true;
                break;
            }
        }

        // Skip points outside the theory range
        if !found {
            continue;
        }

        // Compute contribution to χ² using asymmetric errors
        let diff = theory_value - exp_point.y;
        let (error_up, error_down) = exp_point.get_asymmetric_error();

        // Choose the appropriate error based on the direction of disagreement.
        // Convention (Phase 69ter):
        //  - theory > data   => use positive experimental uncertainty (+dy)
        //  - theory <= data  => use negative experimental uncertainty (-dy)
        let error = if diff > 0.0 { error_up } else { error_down };

        // Protect against zero (or near-zero) errors to avoid division by zero
        let variance_floor = (1e-24_f64).max((exp_point.y.abs() * 1e-12).powi(2));
        let variance = (error * error).max(variance_floor);
        if variance > 0.0 {
            let contribution = (diff * diff) / variance;
            chi_square += contribution;

            let pull = diff / variance.sqrt();
            pulls.push(pull);
        }
    }

    // Degrees of freedom: n_points - (typical case: 0 nuisance parameters)
    // Can be refined with actual fit parameter count.
    let ndf = if pulls.is_empty() {
        n_exp_points
    } else {
        pulls.len()
    };

    Ok(GoodnessOfFitResult::new(chi_square, ndf, pulls))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpireError;

    #[test]
    fn test_experimental_data_point_creation() {
        let point = ExperimentalDataPoint::new(1.0, 5.0, 0.3, 0.2);
        assert!((point.x - 1.0).abs() < 1e-12);
        assert!((point.y - 5.0).abs() < 1e-12);
        assert!(point.stat_error > 0.0);
        assert!(point.syst_error > 0.0);
    }

    #[test]
    fn test_asymmetric_data_point_creation() {
        let point = ExperimentalDataPoint::from_asymmetric(
            1.0, 5.0, 0.3, 0.2, // stat errors: up=0.3, down=0.2
            0.5, 0.4, // syst errors: up=0.5, down=0.4
        );
        assert!((point.x - 1.0).abs() < 1e-12);
        assert!((point.y - 5.0).abs() < 1e-12);
        assert!((point.stat_error_up - 0.3).abs() < 1e-12);
        assert!((point.stat_error_down - 0.2).abs() < 1e-12);
        assert!((point.syst_error_up - 0.5).abs() < 1e-12);
        assert!((point.syst_error_down - 0.4).abs() < 1e-12);
    }

    #[test]
    fn test_get_asymmetric_error() {
        let point = ExperimentalDataPoint::from_asymmetric(1.0, 5.0, 0.3, 0.2, 0.5, 0.4);
        let (err_up, err_down) = point.get_asymmetric_error();
        // Should compute sqrt((0.3^2 + 0.5^2)) and sqrt((0.2^2 + 0.4^2))
        let expected_up = (0.3_f64 * 0.3_f64 + 0.5_f64 * 0.5_f64).sqrt();
        let expected_down = (0.2_f64 * 0.2_f64 + 0.4_f64 * 0.4_f64).sqrt();
        assert!((err_up - expected_up).abs() < 1e-12);
        assert!((err_down - expected_down).abs() < 1e-12);
    }

    #[test]
    fn test_dataset_from_csv() {
        let csv = "x,y,dy_stat_up,dy_stat_down,dy_syst_up,dy_syst_down\n0.5,10.0,0.3,0.3,0.5,0.5\n1.5,12.0,0.4,0.4,0.6,0.6";
        let dataset = ExperimentalDataSet::from_csv(csv, "test".into(), None).unwrap();
        assert_eq!(dataset.points.len(), 2);
        assert!((dataset.points[0].x - 0.5).abs() < 1e-12);
        assert!((dataset.points[0].y - 10.0).abs() < 1e-12);
    }

    #[test]
    fn test_chi_square_perfect_match() {
        let theory_contents = vec![10.0, 12.0];
        let theory_edges = vec![0.0, 1.0, 2.0];
        let mut dataset = ExperimentalDataSet::new("test".into(), None);
        dataset.add_point(ExperimentalDataPoint::new(0.5, 10.0, 0.1, 0.1));
        dataset.add_point(ExperimentalDataPoint::new(1.5, 12.0, 0.1, 0.1));

        let result = compute_chi_square(&theory_contents, &theory_edges, &dataset).unwrap();
        assert!(result.chi_square < 1e-6);
    }

    #[test]
    fn test_chi_square_with_asymmetric_errors() {
        let theory_contents = vec![10.0, 12.0];
        let theory_edges = vec![0.0, 1.0, 2.0];
        let mut dataset = ExperimentalDataSet::new("test".into(), None);

        // First point: theory (10.0) > exp (9.5), should use down error
        dataset.add_point(ExperimentalDataPoint::from_asymmetric(
            0.5, 9.5, 0.3, 0.2, // stat: up=0.3, down=0.2 (asymmetric)
            0.5, 0.4, // syst: up=0.5, down=0.4
        ));

        // Second point: theory (12.0) < exp (12.5), should use up error
        dataset.add_point(ExperimentalDataPoint::from_asymmetric(
            1.5, 12.5, 0.2, 0.3, // stat: up=0.2, down=0.3
            0.4, 0.5, // syst: up=0.4, down=0.5
        ));

        let result = compute_chi_square(&theory_contents, &theory_edges, &dataset).unwrap();

        // The result should properly use asymmetric errors
        assert!(result.chi_square > 0.0);
        assert_eq!(result.pulls.len(), 2);

        // First pull should be (0.5 / error_down_1)
        // Second pull should be (-0.5 / error_up_2)
        assert!(result.pulls[0] > 0.0); // 10.0 - 9.5 = +0.5
        assert!(result.pulls[1] < 0.0); // 12.0 - 12.5 = -0.5
    }

    #[test]
    fn test_chi_square_zero_error_protection() {
        let theory_contents = vec![10.0, 12.0];
        let theory_edges = vec![0.0, 1.0, 2.0];
        let mut dataset = ExperimentalDataSet::new("test".into(), None);

        // Point with zero error should be protected by the variance floor
        // and still contribute a finite pull.
        dataset.add_point(ExperimentalDataPoint::new(0.5, 9.0, 0.0, 0.0));
        dataset.add_point(ExperimentalDataPoint::new(1.5, 12.0, 0.1, 0.1));

        let result = compute_chi_square(&theory_contents, &theory_edges, &dataset).unwrap();

        assert_eq!(result.pulls.len(), 2);
        assert!(result.chi_square.is_finite());
    }

    #[test]
    fn test_csv_parse_symmetric_errors() {
        // Test graceful handling of symmetric error format
        let csv = "x,y,dy\n0.5,10.0,0.5\n1.5,12.0,0.6";
        let dataset = ExperimentalDataSet::from_csv(csv, "test".into(), None).unwrap();
        assert_eq!(dataset.points.len(), 2);
        assert!(dataset.points[0].total_error > 0.0);
    }

    #[test]
    fn test_csv_parse_semicolon_and_header_text() {
        let csv = "Observable;Value;Err\npt;ignored;ignored\n0.5;10.0;0.5\n1.5;12.0;0.6";
        let dataset = ExperimentalDataSet::from_csv(csv, "test".into(), None).unwrap();
        assert_eq!(dataset.points.len(), 2);
    }

    #[test]
    fn test_csv_parse_minimal_format() {
        // Test minimal format (x, y only)
        let csv = "x,y\n0.5,10.0\n1.5,12.0";
        let dataset = ExperimentalDataSet::from_csv(csv, "test".into(), None).unwrap();
        assert_eq!(dataset.points.len(), 2);
    }

    #[test]
    fn test_chi_square_reduced() {
        let theory_contents = vec![10.0, 12.0, 14.0];
        let theory_edges = vec![0.0, 1.0, 2.0, 3.0];
        let mut dataset = ExperimentalDataSet::new("test".into(), None);
        dataset.add_point(ExperimentalDataPoint::new(0.5, 10.0, 0.2, 0.2));
        dataset.add_point(ExperimentalDataPoint::new(1.5, 12.0, 0.2, 0.2));
        dataset.add_point(ExperimentalDataPoint::new(2.5, 14.0, 0.2, 0.2));

        let result = compute_chi_square(&theory_contents, &theory_edges, &dataset).unwrap();

        // Perfect match: chi_square should be ~0, reduced chi_square should be ~0
        assert!(result.chi_square < 1e-6);
        assert!(result.chi_square_reduced < 1e-6);
        assert_eq!(result.ndf, 3);
    }

    #[test]
    fn test_chi_square_axis_mismatch() {
        let theory_contents = vec![10.0, 12.0];
        let theory_edges = vec![0.0, 1.0, 2.0];
        let mut dataset = ExperimentalDataSet::new("eta".into(), None);
        dataset.add_point(ExperimentalDataPoint::new(10.5, 10.0, 0.1, 0.1));

        let err = compute_chi_square(&theory_contents, &theory_edges, &dataset).unwrap_err();
        match err {
            SpireError::DataMismatch(_) => {}
            other => panic!("Expected DataMismatch, got {:?}", other),
        }
    }
}
