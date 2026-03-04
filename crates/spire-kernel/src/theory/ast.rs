//! # Lagrangian Abstract Syntax Tree
//!
//! Defines the symbolic representation of Lagrangian density terms as a
//! composable AST. Each node represents a mathematical object (field operator,
//! gamma matrix, derivative, coupling constant, metric tensor) with explicit
//! tensor index tracking.
//!
//! The AST is intentionally model-agnostic: it can represent terms from
//! QED, QCD, the Standard Model, BSM theories, or any local quantum field
//! theory.
//!
//! ## Index Contraction
//!
//! Every Lorentz and gauge index must appear exactly twice in a valid term
//! (once upper, once lower) according to the Einstein summation convention.
//! The [`validate_index_contraction`] function enforces this.
//!
//! ## String Parser
//!
//! The [`parse_lagrangian_term`] function accepts a human-readable string
//! representation (e.g., `"e * psibar * gamma^mu * psi * A_mu"`) and
//! produces the corresponding AST.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Index System
// ---------------------------------------------------------------------------

/// Classification of a tensor index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexKind {
    /// A spacetime (Lorentz) index: μ, ν, ρ, σ, …
    Lorentz,
    /// A gauge group index: colour a, weak isospin i, …
    /// The string identifies which gauge group factor (e.g., `"SU3_C"`).
    Gauge(String),
}

/// Upper (contravariant) or lower (covariant) position of a tensor index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndexPosition {
    /// Contravariant (upper) index: $T^{\mu}$.
    Upper,
    /// Covariant (lower) index: $T_{\mu}$.
    Lower,
}

/// A single tensor index slot with a name, kind, and position.
///
/// Two index slots with the same name and kind but opposite positions
/// form a contracted pair (Einstein summation).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndexSlot {
    /// Index name (e.g., `"mu"`, `"nu"`, `"a"`).
    pub name: String,
    /// Whether this is a Lorentz or gauge index.
    pub kind: IndexKind,
    /// Upper or lower position.
    pub position: IndexPosition,
}

// ---------------------------------------------------------------------------
// Spin Classification for AST Fields
// ---------------------------------------------------------------------------

/// The Lorentz representation (spin) of a field in the AST.
///
/// This is independent of the `ontology::Spin` type — it classifies the
/// field operator's transformation behaviour for the purpose of Lagrangian
/// construction and Feynman rule derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldSpin {
    /// Spin-0 scalar field: $\phi$, $H$.
    Scalar,
    /// Spin-½ Dirac fermion: $\psi$.
    Fermion,
    /// Spin-1 vector boson: $A_\mu$, $W_\mu$, $G_\mu^a$.
    Vector,
    /// Spin-3/2 Rarita–Schwinger field: $\psi_\mu$.
    ThreeHalf,
    /// Spin-2 tensor field: $h_{\mu\nu}$.
    Tensor2,
}

// ---------------------------------------------------------------------------
// AST Node Types
// ---------------------------------------------------------------------------

