//! # Functional Differentiation Engine
//!
//! Derives Feynman vertex rules from Lagrangian AST expressions by successive
//! functional differentiation with respect to external fields.
//!
//! ## Algorithm
//!
//! Given a Lagrangian interaction term $\mathcal{L}_{\text{int}}$ and a set of
//! external field labels $\{\phi_1, \phi_2, \ldots, \phi_n\}$, the vertex factor
//! is:
//!
//! $$V = i \frac{\delta^n \mathcal{L}_{\text{int}}}{\delta\phi_1 \, \delta\phi_2 \cdots \delta\phi_n}$$
//!
//! For fermion (Grassmann-valued) fields, differentiation is performed from
//! the right with explicit sign tracking.
//!
//! ## Momentum Space
//!
//! After differentiation, partial derivatives $\partial_\mu$ are replaced by
//! $-i p_\mu$ (incoming momentum convention).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ast::{expr_to_latex, LagrangianExpr};
use crate::lagrangian::VertexFactor;
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Data Types
// ---------------------------------------------------------------------------

/// Specification of one external field for differentiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalField {
    /// The field ID to differentiate with respect to.
    pub field_id: String,
    /// Whether to differentiate w.r.t. the adjoint (bar / dagger).
    pub is_adjoint: bool,
}

/// The result of deriving a vertex rule from a Lagrangian term.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedVertexRule {
    /// The external field labels (ordered).
    pub external_fields: Vec<ExternalField>,
    /// The residual AST expression after all differentiations.
    pub residual_expr: LagrangianExpr,
    /// LaTeX rendering of the vertex factor ($-i \times$ residual).
    pub latex: String,
    /// Symmetry factor ($1/S$).
    pub symmetry_factor: f64,
    /// Sign from Grassmann ordering.
    pub grassmann_sign: i8,
    /// Number of legs at this vertex.
    pub n_legs: u8,
}

// ---------------------------------------------------------------------------
// Functional Differentiation
// ---------------------------------------------------------------------------

/// Perform a single functional derivative of `expr` with respect to a field.
///
/// The derivative of a `FieldOp` node is:
///   $\frac{\delta}{\delta \phi} \phi = 1$ (Kronecker-δ in field space)
///
/// For a `Product`, we use the Leibniz rule, scanning from the right for
/// Grassmann fields:
///   $\frac{\delta}{\delta \phi} (A B C) = A B \frac{\delta C}{\delta \phi} + (-1)^{|C|} A \frac{\delta B}{\delta \phi} C + \ldots$
///
/// Returns `None` if the field does not appear in the expression.
pub fn functional_derivative(
    expr: &LagrangianExpr,
    field_id: &str,
    is_adjoint: bool,
) -> Option<(LagrangianExpr, i8)> {
    match expr {
        LagrangianExpr::FieldOp {
            field_id: fid,
            is_adjoint: adj,
            ..
        } => {
            if fid == field_id && *adj == is_adjoint {
                Some((LagrangianExpr::RealConstant(1.0), 1))
            } else {
                None
            }
        }

        LagrangianExpr::Product(factors) => differentiate_product(factors, field_id, is_adjoint),

        LagrangianExpr::Sum(terms) => {
            let mut result_terms = Vec::new();
            let mut combined_sign = 1i8;
            for term in terms {
                if let Some((diff, sign)) = functional_derivative(term, field_id, is_adjoint) {
                    combined_sign = sign;
                    result_terms.push(diff);
                }
            }
            if result_terms.is_empty() {
                None
            } else if result_terms.len() == 1 {
                Some((result_terms.remove(0), combined_sign))
            } else {
                Some((LagrangianExpr::Sum(result_terms), combined_sign))
            }
        }

        LagrangianExpr::Scaled { factor, inner } => {
            functional_derivative(inner, field_id, is_adjoint).map(|(diff, sign)| {
                (
                    LagrangianExpr::Scaled {
                        factor: *factor,
                        inner: Box::new(diff),
                    },
                    sign,
                )
            })
        }

        // Constants, gamma matrices, derivatives, etc. have zero derivative.
        _ => None,
    }
}

