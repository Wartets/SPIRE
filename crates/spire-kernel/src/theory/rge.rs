//! # Renormalization Group Equation Solver
//!
//! Integrates user-defined β-functions to compute the running of coupling
//! constants as a function of the energy scale $\mu$.
//!
//! ## RGE Equation
//!
//! $$\frac{dg}{d \ln \mu} = \beta(g)$$
//!
//! The β-function is defined via a Rhai script, allowing full runtime
//! customisation for any theory.
//!
//! ## Integration Method
//!
//! Uses 4th-order Runge–Kutta (RK4) for robust, accurate integration.

use serde::{Deserialize, Serialize};

use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for an RGE flow computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgeFlowConfig {
    /// Name of the coupling being run (e.g., `"alpha_s"`, `"g_1"`).
    pub coupling_name: String,
    /// The Rhai script defining the β-function.
    ///
    /// The script receives a variable `g` (the coupling value) and must
    /// return the β-function value. Additional couplings can be passed
    /// as named variables in `extra_couplings`.
    ///
    /// **Example** (QCD at 1-loop):
    /// ```text
    /// let b0 = -11.0 + 2.0 * nf / 3.0;
    /// b0 * g * g * g / (16.0 * PI * PI)
    /// ```
    pub beta_script: String,
    /// Initial value of the coupling at $\mu = \mu_{\text{init}}$.
    pub initial_value: f64,
    /// Minimum energy scale (GeV) for the integration.
    pub mu_min: f64,
    /// Maximum energy scale (GeV) for the integration.
    pub mu_max: f64,
    /// Number of integration steps (output points).
    pub n_points: usize,
    /// Extra named constants available in the β-function script.
    #[serde(default)]
    pub extra_constants: std::collections::HashMap<String, f64>,
}

/// The result of an RGE flow integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgeFlowResult {
    /// Name of the coupling.
    pub coupling_name: String,
    /// Energy scale values $\mu$ (GeV), logarithmically spaced.
    pub mu_values: Vec<f64>,
    /// Coupling values $g(\mu)$ at each scale.
    pub coupling_values: Vec<f64>,
}

// ---------------------------------------------------------------------------
// RGE Solver
// ---------------------------------------------------------------------------

