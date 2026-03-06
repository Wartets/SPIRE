//! # Parton Distribution Functions
//!
//! This module provides the framework for evaluating Parton Distribution
//! Functions (PDFs) - the probability densities $f_a^H(x, Q^2)$ describing
//! the momentum distribution of parton flavour $a$ inside hadron $H$ at
//! momentum fraction $x$ and factorisation scale $Q^2$.
//!
//! ## Architecture
//!
//! The [`PdfProvider`] trait defines a fully generic interface. Implementations
//! can range from simple analytical parametrisations (see [`ToyProtonPdf`]) to
//! interpolated grids backed by external data (e.g., a future LHAPDF bridge).
//!
//! ## PDG Parton Codes
//!
//! Parton flavours use the standard Monte Carlo particle numbering (PDG codes):
//!
//! | PDG ID | Parton         |
//! |--------|----------------|
//! |  1     | $d$            |
//! | -1     | $\bar{d}$      |
//! |  2     | $u$            |
//! | -2     | $\bar{u}$      |
//! |  3     | $s$            |
//! | -3     | $\bar{s}$      |
//! | 21     | gluon $g$      |
//!
//! ## Usage
//!
//! ```rust,no_run
//! # use spire_kernel::pdf::*;
//! let pdf = ToyProtonPdf::new();
//! let u_val = pdf.evaluate(2, 0.3, 100.0);  // u-quark at x=0.3, Q²=100 GeV²
//! ```

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

// ===========================================================================
// PdfProvider Trait
// ===========================================================================

/// A generic provider of parton distribution functions.
///
/// Implementations must evaluate the PDF $f_a(x, Q^2)$ for any supported
/// parton flavour. The trait is designed for extensibility - future
/// implementations may wrap interpolated grid files or external C++ libraries.
///
/// # Contract
///
/// - `x` must lie in the range $(0, 1]$. Implementations should return `0.0`
///   for $x \leq 0$ or $x > 1$.
/// - `q2` is the factorisation scale in GeV². Must be positive.
/// - For unsupported `pdg_id` values, implementations should return `0.0`.
pub trait PdfProvider: Send + Sync {
    /// Evaluate the PDF $f_a(x, Q^2)$ for parton with the given PDG ID.
    ///
    /// # Arguments
    ///
    /// * `pdg_id` - Monte Carlo particle numbering code for the parton.
    /// * `x` - Momentum fraction carried by the parton ($0 < x \leq 1$).
    /// * `q2` - Factorisation scale $Q^2$ in GeV².
    ///
    /// # Returns
    ///
    /// The probability density $f_a(x, Q^2)$.  Returns `0.0` for
    /// kinematically forbidden regions ($x \leq 0$, $x > 1$) or unsupported
    /// parton flavours.
    fn evaluate(&self, pdg_id: i32, x: f64, q2: f64) -> f64;

    /// Human-readable name of this PDF set.
    fn name(&self) -> &str;

    /// List the PDG IDs of all supported parton flavours.
    fn supported_flavours(&self) -> &[i32];
}

// ===========================================================================
// ToyProtonPdf
// ===========================================================================

/// Standard parton PDG codes.
const PDG_D: i32 = 1;
const PDG_DBAR: i32 = -1;
const PDG_U: i32 = 2;
const PDG_UBAR: i32 = -2;
const PDG_S: i32 = 3;
const PDG_SBAR: i32 = -3;
const PDG_GLUON: i32 = 21;

/// All flavour codes supported by the toy proton PDF.
const SUPPORTED_FLAVOURS: [i32; 7] = [PDG_D, PDG_DBAR, PDG_U, PDG_UBAR, PDG_S, PDG_SBAR, PDG_GLUON];

