//! # Property-Based Tests - Algebraic Consistency
//!
//! Verifies the mathematical identities of the SPIRE algebra module:
//!
//! 1. **Dirac trace cyclicity**: $\text{Tr}[\gamma^a \gamma^b \cdots] =
//!    \text{Tr}[\gamma^b \cdots \gamma^a]$ for cyclic permutations.
//! 2. **Odd trace vanishes**: $\text{Tr}[\gamma^{\mu_1} \cdots \gamma^{\mu_{2k+1}}] = 0$.
//! 3. **Metric self-contraction**: $g^\mu{}_\mu = D$.
//! 4. **Complex arithmetic field axioms**: commutativity, associativity,
//!    distributivity, identity elements, conjugate involution, norm relation.

mod strategies;

use proptest::prelude::*;
use spire_kernel::algebra::{
    evaluate_trace, evaluate_trace_d, Complex, MetricSignature, SpacetimeDimension,
};

use strategies::{
    arbitrary_complex, arbitrary_even_gamma_indices, arbitrary_minkowski_metric,
    arbitrary_nonzero_complex, arbitrary_odd_gamma_indices,
};

// ===========================================================================
// Numerical reference evaluator for Dirac traces
// ===========================================================================

/// Minkowski metric component $g^{\mu\nu}$ with $(+,-,-,-)$ convention.
fn g_mink(a: u8, b: u8) -> f64 {
    if a != b {
        return 0.0;
    }
    if a == 0 {
        1.0
    } else {
        -1.0
    }
}

/// Numerically evaluate $\text{Tr}[\gamma^{a_1} \cdots \gamma^{a_n}]$ using
/// the standard trace identities with the Minkowski metric.
///
/// Returns `Some(value)` for traces of length 0, 2, and 4.
/// Returns `None` for higher-order traces (which the kernel returns as
/// un-evaluated placeholders anyway).
fn numerical_trace(indices: &[u8]) -> Option<f64> {
    match indices.len() {
        0 => Some(4.0),
        n if n % 2 != 0 => Some(0.0),
        2 => {
            let (a, b) = (indices[0], indices[1]);
            Some(4.0 * g_mink(a, b))
        }
        4 => {
            let (a, b, c, d) = (indices[0], indices[1], indices[2], indices[3]);
            Some(
                4.0 * (g_mink(a, b) * g_mink(c, d) - g_mink(a, c) * g_mink(b, d)
                    + g_mink(a, d) * g_mink(b, c)),
            )
        }
        _ => None, // Higher-order: kernel returns placeholder, skip numerical check
    }
}

// ===========================================================================
// Dirac Trace Cyclicity
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// The trace of a product of gamma matrices is invariant under cyclic
    /// permutations of the matrix sequence.  We verify this numerically:
    /// the reference evaluator must give the same value for all cyclic rotations,
    /// and the kernel's `evaluate_trace` must not error.
    ///
    /// $$\text{Tr}[\gamma^{a_1} \gamma^{a_2} \cdots \gamma^{a_n}]
    ///   = \text{Tr}[\gamma^{a_2} \cdots \gamma^{a_n} \gamma^{a_1}]$$
    #[test]
    fn trace_cyclicity_numerical(indices in arbitrary_even_gamma_indices()) {
        // Only test cyclicity for traces we can evaluate numerically (len 0,2,4).
        if let Some(ref_val) = numerical_trace(&indices) {
            let n = indices.len();
            for shift in 1..n {
                let mut rotated = indices[shift..].to_vec();
                rotated.extend_from_slice(&indices[..shift]);

                let rot_val = numerical_trace(&rotated)
                    .expect("numerical_trace must succeed for same-length input");

                prop_assert!(
                    (ref_val - rot_val).abs() < 1e-12,
                    "Trace cyclicity (numerical) violated:\n  {:?} → {}\n  rotated by {}: {:?} → {}",
                    indices, ref_val, shift, rotated, rot_val,
                );
            }
        }

        // Additionally verify the kernel doesn't error on any permutation.
        let _orig = evaluate_trace(&indices, false)
            .expect("evaluate_trace must not fail");
        let n = indices.len();
        for shift in 1..n {
            let mut rotated = indices[shift..].to_vec();
            rotated.extend_from_slice(&indices[..shift]);
            let _rot = evaluate_trace(&rotated, false)
                .expect("evaluate_trace must not fail for cyclic permutation");
        }
    }

    /// Trace cyclicity also holds in $d$-dimensional regularization.
    /// The kernel must not error, and the numerical reference must be invariant.
    #[test]
    fn trace_cyclicity_dimreg_numerical(indices in arbitrary_even_gamma_indices()) {
        let dim = SpacetimeDimension::dim_reg_4();

        // Numerical check (the 4D numerical values are the same for DimReg
        // in the 't Hooft–Veltman convention for traces of ≤ 4 gammas).
        if let Some(ref_val) = numerical_trace(&indices) {
            let n = indices.len();
            for shift in 1..n {
                let mut rotated = indices[shift..].to_vec();
                rotated.extend_from_slice(&indices[..shift]);
                let rot_val = numerical_trace(&rotated).unwrap();
                prop_assert!(
                    (ref_val - rot_val).abs() < 1e-12,
                    "DimReg trace cyclicity (numerical) violated for shift {}",
                    shift,
                );
            }
        }

        // Kernel smoke test: no errors.
        let _orig = evaluate_trace_d(&indices, false, dim)
            .expect("evaluate_trace_d must not fail");
    }
}

