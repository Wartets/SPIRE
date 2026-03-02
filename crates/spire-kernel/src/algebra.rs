//! # Algebra — Symbolic Mathematics Engine
//!
//! This module implements the symbolic mathematical structures required for
//! constructing and manipulating scattering amplitudes in QFT:
//!
//! - **FourMomentum**: Relativistic 4-vectors $p^\mu = (E, \vec{p})$ with
//!   Minkowski inner product in the $(+,-,-,-)$ metric.
//! - **DiracSpinor**: Solutions of the Dirac equation for external fermion states.
//! - **GammaMatrix**: The Dirac gamma matrices $\gamma^\mu$ and $\gamma^5$.
//! - **PolarizationVector**: Polarization states $\epsilon^\mu$ for vector bosons.
//! - **SymbolicTerm / AmplitudeExpression**: An Abstract Syntax Tree (AST)
//!   representing the multiplicative factors of the invariant amplitude
//!   $\mathcal{M}$.
//!
//! # Metric Convention
//!
//! **West Coast (particle physics) metric**: $g^{\mu\nu} = \mathrm{diag}(+1, -1, -1, -1)$.
//!
//! The Minkowski inner product is therefore:
//! $$p \cdot q = p^0 q^0 - p^1 q^1 - p^2 q^2 - p^3 q^3$$
//!
//! and the invariant mass squared is:
//! $$p^2 = E^2 - |\vec{p}|^2 = m^2$$
//!
//! # Amplitude Construction
//!
//! The [`generate_amplitude`] function maps a [`FeynmanGraph`]
//! to an [`AmplitudeExpression`] by applying the Feynman rules:
//!
//! 1. **External fermion legs** → `SymbolicTerm::Spinor` ($u$, $\bar{u}$, $v$, $\bar{v}$).
//! 2. **External boson legs** → `SymbolicTerm::PolarizationVec`.
//! 3. **Internal propagator lines** → `SymbolicTerm::PropagatorTerm`.
//! 4. **Interaction vertices** → `SymbolicTerm::VertexCoupling`.
//!
//! The result is an ordered list of symbolic terms whose product gives $i\mathcal{M}$.

use nalgebra::Matrix4;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Neg, Sub};

use crate::graph::{FeynmanGraph, NodeKind, OneLoopTopologyKind};
use crate::lagrangian::PropagatorForm;
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Spacetime Dimension — Dimensional Regularization
// ---------------------------------------------------------------------------

/// The spacetime dimension used for loop calculations.
///
/// **Modularity note**: This enum is the single point where the regularization
/// scheme's dimensional parameter is defined. All algebra functions accept this
/// as a parameter.
///
/// - Tree-level calculations use `Fixed(4)`.
/// - 1-loop dimensional regularization uses `DimReg { base: 4 }`, representing $d = 4 - 2\epsilon$.
/// - Future schemes (e.g., 6D for $\mathcal{N}=4$ SYM) simply set `base = 6`.
/// - Pauli-Villars or lattice regularization would add new enum variants without
///   changing any existing code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpacetimeDimension {
    /// Fixed integer spacetime dimension (e.g., $d = 4$ for tree-level).
    Fixed(u32),
    /// Dimensional regularization: $d = \text{base} - 2\epsilon$.
    /// The `base` is typically 4 for QCD/QED calculations.
    DimReg {
        /// The integer base dimension (typically 4).
        base: u32,
    },
}

impl SpacetimeDimension {
    /// Create the standard 4-dimensional spacetime (tree-level).
    pub fn four() -> Self {
        SpacetimeDimension::Fixed(4)
    }

    /// Create the standard dimensional regularization $d = 4 - 2\epsilon$.
    pub fn dim_reg_4() -> Self {
        SpacetimeDimension::DimReg { base: 4 }
    }

    /// Return a symbolic string representation of the dimension.
    ///
    /// - `Fixed(4)` → `"4"`
    /// - `DimReg { base: 4 }` → `"4 - 2ε"`
    pub fn to_symbolic(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(d) => format!("{}", d),
            SpacetimeDimension::DimReg { base } => format!("{} - 2ε", base),
        }
    }

    /// Return the metric trace $g_\mu^\mu = d$.
    pub fn metric_trace(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(d) => format!("{}", d),
            SpacetimeDimension::DimReg { base } => format!("{} - 2ε", base),
        }
    }

    /// Return the gamma contraction $\gamma^\mu \gamma_\mu = d \cdot I$.
    pub fn gamma_contraction(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(d) => format!("{}·I", d),
            SpacetimeDimension::DimReg { base } => format!("({} - 2ε)·I", base),
        }
    }

    /// Return the single-gamma contraction $\gamma^\mu \gamma^\nu \gamma_\mu = (2-d)\gamma^\nu$.
    pub fn single_gamma_contraction(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(d) => format!("{}·γ^ν", 2i32 - *d as i32),
            SpacetimeDimension::DimReg { base } => {
                format!("({} + 2ε)·γ^ν", 2i32 - *base as i32)
            }
        }
    }

    /// Return the double-gamma contraction
    /// $\gamma^\mu \gamma^\nu \gamma^\rho \gamma_\mu = 4g^{\nu\rho} - (4-d)\gamma^\nu\gamma^\rho$.
    pub fn double_gamma_contraction(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(4) => {
                "4·g^{νρ}".into()
            }
            SpacetimeDimension::Fixed(d) => {
                format!("4·g^{{νρ}} - {}·γ^ν γ^ρ", 4i32 - *d as i32)
            }
            SpacetimeDimension::DimReg { base } => {
                let coeff = 4i32 - *base as i32;
                if coeff == 0 {
                    "4·g^{νρ} - (-2ε)·γ^ν γ^ρ".into()
                } else {
                    format!("4·g^{{νρ}} - ({} + 2ε)·γ^ν γ^ρ", coeff)
                }
            }
        }
    }

    /// Render as a LaTeX string.
    pub fn to_latex(&self) -> String {
        match self {
            SpacetimeDimension::Fixed(d) => format!("{}", d),
            SpacetimeDimension::DimReg { base } => format!("{} - 2\\epsilon", base),
        }
    }
}

// ---------------------------------------------------------------------------
// Scalar Loop Integral Classification (Passarino-Veltman Basis)
// ---------------------------------------------------------------------------

/// Classification of standard scalar 1-loop integrals in the Passarino-Veltman
/// decomposition scheme.
///
/// Every tensor loop integral can be reduced to a linear combination of these
/// scalar master integrals (plus rational coefficients). This enum is the
/// mathematical label — the actual integral expressions are symbolic and
/// never evaluated numerically within this module.
///
/// **Extensibility**: Higher-point functions ($E_0$, $F_0$, ...) for multi-loop
/// or higher-multiplicity processes can be added as new variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarIntegralType {
    /// Tadpole integral (1-point): $A_0(m^2) = \int \frac{d^d l}{(2\pi)^d} \frac{1}{l^2 - m^2}$
    A0 {
        /// Mass squared of the propagator.
        mass_sq: f64,
    },
    /// Self-energy integral (2-point): $B_0(p^2; m_1^2, m_2^2)$
    B0 {
        /// External momentum squared flowing through the self-energy.
        p_sq: String,
        /// Mass squared of propagator 1.
        m1_sq: f64,
        /// Mass squared of propagator 2.
        m2_sq: f64,
    },
    /// Triangle integral (3-point): $C_0(p_1^2, p_2^2, (p_1+p_2)^2; m_1^2, m_2^2, m_3^2)$
    C0 {
        /// External momentum squared labels.
        external_momenta_sq: Vec<String>,
        /// Mass squared values for the 3 propagators.
        masses_sq: Vec<f64>,
    },
    /// Box integral (4-point): $D_0(p_1^2, p_2^2, p_3^2, p_4^2, s, t; m_1^2, m_2^2, m_3^2, m_4^2)$
    D0 {
        /// External momentum squared labels.
        external_momenta_sq: Vec<String>,
        /// Mandelstam invariant labels (e.g., `["s", "t"]`).
        mandelstam_invariants: Vec<String>,
        /// Mass squared values for the 4 propagators.
        masses_sq: Vec<f64>,
    },
    /// Generic $n$-point scalar integral (future extensibility).
    NPoint {
        /// Number of propagators.
        n: u32,
        /// External momentum labels.
        external_momenta: Vec<String>,
        /// Propagator masses.
        masses_sq: Vec<f64>,
    },
}

// ---------------------------------------------------------------------------
// Core Mathematical Structures
// ---------------------------------------------------------------------------

/// A relativistic **four-momentum** $p^\mu = (E, p_x, p_y, p_z)$ in natural
/// units ($\hbar = c = 1$), stored in the $(+,-,-,-)$ metric signature.
///
/// # Examples
///
/// ```
/// # use spire_kernel::algebra::FourMomentum;
/// let p = FourMomentum::new(5.0, 3.0, 0.0, 4.0);
/// // Invariant mass squared: E² - |p|² = 25 - 9 - 0 - 16 = 0 (massless)
/// assert!((p.invariant_mass_sq()).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FourMomentum {
    /// Energy component $p^0 = E$ in GeV.
    pub e: f64,
    /// Spatial $x$-component $p^1$ in GeV.
    pub px: f64,
    /// Spatial $y$-component $p^2$ in GeV.
    pub py: f64,
    /// Spatial $z$-component $p^3$ in GeV.
    pub pz: f64,
}

/// A **Dirac spinor** representing an external fermion state.
///
/// For an incoming particle: $u(p, s)$; for an incoming antiparticle: $v(p, s)$.
/// For outgoing: the barred conjugates $\bar{u}$, $\bar{v}$.
///
/// At this stage the spinor is represented symbolically rather than as an
/// explicit 4-component complex column vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiracSpinor {
    /// The 4-momentum of the fermion.
    pub momentum: FourMomentum,
    /// Spin/helicity label.
    pub spin_label: String,
    /// Whether this is a $u$-spinor (particle) or $v$-spinor (antiparticle).
    pub kind: SpinorKind,
    /// Whether this is barred ($\bar{u}$ or $\bar{v}$).
    pub is_barred: bool,
    /// Symbolic tag for this spinor in amplitude expressions (e.g., `"u(p1, s1)"`).
    pub symbol: String,
}

/// Classification of a Dirac spinor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpinorKind {
    /// Positive-energy spinor $u(p, s)$ — particle.
    U,
    /// Barred positive-energy spinor $\bar{u}(p, s)$ — outgoing particle.
    UBar,
    /// Negative-energy spinor $v(p, s)$ — antiparticle.
    V,
    /// Barred negative-energy spinor $\bar{v}(p, s)$ — outgoing antiparticle.
    VBar,
}

/// A **Dirac gamma matrix** $\gamma^\mu$ ($\mu = 0, 1, 2, 3$) or the chiral
/// matrix $\gamma^5 = i\gamma^0\gamma^1\gamma^2\gamma^3$.
///
/// The numerical representation uses `nalgebra::Matrix4<f64>` in the
/// Dirac (standard) basis. At skeleton stage we represent it symbolically
/// with optional numerical content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaMatrix {
    /// The Lorentz index $\mu$ (0–3 for $\gamma^\mu$, or 5 for $\gamma^5$).
    pub index: u8,
    /// Symbolic label (e.g., `"gamma^0"`, `"gamma^5"`).
    pub symbol: String,
    /// The 4×4 numerical matrix representation (Dirac basis).
    /// At skeleton stage this is a placeholder.
    #[serde(skip)]
    pub matrix: Option<Matrix4<f64>>,
}

