//! # Fuzzing Harness — Parser & Function Robustness
//!
//! Verifies the **zero-panic** mandate: every public entry point in the
//! kernel that accepts untrusted string or numeric input must return
//! `Err(SpireError)` for malformed data, never panic.
//!
//! ## Targets
//!
//! 1. `parse_model` — arbitrary byte strings must produce `Err`, not panic.
//! 2. `kallen_lambda` / `cm_momentum` / `is_kinematically_allowed` — NaN, Inf,
//!    negative, and zero inputs must not panic.
//! 3. `compute_mandelstam` — pathological momenta must not panic.
//! 4. `apply_lorentz_boost` — superluminal and degenerate boosts must not panic.
//! 5. RAMBO — edge-case energies and masses must not panic.

mod strategies;

use proptest::prelude::*;
use spire_kernel::algebra::FourMomentum;
use spire_kernel::kinematics::{
    apply_lorentz_boost, cm_momentum, compute_mandelstam, is_kinematically_allowed, kallen_lambda,
    LorentzBoost, PhaseSpaceGenerator, RamboGenerator,
};
use spire_kernel::lagrangian::parse_model;

use strategies::{arbitrary_ascii_string, arbitrary_garbage_string};

// ===========================================================================
// 2parse_model Never Panics on Arbitrary Input
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Feeding arbitrary strings into `parse_model` must return `Err`, never panic.
    #[test]
    fn parse_model_garbage_does_not_panic(
        particles in arbitrary_ascii_string(),
        vertices in arbitrary_ascii_string(),
    ) {
        // We don't care whether it returns Ok or Err — only that it doesn't panic.
        let _result = parse_model(&particles, &vertices);
    }

    /// Feeding arbitrary byte sequences (lossy UTF-8) must also not panic.
    #[test]
    fn parse_model_bytes_does_not_panic(
        particles in arbitrary_garbage_string(),
        vertices in arbitrary_garbage_string(),
    ) {
        let _result = parse_model(&particles, &vertices);
    }

    /// Empty strings are a common edge case.
    #[test]
    fn parse_model_empty_strings(_dummy in 0_u8..1) {
        let _result = parse_model("", "");
    }
}

// ===========================================================================
// Kinematic Functions Never Panic
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// `kallen_lambda` with arbitrary f64 inputs (including NaN, Inf) must not panic.
    #[test]
    fn kallen_lambda_no_panic(
        x in prop::num::f64::ANY,
        y in prop::num::f64::ANY,
        z in prop::num::f64::ANY,
    ) {
        let _result = kallen_lambda(x, y, z);
    }

    /// `cm_momentum` with arbitrary inputs must not panic.
    #[test]
    fn cm_momentum_no_panic(
        s in prop::num::f64::ANY,
        m_a in prop::num::f64::ANY,
        m_b in prop::num::f64::ANY,
    ) {
        let _result = cm_momentum(s, m_a, m_b);
    }

    /// `is_kinematically_allowed` with arbitrary inputs must not panic.
    #[test]
    fn is_kinematically_allowed_no_panic(
        energy in prop::num::f64::ANY,
        n_masses in 0_usize..=10,
    ) {
        let masses: Vec<f64> = (0..n_masses).map(|i| i as f64 * 0.1).collect();
        let _result = is_kinematically_allowed(energy, &masses);
    }

    /// `compute_mandelstam` with arbitrary momenta must not panic.
    #[test]
    fn compute_mandelstam_no_panic(
        e1 in prop::num::f64::ANY,
        px1 in prop::num::f64::ANY,
        e2 in prop::num::f64::ANY,
        px2 in prop::num::f64::ANY,
    ) {
        // Use simplified momenta to avoid combinatorial explosion of parameters.
        let p1 = FourMomentum::new(e1, px1, 0.0, 0.0);
        let p2 = FourMomentum::new(e2, px2, 0.0, 0.0);
        let p3 = FourMomentum::new(e1, -px1, 0.0, 0.0);
        let p4 = FourMomentum::new(e2, -px2, 0.0, 0.0);
        let _result = compute_mandelstam(&p1, &p2, &p3, &p4);
    }
}

// ===========================================================================
// Lorentz Boost Never Panics
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// `apply_lorentz_boost` with extreme beta values must not panic.
    /// It may return `Ok` or `Err` — the only requirement is no panic.
    #[test]
    fn apply_lorentz_boost_no_panic(
        e in prop::num::f64::ANY,
        px in prop::num::f64::ANY,
        py in prop::num::f64::ANY,
        pz in prop::num::f64::ANY,
        bx in -2.0_f64..2.0,
        by in -2.0_f64..2.0,
        bz in -2.0_f64..2.0,
        gamma in 0.0_f64..1000.0,
    ) {
        let p = FourMomentum::new(e, px, py, pz);
        let boost = LorentzBoost {
            beta: [bx, by, bz],
            gamma,
        };
        let _result = apply_lorentz_boost(&p, &boost);
    }
}

// ===========================================================================
// RAMBO Generator Never Panics
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// RAMBO with insufficient energy must return Err, not panic.
    #[test]
    fn rambo_insufficient_energy(
        seed in 0_u64..10_000,
        m1 in 1.0_f64..100.0,
        m2 in 1.0_f64..100.0,
    ) {
        let masses = vec![m1, m2];
        let mass_sum: f64 = masses.iter().sum();
        // Energy strictly below threshold.
        let cms_energy = mass_sum * 0.5;

        let mut gen = RamboGenerator::with_seed(seed);
        let result = gen.generate_event(cms_energy, &masses);
        prop_assert!(
            result.is_err(),
            "RAMBO should fail below threshold but returned Ok"
        );
    }

    /// RAMBO with 0 or 1 particles must return Err, not panic.
    #[test]
    fn rambo_too_few_particles(
        seed in 0_u64..10_000,
        cms_energy in 10.0_f64..1000.0,
    ) {
        let mut gen = RamboGenerator::with_seed(seed);

        let result_empty = gen.generate_event(cms_energy, &[]);
        prop_assert!(result_empty.is_err(), "RAMBO should reject 0 particles");

        let result_one = gen.generate_event(cms_energy, &[1.0]);
        prop_assert!(result_one.is_err(), "RAMBO should reject 1 particle");
    }

    /// RAMBO with zero masses must not panic.
    #[test]
    fn rambo_zero_masses_no_panic(
        seed in 0_u64..10_000,
        n in 2_usize..=5,
        cms_energy in 10.0_f64..1000.0,
    ) {
        let masses = vec![0.0; n];
        let mut gen = RamboGenerator::with_seed(seed);
        let _result = gen.generate_event(cms_energy, &masses);
    }
}

// ===========================================================================
// Complex Division Never Panics
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Complex division with arbitrary inputs (including zero) must not panic.
    #[test]
    fn complex_division_no_panic(
        a_re in prop::num::f64::ANY,
        a_im in prop::num::f64::ANY,
        b_re in prop::num::f64::ANY,
        b_im in prop::num::f64::ANY,
    ) {
        use spire_kernel::algebra::Complex;
        let a = Complex::new(a_re, a_im);
        let b = Complex::new(b_re, b_im);
        let _result = a / b;
    }
}

// ===========================================================================
// Trace Evaluation Never Panics
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// `evaluate_trace` with arbitrary index sequences must not panic.
    #[test]
    fn trace_evaluation_no_panic(
        indices in proptest::collection::vec(0_u8..=255, 0..20),
        include_gamma5 in proptest::bool::ANY,
    ) {
        use spire_kernel::algebra::evaluate_trace;
        let _result = evaluate_trace(&indices, include_gamma5);
    }
}
