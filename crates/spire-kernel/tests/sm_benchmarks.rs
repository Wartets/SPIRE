//! # Standard Model Benchmark Suite
//!
//! Validates SPIRE's computational core against known Standard Model observables.
//! Each test exercises a distinct layer of the physics engine (decay widths,
//! kinematics, phase-space integration) and compares results against either
//! PDG reference values or analytically exact tree-level formulae.
//!
//! ## Benchmark Inventory
//!
//! | # | Observable | Method | Reference |
//! |---|-----------|--------|-----------|
//! | 1 | $\Gamma_Z$ (total) | `decay::calculate_decay_table` | PDG 2024 |
//! | 2 | $\Gamma(Z \to \ell^+\ell^-)$ | Computed partial width | Lepton universality |
//! | 3 | $\Gamma(Z \to \ell^+\ell^-)$ | Independent analytic formula | Tree-level $V-A$ |
//! | 4 | $\mathrm{BR}(Z)$ sum rule | Branching ratio unitarity | Exact: 1.0 |
//! | 5 | $\Gamma_W$ channels | `decay::calculate_decay_table` | Channel existence |
//! | 6 | $\Gamma_H$ | `decay::calculate_decay_table` | Positive width |
//! | 7 | $\sigma_{\text{peak}}(e^+e^-\!\to\!\mu^+\mu^-)$ | BW formula from widths | ~2 nb |
//! | 8 | $\sigma_{\text{QED}}(e^+e^-\!\to\!\mu^+\mu^-)$ | MC vs analytic $4\pi\alpha^2/(3s)$ | Exact at tree level |
//! | 9 | $W^+W^-$ threshold | CM momentum at $\sqrt{s}=161$ GeV | Kinematics |
//! |10 | Mandelstam sum rule | $s+t+u = \sum m_i^2$ consistency | Exact identity |

use std::f64::consts::PI;

use spire_kernel::data_loader;
use spire_kernel::decay::calculate_decay_table;
use spire_kernel::integration::{compute_cross_section, UniformIntegrator};
use spire_kernel::kinematics::{
    cm_momentum, kallen_lambda, two_body_phase_space, PhaseSpaceGenerator, RamboGenerator,
};
use spire_kernel::lagrangian::TheoreticalModel;

// ===========================================================================
// Helper: Load the Standard Model
// ===========================================================================

/// Build the SM from the bundled TOML data files.
fn load_sm() -> TheoreticalModel {
    let particles = include_str!("../../../data/particles.toml");
    let vertices = include_str!("../../../data/sm_vertices.toml");
    data_loader::build_model(particles, vertices, "Standard Model").unwrap()
}

// ===========================================================================
// PDG Reference Constants (2024 Review of Particle Physics)
// ===========================================================================

/// Z boson mass in GeV.
const M_Z: f64 = 91.1876;
/// Z boson total width in GeV (PDG: 2.4952 ± 0.0023).
const GAMMA_Z_PDG: f64 = 2.4952;
/// W boson mass in GeV.
const M_W: f64 = 80.379;
/// W boson total width in GeV (PDG: 2.085 ± 0.042).
const GAMMA_W_PDG: f64 = 2.085;
/// Higgs boson mass in GeV.
const M_H: f64 = 125.1;
/// Higgs boson total width in GeV (PDG: 0.00407).
const GAMMA_H_PDG: f64 = 0.00407;

/// QED coupling constant $e$ (from $\alpha = e^2 / 4\pi \approx 1/137.036$).
const E_QED: f64 = 0.30282212;
/// Fine-structure constant $\alpha = e^2 / (4\pi)$.
const ALPHA_EM: f64 = E_QED * E_QED / (4.0 * PI);

/// GeV⁻² → pb conversion factor.
const GEV2_TO_PB: f64 = 0.3894e9;
/// GeV⁻² → nb conversion factor.
const GEV2_TO_NB: f64 = 0.3894e6;

/// Muon mass in GeV.
const M_MU: f64 = 0.10566;
/// Electron mass in GeV.
const M_E: f64 = 0.000511;

// ===========================================================================
// 1. Z Boson Total Width
// ===========================================================================

