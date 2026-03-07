//! # Likelihood Framework for Global Fits
//!
//! Provides a modular system for defining experimental constraints and
//! computing the global log-likelihood $\ln \mathcal{L}$ required by the
//! MCMC sampler.
//!
//! ## Architecture
//!
//! The framework decouples three concerns:
//!
//! 1. **Parameter Mapping**: A [`FitParameter`] specifies which field in the
//!    `TheoreticalModel` is floated, its prior range, and a setter closure.
//! 2. **Experimental Constraints**: Each [`ExperimentalConstraint`] encapsulates
//!    one measurement (e.g., $m_H = 125.25 \pm 0.17$ GeV) and knows how to
//!    extract the predicted value from the model.
//! 3. **Global Likelihood**: The [`GlobalLikelihood`] orchestrates parameter
//!    injection, constraint evaluation, and $\chi^2$ summation.
//!
//! ## Gaussian Constraint
//!
//! The most common constraint is Gaussian:
//! $$\ln \mathcal{L}_i = -\frac{1}{2} \left(\frac{x_\text{pred} - x_\text{obs}}{\sigma}\right)^2$$
//!
//! The global log-likelihood is the sum over all active constraints:
//! $$\ln \mathcal{L} = \sum_i \ln \mathcal{L}_i$$

use serde::{Deserialize, Serialize};

use crate::lagrangian::TheoreticalModel;

// ===========================================================================
// Fit Parameter Specification
// ===========================================================================

/// A parameter to be floated in the global fit.
///
/// Encapsulates which physical quantity is varied, its prior bounds,
/// and a human-readable label for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitParameter {
    /// Human-readable name (e.g., "Higgs mass", "Z' coupling").
    pub name: String,
    /// Lower bound of the flat (uniform) prior.
    pub min: f64,
    /// Upper bound of the flat (uniform) prior.
    pub max: f64,
    /// The field ID in the model to modify (e.g., "higgs").
    /// If empty, the parameter is a generic coupling/constant.
    pub field_id: String,
    /// Which property of the field to float.
    pub property: FitProperty,
}

/// Which physical property of a field or model constant is being floated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FitProperty {
    /// The rest mass of a field (in GeV).
    Mass,
    /// The total decay width of a field (in GeV).
    Width,
    /// A named coupling constant.
    Coupling(String),
}

// ===========================================================================
// Experimental Constraint Trait
// ===========================================================================

/// An experimental measurement that constrains the model.
///
/// Implementors extract a predicted value from the model and return
/// the chi-squared penalty: positive values penalise disagreement.
/// The MCMC sampler sums these and negates to get $\ln \mathcal{L}$.
pub trait ExperimentalConstraint: Send + Sync {
    /// Human-readable name of this constraint.
    fn name(&self) -> &str;

    /// Evaluate the constraint against the given model.
    ///
    /// Returns the chi-squared contribution (positive):
    /// $$\chi^2_i = \left(\frac{x_\text{pred} - x_\text{obs}}{\sigma}\right)^2$$
    fn chi2_contribution(&self, model: &TheoreticalModel) -> f64;
}

// ===========================================================================
// Gaussian Constraint
// ===========================================================================

/// A Gaussian measurement constraint.
///
/// $$\chi^2_i = \left(\frac{x_\text{pred} - x_\text{obs}}{\sigma}\right)^2$$
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaussianConstraint {
    /// Constraint label.
    pub name: String,
    /// Observed central value.
    pub observed: f64,
    /// Experimental uncertainty ($1\sigma$).
    pub sigma: f64,
    /// The field ID whose mass (or other property) is predicted.
    pub field_id: String,
    /// Which property to compare.
    pub property: FitProperty,
}

impl ExperimentalConstraint for GaussianConstraint {
    fn name(&self) -> &str {
        &self.name
    }

    fn chi2_contribution(&self, model: &TheoreticalModel) -> f64 {
        let predicted = extract_property(model, &self.field_id, &self.property);
        let pull = (predicted - self.observed) / self.sigma;
        pull * pull
    }
}

// ===========================================================================
// Upper/Lower Bound Constraint
// ===========================================================================

