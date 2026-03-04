//! # NLO Counterterm Generator
//!
//! Automated generation of next-to-leading-order (NLO) counterterm vertices
//! from Lagrangian AST expressions via multiplicative field renormalization.
//!
//! ## Algorithm
//!
//! 1. **Field renormalization shift**: Replace each field operator
//!    $\phi \to (1 + \tfrac12 \delta Z_\phi)\,\phi$ in the tree-level AST.
//!
//! 2. **Coupling renormalization**: Replace each coupling constant
//!    $g \to g + \delta g$ in the AST.
//!
//! 3. **Expansion**: Expand all products and collect terms that are
//!    linear in $\delta$ parameters (the NLO counterterms). Drop terms
//!    of order $\delta^2$ and higher.
//!
//! 4. **Vertex extraction**: Feed the counterterm ASTs into the
//!    functional differentiation engine (Phase 32) to produce Feynman
//!    rules for the counterterm vertices.
//!
//! ## Example
//!
//! For the QED vertex term $e \bar\psi \gamma^\mu \psi A_\mu$, the
//! counterterm is:
//! $$\mathcal{L}_{\text{ct}} = (e\,\delta Z_\psi + \tfrac12 e\,\delta Z_A + \delta e)\,\bar\psi \gamma^\mu \psi A_\mu$$

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ast::LagrangianExpr;
use super::derivation::{derive_vertex_rule, DerivedVertexRule, ExternalField};
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Data Structures
// ---------------------------------------------------------------------------

/// A renormalization constant (counterterm parameter).
///
/// Represents $\delta Z_\phi$, $\delta m$, or $\delta g$ — the shift
/// from the bare to the renormalized parameter at one-loop order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenormalizationConstant {
    /// Name of the counterterm (e.g., `"dZ_psi"`, `"dm_e"`, `"de"`).
    pub name: String,
    /// The original parameter this is a counterterm for.
    pub original_parameter: String,
    /// Type of renormalization constant.
    pub kind: CountertermKind,
    /// Known numerical value (if computed from loop integrals).
    pub value: Option<f64>,
}

/// Classification of counterterm types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CountertermKind {
    /// Field strength renormalization: $\phi \to \sqrt{Z}\phi \approx (1 + \tfrac12\delta Z)\phi$.
    FieldStrength,
    /// Mass counterterm: $m \to m + \delta m$.
    Mass,
    /// Coupling counterterm: $g \to g + \delta g$.
    Coupling,
}

/// Specification of how fields and couplings in a term should be renormalized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenormalizationScheme {
    /// Field strength renormalization constants: field_id → counterterm name.
    pub field_renorm: HashMap<String, String>,
    /// Coupling renormalization constants: coupling_name → counterterm name.
    pub coupling_renorm: HashMap<String, String>,
    /// Mass renormalization constants: field_id → counterterm name.
    pub mass_renorm: HashMap<String, String>,
}

/// A generated counterterm expression with its origin information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountertermEntry {
    /// The counterterm AST expression.
    pub expression: LagrangianExpr,
    /// Which renormalization constant(s) this counterterm involves.
    pub delta_parameters: Vec<String>,
    /// Human-readable description.
    pub description: String,
    /// LaTeX rendering.
    pub latex: String,
}

/// Result of the full counterterm generation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountertermResult {
    /// The original tree-level term.
    pub tree_level_expr: LagrangianExpr,
    /// All generated counterterm entries (linear in δ).
    pub counterterms: Vec<CountertermEntry>,
    /// Derived Feynman rules for the counterterm vertices.
    pub counterterm_rules: Vec<DerivedVertexRule>,
    /// Summary of renormalization constants involved.
    pub renorm_constants: Vec<RenormalizationConstant>,
}

// ---------------------------------------------------------------------------
// Field Renormalization Shift
// ---------------------------------------------------------------------------

