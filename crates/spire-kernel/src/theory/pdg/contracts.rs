use serde::{Deserialize, Serialize};

/// Semantic version-like PDG data edition label.
///
/// Examples: `2025-v0.2.2`, `2024-v0.1.4`.
pub type PdgEdition = String;

/// Metadata about the PDG database instance.
///
/// Carries information about the edition, source, and timestamp of the PDG data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdgMetadata {
    /// Dataset edition identifier (e.g. `2025-v0.2.2`).
    pub edition: String,
    /// Version identifier.
    pub version: String,
    /// ISO 8601 timestamp of the database.
    pub timestamp: String,
    /// List of source file paths.
    pub source_files: Vec<String>,
}

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
            | Self::Asymmetric { value, .. } => *value,
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
    /// Dataset release timestamp in ISO 8601 format.
    #[serde(default)]
    pub release_timestamp: Option<String>,
    /// Logical source id selected by arbitration (`local_sqlite`, `pdg_rest`, ...).
    pub source_id: String,
    /// Optional concrete local origin (file path, URL, etc.).
    pub origin: Option<String>,
    /// Normalized local source path used for fingerprinting.
    #[serde(default)]
    pub source_path: Option<String>,
    /// Active extraction policy used for channel/value resolution.
    #[serde(default)]
    pub extraction_policy: Option<String>,
    /// Arbitration outcome for source selection (`local`, `api`, `mixed`, ...).
    #[serde(default)]
    pub source_arbitration_outcome: Option<String>,
    /// Hash of the concrete local PDG file bytes.
    #[serde(default)]
    pub local_file_fingerprint: Option<String>,
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

/// Decay product: either a concrete particle (MCID) or a generic placeholder.
///
/// Generic products represent inclusive or unresolved states that cannot be
/// explicitly enumerated (e.g., "X", "hadrons", "invisible").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PdgDecayProduct {
    /// Concrete particle identified by its PDG Monte Carlo ID.
    Concrete {
        /// PDG Monte Carlo ID (MCID).
        mcid: i32,
    },
    /// Generic or inclusive placeholder.
    Generic {
        /// Description of the generic product (e.g., "X", "hadrons").
        description: String,
    },
}

impl PdgDecayProduct {
    /// Returns true if this is a concrete, samplable particle.
    pub fn is_concrete(&self) -> bool {
        matches!(self, Self::Concrete { .. })
    }

    /// Returns true if this is a generic or inclusive state.
    pub fn is_generic(&self) -> bool {
        matches!(self, Self::Generic { .. })
    }
}

/// A single decay channel of a parent particle.
///
/// Represents one decay mode from parent → products, with multiplicity and branching information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdgDecayChannel {
    /// Internal PDG mode identifier.
    pub mode_id: i32,
    /// Decay products with their multiplicities.
    pub products: Vec<(PdgDecayProduct, u32)>,
    /// Branching ratio or width for this channel (if available).
    pub branching_ratio: Option<PdgValue>,
    /// True if any product is generic (non-concrete).
    pub is_generic: bool,
    /// Human-readable decay description (e.g., "Z0 --> e+ e-").
    pub description: Option<String>,
}

impl PdgDecayChannel {
    /// Returns the branching ratio as a fraction (0.0 to 1.0).
    /// If `is_limit` is true or the value is missing, returns None.
    pub fn branching_fraction(&self) -> Option<f64> {
        self.branching_ratio.as_ref().and_then(|br| {
            if br.is_limit() {
                None
            } else {
                Some(br.central())
            }
        })
    }
}

/// Aggregated decay table for a parent particle.
///
/// Includes all experimentally measured decay modes, both concrete and generic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdgDecayTable {
    /// Parent particle PDG ID.
    pub parent_pdg_id: i32,
    /// All decay channels.
    pub channels: Vec<PdgDecayChannel>,
    /// Edition of the PDG data.
    pub edition: PdgEdition,
}

impl PdgDecayTable {
    /// Returns only the concrete (non-generic) channels.
    pub fn concrete_channels(&self) -> Vec<&PdgDecayChannel> {
        self.channels.iter().filter(|ch| !ch.is_generic).collect()
    }

    /// Returns the sum of branching fractions for concrete channels.
    pub fn concrete_branching_sum(&self) -> f64 {
        self.concrete_channels()
            .iter()
            .filter_map(|ch| ch.branching_fraction())
            .sum()
    }

    /// Returns only the generic channels.
    pub fn generic_channels(&self) -> Vec<&PdgDecayChannel> {
        self.channels.iter().filter(|ch| ch.is_generic).collect()
    }
}