/// A node in the Lagrangian expression tree.
///
/// This is the fundamental building block for representing any local field
/// theory interaction term symbolically. Nodes compose via [`LagrangianExpr::Product`] and
/// [`LagrangianExpr::Sum`] to build arbitrarily complex expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LagrangianExpr {
    /// A real numerical constant (e.g., `1.0`, `-0.25`, `24.0`).
    RealConstant(f64),

    /// A named coupling constant (e.g., `e`, `g_s`, `λ`, `y_e`).
    CouplingConstant {
        name: String,
        /// Known numerical value (if available).
        value: Option<f64>,
    },

    /// A field operator representing a quantum field at the interaction point.
    ///
    /// - `is_adjoint = true` → $\bar{\psi}$ (fermion), $\phi^\dagger$ (complex scalar)
    /// - For vector fields, `lorentz_indices` carries the spacetime index.
    /// - For non-singlet fields, `gauge_indices` carries the colour/isospin index.
    FieldOp {
        /// Reference to a field ID in the theoretical model.
        field_id: String,
        /// Spin classification of this field.
        spin: FieldSpin,
        /// Whether this is the adjoint / conjugate operator.
        is_adjoint: bool,
        /// Lorentz indices carried by this field (e.g., μ for $A_\mu$).
        lorentz_indices: Vec<IndexSlot>,
        /// Gauge group indices (e.g., colour index $a$ for $G_\mu^a$).
        gauge_indices: Vec<IndexSlot>,
    },

    /// A Dirac gamma matrix $\gamma^\mu$.
    GammaMat { index: IndexSlot },

    /// The chiral projection matrix $\gamma^5$.
    Gamma5,

    /// A partial derivative $\partial_\mu$ (position space) or
    /// $-i p_\mu$ (after Fourier transform to momentum space).
    Derivative { index: IndexSlot },

    /// A covariant derivative $D_\mu = \partial_\mu - i g A_\mu^a T^a$.
    CovariantDerivative {
        index: IndexSlot,
        /// Optional: the gauge field associated with this covariant derivative.
        gauge_field_id: Option<String>,
    },

    /// The Minkowski metric tensor $g^{\mu\nu}$ or $g_{\mu\nu}$.
    Metric { mu: IndexSlot, nu: IndexSlot },

    /// A field strength tensor $F_{\mu\nu}$ or $G_{\mu\nu}^a$.
    FieldStrength {
        field_id: String,
        mu: IndexSlot,
        nu: IndexSlot,
        gauge_index: Option<IndexSlot>,
    },

    /// An ordered product of sub-expressions: $A \cdot B \cdot C$.
    ///
    /// Order matters for non-commuting objects (fermion fields, gamma matrices).
    Product(Vec<LagrangianExpr>),

    /// A sum of sub-expressions: $A + B + C$.
    Sum(Vec<LagrangianExpr>),

    /// A scaled sub-expression: `factor × inner`.
    Scaled {
        factor: f64,
        inner: Box<LagrangianExpr>,
    },
}

impl LagrangianExpr {
    /// Collect all index slots from this expression (recursive).
    pub fn collect_indices(&self) -> Vec<IndexSlot> {
        let mut indices = Vec::new();
        self.collect_indices_into(&mut indices);
        indices
    }

    fn collect_indices_into(&self, out: &mut Vec<IndexSlot>) {
        match self {
            LagrangianExpr::RealConstant(_)
            | LagrangianExpr::CouplingConstant { .. }
            | LagrangianExpr::Gamma5 => {}

            LagrangianExpr::FieldOp {
                lorentz_indices,
                gauge_indices,
                ..
            } => {
                out.extend(lorentz_indices.iter().cloned());
                out.extend(gauge_indices.iter().cloned());
            }

            LagrangianExpr::GammaMat { index } => out.push(index.clone()),
            LagrangianExpr::Derivative { index } => out.push(index.clone()),
            LagrangianExpr::CovariantDerivative { index, .. } => out.push(index.clone()),

            LagrangianExpr::Metric { mu, nu } => {
                out.push(mu.clone());
                out.push(nu.clone());
            }

            LagrangianExpr::FieldStrength {
                mu,
                nu,
                gauge_index,
                ..
            } => {
                out.push(mu.clone());
                out.push(nu.clone());
                if let Some(gi) = gauge_index {
                    out.push(gi.clone());
                }
            }

            LagrangianExpr::Product(children) | LagrangianExpr::Sum(children) => {
                for child in children {
                    child.collect_indices_into(out);
                }
            }

            LagrangianExpr::Scaled { inner, .. } => {
                inner.collect_indices_into(out);
            }
        }
    }

    /// Returns `true` if this node represents a Grassmann-valued (fermionic) field.
    pub fn is_grassmann(&self) -> bool {
        matches!(
            self,
            LagrangianExpr::FieldOp {
                spin: FieldSpin::Fermion,
                ..
            }
        )
    }

    /// Returns the field ID if this node is a `FieldOp`.
    pub fn field_id(&self) -> Option<&str> {
        match self {
            LagrangianExpr::FieldOp { field_id, .. } => Some(field_id),
            _ => None,
        }
    }

