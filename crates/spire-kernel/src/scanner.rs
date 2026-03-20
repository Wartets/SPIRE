//! # Parameter Scanner - Automated Parameter Sweeps
//!
//! Provides a modular, parallel parameter sweep engine. The scanner is
//! completely agnostic to what it scans - it takes a dot-separated target
//! path (e.g., `"field.Z_prime.mass"`) and sweeps it over a user-defined
//! range, evaluating the cross-section at each scan point.
//!
//! ## Supported Targets
//!
//! - `"field.<field_id>.mass"` - Varies the mass of a field (and its propagator).
//! - `"field.<field_id>.width"` - Varies the decay width of a field.
//! - `"vertex.<term_id>.coupling"` - Varies the coupling constant of a vertex.
//! - `"cms_energy"` - Varies the centre-of-mass energy.
//!
//! ## Parallelism
//!
//! Each scan point is independent: the model is cloned, the parameter is
//! mutated, and the cross-section is evaluated via [`compute_cross_section`].
//! The scan grid is evaluated in parallel using [`rayon`].

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::integration::{compute_cross_section, IntegrationResult, UniformIntegrator};
use crate::kinematics::RamboGenerator;
use crate::lagrangian::TheoreticalModel;
use crate::{SpireError, SpireResult};

// ===========================================================================
// Public Types
// ===========================================================================

/// How the scan steps are distributed between `min` and `max`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanScale {
    /// Uniformly spaced in the parameter value.
    Linear,
    /// Uniformly spaced in the logarithm of the parameter value.
    /// Requires `min > 0`.
    Logarithmic,
}

/// A single parameter to be swept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanVariable {
    /// Dot-separated path into the model.
    ///
    /// Examples: `"field.Z_prime.mass"`, `"vertex.qqZ.coupling"`, `"cms_energy"`.
    pub target: String,
    /// Lower bound of the sweep.
    pub min: f64,
    /// Upper bound of the sweep.
    pub max: f64,
    /// Number of evaluation points (including endpoints).
    pub steps: usize,
    /// Linear or logarithmic spacing.
    pub scale: ScanScale,
}

/// Configuration for a 1D parameter scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig1D {
    /// The variable to sweep.
    pub variable: ScanVariable,
    /// Base theoretical model (cloned and mutated per point).
    pub model: TheoreticalModel,
    /// Rest masses of the final-state particles (GeV).
    pub final_masses: Vec<f64>,
    /// Centre-of-mass energy in GeV (used when `target ≠ "cms_energy"`).
    pub cms_energy: f64,
    /// Number of Monte Carlo events per scan point.
    pub events_per_point: usize,
}

/// Result of a 1D parameter scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult1D {
    /// The scanned variable definition.
    pub variable: ScanVariable,
    /// The parameter values at each scan point.
    pub x_values: Vec<f64>,
    /// The cross-section (pb) at each scan point.
    pub y_values: Vec<f64>,
    /// The statistical uncertainty (pb) at each scan point.
    pub y_errors: Vec<f64>,
}

/// Configuration for a 2D parameter scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig2D {
    /// First variable (x-axis).
    pub variable_x: ScanVariable,
    /// Second variable (y-axis).
    pub variable_y: ScanVariable,
    /// Base theoretical model.
    pub model: TheoreticalModel,
    /// Rest masses of the final-state particles (GeV).
    pub final_masses: Vec<f64>,
    /// Centre-of-mass energy in GeV.
    pub cms_energy: f64,
    /// Number of Monte Carlo events per scan point.
    pub events_per_point: usize,
}

/// Result of a 2D parameter scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult2D {
    /// The first scanned variable definition.
    pub variable_x: ScanVariable,
    /// The second scanned variable definition.
    pub variable_y: ScanVariable,
    /// x-axis parameter values.
    pub x_values: Vec<f64>,
    /// y-axis parameter values.
    pub y_values: Vec<f64>,
    /// Cross-section values (pb) as a flat row-major matrix: `z[i * ny + j]`.
    pub z_values: Vec<f64>,
    /// Statistical uncertainties (pb) in the same layout.
    pub z_errors: Vec<f64>,
}

// ===========================================================================
// Scan Point Generation
// ===========================================================================

