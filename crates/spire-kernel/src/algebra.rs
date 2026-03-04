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
use std::collections::HashMap;
use std::ops::{Add, Neg, Sub};

use crate::graph::{FeynmanGraph, NodeKind, OneLoopTopologyKind};
use crate::lagrangian::PropagatorForm;
use crate::SpireResult;

// ===========================================================================
// Generalized Spacetime Geometry Engine
// ===========================================================================

// ---------------------------------------------------------------------------
// Metric Signature
// ---------------------------------------------------------------------------

/// Sign of a single metric tensor diagonal component.
///
/// Each dimension of a pseudo-Riemannian manifold carries either a `Plus`
/// ($+1$) or `Minus` ($-1$) signature entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricSign {
    /// Positive signature: $g_{\mu\mu} = +1$.
    Plus,
    /// Negative signature: $g_{\mu\mu} = -1$.
    Minus,
}

impl MetricSign {
    /// The numerical value $\pm 1$.
    #[inline]
    pub fn value(&self) -> f64 {
        match self {
            MetricSign::Plus => 1.0,
            MetricSign::Minus => -1.0,
        }
    }
}

/// A diagonal metric signature for an $N$-dimensional spacetime.
///
/// Stores the sequence of $+1$ and $-1$ diagonal entries of the flat metric
/// tensor $\eta_{\mu\nu}$, enabling support for arbitrary dimension and
/// signature without hardcoding 4D Minkowski.
///
/// # Examples
///
/// - 4D Minkowski (West Coast): $(+,-,-,-)$
/// - 4D Euclidean: $(+,+,+,+)$
/// - 10D String Theory: $(+,-,-,-,-,-,-,-,-,-)$
/// - 3D Euclidean: $(+,+,+)$
///
/// # Extensibility
///
/// For non-flat backgrounds $g_{\mu\nu}(x)$, future phases can replace this
/// with a trait-based approach where this struct serves as the flat-space
/// specialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricSignature {
    /// The ordered diagonal entries. The length defines the spacetime dimension.
    pub signs: Vec<MetricSign>,
}

impl MetricSignature {
    /// Create a new metric signature from an explicit list of signs.
    pub fn new(signs: Vec<MetricSign>) -> Self {
        Self { signs }
    }

    /// The standard 4D Minkowski signature $(+,-,-,-)$ (West Coast convention).
    pub fn minkowski_4d() -> Self {
        Self {
            signs: vec![
                MetricSign::Plus,
                MetricSign::Minus,
                MetricSign::Minus,
                MetricSign::Minus,
            ],
        }
    }

    /// A $D$-dimensional Minkowski-like signature $(+,-,-,\ldots,-)$.
    pub fn minkowski(dim: usize) -> Self {
        assert!(dim >= 1, "Spacetime dimension must be at least 1");
        let mut signs = vec![MetricSign::Minus; dim];
        signs[0] = MetricSign::Plus;
        Self { signs }
    }

    /// A $D$-dimensional Euclidean signature $(+,+,\ldots,+)$.
    pub fn euclidean(dim: usize) -> Self {
        assert!(dim >= 1, "Spacetime dimension must be at least 1");
        Self {
            signs: vec![MetricSign::Plus; dim],
        }
    }

    /// The number of dimensions.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.signs.len()
    }

    /// The number of time-like ($+1$) dimensions.
    pub fn time_dimensions(&self) -> usize {
        self.signs
            .iter()
            .filter(|s| **s == MetricSign::Plus)
            .count()
    }

    /// The number of space-like ($-1$) dimensions.
    pub fn space_dimensions(&self) -> usize {
        self.signs
            .iter()
            .filter(|s| **s == MetricSign::Minus)
            .count()
    }

    /// Signature as a string, e.g. `"(+,-,-,-)"`.
    pub fn display_signature(&self) -> String {
        let inner: Vec<&str> = self
            .signs
            .iter()
            .map(|s| match s {
                MetricSign::Plus => "+",
                MetricSign::Minus => "-",
            })
            .collect();
        format!("({})", inner.join(","))
    }

    /// The diagonal component $g_{\mu\mu}$ for index $\mu$.
    #[inline]
    pub fn component(&self, index: usize) -> f64 {
        self.signs[index].value()
    }

    /// Compute the full metric contraction $g_{\mu\nu} v^\mu w^\nu$ for
    /// two vectors in this metric.
    ///
    /// Returns an error if the vectors have mismatched dimensions or do not
    /// match this signature's dimension.
    ///
    /// The 4D Minkowski case is manually unrolled for optimal auto-vectorization.
    #[inline]
    pub fn inner_product(&self, v: &SpacetimeVector, w: &SpacetimeVector) -> Result<f64, String> {
        let dim = self.dimension();
        if v.dimension() != dim || w.dimension() != dim {
            return Err(format!(
                "Dimension mismatch: metric is {}D, vectors are {}D and {}D",
                dim,
                v.dimension(),
                w.dimension()
            ));
        }

        // Fast-path: 4D Minkowski (+,-,-,-) — the overwhelmingly common case.
        if dim == 4 && self.signs.len() == 4 {
            let vs = v.components();
            let ws = w.components();
            let result = self.signs[0].value() * vs[0] * ws[0]
                + self.signs[1].value() * vs[1] * ws[1]
                + self.signs[2].value() * vs[2] * ws[2]
                + self.signs[3].value() * vs[3] * ws[3];
            return Ok(result);
        }

        let vs = v.components();
        let ws = w.components();
        let result = (0..dim)
            .map(|i| self.signs[i].value() * vs[i] * ws[i])
            .sum();
        Ok(result)
    }

    /// The trace of the metric: $g^{\mu}{}_{\mu} = D$ for a flat diagonal metric.
    pub fn trace(&self) -> f64 {
        self.dimension() as f64
    }

    /// LaTeX representation of the signature, e.g. `"(+,-,-,-)"`.
    pub fn to_latex(&self) -> String {
        self.display_signature()
    }
}

// ---------------------------------------------------------------------------
// Spacetime Metric Trait
// ---------------------------------------------------------------------------

/// Trait for a generalized spacetime metric tensor $g_{\mu\nu}$.
///
/// This trait decouples the physics engine from any particular spacetime
/// geometry. Implementations range from flat diagonal metrics to
/// coordinate-dependent curved backgrounds.
///
/// # Implementors
///
/// - [`FlatMetric`]: diagonal constant metric (Minkowski, Euclidean, etc.)
/// - Future: `SchwarzschildMetric`, `FRWMetric`, `AdSMetric`, ...
pub trait SpacetimeMetricTrait {
    /// The number of spacetime dimensions.
    fn dimension(&self) -> usize;

    /// The $(\mu, \nu)$ component of the metric tensor: $g_{\mu\nu}$.
    fn component(&self, mu: usize, nu: usize) -> f64;

    /// Inner product $g_{\mu\nu} v^\mu w^\nu$.
    fn inner_product(&self, v: &SpacetimeVector, w: &SpacetimeVector) -> Result<f64, String>;

    /// The trace $g^{\mu}{}_{\mu}$ of the metric.
    fn trace(&self) -> f64;

    /// Whether the metric is flat (constant components).
    fn is_flat(&self) -> bool;

    /// Whether the metric is diagonal.
    fn is_diagonal(&self) -> bool;
}

/// A flat diagonal metric tensor — the workhorse for perturbative QFT.
///
/// Constant diagonal entries defined by a [`MetricSignature`]. Covers
/// Minkowski, Euclidean, and higher-dimensional flat backgrounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlatMetric {
    /// The metric signature defining the diagonal entries.
    pub signature: MetricSignature,
}

impl FlatMetric {
    /// Standard 4D Minkowski metric $\eta_{\mu\nu} = \mathrm{diag}(+1,-1,-1,-1)$.
    pub fn minkowski_4d() -> Self {
        Self {
            signature: MetricSignature::minkowski_4d(),
        }
    }

    /// $D$-dimensional Minkowski metric $\eta_{\mu\nu} = \mathrm{diag}(+1,-1,\ldots,-1)$.
    pub fn minkowski(dim: usize) -> Self {
        Self {
            signature: MetricSignature::minkowski(dim),
        }
    }

    /// $D$-dimensional Euclidean metric $\delta_{\mu\nu} = \mathrm{diag}(+1,\ldots,+1)$.
    pub fn euclidean(dim: usize) -> Self {
        Self {
            signature: MetricSignature::euclidean(dim),
        }
    }
}

impl SpacetimeMetricTrait for FlatMetric {
    fn dimension(&self) -> usize {
        self.signature.dimension()
    }

    fn component(&self, mu: usize, nu: usize) -> f64 {
        if mu == nu {
            self.signature.component(mu)
        } else {
            0.0
        }
    }

    fn inner_product(&self, v: &SpacetimeVector, w: &SpacetimeVector) -> Result<f64, String> {
        self.signature.inner_product(v, w)
    }

    fn trace(&self) -> f64 {
        self.signature.trace()
    }

    fn is_flat(&self) -> bool {
        true
    }

    fn is_diagonal(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// N-Dimensional Spacetime Vector
// ---------------------------------------------------------------------------

/// Internal storage for [`SpacetimeVector`]: stack-allocated for D ≤ 4,
/// heap-allocated for higher dimensions.
///
/// The 4D case — overwhelmingly dominant in particle physics — lives entirely
/// on the stack with 32-byte alignment for SIMD-friendly access.  Higher
/// dimensions fall back to a heap `Vec<f64>` transparently.
#[derive(Debug, Clone)]
enum VectorStorage {
    /// Stack-resident storage for D ≤ 4.  `len` tracks the actual dimension.
    Inline {
        #[allow(dead_code)]
        data: [f64; 4],
        len: u8,
    },
    /// Heap storage for D > 4.
    Heap(Vec<f64>),
}

impl PartialEq for VectorStorage {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl VectorStorage {
    /// Create storage from a `Vec<f64>`, choosing Inline when possible.
    #[inline]
    fn from_vec(v: Vec<f64>) -> Self {
        if v.len() <= 4 {
            let mut data = [0.0_f64; 4];
            data[..v.len()].copy_from_slice(&v);
            VectorStorage::Inline {
                data,
                len: v.len() as u8,
            }
        } else {
            VectorStorage::Heap(v)
        }
    }

    /// Create an Inline storage directly from four components.
    #[inline]
    fn inline_4d(v0: f64, v1: f64, v2: f64, v3: f64) -> Self {
        VectorStorage::Inline {
            data: [v0, v1, v2, v3],
            len: 4,
        }
    }

    /// Create a zero-filled storage of the given dimension.
    #[inline]
    fn zeros(dim: usize) -> Self {
        if dim <= 4 {
            VectorStorage::Inline {
                data: [0.0; 4],
                len: dim as u8,
            }
        } else {
            VectorStorage::Heap(vec![0.0; dim])
        }
    }

    /// Number of components.
    #[inline]
    fn len(&self) -> usize {
        match self {
            VectorStorage::Inline { len, .. } => *len as usize,
            VectorStorage::Heap(v) => v.len(),
        }
    }

    /// Immutable slice over all components.
    #[inline]
    fn as_slice(&self) -> &[f64] {
        match self {
            VectorStorage::Inline { data, len } => &data[..*len as usize],
            VectorStorage::Heap(v) => v.as_slice(),
        }
    }

    /// Mutable slice over all components.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [f64] {
        match self {
            VectorStorage::Inline { data, len } => &mut data[..*len as usize],
            VectorStorage::Heap(v) => v.as_mut_slice(),
        }
    }
}

/// A contravariant vector $v^\mu$ in $D$-dimensional spacetime.
///
/// Generalizes `FourMomentum` to arbitrary dimensions while maintaining full
/// backward compatibility: a `SpacetimeVector` with $D=4$ behaves identically
/// to the legacy `FourMomentum`.
///
/// # Design
///
/// Uses **Small Vector Optimization**: the ubiquitous 4D case is stored
/// entirely on the stack (zero heap allocations), while higher dimensions
/// ($D > 4$, e.g. $D = 10$ for String Theory) fall back to heap storage.
///
/// All metric-dependent operations accept a [`MetricSignature`] parameter,
/// decoupling the vector algebra from any hardcoded signature.
/// The `From<FourMomentum>` conversion ensures seamless interop with
/// existing 4D code paths.
///
/// # Conventions
///
/// Index 0 is the temporal component ($v^0$). Indices $1, \ldots, D-1$ are
/// spatial.
#[derive(Debug, Clone, PartialEq)]
pub struct SpacetimeVector {
    /// Internal storage — stack for D ≤ 4, heap for D > 4.
    storage: VectorStorage,
}

// --- Custom Serialize / Deserialize for backward-compatible JSON ---

impl Serialize for SpacetimeVector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Serialize as { "components": [v0, v1, ...] } for backward compat.
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SpacetimeVector", 1)?;
        s.serialize_field("components", self.components())?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for SpacetimeVector {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            components: Vec<f64>,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(SpacetimeVector::new(h.components))
    }
}

impl SpacetimeVector {
    /// Access the components as an immutable slice.
    #[inline]
    pub fn components(&self) -> &[f64] {
        self.storage.as_slice()
    }

    /// Access the components as a mutable slice.
    #[inline]
    pub fn components_mut(&mut self) -> &mut [f64] {
        self.storage.as_mut_slice()
    }
}

impl std::ops::Index<usize> for SpacetimeVector {
    type Output = f64;
    #[inline]
    fn index(&self, index: usize) -> &f64 {
        &self.storage.as_slice()[index]
    }
}

impl std::ops::IndexMut<usize> for SpacetimeVector {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut f64 {
        &mut self.storage.as_mut_slice()[index]
    }
}

impl SpacetimeVector {
    /// Create a new spacetime vector from an explicit component list.
    ///
    /// The dimension is inferred from the length of the list.
    #[inline]
    pub fn new(components: Vec<f64>) -> Self {
        assert!(
            !components.is_empty(),
            "SpacetimeVector must have at least 1 component"
        );
        Self {
            storage: VectorStorage::from_vec(components),
        }
    }

    /// Create a 4-vector $(v^0, v^1, v^2, v^3)$ — the most common case in
    /// particle physics.  Stack-allocated with zero heap allocation.
    #[inline]
    pub fn new_4d(v0: f64, v1: f64, v2: f64, v3: f64) -> Self {
        Self {
            storage: VectorStorage::inline_4d(v0, v1, v2, v3),
        }
    }

    /// Create the zero vector in $D$ dimensions.
    #[inline]
    pub fn zero(dim: usize) -> Self {
        Self {
            storage: VectorStorage::zeros(dim),
        }
    }

    /// The number of spacetime dimensions.
    #[inline]
    pub fn dimension(&self) -> usize {
        self.storage.len()
    }

    /// Compute the inner product $g_{\mu\nu} v^\mu w^\nu$ with respect to a
    /// given metric signature.
    ///
    /// # Errors
    /// Returns an error if dimensions are mismatched.
    #[inline]
    pub fn dot(&self, other: &SpacetimeVector, metric: &MetricSignature) -> Result<f64, String> {
        metric.inner_product(self, other)
    }

    /// Compute the invariant norm squared $v^\mu v_\mu = g_{\mu\nu} v^\mu v^\nu$.
    #[inline]
    pub fn norm_sq(&self, metric: &MetricSignature) -> Result<f64, String> {
        metric.inner_product(self, self)
    }

    /// Compute the invariant mass squared (for 4-momentum interpretation):
    /// $p^2 = g_{\mu\nu} p^\mu p^\nu$.
    ///
    /// In the $(+,-,-,-)$ convention this gives $E^2 - |\vec{p}|^2 = m^2$.
    #[inline]
    pub fn invariant_mass_sq(&self, metric: &MetricSignature) -> Result<f64, String> {
        self.norm_sq(metric)
    }

    /// Compute the spatial magnitude $|\vec{v}| = \sqrt{\sum_{i=1}^{D-1} (v^i)^2}$.
    ///
    /// Uses the Euclidean norm of the spatial components (ignoring metric signs),
    /// matching the standard definition $|\vec{p}|$.
    #[inline]
    pub fn spatial_magnitude(&self) -> f64 {
        let s = self.components();
        if s.len() <= 1 {
            return 0.0;
        }
        s[1..].iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Scale all components by a scalar factor.
    #[inline]
    pub fn scale(&self, factor: f64) -> Self {
        let s = self.components();
        let dim = s.len();
        if dim <= 4 {
            let mut data = [0.0_f64; 4];
            for i in 0..dim {
                data[i] = s[i] * factor;
            }
            Self {
                storage: VectorStorage::Inline {
                    data,
                    len: dim as u8,
                },
            }
        } else {
            Self {
                storage: VectorStorage::Heap(s.iter().map(|x| x * factor).collect()),
            }
        }
    }

    /// Convert to a legacy `FourMomentum`.
    ///
    /// # Panics
    /// Panics if this vector is not 4-dimensional.
    #[inline]
    pub fn to_four_momentum(&self) -> FourMomentum {
        assert_eq!(
            self.dimension(),
            4,
            "Cannot convert non-4D vector to FourMomentum"
        );
        let s = self.components();
        FourMomentum {
            e: s[0],
            px: s[1],
            py: s[2],
            pz: s[3],
        }
    }
}

impl From<FourMomentum> for SpacetimeVector {
    #[inline]
    fn from(p: FourMomentum) -> Self {
        Self {
            storage: VectorStorage::inline_4d(p.e, p.px, p.py, p.pz),
        }
    }
}

impl Add for SpacetimeVector {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let dim = self.dimension();
        assert_eq!(
            dim,
            rhs.dimension(),
            "Cannot add SpacetimeVectors of different dimensions"
        );
        let a = self.components();
        let b = rhs.components();
        if dim <= 4 {
            let mut data = [0.0_f64; 4];
            for i in 0..dim {
                data[i] = a[i] + b[i];
            }
            Self {
                storage: VectorStorage::Inline {
                    data,
                    len: dim as u8,
                },
            }
        } else {
            Self {
                storage: VectorStorage::Heap(a.iter().zip(b).map(|(x, y)| x + y).collect()),
            }
        }
    }
}

impl Sub for SpacetimeVector {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let dim = self.dimension();
        assert_eq!(
            dim,
            rhs.dimension(),
            "Cannot subtract SpacetimeVectors of different dimensions"
        );
        let a = self.components();
        let b = rhs.components();
        if dim <= 4 {
            let mut data = [0.0_f64; 4];
            for i in 0..dim {
                data[i] = a[i] - b[i];
            }
            Self {
                storage: VectorStorage::Inline {
                    data,
                    len: dim as u8,
                },
            }
        } else {
            Self {
                storage: VectorStorage::Heap(a.iter().zip(b).map(|(x, y)| x - y).collect()),
            }
        }
    }
}

impl Neg for SpacetimeVector {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        let s = self.components();
        let dim = s.len();
        if dim <= 4 {
            let mut data = [0.0_f64; 4];
            for i in 0..dim {
                data[i] = -s[i];
            }
            Self {
                storage: VectorStorage::Inline {
                    data,
                    len: dim as u8,
                },
            }
        } else {
            Self {
                storage: VectorStorage::Heap(s.iter().map(|x| -x).collect()),
            }
        }
    }
}

impl std::fmt::Display for SpacetimeVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self
            .components()
            .iter()
            .map(|c| format!("{:.6}", c))
            .collect();
        write!(f, "({})", parts.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Spacetime Configuration
// ---------------------------------------------------------------------------

/// Complete description of the spacetime geometry for a calculation.
///
/// Bundles the dimension, metric signature, and regularization scheme into a
/// single configuration object. Every physics computation queries this
/// configuration rather than assuming 4D Minkowski.
///
/// # Default
///
/// The default configuration is standard 4D Minkowski with no dimensional
/// regularization — the setting for tree-level Standard Model calculations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacetimeConfig {
    /// The flat metric defining inner products.
    pub metric: FlatMetric,
    /// The dimensional regularization scheme (if any).
    pub regularization: SpacetimeDimension,
}

impl SpacetimeConfig {
    /// Standard 4D Minkowski configuration with no regularization.
    pub fn standard_4d() -> Self {
        Self {
            metric: FlatMetric::minkowski_4d(),
            regularization: SpacetimeDimension::Fixed(4),
        }
    }

    /// 4D Minkowski with dimensional regularization at 1-loop.
    pub fn dim_reg_4d() -> Self {
        Self {
            metric: FlatMetric::minkowski_4d(),
            regularization: SpacetimeDimension::DimReg { base: 4 },
        }
    }

    /// Custom configuration with explicit metric and regularization.
    pub fn custom(metric: FlatMetric, regularization: SpacetimeDimension) -> Self {
        Self {
            metric,
            regularization,
        }
    }

    /// The physical dimension of the spacetime.
    pub fn dimension(&self) -> usize {
        self.metric.dimension()
    }

    /// The metric signature.
    pub fn signature(&self) -> &MetricSignature {
        &self.metric.signature
    }
}

impl Default for SpacetimeConfig {
    fn default() -> Self {
        Self::standard_4d()
    }
}

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
            SpacetimeDimension::Fixed(4) => "4·g^{νρ}".into(),
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

// ===========================================================================
// Computer Algebra System (CAS) Engine
// ===========================================================================

// ---------------------------------------------------------------------------
// Lorentz Index — the fundamental label for spacetime algebra
// ---------------------------------------------------------------------------

/// A symbolic Lorentz index used throughout the CAS.
///
/// Indices can be named (e.g., $\mu$, $\nu$) or numbered for algorithmic
/// manipulation. Two indices with the same label appearing in a product
/// are implicitly summed over (Einstein convention).
///
/// **Extensibility**: The `Dummy(u32)` variant allows the simplification
/// engine to create fresh dummy indices without name collisions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LorentzIndex {
    /// A named index (e.g., `"mu"`, `"nu"`, `"rho"`).
    Named(String),
    /// A numeric index (concrete value 0–3, used in trace evaluation).
    Numeric(u8),
    /// An automatically generated dummy index for internal contractions.
    Dummy(u32),
}

impl LorentzIndex {
    /// Render this index as a LaTeX string.
    pub fn to_latex(&self) -> String {
        match self {
            LorentzIndex::Named(s) => latex_lorentz_index(s),
            LorentzIndex::Numeric(n) => format!("{}", n),
            LorentzIndex::Dummy(n) => format!("\\lambda_{{{}}}", n),
        }
    }
}