/// A simple analytical parametrisation of the proton PDF for pipeline
/// validation and testing.
///
/// This toy model provides qualitatively correct behaviour:
///
/// - **Valence quarks**: $u_v(x) \propto x^{0.5}(1-x)^3$ and
///   $d_v(x) \propto x^{0.5}(1-x)^4$, normalised to satisfy the valence
///   number sum rules $\int_0^1 u_v\,dx = 2$ and $\int_0^1 d_v\,dx = 1$.
/// - **Sea quarks**: Symmetric $\bar{u} = \bar{d} = s = \bar{s} \propto
///   (1-x)^7 / \sqrt{x}$.
/// - **Gluon**: $g(x) \propto (1-x)^5 / \sqrt{x}$.
///
/// The parametrisation is scale-independent (no $Q^2$ evolution). For
/// quantitative phenomenology, replace with a grid-based provider such as
/// LHAPDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToyProtonPdf {
    /// Normalisation constant for $u$-valence: $A_u = 2 / B(1.5, 4)$.
    a_u: f64,
    /// Normalisation constant for $d$-valence: $A_d = 1 / B(1.5, 5)$.
    a_d: f64,
    /// Amplitude of the sea-quark distribution.
    a_sea: f64,
    /// Amplitude of the gluon distribution.
    a_gluon: f64,
}

impl ToyProtonPdf {
    /// Create a new toy proton PDF with default parameters.
    ///
    /// The valence normalisations are computed from the Euler beta function:
    /// $A_u = 2/B(1.5, 4)$ and $A_d = 1/B(1.5, 5)$.
    pub fn new() -> Self {
        // B(a,b) = Γ(a)Γ(b) / Γ(a+b)
        // B(1.5, 4) = Γ(1.5)Γ(4) / Γ(5.5)
        let b_u = beta(1.5, 4.0);
        let b_d = beta(1.5, 5.0);

        Self {
            a_u: 2.0 / b_u,
            a_d: 1.0 / b_d,
            a_sea: 0.145,
            a_gluon: 1.8,
        }
    }

    /// Create a toy proton PDF with custom normalisation amplitudes.
    ///
    /// Useful for testing sensitivity to PDF parameters.
    pub fn with_params(a_u: f64, a_d: f64, a_sea: f64, a_gluon: f64) -> Self {
        Self {
            a_u,
            a_d,
            a_sea,
            a_gluon,
        }
    }

    /// Evaluate the $u$-valence distribution: $u_v(x) = A_u x^{0.5} (1-x)^3$.
    fn u_valence(&self, x: f64) -> f64 {
        self.a_u * x.powf(0.5) * (1.0 - x).powi(3)
    }

    /// Evaluate the $d$-valence distribution: $d_v(x) = A_d x^{0.5} (1-x)^4$.
    fn d_valence(&self, x: f64) -> f64 {
        self.a_d * x.powf(0.5) * (1.0 - x).powi(4)
    }

    /// Evaluate the sea-quark distribution: $q_\text{sea}(x) = A_s (1-x)^7 / \sqrt{x}$.
    fn sea(&self, x: f64) -> f64 {
        if x < 1e-15 {
            return 0.0;
        }
        self.a_sea * (1.0 - x).powi(7) / x.sqrt()
    }

    /// Evaluate the gluon distribution: $g(x) = A_g (1-x)^5 / \sqrt{x}$.
    fn gluon(&self, x: f64) -> f64 {
        if x < 1e-15 {
            return 0.0;
        }
        self.a_gluon * (1.0 - x).powi(5) / x.sqrt()
    }
}

impl Default for ToyProtonPdf {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfProvider for ToyProtonPdf {
    fn evaluate(&self, pdg_id: i32, x: f64, _q2: f64) -> f64 {
        // Boundary checks.
        if x <= 0.0 || x > 1.0 {
            return 0.0;
        }

        match pdg_id {
            // u = valence + sea
            PDG_U => self.u_valence(x) + self.sea(x),
            // ū = sea only
            PDG_UBAR => self.sea(x),
            // d = valence + sea
            PDG_D => self.d_valence(x) + self.sea(x),
            // d̄ = sea only
            PDG_DBAR => self.sea(x),
            // s = s̄ = sea
            PDG_S | PDG_SBAR => self.sea(x),
            // gluon
            PDG_GLUON => self.gluon(x),
            // Unsupported flavours return zero.
            _ => 0.0,
        }
    }