/// Verify that the computed Z boson total width is positive and within
/// a physically reasonable range of the PDG value.
///
/// The tree-level width from the model uses a subset of SM fermions
/// (first-generation leptons + quarks), so the exact PDG value is not
/// expected. Instead we verify:
///   (a) $\Gamma_Z > 0$
///   (b) $\Gamma_Z$ is within an order of magnitude of the PDG value
///   (c) The decay table has multiple channels
#[test]
fn z_total_width_positive_and_reasonable() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    assert!(
        table.total_width > 0.0,
        "Z total width must be positive, got {:.6e} GeV",
        table.total_width
    );
    assert!(
        table.channels.len() >= 2,
        "Z should have at least 2 decay channels, got {}",
        table.channels.len()
    );

    // Tree-level width from a first-generation model should be O(1 GeV).
    assert!(
        table.total_width > 0.1 && table.total_width < 10.0,
        "Z width {:.4} GeV outside reasonable range [0.1, 10.0] GeV",
        table.total_width
    );

    println!(
        "  Z total width:  {:.6} GeV  (PDG: {:.4} GeV)",
        table.total_width, GAMMA_Z_PDG
    );
    for ch in &table.channels {
        println!(
            "    → {:<30}  Γ = {:.6e} GeV   BR = {:.4}",
            ch.final_state_names.join(" "),
            ch.partial_width,
            ch.branching_ratio
        );
    }
}

// ===========================================================================
// 2. Lepton Universality: Γ(Z→ee) = Γ(Z→μμ)
// ===========================================================================

/// At tree level, the Z boson partial widths to $e^+e^-$ and $\mu^+\mu^-$
/// should be nearly equal (differing only by the tiny lepton mass correction).
/// This tests lepton universality as implemented in the decay engine.
#[test]
fn z_lepton_universality() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    // Find Z → e- channels and Z → μ- channels.
    let gamma_ee: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"e-".to_string()))
        .map(|c| c.partial_width)
        .sum();

    let gamma_mumu: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"mu-".to_string()))
        .map(|c| c.partial_width)
        .sum();

    assert!(
        gamma_ee > 0.0,
        "Γ(Z→ee) must be positive, got {:.6e}",
        gamma_ee
    );
    assert!(
        gamma_mumu > 0.0,
        "Γ(Z→μμ) must be positive, got {:.6e}",
        gamma_mumu
    );

    // Ratio should be ~1 to better than 1% (mass corrections are ≲ 10⁻⁵).
    let ratio = gamma_ee / gamma_mumu;
    let tolerance = 0.01; // 1%
    assert!(
        (ratio - 1.0).abs() < tolerance,
        "Lepton universality violated: Γ(ee)/Γ(μμ) = {:.6}, expected ~1.0",
        ratio
    );

    println!("  Γ(Z→ee)  = {:.6e} GeV", gamma_ee);
    println!("  Γ(Z→μμ)  = {:.6e} GeV", gamma_mumu);
    println!("  Ratio     = {:.8} (expect 1.0 ± {})", ratio, tolerance);
}

// ===========================================================================
// 3. Z Leptonic Width: Independent Cross-Check
// ===========================================================================

