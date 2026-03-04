//! # Theoretical Consistency Validator
//!
//! Validates proposed Lagrangian terms against the fundamental requirements of
//! a consistent quantum field theory:
//!
//! 1. **Lorentz invariance** — All Lorentz indices must be contracted (Einstein
//!    summation convention). The term must transform as a Lorentz scalar.
//!
//! 2. **Gauge singlet** — The term must be invariant under all gauge symmetries
//!    of the model. For $SU(N)$: the $N$-ality (sum of representation dimensions
//!    mod $N$) must be zero. For $U(1)$: the sum of charges must vanish.
//!
//! 3. **Hermiticity** — The Lagrangian density must be Hermitian:
//!    $\mathcal{L} = \mathcal{L}^\dagger$. Each interaction term must have a
//!    conjugate partner (or be self-conjugate).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ast::{
    validate_index_contraction, FieldSpin, LagrangianExpr,
};
use crate::groups::{GaugeSymmetry, LieGroup, LieGroupRepresentation};

// ---------------------------------------------------------------------------
// Validation Result
// ---------------------------------------------------------------------------

/// The result of validating a Lagrangian term against theoretical consistency
/// requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether all Lorentz indices are properly contracted (term is a scalar).
    pub is_lorentz_scalar: bool,
    /// Whether the term is a gauge singlet under all gauge group factors.
    pub is_gauge_singlet: bool,
    /// Whether the term is Hermitian (self-conjugate or has a conjugate partner).
    pub is_hermitian: bool,
    /// Whether the operator dimension is ≤ 4 (renormalisable).
    pub is_renormalisable: bool,
    /// The mass dimension of the operator.
    pub mass_dimension: u8,
    /// Diagnostic messages for each check.
    pub messages: Vec<ValidationMessage>,
}

/// A single diagnostic message from validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity: "error", "warning", or "info".
    pub severity: String,
    /// The check that produced this message.
    pub check: String,
    /// Human-readable description.
    pub message: String,
}

impl ValidationResult {
    /// Returns `true` if all consistency checks pass.
    pub fn is_valid(&self) -> bool {
        self.is_lorentz_scalar && self.is_gauge_singlet && self.is_hermitian
    }
}

// ---------------------------------------------------------------------------
// Field Representation Info (for gauge checks)
// ---------------------------------------------------------------------------

/// Gauge representation information for a field, used for singlet validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldGaugeInfo {
    /// The field ID.
    pub field_id: String,
    /// Representations under each gauge group factor.
    pub representations: Vec<LieGroupRepresentation>,
    /// U(1) charges (e.g., hypercharge).
    pub u1_charges: HashMap<String, f64>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform all consistency checks on a Lagrangian term.
///
/// # Arguments
/// * `expr` — The parsed Lagrangian AST.
/// * `gauge_symmetry` — Optional gauge symmetry of the model.
/// * `field_gauge_info` — Gauge representation info for each field.
///
/// # Returns
/// A `ValidationResult` with the outcome of each check.
pub fn validate_lagrangian_term(
    expr: &LagrangianExpr,
    gauge_symmetry: Option<&GaugeSymmetry>,
    field_gauge_info: &HashMap<String, FieldGaugeInfo>,
) -> ValidationResult {
    let mut messages = Vec::new();

    // 1. Lorentz invariance check.
    let is_lorentz_scalar = check_lorentz_invariance(expr, &mut messages);

    // 2. Gauge singlet check.
    let is_gauge_singlet =
        check_gauge_singlet(expr, gauge_symmetry, field_gauge_info, &mut messages);

    // 3. Hermiticity check.
    let is_hermitian = check_hermiticity(expr, &mut messages);

    // 4. Mass dimension and renormalisability.
    let mass_dimension = compute_mass_dimension(expr);
    let is_renormalisable = mass_dimension <= 4;

    if is_renormalisable {
        messages.push(ValidationMessage {
            severity: "info".to_string(),
            check: "renormalisability".to_string(),
            message: format!("Operator dimension {} ≤ 4: renormalisable.", mass_dimension),
        });
    } else {
        messages.push(ValidationMessage {
            severity: "warning".to_string(),
            check: "renormalisability".to_string(),
            message: format!(
                "Operator dimension {} > 4: non-renormalisable (effective operator).",
                mass_dimension
            ),
        });
    }

    ValidationResult {
        is_lorentz_scalar,
        is_gauge_singlet,
        is_hermitian,
        is_renormalisable,
        mass_dimension,
        messages,
    }
}