    fn name(&self) -> &str {
        "ToyProtonPdf"
    }

    fn supported_flavours(&self) -> &[i32] {
        &SUPPORTED_FLAVOURS
    }
}

// ===========================================================================
// Flat PDF (for testing)
// ===========================================================================

/// A trivially flat PDF: $f(x) = C$ for all flavours and all $x \in (0,1]$.
///
/// Useful for validating the convolution integral machinery against analytic
/// results. With $C = 1$, the convolution reduces to a simple integral of
/// the partonic cross-section over the $x_1, x_2$ domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatPdf {
    /// The constant value returned for all parton flavours.
    pub value: f64,
}

impl FlatPdf {
    /// Create a flat PDF with a given constant value.
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

impl Default for FlatPdf {
    fn default() -> Self {
        Self { value: 1.0 }
    }
}

impl PdfProvider for FlatPdf {
    fn evaluate(&self, _pdg_id: i32, x: f64, _q2: f64) -> f64 {
        if x <= 0.0 || x > 1.0 {
            return 0.0;
        }
        self.value
    }

    fn name(&self) -> &str {
        "FlatPdf"
    }

    fn supported_flavours(&self) -> &[i32] {
        &SUPPORTED_FLAVOURS
    }
}

// ===========================================================================
// Hadron Types
// ===========================================================================

/// A hadron species for use as a beam particle.
///
/// Each variant carries the parton content (list of PDG codes for the
/// constituent partons that may initiate hard scattering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hadron {
    /// Proton ($uud$): valence quarks, sea quarks, and gluons.
    Proton,
    /// Antiproton ($\bar{u}\bar{u}\bar{d}$).
    AntiProton,
    /// Pion+ ($u\bar{d}$) - simplified parton content.
    PionPlus,
    /// Pion- ($\bar{u}d$) - simplified parton content.
    PionMinus,
}

impl Hadron {
    /// Return the list of active parton PDG codes for this hadron.
    ///
    /// This determines which initial-state partons are summed over in the
    /// hadronic cross-section convolution.
    pub fn parton_content(&self) -> &[i32] {
        match self {
            Hadron::Proton => &[PDG_U, PDG_UBAR, PDG_D, PDG_DBAR, PDG_S, PDG_SBAR, PDG_GLUON],
            Hadron::AntiProton => &[PDG_U, PDG_UBAR, PDG_D, PDG_DBAR, PDG_S, PDG_SBAR, PDG_GLUON],
            Hadron::PionPlus => &[PDG_U, PDG_UBAR, PDG_D, PDG_DBAR, PDG_GLUON],
            Hadron::PionMinus => &[PDG_U, PDG_UBAR, PDG_D, PDG_DBAR, PDG_GLUON],
        }
    }

    /// Human-readable name.
    pub fn label(&self) -> &str {
        match self {
            Hadron::Proton => "p",
            Hadron::AntiProton => "p-bar",
            Hadron::PionPlus => "pi+",
            Hadron::PionMinus => "pi-",
        }
    }
}

// ===========================================================================
// Hadronic Cross-Section Result
// ===========================================================================

/// Result of a hadronic cross-section calculation, bundling the Monte Carlo
/// estimate with process metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadronicCrossSectionResult {
    /// Total hadronic cross-section in natural units (GeV$^{-2}$).
    pub cross_section: f64,
    /// Statistical uncertainty in natural units (GeV$^{-2}$).
    pub uncertainty: f64,
    /// Cross-section in picobarns.
    pub cross_section_pb: f64,
    /// Uncertainty in picobarns.
    pub uncertainty_pb: f64,
    /// Number of events evaluated.
    pub events_evaluated: usize,
    /// Relative error $\delta\sigma / \sigma$.
    pub relative_error: f64,
    /// Squared beam energy $S = (p_1 + p_2)^2$ in GeV$^2$.
    pub beam_energy_sq: f64,
    /// Name of the PDF set used.
    pub pdf_name: String,
    /// Description of the initial-state hadrons.
    pub beam_description: String,
}