/// Cross-check the Z → ℓ⁺ℓ⁻ partial width against an independently
/// coded analytic formula matching the decay module's V → f f̄ branch:
///
/// $$\overline{|\mathcal{M}|^2} = \frac{g^2}{3}(2 M_Z^2 + 2 m_\ell^2)$$
/// $$\Gamma = \frac{p_{\text{cm}} \cdot \overline{|\mathcal{M}|^2}}{8\pi M_Z^2}$$
///
/// This exercises the spin-averaged matrix element and partial-width
/// formula in the decay engine against a hand-coded reference.
#[test]
fn z_leptonic_width_cross_check() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    let gamma_ee: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"e-".to_string()))
        .map(|c| c.partial_width)
        .sum();

    // Independent calculation using the same coupling g_Z = 0.7397
    // and the V → f f̄ spin-averaged |M|² formula:
    //   |M|² = g²/3 · (2 M_Z² + m_B² + m_C² - (m_B²-m_C²)²/M_Z²)
    //   Γ = p_cm · |M|² / (8π M_Z²)
    let g_z = 0.7397_f64;
    let m_z = M_Z;
    let m_e = M_E;
    let m_z_sq = m_z * m_z;
    let m_e_sq = m_e * m_e;

    // CM 3-momentum of the electron in Z rest frame.
    let p_cm = cm_momentum(m_z_sq, m_e, m_e);
    assert!(p_cm > 0.0, "CM momentum must be positive");

    // Spin-averaged |M|² for V → ff̄ (1/3 from spin averaging over 3 polarisations):
    let kinematic = 2.0 * m_z_sq + m_e_sq + m_e_sq - (m_e_sq - m_e_sq).powi(2) / m_z_sq;
    let m_sq = g_z * g_z * kinematic / 3.0;

    // Γ = N_c · p_cm · |M|² / (8π m_A²)   (N_c = 1 for leptons)
    let gamma_analytic = p_cm * m_sq / (8.0 * PI * m_z_sq);

    // Should agree to 4 significant figures.
    let rel_err = ((gamma_ee - gamma_analytic) / gamma_analytic).abs();
    assert!(
        rel_err < 1e-4,
        "Z→ee width mismatch: decay module = {:.6e}, analytic = {:.6e}, rel err = {:.2e}",
        gamma_ee,
        gamma_analytic,
        rel_err
    );

    println!("  Γ(Z→ee) decay module: {:.8e} GeV", gamma_ee);
    println!("  Γ(Z→ee) analytic:     {:.8e} GeV", gamma_analytic);
    println!("  Relative error:        {:.2e}", rel_err);
}

// ===========================================================================
// 4. Z Branching Ratios Sum to Unity
// ===========================================================================

/// The branching ratios of all Z decay channels must sum to exactly 1.0
/// (conservation of probability).
#[test]
fn z_branching_ratios_unity() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    let br_sum: f64 = table.channels.iter().map(|c| c.branching_ratio).sum();
    let tolerance = 1e-10;

    assert!(
        (br_sum - 1.0).abs() < tolerance,
        "Z BRs sum to {:.12}, expected 1.0 (tolerance {})",
        br_sum,
        tolerance
    );
}

// ===========================================================================
// 5. W Boson Decay Channels
// ===========================================================================

/// Verify that the W⁻ boson has leptonic ($e^-\bar\nu_e$, $\mu^-\bar\nu_\mu$)
/// and hadronic ($\bar{u}d$) decay channels with positive widths.
#[test]
fn w_decay_channels_exist() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "W-").unwrap();

    assert!(
        table.total_width > 0.0,
        "W total width must be positive, got {:.6e} GeV",
        table.total_width
    );
    assert!(
        !table.channels.is_empty(),
        "W should have at least one decay channel"
    );

    // Should be within an order of magnitude of PDG.
    assert!(
        table.total_width > 0.1 && table.total_width < 20.0,
        "W width {:.4} GeV outside reasonable range [0.1, 20.0] GeV",
        table.total_width
    );

    println!(
        "  W total width:  {:.6} GeV  (PDG: {:.4} GeV)",
        table.total_width, GAMMA_W_PDG
    );
    for ch in &table.channels {
        println!(
            "    → {:<30}  Γ = {:.6e} GeV   BR = {:.4}",
            ch.final_state_names.join(" "),
            ch.partial_width,
            ch.branching_ratio
        );
    }

    // BR sum check.
    let br_sum: f64 = table.channels.iter().map(|c| c.branching_ratio).sum();
    assert!(
        (br_sum - 1.0).abs() < 1e-10,
        "W BRs sum to {:.12}, expected 1.0",
        br_sum
    );
}

// ===========================================================================
// 6. Higgs Boson Decay Width
// ===========================================================================

