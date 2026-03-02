//! # Property-Based Tests — Kinematic Invariants
//!
//! Verifies the fundamental physical invariants of relativistic kinematics:
//!
//! 1. **Lorentz invariance**: the invariant mass $p^2$ is preserved under boosts.
//! 2. **RAMBO conservation**: generated phase-space events conserve 4-momentum.
//! 3. **RAMBO mass shell**: massive particles satisfy $p_i^2 = m_i^2$.
//! 4. **Mandelstam sum rule**: $s + t + u = \sum m_i^2$.
//! 5. **Källén function symmetry**: $\lambda(x,y,z)$ is fully symmetric.

mod strategies;

use proptest::prelude::*;
use spire_kernel::algebra::{FourMomentum, MetricSignature, SpacetimeVector};
use spire_kernel::kinematics::{
    apply_lorentz_boost, cm_momentum, compute_mandelstam, kallen_lambda,
    PhaseSpaceGenerator, RamboGenerator,
};

use strategies::{
    arbitrary_boost, arbitrary_four_momentum, arbitrary_mass,
};

// ===========================================================================
// Lorentz Invariance of the Invariant Mass
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// For any on-shell 4-momentum $p$ and any valid Lorentz boost $\Lambda$:
    /// $$(\Lambda p)^2 = p^2$$
    #[test]
    fn lorentz_invariance_of_mass_sq(
        m in arbitrary_mass(),
        p in arbitrary_mass().prop_flat_map(arbitrary_four_momentum),
        boost in arbitrary_boost(),
    ) {
        let _ = m; // unused binding suppression

        let p_sq = p.invariant_mass_sq();

        let p_boosted = apply_lorentz_boost(&p, &boost)
            .expect("apply_lorentz_boost must not fail for valid inputs");

        let p_boosted_sq = p_boosted.invariant_mass_sq();

        // Use relative tolerance.  Scale is the larger of |p²| or 1.0.
        let scale = p_sq.abs().max(p_boosted_sq.abs()).max(1.0);
        let diff = (p_boosted_sq - p_sq).abs();

        prop_assert!(
            diff < 1e-9 * scale,
            "Lorentz invariance violated: p² = {:.12}, p'² = {:.12}, diff = {:.2e}",
            p_sq,
            p_boosted_sq,
            diff,
        );
    }

    /// Invariant mass of sum of two momenta is Lorentz invariant.
    #[test]
    fn lorentz_invariance_of_total_momentum(
        p1 in arbitrary_four_momentum(0.938),  // proton-like
        p2 in arbitrary_four_momentum(0.140),  // pion-like
        boost in arbitrary_boost(),
    ) {
        let total = p1 + p2;
        let total_sq = total.invariant_mass_sq();

        let p1_b = apply_lorentz_boost(&p1, &boost).unwrap();
        let p2_b = apply_lorentz_boost(&p2, &boost).unwrap();
        let total_b = p1_b + p2_b;
        let total_b_sq = total_b.invariant_mass_sq();

        let scale = total_sq.abs().max(total_b_sq.abs()).max(1.0);
        let diff = (total_b_sq - total_sq).abs();

        prop_assert!(
            diff < 1e-8 * scale,
            "Total invariant mass not Lorentz invariant: s = {:.12}, s' = {:.12}",
            total_sq,
            total_b_sq,
        );
    }
}