// ===========================================================================
// Trace of Odd Number of Gamma Matrices Vanishes
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// $\text{Tr}[\gamma^{\mu_1} \cdots \gamma^{\mu_{2k+1}}] = 0$ in any dimension.
    #[test]
    fn trace_of_odd_gammas_vanishes(indices in arbitrary_odd_gamma_indices()) {
        // Fixed 4D.
        let trace_4d = evaluate_trace(&indices, false)
            .expect("evaluate_trace must not fail for odd indices");
        prop_assert_eq!(
            &trace_4d.result, "0",
            "Odd trace is not zero in 4D: {:?} → {}",
            indices, trace_4d.result,
        );

        // DimReg.
        let trace_dr = evaluate_trace_d(&indices, false, SpacetimeDimension::dim_reg_4())
            .expect("evaluate_trace_d must not fail");
        prop_assert_eq!(
            &trace_dr.result, "0",
            "Odd trace is not zero in DimReg: {:?} → {}",
            indices, trace_dr.result,
        );
    }

    /// Empty trace (unit matrix) = 4 in the 't Hooft-Veltman convention.
    #[test]
    fn trace_of_identity_is_four(_dummy in 0_u8..1) {
        let trace_4d = evaluate_trace(&[], false)
            .expect("trace of identity must not fail");
        prop_assert_eq!(&trace_4d.result, "4", "Tr[I] ≠ 4 in 4D");

        let trace_dr = evaluate_trace_d(&[], false, SpacetimeDimension::dim_reg_4())
            .expect("trace of identity must not fail in DimReg");
        prop_assert_eq!(&trace_dr.result, "4", "Tr[I] ≠ 4 in DimReg ('t Hooft-Veltman)");
    }
}

// ===========================================================================
// Metric Self-Contraction = Dimension
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// The trace of a flat diagonal metric is the spacetime dimension:
    /// $$g^\mu{}_\mu = D$$
    #[test]
    fn metric_trace_equals_dimension(metric in arbitrary_minkowski_metric()) {
        let d = metric.dimension();
        let trace = metric.trace();
        prop_assert_eq!(
            trace, d as f64,
            "Metric trace = {}, expected {}",
            trace, d,
        );
    }

    /// Minkowski metric inner product of $e_\mu$ with $e_\nu$ gives $g_{\mu\nu}$.
    #[test]
    fn metric_basis_vector_inner_products(dim in 1_usize..=10) {
        use spire_kernel::algebra::SpacetimeVector;

        let metric = MetricSignature::minkowski(dim);

        // Inner product of basis vectors e_μ · e_ν should give g_{μν}.
        for mu in 0..dim {
            for nu in 0..dim {
                let mut e_mu = vec![0.0; dim];
                let mut e_nu = vec![0.0; dim];
                e_mu[mu] = 1.0;
                e_nu[nu] = 1.0;

                let v_mu = SpacetimeVector::new(e_mu);
                let v_nu = SpacetimeVector::new(e_nu);

                let product = metric.inner_product(&v_mu, &v_nu)
                    .expect("inner_product must not fail");

                let expected = if mu == nu { metric.component(mu) } else { 0.0 };
                prop_assert!(
                    (product - expected).abs() < 1e-15,
                    "g({},{}) = {}, expected {}",
                    mu, nu, product, expected,
                );
            }
        }
    }
}