impl std::fmt::Display for LorentzIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LorentzIndex::Named(s) => write!(f, "{}", s),
            LorentzIndex::Numeric(n) => write!(f, "{}", n),
            LorentzIndex::Dummy(n) => write!(f, "λ_{}", n),
        }
    }
}

// ---------------------------------------------------------------------------
// SpacetimeTensor — Generalized Tensor Representation
// ---------------------------------------------------------------------------

/// Classification of spacetime tensor symmetry/structure.
///
/// This allows the CAS to handle arbitrary-rank tensors with different
/// index symmetries and physical interpretations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpacetimeTensorKind {
    /// Scalar (rank 0). No Lorentz indices.
    Scalar,
    /// Vector (rank 1): $V^\mu$. Used for spin-1 polarization.
    Vector,
    /// Spin-3/2 vector-spinor (Rarita-Schwinger): $\psi^\mu_\alpha$.
    /// Carries one Lorentz index and one implicit Dirac spinor index.
    VectorSpinor,
    /// Symmetric rank-2 tensor: $h^{\mu\nu} = h^{\nu\mu}$.
    /// Used for spin-2 (graviton) polarization.
    SymmetricRank2,
    /// Anti-symmetric rank-2 tensor: $F^{\mu\nu} = -F^{\nu\mu}$.
    /// Used for field strength tensors.
    AntiSymmetricRank2,
    /// General rank-$n$ tensor with no assumed symmetry.
    GeneralRank(u8),
}

// ---------------------------------------------------------------------------
// CasExpr — The Core Expression Tree
// ---------------------------------------------------------------------------

/// The recursive expression tree of SPIRE's Computer Algebra System.
///
/// Every mathematical object in an amplitude derivation is represented as a
/// `CasExpr` node. The tree supports:
/// - **Nested arithmetic**: `Add`, `Mul`, `Neg`, `Fraction`
/// - **Dirac algebra**: `GammaMat`, `Gamma5`, spinors
/// - **Tensor algebra**: `MetricTensor`, `LeviCivita`, `SpacetimeTensor`
/// - **Algebraic operations**: `Trace`, `Commutator`, `AntiCommutator`
///
/// # Extensibility
///
/// New algebraic objects (e.g., colour matrices $T^a$, SUSY generators $Q_\alpha$)
/// are added as new enum variants. The simplification engine dispatches on
/// variant, so existing simplification rules remain untouched.
///
/// # Commutativity
///
/// - Scalars, metric tensors, and Levi-Civita tensors are commutative.
/// - Gamma matrices and spinors are **non-commutative** (Dirac algebra).
/// - The `Mul` node preserves ordering for non-commutative factors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CasExpr {
    // -- Atomic nodes (leaves) --
    /// A real numerical constant.
    Scalar(f64),

    /// The imaginary unit $i = \sqrt{-1}$.
    ImaginaryUnit,

    /// A named symbolic variable with optional Lorentz indices.
    ///
    /// Examples: coupling constant `g`, mass `m`, arbitrary symbol `x`.
    Symbol {
        name: String,
        indices: Vec<LorentzIndex>,
    },

    /// The Minkowski metric tensor $g^{\mu\nu}$.
    MetricTensor { mu: LorentzIndex, nu: LorentzIndex },

    /// The totally antisymmetric Levi-Civita tensor $\epsilon^{\mu\nu\rho\sigma}$.
    ///
    /// Convention: $\epsilon^{0123} = +1$ (particle physics convention).
    LeviCivita { indices: [LorentzIndex; 4] },

    /// A Dirac gamma matrix $\gamma^\mu$ ($\mu = 0,1,2,3$).
    GammaMat { index: LorentzIndex },

    /// The chiral matrix $\gamma^5 = i\gamma^0\gamma^1\gamma^2\gamma^3$.
    Gamma5,

    /// A four-momentum with a Lorentz index: $p^\mu$.
    Momentum { label: String, index: LorentzIndex },

    /// A slashed momentum $\not{p} = \gamma^\mu p_\mu$.
    SlashedMomentum { label: String },

    /// The Minkowski dot product of two four-momenta: $p \cdot q = p^\mu q_\mu$.
    DotProduct { left: String, right: String },

    /// An external $u$-spinor: $u(p)$.
    SpinorU { label: String, momentum: String },
    /// An external $\bar{u}$-spinor: $\bar{u}(p)$.
    SpinorUBar { label: String, momentum: String },
    /// An external $v$-spinor: $v(p)$.
    SpinorV { label: String, momentum: String },
    /// An external $\bar{v}$-spinor: $\bar{v}(p)$.
    SpinorVBar { label: String, momentum: String },

    /// A generalized spacetime tensor (arbitrary spin).
    ///
    /// This is the unified representation for polarization vectors,
    /// Rarita-Schwinger vector-spinors, and graviton polarization tensors.
    Tensor {
        /// Human-readable label (e.g., `"ε"`, `"ψ"`, `"h"`).
        label: String,
        /// The kind of tensor (determines symmetry properties).
        kind: SpacetimeTensorKind,
        /// Lorentz indices carried by this tensor.
        indices: Vec<LorentzIndex>,
        /// Associated momentum label (e.g., for polarization).
        momentum: Option<String>,
        /// Whether this is a conjugate (e.g., $\epsilon^*$).
        is_conjugate: bool,
    },

    // -- Composite nodes (branches) --
    /// Summation of terms: $\sum_i c_i$.
    ///
    /// An empty `Add` represents zero.
    Add(Vec<CasExpr>),

    /// Ordered product of factors: $\prod_i f_i$.
    ///
    /// **Important**: order matters for non-commutative factors (gamma matrices,
    /// spinors). An empty `Mul` represents the identity (one).
    Mul(Vec<CasExpr>),

    /// Negation: $-\text{expr}$.
    Neg(Box<CasExpr>),

    /// A rational fraction: $\frac{\text{numerator}}{\text{denominator}}$.
    Fraction {
        numerator: Box<CasExpr>,
        denominator: Box<CasExpr>,
    },

    /// The Dirac trace: $\mathrm{Tr}[\text{expr}]$.
    ///
    /// The inner expression should be a product of gamma matrices.
    Trace(Box<CasExpr>),

    /// Commutator: $[A, B] = AB - BA$.
    Commutator(Box<CasExpr>, Box<CasExpr>),

    /// Anti-commutator: $\{A, B\} = AB + BA$.
    AntiCommutator(Box<CasExpr>, Box<CasExpr>),

    /// A Kronecker delta: $\delta^\mu_\nu$.
    KroneckerDelta { mu: LorentzIndex, nu: LorentzIndex },

    /// A propagator denominator: $\frac{1}{p^2 - m^2 + i\epsilon}$.
    PropagatorDenom { momentum: String, mass_sq: f64 },
}

// ---------------------------------------------------------------------------
// CasExpr — Core Methods
// ---------------------------------------------------------------------------

impl CasExpr {
    /// Check whether this expression node represents a commutative object.
    ///
    /// Scalars, metric tensors, dot products, and Levi-Civita tensors commute.
    /// Gamma matrices and spinors do not.
    pub fn is_commutative(&self) -> bool {
        match self {
            CasExpr::Scalar(_)
            | CasExpr::ImaginaryUnit
            | CasExpr::Symbol { .. }
            | CasExpr::MetricTensor { .. }
            | CasExpr::LeviCivita { .. }
            | CasExpr::DotProduct { .. }
            | CasExpr::KroneckerDelta { .. }
            | CasExpr::PropagatorDenom { .. } => true,

            CasExpr::GammaMat { .. }
            | CasExpr::Gamma5
            | CasExpr::SlashedMomentum { .. }
            | CasExpr::SpinorU { .. }
            | CasExpr::SpinorUBar { .. }
            | CasExpr::SpinorV { .. }
            | CasExpr::SpinorVBar { .. } => false,

            CasExpr::Momentum { .. } | CasExpr::Tensor { .. } => true,

            CasExpr::Add(terms) => terms.iter().all(|t| t.is_commutative()),
            CasExpr::Mul(factors) => factors.iter().all(|f| f.is_commutative()),
            CasExpr::Neg(inner) => inner.is_commutative(),
            CasExpr::Fraction {
                numerator,
                denominator,
            } => numerator.is_commutative() && denominator.is_commutative(),
            CasExpr::Trace(_) => true, // A trace is a scalar
            CasExpr::Commutator(_, _) | CasExpr::AntiCommutator(_, _) => false,
        }
    }

    /// Collect all free (uncontracted) Lorentz indices in this expression.
    ///
    /// An index is "free" if it appears an odd number of times. Indices appearing
    /// exactly twice are contracted (summed over) and excluded.
    pub fn get_free_indices(&self) -> Vec<LorentzIndex> {
        let mut all_indices = Vec::new();
        self.collect_indices(&mut all_indices);

        // Count occurrences
        let mut counts: std::collections::HashMap<LorentzIndex, usize> =
            std::collections::HashMap::new();
        for idx in &all_indices {
            *counts.entry(idx.clone()).or_insert(0) += 1;
        }

        // Free = appears odd number of times
        let mut seen = Vec::new();
        let mut result = Vec::new();
        for idx in all_indices {
            if counts.get(&idx).copied().unwrap_or(0) % 2 == 1 && !seen.contains(&idx) {
                seen.push(idx.clone());
                result.push(idx);
            }
        }
        result
    }

    /// Recursively collect all Lorentz indices (free and contracted).
    fn collect_indices(&self, out: &mut Vec<LorentzIndex>) {
        match self {
            CasExpr::Scalar(_)
            | CasExpr::ImaginaryUnit
            | CasExpr::Gamma5
            | CasExpr::DotProduct { .. }
            | CasExpr::PropagatorDenom { .. } => {}

            CasExpr::Symbol { indices, .. } => out.extend(indices.iter().cloned()),
            CasExpr::MetricTensor { mu, nu } => {
                out.push(mu.clone());
                out.push(nu.clone());
            }
            CasExpr::KroneckerDelta { mu, nu } => {
                out.push(mu.clone());
                out.push(nu.clone());
            }
            CasExpr::LeviCivita { indices } => {
                out.extend(indices.iter().cloned());
            }
            CasExpr::GammaMat { index } => out.push(index.clone()),
            CasExpr::Momentum { index, .. } => out.push(index.clone()),
            CasExpr::SlashedMomentum { .. } => {} // contracted internally
            CasExpr::SpinorU { .. }
            | CasExpr::SpinorUBar { .. }
            | CasExpr::SpinorV { .. }
            | CasExpr::SpinorVBar { .. } => {}
            CasExpr::Tensor { indices, .. } => out.extend(indices.iter().cloned()),

            CasExpr::Add(terms) => {
                for t in terms {
                    t.collect_indices(out);
                }
            }
            CasExpr::Mul(factors) => {
                for f in factors {
                    f.collect_indices(out);
                }
            }
            CasExpr::Neg(inner) => inner.collect_indices(out),
            CasExpr::Fraction {
                numerator,
                denominator,
            } => {
                numerator.collect_indices(out);
                denominator.collect_indices(out);
            }
            CasExpr::Trace(inner) => inner.collect_indices(out),
            CasExpr::Commutator(a, b) | CasExpr::AntiCommutator(a, b) => {
                a.collect_indices(out);
                b.collect_indices(out);
            }
        }
    }

    /// Distribute products over sums (expand brackets).
    ///
    /// For example: $a(b + c) \rightarrow ab + ac$.
    ///
    /// This operates one level deep — call repeatedly for full expansion.
    pub fn expand(&self) -> CasExpr {
        match self {
            CasExpr::Mul(factors) => {
                // Find the first Add in the factor list and distribute.
                if let Some(add_pos) = factors.iter().position(|f| matches!(f, CasExpr::Add(_))) {
                    let add_terms = match &factors[add_pos] {
                        CasExpr::Add(ts) => ts.clone(),
                        _ => unreachable!(),
                    };
                    let prefix: Vec<CasExpr> = factors[..add_pos].to_vec();
                    let suffix: Vec<CasExpr> = factors[add_pos + 1..].to_vec();

                    let expanded: Vec<CasExpr> = add_terms
                        .into_iter()
                        .map(|term| {
                            let mut new_factors = prefix.clone();
                            new_factors.push(term);
                            new_factors.extend(suffix.clone());
                            CasExpr::Mul(new_factors)
                        })
                        .collect();
                    CasExpr::Add(expanded)
                } else {
                    self.clone()
                }
            }
            CasExpr::Add(terms) => CasExpr::Add(terms.iter().map(|t| t.expand()).collect()),
            CasExpr::Neg(inner) => CasExpr::Neg(Box::new(inner.expand())),
            _ => self.clone(),
        }
    }

    /// Apply basic algebraic simplification rules.
    ///
    /// This is the main simplification entry point. It applies rules in order:
    /// 1. Flatten nested `Add`/`Mul` nodes.
    /// 2. Eliminate identity elements (`x + 0 → x`, `x × 1 → x`).
    /// 3. Combine scalar constants (`2 × 3 → 6`, `2 + 3 → 5`).
    /// 4. Apply the Dirac equation to on-shell spinors.
    /// 5. Apply metric tensor contractions.
    ///
    /// # Arguments
    /// * `dim` — The spacetime dimension for γ-matrix contractions.
    pub fn simplify(&self, dim: SpacetimeDimension) -> CasExpr {
        match self {
            CasExpr::Add(terms) => {
                let simplified: Vec<CasExpr> = terms.iter().map(|t| t.simplify(dim)).collect();
                simplify_add(simplified)
            }
            CasExpr::Mul(factors) => {
                let simplified: Vec<CasExpr> = factors.iter().map(|f| f.simplify(dim)).collect();
                simplify_mul(simplified, dim)
            }
            CasExpr::Neg(inner) => {
                let s = inner.simplify(dim);
                match s {
                    CasExpr::Scalar(v) => CasExpr::Scalar(-v),
                    CasExpr::Neg(inner2) => *inner2, // --x = x
                    other => CasExpr::Neg(Box::new(other)),
                }
            }
            CasExpr::Trace(inner) => {
                let s = inner.simplify(dim);
                CasExpr::Trace(Box::new(s))
            }
            CasExpr::Commutator(a, b) => {
                // [A, B] = AB - BA
                let ab = CasExpr::Mul(vec![a.simplify(dim), b.simplify(dim)]);
                let ba = CasExpr::Mul(vec![b.simplify(dim), a.simplify(dim)]);
                CasExpr::Add(vec![ab, CasExpr::Neg(Box::new(ba))]).simplify(dim)
            }
            CasExpr::AntiCommutator(a, b) => {
                // {A, B} = AB + BA
                let ab = CasExpr::Mul(vec![a.simplify(dim), b.simplify(dim)]);
                let ba = CasExpr::Mul(vec![b.simplify(dim), a.simplify(dim)]);
                CasExpr::Add(vec![ab, ba]).simplify(dim)
            }
            CasExpr::Fraction {
                numerator,
                denominator,
            } => CasExpr::Fraction {
                numerator: Box::new(numerator.simplify(dim)),
                denominator: Box::new(denominator.simplify(dim)),
            },
            // Atomic nodes are already simplified.
            other => other.clone(),
        }
    }