// ===========================================================================
// RAMBO Energy-Momentum Conservation
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// RAMBO-generated phase-space events must conserve 4-momentum:
    /// $$\sum_i p_i^\mu = (E_\text{cms}, 0, 0, 0)$$
    #[test]
    fn rambo_momentum_conservation_massless(
        seed in 0_u64..100_000,
        n_particles in 2_usize..=6,
        cms_energy in 10.0_f64..1000.0,
    ) {
        let masses = vec![0.0; n_particles];
        let mut gen = RamboGenerator::with_seed(seed);
        let event = gen.generate_event(cms_energy, &masses)
            .expect("RAMBO must succeed for massless particles with sufficient energy");

        prop_assert_eq!(
            event.momenta.len(), n_particles,
            "Wrong number of particles in event"
        );

        // Sum all 4-momenta.
        let mut total = SpacetimeVector::zero(4);
        for p in &event.momenta {
            for i in 0..4 {
                total.components[i] += p.components[i];
            }
        }

        // Should equal (E_cms, 0, 0, 0).
        let e_diff = (total.components[0] - cms_energy).abs();
        let px_diff = total.components[1].abs();
        let py_diff = total.components[2].abs();
        let pz_diff = total.components[3].abs();

        let tol = 1e-9;
        prop_assert!(
            e_diff < tol,
            "Energy not conserved: E_total = {:.12}, E_cms = {:.12}, diff = {:.2e}",
            total.components[0], cms_energy, e_diff,
        );
        prop_assert!(px_diff < tol, "px not conserved: {:.2e}", px_diff);
        prop_assert!(py_diff < tol, "py not conserved: {:.2e}", py_diff);
        prop_assert!(pz_diff < tol, "pz not conserved: {:.2e}", pz_diff);

        // Weight must be positive and finite.
        prop_assert!(event.weight > 0.0, "Non-positive weight: {}", event.weight);
        prop_assert!(event.weight.is_finite(), "Infinite weight");
    }

    /// RAMBO conservation with massive particles.
    #[test]
    fn rambo_momentum_conservation_massive(
        seed in 0_u64..100_000,
        m1 in 0.01_f64..5.0,
        m2 in 0.01_f64..5.0,
        m3 in 0.01_f64..5.0,
    ) {
        let masses = vec![m1, m2, m3];
        let mass_sum: f64 = masses.iter().sum();
        let cms_energy = mass_sum * 1.5; // safely above threshold

        let mut gen = RamboGenerator::with_seed(seed);
        let event = gen.generate_event(cms_energy, &masses)
            .expect("RAMBO must succeed above threshold");

        // Sum 4-momenta.
        let mut total = SpacetimeVector::zero(4);
        for p in &event.momenta {
            for i in 0..4 {
                total.components[i] += p.components[i];
            }
        }

        let tol = 1e-9;
        let e_diff = (total.components[0] - cms_energy).abs();
        prop_assert!(e_diff < tol, "Energy diff = {:.2e}", e_diff);
        for i in 1..4 {
            prop_assert!(
                total.components[i].abs() < tol,
                "Spatial component {} = {:.2e}",
                i, total.components[i],
            );
        }
    }
}

// ===========================================================================
// RAMBO Mass-Shell Condition
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Every RAMBO-generated massive particle must satisfy $p_i^2 = m_i^2$.
    #[test]
    fn rambo_mass_shell(
        seed in 0_u64..100_000,
        m1 in 0.1_f64..10.0,
        m2 in 0.1_f64..10.0,
    ) {
        let masses = vec![m1, m2];
        let mass_sum: f64 = masses.iter().sum();
        let cms_energy = mass_sum * 2.0;

        let mut gen = RamboGenerator::with_seed(seed);
        let event = gen.generate_event(cms_energy, &masses)
            .expect("RAMBO must succeed above threshold");

        let metric = MetricSignature::minkowski_4d();
        for (i, p) in event.momenta.iter().enumerate() {
            let p_sq = metric
                .inner_product(p, p)
                .expect("inner_product must succeed for 4-vectors");
            let m_sq = masses[i] * masses[i];
            let scale = m_sq.max(1.0);
            let diff = (p_sq - m_sq).abs();

            prop_assert!(
                diff < 1e-6 * scale,
                "Mass shell violated for particle {}: p² = {:.12}, m² = {:.12}, diff = {:.2e}",
                i, p_sq, m_sq, diff,
            );
        }
    }
}

