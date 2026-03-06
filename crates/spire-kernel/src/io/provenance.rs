//! # Data Provenance - Deterministic State Hashing & Reproducibility
//!
//! This module implements a cryptographic provenance engine that guarantees
//! bit-for-bit reproducibility of Monte Carlo integrations by capturing
//! every parameter that influences a computation into a single canonical
//! hash.
//!
//! ## Design
//!
//! Standard Rust `Hash` implementations are **not** deterministic across
//! compilation targets or Rust versions. Instead, we serialize the entire
//! computational state into a **canonical JSON form** (alphabetically
//! sorted keys via `serde_json` with `to_string`) and feed the resulting
//! byte stream into SHA-256 (from the `sha2` crate). This approach is:
//!
//! - **Portable**: The same input produces the same hash on every
//!   platform and Rust toolchain.
//! - **Extensible**: Any `Serialize` type can participate in the
//!   provenance record by embedding it in [`ProvenanceState`].
//! - **Verifiable**: The JSON payload travels alongside the hash in
//!   exported files, so any recipient can independently recompute and
//!   verify the digest.
//!
//! ## Usage
//!
//! ```
//! use spire_kernel::io::provenance::{ProvenanceState, compute_provenance};
//! use spire_kernel::lagrangian::TheoreticalModel;
//!
//! let model = TheoreticalModel::default();
//! let state = ProvenanceState::new(model, None, None, 42);
//! let record = compute_provenance(&state);
//! assert_eq!(record.sha256.len(), 64); // hex-encoded SHA-256
//! ```

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lagrangian::TheoreticalModel;
use crate::s_matrix::Reaction;

// ---------------------------------------------------------------------------
// Provenance State
// ---------------------------------------------------------------------------

/// A snapshot of kinematic configuration parameters that influence
/// Monte Carlo integration results.
///
/// This struct captures the centre-of-mass energy and the number of
/// events requested, both of which are deterministic inputs to the
/// phase-space generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicConfig {
    /// Centre-of-mass energy $\sqrt{s}$ in GeV.
    pub cms_energy: f64,
    /// Number of Monte Carlo events to generate.
    pub num_events: usize,
}

/// The complete deterministic state of a SPIRE computation.
///
/// Every parameter that can influence the numerical output of a Monte
/// Carlo integration, cross-section calculation, or event generation
/// run is captured here. Two [`ProvenanceState`] objects that differ in
/// **any** field will produce different SHA-256 hashes, and two objects
/// with identical field values are guaranteed to produce identical hashes
/// regardless of platform.
///
/// # Canonical Serialization
///
/// The JSON representation produced by `serde_json::to_string` is used
/// as the canonical form. Because `serde_json` emits struct fields in
/// declaration order (which is stable across compilations), and all
/// nested types derive `Serialize`, the output is deterministic.
///
/// # Fields
///
/// - `model` - The full [`TheoreticalModel`] including masses, widths,
///   couplings, vertex factors, propagators, and gauge symmetry.
/// - `reaction` - The scattering process (initial/final states) if one
///   has been configured.
/// - `kinematics` - Centre-of-mass energy and event count.
/// - `seed` - The Monte Carlo random number generator seed.
/// - `version` - The SPIRE version string at the time of computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceState {
    /// The theoretical model in its entirety.
    pub model: TheoreticalModel,
    /// The scattering reaction, if configured.
    pub reaction: Option<Reaction>,
    /// Kinematic configuration (energy, event count).
    pub kinematics: Option<KinematicConfig>,
    /// Monte Carlo random seed.
    pub seed: u64,
    /// SPIRE version at computation time.
    pub version: String,
}