/// Generate the evaluation points for a [`ScanVariable`].
///
/// Returns `steps` values distributed between `min` and `max` according
/// to the requested [`ScanScale`].
pub fn generate_scan_points(var: &ScanVariable) -> SpireResult<Vec<f64>> {
    if var.steps < 2 {
        return Err(SpireError::InternalError(
            "Scan requires at least 2 steps".into(),
        ));
    }
    if var.min >= var.max {
        return Err(SpireError::InternalError(
            "Scan min must be strictly less than max".into(),
        ));
    }

    let n = var.steps;
    let points = match var.scale {
        ScanScale::Linear => (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                var.min + t * (var.max - var.min)
            })
            .collect(),

        ScanScale::Logarithmic => {
            if var.min <= 0.0 {
                return Err(SpireError::InternalError(
                    "Logarithmic scale requires min > 0".into(),
                ));
            }
            let log_min = var.min.ln();
            let log_max = var.max.ln();
            (0..n)
                .map(|i| {
                    let t = i as f64 / (n - 1) as f64;
                    (log_min + t * (log_max - log_min)).exp()
                })
                .collect()
        }
    };

    Ok(points)
}

// ===========================================================================
// Parameter Mutation
// ===========================================================================

/// Apply a parameter value to a model, resolved by the dot-separated `target` path.
///
/// # Supported paths
///
/// | Pattern                         | Effect                                            |
/// |---------------------------------|---------------------------------------------------|
/// | `field.<id>.mass`               | Sets `Field.mass` and `Propagator.mass`           |
/// | `field.<id>.width`              | Sets `Field.width` and `Propagator.width`         |
/// | `vertex.<term_id>.coupling`     | Sets `VertexFactor.coupling_value`                |
/// | `cms_energy`                    | No-op on the model (handled externally)           |
pub fn apply_parameter(model: &mut TheoreticalModel, target: &str, value: f64) -> SpireResult<()> {
    let parts: Vec<&str> = target.split('.').collect();

    match parts.as_slice() {
        ["field", field_id, "mass"] => {
            let field = model
                .fields
                .iter_mut()
                .find(|f| f.id == *field_id)
                .ok_or_else(|| {
                    SpireError::UnknownParticle(format!("Field '{}' not found", field_id))
                })?;
            field.mass = value;
            // Keep propagator mass in sync.
            if let Some(prop) = model
                .propagators
                .iter_mut()
                .find(|p| p.field_id == *field_id)
            {
                prop.mass = value;
            }
            Ok(())
        }

        ["field", field_id, "width"] => {
            let field = model
                .fields
                .iter_mut()
                .find(|f| f.id == *field_id)
                .ok_or_else(|| {
                    SpireError::UnknownParticle(format!("Field '{}' not found", field_id))
                })?;
            field.width = value;
            if let Some(prop) = model
                .propagators
                .iter_mut()
                .find(|p| p.field_id == *field_id)
            {
                prop.width = value;
            }
            Ok(())
        }

        ["vertex", term_id, "coupling"] => {
            let vf = model
                .vertex_factors
                .iter_mut()
                .find(|v| v.term_id == *term_id)
                .ok_or_else(|| {
                    SpireError::InvalidVertex(format!("Vertex '{}' not found", term_id))
                })?;
            vf.coupling_value = Some(value);
            Ok(())
        }

        ["cms_energy"] => {
            // Centre-of-mass energy is handled externally; no model mutation.
            Ok(())
        }

        _ => Err(SpireError::InternalError(format!(
            "Unknown scan target path: '{}'",
            target
        ))),
    }
}

// ===========================================================================
// Single-Point Evaluation
// ===========================================================================

/// Evaluate a single scan point: clone the model, apply the parameter, run MC.
///
/// Returns the cross-section in picobarns.
fn evaluate_scan_point(
    base_model: &TheoreticalModel,
    target: &str,
    value: f64,
    final_masses: &[f64],
    base_cms_energy: f64,
    num_events: usize,
) -> SpireResult<IntegrationResult> {
    let mut model = base_model.clone();
    apply_parameter(&mut model, target, value)?;

    // If scanning cms_energy, use the scan value; otherwise use the base.
    let cms_energy = if target == "cms_energy" {
        value
    } else {
        base_cms_energy
    };

    let integrator = UniformIntegrator::new();
    let mut generator = RamboGenerator::new();

    // Use constant |M|² = 1 (pure phase-space distribution).
    // Future: accept a user-defined amplitude closure or derive |M|² from diagrams.
    let result = compute_cross_section(
        |_| 1.0,
        &integrator,
        &mut generator,
        cms_energy,
        final_masses,
        num_events,
    )?;

    Ok(result.to_picobarns())
}