    /// Count how many times a specific field appears in this expression.
    pub fn count_field(&self, id: &str, adjoint: bool) -> usize {
        match self {
            LagrangianExpr::FieldOp {
                field_id,
                is_adjoint,
                ..
            } => {
                if field_id == id && *is_adjoint == adjoint {
                    1
                } else {
                    0
                }
            }
            LagrangianExpr::Product(children) | LagrangianExpr::Sum(children) => {
                children.iter().map(|c| c.count_field(id, adjoint)).sum()
            }
            LagrangianExpr::Scaled { inner, .. } => inner.count_field(id, adjoint),
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Index Contraction Validation
// ---------------------------------------------------------------------------

/// Validate that all tensor indices in the expression are properly contracted.
///
/// Returns `Ok(())` if every index name appears exactly twice (once upper,
/// once lower) within each index kind. Returns an error describing the
/// uncontracted indices otherwise.
pub fn validate_index_contraction(expr: &LagrangianExpr) -> SpireResult<()> {
    let indices = expr.collect_indices();

    // Group by (name, kind).
    let mut groups: HashMap<(String, String), Vec<IndexPosition>> = HashMap::new();
    for idx in &indices {
        let kind_key = match &idx.kind {
            IndexKind::Lorentz => "Lorentz".to_string(),
            IndexKind::Gauge(g) => format!("Gauge({})", g),
        };
        groups
            .entry((idx.name.clone(), kind_key))
            .or_default()
            .push(idx.position);
    }

    let mut errors = Vec::new();
    for ((name, kind), positions) in &groups {
        if positions.len() != 2 {
            errors.push(format!(
                "Index '{}' ({}) appears {} times (expected exactly 2)",
                name,
                kind,
                positions.len()
            ));
            continue;
        }
        let has_upper = positions.iter().any(|p| *p == IndexPosition::Upper);
        let has_lower = positions.iter().any(|p| *p == IndexPosition::Lower);
        if !has_upper || !has_lower {
            errors.push(format!(
                "Index '{}' ({}) does not have both upper and lower positions",
                name, kind
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SpireError::AlgebraError(format!(
            "Index contraction errors: {}",
            errors.join("; ")
        )))
    }
}

// ---------------------------------------------------------------------------
// String Parser
// ---------------------------------------------------------------------------

/// Parse a human-readable Lagrangian term string into an AST.
///
/// # Syntax
///
/// Tokens are separated by `*` (product). Index notation uses `^` for
/// upper indices and `_` for lower indices.
///
/// - `gamma^mu` → `GammaMat { index: mu, Upper }`
/// - `gamma5` → `Gamma5`
/// - `psibar` → `FieldOp { field_id: "psi", is_adjoint: true, spin: Fermion }`
/// - `A_mu` → `FieldOp { field_id: "A", spin: Vector, lorentz_indices: [mu, Lower] }`
/// - `d^mu` or `partial^mu` → `Derivative { index: mu, Upper }`
/// - Numeric literals → `RealConstant`
/// - `g_{mu nu}` → `Metric { mu, nu }`
/// - Everything else → `CouplingConstant`
///
/// # Arguments
/// * `input` — The term string.
/// * `known_fields` — Map of field_id → FieldSpin for resolving field operators.
///
/// # Examples
/// ```text
/// "e * psibar * gamma^mu * psi * A_mu"
/// "-0.25 * F^{mu nu} * F_{mu nu}"
/// "lambda * phi * phi * phi * phi"
/// ```
pub fn parse_lagrangian_term(
    input: &str,
    known_fields: &HashMap<String, FieldSpin>,
) -> SpireResult<LagrangianExpr> {
    let tokens: Vec<&str> = input
        .split('*')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if tokens.is_empty() {
        return Err(SpireError::ModelParseError(
            "Empty Lagrangian term".to_string(),
        ));
    }

    let mut factors: Vec<LagrangianExpr> = Vec::new();

    for token in tokens {
        let node = parse_token(token, known_fields)?;
        factors.push(node);
    }

    if factors.len() == 1 {
        Ok(factors.remove(0))
    } else {
        Ok(LagrangianExpr::Product(factors))
    }
}

/// Parse a single token into an AST node.
fn parse_token(
    token: &str,
    known_fields: &HashMap<String, FieldSpin>,
) -> SpireResult<LagrangianExpr> {
    let token = token.trim();

    // Numeric literal?
    if let Ok(val) = token.parse::<f64>() {
        return Ok(LagrangianExpr::RealConstant(val));
    }

    // Metric tensor: g^{mu nu}, g_{mu nu}, g^{mu_nu}
    if token.starts_with("g^{") || token.starts_with("g_{") {
        return parse_metric(token);
    }

    // Field strength tensor: F^{mu nu}, F_{mu nu}, G^{a,mu nu}
    if (token.starts_with("F^{")
        || token.starts_with("F_{")
        || token.starts_with("G^{")
        || token.starts_with("G_{"))
        && token.contains('{')
    {
        return parse_field_strength(token, known_fields);
    }

    // Gamma matrix: gamma^mu, gamma^nu
    if token.starts_with("gamma^") || token.starts_with("gamma_") {
        let pos = if token.starts_with("gamma^") {
            IndexPosition::Upper
        } else {
            IndexPosition::Lower
        };
        let idx_name = token[6..].trim().to_string();
        return Ok(LagrangianExpr::GammaMat {
            index: IndexSlot {
                name: idx_name,
                kind: IndexKind::Lorentz,
                position: pos,
            },
        });
    }

    // Gamma5
    if token == "gamma5" || token == "γ5" {
        return Ok(LagrangianExpr::Gamma5);
    }

    // Partial derivative: d^mu, d_mu, partial^mu, partial_mu
    if token.starts_with("d^")
        || token.starts_with("d_")
        || token.starts_with("partial^")
        || token.starts_with("partial_")
    {
        let (prefix_len, pos) = if token.starts_with("partial^") {
            (8, IndexPosition::Upper)
        } else if token.starts_with("partial_") {
            (8, IndexPosition::Lower)
        } else if token.starts_with("d^") {
            (2, IndexPosition::Upper)
        } else {
            (2, IndexPosition::Lower)
        };
        let idx_name = token[prefix_len..].trim().to_string();
        return Ok(LagrangianExpr::Derivative {
            index: IndexSlot {
                name: idx_name,
                kind: IndexKind::Lorentz,
                position: pos,
            },
        });
    }

    // Adjoint field: psibar, ebar, etc.
    if token.ends_with("bar") {
        let base = &token[..token.len() - 3];
        let spin = known_fields
            .get(base)
            .copied()
            .unwrap_or(FieldSpin::Fermion);
        return Ok(LagrangianExpr::FieldOp {
            field_id: base.to_string(),
            spin,
            is_adjoint: true,
            lorentz_indices: Vec::new(),
            gauge_indices: Vec::new(),
        });
    }

    // Dagger field: phi†, phidagger
    if token.ends_with("dagger") || token.ends_with("†") {
        let suffix_len = if token.ends_with("dagger") { 6 } else { 3 }; // "†" is 3 bytes in UTF-8
        let base = &token[..token.len() - suffix_len];
        let spin = known_fields.get(base).copied().unwrap_or(FieldSpin::Scalar);
        return Ok(LagrangianExpr::FieldOp {
            field_id: base.to_string(),
            spin,
            is_adjoint: true,
            lorentz_indices: Vec::new(),
            gauge_indices: Vec::new(),
        });
    }

    // Field with index: A_mu, A^mu, W_mu, G_mu, etc.
    if let Some(underscore_pos) = token.find('_') {
        let base = &token[..underscore_pos];
        if let Some(&spin) = known_fields.get(base) {
            let idx_name = token[underscore_pos + 1..].trim().to_string();
            let mut lorentz_indices = Vec::new();
            if spin == FieldSpin::Vector
                || spin == FieldSpin::ThreeHalf
                || spin == FieldSpin::Tensor2
            {
                lorentz_indices.push(IndexSlot {
                    name: idx_name,
                    kind: IndexKind::Lorentz,
                    position: IndexPosition::Lower,
                });
            }
            return Ok(LagrangianExpr::FieldOp {
                field_id: base.to_string(),
                spin,
                is_adjoint: false,
                lorentz_indices,
                gauge_indices: Vec::new(),
            });
        }
    }

    if let Some(caret_pos) = token.find('^') {
        let base = &token[..caret_pos];
        if let Some(&spin) = known_fields.get(base) {
            let idx_name = token[caret_pos + 1..].trim().to_string();
            let mut lorentz_indices = Vec::new();
            if spin == FieldSpin::Vector
                || spin == FieldSpin::ThreeHalf
                || spin == FieldSpin::Tensor2
            {
                lorentz_indices.push(IndexSlot {
                    name: idx_name,
                    kind: IndexKind::Lorentz,
                    position: IndexPosition::Upper,
                });
            }
            return Ok(LagrangianExpr::FieldOp {
                field_id: base.to_string(),
                spin,
                is_adjoint: false,
                lorentz_indices,
                gauge_indices: Vec::new(),
            });
        }
    }

    // Known scalar/fermion field without indices
    if let Some(&spin) = known_fields.get(token) {
        return Ok(LagrangianExpr::FieldOp {
            field_id: token.to_string(),
            spin,
            is_adjoint: false,
            lorentz_indices: Vec::new(),
            gauge_indices: Vec::new(),
        });
    }

    // Default: treat as a coupling constant.
    Ok(LagrangianExpr::CouplingConstant {
        name: token.to_string(),
        value: None,
    })
}

/// Parse a metric tensor token like `g^{mu nu}` or `g_{mu nu}`.
fn parse_metric(token: &str) -> SpireResult<LagrangianExpr> {
    let pos = if token.starts_with("g^") {
        IndexPosition::Upper
    } else {
        IndexPosition::Lower
    };

    // Extract content between braces.
    let brace_start = token.find('{').ok_or_else(|| {
        SpireError::ModelParseError(format!("Expected '{{' in metric tensor: {}", token))
    })?;
    let brace_end = token.find('}').ok_or_else(|| {
        SpireError::ModelParseError(format!("Expected '}}' in metric tensor: {}", token))
    })?;
    let content = &token[brace_start + 1..brace_end];
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(SpireError::ModelParseError(format!(
            "Metric tensor expects exactly 2 indices, got {}: {}",
            parts.len(),
            token
        )));
    }

    Ok(LagrangianExpr::Metric {
        mu: IndexSlot {
            name: parts[0].to_string(),
            kind: IndexKind::Lorentz,
            position: pos,
        },
        nu: IndexSlot {
            name: parts[1].to_string(),
            kind: IndexKind::Lorentz,
            position: pos,
        },
    })
}

/// Parse a field strength tensor token like `F^{mu nu}` or `F_{mu nu}`.
fn parse_field_strength(
    token: &str,
    known_fields: &HashMap<String, FieldSpin>,
) -> SpireResult<LagrangianExpr> {
    let field_char = &token[..1]; // "F" or "G"
    let pos = if token.chars().nth(1) == Some('^') {
        IndexPosition::Upper
    } else {
        IndexPosition::Lower
    };

    let brace_start = token.find('{').ok_or_else(|| {
        SpireError::ModelParseError(format!("Expected '{{' in field strength: {}", token))
    })?;
    let brace_end = token.find('}').ok_or_else(|| {
        SpireError::ModelParseError(format!("Expected '}}' in field strength: {}", token))
    })?;
    let content = &token[brace_start + 1..brace_end];
    let parts: Vec<&str> = content.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(SpireError::ModelParseError(format!(
            "Field strength expects at least 2 indices: {}",
            token
        )));
    }

    // Determine the associated gauge field.
    let field_id = if field_char == "G" {
        // Default to gluon field if known.
        if known_fields.contains_key("G") {
            "G".to_string()
        } else {
            "A".to_string()
        }
    } else {
        "A".to_string()
    };

    Ok(LagrangianExpr::FieldStrength {
        field_id,
        mu: IndexSlot {
            name: parts[parts.len() - 2].trim_end_matches(',').to_string(),
            kind: IndexKind::Lorentz,
            position: pos,
        },
        nu: IndexSlot {
            name: parts[parts.len() - 1].to_string(),
            kind: IndexKind::Lorentz,
            position: pos,
        },
        gauge_index: if parts.len() > 2 {
            Some(IndexSlot {
                name: parts[0].trim_end_matches(',').to_string(),
                kind: IndexKind::Gauge("SU3_C".to_string()),
                position: pos,
            })
        } else {
            None
        },
    })
}

// ---------------------------------------------------------------------------
// LaTeX Rendering
// ---------------------------------------------------------------------------

/// Render a `LagrangianExpr` as a LaTeX string.
pub fn expr_to_latex(expr: &LagrangianExpr) -> String {
    match expr {
        LagrangianExpr::RealConstant(v) => {
            if (*v - v.round()).abs() < 1e-12 {
                format!("{}", *v as i64)
            } else {
                format!("{:.4}", v)
            }
        }
        LagrangianExpr::CouplingConstant { name, .. } => name.clone(),
        LagrangianExpr::FieldOp {
            field_id,
            is_adjoint,
            lorentz_indices,
            ..
        } => {
            let mut s = if *is_adjoint {
                format!("\\bar{{{}}}", field_id)
            } else {
                field_id.clone()
            };
            for idx in lorentz_indices {
                match idx.position {
                    IndexPosition::Upper => s.push_str(&format!("^{{{}}}", idx.name)),
                    IndexPosition::Lower => s.push_str(&format!("_{{{}}}", idx.name)),
                }
            }
            s
        }
        LagrangianExpr::GammaMat { index } => {
            format!("\\gamma^{{{}}}", index.name)
        }
        LagrangianExpr::Gamma5 => "\\gamma^5".to_string(),
        LagrangianExpr::Derivative { index } => match index.position {
            IndexPosition::Upper => format!("\\partial^{{{}}}", index.name),
            IndexPosition::Lower => format!("\\partial_{{{}}}", index.name),
        },
        LagrangianExpr::CovariantDerivative { index, .. } => match index.position {
            IndexPosition::Upper => format!("D^{{{}}}", index.name),
            IndexPosition::Lower => format!("D_{{{}}}", index.name),
        },
        LagrangianExpr::Metric { mu, nu } => {
            format!("g^{{{} {}}}", mu.name, nu.name)
        }
        LagrangianExpr::FieldStrength {
            field_id,
            mu,
            nu,
            gauge_index,
        } => {
            let base = if let Some(gi) = gauge_index {
                format!("{}^{{{}}}", field_id, gi.name)
            } else {
                field_id.clone()
            };
            match mu.position {
                IndexPosition::Upper => format!("{}^{{{} {}}}", base, mu.name, nu.name),
                IndexPosition::Lower => format!("{}_{{{} {}}}", base, mu.name, nu.name),
            }
        }
        LagrangianExpr::Product(children) => {
            let parts: Vec<String> = children.iter().map(|c| expr_to_latex(c)).collect();
            parts.join(" \\, ")
        }
        LagrangianExpr::Sum(children) => {
            let parts: Vec<String> = children.iter().map(|c| expr_to_latex(c)).collect();
            parts.join(" + ")
        }
        LagrangianExpr::Scaled { factor, inner } => {
            if (*factor - 1.0).abs() < 1e-12 {
                expr_to_latex(inner)
            } else if (*factor + 1.0).abs() < 1e-12 {
                format!("-{}", expr_to_latex(inner))
            } else {
                format!("{} \\, {}", factor, expr_to_latex(inner))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn qed_fields() -> HashMap<String, FieldSpin> {
        let mut m = HashMap::new();
        m.insert("psi".to_string(), FieldSpin::Fermion);
        m.insert("A".to_string(), FieldSpin::Vector);
        m.insert("phi".to_string(), FieldSpin::Scalar);
        m
    }

    #[test]
    fn parse_qed_vertex() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert_eq!(factors.len(), 5);
            // coupling
            assert!(
                matches!(&factors[0], LagrangianExpr::CouplingConstant { name, .. } if name == "e")
            );
            // psi-bar
            assert!(
                matches!(&factors[1], LagrangianExpr::FieldOp { field_id, is_adjoint: true, spin: FieldSpin::Fermion, .. } if field_id == "psi")
            );
            // gamma^mu
            assert!(
                matches!(&factors[2], LagrangianExpr::GammaMat { index } if index.name == "mu")
            );
            // psi
            assert!(
                matches!(&factors[3], LagrangianExpr::FieldOp { field_id, is_adjoint: false, spin: FieldSpin::Fermion, .. } if field_id == "psi")
            );
            // A_mu
            assert!(
                matches!(&factors[4], LagrangianExpr::FieldOp { field_id, is_adjoint: false, spin: FieldSpin::Vector, .. } if field_id == "A")
            );
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn parse_scalar_quartic() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("lambda * phi * phi * phi * phi", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert_eq!(factors.len(), 5);
            let phi_count = factors
                .iter()
                .filter(
                    |f| matches!(f, LagrangianExpr::FieldOp { field_id, .. } if field_id == "phi"),
                )
                .count();
            assert_eq!(phi_count, 4);
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn index_contraction_valid_qed() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        assert!(validate_index_contraction(&expr).is_ok());
    }

    #[test]
    fn index_contraction_unmatched() {
        let fields = qed_fields();
        // gamma^mu with no corresponding lower mu → uncontracted
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi", &fields).unwrap();
        let result = validate_index_contraction(&expr);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("mu"));
    }

    #[test]
    fn parse_field_strength_abelian() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("-0.25 * F^{mu nu} * F_{mu nu}", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert_eq!(factors.len(), 3);
            assert!(
                matches!(&factors[0], LagrangianExpr::RealConstant(v) if (*v + 0.25).abs() < 1e-12)
            );
            assert!(
                matches!(&factors[1], LagrangianExpr::FieldStrength { mu, nu, .. } if mu.position == IndexPosition::Upper && nu.position == IndexPosition::Upper)
            );
            assert!(
                matches!(&factors[2], LagrangianExpr::FieldStrength { mu, nu, .. } if mu.position == IndexPosition::Lower && nu.position == IndexPosition::Lower)
            );
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn index_contraction_valid_field_strength() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("-0.25 * F^{mu nu} * F_{mu nu}", &fields).unwrap();
        assert!(validate_index_contraction(&expr).is_ok());
    }

    #[test]
    fn parse_gamma5() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("g * psibar * gamma5 * psi", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert!(matches!(&factors[2], LagrangianExpr::Gamma5));
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn parse_derivative() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("psibar * gamma^mu * d_mu * psi", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert!(
                matches!(&factors[2], LagrangianExpr::Derivative { index } if index.name == "mu" && index.position == IndexPosition::Lower)
            );
        } else {
            panic!("Expected Product");
        }
    }

    #[test]
    fn latex_rendering_qed_vertex() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let latex = expr_to_latex(&expr);
        assert!(latex.contains("\\bar{psi}"));
        assert!(latex.contains("\\gamma^{mu}"));
        assert!(latex.contains("A_{mu}"));
    }

    #[test]
    fn count_field_phi_quartic() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("lambda * phi * phi * phi * phi", &fields).unwrap();
        assert_eq!(expr.count_field("phi", false), 4);
        assert_eq!(expr.count_field("phi", true), 0);
    }

    #[test]
    fn parse_metric_tensor() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("g^{mu nu}", &fields).unwrap();
        assert!(
            matches!(expr, LagrangianExpr::Metric { mu, nu } if mu.name == "mu" && nu.name == "nu")
        );
    }

    #[test]
    fn parse_empty_input_error() {
        let fields = qed_fields();
        let result = parse_lagrangian_term("", &fields);
        assert!(result.is_err());
    }

    #[test]
    fn parse_numeric_constant() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("3.14", &fields).unwrap();
        assert!(matches!(expr, LagrangianExpr::RealConstant(v) if (v - 3.14).abs() < 1e-12));
    }

    #[test]
    fn parse_dagger_field() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("phidagger * phi", &fields).unwrap();
        if let LagrangianExpr::Product(factors) = &expr {
            assert!(
                matches!(&factors[0], LagrangianExpr::FieldOp { field_id, is_adjoint: true, .. } if field_id == "phi")
            );
            assert!(
                matches!(&factors[1], LagrangianExpr::FieldOp { field_id, is_adjoint: false, .. } if field_id == "phi")
            );
        } else {
            panic!("Expected Product");
        }
    }
}