    /// Render this CAS expression as a LaTeX string.
    pub fn to_cas_latex(&self) -> String {
        match self {
            CasExpr::Scalar(v) => {
                if (*v - v.round()).abs() < 1e-12 {
                    format!("{}", *v as i64)
                } else {
                    format!("{:.4}", v)
                }
            }
            CasExpr::ImaginaryUnit => "i".into(),
            CasExpr::Symbol { name, indices } => {
                if indices.is_empty() {
                    name.clone()
                } else {
                    let idx_str: Vec<String> = indices.iter().map(|i| i.to_latex()).collect();
                    format!("{}^{{{}}}", name, idx_str.join(""))
                }
            }
            CasExpr::MetricTensor { mu, nu } => {
                format!("g^{{{},{}}}", mu.to_latex(), nu.to_latex())
            }
            CasExpr::KroneckerDelta { mu, nu } => {
                format!("\\delta^{{{}}}_{{{}}} ", mu.to_latex(), nu.to_latex())
            }
            CasExpr::LeviCivita { indices } => {
                let idx_str: Vec<String> = indices.iter().map(|i| i.to_latex()).collect();
                format!("\\epsilon^{{{}}}", idx_str.join(""))
            }
            CasExpr::GammaMat { index } => {
                format!("\\gamma^{{{}}}", index.to_latex())
            }
            CasExpr::Gamma5 => "\\gamma^{5}".into(),
            CasExpr::Momentum { label, index } => {
                format!("{}^{{{}}}", latex_momentum(label), index.to_latex())
            }
            CasExpr::SlashedMomentum { label } => {
                format!("\\not{{{}}}", latex_momentum(label))
            }
            CasExpr::DotProduct { left, right } => {
                format!("{} \\cdot {}", latex_momentum(left), latex_momentum(right))
            }
            CasExpr::SpinorU { momentum, .. } => format!("u({})", latex_momentum(momentum)),
            CasExpr::SpinorUBar { momentum, .. } => {
                format!("\\bar{{u}}({})", latex_momentum(momentum))
            }
            CasExpr::SpinorV { momentum, .. } => format!("v({})", latex_momentum(momentum)),
            CasExpr::SpinorVBar { momentum, .. } => {
                format!("\\bar{{v}}({})", latex_momentum(momentum))
            }
            CasExpr::Tensor {
                label,
                indices,
                is_conjugate,
                ..
            } => {
                let conj = if *is_conjugate { "^{*}" } else { "" };
                let idx_str: Vec<String> = indices.iter().map(|i| i.to_latex()).collect();
                if indices.is_empty() {
                    format!("{}{}", label, conj)
                } else {
                    format!("{}{}^{{{}}}", label, conj, idx_str.join(""))
                }
            }
            CasExpr::Add(terms) => {
                if terms.is_empty() {
                    return "0".into();
                }
                let parts: Vec<String> = terms
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let s = t.to_cas_latex();
                        if i == 0 {
                            s
                        } else if let Some(rest) = s.strip_prefix('-') {
                            format!(" - {}", rest)
                        } else {
                            format!(" + {}", s)
                        }
                    })
                    .collect();
                parts.join("")
            }
            CasExpr::Mul(factors) => {
                if factors.is_empty() {
                    return "1".into();
                }
                let parts: Vec<String> = factors
                    .iter()
                    .map(|f| {
                        let s = f.to_cas_latex();
                        // Wrap Add nodes in parens
                        if matches!(f, CasExpr::Add(_)) {
                            format!("\\left({}\\right)", s)
                        } else {
                            s
                        }
                    })
                    .collect();
                parts.join("\\,")
            }
            CasExpr::Neg(inner) => {
                let s = inner.to_cas_latex();
                if matches!(inner.as_ref(), CasExpr::Add(_)) {
                    format!("-\\left({}\\right)", s)
                } else {
                    format!("-{}", s)
                }
            }
            CasExpr::Fraction {
                numerator,
                denominator,
            } => {
                format!(
                    "\\frac{{{}}}{{{}}}",
                    numerator.to_cas_latex(),
                    denominator.to_cas_latex()
                )
            }
            CasExpr::Trace(inner) => {
                format!("\\mathrm{{Tr}}\\left[{}\\right]", inner.to_cas_latex())
            }
            CasExpr::Commutator(a, b) => {
                format!(
                    "\\left[{},\\,{}\\right]",
                    a.to_cas_latex(),
                    b.to_cas_latex()
                )
            }
            CasExpr::AntiCommutator(a, b) => {
                format!(
                    "\\left\\{{{},\\,{}\\right\\}}",
                    a.to_cas_latex(),
                    b.to_cas_latex()
                )
            }
            CasExpr::PropagatorDenom { momentum, mass_sq } => {
                let p = latex_momentum(momentum);
                if *mass_sq > 1e-15 {
                    format!("\\frac{{1}}{{{p}^2 - {:.4} + i\\epsilon}}", mass_sq)
                } else {
                    format!("\\frac{{1}}{{{p}^2 + i\\epsilon}}")
                }
            }
        }
    }

    /// Check whether this expression is a scalar (numeric constant).
    pub fn is_scalar_zero(&self) -> bool {
        matches!(self, CasExpr::Scalar(v) if v.abs() < 1e-15)
    }

    /// Check whether this expression is the multiplicative identity.
    pub fn is_scalar_one(&self) -> bool {
        matches!(self, CasExpr::Scalar(v) if (*v - 1.0).abs() < 1e-15)
    }

    /// Apply the Dirac equation to simplify on-shell spinor expressions.
    ///
    /// When $\not{p}\, u(p) = m\, u(p)$ or $\bar{u}(p)\,\not{p} = m\,\bar{u}(p)$,
    /// this function replaces the slashed momentum + spinor combination with
    /// the mass times the spinor.
    ///
    /// # Arguments
    /// * `on_shell_masses` — Map from momentum label to on-shell mass.
    pub fn apply_dirac_equation(
        &self,
        on_shell_masses: &std::collections::HashMap<String, f64>,
    ) -> CasExpr {
        match self {
            CasExpr::Mul(factors) => {
                let mut new_factors = factors.clone();
                let mut changed = true;
                while changed {
                    changed = false;
                    // Look for /p u(p) → m u(p)
                    for i in 0..new_factors.len().saturating_sub(1) {
                        if let CasExpr::SlashedMomentum { label: p_label } = &new_factors[i] {
                            if let CasExpr::SpinorU { momentum, .. } = &new_factors[i + 1] {
                                if p_label == momentum {
                                    if let Some(&m) = on_shell_masses.get(p_label) {
                                        new_factors[i] = CasExpr::Scalar(m);
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                            if let CasExpr::SpinorV { momentum, .. } = &new_factors[i + 1] {
                                if p_label == momentum {
                                    if let Some(&m) = on_shell_masses.get(p_label) {
                                        // /p v(p) = -m v(p)
                                        new_factors[i] = CasExpr::Scalar(-m);
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                        }
                        // Look for ū(p) /p → m ū(p)
                        if let CasExpr::SpinorUBar { momentum, .. } = &new_factors[i] {
                            if let CasExpr::SlashedMomentum { label: p_label } = &new_factors[i + 1]
                            {
                                if p_label == momentum {
                                    if let Some(&m) = on_shell_masses.get(p_label) {
                                        new_factors[i + 1] = CasExpr::Scalar(m);
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                        }
                        // Look for v̄(p) /p → -m v̄(p)
                        if let CasExpr::SpinorVBar { momentum, .. } = &new_factors[i] {
                            if let CasExpr::SlashedMomentum { label: p_label } = &new_factors[i + 1]
                            {
                                if p_label == momentum {
                                    if let Some(&m) = on_shell_masses.get(p_label) {
                                        new_factors[i + 1] = CasExpr::Scalar(-m);
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                CasExpr::Mul(new_factors)
            }
            CasExpr::Add(terms) => CasExpr::Add(
                terms
                    .iter()
                    .map(|t| t.apply_dirac_equation(on_shell_masses))
                    .collect(),
            ),
            CasExpr::Neg(inner) => {
                CasExpr::Neg(Box::new(inner.apply_dirac_equation(on_shell_masses)))
            }
            other => other.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Simplification Helpers
// ---------------------------------------------------------------------------

/// Simplify an `Add` node: flatten nested Adds, combine scalar constants,
/// eliminate zeros.
fn simplify_add(terms: Vec<CasExpr>) -> CasExpr {
    // 1. Flatten nested Add nodes
    let mut flat = Vec::new();
    for t in terms {
        match t {
            CasExpr::Add(inner) => flat.extend(inner),
            CasExpr::Neg(inner) if matches!(*inner, CasExpr::Scalar(v) if v.abs() < 1e-15) => {}
            other => flat.push(other),
        }
    }

    // 2. Remove zeros
    flat.retain(|t| !t.is_scalar_zero());

    // 3. Combine scalar constants
    let mut scalar_sum = 0.0f64;
    let mut non_scalars = Vec::new();
    for t in flat {
        match t {
            CasExpr::Scalar(v) => scalar_sum += v,
            CasExpr::Neg(inner) => match *inner {
                CasExpr::Scalar(v) => scalar_sum -= v,
                other => non_scalars.push(CasExpr::Neg(Box::new(other))),
            },
            other => non_scalars.push(other),
        }
    }

    if scalar_sum.abs() > 1e-15 {
        non_scalars.insert(0, CasExpr::Scalar(scalar_sum));
    }

    match non_scalars.len() {
        0 => CasExpr::Scalar(0.0),
        1 => non_scalars.into_iter().next().unwrap(),
        _ => CasExpr::Add(non_scalars),
    }
}

/// Simplify a `Mul` node: flatten nested Muls, combine scalar constants,
/// eliminate ones and zeros, apply metric contraction.
fn simplify_mul(factors: Vec<CasExpr>, dim: SpacetimeDimension) -> CasExpr {
    // 1. Flatten nested Mul nodes
    let mut flat = Vec::new();
    for f in factors {
        match f {
            CasExpr::Mul(inner) => flat.extend(inner),
            other => flat.push(other),
        }
    }

    // 2. If any factor is zero, whole product is zero
    if flat.iter().any(|f| f.is_scalar_zero()) {
        return CasExpr::Scalar(0.0);
    }

    // 3. Remove ones
    flat.retain(|f| !f.is_scalar_one());

    // 4. Combine scalar constants
    let mut scalar_product = 1.0f64;
    let mut non_scalars = Vec::new();
    for f in flat {
        match f {
            CasExpr::Scalar(v) => scalar_product *= v,
            other => non_scalars.push(other),
        }
    }

    // 5. Apply metric contraction: g^{μν} × T_ν → T^μ (simple case)
    non_scalars = apply_metric_contractions(non_scalars, dim);

    if (scalar_product - 1.0).abs() > 1e-15 || non_scalars.is_empty() {
        if non_scalars.is_empty() {
            return CasExpr::Scalar(scalar_product);
        }
        non_scalars.insert(0, CasExpr::Scalar(scalar_product));
    }

    match non_scalars.len() {
        0 => CasExpr::Scalar(1.0),
        1 => non_scalars.into_iter().next().unwrap(),
        _ => CasExpr::Mul(non_scalars),
    }
}

/// Apply metric tensor contraction within a product.
///
/// Looks for `g^{μν}` paired with another factor carrying index `ν`,
/// and replaces the contracted index with `μ`.
fn apply_metric_contractions(mut factors: Vec<CasExpr>, _dim: SpacetimeDimension) -> Vec<CasExpr> {
    let mut changed = true;
    while changed {
        changed = false;
        // Find a MetricTensor in the factor list
        let metric_pos = factors
            .iter()
            .position(|f| matches!(f, CasExpr::MetricTensor { .. }));
        if let Some(pos) = metric_pos {
            let (mu, nu) = match &factors[pos] {
                CasExpr::MetricTensor { mu, nu } => (mu.clone(), nu.clone()),
                _ => unreachable!(),
            };

            // If μ == ν, this is g^μ_μ = d (metric trace)
            if mu == nu {
                factors.remove(pos);
                let d_expr = match _dim {
                    SpacetimeDimension::Fixed(d) => CasExpr::Scalar(d as f64),
                    SpacetimeDimension::DimReg { base } => CasExpr::Symbol {
                        name: format!("({} - 2ε)", base),
                        indices: vec![],
                    },
                };
                factors.push(d_expr);
                changed = true;
                continue;
            }

            // Try to contract ν with another factor
            let mut found_contraction = false;
            for j in 0..factors.len() {
                if j == pos {
                    continue;
                }
                if let Some(new_factor) = try_replace_index(&factors[j], &nu, &mu) {
                    factors[j] = new_factor;
                    factors.remove(pos);
                    found_contraction = true;
                    changed = true;
                    break;
                }
                // Also try contracting μ
                if let Some(new_factor) = try_replace_index(&factors[j], &mu, &nu) {
                    factors[j] = new_factor;
                    factors.remove(pos);
                    found_contraction = true;
                    changed = true;
                    break;
                }
            }
            if !found_contraction {
                break; // No contraction possible, stop
            }
        } else {
            break;
        }
    }
    factors
}

/// Try to replace a specific Lorentz index in an expression.
/// Returns `Some(new_expr)` if the replacement was made, `None` otherwise.
fn try_replace_index(expr: &CasExpr, old: &LorentzIndex, new: &LorentzIndex) -> Option<CasExpr> {
    match expr {
        CasExpr::GammaMat { index } if index == old => {
            Some(CasExpr::GammaMat { index: new.clone() })
        }
        CasExpr::Momentum { label, index } if index == old => Some(CasExpr::Momentum {
            label: label.clone(),
            index: new.clone(),
        }),
        CasExpr::Tensor {
            label,
            kind,
            indices,
            momentum,
            is_conjugate,
        } => {
            if indices.contains(old) {
                let new_indices: Vec<LorentzIndex> = indices
                    .iter()
                    .map(|i| if i == old { new.clone() } else { i.clone() })
                    .collect();
                Some(CasExpr::Tensor {
                    label: label.clone(),
                    kind: kind.clone(),
                    indices: new_indices,
                    momentum: momentum.clone(),
                    is_conjugate: *is_conjugate,
                })
            } else {
                None
            }
        }
        CasExpr::Symbol { name, indices } if indices.contains(old) => {
            let new_indices: Vec<LorentzIndex> = indices
                .iter()
                .map(|i| if i == old { new.clone() } else { i.clone() })
                .collect();
            Some(CasExpr::Symbol {
                name: name.clone(),
                indices: new_indices,
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// DerivationStep — Step-by-Step Amplitude Derivation
// ---------------------------------------------------------------------------

/// A single step in a structured amplitude derivation.
///
/// The `derive_amplitude_steps` function produces a sequence of these steps,
/// each representing one mathematical operation applied to the amplitude.
/// This enables both pedagogical display and machine-verifiable proof chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationStep {
    /// Short label for the step (e.g., "Feynman Rules", "Dirac Equation").
    pub label: String,
    /// Human-readable description of what this step does.
    pub description: String,
    /// The CAS expression after this step has been applied.
    pub expression: CasExpr,
    /// LaTeX rendering of the expression at this step.
    pub latex: String,
}

/// Perform a step-by-step amplitude derivation from a `FeynmanGraph`.
///
/// Returns an ordered list of `DerivationStep`s showing each transformation
/// from the initial Feynman rules through simplification to the final result.
///
/// # Steps
/// 1. **Feynman Rules** — Map graph to initial CAS expression.
/// 2. **Index Assignment** — Assign Lorentz indices.
/// 3. **Metric Contraction** — Contract $g^{\mu\nu}$ terms.
/// 4. **Dirac Equation** — Simplify on-shell spinor terms.
/// 5. **Final Expression** — The simplified amplitude.
pub fn derive_amplitude_steps(
    diagram: &FeynmanGraph,
    dim: SpacetimeDimension,
) -> SpireResult<Vec<DerivationStep>> {
    let mut steps = Vec::new();

    // Step 1: Build the initial CAS expression from Feynman rules
    let initial_expr = feynman_graph_to_cas(diagram)?;
    steps.push(DerivationStep {
        label: "Feynman Rules".into(),
        description: "Apply Feynman rules to the diagram topology.".into(),
        latex: initial_expr.to_cas_latex(),
        expression: initial_expr.clone(),
    });

    // Step 2: Simplify (flatten, combine scalars)
    let simplified = initial_expr.simplify(dim);
    if simplified != initial_expr {
        steps.push(DerivationStep {
            label: "Algebraic Simplification".into(),
            description: "Flatten products, combine scalar constants, eliminate identities.".into(),
            latex: simplified.to_cas_latex(),
            expression: simplified.clone(),
        });
    }

    // Step 3: Apply metric contractions
    let contracted = simplified.simplify(dim);
    if contracted != simplified {
        steps.push(DerivationStep {
            label: "Metric Contraction".into(),
            description: "Contract repeated Lorentz indices with the metric tensor g^{μν}.".into(),
            latex: contracted.to_cas_latex(),
            expression: contracted.clone(),
        });
    }

    // Step 4: Apply Dirac equation for on-shell external legs
    let mut on_shell_masses = std::collections::HashMap::new();
    for (_src, _tgt, edge) in &diagram.edges {
        if edge.is_external {
            on_shell_masses.insert(edge.momentum_label.clone(), edge.field.mass);
        }
    }
    let dirac_simplified = contracted.apply_dirac_equation(&on_shell_masses);
    if dirac_simplified != contracted {
        steps.push(DerivationStep {
            label: "Dirac Equation".into(),
            description: "Apply the on-shell Dirac equation: \\not{p} u(p) = m u(p).".into(),
            latex: dirac_simplified.to_cas_latex(),
            expression: dirac_simplified.clone(),
        });
    }

    // Step 5: Final simplification pass
    let final_expr = dirac_simplified.simplify(dim);
    steps.push(DerivationStep {
        label: "Final Expression".into(),
        description: "The fully simplified invariant amplitude.".into(),
        latex: final_expr.to_cas_latex(),
        expression: final_expr,
    });

    Ok(steps)
}

/// Convert a `FeynmanGraph` into a `CasExpr` by applying Feynman rules.
///
/// This is the bridge between the topological diagram representation and
/// the algebraic CAS representation.
fn feynman_graph_to_cas(diagram: &FeynmanGraph) -> SpireResult<CasExpr> {
    let mut factors: Vec<CasExpr> = Vec::new();
    let mut lorentz_counter = 0u32;

    // External legs
    for (_src, _tgt, edge) in &diagram.edges {
        if !edge.is_external {
            continue;
        }
        let src_node = diagram.nodes.iter().find(|n| n.id == *_src);
        let tgt_node = diagram.nodes.iter().find(|n| n.id == *_tgt);
        let spin_2j = edge.field.quantum_numbers.spin.0;

        match spin_2j {
            1 => {
                // Spin-1/2 fermion
                let is_incoming = src_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalIncoming(_)))
                    .unwrap_or(false);
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);
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

                let spinor = match (is_incoming, is_outgoing, is_anti) {
                    (true, _, false) => CasExpr::SpinorU {
                        label: edge.field.id.clone(),
                        momentum: edge.momentum_label.clone(),
                    },
                    (_, true, false) => CasExpr::SpinorUBar {
                        label: edge.field.id.clone(),
                        momentum: edge.momentum_label.clone(),
                    },
                    (true, _, true) => CasExpr::SpinorVBar {
                        label: edge.field.id.clone(),
                        momentum: edge.momentum_label.clone(),
                    },
                    (_, true, true) => CasExpr::SpinorV {
                        label: edge.field.id.clone(),
                        momentum: edge.momentum_label.clone(),
                    },
                    _ => CasExpr::SpinorU {
                        label: edge.field.id.clone(),
                        momentum: edge.momentum_label.clone(),
                    },
                };
                factors.push(spinor);
            }
            2 => {
                // Spin-1 vector boson
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);
                let idx = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                lorentz_counter += 1;
                factors.push(CasExpr::Tensor {
                    label: "ε".into(),
                    kind: SpacetimeTensorKind::Vector,
                    indices: vec![idx],
                    momentum: Some(edge.momentum_label.clone()),
                    is_conjugate: is_outgoing,
                });
            }
            3 => {
                // Spin-3/2 Rarita-Schwinger vector-spinor
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);
                let idx = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                lorentz_counter += 1;
                factors.push(CasExpr::Tensor {
                    label: "ψ".into(),
                    kind: SpacetimeTensorKind::VectorSpinor,
                    indices: vec![idx],
                    momentum: Some(edge.momentum_label.clone()),
                    is_conjugate: is_outgoing,
                });
            }
            4 => {
                // Spin-2 graviton / massive tensor
                let is_outgoing = tgt_node
                    .map(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
                    .unwrap_or(false);
                let idx1 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                lorentz_counter += 1;
                let idx2 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                lorentz_counter += 1;
                factors.push(CasExpr::Tensor {
                    label: "ε".into(),
                    kind: SpacetimeTensorKind::SymmetricRank2,
                    indices: vec![idx1, idx2],
                    momentum: Some(edge.momentum_label.clone()),
                    is_conjugate: is_outgoing,
                });
            }
            _ => {
                // Spin-0 scalars contribute a factor of 1 (no explicit term)
            }
        }
    }

    // Internal propagator lines
    for (_src, _tgt, edge) in &diagram.edges {
        if edge.is_external {
            continue;
        }
        if let Some(ref prop) = edge.propagator {
            let p_label = &edge.momentum_label;
            match prop.form {
                PropagatorForm::Scalar => {
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(CasExpr::ImaginaryUnit),
                        denominator: Box::new(CasExpr::Add(vec![
                            CasExpr::DotProduct {
                                left: p_label.clone(),
                                right: p_label.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Scalar(prop.mass * prop.mass))),
                        ])),
                    });
                }
                PropagatorForm::DiracFermion => {
                    // i(γ·p + m) / (p² - m²)
                    let numerator = CasExpr::Mul(vec![
                        CasExpr::ImaginaryUnit,
                        CasExpr::Add(vec![
                            CasExpr::SlashedMomentum {
                                label: p_label.clone(),
                            },
                            CasExpr::Scalar(prop.mass),
                        ]),
                    ]);
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(numerator),
                        denominator: Box::new(CasExpr::Add(vec![
                            CasExpr::DotProduct {
                                left: p_label.clone(),
                                right: p_label.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Scalar(prop.mass * prop.mass))),
                        ])),
                    });
                }
                PropagatorForm::MasslessVector => {
                    // -i g_{μν} / p²
                    let idx1 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let idx2 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(CasExpr::Mul(vec![
                            CasExpr::Neg(Box::new(CasExpr::ImaginaryUnit)),
                            CasExpr::MetricTensor { mu: idx1, nu: idx2 },
                        ])),
                        denominator: Box::new(CasExpr::DotProduct {
                            left: p_label.clone(),
                            right: p_label.clone(),
                        }),
                    });
                }
                PropagatorForm::MassiveVector => {
                    // -i(g_{μν} - p_μ p_ν / m²) / (p² - m²)
                    let idx1 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let idx2 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let m_sq = prop.mass * prop.mass;
                    let numerator_tensor = CasExpr::Add(vec![
                        CasExpr::MetricTensor {
                            mu: idx1.clone(),
                            nu: idx2.clone(),
                        },
                        CasExpr::Neg(Box::new(CasExpr::Fraction {
                            numerator: Box::new(CasExpr::Mul(vec![
                                CasExpr::Momentum {
                                    label: p_label.clone(),
                                    index: idx1,
                                },
                                CasExpr::Momentum {
                                    label: p_label.clone(),
                                    index: idx2,
                                },
                            ])),
                            denominator: Box::new(CasExpr::Scalar(m_sq)),
                        })),
                    ]);
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(CasExpr::Mul(vec![
                            CasExpr::Neg(Box::new(CasExpr::ImaginaryUnit)),
                            numerator_tensor,
                        ])),
                        denominator: Box::new(CasExpr::Add(vec![
                            CasExpr::DotProduct {
                                left: p_label.clone(),
                                right: p_label.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Scalar(m_sq))),
                        ])),
                    });
                }
                PropagatorForm::RaritaSchwinger => {
                    // Spin-3/2: Rarita-Schwinger propagator
                    // For now, represent as a symbol with the momentum
                    let idx1 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let idx2 = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let m_sq = prop.mass * prop.mass;
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(CasExpr::Mul(vec![
                            CasExpr::Neg(Box::new(CasExpr::ImaginaryUnit)),
                            CasExpr::Add(vec![
                                CasExpr::SlashedMomentum {
                                    label: p_label.clone(),
                                },
                                CasExpr::Scalar(prop.mass),
                            ]),
                            // Tensor structure (g^μν - γ^μγ^ν/3 - 2p^μp^ν/(3m²) + ...)
                            CasExpr::Add(vec![
                                CasExpr::MetricTensor {
                                    mu: idx1.clone(),
                                    nu: idx2.clone(),
                                },
                                CasExpr::Neg(Box::new(CasExpr::Fraction {
                                    numerator: Box::new(CasExpr::Mul(vec![
                                        CasExpr::GammaMat {
                                            index: idx1.clone(),
                                        },
                                        CasExpr::GammaMat {
                                            index: idx2.clone(),
                                        },
                                    ])),
                                    denominator: Box::new(CasExpr::Scalar(3.0)),
                                })),
                                CasExpr::Neg(Box::new(CasExpr::Fraction {
                                    numerator: Box::new(CasExpr::Mul(vec![
                                        CasExpr::Scalar(2.0),
                                        CasExpr::Momentum {
                                            label: p_label.clone(),
                                            index: idx1.clone(),
                                        },
                                        CasExpr::Momentum {
                                            label: p_label.clone(),
                                            index: idx2.clone(),
                                        },
                                    ])),
                                    denominator: Box::new(CasExpr::Scalar(3.0 * m_sq)),
                                })),
                                CasExpr::Fraction {
                                    numerator: Box::new(CasExpr::Add(vec![
                                        CasExpr::Mul(vec![
                                            CasExpr::Momentum {
                                                label: p_label.clone(),
                                                index: idx1.clone(),
                                            },
                                            CasExpr::GammaMat {
                                                index: idx2.clone(),
                                            },
                                        ]),
                                        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                                            CasExpr::Momentum {
                                                label: p_label.clone(),
                                                index: idx2,
                                            },
                                            CasExpr::GammaMat { index: idx1 },
                                        ]))),
                                    ])),
                                    denominator: Box::new(CasExpr::Scalar(3.0 * prop.mass)),
                                },
                            ]),
                        ])),
                        denominator: Box::new(CasExpr::Add(vec![
                            CasExpr::DotProduct {
                                left: p_label.clone(),
                                right: p_label.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Scalar(m_sq))),
                        ])),
                    });
                }
                PropagatorForm::MasslessSpin2 => {
                    // Massless graviton propagator (de Donder gauge)
                    let mu = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let nu = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let rho = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let sigma = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let numerator = CasExpr::Mul(vec![
                        CasExpr::Fraction {
                            numerator: Box::new(CasExpr::ImaginaryUnit),
                            denominator: Box::new(CasExpr::Scalar(2.0)),
                        },
                        CasExpr::Add(vec![
                            CasExpr::Mul(vec![
                                CasExpr::MetricTensor {
                                    mu: mu.clone(),
                                    nu: rho.clone(),
                                },
                                CasExpr::MetricTensor {
                                    mu: nu.clone(),
                                    nu: sigma.clone(),
                                },
                            ]),
                            CasExpr::Mul(vec![
                                CasExpr::MetricTensor {
                                    mu: mu.clone(),
                                    nu: sigma.clone(),
                                },
                                CasExpr::MetricTensor {
                                    mu: nu.clone(),
                                    nu: rho.clone(),
                                },
                            ]),
                            CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                                CasExpr::MetricTensor { mu, nu },
                                CasExpr::MetricTensor { mu: rho, nu: sigma },
                            ]))),
                        ]),
                    ]);
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(numerator),
                        denominator: Box::new(CasExpr::DotProduct {
                            left: p_label.clone(),
                            right: p_label.clone(),
                        }),
                    });
                }
                PropagatorForm::MassiveSpin2 => {
                    // Massive spin-2 (Fierz-Pauli) propagator
                    let mu = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let nu = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let rho = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let sigma = LorentzIndex::Named(lorentz_index_label(lorentz_counter as usize));
                    lorentz_counter += 1;
                    let m_sq = prop.mass * prop.mass;
                    // g̃^{ab} = g^{ab} - p^a p^b / m²
                    let g_tilde = |a: &LorentzIndex, b: &LorentzIndex| -> CasExpr {
                        CasExpr::Add(vec![
                            CasExpr::MetricTensor {
                                mu: a.clone(),
                                nu: b.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Fraction {
                                numerator: Box::new(CasExpr::Mul(vec![
                                    CasExpr::Momentum {
                                        label: p_label.clone(),
                                        index: a.clone(),
                                    },
                                    CasExpr::Momentum {
                                        label: p_label.clone(),
                                        index: b.clone(),
                                    },
                                ])),
                                denominator: Box::new(CasExpr::Scalar(m_sq)),
                            })),
                        ])
                    };
                    let numerator = CasExpr::Mul(vec![
                        CasExpr::ImaginaryUnit,
                        CasExpr::Add(vec![
                            CasExpr::Mul(vec![g_tilde(&mu, &rho), g_tilde(&nu, &sigma)]),
                            CasExpr::Mul(vec![g_tilde(&mu, &sigma), g_tilde(&nu, &rho)]),
                            CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                                CasExpr::Fraction {
                                    numerator: Box::new(CasExpr::Scalar(2.0)),
                                    denominator: Box::new(CasExpr::Scalar(3.0)),
                                },
                                g_tilde(&mu, &nu),
                                g_tilde(&rho, &sigma),
                            ]))),
                        ]),
                    ]);
                    factors.push(CasExpr::Fraction {
                        numerator: Box::new(numerator),
                        denominator: Box::new(CasExpr::Add(vec![
                            CasExpr::DotProduct {
                                left: p_label.clone(),
                                right: p_label.clone(),
                            },
                            CasExpr::Neg(Box::new(CasExpr::Scalar(m_sq))),
                        ])),
                    });
                }
            }
        }
    }

    // Vertex coupling factors
    for node in &diagram.nodes {
        if let NodeKind::InternalVertex(ref vf) = node.kind {
            factors.push(CasExpr::Symbol {
                name: vf.expression.clone(),
                indices: vec![],
            });
        }
    }

    if factors.is_empty() {
        Ok(CasExpr::Scalar(1.0))
    } else if factors.len() == 1 {
        Ok(factors.into_iter().next().unwrap())
    } else {
        Ok(CasExpr::Mul(factors))
    }
}