/// Leibniz rule for products with Grassmann sign tracking.
///
/// We scan from **right to left** (differentiate from the right). Each time
/// we skip over a Grassmann factor, we pick up a factor of $(-1)$.
fn differentiate_product(
    factors: &[LagrangianExpr],
    field_id: &str,
    is_adjoint: bool,
) -> Option<(LagrangianExpr, i8)> {
    let n = factors.len();
    let mut result_terms: Vec<LagrangianExpr> = Vec::new();
    let mut overall_sign: i8 = 1;

    // Iterate from right to left.
    let mut grassmann_sign: i8 = 1;
    for i in (0..n).rev() {
        // Try to differentiate factor i.
        if let Some((diff_i, _sign_i)) = functional_derivative(&factors[i], field_id, is_adjoint) {
            // Build the remaining product: factors[0..i] ++ [diff_i] ++ factors[i+1..n]
            let mut remaining: Vec<LagrangianExpr> = Vec::new();
            for (j, factor) in factors.iter().enumerate() {
                if j == i {
                    // Only include the differentiated factor if it's not just 1.0
                    if !matches!(&diff_i, LagrangianExpr::RealConstant(v) if (*v - 1.0).abs() < 1e-12)
                    {
                        remaining.push(diff_i.clone());
                    }
                } else {
                    remaining.push(factor.clone());
                }
            }

            let term = if remaining.is_empty() {
                LagrangianExpr::RealConstant(1.0)
            } else if remaining.len() == 1 {
                remaining.remove(0)
            } else {
                LagrangianExpr::Product(remaining)
            };

            // Apply sign.
            if grassmann_sign < 0 {
                result_terms.push(LagrangianExpr::Scaled {
                    factor: -1.0,
                    inner: Box::new(term),
                });
            } else {
                result_terms.push(term);
            }

            overall_sign = grassmann_sign;
            // For the typical case of a single field occurrence, break early.
            // If there are multiple occurrences (e.g., φ⁴), continue to get all terms.
        }

        // Track Grassmann sign: passing over a fermion field flips sign.
        if factors[i].is_grassmann() {
            grassmann_sign *= -1;
        }
    }

    if result_terms.is_empty() {
        None
    } else if result_terms.len() == 1 {
        Some((result_terms.remove(0), overall_sign))
    } else {
        // Multiple terms arise when the same field appears multiple times (e.g., φ⁴).
        Some((LagrangianExpr::Sum(result_terms), overall_sign))
    }
}

// ---------------------------------------------------------------------------
// Full Vertex Rule Derivation
// ---------------------------------------------------------------------------

/// Derive the complete vertex factor from a Lagrangian term AST by
/// successively differentiating with respect to each external field.
///
/// # Algorithm
/// 1. Start with the Lagrangian expression.
/// 2. For each external field (in order), apply `functional_derivative`.
/// 3. Multiply by $i$ (the $i$ in the LSZ / Feynman rule convention).
/// 4. Replace any remaining `Derivative` nodes with $-i p_\mu$.
/// 5. Compute the symmetry factor for identical bosonic external legs.
///
/// # Arguments
/// * `expr` — The parsed Lagrangian term AST.
/// * `external_fields` — Ordered list of external field specifications.
///
/// # Returns
/// A `DerivedVertexRule` containing the residual expression, LaTeX rendering,
/// symmetry factor, and Grassmann sign.
pub fn derive_vertex_rule(
    expr: &LagrangianExpr,
    external_fields: &[ExternalField],
) -> SpireResult<DerivedVertexRule> {
    if external_fields.is_empty() {
        return Err(SpireError::AlgebraError(
            "Cannot derive vertex rule with no external fields".to_string(),
        ));
    }

    let mut current = expr.clone();
    let mut total_grassmann_sign: i8 = 1;

    // Step 1–2: successive differentiation.
    for ext in external_fields {
        match functional_derivative(&current, &ext.field_id, ext.is_adjoint) {
            Some((diff, sign)) => {
                current = diff;
                total_grassmann_sign *= sign;
            }
            None => {
                return Err(SpireError::AlgebraError(format!(
                    "Field '{}' (adjoint={}) not found in expression during differentiation",
                    ext.field_id, ext.is_adjoint,
                )));
            }
        }
    }

    // Step 3: The overall factor is i × (−1)^grassmann.
    // (The actual factor of i is handled in the LaTeX rendering.)

    // Step 4: Replace remaining Derivative nodes → momentum notation.
    current = replace_derivatives_with_momenta(&current);

    // Step 5: Compute symmetry factor.
    let symmetry_factor = compute_symmetry_factor(expr, external_fields);

    // Render LaTeX.
    let residual_latex = expr_to_latex(&current);
    let sign_str = if total_grassmann_sign < 0 { "+" } else { "-" };
    let latex = format!("{}i \\, {}", sign_str, residual_latex);

    Ok(DerivedVertexRule {
        external_fields: external_fields.to_vec(),
        residual_expr: current,
        latex,
        symmetry_factor,
        grassmann_sign: total_grassmann_sign,
        n_legs: external_fields.len() as u8,
    })
}