/// A **polarization vector** $\epsilon^\mu(k, \lambda)$ for an external vector
/// boson (photon, W, Z, gluon).
///
/// Encodes the transverse (for massless) or transverse+longitudinal (for massive)
/// polarization degrees of freedom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolarizationVector {
    /// The 4-momentum of the vector boson.
    pub momentum: FourMomentum,
    /// Polarization state label ($\lambda = \pm 1$ transverse, $0$ longitudinal).
    pub polarization: PolarizationState,
    /// The four components $\epsilon^\mu$.
    pub components: [f64; 4],
    /// Symbolic tag (e.g., `"epsilon(k1, +)"`).
    pub symbol: String,
}

/// Polarization state of a vector boson.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolarizationState {
    /// Transverse plus ($\lambda = +1$).
    TransversePlus,
    /// Transverse minus ($\lambda = -1$).
    TransverseMinus,
    /// Longitudinal ($\lambda = 0$, massive bosons only).
    Longitudinal,
}

// ---------------------------------------------------------------------------
// Symbolic Abstract Syntax Tree (AST) for Amplitudes
// ---------------------------------------------------------------------------

/// A single **symbolic term** in the factorized invariant amplitude $i\mathcal{M}$.
///
/// The full amplitude is the ordered product of all terms:
/// $$i\mathcal{M} = \prod_i T_i$$
///
/// where each $T_i$ is one of:
/// - A Dirac spinor for an external fermion leg.
/// - A polarization vector for an external boson leg.
/// - A vertex coupling factor from the Lagrangian.
/// - A propagator denominator and numerator for an internal line.
///
/// The ordering matters for fermion chains (Dirac algebra is non-commutative)
/// but is otherwise flexible for scalar/coupling factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolicTerm {
    /// An external fermion spinor: kind ($u$, $\bar{u}$, $v$, $\bar{v}$)
    /// and the particle/momentum label.
    ///
    /// Convention:
    /// - Incoming particle → $u(p)$
    /// - Outgoing particle → $\bar{u}(p)$
    /// - Incoming antiparticle → $\bar{v}(p)$
    /// - Outgoing antiparticle → $v(p)$
    Spinor {
        /// Spinor type (U, UBar, V, VBar).
        kind: SpinorKind,
        /// Symbolic label for the particle/momentum (e.g., `"p1"`, `"e-"`).
        label: String,
        /// Momentum label associated with this leg.
        momentum_label: String,
    },

    /// A polarization vector for an external vector boson leg.
    ///
    /// $\epsilon^\mu(k)$ for incoming, $\epsilon^{*\mu}(k)$ for outgoing.
    PolarizationVec {
        /// Momentum label of the boson (e.g., `"k1"`).
        momentum_label: String,
        /// Lorentz index symbol (e.g., `"mu"`, `"nu"`).
        lorentz_index: String,
        /// Whether this is conjugated ($\epsilon^*$ for outgoing bosons).
        is_conjugate: bool,
        /// Field name for display (e.g., `"photon"`).
        field_label: String,
    },

    /// A vertex coupling from the Lagrangian interaction term.
    ///
    /// Contains the symbolic expression for the vertex Feynman rule
    /// (e.g., `"-i e gamma^mu"` for QED).
    VertexCoupling {
        /// The symbolic vertex factor expression.
        expression: String,
        /// The originating Lagrangian term ID.
        term_id: String,
    },

    /// A propagator for an internal (off-shell) line.
    ///
    /// Encodes the mathematical form of the propagator and the momentum
    /// flowing through the internal line.
    PropagatorTerm {
        /// The structured form (Scalar, DiracFermion, MasslessVector, MassiveVector).
        form: PropagatorForm,
        /// The momentum label for this internal line (e.g., `"q"`).
        momentum_label: String,
        /// Mass of the propagating field in GeV.
        mass: f64,
        /// Human-readable expression string.
        expression: String,
    },

    /// A symbolic loop integral in the Passarino-Veltman scalar basis.
    ///
    /// This represents the *unevaluated* integral:
    /// $$\int \frac{d^d l}{(2\pi)^d} \frac{1}{\prod_i [(l+q_i)^2 - m_i^2 + i\epsilon]}$$
    ///
    /// **Design philosophy**: SPIRE never numerically evaluates these integrals.
    /// This AST node is a *mathematical object* that carries all the information
    /// needed for any downstream regularization or numerical library to evaluate it.
    ///
    /// Future phases can:
    /// - Reduce tensor integrals to this scalar basis (Passarino-Veltman).
    /// - Export to LoopTools, Package-X, or FIRE for numerical/analytical evaluation.
    /// - Expand in $\epsilon$ to extract UV poles and finite parts.
    LoopIntegral {
        /// The scalar integral classification (A0, B0, C0, D0, NPoint).
        integral_type: ScalarIntegralType,
        /// The spacetime dimension of the integral.
        spacetime_dim: SpacetimeDimension,
        /// The loop momentum label (e.g., `"l"`, `"l1"`).
        loop_momentum_label: String,
        /// Labels of the external momenta flowing into the diagram.
        external_momenta: Vec<String>,
        /// Description string for display.
        description: String,
    },
}

/// The **invariant amplitude** $i\mathcal{M}$ for a single Feynman diagram,
/// represented as an ordered sequence of [`SymbolicTerm`]s.
///
/// The physical amplitude is the product of all terms:
/// $$i\mathcal{M} = \prod_{i} T_i$$
///
/// This AST is the bridge between topological diagrams and analytical
/// mathematics. Downstream modules (trace evaluation, index contraction)
/// consume this representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmplitudeExpression {
    /// The diagram this amplitude corresponds to.
    pub diagram_id: usize,
    /// Ordered list of symbolic factors forming the amplitude.
    pub terms: Vec<SymbolicTerm>,
    /// The coupling constants appearing in this amplitude.
    pub couplings: Vec<String>,
    /// The 4-momenta labels used in the expression.
    pub momenta_labels: Vec<String>,
    /// Human-readable summary expression string (e.g., LaTeX-style).
    pub expression: String,
}

// ---------------------------------------------------------------------------
// LaTeX Export
// ---------------------------------------------------------------------------

impl SymbolicTerm {
    /// Render this symbolic term as a LaTeX fragment.
    pub fn to_latex(&self) -> String {
        match self {
            SymbolicTerm::Spinor { kind, momentum_label, .. } => {
                let p = latex_momentum(momentum_label);
                match kind {
                    SpinorKind::U => format!("u({p})"),
                    SpinorKind::UBar => format!("\\bar{{u}}({p})"),
                    SpinorKind::V => format!("v({p})"),
                    SpinorKind::VBar => format!("\\bar{{v}}({p})"),
                }
            }
            SymbolicTerm::PolarizationVec {
                momentum_label,
                lorentz_index,
                is_conjugate,
                ..
            } => {
                let idx = latex_lorentz_index(lorentz_index);
                let k = latex_momentum(momentum_label);
                if *is_conjugate {
                    format!("\\epsilon^{{*{idx}}}({k})")
                } else {
                    format!("\\epsilon^{{{idx}}}({k})")
                }
            }
            SymbolicTerm::VertexCoupling { expression, .. } => {
                // Convert the symbolic vertex expression to LaTeX.
                latex_vertex_expression(expression)
            }
            SymbolicTerm::PropagatorTerm { form, momentum_label, mass, .. } => {
                let p = latex_momentum(momentum_label);
                match form {
                    PropagatorForm::Scalar => {
                        if *mass > 1e-10 {
                            format!("\\frac{{i}}{{{p}^2 - {:.4}^2 + i\\epsilon}}", mass)
                        } else {
                            format!("\\frac{{i}}{{{p}^2 + i\\epsilon}}")
                        }
                    }
                    PropagatorForm::DiracFermion => {
                        if *mass > 1e-10 {
                            format!(
                                "\\frac{{i(\\gamma \\cdot {p} + {:.4})}}{{{p}^2 - {:.4}^2 + i\\epsilon}}",
                                mass, mass
                            )
                        } else {
                            format!(
                                "\\frac{{i\\,\\gamma \\cdot {p}}}{{{p}^2 + i\\epsilon}}"
                            )
                        }
                    }
                    PropagatorForm::MasslessVector => {
                        format!("\\frac{{-i\\,g_{{\\mu\\nu}}}}{{{p}^2 + i\\epsilon}}")
                    }
                    PropagatorForm::MassiveVector => {
                        format!(
                            "\\frac{{-i\\left(g_{{\\mu\\nu}} - \\frac{{{p}_\\mu {p}_\\nu}}{{{:.4}^2}}\\right)}}{{{p}^2 - {:.4}^2 + i\\epsilon}}",
                            mass, mass
                        )
                    }
                }
            }
            SymbolicTerm::LoopIntegral {
                integral_type,
                spacetime_dim,
                loop_momentum_label,
                ..
            } => {
                let d = spacetime_dim.to_latex();
                let l = latex_momentum(loop_momentum_label);
                let measure = format!(
                    "\\int \\frac{{d^{{{d}}} {l}}}{{(2\\pi)^{{{d}}}}}"
                );
                let denominator = match integral_type {
                    ScalarIntegralType::A0 { mass_sq } => {
                        format!("\\frac{{1}}{{{l}^2 - {:.4}}}", mass_sq)
                    }
                    ScalarIntegralType::B0 { p_sq, m1_sq, m2_sq } => {
                        let p = latex_momentum(p_sq);
                        format!(
                            "\\frac{{1}}{{[{l}^2 - {:.4}][({l}+{p})^2 - {:.4}]}}",
                            m1_sq, m2_sq
                        )
                    }
                    ScalarIntegralType::C0 { external_momenta_sq, masses_sq } => {
                        let props: Vec<String> = masses_sq
                            .iter()
                            .enumerate()
                            .map(|(i, m)| {
                                if i == 0 {
                                    format!("[{l}^2 - {:.4}]", m)
                                } else if i <= external_momenta_sq.len() {
                                    let p = latex_momentum(&external_momenta_sq[i - 1]);
                                    format!("[({l}+{p})^2 - {:.4}]", m)
                                } else {
                                    format!("[...^2 - {:.4}]", m)
                                }
                            })
                            .collect();
                        format!("\\frac{{1}}{{{}}}", props.join(""))
                    }
                    ScalarIntegralType::D0 { external_momenta_sq, masses_sq, .. } => {
                        let props: Vec<String> = masses_sq
                            .iter()
                            .enumerate()
                            .map(|(i, m)| {
                                if i == 0 {
                                    format!("[{l}^2 - {:.4}]", m)
                                } else if i <= external_momenta_sq.len() {
                                    let p = latex_momentum(&external_momenta_sq[i - 1]);
                                    format!("[({l}+{p})^2 - {:.4}]", m)
                                } else {
                                    format!("[...^2 - {:.4}]", m)
                                }
                            })
                            .collect();
                        format!("\\frac{{1}}{{{}}}", props.join(""))
                    }
                    ScalarIntegralType::NPoint { masses_sq, .. } => {
                        let props: Vec<String> = masses_sq
                            .iter()
                            .enumerate()
                            .map(|(i, m)| {
                                if i == 0 {
                                    format!("[{l}^2 - {:.4}]", m)
                                } else {
                                    format!("[...^2 - {:.4}]", m)
                                }
                            })
                            .collect();
                        format!("\\frac{{1}}{{{}}}", props.join(""))
                    }
                };
                format!("{measure}\\,{denominator}")
            }
        }
    }
}