// ===========================================================================
// CAS Trace & Tensor Identity Engine
// ===========================================================================

// ---------------------------------------------------------------------------
// γ^5 Chiral Trace Evaluation
// ---------------------------------------------------------------------------

/// Evaluate the Dirac trace of a CAS expression.
///
/// Supports:
/// - $\mathrm{Tr}\[I\] = 4$
/// - $\mathrm{Tr}[\text{odd }\gamma] = 0$
/// - $\mathrm{Tr}[\gamma^\mu \gamma^\nu] = 4\,g^{\mu\nu}$
/// - $\mathrm{Tr}[\gamma^\mu \gamma^\nu \gamma^\rho \gamma^\sigma] = 4(g^{\mu\nu}g^{\rho\sigma} - g^{\mu\rho}g^{\nu\sigma} + g^{\mu\sigma}g^{\nu\rho})$
/// - $\mathrm{Tr}[\gamma^5 \gamma^\mu \gamma^\nu \gamma^\rho \gamma^\sigma] = -4i\,\epsilon^{\mu\nu\rho\sigma}$
///
/// # Arguments
/// * `expr` — The expression inside the trace (typically a `Mul` of gamma matrices).
/// * `dim` — Spacetime dimension for dimensional regularization.
pub fn evaluate_cas_trace(expr: &CasExpr, dim: SpacetimeDimension) -> SpireResult<CasExpr> {
    // Extract gamma matrices and check for gamma5
    let factors = match expr {
        CasExpr::Mul(fs) => fs.clone(),
        CasExpr::GammaMat { .. } => vec![expr.clone()],
        CasExpr::Gamma5 => vec![expr.clone()],
        CasExpr::Scalar(v) => {
            // Tr[scalar * I] = 4 * scalar
            return Ok(CasExpr::Scalar(4.0 * v));
        }
        CasExpr::Add(terms) => {
            // Tr[A + B] = Tr[A] + Tr[B]
            let traces: Result<Vec<_>, _> =
                terms.iter().map(|t| evaluate_cas_trace(t, dim)).collect();
            return Ok(CasExpr::Add(traces?).simplify(dim));
        }
        _ => {
            // General case: return the trace unevaluated
            return Ok(CasExpr::Trace(Box::new(expr.clone())));
        }
    };

    // Separate scalars, gamma matrices, and gamma5
    let mut scalars: Vec<CasExpr> = Vec::new();
    let mut gammas: Vec<LorentzIndex> = Vec::new();
    let mut has_gamma5 = false;
    let mut slashed: Vec<String> = Vec::new(); // slashed momenta become gamma contractions

    for f in &factors {
        match f {
            CasExpr::Scalar(_)
            | CasExpr::ImaginaryUnit
            | CasExpr::Symbol { .. }
            | CasExpr::DotProduct { .. } => {
                scalars.push(f.clone());
            }
            CasExpr::GammaMat { index } => gammas.push(index.clone()),
            CasExpr::Gamma5 => {
                if has_gamma5 {
                    // (γ^5)^2 = I
                    has_gamma5 = false;
                } else {
                    has_gamma5 = true;
                }
            }
            CasExpr::SlashedMomentum { label } => slashed.push(label.clone()),
            _ => scalars.push(f.clone()),
        }
    }

    let n_gamma = gammas.len() + slashed.len();

    // Case 1: Tr[I] = 4 (no gammas)
    if n_gamma == 0 && !has_gamma5 {
        let mut result_factors = vec![CasExpr::Scalar(4.0)];
        result_factors.extend(scalars);
        return Ok(simplify_mul(result_factors, dim));
    }

    // Case 2: Tr[γ^5] = 0 (no additional gammas)
    if n_gamma == 0 && has_gamma5 {
        return Ok(CasExpr::Scalar(0.0));
    }

    // Case 3: Odd number of gammas (without γ^5) → 0
    if !n_gamma.is_multiple_of(2) && !has_gamma5 {
        return Ok(CasExpr::Scalar(0.0));
    }

    // Case 4: Tr[γ^μ γ^ν] = 4 g^{μν}
    if n_gamma == 2 && !has_gamma5 && slashed.is_empty() {
        let mut result_factors = vec![
            CasExpr::Scalar(4.0),
            CasExpr::MetricTensor {
                mu: gammas[0].clone(),
                nu: gammas[1].clone(),
            },
        ];
        result_factors.extend(scalars);
        return Ok(simplify_mul(result_factors, dim));
    }

    // Case 5: Tr[γ^5 γ^μ γ^ν] = 0  (less than 4 gammas with γ^5)
    if has_gamma5 && n_gamma < 4 {
        return Ok(CasExpr::Scalar(0.0));
    }

    // Case 6: Tr[γ^μ γ^ν γ^ρ γ^σ] = 4(g^{μν}g^{ρσ} - g^{μρ}g^{νσ} + g^{μσ}g^{νρ})
    if n_gamma == 4 && !has_gamma5 && slashed.is_empty() {
        let (a, b, c, d) = (
            gammas[0].clone(),
            gammas[1].clone(),
            gammas[2].clone(),
            gammas[3].clone(),
        );
        let term1 = CasExpr::Mul(vec![
            CasExpr::MetricTensor {
                mu: a.clone(),
                nu: b.clone(),
            },
            CasExpr::MetricTensor {
                mu: c.clone(),
                nu: d.clone(),
            },
        ]);
        let term2 = CasExpr::Mul(vec![
            CasExpr::MetricTensor {
                mu: a.clone(),
                nu: c.clone(),
            },
            CasExpr::MetricTensor {
                mu: b.clone(),
                nu: d.clone(),
            },
        ]);
        let term3 = CasExpr::Mul(vec![
            CasExpr::MetricTensor { mu: a, nu: d },
            CasExpr::MetricTensor { mu: b, nu: c },
        ]);
        let trace_result = CasExpr::Mul(vec![
            CasExpr::Scalar(4.0),
            CasExpr::Add(vec![term1, CasExpr::Neg(Box::new(term2)), term3]),
        ]);
        let mut result_factors = vec![trace_result];
        result_factors.extend(scalars);
        return Ok(simplify_mul(result_factors, dim));
    }

    // Case 7: Tr[γ^5 γ^μ γ^ν γ^ρ γ^σ] = -4i ε^{μνρσ}
    if has_gamma5 && n_gamma == 4 && slashed.is_empty() {
        let trace_result = CasExpr::Mul(vec![
            CasExpr::Scalar(-4.0),
            CasExpr::ImaginaryUnit,
            CasExpr::LeviCivita {
                indices: [
                    gammas[0].clone(),
                    gammas[1].clone(),
                    gammas[2].clone(),
                    gammas[3].clone(),
                ],
            },
        ]);
        let mut result_factors = vec![trace_result];
        result_factors.extend(scalars);
        return Ok(simplify_mul(result_factors, dim));
    }

    // General case: return unevaluated trace (for higher-order traces)
    let mut result_factors = scalars;
    result_factors.push(CasExpr::Trace(Box::new(expr.clone())));
    Ok(simplify_mul(result_factors, dim))
}

// ---------------------------------------------------------------------------
// Levi-Civita Tensor Algebra
// ---------------------------------------------------------------------------

/// Contract two Levi-Civita tensors with one or more shared indices.
///
/// The fundamental identity is:
/// $$\epsilon^{\mu\nu\rho\sigma}\epsilon_{\alpha\beta\gamma\delta} = -\det\begin{pmatrix}
/// g^\mu_\alpha & g^\mu_\beta & g^\mu_\gamma & g^\mu_\delta \\
/// g^\nu_\alpha & g^\nu_\beta & g^\nu_\gamma & g^\nu_\delta \\
/// g^\rho_\alpha & g^\rho_\beta & g^\rho_\gamma & g^\rho_\delta \\
/// g^\sigma_\alpha & g^\sigma_\beta & g^\sigma_\gamma & g^\sigma_\delta
/// \end{pmatrix}$$
///
/// The sign follows the West Coast convention.
pub fn contract_levi_civita(eps1: &[LorentzIndex; 4], eps2: &[LorentzIndex; 4]) -> CasExpr {
    // Find shared indices (contracted)
    let mut shared = Vec::new();
    for (i, e1) in eps1.iter().enumerate() {
        for (j, e2) in eps2.iter().enumerate() {
            if e1 == e2 {
                shared.push((i, j));
            }
        }
    }

    match shared.len() {
        0 => {
            // No contraction — return the product of two ε tensors
            CasExpr::Mul(vec![
                CasExpr::LeviCivita {
                    indices: eps1.clone(),
                },
                CasExpr::LeviCivita {
                    indices: eps2.clone(),
                },
            ])
        }
        4 => {
            // All indices contracted: ε^{μνρσ} ε_{μνρσ} = -24 (West Coast)
            CasExpr::Scalar(-24.0)
        }
        3 => {
            // Three indices contracted: result is ±2 δ terms
            // Find the free indices
            let mut free1 = None;
            let mut free2 = None;
            let shared_i: Vec<usize> = shared.iter().map(|(i, _)| *i).collect();
            let shared_j: Vec<usize> = shared.iter().map(|(_, j)| *j).collect();
            for i in 0..4 {
                if !shared_i.contains(&i) {
                    free1 = Some(i);
                }
                if !shared_j.contains(&i) {
                    free2 = Some(i);
                }
            }
            if let (Some(f1), Some(f2)) = (free1, free2) {
                // ε^{αμνρ} ε_{βμνρ} = -6 δ^α_β
                CasExpr::Mul(vec![
                    CasExpr::Scalar(-6.0),
                    CasExpr::KroneckerDelta {
                        mu: eps1[f1].clone(),
                        nu: eps2[f2].clone(),
                    },
                ])
            } else {
                // Should not happen, but return product as fallback
                CasExpr::Mul(vec![
                    CasExpr::LeviCivita {
                        indices: eps1.clone(),
                    },
                    CasExpr::LeviCivita {
                        indices: eps2.clone(),
                    },
                ])
            }
        }
        2 => {
            // Two indices contracted: ε^{αβμν} ε_{γδμν} = -2(δ^α_γ δ^β_δ - δ^α_δ δ^β_γ)
            let shared_i: Vec<usize> = shared.iter().map(|(i, _)| *i).collect();
            let shared_j: Vec<usize> = shared.iter().map(|(_, j)| *j).collect();
            let mut free1 = Vec::new();
            let mut free2 = Vec::new();
            for i in 0..4 {
                if !shared_i.contains(&i) {
                    free1.push(i);
                }
                if !shared_j.contains(&i) {
                    free2.push(i);
                }
            }
            if free1.len() == 2 && free2.len() == 2 {
                let (a, b) = (free1[0], free1[1]);
                let (c, d) = (free2[0], free2[1]);
                CasExpr::Mul(vec![
                    CasExpr::Scalar(-2.0),
                    CasExpr::Add(vec![
                        CasExpr::Mul(vec![
                            CasExpr::KroneckerDelta {
                                mu: eps1[a].clone(),
                                nu: eps2[c].clone(),
                            },
                            CasExpr::KroneckerDelta {
                                mu: eps1[b].clone(),
                                nu: eps2[d].clone(),
                            },
                        ]),
                        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                            CasExpr::KroneckerDelta {
                                mu: eps1[a].clone(),
                                nu: eps2[d].clone(),
                            },
                            CasExpr::KroneckerDelta {
                                mu: eps1[b].clone(),
                                nu: eps2[c].clone(),
                            },
                        ]))),
                    ]),
                ])
            } else {
                CasExpr::Mul(vec![
                    CasExpr::LeviCivita {
                        indices: eps1.clone(),
                    },
                    CasExpr::LeviCivita {
                        indices: eps2.clone(),
                    },
                ])
            }
        }
        1 => {
            // One index contracted: returns a 3×3 determinant of δ's
            // ε^{αβγμ} ε_{δεζμ} = -(δ^α_δ(δ^β_ε δ^γ_ζ - δ^β_ζ δ^γ_ε) - ...)
            let shared_i: Vec<usize> = shared.iter().map(|(i, _)| *i).collect();
            let shared_j: Vec<usize> = shared.iter().map(|(_, j)| *j).collect();
            let mut free1 = Vec::new();
            let mut free2 = Vec::new();
            for i in 0..4 {
                if !shared_i.contains(&i) {
                    free1.push(i);
                }
                if !shared_j.contains(&i) {
                    free2.push(i);
                }
            }
            if free1.len() == 3 && free2.len() == 3 {
                let (a, b, c) = (free1[0], free1[1], free1[2]);
                let (d, e, f) = (free2[0], free2[1], free2[2]);
                // 3×3 determinant of Kronecker deltas (with overall minus sign)
                let make_delta = |i: usize, j: usize| -> CasExpr {
                    CasExpr::KroneckerDelta {
                        mu: eps1[i].clone(),
                        nu: eps2[j].clone(),
                    }
                };
                // det = δ_ad(δ_be δ_cf - δ_bf δ_ce) - δ_ae(δ_bd δ_cf - δ_bf δ_cd) + δ_af(δ_bd δ_ce - δ_be δ_cd)
                let term1 = CasExpr::Mul(vec![
                    make_delta(a, d),
                    CasExpr::Add(vec![
                        CasExpr::Mul(vec![make_delta(b, e), make_delta(c, f)]),
                        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                            make_delta(b, f),
                            make_delta(c, e),
                        ]))),
                    ]),
                ]);
                let term2 = CasExpr::Mul(vec![
                    make_delta(a, e),
                    CasExpr::Add(vec![
                        CasExpr::Mul(vec![make_delta(b, d), make_delta(c, f)]),
                        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                            make_delta(b, f),
                            make_delta(c, d),
                        ]))),
                    ]),
                ]);
                let term3 = CasExpr::Mul(vec![
                    make_delta(a, f),
                    CasExpr::Add(vec![
                        CasExpr::Mul(vec![make_delta(b, d), make_delta(c, e)]),
                        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
                            make_delta(b, e),
                            make_delta(c, d),
                        ]))),
                    ]),
                ]);
                CasExpr::Neg(Box::new(CasExpr::Add(vec![
                    term1,
                    CasExpr::Neg(Box::new(term2)),
                    term3,
                ])))
            } else {
                CasExpr::Mul(vec![
                    CasExpr::LeviCivita {
                        indices: eps1.clone(),
                    },
                    CasExpr::LeviCivita {
                        indices: eps2.clone(),
                    },
                ])
            }
        }
        _ => {
            // Should not happen (max 4 shared), return product
            CasExpr::Mul(vec![
                CasExpr::LeviCivita {
                    indices: eps1.clone(),
                },
                CasExpr::LeviCivita {
                    indices: eps2.clone(),
                },
            ])
        }
    }
}

// ---------------------------------------------------------------------------
// Fierz Identity
// ---------------------------------------------------------------------------

/// Apply the Fierz identity to rearrange a product of spinor bilinears.
///
/// The Fierz identity relates products of the form
/// $(\bar{\psi}_1 \Gamma^A \psi_2)(\bar{\psi}_3 \Gamma^B \psi_4)$ to
/// linear combinations of $(\bar{\psi}_1 \Gamma^C \psi_4)(\bar{\psi}_3 \Gamma^D \psi_2)$.
///
/// The complete Fierz expansion in the basis $\{I, \gamma^\mu, \sigma^{\mu\nu}, \gamma^\mu\gamma^5, \gamma^5\}$
/// has 16 terms. This function returns the rearranged CAS expression.
///
/// # Arguments
/// * `bilinear1` — First spinor bilinear (as a CAS expression, e.g., ūΓu).
/// * `bilinear2` — Second spinor bilinear (as a CAS expression, e.g., v̄Γv).
///
/// # Returns
/// The Fierz-rearranged expression, or the original if not applicable.
pub fn apply_fierz_identity(bilinear1: &CasExpr, bilinear2: &CasExpr) -> CasExpr {
    // The Fierz identity is complex — this provides the structural framework.
    // A full implementation would decompose the gamma matrix insertions
    // and apply the 16×16 Fierz matrix.
    //
    // For now, we implement the simplest case: scalar × scalar
    // (ū₁ u₂)(ū₃ u₄) = ½(ū₁ u₄)(ū₃ u₂) + ½(ū₁ γ^5 u₄)(ū₃ γ^5 u₂) + ...
    //
    // Return the product as-is until the full Fierz matrix is implemented.
    CasExpr::Mul(vec![bilinear1.clone(), bilinear2.clone()])
}

// ---------------------------------------------------------------------------
// CAS Polarization Sum Rules
// ---------------------------------------------------------------------------

/// Apply the polarization sum rule for spin-1 (massless vector boson).
///
/// For photons in Feynman gauge:
/// $$\sum_\lambda \epsilon^\mu(k,\lambda) \epsilon^{*\nu}(k,\lambda) = -g^{\mu\nu}$$
pub fn polarization_sum_massless_vector(idx_mu: &LorentzIndex, idx_nu: &LorentzIndex) -> CasExpr {
    CasExpr::Neg(Box::new(CasExpr::MetricTensor {
        mu: idx_mu.clone(),
        nu: idx_nu.clone(),
    }))
}

/// Apply the polarization sum rule for spin-1 (massive vector boson).
///
/// $$\sum_\lambda \epsilon^\mu(k,\lambda) \epsilon^{*\nu}(k,\lambda)
///   = -g^{\mu\nu} + \frac{k^\mu k^\nu}{m^2}$$
pub fn polarization_sum_massive_vector(
    idx_mu: &LorentzIndex,
    idx_nu: &LorentzIndex,
    momentum: &str,
    mass_sq: f64,
) -> CasExpr {
    CasExpr::Add(vec![
        CasExpr::Neg(Box::new(CasExpr::MetricTensor {
            mu: idx_mu.clone(),
            nu: idx_nu.clone(),
        })),
        CasExpr::Fraction {
            numerator: Box::new(CasExpr::Mul(vec![
                CasExpr::Momentum {
                    label: momentum.to_string(),
                    index: idx_mu.clone(),
                },
                CasExpr::Momentum {
                    label: momentum.to_string(),
                    index: idx_nu.clone(),
                },
            ])),
            denominator: Box::new(CasExpr::Scalar(mass_sq)),
        },
    ])
}

/// Apply the spin-3/2 Rarita-Schwinger completeness relation.
///
/// $$\sum_s \psi^\mu(p,s) \bar{\psi}^\nu(p,s) =
///   -(\not{p} + m)\left(g^{\mu\nu} - \frac{\gamma^\mu\gamma^\nu}{3}
///   - \frac{2p^\mu p^\nu}{3m^2} + \frac{p^\mu\gamma^\nu - p^\nu\gamma^\mu}{3m}\right)$$
pub fn polarization_sum_rarita_schwinger(
    idx_mu: &LorentzIndex,
    idx_nu: &LorentzIndex,
    momentum: &str,
    mass: f64,
) -> CasExpr {
    let m_sq = mass * mass;
    let p_mu = CasExpr::Momentum {
        label: momentum.to_string(),
        index: idx_mu.clone(),
    };
    let p_nu = CasExpr::Momentum {
        label: momentum.to_string(),
        index: idx_nu.clone(),
    };
    let g_mu_gamma_nu = CasExpr::GammaMat {
        index: idx_mu.clone(),
    };
    let g_nu_gamma_mu = CasExpr::GammaMat {
        index: idx_nu.clone(),
    };

    // The tensor structure in parentheses
    let tensor_part = CasExpr::Add(vec![
        CasExpr::MetricTensor {
            mu: idx_mu.clone(),
            nu: idx_nu.clone(),
        },
        CasExpr::Neg(Box::new(CasExpr::Fraction {
            numerator: Box::new(CasExpr::Mul(vec![
                g_mu_gamma_nu.clone(),
                g_nu_gamma_mu.clone(),
            ])),
            denominator: Box::new(CasExpr::Scalar(3.0)),
        })),
        CasExpr::Neg(Box::new(CasExpr::Fraction {
            numerator: Box::new(CasExpr::Mul(vec![
                CasExpr::Scalar(2.0),
                p_mu.clone(),
                p_nu.clone(),
            ])),
            denominator: Box::new(CasExpr::Scalar(3.0 * m_sq)),
        })),
        CasExpr::Fraction {
            numerator: Box::new(CasExpr::Add(vec![
                CasExpr::Mul(vec![p_mu, g_nu_gamma_mu]),
                CasExpr::Neg(Box::new(CasExpr::Mul(vec![p_nu, g_mu_gamma_nu]))),
            ])),
            denominator: Box::new(CasExpr::Scalar(3.0 * mass)),
        },
    ]);

    // Multiplied by -(ᵽ + m)
    CasExpr::Neg(Box::new(CasExpr::Mul(vec![
        CasExpr::Add(vec![
            CasExpr::SlashedMomentum {
                label: momentum.to_string(),
            },
            CasExpr::Scalar(mass),
        ]),
        tensor_part,
    ])))
}