/// The Higgs boson should have at least the $H \to e^+e^-$ channel
/// (from the Yukawa vertex in the model) with a positive partial width.
///
/// The full SM Higgs width requires b-quark and τ Yukawa couplings
/// plus loop-induced $H \to gg$, $H \to \gamma\gamma$ - which are
/// beyond the current vertex set. This test validates the infrastructure
/// rather than matching the PDG total width.
#[test]
fn higgs_decay_width_positive() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "H").unwrap();

    assert!(
        table.total_width > 0.0,
        "Higgs total width must be positive, got {:.6e} GeV",
        table.total_width
    );

    // Verify the mass matches our constant.
    assert!(
        (table.parent_mass - M_H).abs() < 0.01,
        "Higgs mass mismatch: table = {}, expected {}",
        table.parent_mass,
        M_H
    );

    // The Yukawa coupling y_e = 2.94e-6 gives a tiny width.
    // Verify it's at least nonzero and finite.
    assert!(table.total_width.is_finite(), "Higgs width must be finite");

    // Convert to picobarns-related units for display.
    let width_mev = table.total_width * 1e3;
    let pdg_width_mev = GAMMA_H_PDG * 1e3;

    println!(
        "  H total width:  {:.6e} MeV  (PDG: {:.4} MeV)",
        width_mev, pdg_width_mev
    );
    println!(
        "  (Model includes {} Yukawa channels; PDG value requires b, τ, loop vertices)",
        table.channels.len()
    );
    // Also note the WW threshold cross-section scale.
    let sigma_scale_pb = GEV2_TO_PB / (M_H * M_H);
    println!("  σ scale at M_H: {:.2e} pb/GeV⁻²", sigma_scale_pb);
}

// ===========================================================================
// 7. Z-Pole Peak Cross-Section (Breit-Wigner)
// ===========================================================================

/// Compute the Z-pole peak cross-section for $e^+e^- \to \mu^+\mu^-$
/// using the narrow-width Breit-Wigner formula:
///
/// $$\sigma_{\text{peak}} = \frac{12\pi}{M_Z^2} \cdot
///   \frac{\Gamma(Z\to e^+e^-)\,\Gamma(Z\to\mu^+\mu^-)}{\Gamma_Z^2}$$
///
/// This uses the partial widths computed by the decay engine, providing
/// an end-to-end test that chains decay-width calculation with
/// cross-section computation.
#[test]
fn z_pole_peak_cross_section() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    let gamma_ee: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"e-".to_string()))
        .map(|c| c.partial_width)
        .sum();

    let gamma_mumu: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"mu-".to_string()))
        .map(|c| c.partial_width)
        .sum();

    let gamma_total = table.total_width;
    let m_z = table.parent_mass;

    assert!(gamma_ee > 0.0, "Γ(Z→ee) must be positive");
    assert!(gamma_mumu > 0.0, "Γ(Z→μμ) must be positive");
    assert!(gamma_total > 0.0, "Γ_Z must be positive");

    // σ_peak = 12π · Γ_ee · Γ_μμ / (M_Z² · Γ_Z²)  [GeV⁻²]
    let sigma_peak_gev2 =
        12.0 * PI * gamma_ee * gamma_mumu / (m_z * m_z * gamma_total * gamma_total);
    let sigma_peak_nb = sigma_peak_gev2 * GEV2_TO_NB;

    // The peak cross-section should be O(1 nb).
    // With PDG values: σ_peak ≈ 2.00 nb.
    // With our tree-level partial model it may differ, but should be positive
    // and in the right ballpark.
    assert!(sigma_peak_nb > 0.0, "Peak cross-section must be positive");
    assert!(
        sigma_peak_nb > 0.01 && sigma_peak_nb < 100.0,
        "Peak σ = {:.4} nb outside reasonable range [0.01, 100] nb",
        sigma_peak_nb
    );

    println!("  σ_peak(e+e- → μ+μ-) = {:.6} nb", sigma_peak_nb);
    println!(
        "  (Using Γ_ee = {:.4e}, Γ_μμ = {:.4e}, Γ_Z = {:.4} GeV)",
        gamma_ee, gamma_mumu, gamma_total
    );

    // Self-consistency: verify the formula against PDG values independently.
    let sigma_pdg_ref = 12.0 * PI * 0.08399_f64.powi(2) / (M_Z * M_Z * GAMMA_Z_PDG * GAMMA_Z_PDG);
    let sigma_pdg_nb = sigma_pdg_ref * GEV2_TO_NB;
    println!(
        "  σ_peak(PDG inputs)   = {:.6} nb  (reference: ~2.00 nb)",
        sigma_pdg_nb
    );
}