/// Run the RGE flow integration.
///
/// Evaluates the β-function script using the embedded Rhai engine and
/// integrates the RGE using 4th-order Runge–Kutta in the variable $t = \ln(\mu)$.
///
/// # Arguments
/// * `config` - The RGE flow configuration.
///
/// # Returns
/// An `RgeFlowResult` with the coupling evolution.
pub fn run_rge_flow(config: &RgeFlowConfig) -> SpireResult<RgeFlowResult> {
    if config.mu_min <= 0.0 || config.mu_max <= 0.0 {
        return Err(SpireError::KinematicsForbidden(
            "RGE scale μ must be positive".to_string(),
        ));
    }
    if config.mu_min >= config.mu_max {
        return Err(SpireError::KinematicsForbidden(
            "mu_min must be less than mu_max".to_string(),
        ));
    }
    if config.n_points < 2 {
        return Err(SpireError::KinematicsForbidden(
            "Need at least 2 integration points".to_string(),
        ));
    }

    // Set up the Rhai engine.
    let mut engine = rhai::Engine::new();
    engine.set_max_operations(1_000_000);

    // Register mathematical constants.
    let pi = std::f64::consts::PI;

    // Compile the β-function script.
    // We wrap it: given `g`, return the β value.
    let beta_source = format!(
        "fn beta(g) {{ let PI = {pi}; {extras} {script} }}",
        pi = pi,
        extras = config
            .extra_constants
            .iter()
            .map(|(k, v)| format!("let {} = {};", k, v))
            .collect::<Vec<_>>()
            .join(" "),
        script = config.beta_script,
    );

    let ast = engine.compile(&beta_source).map_err(|e| {
        SpireError::InternalError(format!("Failed to compile β-function script: {}", e))
    })?;

    // Integration in t = ln(μ).
    let t_min = config.mu_min.ln();
    let t_max = config.mu_max.ln();
    let dt = (t_max - t_min) / (config.n_points - 1) as f64;

    let mut mu_values = Vec::with_capacity(config.n_points);
    let mut coupling_values = Vec::with_capacity(config.n_points);

    let mut g = config.initial_value;
    let mut t = t_min;

    for i in 0..config.n_points {
        mu_values.push(t.exp());
        coupling_values.push(g);

        if i < config.n_points - 1 {
            // RK4 step.
            let k1 = dt * eval_beta(&engine, &ast, g)?;
            let k2 = dt * eval_beta(&engine, &ast, g + 0.5 * k1)?;
            let k3 = dt * eval_beta(&engine, &ast, g + 0.5 * k2)?;
            let k4 = dt * eval_beta(&engine, &ast, g + k3)?;
            g += (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            t += dt;
        }
    }

    Ok(RgeFlowResult {
        coupling_name: config.coupling_name.clone(),
        mu_values,
        coupling_values,
    })
}

/// Evaluate the compiled β-function at a given coupling value.
fn eval_beta(engine: &rhai::Engine, ast: &rhai::AST, g: f64) -> SpireResult<f64> {
    let result: f64 = engine
        .call_fn(&mut rhai::Scope::new(), ast, "beta", (g,))
        .map_err(|e| SpireError::InternalError(format!("β-function evaluation failed: {}", e)))?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rge_constant_coupling() {
        // β(g) = 0 → coupling should remain constant.
        let config = RgeFlowConfig {
            coupling_name: "g_test".to_string(),
            beta_script: "0.0".to_string(),
            initial_value: 0.5,
            mu_min: 1.0,
            mu_max: 1000.0,
            n_points: 100,
            extra_constants: Default::default(),
        };
        let result = run_rge_flow(&config).unwrap();
        assert_eq!(result.mu_values.len(), 100);
        assert_eq!(result.coupling_values.len(), 100);
        for &val in &result.coupling_values {
            assert!((val - 0.5).abs() < 1e-10, "Expected 0.5, got {}", val);
        }
    }

    #[test]
    fn rge_linear_beta() {
        // β(g) = 1 → g(t) = g₀ + t, where t = ln(μ/μ₀)
        let config = RgeFlowConfig {
            coupling_name: "g_lin".to_string(),
            beta_script: "1.0".to_string(),
            initial_value: 0.0,
            mu_min: 1.0,                 // ln(1) = 0
            mu_max: std::f64::consts::E, // ln(e) = 1
            n_points: 1001,
            extra_constants: Default::default(),
        };
        let result = run_rge_flow(&config).unwrap();
        let last = *result.coupling_values.last().unwrap();
        // g(ln(e)) = 0 + 1 = 1.0
        assert!((last - 1.0).abs() < 1e-6, "Expected ~1.0, got {}", last);
    }

    #[test]
    fn rge_qcd_like_asymptotic_freedom() {
        // β(g) = -b₀ g³ / (16π²), b₀ = 7 (nf=5 QCD-like)
        // At higher μ, the coupling should decrease (asymptotic freedom).
        let config = RgeFlowConfig {
            coupling_name: "alpha_s".to_string(),
            beta_script: "let b0 = 7.0; -b0 * g * g * g / (16.0 * PI * PI)".to_string(),
            initial_value: 0.3,
            mu_min: 2.0,
            mu_max: 1000.0,
            n_points: 500,
            extra_constants: Default::default(),
        };
        let result = run_rge_flow(&config).unwrap();
        let first = result.coupling_values[0];
        let last = *result.coupling_values.last().unwrap();
        assert!(
            last < first,
            "QCD coupling should decrease: g(2 GeV)={} > g(1000 GeV)={}",
            first,
            last
        );
    }

    #[test]
    fn rge_extra_constants() {
        // Pass nf as an extra constant.
        let mut extras = std::collections::HashMap::new();
        extras.insert("nf".to_string(), 6.0);

        let config = RgeFlowConfig {
            coupling_name: "g_test".to_string(),
            beta_script: "let b0 = 11.0 - 2.0 * nf / 3.0; -b0 * g * g * g / (16.0 * PI * PI)"
                .to_string(),
            initial_value: 0.3,
            mu_min: 10.0,
            mu_max: 1000.0,
            n_points: 100,
            extra_constants: extras,
        };
        let result = run_rge_flow(&config).unwrap();
        assert_eq!(result.coupling_values.len(), 100);
    }

    #[test]
    fn rge_invalid_mu_range() {
        let config = RgeFlowConfig {
            coupling_name: "g".to_string(),
            beta_script: "0.0".to_string(),
            initial_value: 0.1,
            mu_min: 100.0,
            mu_max: 1.0, // Invalid: min > max
            n_points: 50,
            extra_constants: Default::default(),
        };
        assert!(run_rge_flow(&config).is_err());
    }

    #[test]
    fn rge_negative_mu_error() {
        let config = RgeFlowConfig {
            coupling_name: "g".to_string(),
            beta_script: "0.0".to_string(),
            initial_value: 0.1,
            mu_min: -1.0, // Invalid
            mu_max: 100.0,
            n_points: 50,
            extra_constants: Default::default(),
        };
        assert!(run_rge_flow(&config).is_err());
    }

    #[test]
    fn rge_too_few_points_error() {
        let config = RgeFlowConfig {
            coupling_name: "g".to_string(),
            beta_script: "0.0".to_string(),
            initial_value: 0.1,
            mu_min: 1.0,
            mu_max: 100.0,
            n_points: 1, // Invalid: need at least 2
            extra_constants: Default::default(),
        };
        assert!(run_rge_flow(&config).is_err());
    }

    #[test]
    fn rge_result_endpoints() {
        let config = RgeFlowConfig {
            coupling_name: "g".to_string(),
            beta_script: "0.0".to_string(),
            initial_value: 1.23,
            mu_min: 5.0,
            mu_max: 500.0,
            n_points: 10,
            extra_constants: Default::default(),
        };
        let result = run_rge_flow(&config).unwrap();
        assert!((result.mu_values[0] - 5.0).abs() < 1e-10);
        assert!((result.mu_values.last().unwrap() - 500.0).abs() < 0.5);
        assert!((result.coupling_values[0] - 1.23).abs() < 1e-10);
    }
}