// ===========================================================================
// Mandelstam Sum Rule
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// For any 2→2 scattering with on-shell momenta:
    /// $$s + t + u = m_1^2 + m_2^2 + m_3^2 + m_4^2$$
    #[test]
    fn mandelstam_sum_rule(
        m_beam in 0.001_f64..10.0,
        m_target in 0.001_f64..10.0,
        m3 in 0.001_f64..10.0,
        m4 in 0.001_f64..10.0,
        p_beam_mag in 1.0_f64..500.0,
        cos_theta in -1.0_f64..1.0,
        phi in 0.0_f64..std::f64::consts::TAU,
    ) {
        // Build on-shell momenta in the CM frame.
        let e1 = (p_beam_mag * p_beam_mag + m_beam * m_beam).sqrt();
        let p1 = FourMomentum::new(e1, 0.0, 0.0, p_beam_mag);
        let p2 = FourMomentum::new(
            (p_beam_mag * p_beam_mag + m_target * m_target).sqrt(),
            0.0,
            0.0,
            -p_beam_mag,
        );

        let s = (p1 + p2).invariant_mass_sq();
        let p_star = cm_momentum(s, m3, m4);

        // Skip if below threshold (p* = 0).
        prop_assume!(p_star > 1e-10);

        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let e3 = (p_star * p_star + m3 * m3).sqrt();
        let e4 = (p_star * p_star + m4 * m4).sqrt();

        let p3 = FourMomentum::new(
            e3,
            p_star * sin_theta * phi.cos(),
            p_star * sin_theta * phi.sin(),
            p_star * cos_theta,
        );
        let p4 = FourMomentum::new(
            e4,
            -p_star * sin_theta * phi.cos(),
            -p_star * sin_theta * phi.sin(),
            -p_star * cos_theta,
        );

        let m = compute_mandelstam(&p1, &p2, &p3, &p4);
        let masses = [m_beam, m_target, m3, m4];

        prop_assert!(
            m.verify_sum_rule(&masses),
            "Mandelstam sum rule failed: s+t+u = {:.12}, Σm² = {:.12}",
            m.s + m.t + m.u,
            masses.iter().map(|x| x * x).sum::<f64>(),
        );
    }
}

// ===========================================================================
// Källén Function Symmetry
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// The Källén function is fully symmetric:
    /// $$\lambda(x,y,z) = \lambda(y,x,z) = \lambda(z,y,x) = \ldots$$
    #[test]
    fn kallen_symmetry(
        x in -100.0_f64..100.0,
        y in -100.0_f64..100.0,
        z in -100.0_f64..100.0,
    ) {
        let base = kallen_lambda(x, y, z);

        // All 6 permutations.
        let perms = [
            kallen_lambda(x, z, y),
            kallen_lambda(y, x, z),
            kallen_lambda(y, z, x),
            kallen_lambda(z, x, y),
            kallen_lambda(z, y, x),
        ];

        for (i, &val) in perms.iter().enumerate() {
            let diff = (val - base).abs();
            let scale = base.abs().max(1.0);
            prop_assert!(
                diff < 1e-10 * scale,
                "Källén symmetry failed for permutation {}: base = {:.12}, perm = {:.12}",
                i, base, val,
            );
        }
    }

    /// Källén function at threshold vanishes:
    /// $$\lambda((m_a + m_b)^2, m_a^2, m_b^2) = 0$$
    #[test]
    fn kallen_threshold_vanishes(
        m_a in 0.001_f64..100.0,
        m_b in 0.001_f64..100.0,
    ) {
        let s_thr = (m_a + m_b) * (m_a + m_b);
        let lam = kallen_lambda(s_thr, m_a * m_a, m_b * m_b);
        let scale = s_thr * s_thr;

        prop_assert!(
            lam.abs() < 1e-8 * scale,
            "Källén at threshold = {:.2e}, scale = {:.2e}",
            lam, scale,
        );
    }
}

// ===========================================================================
// Boost Composition: Zero Boost Is Identity
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A zero boost must leave the momentum unchanged.
    #[test]
    fn zero_boost_is_identity(
        p in arbitrary_four_momentum(0.938),
    ) {
        use spire_kernel::kinematics::LorentzBoost;

        let zero_boost = LorentzBoost {
            beta: [0.0, 0.0, 0.0],
            gamma: 1.0,
        };

        let p_boosted = apply_lorentz_boost(&p, &zero_boost)
            .expect("zero boost must not fail");

        let diff_e = (p_boosted.e - p.e).abs();
        let diff_px = (p_boosted.px - p.px).abs();
        let diff_py = (p_boosted.py - p.py).abs();
        let diff_pz = (p_boosted.pz - p.pz).abs();

        prop_assert!(diff_e < 1e-15, "Energy changed under zero boost: {:.2e}", diff_e);
        prop_assert!(diff_px < 1e-15, "px changed: {:.2e}", diff_px);
        prop_assert!(diff_py < 1e-15, "py changed: {:.2e}", diff_py);
        prop_assert!(diff_pz < 1e-15, "pz changed: {:.2e}", diff_pz);
    }
}