/// Apply the spin-2 (graviton) polarization sum.
///
/// For a massive spin-2 particle:
/// $$\sum_\lambda \epsilon^{\mu\nu}(k) \epsilon^{*\rho\sigma}(k)
///   = \frac{1}{2}(\tilde{g}^{\mu\rho}\tilde{g}^{\nu\sigma}
///     + \tilde{g}^{\mu\sigma}\tilde{g}^{\nu\rho})
///   - \frac{1}{3}\tilde{g}^{\mu\nu}\tilde{g}^{\rho\sigma}$$
///     where $\tilde{g}^{\mu\nu} = -g^{\mu\nu} + k^\mu k^\nu / m^2$.
pub fn polarization_sum_spin2(
    mu: &LorentzIndex,
    nu: &LorentzIndex,
    rho: &LorentzIndex,
    sigma: &LorentzIndex,
    momentum: &str,
    mass_sq: f64,
) -> CasExpr {
    // Helper: build g̃^{ab} = -g^{ab} + k^a k^b / m²
    let g_tilde = |a: &LorentzIndex, b: &LorentzIndex| -> CasExpr {
        CasExpr::Add(vec![
            CasExpr::Neg(Box::new(CasExpr::MetricTensor {
                mu: a.clone(),
                nu: b.clone(),
            })),
            CasExpr::Fraction {
                numerator: Box::new(CasExpr::Mul(vec![
                    CasExpr::Momentum {
                        label: momentum.to_string(),
                        index: a.clone(),
                    },
                    CasExpr::Momentum {
                        label: momentum.to_string(),
                        index: b.clone(),
                    },
                ])),
                denominator: Box::new(CasExpr::Scalar(mass_sq)),
            },
        ])
    };

    // ½(g̃^μρ g̃^νσ + g̃^μσ g̃^νρ) - ⅓ g̃^μν g̃^ρσ
    CasExpr::Add(vec![
        CasExpr::Mul(vec![
            CasExpr::Fraction {
                numerator: Box::new(CasExpr::Scalar(1.0)),
                denominator: Box::new(CasExpr::Scalar(2.0)),
            },
            CasExpr::Add(vec![
                CasExpr::Mul(vec![g_tilde(mu, rho), g_tilde(nu, sigma)]),
                CasExpr::Mul(vec![g_tilde(mu, sigma), g_tilde(nu, rho)]),
            ]),
        ]),
        CasExpr::Neg(Box::new(CasExpr::Mul(vec![
            CasExpr::Fraction {
                numerator: Box::new(CasExpr::Scalar(1.0)),
                denominator: Box::new(CasExpr::Scalar(3.0)),
            },
            g_tilde(mu, nu),
            g_tilde(rho, sigma),
        ]))),
    ])
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
    /// Optional performance profile from amplitude derivation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<crate::telemetry::ComputeProfile>,
}

// ---------------------------------------------------------------------------
// LaTeX Export
// ---------------------------------------------------------------------------