/// A one-sided bound constraint (e.g., exclusion limit).
///
/// Returns 0 if within bounds, large penalty if violated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundConstraint {
    /// Constraint label.
    pub name: String,
    /// Lower bound (or `f64::NEG_INFINITY` for no lower bound).
    pub lower: f64,
    /// Upper bound (or `f64::INFINITY` for no upper bound).
    pub upper: f64,
    /// The field ID to check.
    pub field_id: String,
    /// Which property to check.
    pub property: FitProperty,
    /// Penalty scale for violation (default: 1e6).
    pub penalty_scale: f64,
}

impl ExperimentalConstraint for BoundConstraint {
    fn name(&self) -> &str {
        &self.name
    }

    fn chi2_contribution(&self, model: &TheoreticalModel) -> f64 {
        let value = extract_property(model, &self.field_id, &self.property);
        if value < self.lower {
            let pull = (value - self.lower) / self.penalty_scale.max(1.0);
            pull * pull * self.penalty_scale
        } else if value > self.upper {
            let pull = (value - self.upper) / self.penalty_scale.max(1.0);
            pull * pull * self.penalty_scale
        } else {
            0.0
        }
    }
}

// ===========================================================================
// Relic Density Constraint
// ===========================================================================

/// Constraint on the cosmological relic density $\Omega h^2$.
///
/// Uses the Planck 2018 measurement: $\Omega h^2 = 0.120 \pm 0.001$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicDensityConstraint {
    /// Constraint label.
    pub name: String,
    /// Observed relic density (default: 0.120).
    pub observed: f64,
    /// Experimental uncertainty (default: 0.001).
    pub sigma: f64,
    /// Dark matter candidate field ID.
    pub dm_field_id: String,
}

impl Default for RelicDensityConstraint {
    fn default() -> Self {
        Self {
            name: "Planck ΩDMh² (2018)".into(),
            observed: 0.120,
            sigma: 0.001,
            dm_field_id: String::new(),
        }
    }
}

impl ExperimentalConstraint for RelicDensityConstraint {
    fn name(&self) -> &str {
        &self.name
    }

    fn chi2_contribution(&self, model: &TheoreticalModel) -> f64 {
        // Extract DM mass from the model
        let dm_mass = if self.dm_field_id.is_empty() {
            100.0 // Default placeholder
        } else {
            extract_property(model, &self.dm_field_id, &FitProperty::Mass)
        };

        // Simple thermal relic estimate using the kernel's relic density module.
        // compute_relic_density returns RelicDensityReport directly.
        let config = crate::cosmology::relic::RelicConfig {
            dm_mass,
            sigma_v_a: 3e-26,
            sigma_v_b: 0.0,
            g_dm: 2.0,
            g_star: 86.25,
            g_star_s: 86.25,
            x_start: 1.0,
            x_end: 1000.0,
        };

        let report = crate::cosmology::relic::compute_relic_density(&config);
        let pull = (report.omega_h2 - self.observed) / self.sigma;
        pull * pull
    }
}

// ===========================================================================
// Global Likelihood
// ===========================================================================

/// MCMC fit configuration sent from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFitConfig {
    /// Parameters to float in the fit.
    pub parameters: Vec<FitParameter>,
    /// Active Gaussian constraints.
    pub gaussian_constraints: Vec<GaussianConstraint>,
    /// Number of MCMC walkers (must be even, ≥ 2 × n_dim).
    pub n_walkers: usize,
    /// Number of MCMC steps.
    pub n_steps: usize,
    /// Burn-in steps to discard.
    pub burn_in: usize,
}

/// The global likelihood evaluator.
///
/// Takes a base `TheoreticalModel`, a list of parameters to float,
/// and a list of constraints.  Provides a `log_prob(&[f64]) -> f64`
/// function suitable for the MCMC sampler.
pub struct GlobalLikelihood {
    /// The base model (template).
    base_model: TheoreticalModel,
    /// Parameters being floated.
    parameters: Vec<FitParameter>,
    /// Active constraints.
    constraints: Vec<Box<dyn ExperimentalConstraint>>,
}

impl GlobalLikelihood {
    /// Create a new global likelihood evaluator.
    pub fn new(
        base_model: TheoreticalModel,
        parameters: Vec<FitParameter>,
        constraints: Vec<Box<dyn ExperimentalConstraint>>,
    ) -> Self {
        Self {
            base_model,
            parameters,
            constraints,
        }
    }