// ===========================================================================
// Public API - 1D Scan
// ===========================================================================

/// Run a 1D parameter scan with parallel evaluation via [`rayon`].
///
/// Each scan point is evaluated independently: the model is cloned, the
/// target parameter is mutated, and the cross-section is computed via
/// flat Monte Carlo integration.
///
/// # Returns
///
/// A [`ScanResult1D`] with x-values (parameter), y-values (σ in pb),
/// and y-errors (δσ in pb).
pub fn run_scan_1d(config: &ScanConfig1D) -> SpireResult<ScanResult1D> {
    let x_values = generate_scan_points(&config.variable)?;

    let results: Vec<SpireResult<IntegrationResult>> = x_values
        .par_iter()
        .map(|&x| {
            evaluate_scan_point(
                &config.model,
                &config.variable.target,
                x,
                &config.final_masses,
                config.cms_energy,
                config.events_per_point,
            )
        })
        .collect();

    let mut y_values = Vec::with_capacity(x_values.len());
    let mut y_errors = Vec::with_capacity(x_values.len());

    for result in results {
        let ir = result?;
        y_values.push(ir.value);
        y_errors.push(ir.error);
    }

    Ok(ScanResult1D {
        variable: config.variable.clone(),
        x_values,
        y_values,
        y_errors,
    })
}

// ===========================================================================
// Public API - 2D Scan
// ===========================================================================