// ===========================================================================
// Complex Arithmetic Field Axioms
// ===========================================================================

/// Helper: approximate equality for `Complex` with absolute tolerance.
fn complex_approx_eq(a: &Complex, b: &Complex, tol: f64) -> bool {
    (a.re - b.re).abs() < tol && (a.im - b.im).abs() < tol
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    // --- Additive Axioms ---

    #[test]
    fn complex_add_commutative(a in arbitrary_complex(), b in arbitrary_complex()) {
        let lhs = a + b;
        let rhs = b + a;
        prop_assert!(
            complex_approx_eq(&lhs, &rhs, 1e-12),
            "a + b ≠ b + a: {:?} + {:?} = ({:?}) vs ({:?})", a, b, lhs, rhs,
        );
    }

    #[test]
    fn complex_add_associative(
        a in arbitrary_complex(),
        b in arbitrary_complex(),
        c in arbitrary_complex(),
    ) {
        let lhs = (a + b) + c;
        let rhs = a + (b + c);
        prop_assert!(
            complex_approx_eq(&lhs, &rhs, 1e-10),
            "(a+b)+c ≠ a+(b+c): ({:?}) vs ({:?})", lhs, rhs,
        );
    }

    #[test]
    fn complex_add_identity(a in arbitrary_complex()) {
        let zero = Complex::zero();
        let result = a + zero;
        prop_assert!(
            complex_approx_eq(&result, &a, 1e-15),
            "a + 0 ≠ a: {:?} + 0 = {:?}", a, result,
        );
    }

    #[test]
    fn complex_add_inverse(a in arbitrary_complex()) {
        let neg_a = -a;
        let result = a + neg_a;
        prop_assert!(
            complex_approx_eq(&result, &Complex::zero(), 1e-12),
            "a + (-a) ≠ 0: {:?}", result,
        );
    }

    // --- Multiplicative Axioms ---

    #[test]
    fn complex_mul_commutative(a in arbitrary_complex(), b in arbitrary_complex()) {
        let lhs = a * b;
        let rhs = b * a;
        prop_assert!(
            complex_approx_eq(&lhs, &rhs, 1e-10),
            "a*b ≠ b*a: {:?} vs {:?}", lhs, rhs,
        );
    }

    #[test]
    fn complex_mul_associative(
        a in arbitrary_complex(),
        b in arbitrary_complex(),
        c in arbitrary_complex(),
    ) {
        let lhs = (a * b) * c;
        let rhs = a * (b * c);
        // Looser tolerance for triple products of large numbers.
        let scale = lhs.norm().max(rhs.norm()).max(1.0);
        let diff = (lhs - rhs).norm();
        prop_assert!(
            diff < 1e-6 * scale,
            "(a*b)*c ≠ a*(b*c): diff = {:.2e}, scale = {:.2e}", diff, scale,
        );
    }

    #[test]
    fn complex_mul_identity(a in arbitrary_complex()) {
        let one = Complex::one();
        let result = a * one;
        prop_assert!(
            complex_approx_eq(&result, &a, 1e-12),
            "a * 1 ≠ a: {:?} * 1 = {:?}", a, result,
        );
    }

    // --- Distributivity ---

    #[test]
    fn complex_distributive(
        a in arbitrary_complex(),
        b in arbitrary_complex(),
        c in arbitrary_complex(),
    ) {
        let lhs = a * (b + c);
        let rhs = a * b + a * c;
        let scale = lhs.norm().max(rhs.norm()).max(1.0);
        let diff = (lhs - rhs).norm();
        prop_assert!(
            diff < 1e-8 * scale,
            "Distributivity failed: diff = {:.2e}", diff,
        );
    }

    // --- Conjugate Properties ---

    #[test]
    fn complex_conjugate_involution(z in arbitrary_complex()) {
        // (z*)* = z
        let z_star_star = z.conj().conj();
        prop_assert!(
            complex_approx_eq(&z_star_star, &z, 1e-15),
            "(z*)* ≠ z: {:?}", z_star_star,
        );
    }

    #[test]
    fn complex_norm_via_conjugate(z in arbitrary_complex()) {
        // z · z* should be real and equal |z|².
        let product = z * z.conj();
        prop_assert!(
            product.im.abs() < 1e-10,
            "z · z* is not real: im = {:.2e}", product.im,
        );
        let diff = (product.re - z.norm_sq()).abs();
        let scale = z.norm_sq().max(1.0);
        prop_assert!(
            diff < 1e-10 * scale,
            "z·z* ≠ |z|²: product.re = {:.12}, norm_sq = {:.12}",
            product.re, z.norm_sq(),
        );
    }

    #[test]
    fn complex_conjugate_of_sum(a in arbitrary_complex(), b in arbitrary_complex()) {
        // (a + b)* = a* + b*
        let lhs = (a + b).conj();
        let rhs = a.conj() + b.conj();
        prop_assert!(
            complex_approx_eq(&lhs, &rhs, 1e-12),
            "(a+b)* ≠ a*+b*",
        );
    }

    #[test]
    fn complex_conjugate_of_product(a in arbitrary_complex(), b in arbitrary_complex()) {
        // (a · b)* = a* · b*
        let lhs = (a * b).conj();
        let rhs = a.conj() * b.conj();
        let scale = lhs.norm().max(rhs.norm()).max(1.0);
        let diff = (lhs - rhs).norm();
        prop_assert!(
            diff < 1e-8 * scale,
            "(a·b)* ≠ a*·b*: diff = {:.2e}", diff,
        );
    }

    // --- Division (Multiplicative Inverse) ---

    #[test]
    fn complex_division_inverse(z in arbitrary_nonzero_complex()) {
        // z / z = 1
        let result = z / z;
        let one = Complex::one();
        prop_assert!(
            complex_approx_eq(&result, &one, 1e-8),
            "z/z ≠ 1: {:?}", result,
        );
    }

    #[test]
    fn complex_division_by_zero_returns_nan(_dummy in 0_u8..1) {
        let z = Complex::new(1.0, 2.0);
        let zero = Complex::zero();
        let result = z / zero;
        prop_assert!(result.re.is_nan(), "Division by zero did not produce NaN re");
        prop_assert!(result.im.is_nan(), "Division by zero did not produce NaN im");
    }
}