    /// Evaluate the log-probability for a given parameter vector.
    ///
    /// This is the function passed to the MCMC sampler.
    ///
    /// 1. Checks that all parameters are within their prior bounds.
    /// 2. Injects the parameter values into a clone of the base model.
    /// 3. Evaluates all constraints and sums the $\chi^2$ contributions.
    /// 4. Returns $\ln \mathcal{L} = -\frac{1}{2} \sum_i \chi^2_i$.
    pub fn log_prob(&self, params: &[f64]) -> f64 {
        assert_eq!(params.len(), self.parameters.len());

        // Check flat priors
        for (i, p) in self.parameters.iter().enumerate() {
            if params[i] < p.min || params[i] > p.max {
                return f64::NEG_INFINITY; // Outside prior → reject
            }
        }

        // Inject parameters into a model clone
        let mut model = self.base_model.clone();
        for (i, param_def) in self.parameters.iter().enumerate() {
            inject_parameter(&mut model, param_def, params[i]);
        }

        // Sum χ² from all constraints
        let total_chi2: f64 = self
            .constraints
            .iter()
            .map(|c| c.chi2_contribution(&model))
            .sum();

        // Return log-likelihood = -χ²/2
        -0.5 * total_chi2
    }
}

// ===========================================================================
// Helper: Parameter Injection
// ===========================================================================

/// Inject a parameter value into the appropriate field of the model.
fn inject_parameter(model: &mut TheoreticalModel, param: &FitParameter, value: f64) {
    if param.field_id.is_empty() {
        return;
    }
    for field in &mut model.fields {
        if field.id == param.field_id {
            match &param.property {
                FitProperty::Mass => field.mass = value,
                FitProperty::Width => field.width = value,
                FitProperty::Coupling(_) => {
                    // Future: modify coupling constants in vertex factors
                }
            }
            break;
        }
    }
}