/// Run a 2D parameter scan with parallel evaluation via [`rayon`].
///
/// The scan grid is the Cartesian product of the two variable ranges.
/// Each `(x, y)` point is evaluated independently in parallel.
///
/// # Returns
///
/// A [`ScanResult2D`] with x/y-axis values and a row-major z-matrix
/// of cross-sections and uncertainties.
pub fn run_scan_2d(config: &ScanConfig2D) -> SpireResult<ScanResult2D> {
    let x_values = generate_scan_points(&config.variable_x)?;
    let y_values = generate_scan_points(&config.variable_y)?;

    let nx = x_values.len();
    let ny = y_values.len();

    // Build all (i, j) index pairs for the 2D grid.
    let pairs: Vec<(usize, usize)> = (0..nx).flat_map(|i| (0..ny).map(move |j| (i, j))).collect();

    let results: Vec<SpireResult<(usize, IntegrationResult)>> = pairs
        .par_iter()
        .map(|&(i, j)| {
            let mut model = config.model.clone();
            apply_parameter(&mut model, &config.variable_x.target, x_values[i])?;
            apply_parameter(&mut model, &config.variable_y.target, y_values[j])?;

            let cms_energy = if config.variable_x.target == "cms_energy" {
                x_values[i]
            } else if config.variable_y.target == "cms_energy" {
                y_values[j]
            } else {
                config.cms_energy
            };

            let integrator = UniformIntegrator::new();
            let mut generator = RamboGenerator::new();

            let result = compute_cross_section(
                |_| 1.0,
                &integrator,
                &mut generator,
                cms_energy,
                &config.final_masses,
                config.events_per_point,
            )?;

            Ok((i * ny + j, result.to_picobarns()))
        })
        .collect();

    let mut z_values = vec![0.0; nx * ny];
    let mut z_errors = vec![0.0; nx * ny];

    for result in results {
        let (idx, ir) = result?;
        z_values[idx] = ir.value;
        z_errors[idx] = ir.error;
    }

    Ok(ScanResult2D {
        variable_x: config.variable_x.clone(),
        variable_y: config.variable_y.clone(),
        x_values,
        y_values,
        z_values,
        z_errors,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrangian::{Propagator, PropagatorForm, VertexFactor};
    use crate::ontology::*;

    /// Build a minimal test model with two light particles and a propagator.
    fn test_model() -> TheoreticalModel {
        let qn = QuantumNumbers {
            electric_charge: ElectricCharge(0),
            weak_isospin: WeakIsospin(0),
            hypercharge: Hypercharge(0),
            baryon_number: BaryonNumber(0),
            lepton_numbers: LeptonNumbers {
                electron: 0,
                muon: 0,
                tau: 0,
            },
            spin: Spin(1), // spin-½
            parity: Parity::Odd,
            charge_conjugation: ChargeConjugation::Undefined,
            color: ColorRepresentation::Singlet,
            weak_multiplet: WeakMultiplet::Singlet,
            representations: vec![],
        };

        let electron = Field {
            id: "electron".into(),
            name: "Electron".into(),
            symbol: "e^-".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: qn.clone(),
            interactions: vec![],
        };

        let muon = Field {
            id: "muon".into(),
            name: "Muon".into(),
            symbol: "\\mu^-".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: qn.clone(),
            interactions: vec![],
        };

        let z_boson = Field {
            id: "Z".into(),
            name: "Z boson".into(),
            symbol: "Z^0".into(),
            mass: 91.1876,
            width: 2.4952,
            quantum_numbers: qn,
            interactions: vec![],
        };

        let z_propagator = Propagator {
            field_id: "Z".into(),
            spin: Spin(2), // spin-1
            mass: 91.1876,
            width: 2.4952,
            expression: "i(-g_{mu nu} + p_mu p_nu / m_Z^2) / (p^2 - m_Z^2 + i m_Z Gamma_Z)".into(),
            gauge_parameter: None,
            form: PropagatorForm::MassiveVector,
        };

        let vertex = VertexFactor {
            term_id: "eeZ".into(),
            field_ids: vec!["electron".into(), "electron".into(), "Z".into()],
            expression: "-i (g / cos(theta_W)) gamma^mu (g_V - g_A gamma^5)".into(),
            coupling_value: Some(0.0753),
            n_legs: 3,
        };

        TheoreticalModel {
            name: "Test Model".into(),
            description: "Minimal model for scanner tests".into(),
            fields: vec![electron, muon, z_boson],
            terms: vec![],
            vertex_factors: vec![vertex],
            propagators: vec![z_propagator],
            gauge_symmetry: None,
            spacetime: Default::default(),
            constants: Default::default(),
            pdg_provenance: None,
        }
    }

    // -----------------------------------------------------------------------
    // Scan Point Generation
    // -----------------------------------------------------------------------

    #[test]
    fn scan_points_linear() {
        let var = ScanVariable {
            target: "cms_energy".into(),
            min: 10.0,
            max: 100.0,
            steps: 10,
            scale: ScanScale::Linear,
        };
        let pts = generate_scan_points(&var).unwrap();
        assert_eq!(pts.len(), 10);
        assert!((pts[0] - 10.0).abs() < 1e-10);
        assert!((pts[9] - 100.0).abs() < 1e-10);
        // Step size should be uniform.
        let step = (100.0 - 10.0) / 9.0;
        for i in 1..10 {
            assert!((pts[i] - pts[i - 1] - step).abs() < 1e-10);
        }
    }

    #[test]
    fn scan_points_logarithmic() {
        let var = ScanVariable {
            target: "cms_energy".into(),
            min: 1.0,
            max: 1000.0,
            steps: 4,
            scale: ScanScale::Logarithmic,
        };
        let pts = generate_scan_points(&var).unwrap();
        assert_eq!(pts.len(), 4);
        assert!((pts[0] - 1.0).abs() < 1e-10);
        assert!((pts[3] - 1000.0).abs() < 1e-6);
        // Logarithmic spacing: intermediate points should be ~10 and ~100.
        assert!((pts[1] - 10.0).abs() < 0.01, "Expected ~10, got {}", pts[1]);
        assert!(
            (pts[2] - 100.0).abs() < 0.1,
            "Expected ~100, got {}",
            pts[2]
        );
    }

    #[test]
    fn scan_points_rejects_insufficient_steps() {
        let var = ScanVariable {
            target: "x".into(),
            min: 1.0,
            max: 10.0,
            steps: 1,
            scale: ScanScale::Linear,
        };
        assert!(generate_scan_points(&var).is_err());
    }

    #[test]
    fn scan_points_rejects_inverted_range() {
        let var = ScanVariable {
            target: "x".into(),
            min: 100.0,
            max: 10.0,
            steps: 5,
            scale: ScanScale::Linear,
        };
        assert!(generate_scan_points(&var).is_err());
    }

    #[test]
    fn scan_points_rejects_negative_log() {
        let var = ScanVariable {
            target: "x".into(),
            min: -1.0,
            max: 10.0,
            steps: 5,
            scale: ScanScale::Logarithmic,
        };
        assert!(generate_scan_points(&var).is_err());
    }

    // -----------------------------------------------------------------------
    // Parameter Application
    // -----------------------------------------------------------------------

    #[test]
    fn apply_field_mass() {
        let mut model = test_model();
        apply_parameter(&mut model, "field.Z.mass", 200.0).unwrap();
        let z = model.fields.iter().find(|f| f.id == "Z").unwrap();
        assert!((z.mass - 200.0).abs() < 1e-10);
        // Propagator should also be updated.
        let prop = model
            .propagators
            .iter()
            .find(|p| p.field_id == "Z")
            .unwrap();
        assert!((prop.mass - 200.0).abs() < 1e-10);
    }

    #[test]
    fn apply_field_width() {
        let mut model = test_model();
        apply_parameter(&mut model, "field.Z.width", 5.0).unwrap();
        let z = model.fields.iter().find(|f| f.id == "Z").unwrap();
        assert!((z.width - 5.0).abs() < 1e-10);
        let prop = model
            .propagators
            .iter()
            .find(|p| p.field_id == "Z")
            .unwrap();
        assert!((prop.width - 5.0).abs() < 1e-10);
    }

    #[test]
    fn apply_vertex_coupling() {
        let mut model = test_model();
        apply_parameter(&mut model, "vertex.eeZ.coupling", 0.5).unwrap();
        let vf = model
            .vertex_factors
            .iter()
            .find(|v| v.term_id == "eeZ")
            .unwrap();
        assert!((vf.coupling_value.unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn apply_cms_energy_is_noop() {
        let mut model = test_model();
        let before = model.clone();
        apply_parameter(&mut model, "cms_energy", 9999.0).unwrap();
        // Model should be unchanged.
        assert_eq!(model.fields.len(), before.fields.len());
    }

    #[test]
    fn apply_unknown_target_fails() {
        let mut model = test_model();
        assert!(apply_parameter(&mut model, "bogus.path.here", 1.0).is_err());
    }

    #[test]
    fn apply_unknown_field_fails() {
        let mut model = test_model();
        assert!(apply_parameter(&mut model, "field.nonexistent.mass", 1.0).is_err());
    }

    // -----------------------------------------------------------------------
    // Integration: 1D Scan
    // -----------------------------------------------------------------------

    #[test]
    fn run_1d_scan_cms_energy() {
        let model = test_model();
        let config = ScanConfig1D {
            variable: ScanVariable {
                target: "cms_energy".into(),
                min: 10.0,
                max: 200.0,
                steps: 5,
                scale: ScanScale::Linear,
            },
            model,
            final_masses: vec![0.000511, 0.000511],
            cms_energy: 100.0, // ignored when target = cms_energy
            events_per_point: 500,
        };

        let result = run_scan_1d(&config).unwrap();
        assert_eq!(result.x_values.len(), 5);
        assert_eq!(result.y_values.len(), 5);
        assert_eq!(result.y_errors.len(), 5);

        // Cross-section should be positive at all points.
        for (i, &y) in result.y_values.iter().enumerate() {
            assert!(
                y > 0.0,
                "Cross-section at point {} (E={}) should be positive, got {}",
                i,
                result.x_values[i],
                y
            );
        }
    }

    #[test]
    fn run_1d_scan_field_mass() {
        let model = test_model();
        let config = ScanConfig1D {
            variable: ScanVariable {
                target: "field.electron.mass".into(),
                min: 0.0001,
                max: 1.0,
                steps: 3,
                scale: ScanScale::Logarithmic,
            },
            model,
            final_masses: vec![0.000511, 0.000511],
            cms_energy: 100.0,
            events_per_point: 500,
        };

        let result = run_scan_1d(&config).unwrap();
        assert_eq!(result.x_values.len(), 3);
        for &y in &result.y_values {
            assert!(y > 0.0);
        }
    }
}