/// Apply the field renormalization shift to a Lagrangian AST expression.
///
/// Replaces each `FieldOp` node $\phi$ with $\phi + \tfrac12\delta Z_\phi \cdot \phi$.
///
/// The result is a `Sum` of the original expression and all linear-in-$\delta Z$
/// terms. Quadratic and higher terms are dropped.
///
/// # Arguments
///
/// * `expr` — The tree-level Lagrangian expression.
/// * `field_renorm` — Mapping from field_id to the $\delta Z$ counterterm name.
pub fn apply_field_renormalization(
    expr: &LagrangianExpr,
    field_renorm: &HashMap<String, String>,
) -> LagrangianExpr {
    match expr {
        LagrangianExpr::FieldOp {
            field_id,
            spin: _,
            is_adjoint: _,
            lorentz_indices: _,
            gauge_indices: _,
        } => {
            if let Some(dz_name) = field_renorm.get(field_id) {
                // φ → φ + ½δZ·φ
                // Scaled has factor: f64, so use Scaled(0.5, Product([δZ, φ]))
                let original = expr.clone();
                let delta_z = LagrangianExpr::CouplingConstant {
                    name: dz_name.clone(),
                    value: None,
                };
                let delta_times_field = LagrangianExpr::Product(vec![
                    delta_z,
                    original.clone(),
                ]);
                let half_delta_field = LagrangianExpr::Scaled {
                    factor: 0.5,
                    inner: Box::new(delta_times_field),
                };
                LagrangianExpr::Sum(vec![original, half_delta_field])
            } else {
                expr.clone()
            }
        }

        LagrangianExpr::Product(factors) => {
            // For a product A·B·C, the linear-order expansion is:
            // (A+δA)·(B+δB)·(C+δC) ≈ A·B·C + δA·B·C + A·δB·C + A·B·δC
            let shifted_factors: Vec<LagrangianExpr> = factors
                .iter()
                .map(|f| apply_field_renormalization(f, field_renorm))
                .collect();

            let mut terms = vec![LagrangianExpr::Product(factors.clone())]; // tree-level

            for i in 0..factors.len() {
                if let Some(delta_part) = extract_delta_part(&shifted_factors[i]) {
                    let mut ct_factors = Vec::with_capacity(factors.len());
                    for (j, f) in factors.iter().enumerate() {
                        if j == i {
                            ct_factors.push(delta_part.clone());
                        } else {
                            ct_factors.push(f.clone());
                        }
                    }
                    terms.push(LagrangianExpr::Product(ct_factors));
                }
            }

            if terms.len() == 1 {
                terms.into_iter().next().unwrap()
            } else {
                LagrangianExpr::Sum(terms)
            }
        }

        LagrangianExpr::Scaled { factor, inner } => {
            let shifted_inner = apply_field_renormalization(inner, field_renorm);
            LagrangianExpr::Scaled {
                factor: *factor,
                inner: Box::new(shifted_inner),
            }
        }

        LagrangianExpr::Sum(terms) => {
            LagrangianExpr::Sum(
                terms
                    .iter()
                    .map(|t| apply_field_renormalization(t, field_renorm))
                    .collect(),
            )
        }

        _ => expr.clone(),
    }
}

/// Apply coupling renormalization: $g \to g + \delta g$.
///
/// Replaces each `CouplingConstant` node whose name matches a key in
/// `coupling_renorm` with the sum $g + \delta g$.
pub fn apply_coupling_renormalization(
    expr: &LagrangianExpr,
    coupling_renorm: &HashMap<String, String>,
) -> LagrangianExpr {
    match expr {
        LagrangianExpr::CouplingConstant { name, value: _ } => {
            if let Some(dg_name) = coupling_renorm.get(name) {
                let original = expr.clone();
                let delta_g = LagrangianExpr::CouplingConstant {
                    name: dg_name.clone(),
                    value: None,
                };
                LagrangianExpr::Sum(vec![original, delta_g])
            } else {
                expr.clone()
            }
        }

        LagrangianExpr::Product(factors) => {
            LagrangianExpr::Product(
                factors
                    .iter()
                    .map(|f| apply_coupling_renormalization(f, coupling_renorm))
                    .collect(),
            )
        }

        LagrangianExpr::Sum(terms) => {
            LagrangianExpr::Sum(
                terms
                    .iter()
                    .map(|t| apply_coupling_renormalization(t, coupling_renorm))
                    .collect(),
            )
        }

        LagrangianExpr::Scaled { factor, inner } => {
            LagrangianExpr::Scaled {
                factor: *factor,
                inner: Box::new(apply_coupling_renormalization(inner, coupling_renorm)),
            }
        }

        _ => expr.clone(),
    }
}