/// Extract a predicted property value from the model.
fn extract_property(model: &TheoreticalModel, field_id: &str, property: &FitProperty) -> f64 {
    for field in &model.fields {
        if field.id == field_id {
            return match property {
                FitProperty::Mass => field.mass,
                FitProperty::Width => field.width,
                FitProperty::Coupling(_) => 0.0, // Placeholder
            };
        }
    }
    0.0
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{
        BaryonNumber, ChargeConjugation, ColorRepresentation, ElectricCharge, Field, Hypercharge,
        InteractionType, LeptonNumbers, Parity, QuantumNumbers, Spin, WeakIsospin, WeakMultiplet,
    };

    /// Build a zero-charged scalar quantum number set suitable for test fields.
    fn scalar_qn() -> QuantumNumbers {
        QuantumNumbers {
            electric_charge: ElectricCharge(0),
            weak_isospin: WeakIsospin(0),
            hypercharge: Hypercharge(0),
            baryon_number: BaryonNumber(0),
            lepton_numbers: LeptonNumbers {
                electron: 0,
                muon: 0,
                tau: 0,
            },
            spin: Spin(0),
            parity: Parity::Even,
            charge_conjugation: ChargeConjugation::Even,
            color: ColorRepresentation::Singlet,
            weak_multiplet: WeakMultiplet::Singlet,
            representations: vec![],
        }
    }

    fn make_test_model() -> TheoreticalModel {
        let higgs = Field {
            id: "higgs".into(),
            name: "Higgs boson".into(),
            symbol: "H".into(),
            mass: 125.25,
            width: 0.004,
            quantum_numbers: scalar_qn(),
            interactions: vec![InteractionType::WeakCC],
        };

        TheoreticalModel {
            name: "Test Model".into(),
            description: "Minimal test model".into(),
            fields: vec![higgs],
            ..Default::default()
        }
    }

    #[test]
    fn gaussian_constraint_exact_match() {
        let model = make_test_model();
        let constraint = GaussianConstraint {
            name: "Higgs mass".into(),
            observed: 125.25,
            sigma: 0.17,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        };

        let chi2 = constraint.chi2_contribution(&model);
        assert!(
            chi2 < 1e-10,
            "χ² should be ~0 for exact match, got {}",
            chi2
        );
    }

    #[test]
    fn gaussian_constraint_one_sigma_pull() {
        let model = make_test_model();
        let constraint = GaussianConstraint {
            name: "Higgs mass".into(),
            observed: 125.08, // 1σ away from model's 125.25
            sigma: 0.17,      // Pull = (125.25 - 125.08) / 0.17 = 1.0
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        };

        let chi2 = constraint.chi2_contribution(&model);
        assert!(
            (chi2 - 1.0).abs() < 0.05,
            "χ² should be ~1.0 for 1σ pull, got {}",
            chi2
        );
    }

    #[test]
    fn global_likelihood_prior_rejection() {
        let model = make_test_model();
        let params = vec![FitParameter {
            name: "Higgs mass".into(),
            min: 100.0,
            max: 150.0,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        }];
        let constraints: Vec<Box<dyn ExperimentalConstraint>> =
            vec![Box::new(GaussianConstraint {
                name: "Higgs mass".into(),
                observed: 125.25,
                sigma: 0.17,
                field_id: "higgs".into(),
                property: FitProperty::Mass,
            })];

        let likelihood = GlobalLikelihood::new(model, params, constraints);

        // Value within prior
        let lp = likelihood.log_prob(&[125.0]);
        assert!(lp.is_finite(), "Should be finite within prior");

        // Value outside prior
        let lp_out = likelihood.log_prob(&[200.0]);
        assert!(lp_out == f64::NEG_INFINITY, "Should be -∞ outside prior");
    }

    #[test]
    fn global_likelihood_finds_maximum() {
        let model = make_test_model();
        let params = vec![FitParameter {
            name: "Higgs mass".into(),
            min: 100.0,
            max: 150.0,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        }];
        let constraints: Vec<Box<dyn ExperimentalConstraint>> =
            vec![Box::new(GaussianConstraint {
                name: "Higgs mass".into(),
                observed: 125.25,
                sigma: 2.0,
                field_id: "higgs".into(),
                property: FitProperty::Mass,
            })];

        let likelihood = GlobalLikelihood::new(model, params, constraints);

        // The log-prob should be maximised at the observed value
        let lp_at_obs = likelihood.log_prob(&[125.25]);
        let lp_away = likelihood.log_prob(&[130.0]);

        assert!(
            lp_at_obs > lp_away,
            "Log-prob at observed ({}) should exceed away ({})",
            lp_at_obs,
            lp_away
        );
    }

    #[test]
    fn parameter_injection_updates_mass() {
        let mut model = make_test_model();
        let param = FitParameter {
            name: "Higgs mass".into(),
            min: 100.0,
            max: 150.0,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        };

        inject_parameter(&mut model, &param, 130.0);
        assert_eq!(model.fields[0].mass, 130.0);
    }

    #[test]
    fn mcmc_recovers_higgs_mass() {
        use crate::math::mcmc::{EnsembleSampler, McmcConfig};

        let model = make_test_model();
        let params = vec![FitParameter {
            name: "Higgs mass".into(),
            min: 100.0,
            max: 150.0,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
        }];
        let constraints: Vec<Box<dyn ExperimentalConstraint>> =
            vec![Box::new(GaussianConstraint {
                name: "Higgs mass".into(),
                observed: 125.0,
                sigma: 2.0,
                field_id: "higgs".into(),
                property: FitProperty::Mass,
            })];

        let likelihood = GlobalLikelihood::new(model, params, constraints);

        let config = McmcConfig {
            n_walkers: 16,
            n_dim: 1,
            n_steps: 2000,
            stretch_factor: 2.0,
        };

        let mut rng = rand::thread_rng();
        let initial: Vec<Vec<f64>> = (0..16)
            .map(|_| vec![100.0 + rand::Rng::gen_range(&mut rng, 0.0..50.0)])
            .collect();

        let mut sampler = EnsembleSampler::new(config, initial);
        let chain = sampler.run(&|p: &[f64]| likelihood.log_prob(p));

        let means = chain.parameter_means(500);
        assert!(
            (means[0] - 125.0).abs() < 3.0,
            "MCMC mean should be ~125.0, got {}",
            means[0]
        );
    }

    #[test]
    fn bound_constraint_within_bounds() {
        let model = make_test_model();
        let constraint = BoundConstraint {
            name: "Higgs mass bound".into(),
            lower: 100.0,
            upper: 200.0,
            field_id: "higgs".into(),
            property: FitProperty::Mass,
            penalty_scale: 1e6,
        };

        let chi2 = constraint.chi2_contribution(&model);
        assert!(chi2 < 1e-10, "Should be 0 within bounds, got {}", chi2);
    }
}