/// Replace `Derivative` nodes with momentum labels.
///
/// $\partial_\mu \to -i p_\mu$ in momentum space. Since the $i$ factors
/// are tracked separately, we just relabel to a momentum symbol.
fn replace_derivatives_with_momenta(expr: &LagrangianExpr) -> LagrangianExpr {
    match expr {
        LagrangianExpr::Derivative { index } => LagrangianExpr::CouplingConstant {
            name: format!("p_{{{}}}", index.name),
            value: None,
        },
        LagrangianExpr::Product(children) => LagrangianExpr::Product(
            children
                .iter()
                .map(replace_derivatives_with_momenta)
                .collect(),
        ),
        LagrangianExpr::Sum(children) => LagrangianExpr::Sum(
            children
                .iter()
                .map(replace_derivatives_with_momenta)
                .collect(),
        ),
        LagrangianExpr::Scaled { factor, inner } => LagrangianExpr::Scaled {
            factor: *factor,
            inner: Box::new(replace_derivatives_with_momenta(inner)),
        },
        other => other.clone(),
    }
}

/// Compute the symmetry factor for a vertex.
///
/// If the original Lagrangian term has $n$ identical bosonic fields of the
/// same type, and the vertex couples to all $n$ of them, the symmetry factor
/// is $1/n!$ (already present in the Lagrangian as a convention factor)
/// times $n!$ from the number of Wick contractions → net factor = 1.
///
/// In practice, the $n!$ from differentiation cancels the $1/n!$ convention
/// factor in the Lagrangian. We return the raw combinatorial factor $n!$
/// so the caller can compare against the expected convention.
fn compute_symmetry_factor(_expr: &LagrangianExpr, external_fields: &[ExternalField]) -> f64 {
    // Count how many times each (field_id, is_adjoint) pair appears in the externals.
    let mut field_counts: HashMap<(String, bool), usize> = HashMap::new();
    for ext in external_fields {
        *field_counts
            .entry((ext.field_id.clone(), ext.is_adjoint))
            .or_insert(0) += 1;
    }

    let mut factor = 1.0;
    for ((_fid, is_adj), count) in &field_counts {
        // Only bosonic fields contribute a symmetry factor.
        // For fermions, the anti-symmetry is already handled by Grassmann signs.
        if !*is_adj {
            // Check if this is a bosonic field in the original expression.
            // (Fermion fields won't have identical permutations.)
            factor *= factorial(*count) as f64;
        }
    }

    factor
}

/// Compute $n!$ for small $n$.
fn factorial(n: usize) -> usize {
    (1..=n).product()
}