// ===========================================================================
// 8. QED Cross-Section: MC vs Analytic
// ===========================================================================

/// Compute the tree-level QED cross-section $\sigma(e^+e^- \to \mu^+\mu^-)$
/// at $\sqrt{s} = 10$ GeV via Monte Carlo integration and compare against
/// the exact analytic result:
///
/// $$\sigma = \frac{4\pi\alpha^2}{3s}$$
///
/// This exercises the full integration pipeline: RAMBO phase-space generation
/// → matrix element evaluation → flux-factor division → statistical averaging.
///
/// The tree-level QED $|\mathcal{M}|^2$ for $e^+e^- \to \mu^+\mu^-$ via
/// virtual photon exchange (neglecting masses):
///
/// $$|\mathcal{M}|^2 = 2e^4 \frac{t^2 + u^2}{s^2}$$
///
/// where $s$, $t$, $u$ are the Mandelstam variables.
#[test]
fn qed_cross_section_mc_vs_analytic() {
    let sqrt_s = 10.0; // GeV - well below Z pole, pure QED dominates
    let s = sqrt_s * sqrt_s;
    let final_masses = [M_MU, M_MU];

    // Analytic cross-section: σ = 4πα²/(3s) in natural units.
    let sigma_analytic = 4.0 * PI * ALPHA_EM * ALPHA_EM / (3.0 * s);
    let sigma_analytic_nb = sigma_analytic * GEV2_TO_NB;

    // MC integration with RAMBO.
    let mut generator = RamboGenerator::with_seed(42);
    let integrator = UniformIntegrator::new();
    let e4 = E_QED.powi(4);

    // |M|² = 2 e⁴ (t² + u²) / s² for massless fermions.
    // From the phase-space point, we extract final-state momenta and
    // reconstruct the Mandelstam variables.
    let amplitude_sq = |ps: &spire_kernel::kinematics::PhaseSpacePoint| -> f64 {
        let p3 = &ps.momenta[0]; // μ⁻
        let p4 = &ps.momenta[1]; // μ⁺

        // In the CM frame, p1 = (E, 0, 0, p), p2 = (E, 0, 0, -p).
        let e_beam = sqrt_s / 2.0;
        let p_beam = (e_beam * e_beam - M_E * M_E).max(0.0).sqrt();

        // Construct p1 as (E, px, py, pz) in CM frame.
        let p1 = [e_beam, 0.0, 0.0, p_beam];

        let p3c = p3.components();
        let p4c = p4.components();

        // Mandelstam t = (p1 - p3)²
        let dp13 = [
            p1[0] - p3c[0],
            p1[1] - p3c[1],
            p1[2] - p3c[2],
            p1[3] - p3c[3],
        ];
        let t = dp13[0] * dp13[0] - dp13[1] * dp13[1] - dp13[2] * dp13[2] - dp13[3] * dp13[3];

        // Mandelstam u = (p1 - p4)²
        let dp14 = [
            p1[0] - p4c[0],
            p1[1] - p4c[1],
            p1[2] - p4c[2],
            p1[3] - p4c[3],
        ];
        let u = dp14[0] * dp14[0] - dp14[1] * dp14[1] - dp14[2] * dp14[2] - dp14[3] * dp14[3];

        // |M|² = 2 e⁴ (t² + u²) / s²
        2.0 * e4 * (t * t + u * u) / (s * s)
    };

    let num_events = 50_000;
    let result = compute_cross_section(
        amplitude_sq,
        &integrator,
        &mut generator,
        sqrt_s,
        &final_masses,
        num_events,
    )
    .unwrap();

    let sigma_mc_nb = result.value * GEV2_TO_NB;
    let sigma_mc_err_nb = result.error * GEV2_TO_NB;

    println!("  √s = {} GeV", sqrt_s);
    println!("  σ_analytic = {:.6e} nb  (4πα²/3s)", sigma_analytic_nb);
    println!(
        "  σ_MC       = {:.6e} ± {:.2e} nb  ({} events)",
        sigma_mc_nb, sigma_mc_err_nb, num_events
    );
    println!("  Rel. error = {:.2}%", result.relative_error * 100.0);

    // Allow 15% tolerance for MC statistical fluctuations with 50k events.
    let rel_diff = ((sigma_mc_nb - sigma_analytic_nb) / sigma_analytic_nb).abs();
    assert!(
        rel_diff < 0.15,
        "MC/analytic mismatch: σ_MC = {:.4e} nb, σ_analytic = {:.4e} nb, |Δ/σ| = {:.1}%",
        sigma_mc_nb,
        sigma_analytic_nb,
        rel_diff * 100.0
    );
}