impl ProvenanceState {
    /// Create a new provenance state snapshot.
    pub fn new(
        model: TheoreticalModel,
        reaction: Option<Reaction>,
        kinematics: Option<KinematicConfig>,
        seed: u64,
    ) -> Self {
        Self {
            model,
            reaction,
            kinematics,
            seed,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Provenance Record
// ---------------------------------------------------------------------------

/// The result of hashing a [`ProvenanceState`].
///
/// Contains both the hex-encoded SHA-256 digest and the canonical JSON
/// payload, so that recipients can independently verify the hash by
/// re-serializing and re-hashing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Hex-encoded SHA-256 hash (64 lowercase hex characters).
    pub sha256: String,
    /// The canonical JSON representation of the [`ProvenanceState`].
    pub payload: String,
}

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// Compute the SHA-256 provenance hash for a given state.
///
/// The state is serialized to JSON (canonical form: struct fields in
/// declaration order, no pretty-printing, no trailing whitespace) and
/// then hashed with SHA-256. The result is a [`ProvenanceRecord`]
/// containing both the hex digest and the serialized payload.
///
/// # Determinism Guarantee
///
/// For any two `ProvenanceState` values `a` and `b`:
/// - If `serde_json::to_string(&a) == serde_json::to_string(&b)`,
///   then `compute_provenance(&a).sha256 == compute_provenance(&b).sha256`.
/// - Conversely, if any field differs, the hashes will differ with
///   overwhelming probability ($2^{-256}$ collision chance).
///
/// # Example
///
/// ```
/// use spire_kernel::io::provenance::{ProvenanceState, compute_provenance};
/// use spire_kernel::lagrangian::TheoreticalModel;
///
/// let state = ProvenanceState::new(
///     TheoreticalModel::default(),
///     None,
///     None,
///     12345,
/// );
/// let record = compute_provenance(&state);
/// assert_eq!(record.sha256.len(), 64);
/// ```
pub fn compute_provenance(state: &ProvenanceState) -> ProvenanceRecord {
    let payload = serde_json::to_string(state).expect("ProvenanceState is always serializable");
    let hash = Sha256::digest(payload.as_bytes());
    ProvenanceRecord {
        sha256: hex::encode(hash),
        payload,
    }
}

/// Verify that a [`ProvenanceRecord`] is internally consistent.
///
/// Recomputes the SHA-256 hash from the stored payload and compares it
/// to the stored digest. Returns `true` if they match, `false` if the
/// payload has been tampered with or corrupted.
pub fn verify_provenance(record: &ProvenanceRecord) -> bool {
    let hash = Sha256::digest(record.payload.as_bytes());
    hex::encode(hash) == record.sha256
}

/// Deserialize a [`ProvenanceState`] from a [`ProvenanceRecord`],
/// verifying integrity first.
///
/// Returns `Err` if the hash does not match the payload or if the
/// JSON cannot be deserialized into a valid `ProvenanceState`.
pub fn load_provenance(record: &ProvenanceRecord) -> crate::SpireResult<ProvenanceState> {
    if !verify_provenance(record) {
        return Err(crate::SpireError::InternalError(
            "Provenance hash mismatch: the payload has been modified or corrupted".into(),
        ));
    }
    serde_json::from_str(&record.payload).map_err(|e| {
        crate::SpireError::InternalError(format!("Failed to deserialize provenance state: {}", e))
    })
}

/// Format a provenance record as a human-readable comment block
/// suitable for embedding in exported files (LHE, SLHA, LaTeX).
///
/// The output contains two sections:
/// 1. A single-line SHA-256 hash identifier.
/// 2. The full JSON payload (for independent verification).
///
/// The `prefix` argument specifies the comment character(s) for the
/// target format (e.g., `"#"` for SLHA, `"%"` for LaTeX, `"<!--"` for
/// XML/LHE).
pub fn format_provenance_block(record: &ProvenanceRecord, prefix: &str, suffix: &str) -> String {
    let mut out = String::with_capacity(record.payload.len() + 256);

    out.push_str(&format!(
        "{} SPIRE PROVENANCE HASH: {}{}\n",
        prefix, record.sha256, suffix
    ));
    out.push_str(&format!(
        "{} SPIRE VERSION: {}{}\n",
        prefix,
        env!("CARGO_PKG_VERSION"),
        suffix
    ));
    out.push_str(&format!("{} BEGIN PROVENANCE PAYLOAD{}\n", prefix, suffix));

    // Pretty-print the JSON payload for readability, prefixing each line.
    let pretty = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&record.payload).expect("payload is valid JSON"),
    )
    .expect("re-serialization cannot fail");

    for line in pretty.lines() {
        out.push_str(&format!("{} {}{}\n", prefix, line, suffix));
    }

    out.push_str(&format!("{} END PROVENANCE PAYLOAD{}\n", prefix, suffix));
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrangian::TheoreticalModel;