impl AmplitudeExpression {
    /// Render the full amplitude as a publication-quality **LaTeX** string.
    ///
    /// The output is suitable for use inside a `\begin{equation}` environment
    /// or an inline `$...$` block.
    ///
    /// # Example output
    ///
    /// ```text
    /// i\mathcal{M} = \bar{u}(p_3)\,(-ie\gamma^{\mu})\,
    ///   \frac{i(\gamma \cdot q + 0.0005)}{q^2 - 0.0005^2 + i\epsilon}\,
    ///   (-ie\gamma^{\nu})\,u(p_1)\,\epsilon^{*\mu}(k_1)\,\epsilon^{*\nu}(k_2)
    /// ```
    pub fn to_latex(&self) -> String {
        if self.terms.is_empty() {
            return "i\\mathcal{M} = 1".to_string();
        }

        let body: Vec<String> = self.terms.iter().map(|t| t.to_latex()).collect();
        format!("i\\mathcal{{M}} = {}", body.join("\\,"))
    }
}

/// Convert a momentum label like `"p1"` into LaTeX `p_{1}`.
fn latex_momentum(label: &str) -> String {
    // Try to split a trailing numeric suffix: "p1" → "p_{1}", "q" → "q"
    if let Some(pos) = label.find(|c: char| c.is_ascii_digit()) {
        let (base, num) = label.split_at(pos);
        format!("{base}_{{{num}}}")
    } else {
        label.to_string()
    }
}

/// Convert a Lorentz index label to LaTeX Greek: `"mu"` → `\\mu`, `"nu"` → `\\nu`, etc.
fn latex_lorentz_index(idx: &str) -> String {
    match idx {
        "mu" => "\\mu".into(),
        "nu" => "\\nu".into(),
        "rho" => "\\rho".into(),
        "sigma" => "\\sigma".into(),
        "alpha" => "\\alpha".into(),
        "beta" => "\\beta".into(),
        other => {
            // Handle indexed forms like "mu_6" → "\\mu_{6}"
            if let Some(rest) = other.strip_prefix("mu_") {
                format!("\\mu_{{{rest}}}")
            } else if let Some(rest) = other.strip_prefix("nu_") {
                format!("\\nu_{{{rest}}}")
            } else {
                other.to_string()
            }
        }
    }
}

/// Convert a symbolic vertex expression string to LaTeX notation.
fn latex_vertex_expression(expr: &str) -> String {
    let mut s = expr.to_string();
    // Replace common symbolic patterns with LaTeX.
    s = s.replace("γ^μ", "\\gamma^{\\mu}");
    s = s.replace("gamma^mu", "\\gamma^{\\mu}");
    s = s.replace("γ^5", "\\gamma^{5}");
    s = s.replace("P_L", "P_L");
    s = s.replace("P_R", "P_R");
    s = s.replace("Λ²", "\\Lambda^2");
    s = s.replace("T^a", "T^a");
    // Wrap the coupling symbol: "-i e γ^μ" → "-ie\\gamma^{\\mu}"
    // If there's a leading `-i `, convert to `-i`
    if s.starts_with("-i ") {
        s = format!("-i{}", &s[3..]);
    }
    s
}
///
/// Contains the simplified scalar expression (in terms of metric tensors,
/// Levi-Civita symbols, and 4-momentum dot products).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    /// The original trace expression that was evaluated.
    pub input: String,
    /// The simplified scalar result.
    pub result: String,
    /// Intermediate steps of the simplification (for pedagogical display).
    pub steps: Vec<String>,
}

/// The result of contracting Lorentz indices in an amplitude expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractionResult {
    /// The expression before contraction.
    pub input: String,
    /// The expression after all repeated indices are summed over.
    pub result: String,
}

// ---------------------------------------------------------------------------
// FourMomentum — Arithmetic and Kinematics
// ---------------------------------------------------------------------------

impl FourMomentum {
    /// Create a new four-momentum from components $(E, p_x, p_y, p_z)$.
    pub fn new(e: f64, px: f64, py: f64, pz: f64) -> Self {
        Self { e, px, py, pz }
    }

    /// Create the zero four-momentum.
    pub fn zero() -> Self {
        Self { e: 0.0, px: 0.0, py: 0.0, pz: 0.0 }
    }

    /// Compute the **Minkowski inner product** in the $(+,-,-,-)$ metric:
    ///
    /// $$p \cdot q = p^0 q^0 - p^1 q^1 - p^2 q^2 - p^3 q^3$$
    pub fn dot(&self, other: &FourMomentum) -> f64 {
        self.e * other.e - self.px * other.px - self.py * other.py - self.pz * other.pz
    }

    /// Compute the **invariant mass squared** $p^2 = p^\mu p_\mu$:
    ///
    /// $$p^2 = E^2 - |\vec{p}|^2$$
    ///
    /// For an on-shell particle this equals $m^2$.
    pub fn invariant_mass_sq(&self) -> f64 {
        self.dot(self)
    }

    /// Compute the **three-momentum magnitude** $|\vec{p}| = \sqrt{p_x^2 + p_y^2 + p_z^2}$.
    pub fn three_momentum_magnitude(&self) -> f64 {
        (self.px * self.px + self.py * self.py + self.pz * self.pz).sqrt()
    }

    /// Scale all components by a scalar factor.
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            e: self.e * factor,
            px: self.px * factor,
            py: self.py * factor,
            pz: self.pz * factor,
        }
    }
}

/// Vector addition of two four-momenta: $(p + q)^\mu = p^\mu + q^\mu$.
impl Add for FourMomentum {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            e: self.e + rhs.e,
            px: self.px + rhs.px,
            py: self.py + rhs.py,
            pz: self.pz + rhs.pz,
        }
    }
}

/// Vector subtraction: $(p - q)^\mu = p^\mu - q^\mu$.
impl Sub for FourMomentum {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            e: self.e - rhs.e,
            px: self.px - rhs.px,
            py: self.py - rhs.py,
            pz: self.pz - rhs.pz,
        }
    }
}

/// Negation: $(-p)^\mu = -p^\mu$.
impl Neg for FourMomentum {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            e: -self.e,
            px: -self.px,
            py: -self.py,
            pz: -self.pz,
        }
    }
}

// ---------------------------------------------------------------------------
// Feynman Rules Mapper — generate_amplitude
// ---------------------------------------------------------------------------

/// Assign a Lorentz index label based on a running counter.
///
/// Returns `"mu"`, `"nu"`, `"rho"`, `"sigma"`, `"alpha"`, `"beta"`, ... for
/// indices 0, 1, 2, 3, 4, 5, ...
fn lorentz_index_label(counter: usize) -> String {
    match counter {
        0 => "mu".into(),
        1 => "nu".into(),
        2 => "rho".into(),
        3 => "sigma".into(),
        4 => "alpha".into(),
        5 => "beta".into(),
        n => format!("mu_{}", n),
    }
}