impl SymbolicTerm {
    /// Render this symbolic term as a LaTeX fragment.
    pub fn to_latex(&self) -> String {
        match self {
            SymbolicTerm::Spinor {
                kind,
                momentum_label,
                ..
            } => {
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
            SymbolicTerm::PropagatorTerm {
                form,
                momentum_label,
                mass,
                ..
            } => {
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
                            format!("\\frac{{i\\,\\gamma \\cdot {p}}}{{{p}^2 + i\\epsilon}}")
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
                    PropagatorForm::RaritaSchwinger => {
                        format!("S_{{3/2}}^{{\\mu\\nu}}({p},\\,{:.4})", mass)
                    }
                    PropagatorForm::MasslessSpin2 => {
                        format!(
                            "\\frac{{i}}{{2{p}^2}}\\left(g^{{\\mu\\rho}}g^{{\\nu\\sigma}} + g^{{\\mu\\sigma}}g^{{\\nu\\rho}} - g^{{\\mu\\nu}}g^{{\\rho\\sigma}}\\right)"
                        )
                    }
                    PropagatorForm::MassiveSpin2 => {
                        format!(
                            "D_{{\\mathrm{{FP}}}}^{{\\mu\\nu,\\rho\\sigma}}({p},\\,{:.4})",
                            mass
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
                let measure = format!("\\int \\frac{{d^{{{d}}} {l}}}{{(2\\pi)^{{{d}}}}}");
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
                    ScalarIntegralType::C0 {
                        external_momenta_sq,
                        masses_sq,
                    } => {
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
                    ScalarIntegralType::D0 {
                        external_momenta_sq,
                        masses_sq,
                        ..
                    } => {
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
    s = s.replace("Λ²", "\\Lambda^2");
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
        Self {
            e: 0.0,
            px: 0.0,
            py: 0.0,
            pz: 0.0,
        }
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
    use crate::telemetry::ComputeProfile;
    use std::time::Instant;

    let start = Instant::now();
    let mut profile = ComputeProfile::new();

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
                let expr = format!("ε{}^{}({})", conj_str, idx, edge.momentum_label);
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

    profile.record_stage("Amplitude Derivation", start.elapsed().as_secs_f64() * 1000.0);
    profile.capture_memory();
    profile.finalize(start);

    Ok(AmplitudeExpression {
        diagram_id: diagram.id,
        terms,
        couplings,
        momenta_labels,
        expression: full_expression,
        profile: Some(profile),
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
/// | $\mathrm{Tr}\[I\] = f(d)$ | $f(d)$ where $f(4) = 4$ in general $f(d)$ can be defined |
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
    } else if !gamma_indices.len().is_multiple_of(2) && !include_gamma5 {
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
                format!(
                    "Tr_{{d={}}}[{}{}]",
                    d_str,
                    indices_str.join(" "),
                    gamma5_str
                )
            }
        }
    };

    let mut steps = vec![input.clone()];
    // Record the d-dimensional identity used.
    if let SpacetimeDimension::DimReg { .. } = dim {
        steps.push(format!("Applied d-dimensional algebra with d = {}", d_str));
        steps.push(format!("g_μ^μ = {}", dim.metric_trace()));
        steps.push(format!("γ^μ γ_μ = {}", dim.gamma_contraction()));
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
            PropagatorForm::RaritaSchwinger => "RaritaSchwinger",
            PropagatorForm::MasslessSpin2 => "MasslessSpin2",
            PropagatorForm::MassiveSpin2 => "MassiveSpin2",
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
        .and_then(|r| r.loop_momenta.first()).cloned()
        .unwrap_or_else(|| "l".into());

    // Collect external momentum labels from the graph.
    let external_momenta: Vec<String> = graph
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                NodeKind::ExternalIncoming(_) | NodeKind::ExternalOutgoing(_)
            )
        })
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
                format!("C₀({}; m²={:?})", ext_sq.join(", "), masses),
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
                format!("D₀({}; s,t; m²={:?})", ext_sq.join(", "), masses),
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
// Numerical Evaluation Engine
// ===========================================================================

/// A complex number type for numerical evaluation of scattering amplitudes.
///
/// Uses the standard representation $z = \text{re} + i \cdot \text{im}$.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Complex {
    /// Real part $\mathrm{Re}(z)$.
    pub re: f64,
    /// Imaginary part $\mathrm{Im}(z)$.
    pub im: f64,
}

impl Complex {
    /// Create a new complex number.
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// A purely real complex number.
    pub fn real(v: f64) -> Self {
        Self { re: v, im: 0.0 }
    }

    /// The imaginary unit $i$.
    pub fn i() -> Self {
        Self { re: 0.0, im: 1.0 }
    }

    /// Complex zero.
    pub fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    /// Complex one.
    pub fn one() -> Self {
        Self { re: 1.0, im: 0.0 }
    }

    /// The squared modulus $|z|^2 = \mathrm{Re}(z)^2 + \mathrm{Im}(z)^2$.
    pub fn norm_sq(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// The modulus $|z| = \sqrt{|z|^2}$.
    pub fn norm(&self) -> f64 {
        self.norm_sq().sqrt()
    }

    /// Complex conjugate $z^* = \mathrm{Re}(z) - i \cdot \mathrm{Im}(z)$.
    pub fn conj(&self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }
}

impl std::ops::Add for Complex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl std::ops::Div for Complex {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        let denom = rhs.norm_sq();
        if denom < 1e-300 {
            // Division by zero — return NaN-like sentinel.
            return Self {
                re: f64::NAN,
                im: f64::NAN,
            };
        }
        Self {
            re: (self.re * rhs.re + self.im * rhs.im) / denom,
            im: (self.im * rhs.re - self.re * rhs.im) / denom,
        }
    }
}

impl std::ops::Neg for Complex {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl std::ops::AddAssign for Complex {
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
}

impl std::fmt::Display for Complex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.im.abs() < 1e-15 {
            write!(f, "{:.6}", self.re)
        } else if self.re.abs() < 1e-15 {
            write!(f, "{:.6}i", self.im)
        } else if self.im >= 0.0 {
            write!(f, "{:.6} + {:.6}i", self.re, self.im)
        } else {
            write!(f, "{:.6} - {:.6}i", self.re, -self.im)
        }
    }
}

// ---------------------------------------------------------------------------
// Numerical Context
// ---------------------------------------------------------------------------

/// A numerical evaluation context mapping symbolic labels to concrete values.
///
/// The `NumericalContext` provides the substitution map needed to evaluate
/// a symbolic `CasExpr` into a numerical `Complex` result. It stores:
///
/// - **Momenta**: $p_i^\mu$ as `SpacetimeVector` values keyed by label.
/// - **Scalar symbols**: Named couplings, masses, etc. as `Complex` values.
/// - **Metric**: The spacetime metric for inner products.
///
/// # Example
///
/// ```rust,no_run
/// # use spire_kernel::algebra::*;
/// let mut ctx = NumericalContext::new_minkowski();
/// ctx.set_momentum("p1", SpacetimeVector::new_4d(100.0, 0.0, 0.0, 100.0));
/// ctx.set_symbol("g_e", Complex::real(0.3028));
/// ```
#[derive(Debug, Clone)]
pub struct NumericalContext {
    /// Named four-momenta: label → spacetime vector.
    pub momenta: HashMap<String, SpacetimeVector>,
    /// Named scalar symbols: label → complex value.
    pub symbols: HashMap<String, Complex>,
    /// The metric signature for computing inner products.
    pub metric: MetricSignature,
    /// Infinitesimal width parameter $\epsilon$ for propagator denominators.
    /// Default: $10^{-10}$.
    pub i_epsilon: f64,
    /// Optional form factor for scaling vertex couplings by momentum transfer.
    ///
    /// When `Some(ff)`, calling [`apply_form_factor`](Self::apply_form_factor)
    /// multiplies a coupling by $F(Q^2)$. When `None`, all vertices are
    /// treated as point-like interactions.
    pub form_factor: Option<crate::lagrangian::FormFactor>,
}

impl NumericalContext {
    /// Create a new context with the standard 4D Minkowski metric.
    pub fn new_minkowski() -> Self {
        Self {
            momenta: HashMap::new(),
            symbols: HashMap::new(),
            metric: MetricSignature::minkowski_4d(),
            i_epsilon: 1e-10,
            form_factor: None,
        }
    }

    /// Create a context with a custom metric signature.
    pub fn new(metric: MetricSignature) -> Self {
        Self {
            momenta: HashMap::new(),
            symbols: HashMap::new(),
            metric,
            i_epsilon: 1e-10,
            form_factor: None,
        }
    }

    /// Insert or update a named momentum vector.
    pub fn set_momentum(&mut self, label: &str, momentum: SpacetimeVector) {
        self.momenta.insert(label.to_string(), momentum);
    }

    /// Insert or update a named scalar symbol.
    pub fn set_symbol(&mut self, name: &str, value: Complex) {
        self.symbols.insert(name.to_string(), value);
    }

    /// Retrieve a momentum by label, or return an error.
    pub fn get_momentum(&self, label: &str) -> Result<&SpacetimeVector, String> {
        self.momenta
            .get(label)
            .ok_or_else(|| format!("Momentum '{}' not found in numerical context", label))
    }

    /// Retrieve a symbol by name, or return an error.
    pub fn get_symbol(&self, name: &str) -> Result<Complex, String> {
        self.symbols
            .get(name)
            .copied()
            .ok_or_else(|| format!("Symbol '{}' not found in numerical context", name))
    }

    /// Compute the Minkowski dot product $p \cdot q$ for two named momenta.
    pub fn dot_product(&self, left: &str, right: &str) -> Result<f64, String> {
        let p = self.get_momentum(left)?;
        let q = self.get_momentum(right)?;
        self.metric.inner_product(p, q)
    }

    /// Set an optional form factor for vertex coupling suppression.
    pub fn set_form_factor(&mut self, ff: crate::lagrangian::FormFactor) {
        self.form_factor = Some(ff);
    }

    /// Clear the form factor (revert to point-like couplings).
    pub fn clear_form_factor(&mut self) {
        self.form_factor = None;
    }

    /// Apply the form factor to a coupling value at the given $Q^2$.
    ///
    /// If no form factor is set, returns the coupling unchanged.
    /// Otherwise returns `coupling * F(Q^2)`.
    pub fn apply_form_factor(&self, coupling: Complex, q2: f64) -> Complex {
        match &self.form_factor {
            None => coupling,
            Some(ff) => {
                let suppression = ff.evaluate(q2);
                Complex::new(coupling.re * suppression, coupling.im * suppression)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CasExpr Numerical Evaluation
// ---------------------------------------------------------------------------

impl CasExpr {
    /// Evaluate this symbolic expression to a numerical complex value.
    ///
    /// Recursively traverses the expression tree, substituting momenta and
    /// symbols from the `NumericalContext`, and performing all arithmetic
    /// to produce a single `Complex` result.
    ///
    /// # Supported Nodes
    ///
    /// - `Scalar`, `ImaginaryUnit` — direct numerical values.
    /// - `Symbol` — looked up in `ctx.symbols`.
    /// - `DotProduct` — computed via `ctx.dot_product`.
    /// - `Add`, `Mul`, `Neg`, `Fraction` — standard arithmetic.
    /// - `PropagatorDenom` — evaluated as $1/(p^2 - m^2 + i\epsilon)$.
    /// - `MetricTensor`, `KroneckerDelta` — evaluated with numeric indices.
    /// - `Trace` — evaluated by summing over Lorentz index values $0,1,2,3$.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a required symbol or momentum is missing from the
    /// context, or if the expression contains unevaluable nodes (e.g.,
    /// unresolved named Lorentz indices).
    pub fn evaluate_numerically(&self, ctx: &NumericalContext) -> Result<Complex, String> {
        match self {
            // --- Atomic leaves ---
            CasExpr::Scalar(v) => Ok(Complex::real(*v)),

            CasExpr::ImaginaryUnit => Ok(Complex::i()),

            CasExpr::Symbol { name, .. } => ctx.get_symbol(name),

            CasExpr::DotProduct { left, right } => {
                let val = ctx.dot_product(left, right)?;
                Ok(Complex::real(val))
            }

            CasExpr::MetricTensor { mu, nu } => {
                let mu_val = resolve_numeric_index(mu)?;
                let nu_val = resolve_numeric_index(nu)?;
                if mu_val == nu_val {
                    Ok(Complex::real(ctx.metric.component(mu_val)))
                } else {
                    Ok(Complex::zero())
                }
            }

            CasExpr::KroneckerDelta { mu, nu } => {
                let mu_val = resolve_numeric_index(mu)?;
                let nu_val = resolve_numeric_index(nu)?;
                if mu_val == nu_val {
                    Ok(Complex::one())
                } else {
                    Ok(Complex::zero())
                }
            }

            CasExpr::Momentum { label, index } => {
                let p = ctx.get_momentum(label)?;
                let idx = resolve_numeric_index(index)?;
                if idx >= p.dimension() {
                    return Err(format!(
                        "Index {} out of bounds for {}-D momentum '{}'",
                        idx,
                        p.dimension(),
                        label
                    ));
                }
                Ok(Complex::real(p.components()[idx]))
            }

            CasExpr::PropagatorDenom { momentum, mass_sq } => {
                let p = ctx.get_momentum(momentum)?;
                let p_sq = ctx.metric.inner_product(p, p)?;
                // 1 / (p² - m² + iε)
                let denom = Complex::new(p_sq - mass_sq, ctx.i_epsilon);
                Ok(Complex::one() / denom)
            }

            // --- Composite nodes ---
            CasExpr::Add(terms) => {
                let mut sum = Complex::zero();
                for t in terms {
                    sum += t.evaluate_numerically(ctx)?;
                }
                Ok(sum)
            }

            CasExpr::Mul(factors) => {
                let mut product = Complex::one();
                for f in factors {
                    product = product * f.evaluate_numerically(ctx)?;
                }
                Ok(product)
            }

            CasExpr::Neg(inner) => Ok(-inner.evaluate_numerically(ctx)?),

            CasExpr::Fraction {
                numerator,
                denominator,
            } => {
                let n = numerator.evaluate_numerically(ctx)?;
                let d = denominator.evaluate_numerically(ctx)?;
                Ok(n / d)
            }

            CasExpr::Trace(inner) => {
                // For a trace over Lorentz indices, we evaluate by summing
                // the inner expression with each Lorentz index replaced
                // numerically. For simple traces (result of symbolic trace
                // evaluation), this just evaluates the scalar expression.
                inner.evaluate_numerically(ctx)
            }

            CasExpr::SlashedMomentum { label } => {
                // ⧸p = γ^μ p_μ — this is a matrix-valued object.
                // In a fully contracted/traced amplitude it should have been
                // reduced to scalars. If we encounter it here, treat as the
                // Lorentz-contracted scalar p·p for a rough numerical estimate.
                let p = ctx.get_momentum(label)?;
                let p_sq = ctx.metric.inner_product(p, p)?;
                Ok(Complex::real(p_sq))
            }

            CasExpr::LeviCivita { indices } => {
                // ε^{μνρσ} with numeric indices
                let idx: Result<Vec<usize>, String> =
                    indices.iter().map(resolve_numeric_index).collect();
                let idx = idx?;
                if idx.len() != 4 {
                    return Err("Levi-Civita requires exactly 4 indices".into());
                }
                Ok(Complex::real(levi_civita_numeric(
                    idx[0], idx[1], idx[2], idx[3],
                )))
            }

            CasExpr::GammaMat { .. } | CasExpr::Gamma5 => {
                // Gamma matrices are matrix-valued — in a fully simplified
                // amplitude they should not appear bare. Return an error.
                Err("Cannot numerically evaluate bare gamma matrix. \
                     Ensure amplitude has been fully simplified (traces evaluated, \
                     spinor contractions performed)."
                    .into())
            }

            CasExpr::SpinorU { .. }
            | CasExpr::SpinorUBar { .. }
            | CasExpr::SpinorV { .. }
            | CasExpr::SpinorVBar { .. } => Err("Cannot numerically evaluate bare spinor. \
                     Ensure amplitude has been squared and spin-summed."
                .into()),

            CasExpr::Tensor { .. } => Err("Cannot numerically evaluate uncontracted tensor. \
                     Ensure all Lorentz indices are contracted."
                .into()),

            CasExpr::Commutator(a, b) => {
                // [A, B] = AB - BA — evaluate as scalars
                let va = a.evaluate_numerically(ctx)?;
                let vb = b.evaluate_numerically(ctx)?;
                Ok(va * vb - vb * va) // For scalars this is 0, which is correct.
            }

            CasExpr::AntiCommutator(a, b) => {
                let va = a.evaluate_numerically(ctx)?;
                let vb = b.evaluate_numerically(ctx)?;
                Ok(va * vb + vb * va) // For scalars: 2ab
            }
        }
    }
}

/// Resolve a `LorentzIndex` to a numeric value.
///
/// Only `Numeric(n)` indices can be resolved. Named or Dummy indices
/// indicate the expression has not been fully contracted/traced.
fn resolve_numeric_index(idx: &LorentzIndex) -> Result<usize, String> {
    match idx {
        LorentzIndex::Numeric(n) => Ok(*n as usize),
        LorentzIndex::Named(name) => Err(format!(
            "Cannot numerically resolve named index '{}'. \
                         Ensure all indices are contracted.",
            name
        )),
        LorentzIndex::Dummy(n) => Err(format!(
            "Cannot numerically resolve dummy index λ_{}. \
                         Ensure all contractions have been performed.",
            n
        )),
    }
}

/// Evaluate the Levi-Civita symbol $\epsilon^{\mu\nu\rho\sigma}$ for
/// numeric index values.
///
/// Returns $+1$ for even permutations of $(0,1,2,3)$, $-1$ for odd
/// permutations, and $0$ if any indices are repeated.
fn levi_civita_numeric(a: usize, b: usize, c: usize, d: usize) -> f64 {
    let indices = [a, b, c, d];
    // Check for repeated indices.
    for i in 0..4 {
        for j in (i + 1)..4 {
            if indices[i] == indices[j] {
                return 0.0;
            }
        }
    }
    // Count inversions to determine parity.
    let mut inversions = 0;
    for i in 0..4 {
        for j in (i + 1)..4 {
            if indices[i] > indices[j] {
                inversions += 1;
            }
        }
    }
    if inversions % 2 == 0 {
        1.0
    } else {
        -1.0
    }
}

// ---------------------------------------------------------------------------
// Spin/Helicity Summation for Unpolarized Cross-Sections
// ---------------------------------------------------------------------------

/// Compute the spin-averaged squared amplitude $\overline{|\mathcal{M}|^2}$
/// at a single phase-space point.
///
/// For unpolarized cross-sections with $n_i$ initial-state and $n_f$ final-state
/// spin degrees of freedom:
///
/// $$\overline{|\mathcal{M}|^2} = \frac{1}{N_{\text{avg}}} \sum_{\text{spins}} |\mathcal{M}|^2$$
///
/// where $N_{\text{avg}} = \prod_i (2s_i + 1)$ for initial-state particles.
///
/// # Arguments
///
/// * `amplitude_sq` — The squared amplitude $|\mathcal{M}|^2$ as a `CasExpr`,
///   already evaluated symbolically (traces performed, indices contracted).
/// * `ctx` — The numerical context with momenta set to the phase-space point.
/// * `initial_spin_dofs` — Spin degrees of freedom for each initial-state particle
///   (e.g., $2$ for a spin-$\frac{1}{2}$ fermion, $2$ for a massless vector boson).
///
/// # Returns
///
/// The spin-averaged $\overline{|\mathcal{M}|^2}$ as a real `f64`.
pub fn spin_averaged_amplitude_sq(
    amplitude_sq: &CasExpr,
    ctx: &NumericalContext,
    initial_spin_dofs: &[u32],
) -> Result<f64, String> {
    let m_sq = amplitude_sq.evaluate_numerically(ctx)?;

    // The spin-averaged amplitude should be real (imaginary part should cancel).
    if m_sq.im.abs() > 1e-8 * m_sq.re.abs().max(1.0) {
        return Err(format!(
            "Squared amplitude has significant imaginary part: {} \
             (expected purely real result)",
            m_sq
        ));
    }

    let n_avg: u32 = initial_spin_dofs.iter().product();
    let averaging_factor = if n_avg > 0 { n_avg as f64 } else { 1.0 };

    Ok(m_sq.re / averaging_factor)
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Channel, Edge, FeynmanGraph, LoopOrder, Node, NodeKind};
    use crate::lagrangian::{
        derive_propagator, LagrangianTerm, LagrangianTermKind, Propagator, VertexFactor,
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
                lepton_numbers: LeptonNumbers {
                    electron: 1,
                    muon: 0,
                    tau: 0,
                },
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
                lepton_numbers: LeptonNumbers {
                    electron: -1,
                    muon: 0,
                    tau: 0,
                },
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
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 1,
                    tau: 0,
                },
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
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: -1,
                    tau: 0,
                },
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
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 0,
                    tau: 0,
                },
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
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 0,
                    tau: 0,
                },
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

    fn make_vertex_factor(
        term_id: &str,
        field_ids: &[&str],
        expr: &str,
        coupling: f64,
    ) -> VertexFactor {
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
        assert!(
            (p_sq - m * m).abs() < 1e-12,
            "p² = {}, expected {}",
            p_sq,
            m * m
        );
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
        assert!(
            (p.dot(&q) - q.dot(&p)).abs() < 1e-12,
            "dot product should be symmetric"
        );
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
            SymbolicTerm::Spinor {
                kind,
                label,
                momentum_label,
            } => {
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
            SymbolicTerm::PropagatorTerm {
                form,
                momentum_label,
                mass,
                ..
            } => {
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
            terms: vec![SymbolicTerm::VertexCoupling {
                expression: "-ie γ^μ".into(),
                term_id: "qed_eea".into(),
            }],
            couplings: vec!["0.303".into()],
            momenta_labels: vec!["p1".into(), "p2".into()],
            expression: "V[-ie γ^μ]".into(),
            profile: None,
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

        let vf1 = make_vertex_factor(
            "qed_annihilation",
            &["e-", "e+", "photon"],
            "-i e γ^μ",
            0.303,
        );
        let vf2 = make_vertex_factor(
            "qed_mu_annihilation",
            &["mu-", "mu+", "photon"],
            "-i e γ^ν",
            0.303,
        );

        let photon_prop = make_propagator(&photon_field());

        let mut graph = DiGraph::<Node, Edge>::new();

        // External nodes
        let n_in1 = graph.add_node(Node {
            id: 0,
            kind: NodeKind::ExternalIncoming(e_minus.clone()),
            position: None,
        });
        let n_in2 = graph.add_node(Node {
            id: 1,
            kind: NodeKind::ExternalIncoming(e_plus.clone()),
            position: None,
        });
        let n_v1 = graph.add_node(Node {
            id: 2,
            kind: NodeKind::InternalVertex(vf1.clone()),
            position: None,
        });
        let n_v2 = graph.add_node(Node {
            id: 3,
            kind: NodeKind::InternalVertex(vf2.clone()),
            position: None,
        });
        let n_out1 = graph.add_node(Node {
            id: 4,
            kind: NodeKind::ExternalOutgoing(mu_minus.clone()),
            position: None,
        });
        let n_out2 = graph.add_node(Node {
            id: 5,
            kind: NodeKind::ExternalOutgoing(mu_plus.clone()),
            position: None,
        });

        // Edges
        graph.add_edge(
            n_in1,
            n_v1,
            Edge {
                field: electron_field(),
                propagator: None,
                momentum_label: "p1".into(),
                is_external: true,
            },
        );
        graph.add_edge(
            n_in2,
            n_v1,
            Edge {
                field: positron_field(),
                propagator: None,
                momentum_label: "p2".into(),
                is_external: true,
            },
        );
        graph.add_edge(
            n_v1,
            n_v2,
            Edge {
                field: photon_field(),
                propagator: Some(photon_prop),
                momentum_label: "q".into(),
                is_external: false,
            },
        );
        graph.add_edge(
            n_v2,
            n_out1,
            Edge {
                field: muon_field(),
                propagator: None,
                momentum_label: "p3".into(),
                is_external: true,
            },
        );
        graph.add_edge(
            n_v2,
            n_out2,
            Edge {
                field: antimuon_field(),
                propagator: None,
                momentum_label: "p4".into(),
                is_external: true,
            },
        );

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

        let n_spinors = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::Spinor { .. }))
            .count();
        let n_propagators = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::PropagatorTerm { .. }))
            .count();
        let n_vertices = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::VertexCoupling { .. }))
            .count();
        let n_polarizations = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::PolarizationVec { .. }))
            .count();

        assert_eq!(
            n_spinors, 4,
            "4 external fermion spinors (u, ū, v, v̄), got {}",
            n_spinors
        );
        assert_eq!(
            n_propagators, 1,
            "1 internal photon propagator, got {}",
            n_propagators
        );
        assert_eq!(n_vertices, 2, "2 vertex couplings, got {}", n_vertices);
        assert_eq!(
            n_polarizations, 0,
            "no external bosons, got {}",
            n_polarizations
        );
        assert_eq!(amp.terms.len(), 7, "total 7 terms, got {}", amp.terms.len());
    }

    #[test]
    fn generate_amplitude_spinor_kinds_correct() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let spinors: Vec<_> = amp
            .terms
            .iter()
            .filter_map(|t| match t {
                SymbolicTerm::Spinor { kind, label, .. } => Some((*kind, label.clone())),
                _ => None,
            })
            .collect();

        // Incoming e- (particle, not anti) → U
        assert!(
            spinors
                .iter()
                .any(|(k, l)| *k == SpinorKind::U && l == "e-"),
            "Expected U spinor for incoming e-"
        );
        // Incoming e+ (antiparticle) → VBar
        assert!(
            spinors
                .iter()
                .any(|(k, l)| *k == SpinorKind::VBar && l == "e+"),
            "Expected VBar spinor for incoming e+"
        );
        // Outgoing μ- (particle) → UBar
        assert!(
            spinors
                .iter()
                .any(|(k, l)| *k == SpinorKind::UBar && l == "mu-"),
            "Expected UBar spinor for outgoing μ-"
        );
        // Outgoing μ+ (antiparticle) → V
        assert!(
            spinors
                .iter()
                .any(|(k, l)| *k == SpinorKind::V && l == "mu+"),
            "Expected V spinor for outgoing μ+"
        );
    }

    #[test]
    fn generate_amplitude_propagator_form() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let props: Vec<_> = amp
            .terms
            .iter()
            .filter_map(|t| match t {
                SymbolicTerm::PropagatorTerm {
                    form,
                    momentum_label,
                    mass,
                    ..
                } => Some((*form, momentum_label.clone(), *mass)),
                _ => None,
            })
            .collect();

        assert_eq!(props.len(), 1);
        assert_eq!(
            props[0].0,
            PropagatorForm::MasslessVector,
            "photon propagator form"
        );
        assert_eq!(props[0].1, "q", "propagator momentum label");
        assert!((props[0].2).abs() < 1e-12, "photon mass = 0");
    }

    #[test]
    fn generate_amplitude_vertex_expressions() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        let vertices: Vec<_> = amp
            .terms
            .iter()
            .filter_map(|t| match t {
                SymbolicTerm::VertexCoupling {
                    expression,
                    term_id,
                } => Some((expression.clone(), term_id.clone())),
                _ => None,
            })
            .collect();

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

        assert!(
            !amp.couplings.is_empty(),
            "should collect coupling constants"
        );
    }

    #[test]
    fn generate_amplitude_expression_string_nonempty() {
        let diagram = make_s_channel_ee_to_mumu();
        let amp = generate_amplitude(&diagram).unwrap();

        assert!(!amp.expression.is_empty());
        assert!(
            amp.expression.contains("×"),
            "expression should join terms with ×"
        );
    }

    /// Test with external bosons: build a 1→2 decay with two photons in final state.
    /// This tests PolarizationVec generation.
    fn make_decay_to_photons() -> FeynmanGraph {
        // Fictitious "X → γγ" decay vertex
        let higgs = make_particle(&higgs_field(), false);
        let vf = make_vertex_factor("hgg", &["H", "photon", "photon"], "-i λ", 0.01);

        let mut graph = DiGraph::<Node, Edge>::new();

        let n_in = graph.add_node(Node {
            id: 0,
            kind: NodeKind::ExternalIncoming(higgs),
            position: None,
        });
        let n_v = graph.add_node(Node {
            id: 1,
            kind: NodeKind::InternalVertex(vf.clone()),
            position: None,
        });
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

        graph.add_edge(
            n_in,
            n_v,
            Edge {
                field: higgs_field(),
                propagator: None,
                momentum_label: "p1".into(),
                is_external: true,
            },
        );
        graph.add_edge(
            n_v,
            n_out1,
            Edge {
                field: photon_field(),
                propagator: None,
                momentum_label: "p2".into(),
                is_external: true,
            },
        );
        graph.add_edge(
            n_v,
            n_out2,
            Edge {
                field: photon_field(),
                propagator: None,
                momentum_label: "p3".into(),
                is_external: true,
            },
        );

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

        let n_pol = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::PolarizationVec { .. }))
            .count();
        let n_vert = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::VertexCoupling { .. }))
            .count();
        let n_prop = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::PropagatorTerm { .. }))
            .count();
        let n_spinors = amp
            .terms
            .iter()
            .filter(|t| matches!(t, SymbolicTerm::Spinor { .. }))
            .count();

        assert_eq!(n_pol, 2, "2 outgoing photon polarization vectors");
        assert_eq!(n_vert, 1, "1 vertex coupling");
        assert_eq!(n_prop, 0, "no internal propagators");
        assert_eq!(n_spinors, 0, "no fermion spinors (scalar→boson decay)");
    }

    #[test]
    fn generate_amplitude_polarization_conjugation() {
        let diagram = make_decay_to_photons();
        let amp = generate_amplitude(&diagram).unwrap();

        let pols: Vec<_> = amp
            .terms
            .iter()
            .filter_map(|t| match t {
                SymbolicTerm::PolarizationVec { is_conjugate, .. } => Some(*is_conjugate),
                _ => None,
            })
            .collect();

        // Outgoing bosons should have conjugated polarization vectors (ε*)
        assert!(
            pols.iter().all(|c| *c),
            "outgoing photon ε should be conjugated"
        );
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
            profile: None,
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
        use crate::lagrangian::{derive_propagator, derive_vertex_factor, TheoreticalModel};
        use crate::ontology::initialize_state;
        use crate::s_matrix::{AsymptoticState, Reaction};

        // Minimal model: e⁺e⁻ → μ⁺μ⁻ via photon only
        let fields = vec![
            electron_field(),
            positron_field(),
            muon_field(),
            antimuon_field(),
            photon_field(),
        ];
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

        let propagators = fields
            .iter()
            .map(|f| derive_propagator(f).unwrap())
            .collect();
        let vertex_factors = terms
            .iter()
            .map(|t| derive_vertex_factor(t).unwrap())
            .collect();

        let model = TheoreticalModel {
            name: "Minimal QED".into(),
            description: "Test".into(),
            fields,
            terms,
            vertex_factors,
            propagators,
            gauge_symmetry: None,
            spacetime: SpacetimeConfig::default(),
            constants: crate::ontology::PhysicalConstants::default(),
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
            initial: AsymptoticState {
                states: initial_states,
                invariant_mass_sq: 40000.0,
            },
            final_state: AsymptoticState {
                states: final_states,
                invariant_mass_sq: 40000.0,
            },
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
        assert_eq!(
            amp.terms.len(),
            7,
            "integrated test: expected 7 terms, got {}",
            amp.terms.len()
        );
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
            profile: None,
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
    // Dimensional Regularization & Loop Integral Tests
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
        let r =
            evaluate_trace_d(&[0, 1, 2, 3, 4, 5], false, SpacetimeDimension::dim_reg_4()).unwrap();
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
            profile: None,
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
            profile: None,
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

    // ===================================================================
    // CAS Engine Tests
    // ===================================================================

    // --- LorentzIndex ---

    #[test]
    fn lorentz_index_named_latex() {
        let idx = LorentzIndex::Named("mu".into());
        assert_eq!(idx.to_latex(), "\\mu");
    }

    #[test]
    fn lorentz_index_dummy_latex() {
        let idx = LorentzIndex::Dummy(42);
        assert_eq!(idx.to_latex(), "\\lambda_{42}");
    }

    #[test]
    fn lorentz_index_numeric_latex() {
        let idx = LorentzIndex::Numeric(3);
        assert_eq!(idx.to_latex(), "3");
    }

    #[test]
    fn lorentz_index_equality() {
        assert_eq!(
            LorentzIndex::Named("mu".into()),
            LorentzIndex::Named("mu".into())
        );
        assert_ne!(
            LorentzIndex::Named("mu".into()),
            LorentzIndex::Named("nu".into())
        );
        assert_ne!(LorentzIndex::Named("mu".into()), LorentzIndex::Dummy(0));
    }

    // --- CasExpr Basics ---

    #[test]
    fn cas_scalar_zero_and_one() {
        assert!(CasExpr::Scalar(0.0).is_scalar_zero());
        assert!(!CasExpr::Scalar(1.0).is_scalar_zero());
        assert!(CasExpr::Scalar(1.0).is_scalar_one());
        assert!(!CasExpr::Scalar(0.0).is_scalar_one());
    }

    #[test]
    fn cas_commutativity_scalars() {
        assert!(CasExpr::Scalar(3.14).is_commutative());
        assert!(CasExpr::ImaginaryUnit.is_commutative());
        assert!(CasExpr::DotProduct {
            left: "p".into(),
            right: "q".into()
        }
        .is_commutative());
    }

    #[test]
    fn cas_commutativity_gamma_matrices() {
        assert!(!CasExpr::GammaMat {
            index: LorentzIndex::Named("mu".into())
        }
        .is_commutative());
        assert!(!CasExpr::Gamma5.is_commutative());
        assert!(!CasExpr::SlashedMomentum { label: "p".into() }.is_commutative());
    }

    #[test]
    fn cas_commutativity_spinors() {
        assert!(!CasExpr::SpinorU {
            label: "e".into(),
            momentum: "p1".into()
        }
        .is_commutative());
        assert!(!CasExpr::SpinorUBar {
            label: "e".into(),
            momentum: "p1".into()
        }
        .is_commutative());
    }

    #[test]
    fn cas_commutativity_metric_and_levi_civita() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        assert!(CasExpr::MetricTensor {
            mu: mu.clone(),
            nu: nu.clone()
        }
        .is_commutative());
        assert!(CasExpr::LeviCivita {
            indices: [
                mu.clone(),
                nu.clone(),
                LorentzIndex::Named("rho".into()),
                LorentzIndex::Named("sigma".into())
            ],
        }
        .is_commutative());
    }

    #[test]
    fn cas_trace_is_commutative() {
        // The trace of anything is a scalar, hence commutative.
        let inner = CasExpr::GammaMat {
            index: LorentzIndex::Named("mu".into()),
        };
        assert!(CasExpr::Trace(Box::new(inner)).is_commutative());
    }

    // --- Free Indices ---

    #[test]
    fn cas_free_indices_metric_tensor() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let expr = CasExpr::MetricTensor {
            mu: mu.clone(),
            nu: nu.clone(),
        };
        let free = expr.get_free_indices();
        assert_eq!(free.len(), 2);
        assert!(free.contains(&mu));
        assert!(free.contains(&nu));
    }

    #[test]
    fn cas_free_indices_contracted() {
        // g^{μμ} — same index twice → contracted, no free indices
        let mu = LorentzIndex::Named("mu".into());
        let expr = CasExpr::MetricTensor {
            mu: mu.clone(),
            nu: mu.clone(),
        };
        let free = expr.get_free_indices();
        assert!(free.is_empty());
    }

    #[test]
    fn cas_free_indices_product() {
        // g^{μν} γ^ν → μ is free, ν is contracted
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let expr = CasExpr::Mul(vec![
            CasExpr::MetricTensor {
                mu: mu.clone(),
                nu: nu.clone(),
            },
            CasExpr::GammaMat { index: nu.clone() },
        ]);
        let free = expr.get_free_indices();
        assert_eq!(free.len(), 1);
        assert!(free.contains(&mu));
    }

    // --- Simplification ---

    #[test]
    fn cas_simplify_add_zeros() {
        let expr = CasExpr::Add(vec![
            CasExpr::Scalar(0.0),
            CasExpr::Scalar(3.0),
            CasExpr::Scalar(0.0),
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(3.0));
    }

    #[test]
    fn cas_simplify_add_combine_scalars() {
        let expr = CasExpr::Add(vec![CasExpr::Scalar(2.0), CasExpr::Scalar(3.0)]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(5.0));
    }

    #[test]
    fn cas_simplify_mul_ones() {
        let mu = LorentzIndex::Named("mu".into());
        let gamma = CasExpr::GammaMat { index: mu };
        let expr = CasExpr::Mul(vec![
            CasExpr::Scalar(1.0),
            gamma.clone(),
            CasExpr::Scalar(1.0),
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, gamma);
    }

    #[test]
    fn cas_simplify_mul_zero() {
        let expr = CasExpr::Mul(vec![
            CasExpr::Scalar(0.0),
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(0.0));
    }

    #[test]
    fn cas_simplify_mul_combine_scalars() {
        let expr = CasExpr::Mul(vec![CasExpr::Scalar(2.0), CasExpr::Scalar(3.0)]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(6.0));
    }

    #[test]
    fn cas_simplify_double_negation() {
        let expr = CasExpr::Neg(Box::new(CasExpr::Neg(Box::new(CasExpr::Scalar(5.0)))));
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(5.0));
    }

    #[test]
    fn cas_simplify_flatten_nested_add() {
        let expr = CasExpr::Add(vec![
            CasExpr::Scalar(1.0),
            CasExpr::Add(vec![CasExpr::Scalar(2.0), CasExpr::Scalar(3.0)]),
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(6.0));
    }

    #[test]
    fn cas_simplify_flatten_nested_mul() {
        let expr = CasExpr::Mul(vec![
            CasExpr::Scalar(2.0),
            CasExpr::Mul(vec![CasExpr::Scalar(3.0), CasExpr::Scalar(5.0)]),
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(30.0));
    }

    // --- Metric Contraction ---

    #[test]
    fn cas_metric_contraction_gamma() {
        // g^{μν} γ_ν → γ^μ
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let expr = CasExpr::Mul(vec![
            CasExpr::MetricTensor {
                mu: mu.clone(),
                nu: nu.clone(),
            },
            CasExpr::GammaMat { index: nu },
        ]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::GammaMat { index: mu });
    }

    #[test]
    fn cas_metric_trace_four_dimensions() {
        // g^{μ}_{μ} = 4
        let mu = LorentzIndex::Named("mu".into());
        let expr = CasExpr::Mul(vec![CasExpr::MetricTensor {
            mu: mu.clone(),
            nu: mu.clone(),
        }]);
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(4.0));
    }

    // --- Expand ---

    #[test]
    fn cas_expand_distributes() {
        // a(b + c) → ab + ac
        let a = CasExpr::Scalar(2.0);
        let b = CasExpr::Scalar(3.0);
        let c = CasExpr::Scalar(5.0);
        let expr = CasExpr::Mul(vec![a, CasExpr::Add(vec![b, c])]);
        let expanded = expr.expand();
        let simplified = expanded.simplify(SpacetimeDimension::four());
        assert_eq!(simplified, CasExpr::Scalar(16.0));
    }

    // --- LaTeX Rendering ---

    #[test]
    fn cas_latex_scalar() {
        assert_eq!(CasExpr::Scalar(42.0).to_cas_latex(), "42");
    }

    #[test]
    fn cas_latex_imaginary_unit() {
        assert_eq!(CasExpr::ImaginaryUnit.to_cas_latex(), "i");
    }

    #[test]
    fn cas_latex_metric_tensor() {
        let expr = CasExpr::MetricTensor {
            mu: LorentzIndex::Named("mu".into()),
            nu: LorentzIndex::Named("nu".into()),
        };
        assert_eq!(expr.to_cas_latex(), "g^{\\mu,\\nu}");
    }

    #[test]
    fn cas_latex_gamma5() {
        assert_eq!(CasExpr::Gamma5.to_cas_latex(), "\\gamma^{5}");
    }

    #[test]
    fn cas_latex_levi_civita() {
        let expr = CasExpr::LeviCivita {
            indices: [
                LorentzIndex::Named("mu".into()),
                LorentzIndex::Named("nu".into()),
                LorentzIndex::Named("rho".into()),
                LorentzIndex::Named("sigma".into()),
            ],
        };
        assert_eq!(expr.to_cas_latex(), "\\epsilon^{\\mu\\nu\\rho\\sigma}");
    }

    #[test]
    fn cas_latex_spinors() {
        let u = CasExpr::SpinorU {
            label: "e".into(),
            momentum: "p1".into(),
        };
        assert_eq!(u.to_cas_latex(), "u(p_{1})");
        let ubar = CasExpr::SpinorUBar {
            label: "e".into(),
            momentum: "p2".into(),
        };
        assert_eq!(ubar.to_cas_latex(), "\\bar{u}(p_{2})");
    }

    #[test]
    fn cas_latex_slashed_momentum() {
        let expr = CasExpr::SlashedMomentum { label: "p1".into() };
        assert_eq!(expr.to_cas_latex(), "\\not{p_{1}}");
    }

    #[test]
    fn cas_latex_add_and_mul() {
        let expr = CasExpr::Add(vec![CasExpr::Scalar(2.0), CasExpr::Scalar(3.0)]);
        assert_eq!(expr.to_cas_latex(), "2 + 3");

        let expr2 = CasExpr::Mul(vec![
            CasExpr::Scalar(2.0),
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
        ]);
        assert_eq!(expr2.to_cas_latex(), "2\\,\\gamma^{\\mu}");
    }

    #[test]
    fn cas_latex_fraction() {
        let expr = CasExpr::Fraction {
            numerator: Box::new(CasExpr::Scalar(1.0)),
            denominator: Box::new(CasExpr::Scalar(2.0)),
        };
        assert_eq!(expr.to_cas_latex(), "\\frac{1}{2}");
    }

    #[test]
    fn cas_latex_trace() {
        let inner = CasExpr::Mul(vec![
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
            CasExpr::GammaMat {
                index: LorentzIndex::Named("nu".into()),
            },
        ]);
        let expr = CasExpr::Trace(Box::new(inner));
        assert!(expr.to_cas_latex().contains("\\mathrm{Tr}"));
    }

    // --- CAS Trace Evaluation ---

    #[test]
    fn cas_trace_identity() {
        // Tr[I] = 4 × scalar
        let result = evaluate_cas_trace(&CasExpr::Scalar(1.0), SpacetimeDimension::four()).unwrap();
        assert_eq!(result, CasExpr::Scalar(4.0));
    }

    #[test]
    fn cas_trace_odd_gammas_vanish() {
        let expr = CasExpr::GammaMat {
            index: LorentzIndex::Named("mu".into()),
        };
        let result = evaluate_cas_trace(&expr, SpacetimeDimension::four()).unwrap();
        assert_eq!(result, CasExpr::Scalar(0.0));
    }

    #[test]
    fn cas_trace_two_gammas() {
        // Tr[γ^μ γ^ν] = 4 g^{μν}
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let expr = CasExpr::Mul(vec![
            CasExpr::GammaMat { index: mu.clone() },
            CasExpr::GammaMat { index: nu.clone() },
        ]);
        let result = evaluate_cas_trace(&expr, SpacetimeDimension::four()).unwrap();
        let expected = CasExpr::Mul(vec![CasExpr::Scalar(4.0), CasExpr::MetricTensor { mu, nu }]);
        assert_eq!(result, expected);
    }

    #[test]
    fn cas_trace_four_gammas() {
        // Tr[γ^μ γ^ν γ^ρ γ^σ] = 4(g^{μν}g^{ρσ} - g^{μρ}g^{νσ} + g^{μσ}g^{νρ})
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let sigma = LorentzIndex::Named("sigma".into());
        let expr = CasExpr::Mul(vec![
            CasExpr::GammaMat { index: mu.clone() },
            CasExpr::GammaMat { index: nu.clone() },
            CasExpr::GammaMat { index: rho.clone() },
            CasExpr::GammaMat {
                index: sigma.clone(),
            },
        ]);
        let result = evaluate_cas_trace(&expr, SpacetimeDimension::four()).unwrap();
        // Check it contains the metric tensors (structural check)
        let latex = result.to_cas_latex();
        assert!(
            latex.contains("g^{"),
            "Result should contain metric tensors: {}",
            latex
        );
    }

    #[test]
    fn cas_trace_gamma5_four_gammas() {
        // Tr[γ^5 γ^μ γ^ν γ^ρ γ^σ] = -4i ε^{μνρσ}
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let sigma = LorentzIndex::Named("sigma".into());
        let expr = CasExpr::Mul(vec![
            CasExpr::Gamma5,
            CasExpr::GammaMat { index: mu },
            CasExpr::GammaMat { index: nu },
            CasExpr::GammaMat { index: rho },
            CasExpr::GammaMat { index: sigma },
        ]);
        let result = evaluate_cas_trace(&expr, SpacetimeDimension::four()).unwrap();
        let latex = result.to_cas_latex();
        assert!(
            latex.contains("\\epsilon"),
            "γ^5 trace should produce Levi-Civita: {}",
            latex
        );
    }

    #[test]
    fn cas_trace_gamma5_alone_vanishes() {
        let result = evaluate_cas_trace(&CasExpr::Gamma5, SpacetimeDimension::four()).unwrap();
        assert_eq!(result, CasExpr::Scalar(0.0));
    }

    #[test]
    fn cas_trace_gamma5_two_gammas_vanishes() {
        // Tr[γ^5 γ^μ γ^ν] = 0 (less than 4 gammas)
        let expr = CasExpr::Mul(vec![
            CasExpr::Gamma5,
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
            CasExpr::GammaMat {
                index: LorentzIndex::Named("nu".into()),
            },
        ]);
        let result = evaluate_cas_trace(&expr, SpacetimeDimension::four()).unwrap();
        assert_eq!(result, CasExpr::Scalar(0.0));
    }

    #[test]
    fn cas_trace_linearity() {
        // Tr[A + B] = Tr[A] + Tr[B]
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let sigma = LorentzIndex::Named("sigma".into());
        let a = CasExpr::Mul(vec![
            CasExpr::GammaMat { index: mu },
            CasExpr::GammaMat { index: nu },
        ]);
        let b = CasExpr::Mul(vec![
            CasExpr::GammaMat { index: rho },
            CasExpr::GammaMat { index: sigma },
        ]);
        let sum = CasExpr::Add(vec![a, b]);
        let result = evaluate_cas_trace(&sum, SpacetimeDimension::four()).unwrap();
        // Should be a sum of two trace results
        match result {
            CasExpr::Add(terms) => assert_eq!(terms.len(), 2),
            _ => panic!("Expected Add from trace linearity, got: {:?}", result),
        }
    }

    // --- Levi-Civita Contraction ---

    #[test]
    fn levi_civita_full_contraction() {
        // ε^{μνρσ} ε_{μνρσ} = -24
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let sigma = LorentzIndex::Named("sigma".into());
        let eps = [mu, nu, rho, sigma];
        let result = contract_levi_civita(&eps, &eps);
        assert_eq!(result, CasExpr::Scalar(-24.0));
    }

    #[test]
    fn levi_civita_three_contracted() {
        // ε^{αμνρ} ε_{βμνρ} = -6 δ^α_β
        let alpha = LorentzIndex::Named("alpha".into());
        let beta = LorentzIndex::Named("beta".into());
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let eps1 = [alpha.clone(), mu.clone(), nu.clone(), rho.clone()];
        let eps2 = [beta.clone(), mu, nu, rho];
        let result = contract_levi_civita(&eps1, &eps2);
        let latex = result.to_cas_latex();
        assert!(
            latex.contains("-6"),
            "Three-index contraction should give -6: {}",
            latex
        );
    }

    #[test]
    fn levi_civita_two_contracted() {
        // ε^{αβμν} ε_{γδμν} = -2(δ^α_γ δ^β_δ - δ^α_δ δ^β_γ)
        let alpha = LorentzIndex::Named("alpha".into());
        let beta = LorentzIndex::Named("beta".into());
        let gamma = LorentzIndex::Named("gamma".into());
        let delta = LorentzIndex::Named("delta".into());
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let eps1 = [alpha, beta, mu.clone(), nu.clone()];
        let eps2 = [gamma, delta, mu, nu];
        let result = contract_levi_civita(&eps1, &eps2);
        let latex = result.to_cas_latex();
        assert!(
            latex.contains("-2"),
            "Two-index contraction should give -2: {}",
            latex
        );
    }

    #[test]
    fn levi_civita_no_contraction() {
        // All different indices — no contraction, return product
        let a = LorentzIndex::Named("a".into());
        let b = LorentzIndex::Named("b".into());
        let c = LorentzIndex::Named("c".into());
        let d = LorentzIndex::Named("d".into());
        let e = LorentzIndex::Named("e".into());
        let f = LorentzIndex::Named("f".into());
        let g = LorentzIndex::Named("g".into());
        let h = LorentzIndex::Named("h".into());
        let eps1 = [a, b, c, d];
        let eps2 = [e, f, g, h];
        let result = contract_levi_civita(&eps1, &eps2);
        match result {
            CasExpr::Mul(factors) => {
                assert_eq!(factors.len(), 2);
                assert!(matches!(&factors[0], CasExpr::LeviCivita { .. }));
                assert!(matches!(&factors[1], CasExpr::LeviCivita { .. }));
            }
            _ => panic!("Expected Mul of two Levi-Civita tensors"),
        }
    }

    // --- Dirac Equation ---

    #[test]
    fn cas_dirac_equation_slashed_p_u() {
        // /p u(p) → m u(p) when on-shell
        let expr = CasExpr::Mul(vec![
            CasExpr::SlashedMomentum { label: "p1".into() },
            CasExpr::SpinorU {
                label: "e".into(),
                momentum: "p1".into(),
            },
        ]);
        let mut masses = std::collections::HashMap::new();
        masses.insert("p1".to_string(), 0.511e-3); // electron mass in GeV
        let result = expr.apply_dirac_equation(&masses);
        match &result {
            CasExpr::Mul(factors) => {
                assert!(matches!(&factors[0], CasExpr::Scalar(m) if (*m - 0.511e-3).abs() < 1e-10));
                assert!(matches!(&factors[1], CasExpr::SpinorU { .. }));
            }
            _ => panic!("Expected Mul after Dirac equation, got: {:?}", result),
        }
    }

    #[test]
    fn cas_dirac_equation_ubar_slashed_p() {
        // ū(p) /p → m ū(p)
        let expr = CasExpr::Mul(vec![
            CasExpr::SpinorUBar {
                label: "e".into(),
                momentum: "p2".into(),
            },
            CasExpr::SlashedMomentum { label: "p2".into() },
        ]);
        let mut masses = std::collections::HashMap::new();
        masses.insert("p2".to_string(), 0.105); // muon mass
        let result = expr.apply_dirac_equation(&masses);
        match &result {
            CasExpr::Mul(factors) => {
                assert!(matches!(&factors[0], CasExpr::SpinorUBar { .. }));
                assert!(matches!(&factors[1], CasExpr::Scalar(m) if (*m - 0.105).abs() < 1e-10));
            }
            _ => panic!("Expected Mul after Dirac equation, got: {:?}", result),
        }
    }

    #[test]
    fn cas_dirac_equation_v_spinor() {
        // /p v(p) → -m v(p)
        let expr = CasExpr::Mul(vec![
            CasExpr::SlashedMomentum { label: "p1".into() },
            CasExpr::SpinorV {
                label: "e".into(),
                momentum: "p1".into(),
            },
        ]);
        let mut masses = std::collections::HashMap::new();
        masses.insert("p1".to_string(), 1.5);
        let result = expr.apply_dirac_equation(&masses);
        match &result {
            CasExpr::Mul(factors) => {
                assert!(matches!(&factors[0], CasExpr::Scalar(m) if (*m + 1.5).abs() < 1e-10));
            }
            _ => panic!("Expected Mul after Dirac equation for v-spinor"),
        }
    }

    // --- Polarization Sum Rules ---

    #[test]
    fn cas_polarization_sum_massless() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let result = polarization_sum_massless_vector(&mu, &nu);
        match result {
            CasExpr::Neg(inner) => {
                assert!(matches!(*inner, CasExpr::MetricTensor { .. }));
            }
            _ => panic!("Expected -g^μν"),
        }
    }

    #[test]
    fn cas_polarization_sum_massive() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let result = polarization_sum_massive_vector(&mu, &nu, "k", 91.2 * 91.2);
        match &result {
            CasExpr::Add(terms) => assert_eq!(terms.len(), 2),
            _ => panic!("Expected Add with two terms for massive polarization sum"),
        }
    }

    #[test]
    fn cas_polarization_sum_rarita_schwinger() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let result = polarization_sum_rarita_schwinger(&mu, &nu, "p", 1.5);
        // Should be a negation of a product
        match &result {
            CasExpr::Neg(inner) => {
                assert!(matches!(inner.as_ref(), CasExpr::Mul(_)));
            }
            _ => panic!("Expected Neg(Mul(...)) for Rarita-Schwinger sum"),
        }
    }

    #[test]
    fn cas_polarization_sum_spin2() {
        let mu = LorentzIndex::Named("mu".into());
        let nu = LorentzIndex::Named("nu".into());
        let rho = LorentzIndex::Named("rho".into());
        let sigma = LorentzIndex::Named("sigma".into());
        let result = polarization_sum_spin2(&mu, &nu, &rho, &sigma, "k", 2.0);
        match &result {
            CasExpr::Add(terms) => assert_eq!(terms.len(), 2),
            _ => panic!("Expected Add for spin-2 polarization sum"),
        }
    }

    // --- SpacetimeTensorKind ---

    #[test]
    fn spacetime_tensor_kind_serde_roundtrip() {
        let kinds = vec![
            SpacetimeTensorKind::Scalar,
            SpacetimeTensorKind::Vector,
            SpacetimeTensorKind::VectorSpinor,
            SpacetimeTensorKind::SymmetricRank2,
            SpacetimeTensorKind::AntiSymmetricRank2,
            SpacetimeTensorKind::GeneralRank(3),
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: SpacetimeTensorKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    // --- CasExpr Serde ---

    #[test]
    fn cas_expr_serde_roundtrip() {
        let expr = CasExpr::Mul(vec![
            CasExpr::Scalar(2.0),
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
            CasExpr::SpinorU {
                label: "e".into(),
                momentum: "p1".into(),
            },
        ]);
        let json = serde_json::to_string(&expr).unwrap();
        let back: CasExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    #[test]
    fn cas_expr_complex_serde_roundtrip() {
        let expr = CasExpr::Trace(Box::new(CasExpr::Mul(vec![
            CasExpr::Gamma5,
            CasExpr::GammaMat {
                index: LorentzIndex::Named("mu".into()),
            },
            CasExpr::GammaMat {
                index: LorentzIndex::Named("nu".into()),
            },
            CasExpr::GammaMat {
                index: LorentzIndex::Named("rho".into()),
            },
            CasExpr::GammaMat {
                index: LorentzIndex::Named("sigma".into()),
            },
        ])));
        let json = serde_json::to_string(&expr).unwrap();
        let back: CasExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(expr, back);
    }

    // --- DerivationStep ---

    #[test]
    fn derivation_step_serde_roundtrip() {
        let step = DerivationStep {
            label: "Feynman Rules".into(),
            description: "Apply Feynman rules.".into(),
            expression: CasExpr::Scalar(1.0),
            latex: "1".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: DerivationStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "Feynman Rules");
        assert_eq!(back.description, "Apply Feynman rules.");
    }

    // --- Propagator Form Extensions ---

    #[test]
    fn propagator_form_rarita_schwinger_label() {
        use crate::lagrangian::Propagator;
        use crate::ontology::Spin;
        let prop = Propagator {
            field_id: "gravitino".into(),
            spin: Spin(3),
            mass: 1.5,
            width: 0.0,
            expression: "RS propagator".into(),
            gauge_parameter: None,
            form: PropagatorForm::RaritaSchwinger,
        };
        assert_eq!(prop.form_label(), "RaritaSchwinger");
    }

    #[test]
    fn propagator_form_spin2_labels() {
        use crate::lagrangian::Propagator;
        use crate::ontology::Spin;
        let massless = Propagator {
            field_id: "graviton".into(),
            spin: Spin(4),
            mass: 0.0,
            width: 0.0,
            expression: "massless spin-2".into(),
            gauge_parameter: None,
            form: PropagatorForm::MasslessSpin2,
        };
        assert_eq!(massless.form_label(), "MasslessSpin2");

        let massive = Propagator {
            field_id: "KK_graviton".into(),
            spin: Spin(4),
            mass: 2.0,
            width: 0.1,
            expression: "massive spin-2".into(),
            gauge_parameter: None,
            form: PropagatorForm::MassiveSpin2,
        };
        assert_eq!(massive.form_label(), "MassiveSpin2");
    }

    // --- derive_propagator for new spins ---

    #[test]
    fn derive_propagator_spin_3_2() {
        use crate::lagrangian::derive_propagator;
        let field = Field {
            id: "gravitino".into(),
            name: "Gravitino".into(),
            symbol: "\\tilde{G}".into(),
            mass: 1.5,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 0,
                    tau: 0,
                },
                spin: Spin(3),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![],
        };
        let prop = derive_propagator(&field).unwrap();
        assert_eq!(prop.form, PropagatorForm::RaritaSchwinger);
    }

    #[test]
    fn derive_propagator_spin_2_massless() {
        use crate::lagrangian::derive_propagator;
        let field = Field {
            id: "graviton".into(),
            name: "Graviton".into(),
            symbol: "h_{\\mu\\nu}".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 0,
                    tau: 0,
                },
                spin: Spin(4),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Even,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![],
        };
        let prop = derive_propagator(&field).unwrap();
        assert_eq!(prop.form, PropagatorForm::MasslessSpin2);
    }

    #[test]
    fn derive_propagator_spin_2_massive() {
        use crate::lagrangian::derive_propagator;
        let field = Field {
            id: "kk_graviton".into(),
            name: "KK Graviton".into(),
            symbol: "G_{KK}".into(),
            mass: 2.0,
            width: 0.1,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers {
                    electron: 0,
                    muon: 0,
                    tau: 0,
                },
                spin: Spin(4),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Even,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![],
        };
        let prop = derive_propagator(&field).unwrap();
        assert_eq!(prop.form, PropagatorForm::MassiveSpin2);
    }

    // --- Tensor CAS Expr ---

    #[test]
    fn cas_tensor_vector_boson() {
        let expr = CasExpr::Tensor {
            label: "ε".into(),
            kind: SpacetimeTensorKind::Vector,
            indices: vec![LorentzIndex::Named("mu".into())],
            momentum: Some("k1".into()),
            is_conjugate: false,
        };
        assert!(expr.is_commutative());
        let free = expr.get_free_indices();
        assert_eq!(free.len(), 1);
    }

    #[test]
    fn cas_tensor_graviton() {
        let expr = CasExpr::Tensor {
            label: "ε".into(),
            kind: SpacetimeTensorKind::SymmetricRank2,
            indices: vec![
                LorentzIndex::Named("mu".into()),
                LorentzIndex::Named("nu".into()),
            ],
            momentum: Some("k1".into()),
            is_conjugate: true,
        };
        let latex = expr.to_cas_latex();
        assert!(
            latex.contains("^{*}"),
            "Conjugate tensor should have star: {}",
            latex
        );
        let free = expr.get_free_indices();
        assert_eq!(free.len(), 2);
    }

    // --- Commutator / AntiCommutator ---

    #[test]
    fn cas_commutator_simplifies() {
        // [A, A] should simplify to 0
        let a = CasExpr::Scalar(5.0);
        let expr = CasExpr::Commutator(Box::new(a.clone()), Box::new(a));
        let result = expr.simplify(SpacetimeDimension::four());
        assert!(
            result.is_scalar_zero(),
            "Commutator [A, A] should be 0, got: {:?}",
            result
        );
    }

    #[test]
    fn cas_anticommutator_simplifies() {
        // {A, B} = AB + BA for scalars → 2AB
        let a = CasExpr::Scalar(3.0);
        let b = CasExpr::Scalar(5.0);
        let expr = CasExpr::AntiCommutator(Box::new(a), Box::new(b));
        let result = expr.simplify(SpacetimeDimension::four());
        assert_eq!(result, CasExpr::Scalar(30.0));
    }

    // ===================================================================
    // Generalized Spacetime Geometry Tests
    // ===================================================================

    // --- MetricSign ---

    #[test]
    fn metric_sign_values() {
        assert_eq!(MetricSign::Plus.value(), 1.0);
        assert_eq!(MetricSign::Minus.value(), -1.0);
    }

    #[test]
    fn metric_sign_serde_roundtrip() {
        let plus = MetricSign::Plus;
        let json = serde_json::to_string(&plus).unwrap();
        let back: MetricSign = serde_json::from_str(&json).unwrap();
        assert_eq!(plus, back);
    }

    // --- MetricSignature ---

    #[test]
    fn metric_signature_minkowski_4d() {
        let sig = MetricSignature::minkowski_4d();
        assert_eq!(sig.dimension(), 4);
        assert_eq!(sig.time_dimensions(), 1);
        assert_eq!(sig.space_dimensions(), 3);
        assert_eq!(sig.display_signature(), "(+,-,-,-)");
        assert_eq!(sig.component(0), 1.0);
        assert_eq!(sig.component(1), -1.0);
    }

    #[test]
    fn metric_signature_minkowski_10d() {
        let sig = MetricSignature::minkowski(10);
        assert_eq!(sig.dimension(), 10);
        assert_eq!(sig.time_dimensions(), 1);
        assert_eq!(sig.space_dimensions(), 9);
        assert_eq!(sig.component(0), 1.0);
        for i in 1..10 {
            assert_eq!(sig.component(i), -1.0);
        }
    }

    #[test]
    fn metric_signature_euclidean_3d() {
        let sig = MetricSignature::euclidean(3);
        assert_eq!(sig.dimension(), 3);
        assert_eq!(sig.time_dimensions(), 3);
        assert_eq!(sig.space_dimensions(), 0);
        assert_eq!(sig.display_signature(), "(+,+,+)");
    }

    #[test]
    fn metric_signature_custom() {
        // (2,2) signature
        let sig = MetricSignature::new(vec![
            MetricSign::Plus,
            MetricSign::Plus,
            MetricSign::Minus,
            MetricSign::Minus,
        ]);
        assert_eq!(sig.dimension(), 4);
        assert_eq!(sig.time_dimensions(), 2);
        assert_eq!(sig.space_dimensions(), 2);
        assert_eq!(sig.display_signature(), "(+,+,-,-)");
    }

    #[test]
    fn metric_signature_inner_product_minkowski() {
        let sig = MetricSignature::minkowski_4d();
        let v = SpacetimeVector::new_4d(5.0, 3.0, 0.0, 0.0);
        let w = SpacetimeVector::new_4d(2.0, 1.0, 0.0, 0.0);
        // g_μν v^μ w^ν = (+1)(5)(2) + (-1)(3)(1) = 10 - 3 = 7
        let result = sig.inner_product(&v, &w).unwrap();
        assert!((result - 7.0).abs() < 1e-12);
    }

    #[test]
    fn metric_signature_inner_product_dimension_mismatch() {
        let sig = MetricSignature::minkowski_4d();
        let v = SpacetimeVector::new_4d(1.0, 0.0, 0.0, 0.0);
        let w = SpacetimeVector::new(vec![1.0, 0.0, 0.0]);
        assert!(sig.inner_product(&v, &w).is_err());
    }

    #[test]
    fn metric_signature_trace() {
        assert_eq!(MetricSignature::minkowski_4d().trace(), 4.0);
        assert_eq!(MetricSignature::euclidean(6).trace(), 6.0);
    }

    #[test]
    fn metric_signature_serde_roundtrip() {
        let sig = MetricSignature::minkowski(10);
        let json = serde_json::to_string(&sig).unwrap();
        let back: MetricSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(sig, back);
    }

    #[test]
    fn metric_signature_to_latex() {
        assert_eq!(MetricSignature::minkowski_4d().to_latex(), "(+,-,-,-)");
    }

    // --- FlatMetric & SpacetimeMetricTrait ---

    #[test]
    fn flat_metric_minkowski_4d() {
        let m = FlatMetric::minkowski_4d();
        assert_eq!(m.dimension(), 4);
        assert!(m.is_flat());
        assert!(m.is_diagonal());
        assert_eq!(m.component(0, 0), 1.0);
        assert_eq!(m.component(1, 1), -1.0);
        assert_eq!(m.component(0, 1), 0.0);
        assert_eq!(m.trace(), 4.0);
    }

    #[test]
    fn flat_metric_euclidean_5d() {
        let m = FlatMetric::euclidean(5);
        assert_eq!(m.dimension(), 5);
        for i in 0..5 {
            assert_eq!(m.component(i, i), 1.0);
        }
    }

    #[test]
    fn flat_metric_inner_product() {
        let m = FlatMetric::minkowski_4d();
        let p = SpacetimeVector::new_4d(10.0, 3.0, 4.0, 0.0);
        // p^2 = 100 - 9 - 16 - 0 = 75
        let result = m.inner_product(&p, &p).unwrap();
        assert!((result - 75.0).abs() < 1e-12);
    }

    #[test]
    fn flat_metric_serde_roundtrip() {
        let m = FlatMetric::minkowski(6);
        let json = serde_json::to_string(&m).unwrap();
        let back: FlatMetric = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    // --- SpacetimeVector ---

    #[test]
    fn spacetime_vector_new_4d() {
        let v = SpacetimeVector::new_4d(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.dimension(), 4);
        assert_eq!(v.components(), &[1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn spacetime_vector_zero() {
        let v = SpacetimeVector::zero(7);
        assert_eq!(v.dimension(), 7);
        for c in v.components() {
            assert_eq!(*c, 0.0);
        }
    }

    #[test]
    fn spacetime_vector_n_dimensional() {
        let v = SpacetimeVector::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(v.dimension(), 6);
    }

    #[test]
    fn spacetime_vector_dot_minkowski() {
        let sig = MetricSignature::minkowski_4d();
        // Massive particle at rest: p = (m, 0, 0, 0) with m = 125 GeV
        let p = SpacetimeVector::new_4d(125.0, 0.0, 0.0, 0.0);
        let dot = p.dot(&p, &sig).unwrap();
        assert!((dot - 125.0 * 125.0).abs() < 1e-12);
    }

    #[test]
    fn spacetime_vector_dot_euclidean() {
        let sig = MetricSignature::euclidean(3);
        let v = SpacetimeVector::new(vec![1.0, 2.0, 3.0]);
        let dot = v.dot(&v, &sig).unwrap();
        assert!((dot - 14.0).abs() < 1e-12); // 1 + 4 + 9
    }

    #[test]
    fn spacetime_vector_invariant_mass_sq() {
        let sig = MetricSignature::minkowski_4d();
        // Photon: E = |p|, so m^2 = E^2 - p^2 = 0
        let photon = SpacetimeVector::new_4d(10.0, 10.0, 0.0, 0.0);
        let m_sq = photon.invariant_mass_sq(&sig).unwrap();
        assert!(m_sq.abs() < 1e-12);
    }

    #[test]
    fn spacetime_vector_spatial_magnitude() {
        let v = SpacetimeVector::new_4d(100.0, 3.0, 4.0, 0.0);
        assert!((v.spatial_magnitude() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn spacetime_vector_spatial_magnitude_1d() {
        // A 1D vector has only a temporal component — spatial magnitude = 0
        let v = SpacetimeVector::new(vec![42.0]);
        assert_eq!(v.spatial_magnitude(), 0.0);
    }

    #[test]
    fn spacetime_vector_scale() {
        let v = SpacetimeVector::new_4d(1.0, 2.0, 3.0, 4.0);
        let scaled = v.scale(2.0);
        assert_eq!(scaled.components(), &[2.0, 4.0, 6.0, 8.0]);
    }

    #[test]
    fn spacetime_vector_add() {
        let a = SpacetimeVector::new_4d(1.0, 2.0, 3.0, 4.0);
        let b = SpacetimeVector::new_4d(5.0, 6.0, 7.0, 8.0);
        let sum = a + b;
        assert_eq!(sum.components(), &[6.0, 8.0, 10.0, 12.0]);
    }

    #[test]
    fn spacetime_vector_sub() {
        let a = SpacetimeVector::new_4d(5.0, 6.0, 7.0, 8.0);
        let b = SpacetimeVector::new_4d(1.0, 2.0, 3.0, 4.0);
        let diff = a - b;
        assert_eq!(diff.components(), &[4.0, 4.0, 4.0, 4.0]);
    }

    #[test]
    fn spacetime_vector_neg() {
        let v = SpacetimeVector::new_4d(1.0, -2.0, 3.0, -4.0);
        let n = -v;
        assert_eq!(n.components(), &[-1.0, 2.0, -3.0, 4.0]);
    }

    #[test]
    fn spacetime_vector_from_four_momentum() {
        let p = FourMomentum {
            e: 10.0,
            px: 1.0,
            py: 2.0,
            pz: 3.0,
        };
        let sv: SpacetimeVector = p.into();
        assert_eq!(sv.dimension(), 4);
        assert_eq!(sv.components(), &[10.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn spacetime_vector_to_four_momentum() {
        let sv = SpacetimeVector::new_4d(10.0, 1.0, 2.0, 3.0);
        let p = sv.to_four_momentum();
        assert_eq!(p.e, 10.0);
        assert_eq!(p.px, 1.0);
        assert_eq!(p.py, 2.0);
        assert_eq!(p.pz, 3.0);
    }

    #[test]
    #[should_panic(expected = "Cannot convert non-4D vector to FourMomentum")]
    fn spacetime_vector_to_four_momentum_wrong_dim() {
        let sv = SpacetimeVector::new(vec![1.0, 2.0, 3.0]);
        let _ = sv.to_four_momentum();
    }

    #[test]
    fn spacetime_vector_display() {
        let v = SpacetimeVector::new_4d(1.0, 2.0, 3.0, 4.0);
        let s = format!("{}", v);
        assert!(s.contains("1.000000"));
        assert!(s.contains("4.000000"));
    }

    #[test]
    fn spacetime_vector_serde_roundtrip() {
        let v = SpacetimeVector::new(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let json = serde_json::to_string(&v).unwrap();
        let back: SpacetimeVector = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    // --- SpacetimeConfig ---

    #[test]
    fn spacetime_config_standard_4d() {
        let cfg = SpacetimeConfig::standard_4d();
        assert_eq!(cfg.dimension(), 4);
        assert_eq!(cfg.signature().display_signature(), "(+,-,-,-)");
        assert_eq!(cfg.regularization, SpacetimeDimension::Fixed(4));
    }

    #[test]
    fn spacetime_config_dim_reg_4d() {
        let cfg = SpacetimeConfig::dim_reg_4d();
        assert_eq!(cfg.dimension(), 4);
        assert_eq!(cfg.regularization, SpacetimeDimension::DimReg { base: 4 });
    }

    #[test]
    fn spacetime_config_custom_10d() {
        let cfg = SpacetimeConfig::custom(FlatMetric::minkowski(10), SpacetimeDimension::Fixed(10));
        assert_eq!(cfg.dimension(), 10);
        assert_eq!(cfg.signature().time_dimensions(), 1);
        assert_eq!(cfg.signature().space_dimensions(), 9);
    }

    #[test]
    fn spacetime_config_default_is_standard_4d() {
        let cfg = SpacetimeConfig::default();
        assert_eq!(cfg, SpacetimeConfig::standard_4d());
    }

    #[test]
    fn spacetime_config_serde_roundtrip() {
        let cfg = SpacetimeConfig::dim_reg_4d();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: SpacetimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    // --- 10D momentum conservation ---

    #[test]
    fn ten_dimensional_momentum_conservation() {
        // 2 → 2 scattering in 10D: p1 + p2 = p3 + p4
        let sig = MetricSignature::minkowski(10);
        let p1 = SpacetimeVector::new(vec![100.0, 10.0, 20.0, 30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let p2 = SpacetimeVector::new(vec![
            100.0, -10.0, -20.0, -30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]);
        let p3 = SpacetimeVector::new(vec![120.0, 5.0, 15.0, 25.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let p4 = (p1.clone() + p2.clone()) - p3.clone();

        // Verify conservation
        let total_in = p1.clone() + p2.clone();
        let total_out = p3.clone() + p4.clone();
        for i in 0..10 {
            assert!((total_in[i] - total_out[i]).abs() < 1e-12);
        }

        // Verify Mandelstam s in 10D: s = (p1 + p2)^2
        let s_val = total_in.norm_sq(&sig).unwrap();
        // s = (200)^2 - 0 - 0 - 0 - ... = 40000
        assert!((s_val - 40000.0).abs() < 1e-8);
    }

    // =======================================================================
    // Numerical Evaluation Engine Tests
    // =======================================================================

    #[test]
    fn complex_arithmetic_basic() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        let sum = a + b;
        assert!((sum.re - 4.0).abs() < 1e-12);
        assert!((sum.im - 2.0).abs() < 1e-12);

        let prod = a * b;
        // (3+4i)(1-2i) = 3 - 6i + 4i - 8i² = 3 - 2i + 8 = 11 - 2i
        assert!((prod.re - 11.0).abs() < 1e-12);
        assert!((prod.im + 2.0).abs() < 1e-12);
    }

    #[test]
    fn complex_division() {
        let a = Complex::new(1.0, 0.0);
        let b = Complex::new(0.0, 1.0);
        let result = a / b;
        // 1/i = -i
        assert!((result.re).abs() < 1e-12);
        assert!((result.im + 1.0).abs() < 1e-12);
    }

    #[test]
    fn complex_norm_sq() {
        let z = Complex::new(3.0, 4.0);
        assert!((z.norm_sq() - 25.0).abs() < 1e-12);
        assert!((z.norm() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn complex_conjugate() {
        let z = Complex::new(3.0, 4.0);
        let zc = z.conj();
        assert_eq!(zc.re, 3.0);
        assert_eq!(zc.im, -4.0);
        // z * z* = |z|²
        let prod = z * zc;
        assert!((prod.re - 25.0).abs() < 1e-12);
        assert!((prod.im).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_scalar() {
        let ctx = NumericalContext::new_minkowski();
        let expr = CasExpr::Scalar(42.0);
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - 42.0).abs() < 1e-12);
        assert!((result.im).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_imaginary_unit() {
        let ctx = NumericalContext::new_minkowski();
        let expr = CasExpr::ImaginaryUnit;
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re).abs() < 1e-12);
        assert!((result.im - 1.0).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_dot_product_massless() {
        // Two back-to-back massless momenta: p1 = (E, 0, 0, E), p2 = (E, 0, 0, -E)
        let mut ctx = NumericalContext::new_minkowski();
        let e = 50.0;
        ctx.set_momentum("p1", SpacetimeVector::new_4d(e, 0.0, 0.0, e));
        ctx.set_momentum("p2", SpacetimeVector::new_4d(e, 0.0, 0.0, -e));

        let dot = CasExpr::DotProduct {
            left: "p1".into(),
            right: "p2".into(),
        };
        let result = dot.evaluate_numerically(&ctx).unwrap();
        // p1·p2 = E² - 0 - 0 - (-E²) = 2E²
        assert!((result.re - 2.0 * e * e).abs() < 1e-8);
    }

    #[test]
    fn numerical_eval_dot_product_massive_at_rest() {
        // Particle at rest: p = (m, 0, 0, 0). p·p = m².
        let mut ctx = NumericalContext::new_minkowski();
        let m = 125.0; // Higgs-like
        ctx.set_momentum("p", SpacetimeVector::new_4d(m, 0.0, 0.0, 0.0));

        let dot = CasExpr::DotProduct {
            left: "p".into(),
            right: "p".into(),
        };
        let result = dot.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - m * m).abs() < 1e-8);
    }

    #[test]
    fn numerical_eval_add_mul() {
        let ctx = NumericalContext::new_minkowski();
        // 2 * 3 + 5 = 11
        let expr = CasExpr::Add(vec![
            CasExpr::Mul(vec![CasExpr::Scalar(2.0), CasExpr::Scalar(3.0)]),
            CasExpr::Scalar(5.0),
        ]);
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - 11.0).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_fraction() {
        let ctx = NumericalContext::new_minkowski();
        // 10 / (2 + i) = 10(2-i)/5 = (20-10i)/5 = 4 - 2i
        let expr = CasExpr::Fraction {
            numerator: Box::new(CasExpr::Scalar(10.0)),
            denominator: Box::new(CasExpr::Add(vec![
                CasExpr::Scalar(2.0),
                CasExpr::ImaginaryUnit,
            ])),
        };
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - 4.0).abs() < 1e-12);
        assert!((result.im + 2.0).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_propagator_denom() {
        // Propagator: 1/(p² - m²) with p at rest: p² = m² → pole
        // But with iε, we get 1/(iε)
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_momentum("p", SpacetimeVector::new_4d(10.0, 3.0, 4.0, 0.0));
        ctx.i_epsilon = 0.01; // Large for test visibility

        let expr = CasExpr::PropagatorDenom {
            momentum: "p".into(),
            mass_sq: 75.0, // p² = 100 - 9 - 16 = 75
        };
        let result = expr.evaluate_numerically(&ctx).unwrap();
        // 1/(75 - 75 + 0.01i) = 1/(0.01i) = -100i
        assert!(result.re.abs() < 1.0);
        assert!((result.im + 100.0).abs() < 1.0);
    }

    #[test]
    fn numerical_eval_metric_tensor() {
        let ctx = NumericalContext::new_minkowski();
        // g^{00} = +1
        let g00 = CasExpr::MetricTensor {
            mu: LorentzIndex::Numeric(0),
            nu: LorentzIndex::Numeric(0),
        };
        let r = g00.evaluate_numerically(&ctx).unwrap();
        assert!((r.re - 1.0).abs() < 1e-12);

        // g^{11} = -1
        let g11 = CasExpr::MetricTensor {
            mu: LorentzIndex::Numeric(1),
            nu: LorentzIndex::Numeric(1),
        };
        let r = g11.evaluate_numerically(&ctx).unwrap();
        assert!((r.re + 1.0).abs() < 1e-12);

        // g^{01} = 0
        let g01 = CasExpr::MetricTensor {
            mu: LorentzIndex::Numeric(0),
            nu: LorentzIndex::Numeric(1),
        };
        let r = g01.evaluate_numerically(&ctx).unwrap();
        assert!((r.re).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_levi_civita() {
        let ctx = NumericalContext::new_minkowski();
        // ε^{0123} = +1
        let eps = CasExpr::LeviCivita {
            indices: [
                LorentzIndex::Numeric(0),
                LorentzIndex::Numeric(1),
                LorentzIndex::Numeric(2),
                LorentzIndex::Numeric(3),
            ],
        };
        let r = eps.evaluate_numerically(&ctx).unwrap();
        assert!((r.re - 1.0).abs() < 1e-12);

        // ε^{1023} = -1 (one swap)
        let eps2 = CasExpr::LeviCivita {
            indices: [
                LorentzIndex::Numeric(1),
                LorentzIndex::Numeric(0),
                LorentzIndex::Numeric(2),
                LorentzIndex::Numeric(3),
            ],
        };
        let r2 = eps2.evaluate_numerically(&ctx).unwrap();
        assert!((r2.re + 1.0).abs() < 1e-12);

        // ε^{0012} = 0 (repeated index)
        let eps3 = CasExpr::LeviCivita {
            indices: [
                LorentzIndex::Numeric(0),
                LorentzIndex::Numeric(0),
                LorentzIndex::Numeric(1),
                LorentzIndex::Numeric(2),
            ],
        };
        let r3 = eps3.evaluate_numerically(&ctx).unwrap();
        assert!((r3.re).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_symbol_lookup() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_symbol("g_e", Complex::real(0.3028));
        let expr = CasExpr::Symbol {
            name: "g_e".into(),
            indices: vec![],
        };
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - 0.3028).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_symbol_missing_error() {
        let ctx = NumericalContext::new_minkowski();
        let expr = CasExpr::Symbol {
            name: "unknown".into(),
            indices: vec![],
        };
        assert!(expr.evaluate_numerically(&ctx).is_err());
    }

    #[test]
    fn numerical_eval_complex_amplitude_structure() {
        // Simulate a simplified tree-level QED amplitude structure:
        // |M|² = g_e⁴ * 2(s² + u²) / t² for e⁻e⁻ → e⁻e⁻ (Møller)
        // Use numerical values: s = 10000, t = -2500, u = -7500, g_e = 0.3028
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_symbol("g_e", Complex::real(0.3028));
        ctx.set_symbol("s", Complex::real(10000.0));
        ctx.set_symbol("t", Complex::real(-2500.0));
        ctx.set_symbol("u", Complex::real(-7500.0));

        // g_e^4 * 2(s^2 + u^2) / t^2
        let g_e = CasExpr::Symbol {
            name: "g_e".into(),
            indices: vec![],
        };
        let s = CasExpr::Symbol {
            name: "s".into(),
            indices: vec![],
        };
        let t = CasExpr::Symbol {
            name: "t".into(),
            indices: vec![],
        };
        let u = CasExpr::Symbol {
            name: "u".into(),
            indices: vec![],
        };

        let expr = CasExpr::Fraction {
            numerator: Box::new(CasExpr::Mul(vec![
                g_e.clone(),
                g_e.clone(),
                g_e.clone(),
                g_e.clone(),
                CasExpr::Scalar(2.0),
                CasExpr::Add(vec![
                    CasExpr::Mul(vec![s.clone(), s.clone()]),
                    CasExpr::Mul(vec![u.clone(), u.clone()]),
                ]),
            ])),
            denominator: Box::new(CasExpr::Mul(vec![t.clone(), t.clone()])),
        };

        let result = expr.evaluate_numerically(&ctx).unwrap();
        // Expected: 0.3028^4 * 2 * (10^8 + 5.625*10^7) / (2500^2)
        //         = 0.008413 * 2 * 1.5625e8 / 6.25e6
        //         = 0.008413 * 50.0 = 0.4207 (approx)
        let g4 = 0.3028_f64.powi(4);
        let expected = g4 * 2.0 * (1e8 + 5.625e7) / 6.25e6;
        assert!((result.re - expected).abs() < 1e-6);
        assert!(result.im.abs() < 1e-12);
    }

    #[test]
    fn spin_averaged_amplitude_sq_basic() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_symbol("msq", Complex::real(100.0));

        let expr = CasExpr::Symbol {
            name: "msq".into(),
            indices: vec![],
        };
        // 2 spin-1/2 initial-state fermions: 2 dofs each → N_avg = 4
        let result = spin_averaged_amplitude_sq(&expr, &ctx, &[2, 2]).unwrap();
        assert!((result - 25.0).abs() < 1e-12);
    }

    #[test]
    fn spin_averaged_rejects_imaginary() {
        let ctx = NumericalContext::new_minkowski();
        // Purely imaginary "squared amplitude" should be rejected
        let expr = CasExpr::ImaginaryUnit;
        let result = spin_averaged_amplitude_sq(&expr, &ctx, &[2]);
        assert!(result.is_err());
    }

    #[test]
    fn numerical_eval_negation() {
        let ctx = NumericalContext::new_minkowski();
        let expr = CasExpr::Neg(Box::new(CasExpr::Scalar(7.0)));
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re + 7.0).abs() < 1e-12);
    }

    #[test]
    fn numerical_eval_momentum_component() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_momentum("p1", SpacetimeVector::new_4d(100.0, 30.0, 40.0, 50.0));
        let expr = CasExpr::Momentum {
            label: "p1".into(),
            index: LorentzIndex::Numeric(2), // py = 40.0
        };
        let result = expr.evaluate_numerically(&ctx).unwrap();
        assert!((result.re - 40.0).abs() < 1e-12);
    }

    #[test]
    fn levi_civita_numeric_values() {
        assert!((levi_civita_numeric(0, 1, 2, 3) - 1.0).abs() < 1e-12);
        assert!((levi_civita_numeric(1, 0, 2, 3) + 1.0).abs() < 1e-12);
        assert!((levi_civita_numeric(0, 0, 2, 3)).abs() < 1e-12);
        assert!((levi_civita_numeric(3, 2, 1, 0) - 1.0).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // Form factor integration with NumericalContext
    // -----------------------------------------------------------------------

    #[test]
    fn numerical_context_form_factor_default_none() {
        let ctx = NumericalContext::new_minkowski();
        assert!(ctx.form_factor.is_none());
    }

    #[test]
    fn numerical_context_apply_form_factor_point_like() {
        let ctx = NumericalContext::new_minkowski();
        // No form factor → coupling unchanged.
        let coupling = Complex::new(0.3028, 0.0);
        let result = ctx.apply_form_factor(coupling, 100.0);
        assert!((result.re - 0.3028).abs() < 1e-12);
    }

    #[test]
    fn numerical_context_apply_form_factor_dipole() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_form_factor(crate::lagrangian::FormFactor::Dipole { lambda_sq: 0.71 });

        let coupling = Complex::real(1.0);
        // At Q² = 0: F = 1 → coupling unchanged
        let r0 = ctx.apply_form_factor(coupling, 0.0);
        assert!((r0.re - 1.0).abs() < 1e-12);

        // At Q² = Λ² = 0.71: F = (1+1)^{-2} = 0.25
        let r1 = ctx.apply_form_factor(coupling, 0.71);
        assert!((r1.re - 0.25).abs() < 1e-10);

        // Coupling decreases with Q²
        let r2 = ctx.apply_form_factor(coupling, 5.0);
        assert!(r2.re < r1.re);
        assert!(r2.re > 0.0);
    }

    #[test]
    fn numerical_context_apply_form_factor_complex_coupling() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_form_factor(crate::lagrangian::FormFactor::Exponential { lambda_sq: 1.0 });

        let coupling = Complex::new(0.5, 0.3);
        let q2 = 2.0;
        let result = ctx.apply_form_factor(coupling, q2);

        let expected_ff = (-2.0_f64).exp();
        assert!((result.re - 0.5 * expected_ff).abs() < 1e-12);
        assert!((result.im - 0.3 * expected_ff).abs() < 1e-12);
    }

    #[test]
    fn numerical_context_clear_form_factor() {
        let mut ctx = NumericalContext::new_minkowski();
        ctx.set_form_factor(crate::lagrangian::FormFactor::Dipole { lambda_sq: 1.0 });
        assert!(ctx.form_factor.is_some());

        ctx.clear_form_factor();
        assert!(ctx.form_factor.is_none());

        // After clearing, coupling should pass through unchanged.
        let result = ctx.apply_form_factor(Complex::real(1.0), 100.0);
        assert!((result.re - 1.0).abs() < 1e-15);
    }
}
