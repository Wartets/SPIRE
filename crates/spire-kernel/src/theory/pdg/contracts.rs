use serde::{Deserialize, Serialize};

/// Semantic version-like PDG data edition label.
///
/// Examples: `2025-v0.2.2`, `2024-v0.1.4`.
pub type PdgEdition = String;

/// Symmetric representation of asymmetric uncertainties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsymmetricError {
    /// Downward uncertainty component.
    pub minus: f64,
    /// Upward uncertainty component.
    pub plus: f64,
}

impl AsymmetricError {
    /// Convenience constructor.
    pub fn new(minus: f64, plus: f64) -> Self {
        Self { minus, plus }
    }
}

/// Canonical PDG value representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PdgValue {
    /// Value without an explicit uncertainty.
    Exact {
        /// Central value.
        value: f64,
        /// True when this value is reported as an upper/lower limit.
        #[serde(default)]
        is_limit: bool,
    },
    /// Value with symmetric uncertainty.
    Symmetric {
        /// Central value.
        value: f64,
        /// One-sigma uncertainty.
        error: f64,
        /// True when this value is reported as an upper/lower limit.
        #[serde(default)]
        is_limit: bool,
    },
    /// Value with asymmetric uncertainty.
    Asymmetric {
        /// Central value.
        value: f64,
        /// Asymmetric one-sigma uncertainty.
        error: AsymmetricError,
        /// True when this value is reported as an upper/lower limit.
        #[serde(default)]
        is_limit: bool,
    },
}

impl PdgValue {
    /// Returns the central numeric value.
    pub fn central(&self) -> f64 {
        match self {
            Self::Exact { value, .. }
            | Self::Symmetric { value, .. }
            | Self::Asymmetric { value, .. } => {
                *value
            }
        }
    }

    /// Returns whether this value is a limit measurement.
    pub fn is_limit(&self) -> bool {
        match self {
            Self::Exact { is_limit, .. }
            | Self::Symmetric { is_limit, .. }
            | Self::Asymmetric { is_limit, .. } => *is_limit,
        }
    }
}

/// Baseline quantum metadata available in PDG particle rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PdgQuantumNumbers {
    /// Electric charge in units of $e$.
    pub charge: Option<f64>,
    /// Total spin $J$ representation text (e.g. `1/2`, `1`).
    pub spin_j: Option<String>,
    /// Intrinsic parity quantum number.
    pub parity: Option<String>,
    /// Charge-conjugation quantum number.
    pub c_parity: Option<String>,
}

/// Branching-fraction-like entry linked to a parent particle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdgBranchingFraction {
    /// Child PDG quantity identifier.
    pub pdgid: String,
    /// Human-readable decay/ratio descriptor.
    pub description: String,
    /// Extracted value for this branch entry.
    pub value: PdgValue,
}

/// Provenance record carried with model state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdgProvenance {
    /// Dataset edition identifier (e.g. `2025-v0.2.2`).
    pub edition: PdgEdition,
    /// Logical source id selected by arbitration (`local_sqlite`, `pdg_rest`, ...).
    pub source_id: String,
    /// Optional concrete local origin (file path, URL, etc.).
    pub origin: Option<String>,
    /// Stable hash/fingerprint for reproducibility.
    pub fingerprint: String,
}

/// Record-level contract for particle mass/width values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdgParticleRecord {
    /// PDG Monte Carlo particle identifier.
    pub pdg_id: i32,
    /// Optional human-readable label.
    pub label: Option<String>,
    /// Mass value and uncertainty representation.
    pub mass: Option<PdgValue>,
    /// Decay width value and uncertainty representation.
    pub width: Option<PdgValue>,
    /// Lifetime value and uncertainty representation.
    pub lifetime: Option<PdgValue>,
    /// Branching-fraction observables linked to this particle.
    #[serde(default)]
    pub branching_fractions: Vec<PdgBranchingFraction>,
    /// Baseline particle quantum metadata.
    #[serde(default)]
    pub quantum_numbers: PdgQuantumNumbers,
    /// Record provenance.
    pub provenance: PdgProvenance,
}