/// Generate the full invariant amplitude $i\mathcal{M}$ for a Feynman diagram.
///
/// Reads the graph topology and applies the Feynman rules:
/// - **External fermion legs** → Dirac spinors $u$, $\bar{u}$, $v$, $\bar{v}$.
/// - **External boson legs** → polarization vectors $\epsilon^\mu$.
/// - **Internal lines** → propagator terms.
/// - **Vertices** → vertex coupling factors.
///
/// # Fermion Flow Convention
///
/// Feynman rules for fermion chains are written by reading **backward**
/// along the fermion line (against the fermion-number arrow):
///
/// $$\bar{u}(p_3) \cdot (-ie\gamma^\mu) \cdot \frac{i(\not{q} + m)}{q^2 - m^2} \cdot (-ie\gamma^\nu) \cdot u(p_1)$$
///
/// For this iteration, we collect all symbolic terms from the graph.
/// Perfect fermion-line ordering is a refinement for a future phase.
///
/// # Arguments
/// * `diagram` — The Feynman diagram to translate into an amplitude.
pub fn generate_amplitude(diagram: &FeynmanGraph) -> SpireResult<AmplitudeExpression> {
    let mut terms = Vec::new();
    let mut couplings = Vec::new();
    let mut momenta_labels = Vec::new();
    let mut lorentz_counter = 0usize;
    let mut expression_parts: Vec<String> = Vec::new();

    // -----------------------------------------------------------------------
    // 1. External legs (edges where is_external == true)
    // -----------------------------------------------------------------------
    for (_src, _tgt, edge) in &diagram.edges {
        if !edge.is_external {
            continue;
        }

        // Determine which node is the external state node
        // (the source for incoming, the target for outgoing).
        // We find the matching node by checking the endpoint node kinds.
        let src_node = diagram.nodes.iter().find(|n| n.id == *_src);
        let tgt_node = diagram.nodes.iter().find(|n| n.id == *_tgt);

        // Determine the spin (2J) of this field
        let spin_2j = edge.field.quantum_numbers.spin.0;

        match spin_2j {
            // ---------------------------------------------------------------
            // Spin-½ fermion (2J = 1)
            // ---------------------------------------------------------------
            1 => {
                // Determine direction and particle/antiparticle status
                let is_incoming = src_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalIncoming(_)))
                    .unwrap_or(false);
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);

                // Check antiparticle status from the external node's particle
                let is_anti = if is_incoming {
                    src_node
                        .and_then(|n| match &n.kind {
                            NodeKind::ExternalIncoming(p) => Some(p.is_anti),
                            _ => None,
                        })
                        .unwrap_or(false)
                } else if is_outgoing {
                    tgt_node
                        .and_then(|n| match &n.kind {
                            NodeKind::ExternalOutgoing(p) => Some(p.is_anti),
                            _ => None,
                        })
                        .unwrap_or(false)
                } else {
                    false
                };

                let spinor_kind = match (is_incoming, is_outgoing, is_anti) {
                    // Incoming particle → u(p)
                    (true, _, false) => SpinorKind::U,
                    // Outgoing particle → ū(p)
                    (_, true, false) => SpinorKind::UBar,
                    // Incoming antiparticle → v̄(p)  (reads as outgoing in fermion flow)
                    (true, _, true) => SpinorKind::VBar,
                    // Outgoing antiparticle → v(p)
                    (_, true, true) => SpinorKind::V,
                    _ => SpinorKind::U, // fallback
                };

                let kind_str = match spinor_kind {
                    SpinorKind::U => "u",
                    SpinorKind::UBar => "ū",
                    SpinorKind::V => "v",
                    SpinorKind::VBar => "v̄",
                };
                let expr = format!("{}({})", kind_str, edge.momentum_label);
                expression_parts.push(expr);

                terms.push(SymbolicTerm::Spinor {
                    kind: spinor_kind,
                    label: edge.field.id.clone(),
                    momentum_label: edge.momentum_label.clone(),
                });

                if !momenta_labels.contains(&edge.momentum_label) {
                    momenta_labels.push(edge.momentum_label.clone());
                }
            }

            // ---------------------------------------------------------------
            // Spin-1 vector boson (2J = 2)
            // ---------------------------------------------------------------
            2 => {
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);

                let idx = lorentz_index_label(lorentz_counter);
                lorentz_counter += 1;

                let conj_str = if is_outgoing { "*" } else { "" };
                let expr = format!(
                    "ε{}^{}({})",
                    conj_str, idx, edge.momentum_label
                );
                expression_parts.push(expr);

                terms.push(SymbolicTerm::PolarizationVec {
                    momentum_label: edge.momentum_label.clone(),
                    lorentz_index: idx,
                    is_conjugate: is_outgoing,
                    field_label: edge.field.id.clone(),
                });

                if !momenta_labels.contains(&edge.momentum_label) {
                    momenta_labels.push(edge.momentum_label.clone());
                }
            }

            // ---------------------------------------------------------------
            // Spin-0 scalar (2J = 0) — external scalar contributes factor 1
            // (no spinor or polarization vector, just a trivial factor).
            // ---------------------------------------------------------------
            _ => {
                // Scalar fields contribute a factor of 1 to the external state.
                // No explicit SymbolicTerm needed, but track the momentum.
                if !momenta_labels.contains(&edge.momentum_label) {
                    momenta_labels.push(edge.momentum_label.clone());
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 2. Internal propagator lines (edges where is_external == false)
    // -----------------------------------------------------------------------
    for (_src, _tgt, edge) in &diagram.edges {
        if edge.is_external {
            continue;
        }

        if let Some(ref prop) = edge.propagator {
            let expr = format!(
                "Propagator[{}, {}, m={}]",
                prop.form_label(),
                edge.momentum_label,
                prop.mass
            );
            expression_parts.push(expr);

            terms.push(SymbolicTerm::PropagatorTerm {
                form: prop.form,
                momentum_label: edge.momentum_label.clone(),
                mass: prop.mass,
                expression: prop.expression.clone(),
            });

            if !momenta_labels.contains(&edge.momentum_label) {
                momenta_labels.push(edge.momentum_label.clone());
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3. Vertex coupling factors (internal vertex nodes)
    // -----------------------------------------------------------------------
    for node in &diagram.nodes {
        if let NodeKind::InternalVertex(ref vf) = node.kind {
            let expr = format!("V[{}]", vf.expression);
            expression_parts.push(expr);

            if let Some(cv) = vf.coupling_value {
                let coupling_str = format!("{:.6}", cv);
                if !couplings.contains(&coupling_str) {
                    couplings.push(coupling_str);
                }
            }

            terms.push(SymbolicTerm::VertexCoupling {
                expression: vf.expression.clone(),
                term_id: vf.term_id.clone(),
            });
        }
    }

    // Build the summary expression string
    let full_expression = if expression_parts.is_empty() {
        "1".into()
    } else {
        expression_parts.join(" × ")
    };

    Ok(AmplitudeExpression {
        diagram_id: diagram.id,
        terms,
        couplings,
        momenta_labels,
        expression: full_expression,
    })
}

// ---------------------------------------------------------------------------
// Algebra Stubs — Trace Evaluation & Index Contraction
// ---------------------------------------------------------------------------

/// Evaluate the trace of a product of Dirac gamma matrices.
///
/// Implements the standard trace identities (Casimir tricks) for computing
/// $\text{Tr}[\gamma^{\mu_1} \gamma^{\mu_2} \cdots]$, including traces
/// involving $\gamma^5$.
///
/// # Current Implementation
///
/// Returns a placeholder `TraceResult` wrapping the input in a `Tr[...]`
/// notation. Full symbolic evaluation of Casimir tricks is planned for a
/// future phase.
///
/// # Arguments
/// * `gamma_indices` — Ordered sequence of gamma matrix Lorentz indices.
/// * `include_gamma5` — Whether $\gamma^5$ appears in the product.
pub fn evaluate_trace(gamma_indices: &[u8], include_gamma5: bool) -> SpireResult<TraceResult> {
    evaluate_trace_d(gamma_indices, include_gamma5, SpacetimeDimension::four())
}

/// Evaluate a Dirac trace in $d$ dimensions.
///
/// This is the core trace routine that all other trace functions delegate to.
/// It uses the standard $d$-dimensional identities:
///
/// | Identity | Result |
/// |----------|--------|
/// | $\mathrm{Tr}[I] = f(d)$ | $f(d)$ where $f(4) = 4$ in general $f(d)$ can be defined |
/// | $\mathrm{Tr}[\text{odd } \gamma]$ | 0 |
/// | $\mathrm{Tr}[\gamma^\mu \gamma^\nu]$ | $f(d)\,g^{\mu\nu}$ |
/// | $\gamma^\mu \gamma_\mu$ | $d \cdot I$ |
/// | $\gamma^\mu \gamma^\nu \gamma_\mu$ | $(2 - d)\gamma^\nu$ |
/// | $\gamma^\mu \gamma^\nu \gamma^\rho \gamma_\mu$ | $4g^{\nu\rho} - (4-d)\gamma^\nu\gamma^\rho$ |
///
/// # Arguments
/// * `gamma_indices` — Ordered sequence of gamma matrix Lorentz indices.
/// * `include_gamma5` — Whether $\gamma^5$ appears in the product.
/// * `dim` — The spacetime dimension to use for the trace algebra.
///
/// # Current Implementation
///
/// Returns a symbolic `TraceResult`. For $d = 4$ (or `Fixed(4)`), the unit
/// trace is `4`; for `DimReg`, symbolic $d$-dependent expressions are returned.
pub fn evaluate_trace_d(
    gamma_indices: &[u8],
    include_gamma5: bool,
    dim: SpacetimeDimension,
) -> SpireResult<TraceResult> {
    let indices_str: Vec<String> = gamma_indices.iter().map(|i| format!("γ^{}", i)).collect();
    let gamma5_str = if include_gamma5 { " γ^5" } else { "" };

    let input = format!("Tr[{}{}]", indices_str.join(" "), gamma5_str);
    let d_str = dim.to_symbolic();

    let result = if gamma_indices.is_empty() {
        // Tr[I] = d  (for DimReg) or 4 (for Fixed(4))
        // In practice the "trace of the unit matrix" in d dimensions is
        // conventionally kept as f(d); the most common convention is f(d)=4
        // for all d (i.e. the trace is over the 4-component spinor space,
        // independent of d). We follow the 't Hooft-Veltman convention:
        // Tr[I] = 4 always.
        "4".into()
    } else if gamma_indices.len() % 2 != 0 && !include_gamma5 {
        // Tr of odd number of gamma matrices = 0 in any dimension.
        "0".into()
    } else if gamma_indices.len() == 2 {
        // Tr[γ^μ γ^ν] = 4 g^{μν}  (convention: Tr[I]=4 in all d)
        let (a, b) = (gamma_indices[0], gamma_indices[1]);
        format!("4·g^{{{},{}}}", a, b)
    } else if gamma_indices.len() == 4 && !include_gamma5 {
        // Tr[γ^μ γ^ν γ^ρ γ^σ] = 4(g^{μν}g^{ρσ} - g^{μρ}g^{νσ} + g^{μσ}g^{νρ})
        let (a, b, c, dd) = (
            gamma_indices[0],
            gamma_indices[1],
            gamma_indices[2],
            gamma_indices[3],
        );
        format!(
            "4·(g^{{{a},{b}}}g^{{{c},{dd}}} - g^{{{a},{c}}}g^{{{b},{dd}}} + g^{{{a},{dd}}}g^{{{b},{c}}})"
        )
    } else {
        // General case: return symbolic Tr[...] annotated with dimension.
        match dim {
            SpacetimeDimension::Fixed(4) => {
                format!("Tr[{}{}]", indices_str.join(" "), gamma5_str)
            }
            _ => {
                format!("Tr_{{d={}}}[{}{}]", d_str, indices_str.join(" "), gamma5_str)
            }
        }
    };

    let mut steps = vec![input.clone()];
    // Record the d-dimensional identity used.
    match dim {
        SpacetimeDimension::DimReg { .. } => {
            steps.push(format!("Applied d-dimensional algebra with d = {}", d_str));
            steps.push(format!("g_μ^μ = {}", dim.metric_trace()));
            steps.push(format!("γ^μ γ_μ = {}", dim.gamma_contraction()));
        }
        _ => {}
    }
    steps.push(result.clone());

    Ok(TraceResult {
        input: input.clone(),
        result,
        steps,
    })
}

/// Contract all repeated Lorentz indices in an amplitude expression.
///
/// Sums over repeated $\mu$, $\nu$, etc. indices using the Minkowski metric
/// $g^{\mu\nu}$ and produces a scalar expression in terms of 4-momentum
/// dot products.
///
/// # Current Implementation
///
/// Returns a placeholder `ContractionResult` wrapping the input expression.
/// Full symbolic index contraction is planned for a future phase.
///
/// # Arguments
/// * `expression` — The symbolic amplitude expression containing uncontracted indices.
pub fn contract_indices(expression: &AmplitudeExpression) -> SpireResult<ContractionResult> {
    contract_indices_d(expression, SpacetimeDimension::four())
}

/// Contract Lorentz indices in $d$ dimensions.
///
/// This is the d-dimensional generalization of [`contract_indices`]. When
/// working in $d = 4 - 2\epsilon$ the metric contraction $g_\mu^\mu = d$
/// introduces $\epsilon$-dependent terms that must be tracked for
/// UV-divergent integrals.
///
/// # Arguments
/// * `expression` — The symbolic amplitude expression.
/// * `dim` — The spacetime dimension for the contraction algebra.
pub fn contract_indices_d(
    expression: &AmplitudeExpression,
    dim: SpacetimeDimension,
) -> SpireResult<ContractionResult> {
    let input = expression.expression.clone();
    let d_str = dim.to_symbolic();

    let result = match dim {
        SpacetimeDimension::Fixed(4) => format!("Contracted[{}]", input),
        _ => format!("Contracted_{{d={}}}[{}]", d_str, input),
    };

    Ok(ContractionResult { input, result })
}

/// Perform spin summation/averaging for unpolarized cross-section calculation.
///
/// Replaces external spinor bilinears with energy-momentum projection operators:
/// $$\sum_s u(p,s)\bar{u}(p,s) = \not{p} + m$$
/// $$\sum_s v(p,s)\bar{v}(p,s) = \not{p} - m$$
///
/// # Arguments
/// * `amplitude` — The amplitude expression to process.
pub fn perform_spin_summation(
    _amplitude: &AmplitudeExpression,
) -> SpireResult<AmplitudeExpression> {
    todo!("Spin summation not yet implemented — requires full Dirac algebra engine")
}

/// Apply polarization sum rules for external vector bosons.
///
/// For massless bosons (photons): $\sum_\lambda \epsilon^\mu \epsilon^{*\nu} = -g^{\mu\nu}$
/// (in Feynman gauge).
/// For massive bosons: includes the longitudinal contribution.
///
/// # Arguments
/// * `amplitude` — The amplitude expression to process.
pub fn apply_polarization_sum(
    _amplitude: &AmplitudeExpression,
) -> SpireResult<AmplitudeExpression> {
    todo!("Polarization sum not yet implemented — requires full tensor algebra engine")
}

// ---------------------------------------------------------------------------
// Helper: Propagator form label
// ---------------------------------------------------------------------------

/// Extension trait for `Propagator` to provide a human-readable form label.
trait PropagatorFormLabel {
    fn form_label(&self) -> &str;
}

impl PropagatorFormLabel for crate::lagrangian::Propagator {
    fn form_label(&self) -> &str {
        match self.form {
            PropagatorForm::Scalar => "Scalar",
            PropagatorForm::DiracFermion => "Dirac",
            PropagatorForm::MasslessVector => "MasslessVector",
            PropagatorForm::MassiveVector => "MassiveVector",
        }
    }
}

// ---------------------------------------------------------------------------
// Loop Integral Construction from FeynmanGraph
// ---------------------------------------------------------------------------

/// Classify the loop integral(s) appearing in a 1-loop `FeynmanGraph` and
/// return the corresponding `SymbolicTerm::LoopIntegral` AST node.
///
/// This function examines the graph's `loop_topology_kind` and
/// `momentum_routing` to build the correct Passarino-Veltman scalar integral
/// classification.
///
/// # Arguments
/// * `graph` — A 1-loop FeynmanGraph with topology already classified.
/// * `propagator_masses` — Masses of the internal loop propagators in order.
/// * `dim` — The spacetime dimension for the integral.
///
/// # Returns
/// A `SymbolicTerm::LoopIntegral` representing the scalar integral, or an
/// error if the graph is not a recognized 1-loop topology.
pub fn classify_loop_integral(
    graph: &FeynmanGraph,
    propagator_masses: &[f64],
    dim: SpacetimeDimension,
) -> SpireResult<SymbolicTerm> {
    let topology = graph.loop_topology_kind.as_ref().ok_or_else(|| {
        crate::SpireError::InternalError(
            "Cannot classify loop integral: graph has no loop_topology_kind".into(),
        )
    })?;

    let routing = graph.momentum_routing.as_ref();
    let loop_label = routing
        .and_then(|r| r.loop_momenta.first())
        .map(|s| s.clone())
        .unwrap_or_else(|| "l".into());

    // Collect external momentum labels from the graph.
    let external_momenta: Vec<String> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::ExternalIncoming(_) | NodeKind::ExternalOutgoing(_)))
        .map(|n| format!("p_{}", n.id))
        .collect();

    let (integral_type, description) = match topology {
        OneLoopTopologyKind::Tadpole => {
            let m_sq = propagator_masses.first().copied().unwrap_or(0.0);
            (
                ScalarIntegralType::A0 { mass_sq: m_sq },
                format!("A₀(m²={:.4})", m_sq),
            )
        }
        OneLoopTopologyKind::Bubble => {
            let m1_sq = propagator_masses.first().copied().unwrap_or(0.0);
            let m2_sq = propagator_masses.get(1).copied().unwrap_or(0.0);
            let p_sq = if external_momenta.len() >= 2 {
                format!("({})²", external_momenta[0])
            } else {
                "p²".into()
            };
            (
                ScalarIntegralType::B0 {
                    p_sq: p_sq.clone(),
                    m1_sq,
                    m2_sq,
                },
                format!("B₀({}; m₁²={:.4}, m₂²={:.4})", p_sq, m1_sq, m2_sq),
            )
        }
        OneLoopTopologyKind::Triangle => {
            let masses: Vec<f64> = (0..3)
                .map(|i| propagator_masses.get(i).copied().unwrap_or(0.0))
                .collect();
            let ext_sq: Vec<String> = external_momenta
                .iter()
                .take(3)
                .map(|p| format!("{}²", p))
                .collect();
            (
                ScalarIntegralType::C0 {
                    external_momenta_sq: ext_sq.clone(),
                    masses_sq: masses.clone(),
                },
                format!(
                    "C₀({}; m²={:?})",
                    ext_sq.join(", "),
                    masses
                ),
            )
        }
        OneLoopTopologyKind::Box => {
            let masses: Vec<f64> = (0..4)
                .map(|i| propagator_masses.get(i).copied().unwrap_or(0.0))
                .collect();
            let ext_sq: Vec<String> = external_momenta
                .iter()
                .take(4)
                .map(|p| format!("{}²", p))
                .collect();
            (
                ScalarIntegralType::D0 {
                    external_momenta_sq: ext_sq.clone(),
                    mandelstam_invariants: vec!["s".into(), "t".into()],
                    masses_sq: masses.clone(),
                },
                format!(
                    "D₀({}; s,t; m²={:?})",
                    ext_sq.join(", "),
                    masses
                ),
            )
        }
        OneLoopTopologyKind::NPoint(n) => {
            let masses: Vec<f64> = (0..*n as usize)
                .map(|i| propagator_masses.get(i).copied().unwrap_or(0.0))
                .collect();
            (
                ScalarIntegralType::NPoint {
                    n: *n,
                    external_momenta: external_momenta.clone(),
                    masses_sq: masses,
                },
                format!("{}-point scalar integral", n),
            )
        }
    };

    Ok(SymbolicTerm::LoopIntegral {
        integral_type,
        spacetime_dim: dim,
        loop_momentum_label: loop_label,
        external_momenta,
        description,
    })
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{
        Channel, Edge, FeynmanGraph, LoopOrder, Node, NodeKind,
    };
    use crate::lagrangian::{
        derive_propagator, LagrangianTerm, LagrangianTermKind, Propagator,
        VertexFactor,
    };
    use crate::ontology::*;
    use petgraph::graph::DiGraph;

    // -----------------------------------------------------------------------
    // Helper: Field constructors (reused from graph tests)
    // -----------------------------------------------------------------------

    fn electron_field() -> Field {
        Field {
            id: "e-".into(),
            name: "Electron".into(),
            symbol: "e^-".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn positron_field() -> Field {
        Field {
            id: "e+".into(),
            name: "Positron".into(),
            symbol: "e^+".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: -1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn muon_field() -> Field {
        Field {
            id: "mu-".into(),
            name: "Muon".into(),
            symbol: "μ^-".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn antimuon_field() -> Field {
        Field {
            id: "mu+".into(),
            name: "Antimuon".into(),
            symbol: "μ^+".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: -1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn photon_field() -> Field {
        Field {
            id: "photon".into(),
            name: "Photon".into(),
            symbol: "γ".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(2),
                parity: Parity::Odd,
                charge_conjugation: ChargeConjugation::Odd,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn higgs_field() -> Field {
        Field {
            id: "H".into(),
            name: "Higgs".into(),
            symbol: "H".into(),
            mass: 125.1,
            width: 0.0032,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(0),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Even,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Yukawa],
        }
    }

    fn make_particle(field: &Field, is_anti: bool) -> Particle {
        Particle {
            field: field.clone(),
            shell: ShellCondition::OnShell,
            helicity: None,
            chirality: None,
            is_anti,
        }
    }

    fn make_vertex_factor(term_id: &str, field_ids: &[&str], expr: &str, coupling: f64) -> VertexFactor {
        VertexFactor {
            term_id: term_id.into(),
            field_ids: field_ids.iter().map(|s| s.to_string()).collect(),
            expression: expr.into(),
            coupling_value: Some(coupling),
            n_legs: field_ids.len() as u8,
        }
    }

    fn make_propagator(field: &Field) -> Propagator {
        derive_propagator(field).unwrap()
    }

    // -----------------------------------------------------------------------
    // 6.1: FourMomentum Kinematics Tests
    // -----------------------------------------------------------------------

    #[test]
    fn four_momentum_new() {
        let p = FourMomentum::new(10.0, 0.0, 0.0, 10.0);
        assert_eq!(p.e, 10.0);
        assert_eq!(p.pz, 10.0);
    }

    #[test]
    fn four_momentum_serde_roundtrip() {
        let p = FourMomentum::new(5.0, 1.0, 2.0, 3.0);
        let json = serde_json::to_string(&p).unwrap();
        let p2: FourMomentum = serde_json::from_str(&json).unwrap();
        assert_eq!(p.e, p2.e);
        assert_eq!(p.px, p2.px);
    }

    #[test]
    fn dot_product_metric_signature() {
        // p = (E, px, py, pz)
        // q = (E, px, py, pz)
        // p · q = E1*E2 - px1*px2 - py1*py2 - pz1*pz2  (+ - - -)
        let p = FourMomentum::new(5.0, 1.0, 2.0, 3.0);
        let q = FourMomentum::new(3.0, 0.5, 1.0, 1.5);
        // 5*3 - 1*0.5 - 2*1.0 - 3*1.5 = 15 - 0.5 - 2.0 - 4.5 = 8.0
        let result = p.dot(&q);
        assert!((result - 8.0).abs() < 1e-12, "dot product = {}", result);
    }

    #[test]
    fn invariant_mass_particle_at_rest() {
        // A particle at rest: p = (m, 0, 0, 0)
        // p² = m² - 0 = m²
        let m = 0.938; // proton mass in GeV
        let p = FourMomentum::new(m, 0.0, 0.0, 0.0);
        let p_sq = p.invariant_mass_sq();
        assert!((p_sq - m * m).abs() < 1e-12, "p² = {}, expected {}", p_sq, m * m);
    }

    #[test]
    fn invariant_mass_massless_particle() {
        // Massless particle: E = |p|
        let p = FourMomentum::new(5.0, 3.0, 0.0, 4.0);
        // E² = 25, |p|² = 9 + 0 + 16 = 25  → p² = 0
        let p_sq = p.invariant_mass_sq();
        assert!(p_sq.abs() < 1e-12, "massless p² = {}, expected 0", p_sq);
    }

    #[test]
    fn invariant_mass_on_shell_electron() {
        // Electron with known momentum
        let m_e: f64 = 0.000511;
        let pz: f64 = 10.0; // 10 GeV along z
        let e = (m_e * m_e + pz * pz).sqrt();
        let p = FourMomentum::new(e, 0.0, 0.0, pz);
        let p_sq = p.invariant_mass_sq();
        assert!(
            (p_sq - m_e * m_e).abs() < 1e-9,
            "electron p² = {}, expected m² = {}",
            p_sq,
            m_e * m_e
        );
    }

    #[test]
    fn four_momentum_addition() {
        let p1 = FourMomentum::new(5.0, 1.0, 0.0, 0.0);
        let p2 = FourMomentum::new(3.0, -1.0, 2.0, 1.0);
        let sum = p1 + p2;
        assert!((sum.e - 8.0).abs() < 1e-12);
        assert!((sum.px - 0.0).abs() < 1e-12);
        assert!((sum.py - 2.0).abs() < 1e-12);
        assert!((sum.pz - 1.0).abs() < 1e-12);
    }

    #[test]
    fn four_momentum_subtraction() {
        let p1 = FourMomentum::new(5.0, 1.0, 2.0, 3.0);
        let p2 = FourMomentum::new(3.0, 0.5, 1.0, 1.5);
        let diff = p1 - p2;
        assert!((diff.e - 2.0).abs() < 1e-12);
        assert!((diff.px - 0.5).abs() < 1e-12);
        assert!((diff.py - 1.0).abs() < 1e-12);
        assert!((diff.pz - 1.5).abs() < 1e-12);
    }

    #[test]
    fn four_momentum_negation() {
        let p = FourMomentum::new(5.0, 1.0, -2.0, 3.0);
        let neg_p = -p;
        assert!((neg_p.e + 5.0).abs() < 1e-12);
        assert!((neg_p.px + 1.0).abs() < 1e-12);
        assert!((neg_p.py - 2.0).abs() < 1e-12);
        assert!((neg_p.pz + 3.0).abs() < 1e-12);
    }

    #[test]
    fn momentum_conservation_decay() {
        // 1 → 2 decay: p_parent = p_daughter1 + p_daughter2
        let p_parent = FourMomentum::new(10.0, 0.0, 0.0, 0.0);
        let p_d1 = FourMomentum::new(6.0, 3.0, 2.0, 1.0);
        let p_d2 = FourMomentum::new(4.0, -3.0, -2.0, -1.0);

        let sum = p_d1 + p_d2;
        assert!((sum.e - p_parent.e).abs() < 1e-12, "energy not conserved");
        assert!((sum.px - p_parent.px).abs() < 1e-12, "px not conserved");
        assert!((sum.py - p_parent.py).abs() < 1e-12, "py not conserved");
        assert!((sum.pz - p_parent.pz).abs() < 1e-12, "pz not conserved");
    }

    #[test]
    fn three_momentum_magnitude() {
        let p = FourMomentum::new(5.0, 3.0, 0.0, 4.0);
        assert!((p.three_momentum_magnitude() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn dot_product_symmetry() {
        let p = FourMomentum::new(5.0, 1.0, 2.0, 3.0);
        let q = FourMomentum::new(3.0, 0.5, 1.0, 1.5);
        assert!((p.dot(&q) - q.dot(&p)).abs() < 1e-12, "dot product should be symmetric");
    }

    #[test]
    fn mandelstam_s_from_momenta() {
        // s = (p1 + p2)² for a 2→2 process in the CM frame
        // e⁺e⁻ at 100 GeV each along z-axis (CM frame, massless approx)
        let p1 = FourMomentum::new(100.0, 0.0, 0.0, 100.0);
        let p2 = FourMomentum::new(100.0, 0.0, 0.0, -100.0);
        let s = (p1 + p2).invariant_mass_sq();
        // s = (200)² - 0 = 40000
        assert!((s - 40000.0).abs() < 1e-8, "s = {}, expected 40000", s);
    }

    // -----------------------------------------------------------------------
    // 6.2: Symbolic AST Tests
    // -----------------------------------------------------------------------

    #[test]
    fn spinor_kind_variants() {
        assert_ne!(SpinorKind::U, SpinorKind::V);
        assert_ne!(SpinorKind::UBar, SpinorKind::VBar);
        assert_ne!(SpinorKind::U, SpinorKind::UBar);
    }

    #[test]
    fn symbolic_term_spinor_serde() {
        let term = SymbolicTerm::Spinor {
            kind: SpinorKind::UBar,
            label: "e-".into(),
            momentum_label: "p3".into(),
        };
        let json = serde_json::to_string(&term).unwrap();
        let term2: SymbolicTerm = serde_json::from_str(&json).unwrap();
        match term2 {
            SymbolicTerm::Spinor { kind, label, momentum_label } => {
                assert_eq!(kind, SpinorKind::UBar);
                assert_eq!(label, "e-");
                assert_eq!(momentum_label, "p3");
            }
            _ => panic!("expected Spinor variant"),
        }
    }

    #[test]
    fn symbolic_term_propagator_serde() {
        let term = SymbolicTerm::PropagatorTerm {
            form: PropagatorForm::DiracFermion,
            momentum_label: "q".into(),
            mass: 0.000511,
            expression: "i(γ·q + m)/(q²-m²)".into(),
        };
        let json = serde_json::to_string(&term).unwrap();
        let term2: SymbolicTerm = serde_json::from_str(&json).unwrap();
        match term2 {
            SymbolicTerm::PropagatorTerm { form, momentum_label, mass, .. } => {
                assert_eq!(form, PropagatorForm::DiracFermion);
                assert_eq!(momentum_label, "q");
                assert!((mass - 0.000511).abs() < 1e-12);
            }
            _ => panic!("expected PropagatorTerm variant"),
        }
    }

    #[test]
    fn amplitude_expression_serde_roundtrip() {
        let amp = AmplitudeExpression {
            diagram_id: 42,
            terms: vec![
                SymbolicTerm::VertexCoupling {
                    expression: "-ie γ^μ".into(),
                    term_id: "qed_eea".into(),
                },
            ],
            couplings: vec!["0.303".into()],
            momenta_labels: vec!["p1".into(), "p2".into()],
            expression: "V[-ie γ^μ]".into(),
        };
        let json = serde_json::to_string(&amp).unwrap();
        let amp2: AmplitudeExpression = serde_json::from_str(&json).unwrap();
        assert_eq!(amp2.diagram_id, 42);
        assert_eq!(amp2.terms.len(), 1);
        assert_eq!(amp2.couplings, vec!["0.303"]);
    }

    // -----------------------------------------------------------------------
    // 6.3: generate_amplitude Tests
    // -----------------------------------------------------------------------

    /// Construct a mock s-channel FeynmanGraph for e⁺e⁻ → μ⁺μ⁻ via γ.
    ///
    /// Structure:
    /// ```text
    ///   [e- (in)] ─p1─→ (V1) ─q─→ (V2) ─p3─→ [μ- (out)]
    ///   [e+ (in)] ─p2─→ (V1)       (V2) ─p4─→ [μ+ (out)]
    /// ```
    ///
    /// 6 nodes, 5 edges (4 external fermion + 1 internal photon propagator).
    fn make_s_channel_ee_to_mumu() -> FeynmanGraph {
        let e_minus = make_particle(&electron_field(), false);
        let e_plus = make_particle(&positron_field(), true);
        let mu_minus = make_particle(&muon_field(), false);
        let mu_plus = make_particle(&antimuon_field(), true);

        let vf1 = make_vertex_factor("qed_annihilation", &["e-", "e+", "photon"], "-i e γ^μ", 0.303);
        let vf2 = make_vertex_factor("qed_mu_annihilation", &["mu-", "mu+", "photon"], "-i e γ^ν", 0.303);

        let photon_prop = make_propagator(&photon_field());

        let mut graph = DiGraph::<Node, Edge>::new();

        // External nodes
        let n_in1 = graph.add_node(Node { id: 0, kind: NodeKind::ExternalIncoming(e_minus.clone()), position: None });
        let n_in2 = graph.add_node(Node { id: 1, kind: NodeKind::ExternalIncoming(e_plus.clone()), position: None });
        let n_v1 = graph.add_node(Node { id: 2, kind: NodeKind::InternalVertex(vf1.clone()), position: None });
        let n_v2 = graph.add_node(Node { id: 3, kind: NodeKind::InternalVertex(vf2.clone()), position: None });
        let n_out1 = graph.add_node(Node { id: 4, kind: NodeKind::ExternalOutgoing(mu_minus.clone()), position: None });
        let n_out2 = graph.add_node(Node { id: 5, kind: NodeKind::ExternalOutgoing(mu_plus.clone()), position: None });

        // Edges
        graph.add_edge(n_in1, n_v1, Edge {
            field: electron_field(), propagator: None,
            momentum_label: "p1".into(), is_external: true,
        });
        graph.add_edge(n_in2, n_v1, Edge {
            field: positron_field(), propagator: None,
            momentum_label: "p2".into(), is_external: true,
        });
        graph.add_edge(n_v1, n_v2, Edge {
            field: photon_field(), propagator: Some(photon_prop),
            momentum_label: "q".into(), is_external: false,
        });
        graph.add_edge(n_v2, n_out1, Edge {
            field: muon_field(), propagator: None,
            momentum_label: "p3".into(), is_external: true,
        });
        graph.add_edge(n_v2, n_out2, Edge {
            field: antimuon_field(), propagator: None,
            momentum_label: "p4".into(), is_external: true,
        });

        let nodes: Vec<Node> = graph.node_weights().cloned().collect();
        let edges: Vec<(usize, usize, Edge)> = graph
            .edge_indices()
            .map(|ei| {
                let (s, t) = graph.edge_endpoints(ei).unwrap();
                (s.index(), t.index(), graph[ei].clone())
            })
            .collect();

        FeynmanGraph {
            id: 0,
            graph,
            nodes,
            edges,
            channels: vec![Channel::S],
            loop_order: LoopOrder::Tree,
            symmetry_factor: 1.0,
            is_connected: true,
            is_one_particle_irreducible: None,
            loop_topology_kind: None,
            momentum_routing: None,
        }
    }

    #[test]
    fn generate_amplitude_s_channel_term_count() {
        // e⁺e⁻ → γ → μ⁺μ⁻ (s-channel)
        // Expected: 4 external spinors + 1 propagator + 2 vertex couplings = 7 terms
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let n_spinors = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::Spinor { .. })).count();
        let n_propagators = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::PropagatorTerm { .. })).count();
        let n_vertices = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::VertexCoupling { .. })).count();
        let n_polarizations = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::PolarizationVec { .. })).count();

        assert_eq!(n_spinors, 4, "4 external fermion spinors (u, ū, v, v̄), got {}", n_spinors);
        assert_eq!(n_propagators, 1, "1 internal photon propagator, got {}", n_propagators);
        assert_eq!(n_vertices, 2, "2 vertex couplings, got {}", n_vertices);
        assert_eq!(n_polarizations, 0, "no external bosons, got {}", n_polarizations);
        assert_eq!(amp.terms.len(), 7, "total 7 terms, got {}", amp.terms.len());
    }

    #[test]
    fn generate_amplitude_spinor_kinds_correct() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let spinors: Vec<_> = amp.terms.iter().filter_map(|t| match t {
            SymbolicTerm::Spinor { kind, label, .. } => Some((*kind, label.clone())),
            _ => None,
        }).collect();

        // Incoming e- (particle, not anti) → U
        assert!(spinors.iter().any(|(k, l)| *k == SpinorKind::U && l == "e-"),
            "Expected U spinor for incoming e-");
        // Incoming e+ (antiparticle) → VBar
        assert!(spinors.iter().any(|(k, l)| *k == SpinorKind::VBar && l == "e+"),
            "Expected VBar spinor for incoming e+");
        // Outgoing μ- (particle) → UBar
        assert!(spinors.iter().any(|(k, l)| *k == SpinorKind::UBar && l == "mu-"),
            "Expected UBar spinor for outgoing μ-");
        // Outgoing μ+ (antiparticle) → V
        assert!(spinors.iter().any(|(k, l)| *k == SpinorKind::V && l == "mu+"),
            "Expected V spinor for outgoing μ+");
    }

    #[test]
    fn generate_amplitude_propagator_form() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let props: Vec<_> = amp.terms.iter().filter_map(|t| match t {
            SymbolicTerm::PropagatorTerm { form, momentum_label, mass, .. } => {
                Some((*form, momentum_label.clone(), *mass))
            }
            _ => None,
        }).collect();

        assert_eq!(props.len(), 1);
        assert_eq!(props[0].0, PropagatorForm::MasslessVector, "photon propagator form");
        assert_eq!(props[0].1, "q", "propagator momentum label");
        assert!((props[0].2).abs() < 1e-12, "photon mass = 0");
    }

    #[test]
    fn generate_amplitude_vertex_expressions() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let vertices: Vec<_> = amp.terms.iter().filter_map(|t| match t {
            SymbolicTerm::VertexCoupling { expression, term_id } => {
                Some((expression.clone(), term_id.clone()))
            }
            _ => None,
        }).collect();

        assert_eq!(vertices.len(), 2);
        assert!(vertices.iter().any(|(e, _)| e.contains("γ^μ")));
        assert!(vertices.iter().any(|(e, _)| e.contains("γ^ν")));
    }

    #[test]
    fn generate_amplitude_momenta_collected() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        assert!(amp.momenta_labels.contains(&"p1".to_string()));
        assert!(amp.momenta_labels.contains(&"p2".to_string()));
        assert!(amp.momenta_labels.contains(&"p3".to_string()));
        assert!(amp.momenta_labels.contains(&"p4".to_string()));
        assert!(amp.momenta_labels.contains(&"q".to_string()));
    }

    #[test]
    fn generate_amplitude_couplings_collected() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        assert!(!amp.couplings.is_empty(), "should collect coupling constants");
    }

    #[test]
    fn generate_amplitude_expression_string_nonempty() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        assert!(!amp.expression.is_empty());
        assert!(amp.expression.contains("×"), "expression should join terms with ×");
    }

    /// Test with external bosons: build a 1→2 decay with two photons in final state.
    /// This tests PolarizationVec generation.
    fn make_decay_to_photons() -> FeynmanGraph {
        // Fictitious "X → γγ" decay vertex
        let higgs = make_particle(&higgs_field(), false);
        let vf = make_vertex_factor("hgg", &["H", "photon", "photon"], "-i λ", 0.01);

        let mut graph = DiGraph::<Node, Edge>::new();

        let n_in = graph.add_node(Node { id: 0, kind: NodeKind::ExternalIncoming(higgs), position: None });
        let n_v = graph.add_node(Node { id: 1, kind: NodeKind::InternalVertex(vf.clone()), position: None });
        let n_out1 = graph.add_node(Node {
            id: 2,
            kind: NodeKind::ExternalOutgoing(make_particle(&photon_field(), false)),
            position: None,
        });
        let n_out2 = graph.add_node(Node {
            id: 3,
            kind: NodeKind::ExternalOutgoing(make_particle(&photon_field(), false)),
            position: None,
        });

        graph.add_edge(n_in, n_v, Edge {
            field: higgs_field(), propagator: None,
            momentum_label: "p1".into(), is_external: true,
        });
        graph.add_edge(n_v, n_out1, Edge {
            field: photon_field(), propagator: None,
            momentum_label: "p2".into(), is_external: true,
        });
        graph.add_edge(n_v, n_out2, Edge {
            field: photon_field(), propagator: None,
            momentum_label: "p3".into(), is_external: true,
        });

        let nodes: Vec<Node> = graph.node_weights().cloned().collect();
        let edges: Vec<(usize, usize, Edge)> = graph
            .edge_indices()
            .map(|ei| {
                let (s, t) = graph.edge_endpoints(ei).unwrap();
                (s.index(), t.index(), graph[ei].clone())
            })
            .collect();

        FeynmanGraph {
            id: 1,
            graph,
            nodes,
            edges,
            channels: vec![Channel::S],
            loop_order: LoopOrder::Tree,
            symmetry_factor: 0.5,
            is_connected: true,
            is_one_particle_irreducible: None,
            loop_topology_kind: None,
            momentum_routing: None,
        }
    }

    #[test]
    fn generate_amplitude_external_bosons() {
        // H → γγ: 2 outgoing photons → 2 PolarizationVec + 1 vertex + 0 propagators
        let diagram = make_decay_to_photons();
        let amp = generate_amplitude(&diagram).unwrap();

        let n_pol = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::PolarizationVec { .. })).count();
        let n_vert = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::VertexCoupling { .. })).count();
        let n_prop = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::PropagatorTerm { .. })).count();
        let n_spinors = amp.terms.iter().filter(|t| matches!(t, SymbolicTerm::Spinor { .. })).count();

        assert_eq!(n_pol, 2, "2 outgoing photon polarization vectors");
        assert_eq!(n_vert, 1, "1 vertex coupling");
        assert_eq!(n_prop, 0, "no internal propagators");
        assert_eq!(n_spinors, 0, "no fermion spinors (scalar→boson decay)");
    }

    #[test]
    fn generate_amplitude_polarization_conjugation() {
        let diagram = make_decay_to_photons();
        let amp = generate_amplitude(&diagram).unwrap();

        let pols: Vec<_> = amp.terms.iter().filter_map(|t| match t {
            SymbolicTerm::PolarizationVec { is_conjugate, .. } => Some(*is_conjugate),
            _ => None,
        }).collect();

        // Outgoing bosons should have conjugated polarization vectors (ε*)
        assert!(pols.iter().all(|c| *c), "outgoing photon ε should be conjugated");
    }

    // -----------------------------------------------------------------------
    // 6.4: Trace and Contraction Stub Tests
    // -----------------------------------------------------------------------

    #[test]
    fn evaluate_trace_identity() {
        // Tr[I] = 4
        let result = evaluate_trace(&[], false).unwrap();
        assert_eq!(result.result, "4");
    }

    #[test]
    fn evaluate_trace_odd_gamma_vanishes() {
        // Tr[γ^μ] = 0, Tr[γ^μ γ^ν γ^ρ] = 0 (odd number)
        let result = evaluate_trace(&[0], false).unwrap();
        assert_eq!(result.result, "0");

        let result3 = evaluate_trace(&[0, 1, 2], false).unwrap();
        assert_eq!(result3.result, "0");
    }

    #[test]
    fn evaluate_trace_even_gamma_symbolic() {
        // Tr[γ^0 γ^1] = 4·g^{0,1}  (standard 2-gamma trace identity)
        let result = evaluate_trace(&[0, 1], false).unwrap();
        assert!(
            result.result.contains("4·g^{0,1}"),
            "even trace: {}",
            result.result
        );
    }

    #[test]
    fn evaluate_trace_with_gamma5() {
        let result = evaluate_trace(&[0, 1, 2, 3], true).unwrap();
        assert!(result.input.contains("γ^5"));
    }

    #[test]
    fn contract_indices_placeholder() {
        let amp = AmplitudeExpression {
            diagram_id: 0,
            terms: vec![],
            couplings: vec![],
            momenta_labels: vec![],
            expression: "ū(p3) (-ieγ^μ) u(p1) × Prop(q) × ū(p4) (-ieγ^ν) u(p2)".into(),
        };

        let result = contract_indices(&amp).unwrap();
        assert!(result.result.starts_with("Contracted["));
        assert_eq!(result.input, amp.expression);
    }

    // -----------------------------------------------------------------------
    // 6.5: Integration with graph module
    // -----------------------------------------------------------------------

    #[test]
    fn generate_amplitude_from_graph_module() {
        // Use the graph module's full model to generate topologies,
        // then translate a diagram to an amplitude.
        use crate::graph::generate_topologies;
        use crate::lagrangian::{TheoreticalModel, derive_propagator, derive_vertex_factor};
        use crate::s_matrix::{AsymptoticState, Reaction};
        use crate::ontology::initialize_state;

        // Minimal model: e⁺e⁻ → μ⁺μ⁻ via photon only
        let fields = vec![electron_field(), positron_field(), muon_field(), antimuon_field(), photon_field()];
        let terms = vec![
            LagrangianTerm {
                id: "qed_annihilation_a".into(),
                description: "e+e- annihilation photon".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["e-".into(), "e+".into(), "photon".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            LagrangianTerm {
                id: "qed_mu_annihilation_a".into(),
                description: "mu+mu- annihilation photon".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["mu-".into(), "mu+".into(), "photon".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
        ];

        let propagators = fields.iter().map(|f| derive_propagator(f).unwrap()).collect();
        let vertex_factors = terms.iter().map(|t| derive_vertex_factor(t).unwrap()).collect();

        let model = TheoreticalModel {
            name: "Minimal QED".into(),
            description: "Test".into(),
            fields,
            terms,
            vertex_factors,
            propagators,
            gauge_symmetry: None,
        };

        let initial_states = vec![
            initialize_state(&electron_field(), [100.0, 0.0, 0.0, 100.0], None).unwrap(),
            initialize_state(&positron_field(), [100.0, 0.0, 0.0, -100.0], None).unwrap(),
        ];
        let final_states = vec![
            initialize_state(&muon_field(), [100.0, 50.0, 0.0, 86.6], None).unwrap(),
            initialize_state(&antimuon_field(), [100.0, -50.0, 0.0, -86.6], None).unwrap(),
        ];

        let reaction = Reaction {
            initial: AsymptoticState { states: initial_states, invariant_mass_sq: 40000.0 },
            final_state: AsymptoticState { states: final_states, invariant_mass_sq: 40000.0 },
            is_valid: true,
            violation_diagnostics: vec![],
            interaction_types: vec![InteractionType::Electromagnetic],
            mediating_bosons: vec![],
            perturbative_order: 0,
        };

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert!(topos.count > 0, "Should generate at least 1 diagram");

        // Generate amplitude for the first diagram
        let amp = generate_amplitude(&topos.diagrams[0]).unwrap();

        // For a 2→2 tree-level s-channel: 4 spinors + 1 propagator + 2 vertices = 7
        assert_eq!(amp.terms.len(), 7, "integrated test: expected 7 terms, got {}", amp.terms.len());
    }

    #[test]
    fn generate_amplitude_empty_graph() {
        let diagram = FeynmanGraph {
            id: 99,
            graph: DiGraph::new(),
            nodes: vec![],
            edges: vec![],
            channels: vec![],
            loop_order: LoopOrder::Tree,
            symmetry_factor: 1.0,
            is_connected: true,
            is_one_particle_irreducible: None,
            loop_topology_kind: None,
            momentum_routing: None,
        };

        let amp = generate_amplitude(&diagram).unwrap();
        assert_eq!(amp.terms.len(), 0);
        assert_eq!(amp.expression, "1");
    }

    // -----------------------------------------------------------------------
    // 6.6: LaTeX Export Tests
    // -----------------------------------------------------------------------

    #[test]
    fn latex_spinor_u() {
        let term = SymbolicTerm::Spinor {
            kind: SpinorKind::U,
            label: "e-".into(),
            momentum_label: "p1".into(),
        };
        let latex = term.to_latex();
        assert_eq!(latex, "u(p_{1})");
    }

    #[test]
    fn latex_spinor_ubar() {
        let term = SymbolicTerm::Spinor {
            kind: SpinorKind::UBar,
            label: "mu-".into(),
            momentum_label: "p3".into(),
        };
        let latex = term.to_latex();
        assert_eq!(latex, "\\bar{u}(p_{3})");
    }

    #[test]
    fn latex_spinor_v() {
        let term = SymbolicTerm::Spinor {
            kind: SpinorKind::V,
            label: "e+".into(),
            momentum_label: "p2".into(),
        };
        let latex = term.to_latex();
        assert_eq!(latex, "v(p_{2})");
    }

    #[test]
    fn latex_spinor_vbar() {
        let term = SymbolicTerm::Spinor {
            kind: SpinorKind::VBar,
            label: "mu+".into(),
            momentum_label: "p4".into(),
        };
        let latex = term.to_latex();
        assert_eq!(latex, "\\bar{v}(p_{4})");
    }

    #[test]
    fn latex_polarization_vec_conjugate() {
        let term = SymbolicTerm::PolarizationVec {
            momentum_label: "k1".into(),
            lorentz_index: "mu".into(),
            is_conjugate: true,
            field_label: "photon".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\epsilon^{*\\mu}"));
        assert!(latex.contains("k_{1}"));
    }

    #[test]
    fn latex_polarization_vec_not_conjugate() {
        let term = SymbolicTerm::PolarizationVec {
            momentum_label: "k2".into(),
            lorentz_index: "nu".into(),
            is_conjugate: false,
            field_label: "Z0".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\epsilon^{\\nu}"));
        assert!(latex.contains("k_{2}"));
    }

    #[test]
    fn latex_vertex_coupling() {
        let term = SymbolicTerm::VertexCoupling {
            expression: "-ie γ^μ".into(),
            term_id: "qed_eea".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\gamma^{\\mu}"), "got: {}", latex);
    }

    #[test]
    fn latex_propagator_scalar() {
        let term = SymbolicTerm::PropagatorTerm {
            form: PropagatorForm::Scalar,
            momentum_label: "q".into(),
            mass: 125.1,
            expression: "scalar prop".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\frac{i}"), "got: {}", latex);
        assert!(latex.contains("q^2"), "got: {}", latex);
        assert!(latex.contains("125.1"), "got: {}", latex);
    }

    #[test]
    fn latex_propagator_dirac_fermion() {
        let term = SymbolicTerm::PropagatorTerm {
            form: PropagatorForm::DiracFermion,
            momentum_label: "q".into(),
            mass: 0.000511,
            expression: "fermion prop".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\gamma \\cdot q"), "got: {}", latex);
        assert!(latex.contains("0.0005"), "got: {}", latex);
    }

    #[test]
    fn latex_propagator_massless_vector() {
        let term = SymbolicTerm::PropagatorTerm {
            form: PropagatorForm::MasslessVector,
            momentum_label: "q".into(),
            mass: 0.0,
            expression: "photon prop".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("g_{\\mu\\nu}"), "got: {}", latex);
        assert!(latex.contains("q^2"), "got: {}", latex);
    }

    #[test]
    fn latex_propagator_massive_vector() {
        let term = SymbolicTerm::PropagatorTerm {
            form: PropagatorForm::MassiveVector,
            momentum_label: "q".into(),
            mass: 91.1876,
            expression: "Z prop".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("g_{\\mu\\nu}"), "got: {}", latex);
        assert!(latex.contains("q_\\mu q_\\nu"), "got: {}", latex);
        assert!(latex.contains("91.1876"), "got: {}", latex);
    }

    #[test]
    fn latex_amplitude_expression_prefix() {
        let amp = AmplitudeExpression {
            diagram_id: 0,
            terms: vec![
                SymbolicTerm::Spinor {
                    kind: SpinorKind::UBar,
                    label: "mu-".into(),
                    momentum_label: "p3".into(),
                },
                SymbolicTerm::VertexCoupling {
                    expression: "-ie γ^μ".into(),
                    term_id: "qed".into(),
                },
                SymbolicTerm::Spinor {
                    kind: SpinorKind::U,
                    label: "e-".into(),
                    momentum_label: "p1".into(),
                },
            ],
            couplings: vec!["e".into()],
            momenta_labels: vec!["p1".into(), "p3".into()],
            expression: "test".into(),
        };
        let latex = amp.to_latex();
        assert!(latex.starts_with("i\\mathcal{M} = "), "got: {}", latex);
        assert!(latex.contains("\\bar{u}(p_{3})"));
        assert!(latex.contains("u(p_{1})"));
    }

    #[test]
    fn latex_momentum_subscript() {
        assert_eq!(latex_momentum("p1"), "p_{1}");
        assert_eq!(latex_momentum("p12"), "p_{12}");
        assert_eq!(latex_momentum("q"), "q");
        assert_eq!(latex_momentum("k3"), "k_{3}");
    }

    #[test]
    fn latex_lorentz_index_greek() {
        assert_eq!(latex_lorentz_index("mu"), "\\mu");
        assert_eq!(latex_lorentz_index("nu"), "\\nu");
        assert_eq!(latex_lorentz_index("rho"), "\\rho");
        assert_eq!(latex_lorentz_index("sigma"), "\\sigma");
        assert_eq!(latex_lorentz_index("mu_3"), "\\mu_{3}");
    }

    // ===================================================================
    // Phase 15 — Dimensional Regularization & Loop Integral Tests
    // ===================================================================

    #[test]
    fn spacetime_dimension_fixed_four() {
        let d = SpacetimeDimension::four();
        assert_eq!(d.to_symbolic(), "4");
        assert_eq!(d.metric_trace(), "4");
        assert_eq!(d.gamma_contraction(), "4·I");
        assert_eq!(d.to_latex(), "4");
    }

    #[test]
    fn spacetime_dimension_dim_reg() {
        let d = SpacetimeDimension::dim_reg_4();
        assert_eq!(d.to_symbolic(), "4 - 2ε");
        assert_eq!(d.metric_trace(), "4 - 2ε");
        assert_eq!(d.gamma_contraction(), "(4 - 2ε)·I");
        assert_eq!(d.single_gamma_contraction(), "(-2 + 2ε)·γ^ν");
        assert_eq!(d.to_latex(), "4 - 2\\epsilon");
    }

    #[test]
    fn spacetime_dimension_double_gamma_contraction() {
        let d4 = SpacetimeDimension::four();
        assert_eq!(d4.double_gamma_contraction(), "4·g^{νρ}");

        let dreg = SpacetimeDimension::dim_reg_4();
        assert!(dreg.double_gamma_contraction().contains("4·g^{νρ}"));
        assert!(dreg.double_gamma_contraction().contains("2ε"));
    }

    #[test]
    fn spacetime_dimension_serde_roundtrip() {
        let dims = vec![
            SpacetimeDimension::Fixed(4),
            SpacetimeDimension::DimReg { base: 4 },
            SpacetimeDimension::Fixed(6),
        ];
        for d in &dims {
            let json = serde_json::to_string(d).unwrap();
            let back: SpacetimeDimension = serde_json::from_str(&json).unwrap();
            assert_eq!(*d, back);
        }
    }

    #[test]
    fn evaluate_trace_d_identity_always_4() {
        // Tr[I] = 4 in any dimension ('t Hooft-Veltman convention)
        let r4 = evaluate_trace_d(&[], false, SpacetimeDimension::four()).unwrap();
        assert_eq!(r4.result, "4");

        let rd = evaluate_trace_d(&[], false, SpacetimeDimension::dim_reg_4()).unwrap();
        assert_eq!(rd.result, "4");
    }

    #[test]
    fn evaluate_trace_d_odd_vanishes() {
        let r = evaluate_trace_d(&[0, 1, 2], false, SpacetimeDimension::dim_reg_4()).unwrap();
        assert_eq!(r.result, "0");
    }

    #[test]
    fn evaluate_trace_d_two_gamma() {
        let r = evaluate_trace_d(&[0, 1], false, SpacetimeDimension::dim_reg_4()).unwrap();
        assert!(r.result.contains("4·g^{0,1}"), "got: {}", r.result);
    }

    #[test]
    fn evaluate_trace_d_four_gamma() {
        let r = evaluate_trace_d(&[0, 1, 2, 3], false, SpacetimeDimension::four()).unwrap();
        assert!(r.result.contains("g^{0,1}"), "got: {}", r.result);
        assert!(r.result.contains("g^{2,3}"), "got: {}", r.result);
    }

    #[test]
    fn evaluate_trace_d_steps_record_dim_info() {
        let r = evaluate_trace_d(&[0, 1, 2, 3, 4, 5], false, SpacetimeDimension::dim_reg_4())
            .unwrap();
        assert!(r.steps.iter().any(|s| s.contains("d-dimensional")));
        assert!(r.steps.iter().any(|s| s.contains("4 - 2ε")));
    }

    #[test]
    fn contract_indices_d_four() {
        let amp = AmplitudeExpression {
            diagram_id: 0,
            terms: vec![],
            couplings: vec![],
            momenta_labels: vec![],
            expression: "test_expr".into(),
        };
        let r = contract_indices_d(&amp, SpacetimeDimension::four()).unwrap();
        assert_eq!(r.result, "Contracted[test_expr]");
    }

    #[test]
    fn contract_indices_d_dim_reg() {
        let amp = AmplitudeExpression {
            diagram_id: 0,
            terms: vec![],
            couplings: vec![],
            momenta_labels: vec![],
            expression: "test_expr".into(),
        };
        let r = contract_indices_d(&amp, SpacetimeDimension::dim_reg_4()).unwrap();
        assert!(r.result.contains("d=4 - 2ε"), "got: {}", r.result);
    }

    #[test]
    fn scalar_integral_a0_serde_roundtrip() {
        let integral = ScalarIntegralType::A0 { mass_sq: 0.105658 };
        let json = serde_json::to_string(&integral).unwrap();
        let back: ScalarIntegralType = serde_json::from_str(&json).unwrap();
        assert_eq!(integral, back);
    }

    #[test]
    fn scalar_integral_b0_serde_roundtrip() {
        let integral = ScalarIntegralType::B0 {
            p_sq: "p²".into(),
            m1_sq: 0.000511,
            m2_sq: 0.000511,
        };
        let json = serde_json::to_string(&integral).unwrap();
        let back: ScalarIntegralType = serde_json::from_str(&json).unwrap();
        assert_eq!(integral, back);
    }

    #[test]
    fn loop_integral_latex_a0() {
        let term = SymbolicTerm::LoopIntegral {
            integral_type: ScalarIntegralType::A0 { mass_sq: 0.25 },
            spacetime_dim: SpacetimeDimension::dim_reg_4(),
            loop_momentum_label: "l".into(),
            external_momenta: vec!["p1".into()],
            description: "A₀(m²=0.25)".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\int"), "got: {}", latex);
        assert!(latex.contains("d^{4 - 2\\epsilon}"), "got: {}", latex);
        assert!(latex.contains("(2\\pi)"), "got: {}", latex);
        assert!(latex.contains("0.2500"), "got: {}", latex);
    }

    #[test]
    fn loop_integral_latex_b0() {
        let term = SymbolicTerm::LoopIntegral {
            integral_type: ScalarIntegralType::B0 {
                p_sq: "p1".into(),
                m1_sq: 0.0,
                m2_sq: 0.000511,
            },
            spacetime_dim: SpacetimeDimension::Fixed(4),
            loop_momentum_label: "l".into(),
            external_momenta: vec!["p1".into(), "p2".into()],
            description: "B₀".into(),
        };
        let latex = term.to_latex();
        assert!(latex.contains("\\int"), "got: {}", latex);
        assert!(latex.contains("l"), "got: {}", latex);
    }

    #[test]
    fn loop_integral_serde_roundtrip() {
        let term = SymbolicTerm::LoopIntegral {
            integral_type: ScalarIntegralType::C0 {
                external_momenta_sq: vec!["p1²".into(), "p2²".into()],
                masses_sq: vec![0.0, 0.0, 0.0],
            },
            spacetime_dim: SpacetimeDimension::dim_reg_4(),
            loop_momentum_label: "l".into(),
            external_momenta: vec!["p1".into(), "p2".into(), "p3".into()],
            description: "C₀ triangle".into(),
        };
        let json = serde_json::to_string(&term).unwrap();
        let back: SymbolicTerm = serde_json::from_str(&json).unwrap();
        // Just check it round-trips without panic
        if let SymbolicTerm::LoopIntegral { integral_type, .. } = &back {
            matches!(integral_type, ScalarIntegralType::C0 { .. });
        } else {
            panic!("Expected LoopIntegral variant");
        }
    }
}