/// Extract the $\delta$-part from a shifted expression.
///
/// Given `Sum([original, δ_part])`, returns `Some(δ_part)`.
/// Returns `None` for expressions that were not shifted.
fn extract_delta_part(expr: &LagrangianExpr) -> Option<LagrangianExpr> {
    match expr {
        LagrangianExpr::Sum(terms) if terms.len() == 2 => {
            Some(terms[1].clone())
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Linear-Order Counterterm Extraction
// ---------------------------------------------------------------------------

/// Extract all counterterm expressions (linear in $\delta$) from a
/// renormalized Lagrangian expression.
///
/// Takes the tree-level expression and the renormalization scheme,
/// and collects the counterterm terms.
pub fn extract_counterterms(
    tree_level: &LagrangianExpr,
    scheme: &RenormalizationScheme,
) -> Vec<CountertermEntry> {
    let mut counterterms = Vec::new();

    // --- Field strength counterterms ---
    let field_ids = collect_field_ids(tree_level);
    for field_id in &field_ids {
        if let Some(dz_name) = scheme.field_renorm.get(field_id) {
            let delta_z = LagrangianExpr::CouplingConstant {
                name: dz_name.clone(),
                value: None,
            };
            // ½ δZ × tree_level  =  Scaled(0.5, Product([δZ, tree_level]))
            let half_dz_tree = LagrangianExpr::Scaled {
                factor: 0.5,
                inner: Box::new(LagrangianExpr::Product(vec![
                    delta_z,
                    tree_level.clone(),
                ])),
            };

            counterterms.push(CountertermEntry {
                expression: half_dz_tree,
                delta_parameters: vec![dz_name.clone()],
                description: format!(
                    "Field strength counterterm for {} (½{}·L_tree)",
                    field_id, dz_name
                ),
                latex: format!(
                    "\\frac{{1}}{{2}}\\delta Z_{{{}}} \\cdot \\mathcal{{L}}_{{\\text{{tree}}}}",
                    field_id
                ),
            });
        }
    }

    // --- Coupling counterterms ---
    let coupling_names = collect_coupling_names(tree_level);
    for coupling in &coupling_names {
        if let Some(dg_name) = scheme.coupling_renorm.get(coupling) {
            let ct_expr = replace_coupling_with_delta(tree_level, coupling, dg_name);

            counterterms.push(CountertermEntry {
                expression: ct_expr,
                delta_parameters: vec![dg_name.clone()],
                description: format!(
                    "Coupling counterterm: {} → {} in tree-level term",
                    coupling, dg_name
                ),
                latex: format!(
                    "\\delta {} \\cdot \\frac{{\\partial \\mathcal{{L}}}}{{\\partial {}}}",
                    dg_name, coupling
                ),
            });
        }
    }

    // --- Mass counterterms ---
    for field_id in &field_ids {
        if let Some(dm_name) = scheme.mass_renorm.get(field_id) {
            let mass_ct = LagrangianExpr::Product(vec![
                LagrangianExpr::CouplingConstant {
                    name: dm_name.clone(),
                    value: None,
                },
                tree_level.clone(),
            ]);

            counterterms.push(CountertermEntry {
                expression: mass_ct,
                delta_parameters: vec![dm_name.clone()],
                description: format!("Mass counterterm for {} ({}·L_tree)", field_id, dm_name),
                latex: format!(
                    "\\delta m_{{{}}} \\cdot \\mathcal{{L}}_{{\\text{{tree}}}}",
                    field_id
                ),
            });
        }
    }

    counterterms
}

/// Replace a coupling constant with its counterterm in an expression.
fn replace_coupling_with_delta(
    expr: &LagrangianExpr,
    coupling_name: &str,
    delta_name: &str,
) -> LagrangianExpr {
    match expr {
        LagrangianExpr::CouplingConstant { name, .. } if name == coupling_name => {
            LagrangianExpr::CouplingConstant {
                name: delta_name.to_string(),
                value: None,
            }
        }
        LagrangianExpr::Product(factors) => {
            LagrangianExpr::Product(
                factors
                    .iter()
                    .map(|f| replace_coupling_with_delta(f, coupling_name, delta_name))
                    .collect(),
            )
        }
        LagrangianExpr::Sum(terms) => {
            LagrangianExpr::Sum(
                terms
                    .iter()
                    .map(|t| replace_coupling_with_delta(t, coupling_name, delta_name))
                    .collect(),
            )
        }
        LagrangianExpr::Scaled { factor, inner } => {
            LagrangianExpr::Scaled {
                factor: *factor,
                inner: Box::new(replace_coupling_with_delta(inner, coupling_name, delta_name)),
            }
        }
        _ => expr.clone(),
    }
}

// ---------------------------------------------------------------------------
// AST Traversal Helpers
// ---------------------------------------------------------------------------

/// Collect all unique field IDs from an AST expression.
fn collect_field_ids(expr: &LagrangianExpr) -> Vec<String> {
    let mut ids = Vec::new();
    collect_field_ids_inner(expr, &mut ids);
    ids.sort();
    ids.dedup();
    ids
}

fn collect_field_ids_inner(expr: &LagrangianExpr, ids: &mut Vec<String>) {
    match expr {
        LagrangianExpr::FieldOp { field_id, .. } => {
            ids.push(field_id.clone());
        }
        LagrangianExpr::Product(factors) => {
            for f in factors {
                collect_field_ids_inner(f, ids);
            }
        }
        LagrangianExpr::Sum(terms) => {
            for t in terms {
                collect_field_ids_inner(t, ids);
            }
        }
        LagrangianExpr::Scaled { inner, .. } => {
            collect_field_ids_inner(inner, ids);
        }
        _ => {}
    }
}

/// Collect all unique coupling constant names from an AST expression.
fn collect_coupling_names(expr: &LagrangianExpr) -> Vec<String> {
    let mut names = Vec::new();
    collect_coupling_names_inner(expr, &mut names);
    names.sort();
    names.dedup();
    names
}

fn collect_coupling_names_inner(expr: &LagrangianExpr, names: &mut Vec<String>) {
    match expr {
        LagrangianExpr::CouplingConstant { name, .. } => {
            names.push(name.clone());
        }
        LagrangianExpr::Product(factors) => {
            for f in factors {
                collect_coupling_names_inner(f, names);
            }
        }
        LagrangianExpr::Sum(terms) => {
            for t in terms {
                collect_coupling_names_inner(t, names);
            }
        }
        LagrangianExpr::Scaled { inner, .. } => {
            collect_coupling_names_inner(inner, names);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Full Pipeline
// ---------------------------------------------------------------------------

/// Generate NLO counterterms from a tree-level Lagrangian term.
///
/// This is the main entry point for the counterterm generation pipeline.
/// It:
/// 1. Extracts linear-order counterterm expressions.
/// 2. Derives Feynman rules for each counterterm vertex.
/// 3. Collects all renormalization constants.
///
/// # Arguments
///
/// * `tree_level` — The tree-level Lagrangian AST expression.
/// * `scheme` — The renormalization scheme.
/// * `external_fields` — External field specification for vertex rule derivation.
pub fn generate_counterterms(
    tree_level: &LagrangianExpr,
    scheme: &RenormalizationScheme,
    external_fields: &[ExternalField],
) -> SpireResult<CountertermResult> {
    let counterterms = extract_counterterms(tree_level, scheme);

    let mut counterterm_rules = Vec::new();
    for ct in &counterterms {
        match derive_vertex_rule(&ct.expression, external_fields) {
            Ok(rule) => counterterm_rules.push(rule),
            Err(_) => {
                // Some counterterms may not produce valid vertex rules
                // (e.g., if the external fields don't match). Skip gracefully.
            }
        }
    }

    let mut renorm_constants = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (field_id, dz_name) in &scheme.field_renorm {
        if seen.insert(dz_name.clone()) {
            renorm_constants.push(RenormalizationConstant {
                name: dz_name.clone(),
                original_parameter: field_id.clone(),
                kind: CountertermKind::FieldStrength,
                value: None,
            });
        }
    }

    for (coupling, dg_name) in &scheme.coupling_renorm {
        if seen.insert(dg_name.clone()) {
            renorm_constants.push(RenormalizationConstant {
                name: dg_name.clone(),
                original_parameter: coupling.clone(),
                kind: CountertermKind::Coupling,
                value: None,
            });
        }
    }

    for (field_id, dm_name) in &scheme.mass_renorm {
        if seen.insert(dm_name.clone()) {
            renorm_constants.push(RenormalizationConstant {
                name: dm_name.clone(),
                original_parameter: field_id.clone(),
                kind: CountertermKind::Mass,
                value: None,
            });
        }
    }

    Ok(CountertermResult {
        tree_level_expr: tree_level.clone(),
        counterterms,
        counterterm_rules,
        renorm_constants,
    })
}

/// Build a default QED-like renormalization scheme.
///
/// Convenience function that scans an expression for all fields and
/// coupling constants, generating $\delta Z_\phi$ for each field and
/// $\delta g$ for each coupling.
pub fn build_default_scheme(expr: &LagrangianExpr) -> RenormalizationScheme {
    let mut field_renorm = HashMap::new();
    let mut coupling_renorm = HashMap::new();
    let mass_renorm = HashMap::new();

    build_scheme_inner(expr, &mut field_renorm, &mut coupling_renorm);

    RenormalizationScheme {
        field_renorm,
        coupling_renorm,
        mass_renorm,
    }
}

fn build_scheme_inner(
    expr: &LagrangianExpr,
    field_renorm: &mut HashMap<String, String>,
    coupling_renorm: &mut HashMap<String, String>,
) {
    match expr {
        LagrangianExpr::FieldOp { field_id, .. } => {
            if !field_renorm.contains_key(field_id) {
                field_renorm.insert(field_id.clone(), format!("dZ_{}", field_id));
            }
        }
        LagrangianExpr::CouplingConstant { name, .. } => {
            if !coupling_renorm.contains_key(name) {
                coupling_renorm.insert(name.clone(), format!("d{}", name));
            }
        }
        LagrangianExpr::Product(factors) => {
            for f in factors {
                build_scheme_inner(f, field_renorm, coupling_renorm);
            }
        }
        LagrangianExpr::Sum(terms) => {
            for t in terms {
                build_scheme_inner(t, field_renorm, coupling_renorm);
            }
        }
        LagrangianExpr::Scaled { inner, .. } => {
            build_scheme_inner(inner, field_renorm, coupling_renorm);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::ast::{FieldSpin, IndexSlot, IndexKind, IndexPosition};

    /// Build a simple QED vertex term: e * ψ̄ * γ^μ * ψ * A_μ
    fn qed_vertex_expr() -> LagrangianExpr {
        let e = LagrangianExpr::CouplingConstant {
            name: "e".into(),
            value: Some(0.3028),
        };
        let psi_bar = LagrangianExpr::FieldOp {
            field_id: "psi".into(),
            spin: FieldSpin::Fermion,
            is_adjoint: true,
            lorentz_indices: vec![],
            gauge_indices: vec![],
        };
        let gamma = LagrangianExpr::GammaMat {
            index: IndexSlot {
                name: "mu".into(),
                kind: IndexKind::Lorentz,
                position: IndexPosition::Upper,
            },
        };
        let psi = LagrangianExpr::FieldOp {
            field_id: "psi".into(),
            spin: FieldSpin::Fermion,
            is_adjoint: false,
            lorentz_indices: vec![],
            gauge_indices: vec![],
        };
        let a_mu = LagrangianExpr::FieldOp {
            field_id: "A".into(),
            spin: FieldSpin::Vector,
            is_adjoint: false,
            lorentz_indices: vec![IndexSlot {
                name: "mu".into(),
                kind: IndexKind::Lorentz,
                position: IndexPosition::Lower,
            }],
            gauge_indices: vec![],
        };

        LagrangianExpr::Product(vec![e, psi_bar, gamma, psi, a_mu])
    }

    /// Build a mass term: -m * ψ̄ * ψ
    fn mass_term_expr() -> LagrangianExpr {
        let minus_m = LagrangianExpr::Product(vec![
            LagrangianExpr::RealConstant(-1.0),
            LagrangianExpr::CouplingConstant {
                name: "m_e".into(),
                value: Some(0.000511),
            },
        ]);
        let psi_bar = LagrangianExpr::FieldOp {
            field_id: "psi".into(),
            spin: FieldSpin::Fermion,
            is_adjoint: true,
            lorentz_indices: vec![],
            gauge_indices: vec![],
        };
        let psi = LagrangianExpr::FieldOp {
            field_id: "psi".into(),
            spin: FieldSpin::Fermion,
            is_adjoint: false,
            lorentz_indices: vec![],
            gauge_indices: vec![],
        };

        LagrangianExpr::Product(vec![minus_m, psi_bar, psi])
    }

    #[test]
    fn collect_field_ids_from_qed_vertex() {
        let expr = qed_vertex_expr();
        let ids = collect_field_ids(&expr);

        assert!(ids.contains(&"psi".to_string()));
        assert!(ids.contains(&"A".to_string()));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn collect_coupling_names_from_qed_vertex() {
        let expr = qed_vertex_expr();
        let names = collect_coupling_names(&expr);

        assert_eq!(names, vec!["e"]);
    }

    #[test]
    fn build_default_scheme_qed() {
        let expr = qed_vertex_expr();
        let scheme = build_default_scheme(&expr);

        assert!(scheme.field_renorm.contains_key("psi"));
        assert!(scheme.field_renorm.contains_key("A"));
        assert!(scheme.coupling_renorm.contains_key("e"));

        assert_eq!(scheme.field_renorm["psi"], "dZ_psi");
        assert_eq!(scheme.field_renorm["A"], "dZ_A");
        assert_eq!(scheme.coupling_renorm["e"], "de");
    }

    #[test]
    fn extract_counterterms_qed_vertex() {
        let expr = qed_vertex_expr();
        let scheme = build_default_scheme(&expr);
        let cts = extract_counterterms(&expr, &scheme);

        // Should have:
        // - ½δZ_A (field strength for A)
        // - ½δZ_psi (field strength for ψ)
        // - δe (coupling)
        assert!(cts.len() >= 3, "Expected at least 3 counterterms, got {}", cts.len());

        let delta_params: Vec<&str> = cts
            .iter()
            .flat_map(|ct| ct.delta_parameters.iter().map(|s| s.as_str()))
            .collect();

        assert!(delta_params.contains(&"dZ_psi"));
        assert!(delta_params.contains(&"dZ_A"));
        assert!(delta_params.contains(&"de"));
    }

    #[test]
    fn extract_counterterms_mass_term() {
        let expr = mass_term_expr();

        let mut scheme = RenormalizationScheme {
            field_renorm: HashMap::new(),
            coupling_renorm: HashMap::new(),
            mass_renorm: HashMap::new(),
        };
        scheme
            .field_renorm
            .insert("psi".into(), "dZ_psi".into());
        scheme
            .mass_renorm
            .insert("psi".into(), "dm_e".into());
        scheme
            .coupling_renorm
            .insert("m_e".into(), "dm_e_coupling".into());

        let cts = extract_counterterms(&expr, &scheme);

        assert!(cts.len() >= 2, "Expected at least 2 counterterms, got {}", cts.len());

        let has_mass_ct = cts.iter().any(|ct| ct.delta_parameters.contains(&"dm_e".to_string()));
        assert!(has_mass_ct, "Should contain mass counterterm");
    }

    #[test]
    fn apply_field_renormalization_single_field() {
        let field = LagrangianExpr::FieldOp {
            field_id: "psi".into(),
            spin: FieldSpin::Fermion,
            is_adjoint: false,
            lorentz_indices: vec![],
            gauge_indices: vec![],
        };

        let mut field_renorm = HashMap::new();
        field_renorm.insert("psi".into(), "dZ_psi".into());

        let shifted = apply_field_renormalization(&field, &field_renorm);

        // Should be Sum([ψ, Scaled(0.5, Product([dZ_psi, ψ]))])
        match &shifted {
            LagrangianExpr::Sum(terms) => {
                assert_eq!(terms.len(), 2);
                assert!(matches!(&terms[0], LagrangianExpr::FieldOp { .. }));
                assert!(matches!(&terms[1], LagrangianExpr::Scaled { .. }));
            }
            _ => panic!("Expected Sum after field renormalization"),
        }
    }

    #[test]
    fn apply_coupling_renormalization_replaces() {
        let expr = LagrangianExpr::CouplingConstant {
            name: "e".into(),
            value: Some(0.3028),
        };

        let mut coupling_renorm = HashMap::new();
        coupling_renorm.insert("e".into(), "de".into());

        let shifted = apply_coupling_renormalization(&expr, &coupling_renorm);

        match &shifted {
            LagrangianExpr::Sum(terms) => {
                assert_eq!(terms.len(), 2);
                match &terms[0] {
                    LagrangianExpr::CouplingConstant { name, .. } => assert_eq!(name, "e"),
                    _ => panic!("Expected CouplingConstant"),
                }
                match &terms[1] {
                    LagrangianExpr::CouplingConstant { name, .. } => assert_eq!(name, "de"),
                    _ => panic!("Expected CouplingConstant δe"),
                }
            }
            _ => panic!("Expected Sum after coupling renormalization"),
        }
    }

    #[test]
    fn replace_coupling_with_delta_works() {
        let expr = LagrangianExpr::Product(vec![
            LagrangianExpr::CouplingConstant { name: "g".into(), value: None },
            LagrangianExpr::FieldOp {
                field_id: "phi".into(),
                spin: FieldSpin::Scalar,
                is_adjoint: false,
                lorentz_indices: vec![],
                gauge_indices: vec![],
            },
        ]);

        let replaced = replace_coupling_with_delta(&expr, "g", "dg");

        match &replaced {
            LagrangianExpr::Product(factors) => {
                match &factors[0] {
                    LagrangianExpr::CouplingConstant { name, .. } => assert_eq!(name, "dg"),
                    _ => panic!("Expected replaced coupling"),
                }
            }
            _ => panic!("Expected Product"),
        }
    }

    #[test]
    fn generate_counterterms_full_pipeline() {
        let expr = qed_vertex_expr();
        let scheme = build_default_scheme(&expr);

        let external = vec![
            ExternalField { field_id: "psi".into(), is_adjoint: true },
            ExternalField { field_id: "psi".into(), is_adjoint: false },
            ExternalField { field_id: "A".into(), is_adjoint: false },
        ];

        let result = generate_counterterms(&expr, &scheme, &external).unwrap();

        assert!(!result.counterterms.is_empty());
        assert!(!result.renorm_constants.is_empty());

        let kinds: Vec<CountertermKind> = result
            .renorm_constants
            .iter()
            .map(|rc| rc.kind)
            .collect();
        assert!(kinds.contains(&CountertermKind::FieldStrength));
        assert!(kinds.contains(&CountertermKind::Coupling));
    }

    #[test]
    fn renormalization_constant_serde() {
        let rc = RenormalizationConstant {
            name: "dZ_psi".into(),
            original_parameter: "psi".into(),
            kind: CountertermKind::FieldStrength,
            value: Some(0.01),
        };

        let json = serde_json::to_string(&rc).unwrap();
        let deser: RenormalizationConstant = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "dZ_psi");
        assert_eq!(deser.kind, CountertermKind::FieldStrength);
    }

    #[test]
    fn counterterm_result_serde() {
        let expr = LagrangianExpr::RealConstant(1.0);
        let result = CountertermResult {
            tree_level_expr: expr,
            counterterms: vec![],
            counterterm_rules: vec![],
            renorm_constants: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("tree_level_expr"));
    }
}