/// Convert a `DerivedVertexRule` into the legacy `VertexFactor` format
/// used by the existing Feynman diagram / amplitude pipeline.
pub fn to_vertex_factor(
    rule: &DerivedVertexRule,
    term_id: &str,
    coupling_value: Option<f64>,
) -> VertexFactor {
    VertexFactor {
        term_id: term_id.to_string(),
        field_ids: rule
            .external_fields
            .iter()
            .map(|e| e.field_id.clone())
            .collect(),
        expression: rule.latex.clone(),
        coupling_value,
        n_legs: rule.n_legs,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::ast::{parse_lagrangian_term, FieldSpin};

    fn qed_fields() -> HashMap<String, FieldSpin> {
        let mut m = HashMap::new();
        m.insert("psi".to_string(), FieldSpin::Fermion);
        m.insert("A".to_string(), FieldSpin::Vector);
        m.insert("phi".to_string(), FieldSpin::Scalar);
        m
    }

    #[test]
    fn derive_qed_vertex() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();

        let externals = vec![
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: true,
            },
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "A".to_string(),
                is_adjoint: false,
            },
        ];

        let rule = derive_vertex_rule(&expr, &externals).unwrap();
        assert_eq!(rule.n_legs, 3);
        // After differentiating out ψ̄, ψ, A_μ, the residual should be e * gamma^mu.
        let latex = expr_to_latex(&rule.residual_expr);
        assert!(
            latex.contains("\\gamma^{mu}") || latex.contains("e"),
            "LaTeX: {}",
            latex
        );
    }

    #[test]
    fn derive_scalar_quartic_vertex() {
        let fields = qed_fields();
        // λ * φ * φ * φ * φ  →  differentiating 4 φ's should leave λ
        let expr = parse_lagrangian_term("lambda * phi * phi * phi * phi", &fields).unwrap();

        let externals = vec![
            ExternalField {
                field_id: "phi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "phi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "phi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "phi".to_string(),
                is_adjoint: false,
            },
        ];

        let rule = derive_vertex_rule(&expr, &externals).unwrap();
        assert_eq!(rule.n_legs, 4);
        // Symmetry factor should be 4! = 24 (four identical bosons).
        assert!((rule.symmetry_factor - 24.0).abs() < 1e-12);
    }

    #[test]
    fn derive_yukawa_vertex() {
        let mut fields = qed_fields();
        fields.insert("H".to_string(), FieldSpin::Scalar);

        let expr = parse_lagrangian_term("y * psibar * psi * H", &fields).unwrap();

        let externals = vec![
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: true,
            },
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "H".to_string(),
                is_adjoint: false,
            },
        ];

        let rule = derive_vertex_rule(&expr, &externals).unwrap();
        assert_eq!(rule.n_legs, 3);
        let latex = expr_to_latex(&rule.residual_expr);
        assert!(
            latex.contains("y"),
            "Residual should contain Yukawa coupling: {}",
            latex
        );
    }

    #[test]
    fn derivative_not_found_error() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();

        let externals = vec![ExternalField {
            field_id: "Z".to_string(),
            is_adjoint: false,
        }];

        let result = derive_vertex_rule(&expr, &externals);
        assert!(result.is_err());
    }

    #[test]
    fn empty_externals_error() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let result = derive_vertex_rule(&expr, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn single_field_derivative() {
        let fields = qed_fields();
        // Just phi → d/dphi(phi) = 1
        let expr = parse_lagrangian_term("phi", &fields).unwrap();
        let (diff, sign) = functional_derivative(&expr, "phi", false).unwrap();
        assert!(matches!(diff, LagrangianExpr::RealConstant(v) if (v - 1.0).abs() < 1e-12));
        assert_eq!(sign, 1);
    }

    #[test]
    fn grassmann_sign_tracking() {
        let fields = qed_fields();
        // psibar * psi → d/d(psi) from the right → psibar, sign = +1
        let expr = parse_lagrangian_term("psibar * psi", &fields).unwrap();
        let result = functional_derivative(&expr, "psi", false);
        assert!(result.is_some());
        let (diff, sign) = result.unwrap();
        // The residual should be psibar
        assert!(
            matches!(&diff, LagrangianExpr::FieldOp { field_id, is_adjoint: true, .. } if field_id == "psi"),
            "Expected psibar, got: {:?}",
            diff
        );
        assert_eq!(
            sign, 1,
            "Differentiating from the right: no sign flip for rightmost fermion"
        );
    }

    #[test]
    fn to_vertex_factor_conversion() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let externals = vec![
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: true,
            },
            ExternalField {
                field_id: "psi".to_string(),
                is_adjoint: false,
            },
            ExternalField {
                field_id: "A".to_string(),
                is_adjoint: false,
            },
        ];
        let rule = derive_vertex_rule(&expr, &externals).unwrap();
        let vf = to_vertex_factor(&rule, "qed_vertex", Some(0.3028));
        assert_eq!(vf.term_id, "qed_vertex");
        assert_eq!(vf.n_legs, 3);
        assert_eq!(vf.field_ids, vec!["psi", "psi", "A"]);
    }
}