    /// Helper: create a minimal default model for testing.
    fn test_model() -> TheoreticalModel {
        TheoreticalModel::default()
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let record = compute_provenance(&state);
        assert_eq!(record.sha256.len(), 64);
        assert!(record.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn identical_states_produce_identical_hashes() {
        let s1 = ProvenanceState::new(test_model(), None, None, 42);
        let s2 = ProvenanceState::new(test_model(), None, None, 42);
        let r1 = compute_provenance(&s1);
        let r2 = compute_provenance(&s2);
        assert_eq!(r1.sha256, r2.sha256);
        assert_eq!(r1.payload, r2.payload);
    }

    #[test]
    fn different_seed_produces_different_hash() {
        let s1 = ProvenanceState::new(test_model(), None, None, 42);
        let s2 = ProvenanceState::new(test_model(), None, None, 43);
        let r1 = compute_provenance(&s1);
        let r2 = compute_provenance(&s2);
        assert_ne!(r1.sha256, r2.sha256);
    }

    #[test]
    fn different_model_produces_different_hash() {
        let mut model_a = test_model();
        model_a.name = "Model A".into();
        let mut model_b = test_model();
        model_b.name = "Model B".into();

        let r1 = compute_provenance(&ProvenanceState::new(model_a, None, None, 1));
        let r2 = compute_provenance(&ProvenanceState::new(model_b, None, None, 1));
        assert_ne!(r1.sha256, r2.sha256);
    }

    #[test]
    fn coupling_perturbation_changes_hash() {
        use crate::lagrangian::VertexFactor;

        let mut model_a = test_model();
        model_a.vertex_factors.push(VertexFactor {
            term_id: "v1".into(),
            field_ids: vec!["e".into(), "e".into(), "gamma".into()],
            expression: "e".into(),
            coupling_value: Some(0.3028),
            n_legs: 3,
        });

        let mut model_b = model_a.clone();
        // Perturb the coupling by 0.0001
        model_b.vertex_factors[0].coupling_value = Some(0.3029);

        let r1 = compute_provenance(&ProvenanceState::new(model_a, None, None, 1));
        let r2 = compute_provenance(&ProvenanceState::new(model_b, None, None, 1));
        assert_ne!(r1.sha256, r2.sha256);
    }

    #[test]
    fn kinematics_changes_hash() {
        let kin_a = Some(KinematicConfig {
            cms_energy: 91.1876,
            num_events: 10_000,
        });
        let kin_b = Some(KinematicConfig {
            cms_energy: 91.1877,
            num_events: 10_000,
        });
        let r1 = compute_provenance(&ProvenanceState::new(test_model(), None, kin_a, 1));
        let r2 = compute_provenance(&ProvenanceState::new(test_model(), None, kin_b, 1));
        assert_ne!(r1.sha256, r2.sha256);
    }

    #[test]
    fn verify_provenance_accepts_valid_record() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let record = compute_provenance(&state);
        assert!(verify_provenance(&record));
    }

    #[test]
    fn verify_provenance_rejects_tampered_payload() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let mut record = compute_provenance(&state);
        record.payload = record.payload.replace("42", "99");
        assert!(!verify_provenance(&record));
    }

    #[test]
    fn verify_provenance_rejects_tampered_hash() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let mut record = compute_provenance(&state);
        record.sha256 = "0".repeat(64);
        assert!(!verify_provenance(&record));
    }

    #[test]
    fn load_provenance_roundtrip() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let record = compute_provenance(&state);
        let restored = load_provenance(&record).unwrap();
        assert_eq!(restored.seed, 42);
        assert_eq!(restored.version, state.version);
        assert_eq!(restored.model.name, state.model.name);
    }

    #[test]
    fn load_provenance_rejects_corrupted_record() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let mut record = compute_provenance(&state);
        record.payload = "invalid json".into();
        assert!(load_provenance(&record).is_err());
    }

    #[test]
    fn format_provenance_block_contains_hash() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let record = compute_provenance(&state);
        let block = format_provenance_block(&record, "%", "");
        assert!(block.contains("SPIRE PROVENANCE HASH:"));
        assert!(block.contains(&record.sha256));
        assert!(block.contains("BEGIN PROVENANCE PAYLOAD"));
        assert!(block.contains("END PROVENANCE PAYLOAD"));
    }

    #[test]
    fn format_provenance_block_xml_style() {
        let state = ProvenanceState::new(test_model(), None, None, 1);
        let record = compute_provenance(&state);
        let block = format_provenance_block(&record, "<!--", " -->");
        assert!(block.contains("<!-- SPIRE PROVENANCE HASH:"));
        assert!(block.contains(" -->"));
    }

    #[test]
    fn serialization_roundtrip_preserves_state() {
        let kin = Some(KinematicConfig {
            cms_energy: 13000.0,
            num_events: 100_000,
        });
        let state = ProvenanceState::new(test_model(), None, kin, 0xDEADBEEF);
        let json = serde_json::to_string(&state).unwrap();
        let restored: ProvenanceState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.seed, 0xDEADBEEF);
        assert_eq!(restored.kinematics.as_ref().unwrap().cms_energy, 13000.0);
        assert_eq!(restored.kinematics.as_ref().unwrap().num_events, 100_000);
    }

    #[test]
    fn none_reaction_vs_some_reaction_different_hash() {
        let state_none = ProvenanceState::new(test_model(), None, None, 1);

        // Build a minimal Reaction
        let reaction = Reaction {
            initial: crate::s_matrix::AsymptoticState {
                states: vec![],
                invariant_mass_sq: 0.0,
            },
            final_state: crate::s_matrix::AsymptoticState {
                states: vec![],
                invariant_mass_sq: 0.0,
            },
            is_valid: true,
            violation_diagnostics: vec![],
            interaction_types: vec![],
            mediating_bosons: vec![],
            perturbative_order: 2,
        };
        let state_some = ProvenanceState::new(test_model(), Some(reaction), None, 1);

        let r1 = compute_provenance(&state_none);
        let r2 = compute_provenance(&state_some);
        assert_ne!(r1.sha256, r2.sha256);
    }
}