// ===========================================================================
// Helper: Euler beta function
// ===========================================================================

/// Compute the Euler beta function $B(a, b) = \Gamma(a)\Gamma(b)/\Gamma(a+b)$.
///
/// Uses the Lanczos approximation for the gamma function.
fn beta(a: f64, b: f64) -> f64 {
    (gamma_ln(a) + gamma_ln(b) - gamma_ln(a + b)).exp()
}

/// Natural logarithm of the gamma function $\ln\Gamma(z)$.
///
/// Uses the Lanczos approximation with $g = 7$ and $n = 9$ coefficients.
/// Accurate to ~15 significant digits for $z > 0.5$.
fn gamma_ln(z: f64) -> f64 {
    // Lanczos coefficients for g=7, n=9
    const COEFFS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];

    if z < 0.5 {
        // Reflection formula: Γ(z)Γ(1-z) = π/sin(πz)
        let reflected = gamma_ln(1.0 - z);
        return (PI / (PI * z).sin()).ln() - reflected;
    }

    let z = z - 1.0;
    let mut x = COEFFS[0];
    for (i, &c) in COEFFS[1..].iter().enumerate() {
        x += c / (z + i as f64 + 1.0);
    }

    let t = z + 7.5; // g + 0.5
    0.5 * (2.0 * PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper: simple numerical integration via trapezoidal rule
    // -----------------------------------------------------------------------
    fn integrate_trapezoidal<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> f64 {
        let h = (b - a) / n as f64;
        let mut sum = 0.5 * (f(a) + f(b));
        for i in 1..n {
            sum += f(a + i as f64 * h);
        }
        sum * h
    }

    // -----------------------------------------------------------------------
    // Beta function / gamma function tests
    // -----------------------------------------------------------------------

    #[test]
    fn beta_function_known_values() {
        // B(1, 1) = 1
        let b11 = beta(1.0, 1.0);
        assert!((b11 - 1.0).abs() < 1e-10, "B(1,1) = {}", b11);

        // B(2, 3) = 1/12
        let b23 = beta(2.0, 3.0);
        assert!((b23 - 1.0 / 12.0).abs() < 1e-10, "B(2,3) = {}", b23);

        // B(0.5, 0.5) = π
        let b_half = beta(0.5, 0.5);
        assert!((b_half - PI).abs() < 1e-8, "B(0.5,0.5) = {}", b_half);
    }

    #[test]
    fn gamma_ln_factorial_values() {
        // Γ(1) = 1 → ln Γ(1) = 0
        assert!(gamma_ln(1.0).abs() < 1e-10);

        // Γ(5) = 24 → ln Γ(5) = ln(24)
        let expected = 24.0_f64.ln();
        assert!((gamma_ln(5.0) - expected).abs() < 1e-10);

        // Γ(0.5) = √π → ln Γ(0.5) = 0.5 ln(π)
        let expected_half = 0.5 * PI.ln();
        assert!(
            (gamma_ln(0.5) - expected_half).abs() < 1e-10,
            "ln Γ(0.5) = {} vs {}",
            gamma_ln(0.5),
            expected_half
        );
    }

    // -----------------------------------------------------------------------
    // ToyProtonPdf creation and basic evaluation
    // -----------------------------------------------------------------------

    #[test]
    fn toy_pdf_default_creation() {
        let pdf = ToyProtonPdf::new();
        assert!(pdf.a_u > 0.0);
        assert!(pdf.a_d > 0.0);
        assert_eq!(pdf.name(), "ToyProtonPdf");
    }

    #[test]
    fn toy_pdf_boundary_conditions() {
        let pdf = ToyProtonPdf::new();

        // x ≤ 0 → 0
        assert_eq!(pdf.evaluate(PDG_U, 0.0, 100.0), 0.0);
        assert_eq!(pdf.evaluate(PDG_U, -0.1, 100.0), 0.0);

        // x > 1 → 0
        assert_eq!(pdf.evaluate(PDG_U, 1.1, 100.0), 0.0);

        // Unsupported flavour → 0
        assert_eq!(pdf.evaluate(999, 0.5, 100.0), 0.0);
    }

    #[test]
    fn toy_pdf_u_quark_positive() {
        let pdf = ToyProtonPdf::new();
        for &x in &[0.01, 0.1, 0.3, 0.5, 0.9] {
            let val = pdf.evaluate(PDG_U, x, 100.0);
            assert!(val > 0.0, "u(x={}) should be positive, got {}", x, val);
        }
    }

    #[test]
    fn toy_pdf_d_quark_positive() {
        let pdf = ToyProtonPdf::new();
        for &x in &[0.01, 0.1, 0.3, 0.5, 0.9] {
            let val = pdf.evaluate(PDG_D, x, 100.0);
            assert!(val > 0.0, "d(x={}) should be positive, got {}", x, val);
        }
    }

    #[test]
    fn toy_pdf_gluon_positive() {
        let pdf = ToyProtonPdf::new();
        for &x in &[0.01, 0.1, 0.3, 0.5] {
            let val = pdf.evaluate(PDG_GLUON, x, 100.0);
            assert!(val > 0.0, "g(x={}) should be positive, got {}", x, val);
        }
    }

    // -----------------------------------------------------------------------
    // Valence number sum rules
    // -----------------------------------------------------------------------

    #[test]
    fn toy_pdf_u_valence_sum_rule() {
        // ∫₀¹ u_v(x) dx = 2  (proton has 2 valence u quarks)
        let pdf = ToyProtonPdf::new();
        let integral = integrate_trapezoidal(|x| pdf.u_valence(x), 1e-6, 1.0 - 1e-6, 100_000);
        assert!(
            (integral - 2.0).abs() < 0.01,
            "u-valence sum rule: ∫u_v dx = {} (expected 2)",
            integral
        );
    }

    #[test]
    fn toy_pdf_d_valence_sum_rule() {
        // ∫₀¹ d_v(x) dx = 1  (proton has 1 valence d quark)
        let pdf = ToyProtonPdf::new();
        let integral = integrate_trapezoidal(|x| pdf.d_valence(x), 1e-6, 1.0 - 1e-6, 100_000);
        assert!(
            (integral - 1.0).abs() < 0.01,
            "d-valence sum rule: ∫d_v dx = {} (expected 1)",
            integral
        );
    }

    // -----------------------------------------------------------------------
    // PDF symmetry properties
    // -----------------------------------------------------------------------

    #[test]
    fn toy_pdf_sea_symmetric() {
        // Sea quarks: ū = d̄ = s = s̄
        let pdf = ToyProtonPdf::new();
        let x = 0.25;
        let q2 = 100.0;
        let ubar = pdf.evaluate(PDG_UBAR, x, q2);
        let dbar = pdf.evaluate(PDG_DBAR, x, q2);
        let s = pdf.evaluate(PDG_S, x, q2);
        let sbar = pdf.evaluate(PDG_SBAR, x, q2);
        assert!((ubar - dbar).abs() < 1e-12);
        assert!((ubar - s).abs() < 1e-12);
        assert!((s - sbar).abs() < 1e-12);
    }

    #[test]
    fn toy_pdf_u_larger_than_d_at_high_x() {
        // At large x, the u-quark PDF should exceed d-quark (proton has more u valence)
        let pdf = ToyProtonPdf::new();
        let u = pdf.evaluate(PDG_U, 0.5, 100.0);
        let d = pdf.evaluate(PDG_D, 0.5, 100.0);
        assert!(u > d, "u(0.5) = {} should exceed d(0.5) = {}", u, d);
    }

    #[test]
    fn toy_pdf_scale_independent() {
        // The toy PDF has no Q² evolution - values should not change with scale.
        let pdf = ToyProtonPdf::new();
        let v1 = pdf.evaluate(PDG_U, 0.3, 10.0);
        let v2 = pdf.evaluate(PDG_U, 0.3, 10000.0);
        assert!((v1 - v2).abs() < 1e-15);
    }

    // -----------------------------------------------------------------------
    // FlatPdf
    // -----------------------------------------------------------------------

    #[test]
    fn flat_pdf_constant() {
        let pdf = FlatPdf::new(3.14);
        assert!((pdf.evaluate(PDG_U, 0.5, 100.0) - 3.14).abs() < 1e-15);
        assert!((pdf.evaluate(PDG_GLUON, 0.99, 50.0) - 3.14).abs() < 1e-15);
    }

    #[test]
    fn flat_pdf_boundaries() {
        let pdf = FlatPdf::default();
        assert_eq!(pdf.evaluate(PDG_U, 0.0, 100.0), 0.0);
        assert_eq!(pdf.evaluate(PDG_U, 1.5, 100.0), 0.0);
    }

    #[test]
    fn flat_pdf_name() {
        let pdf = FlatPdf::default();
        assert_eq!(pdf.name(), "FlatPdf");
    }

    // -----------------------------------------------------------------------
    // Hadron
    // -----------------------------------------------------------------------

    #[test]
    fn hadron_proton_parton_content() {
        let content = Hadron::Proton.parton_content();
        assert!(content.contains(&PDG_U));
        assert!(content.contains(&PDG_D));
        assert!(content.contains(&PDG_GLUON));
        assert_eq!(content.len(), 7);
    }

    #[test]
    fn hadron_labels() {
        assert_eq!(Hadron::Proton.label(), "p");
        assert_eq!(Hadron::AntiProton.label(), "p-bar");
        assert_eq!(Hadron::PionPlus.label(), "pi+");
    }

    #[test]
    fn hadron_serde_roundtrip() {
        let h = Hadron::Proton;
        let json = serde_json::to_string(&h).unwrap();
        let h2: Hadron = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn toy_pdf_serde_roundtrip() {
        let pdf = ToyProtonPdf::new();
        let json = serde_json::to_string(&pdf).unwrap();
        let pdf2: ToyProtonPdf = serde_json::from_str(&json).unwrap();
        assert!((pdf.a_u - pdf2.a_u).abs() < 1e-15);
    }

    #[test]
    fn hadronic_result_serde_roundtrip() {
        let result = HadronicCrossSectionResult {
            cross_section: 1.5e-6,
            uncertainty: 2.0e-8,
            cross_section_pb: 584.1,
            uncertainty_pb: 7.79,
            events_evaluated: 10000,
            relative_error: 0.013,
            beam_energy_sq: 1.96e7,
            pdf_name: "ToyProtonPdf".into(),
            beam_description: "pp".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let r2: HadronicCrossSectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r2.events_evaluated, 10000);
        assert!((r2.cross_section - 1.5e-6).abs() < 1e-20);
    }

    #[test]
    fn supported_flavours_contains_all() {
        let pdf = ToyProtonPdf::new();
        let flavours = pdf.supported_flavours();
        assert!(flavours.contains(&PDG_U));
        assert!(flavours.contains(&PDG_UBAR));
        assert!(flavours.contains(&PDG_D));
        assert!(flavours.contains(&PDG_DBAR));
        assert!(flavours.contains(&PDG_S));
        assert!(flavours.contains(&PDG_SBAR));
        assert!(flavours.contains(&PDG_GLUON));
    }
}