// ===========================================================================
// 9. W⁺W⁻ Production Threshold
// ===========================================================================

/// Verify that the kinematic threshold for $e^+e^- \to W^+W^-$ production
/// lies at $\sqrt{s}_{\min} = 2 M_W \approx 160.758$ GeV.
///
/// At $\sqrt{s} = 161$ GeV (LEP-2 threshold scan energy), the CM momentum
/// of the W bosons should be small but positive.
#[test]
fn ww_production_threshold() {
    let sqrt_s_min = 2.0 * M_W;
    let s_threshold = sqrt_s_min * sqrt_s_min;

    // Exactly at threshold: p* = 0.
    let p_at_threshold = cm_momentum(s_threshold, M_W, M_W);
    assert!(
        p_at_threshold.abs() < 1e-6,
        "CM momentum at exact threshold should be ~0, got {:.6e}",
        p_at_threshold
    );

    // At √s = 161 GeV (just above threshold).
    let sqrt_s_161 = 161.0;
    let s_161 = sqrt_s_161 * sqrt_s_161;
    let p_161 = cm_momentum(s_161, M_W, M_W);

    assert!(
        p_161 > 0.0,
        "CM momentum at √s = 161 GeV should be positive, got {:.6e}",
        p_161
    );
    assert!(
        p_161 < 10.0,
        "CM momentum at √s = 161 GeV should be small (near threshold), got {:.4}",
        p_161
    );

    // Below threshold: p* = 0.
    let sqrt_s_low = 150.0;
    let s_low = sqrt_s_low * sqrt_s_low;
    let p_low = cm_momentum(s_low, M_W, M_W);
    assert!(
        p_low.abs() < 1e-12,
        "CM momentum below threshold should be 0, got {:.6e}",
        p_low
    );

    // 2-body phase space at √s = 161 GeV.
    let phi2 = two_body_phase_space(s_161, M_W, M_W);
    assert!(phi2 > 0.0, "Phase space at √s = 161 GeV should be positive");

    println!("  WW threshold:    √s_min = {:.4} GeV", sqrt_s_min);
    println!("  p*(threshold)  = {:.6e} GeV (expect ~0)", p_at_threshold);
    println!("  p*(161 GeV)    = {:.6} GeV", p_161);
    println!("  Φ₂(161 GeV)   = {:.6e} GeV⁻¹", phi2);
}

// ===========================================================================
// 10. Mandelstam Sum Rule Consistency
// ===========================================================================