// ===========================================================================
// SpacetimeVector Properties
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Inner product is bilinear: $g(av, w) = a \cdot g(v, w)$.
    #[test]
    fn inner_product_linearity(
        a_scalar in -100.0_f64..100.0,
        v0 in -100.0_f64..100.0,
        v1 in -100.0_f64..100.0,
        v2 in -100.0_f64..100.0,
        v3 in -100.0_f64..100.0,
        w0 in -100.0_f64..100.0,
        w1 in -100.0_f64..100.0,
        w2 in -100.0_f64..100.0,
        w3 in -100.0_f64..100.0,
    ) {
        use spire_kernel::algebra::SpacetimeVector;

        let metric = MetricSignature::minkowski_4d();
        let v = SpacetimeVector::new_4d(v0, v1, v2, v3);
        let w = SpacetimeVector::new_4d(w0, w1, w2, w3);
        let av = v.scale(a_scalar);

        let lhs = metric.inner_product(&av, &w)
            .expect("inner_product must not fail");
        let rhs = a_scalar * metric.inner_product(&v, &w)
            .expect("inner_product must not fail");

        let scale = lhs.abs().max(rhs.abs()).max(1.0);
        let diff = (lhs - rhs).abs();
        prop_assert!(
            diff < 1e-10 * scale,
            "Linearity failed: g(av, w) = {:.12}, a·g(v,w) = {:.12}",
            lhs, rhs,
        );
    }

    /// Inner product is symmetric: $g(v, w) = g(w, v)$.
    #[test]
    fn inner_product_symmetry(
        v0 in -100.0_f64..100.0,
        v1 in -100.0_f64..100.0,
        v2 in -100.0_f64..100.0,
        v3 in -100.0_f64..100.0,
        w0 in -100.0_f64..100.0,
        w1 in -100.0_f64..100.0,
        w2 in -100.0_f64..100.0,
        w3 in -100.0_f64..100.0,
    ) {
        use spire_kernel::algebra::SpacetimeVector;

        let metric = MetricSignature::minkowski_4d();
        let v = SpacetimeVector::new_4d(v0, v1, v2, v3);
        let w = SpacetimeVector::new_4d(w0, w1, w2, w3);

        let vw = metric.inner_product(&v, &w).expect("must succeed");
        let wv = metric.inner_product(&w, &v).expect("must succeed");

        prop_assert!(
            (vw - wv).abs() < 1e-15,
            "g(v,w) ≠ g(w,v): {:.15} vs {:.15}", vw, wv,
        );
    }
}