// ---------------------------------------------------------------------------
// Lorentz Invariance
// ---------------------------------------------------------------------------

/// Check that all Lorentz indices are properly contracted.
///
/// A valid Lagrangian term must be a Lorentz scalar: every Lorentz index
/// appears exactly twice (once upper, once lower).
fn check_lorentz_invariance(
    expr: &LagrangianExpr,
    messages: &mut Vec<ValidationMessage>,
) -> bool {
    match validate_index_contraction(expr) {
        Ok(()) => {
            messages.push(ValidationMessage {
                severity: "info".to_string(),
                check: "lorentz".to_string(),
                message: "All Lorentz indices properly contracted.".to_string(),
            });
            true
        }
        Err(e) => {
            messages.push(ValidationMessage {
                severity: "error".to_string(),
                check: "lorentz".to_string(),
                message: format!("Lorentz invariance violation: {}", e),
            });
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Gauge Singlet
// ---------------------------------------------------------------------------

/// Check that the term is invariant under all gauge symmetries.
///
/// For each gauge group factor:
/// - **$U(1)$**: the sum of charges of all fields must be zero.
/// - **$SU(N)$**: the $N$-ality (product of representations) must yield
///   a singlet. In the simplest check, we verify that the sum of the
///   "N-ality indices" (dimension mod N) is 0 mod N.
fn check_gauge_singlet(
    expr: &LagrangianExpr,
    gauge_symmetry: Option<&GaugeSymmetry>,
    field_gauge_info: &HashMap<String, FieldGaugeInfo>,
    messages: &mut Vec<ValidationMessage>,
) -> bool {
    let gs = match gauge_symmetry {
        Some(gs) => gs,
        None => {
            messages.push(ValidationMessage {
                severity: "info".to_string(),
                check: "gauge".to_string(),
                message: "No gauge symmetry specified; skipping gauge singlet check.".to_string(),
            });
            return true;
        }
    };

    // Collect all field IDs from the expression.
    let field_ids = collect_field_ids(expr);
    if field_ids.is_empty() {
        messages.push(ValidationMessage {
            severity: "info".to_string(),
            check: "gauge".to_string(),
            message: "No fields in expression; trivially gauge invariant.".to_string(),
        });
        return true;
    }

    let mut all_ok = true;

    for group in &gs.groups {
        match group {
            LieGroup::U1 { label } => {
                // Sum of U(1) charges must be zero.
                let mut charge_sum = 0.0;
                for (fid, is_adj) in &field_ids {
                    if let Some(info) = field_gauge_info.get(fid.as_str()) {
                        let q = info.u1_charges.get(label).copied().unwrap_or(0.0);
                        charge_sum += if *is_adj { -q } else { q };
                    }
                }
                if charge_sum.abs() > 1e-10 {
                    messages.push(ValidationMessage {
                        severity: "error".to_string(),
                        check: "gauge".to_string(),
                        message: format!(
                            "U(1)_{} charge not conserved: sum = {:.4}",
                            label, charge_sum
                        ),
                    });
                    all_ok = false;
                } else {
                    messages.push(ValidationMessage {
                        severity: "info".to_string(),
                        check: "gauge".to_string(),
                        message: format!("U(1)_{} charge conserved.", label),
                    });
                }
            }
            LieGroup::SU { n, label } => {
                // N-ality check: sum of representation dimensions mod N.
                let mut nality_sum: usize = 0;
                for (fid, is_adj) in &field_ids {
                    if let Some(info) = field_gauge_info.get(fid.as_str()) {
                        for rep in &info.representations {
                            if let LieGroup::SU { n: rn, label: rl } = &rep.group {
                                if *rl == *label && *rn == *n {
                                    let d = rep.dimension as usize;
                                    // Conjugate flips N-ality: k → N-k.
                                    if *is_adj || rep.is_conjugate {
                                        nality_sum += *n as usize - (d % *n as usize);
                                    } else {
                                        nality_sum += d % *n as usize;
                                    }
                                }
                            }
                        }
                    }
                }
                if nality_sum % (*n as usize) != 0 {
                    messages.push(ValidationMessage {
                        severity: "error".to_string(),
                        check: "gauge".to_string(),
                        message: format!(
                            "SU({}){} N-ality violation: sum mod {} = {}",
                            n,
                            if label.is_empty() {
                                String::new()
                            } else {
                                format!("_{}", label)
                            },
                            n,
                            nality_sum % (*n as usize)
                        ),
                    });
                    all_ok = false;
                } else {
                    messages.push(ValidationMessage {
                        severity: "info".to_string(),
                        check: "gauge".to_string(),
                        message: format!(
                            "SU({}){} N-ality check passed.",
                            n,
                            if label.is_empty() {
                                String::new()
                            } else {
                                format!("_{}", label)
                            }
                        ),
                    });
                }
            }
            LieGroup::SO { n, label } => {
                // For SO(N), the singlet condition is more involved.
                // Simple check: all fields in real representations.
                messages.push(ValidationMessage {
                    severity: "info".to_string(),
                    check: "gauge".to_string(),
                    message: format!(
                        "SO({}){} gauge check: assumed valid (real representations).",
                        n,
                        if label.is_empty() {
                            String::new()
                        } else {
                            format!("_{}", label)
                        }
                    ),
                });
            }
        }
    }

    all_ok
}

/// Collect all field IDs and their adjoint status from an expression.
fn collect_field_ids(expr: &LagrangianExpr) -> Vec<(String, bool)> {
    let mut ids = Vec::new();
    collect_field_ids_recursive(expr, &mut ids);
    ids
}

fn collect_field_ids_recursive(expr: &LagrangianExpr, out: &mut Vec<(String, bool)>) {
    match expr {
        LagrangianExpr::FieldOp {
            field_id,
            is_adjoint,
            ..
        } => {
            out.push((field_id.clone(), *is_adjoint));
        }
        LagrangianExpr::FieldStrength { field_id, .. } => {
            out.push((field_id.clone(), false));
        }
        LagrangianExpr::Product(children) | LagrangianExpr::Sum(children) => {
            for child in children {
                collect_field_ids_recursive(child, out);
            }
        }
        LagrangianExpr::Scaled { inner, .. } => {
            collect_field_ids_recursive(inner, out);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Hermiticity
// ---------------------------------------------------------------------------

/// Check whether the term is Hermitian (self-conjugate).
///
/// A term is self-conjugate if:
/// - Every fermion bilinear $\bar\psi \Gamma \psi$ has its conjugate partner.
/// - Complex scalar couplings appear with their conjugates.
///
/// This is a heuristic check: we verify that for every fermion field
/// appearing as $\psi$, there is a corresponding $\bar\psi$ (and vice versa).
fn check_hermiticity(
    expr: &LagrangianExpr,
    messages: &mut Vec<ValidationMessage>,
) -> bool {
    let field_ids = collect_field_ids(expr);

    // Count fields and their adjoints.
    let mut field_count: HashMap<String, (usize, usize)> = HashMap::new();
    for (fid, is_adj) in &field_ids {
        let entry = field_count.entry(fid.clone()).or_insert((0, 0));
        if *is_adj {
            entry.1 += 1;
        } else {
            entry.0 += 1;
        }
    }

    let mut is_hermitian = true;

    for (fid, (normal, adjoint)) in &field_count {
        // For fermions: every ψ should have a ψ̄.
        // For complex scalars: every φ should have a φ†.
        // Real scalars can appear without a dagger.
        if *normal != *adjoint && *adjoint > 0 {
            // There's an adjoint but counts don't match → possibly non-Hermitian.
            // This is a heuristic; some valid terms have unequal counts
            // if they appear in a sum with their conjugate.
            messages.push(ValidationMessage {
                severity: "warning".to_string(),
                check: "hermiticity".to_string(),
                message: format!(
                    "Field '{}': {} normal vs {} adjoint occurrences. \
                     Ensure the conjugate term exists in the full Lagrangian.",
                    fid, normal, adjoint
                ),
            });
        }
    }

    // Check that fermions always appear in bilinear pairs (ψ̄...ψ).
    for (fid, (normal, adjoint)) in &field_count {
        if *normal > 0 && *adjoint == 0 {
            // A fermion field with no bar → check if it's a scalar (ok) or fermion (bad).
            // We can't determine spin from field_id alone without context,
            // so we rely on the AST node's FieldSpin.
            let has_fermion = has_unmatched_fermion(expr, fid);
            if has_fermion {
                messages.push(ValidationMessage {
                    severity: "error".to_string(),
                    check: "hermiticity".to_string(),
                    message: format!(
                        "Fermion field '{}' appears without its adjoint ψ̄. \
                         Fermion bilinears require ψ̄...ψ pairs.",
                        fid
                    ),
                });
                is_hermitian = false;
            }
        }
    }

    if is_hermitian {
        messages.push(ValidationMessage {
            severity: "info".to_string(),
            check: "hermiticity".to_string(),
            message: "Hermiticity check passed (field pairing consistent).".to_string(),
        });
    }

    is_hermitian
}

/// Check if a specific field ID corresponds to an unmatched fermion in the expression.
fn has_unmatched_fermion(expr: &LagrangianExpr, target_id: &str) -> bool {
    match expr {
        LagrangianExpr::FieldOp {
            field_id,
            spin: FieldSpin::Fermion,
            is_adjoint: false,
            ..
        } => field_id == target_id,
        LagrangianExpr::Product(children) | LagrangianExpr::Sum(children) => {
            children.iter().any(|c| has_unmatched_fermion(c, target_id))
        }
        LagrangianExpr::Scaled { inner, .. } => has_unmatched_fermion(inner, target_id),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Mass Dimension
// ---------------------------------------------------------------------------

/// Compute the mass dimension of a Lagrangian term in $d = 4$ spacetime.
///
/// Mass dimensions:
/// - Scalar field: 1
/// - Fermion field: 3/2 → we use integer × 2 representation.
/// - Vector field: 1
/// - Derivative: 1
/// - Coupling constant: 0 (dimensionless by default)
/// - Gamma matrix: 0
/// - Metric tensor: 0
///
/// We return the dimension rounded to the nearest integer.
pub fn compute_mass_dimension(expr: &LagrangianExpr) -> u8 {
    // Use half-integer units: scalar = 2, fermion = 3, vector = 2, derivative = 2.
    let half_dim = compute_half_mass_dimension(expr);
    // The Lagrangian density has dimension 4 in d=4, so the operator
    // dimension is just the sum of field dimensions.
    (half_dim / 2) as u8
}

fn compute_half_mass_dimension(expr: &LagrangianExpr) -> u32 {
    match expr {
        LagrangianExpr::RealConstant(_) => 0,
        LagrangianExpr::CouplingConstant { .. } => 0,
        LagrangianExpr::FieldOp { spin, .. } => match spin {
            FieldSpin::Scalar => 2,
            FieldSpin::Fermion => 3,
            FieldSpin::Vector => 2,
            FieldSpin::ThreeHalf => 5,
            FieldSpin::Tensor2 => 2,
        },
        LagrangianExpr::GammaMat { .. } => 0,
        LagrangianExpr::Gamma5 => 0,
        LagrangianExpr::Derivative { .. } => 2,
        LagrangianExpr::CovariantDerivative { .. } => 2,
        LagrangianExpr::Metric { .. } => 0,
        LagrangianExpr::FieldStrength { .. } => 4, // F ~ ∂A, dimension 2
        LagrangianExpr::Product(children) => {
            children.iter().map(|c| compute_half_mass_dimension(c)).sum()
        }
        LagrangianExpr::Sum(children) => {
            // All terms in a sum should have the same dimension; take the max.
            children
                .iter()
                .map(|c| compute_half_mass_dimension(c))
                .max()
                .unwrap_or(0)
        }
        LagrangianExpr::Scaled { inner, .. } => compute_half_mass_dimension(inner),
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
    fn lorentz_valid_qed_vertex() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(result.is_lorentz_scalar);
    }

    #[test]
    fn lorentz_invalid_uncontracted() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(!result.is_lorentz_scalar);
        assert!(result.messages.iter().any(|m| m.severity == "error"));
    }

    #[test]
    fn hermiticity_valid_bilinear() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(result.is_hermitian);
    }

    #[test]
    fn hermiticity_invalid_lone_fermion() {
        let fields = qed_fields();
        // psi alone without psibar → non-Hermitian
        let expr = parse_lagrangian_term("g * psi * phi", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(!result.is_hermitian);
    }

    #[test]
    fn mass_dimension_qed_vertex() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        // ψ̄ (3/2) + ψ (3/2) + A (1) = 4 → renormalisable
        let dim = compute_mass_dimension(&expr);
        assert_eq!(dim, 4);
    }

    #[test]
    fn mass_dimension_scalar_quartic() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("lambda * phi * phi * phi * phi", &fields).unwrap();
        // 4 × φ (1) = 4 → renormalisable
        let dim = compute_mass_dimension(&expr);
        assert_eq!(dim, 4);
    }

    #[test]
    fn mass_dimension_non_renormalisable() {
        let fields = qed_fields();
        // Six scalar fields → dimension 6 → non-renormalisable
        let expr = parse_lagrangian_term(
            "g * phi * phi * phi * phi * phi * phi",
            &fields,
        )
        .unwrap();
        let dim = compute_mass_dimension(&expr);
        assert_eq!(dim, 6);
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(!result.is_renormalisable);
    }

    #[test]
    fn gauge_singlet_u1_valid() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();

        let gs = GaugeSymmetry {
            groups: vec![LieGroup::U1 {
                label: "EM".to_string(),
            }],
            label: "QED".to_string(),
        };

        let mut field_info: HashMap<String, FieldGaugeInfo> = HashMap::new();
        field_info.insert(
            "psi".to_string(),
            FieldGaugeInfo {
                field_id: "psi".to_string(),
                representations: vec![],
                u1_charges: {
                    let mut c = HashMap::new();
                    c.insert("EM".to_string(), -1.0);
                    c
                },
            },
        );
        field_info.insert(
            "A".to_string(),
            FieldGaugeInfo {
                field_id: "A".to_string(),
                representations: vec![],
                u1_charges: {
                    let mut c = HashMap::new();
                    c.insert("EM".to_string(), 0.0);
                    c
                },
            },
        );

        let result = validate_lagrangian_term(&expr, Some(&gs), &field_info);
        // ψ̄ has charge +1, ψ has charge -1, A has charge 0 → sum = 0.
        assert!(result.is_gauge_singlet);
    }

    #[test]
    fn gauge_singlet_u1_violation() {
        let fields = qed_fields();
        // psi * psi → both have charge -1 → sum = -2 ≠ 0
        let expr = parse_lagrangian_term("g * psi * psi", &fields).unwrap();

        let gs = GaugeSymmetry {
            groups: vec![LieGroup::U1 {
                label: "EM".to_string(),
            }],
            label: "QED".to_string(),
        };

        let mut field_info: HashMap<String, FieldGaugeInfo> = HashMap::new();
        field_info.insert(
            "psi".to_string(),
            FieldGaugeInfo {
                field_id: "psi".to_string(),
                representations: vec![],
                u1_charges: {
                    let mut c = HashMap::new();
                    c.insert("EM".to_string(), -1.0);
                    c
                },
            },
        );

        let result = validate_lagrangian_term(&expr, Some(&gs), &field_info);
        assert!(!result.is_gauge_singlet);
    }

    #[test]
    fn validation_result_is_valid() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("e * psibar * gamma^mu * psi * A_mu", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(result.is_valid());
    }

    #[test]
    fn no_gauge_symmetry_passes() {
        let fields = qed_fields();
        let expr = parse_lagrangian_term("lambda * phi * phi * phi * phi", &fields).unwrap();
        let result = validate_lagrangian_term(&expr, None, &HashMap::new());
        assert!(result.is_gauge_singlet);
    }
}