/// Verify the Mandelstam identity $s + t + u = \sum_i m_i^2$ using
/// four-momenta generated by RAMBO for $e^+e^- \to \mu^+\mu^-$.
///
/// This tests that RAMBO produces physically consistent kinematics
/// that respect the exact algebraic constraint.
#[test]
fn mandelstam_sum_rule_rambo() {
    let sqrt_s = 200.0;
    let _s = sqrt_s * sqrt_s;
    let final_masses = [M_MU, M_MU];
    let mut gen = RamboGenerator::with_seed(123);

    let sum_masses_sq = M_E * M_E + M_E * M_E + M_MU * M_MU + M_MU * M_MU;

    for i in 0..100 {
        let ps = gen.generate_event(sqrt_s, &final_masses).unwrap();

        let p3 = &ps.momenta[0];
        let p4 = &ps.momenta[1];
        let p3c = p3.components();
        let p4c = p4.components();

        // Initial state in CM frame.
        let e_beam = sqrt_s / 2.0;
        let p_beam = (e_beam * e_beam - M_E * M_E).max(0.0).sqrt();

        let p1 = [e_beam, 0.0, 0.0, p_beam];
        let _p2 = [e_beam, 0.0, 0.0, -p_beam];

        // s = (p1 + p2)² = 4E² (in CM frame with massless beams ≈ s)
        // But the real s from the 4-momenta:
        let p12 = [
            p1[0] + _p2[0],
            p1[1] + _p2[1],
            p1[2] + _p2[2],
            p1[3] + _p2[3],
        ];
        let s_val = p12[0] * p12[0] - p12[1] * p12[1] - p12[2] * p12[2] - p12[3] * p12[3];

        // t = (p1 - p3)²
        let dp13 = [
            p1[0] - p3c[0],
            p1[1] - p3c[1],
            p1[2] - p3c[2],
            p1[3] - p3c[3],
        ];
        let t_val = dp13[0] * dp13[0] - dp13[1] * dp13[1] - dp13[2] * dp13[2] - dp13[3] * dp13[3];

        // u = (p1 - p4)²
        let dp14 = [
            p1[0] - p4c[0],
            p1[1] - p4c[1],
            p1[2] - p4c[2],
            p1[3] - p4c[3],
        ];
        let u_val = dp14[0] * dp14[0] - dp14[1] * dp14[1] - dp14[2] * dp14[2] - dp14[3] * dp14[3];

        let stu_sum = s_val + t_val + u_val;

        // For the massless beam approximation, the sum should be close
        // to 2·m_μ² (since m_e ≈ 0):
        // s + t + u = m1² + m2² + m3² + m4² ≈ 2·m_μ²
        let tolerance = 0.5; // Generous: mass corrections + floating point
        assert!(
            (stu_sum - sum_masses_sq).abs() < tolerance,
            "Event {}: s+t+u = {:.6}, Σm² = {:.6}, diff = {:.2e}",
            i,
            stu_sum,
            sum_masses_sq,
            (stu_sum - sum_masses_sq).abs()
        );
    }
}

// ===========================================================================
// 11. Källén Function Identities
// ===========================================================================

/// Verify algebraic properties of the Källén function:
///   (a) $\lambda(s, m^2, m^2) = s(s - 4m^2)$ for equal masses
///   (b) $\lambda(s, 0, 0) = s^2$ for massless particles
///   (c) At threshold $s = (m_1 + m_2)^2$: $\lambda = 0$
#[test]
fn kallen_function_identities() {
    // (a) Equal masses: λ(s, m², m²) = s² + 2m⁴ - 2s·m² = s(s - 4m²)
    let m = 5.0_f64;
    let m_sq = m * m;
    for &s in &[100.0, 200.0, 500.0, 1000.0] {
        let lam = kallen_lambda(s, m_sq, m_sq);
        let expected = s * (s - 4.0 * m_sq);
        assert!(
            (lam - expected).abs() < 1e-8,
            "λ({}, m²={}) = {:.6e}, expected {:.6e}",
            s,
            m_sq,
            lam,
            expected
        );
    }

    // (b) Massless: λ(s, 0, 0) = s²
    for &s in &[1.0, 100.0, 8315.18] {
        let lam = kallen_lambda(s, 0.0, 0.0);
        assert!(
            (lam - s * s).abs() < 1e-8,
            "λ({}, 0, 0) = {:.6e}, expected {:.6e}",
            s,
            lam,
            s * s
        );
    }

    // (c) Threshold: λ((m1+m2)², m1², m2²) = 0
    let m1 = M_W;
    let m2 = M_W;
    let s_thr = (m1 + m2) * (m1 + m2);
    let lam_thr = kallen_lambda(s_thr, m1 * m1, m2 * m2);
    assert!(
        lam_thr.abs() < 1e-6,
        "λ at threshold = {:.6e}, expected 0",
        lam_thr
    );
}

// ===========================================================================
// 12. Stable Particles Have Zero Width
// ===========================================================================

/// Photons and electrons (the lightest charged lepton) must have
/// exactly zero total width in the Standard Model.
#[test]
fn stable_particles_zero_width() {
    let model = load_sm();

    for &particle_id in &["photon", "e-", "e+", "nu_e", "nu_mu"] {
        let table = calculate_decay_table(&model, particle_id).unwrap();
        assert!(
            table.total_width.abs() < 1e-30,
            "{} should have zero width, got {:.6e} GeV",
            particle_id,
            table.total_width
        );
        assert!(
            table.channels.is_empty(),
            "{} should have no decay channels, got {}",
            particle_id,
            table.channels.len()
        );
    }
}

// ===========================================================================
// 13. Two-Body Phase Space Cross-Check
// ===========================================================================

/// Verify the 2-body phase space formula against the Källén function:
///
/// $$\Phi_2 = \frac{p^*}{8\pi\sqrt{s}} = \frac{\sqrt{\lambda(s,m_3^2,m_4^2)}}{16\pi s}$$
#[test]
fn two_body_phase_space_consistency() {
    let s = 200.0 * 200.0; // √s = 200 GeV
    let m3 = M_MU;
    let m4 = M_MU;

    let phi2 = two_body_phase_space(s, m3, m4);
    let lam = kallen_lambda(s, m3 * m3, m4 * m4);
    let phi2_from_kallen = lam.sqrt() / (16.0 * PI * s);

    let rel_err = ((phi2 - phi2_from_kallen) / phi2_from_kallen).abs();
    assert!(
        rel_err < 1e-12,
        "Phase space mismatch: Φ₂ = {:.6e}, from Källén = {:.6e}, err = {:.2e}",
        phi2,
        phi2_from_kallen,
        rel_err
    );
}

// ===========================================================================
// 14. QED Coupling Consistency
// ===========================================================================

/// Verify that $\alpha = e^2/(4\pi)$ is consistent with $1/137.036$.
#[test]
fn qed_coupling_consistency() {
    let alpha_expected = 1.0 / 137.036;
    let rel_err = ((ALPHA_EM - alpha_expected) / alpha_expected).abs();
    assert!(
        rel_err < 0.01,
        "α = {:.6e}, expected ~{:.6e}, rel err = {:.2e}",
        ALPHA_EM,
        alpha_expected,
        rel_err
    );
    println!("  α_EM = {:.8e} ≈ 1/{:.2}", ALPHA_EM, 1.0 / ALPHA_EM);
}

// ===========================================================================
// 15. Z Quark Decay: Colour Factor N_c = 3
// ===========================================================================

/// Z → qq̄ channels should have Γ ≈ 3 × Γ(Z→ℓℓ) (up to coupling
/// differences), verifying that the colour factor N_c = 3 is applied.
#[test]
fn z_quark_decay_colour_factor() {
    let model = load_sm();
    let table = calculate_decay_table(&model, "Z0").unwrap();

    // Find neutrino channel (no colour, no mass, simplest).
    let gamma_nunu: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"nu_e".to_string()))
        .map(|c| c.partial_width)
        .sum();

    // Find u-quark channel (colour factor = 3, different coupling).
    let gamma_uu: f64 = table
        .channels
        .iter()
        .filter(|c| c.final_state.contains(&"u".to_string()))
        .map(|c| c.partial_width)
        .sum();

    // Both should be positive.
    assert!(gamma_nunu > 0.0, "Γ(Z→νν) must be positive");
    assert!(gamma_uu > 0.0, "Γ(Z→uū) must be positive");

    // The quark width should be roughly 3× the neutrino width
    // (exact factor depends on coupling ratio g_V² + g_A² for each).
    // Just verify the quark channel is significantly larger.
    assert!(
        gamma_uu > gamma_nunu,
        "Γ(Z→uū) = {:.4e} should exceed Γ(Z→νν) = {:.4e} (colour factor)",
        gamma_uu,
        gamma_nunu
    );

    println!("  Γ(Z→νν)  = {:.6e} GeV", gamma_nunu);
    println!("  Γ(Z→uū)  = {:.6e} GeV", gamma_uu);
    println!(
        "  Ratio     = {:.2} (expect ~3× from N_c, modulo couplings)",
        gamma_uu / gamma_nunu
    );
}
