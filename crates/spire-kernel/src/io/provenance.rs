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

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::lagrangian::TheoreticalModel;
use crate::s_matrix::Reaction;
use crate::theory::pdg::contracts::PdgProvenance;
use crate::{SpireError, SpireResult};

#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicConfig {
    pub cms_energy: f64,
    pub num_events: usize,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionPdgProvenance {
    pub edition: String,
    pub release_timestamp: Option<String>,
    pub source_id: String,
    pub origin: Option<String>,
    pub source_path: Option<String>,
    pub extraction_policy: Option<String>,
    pub source_arbitration_outcome: Option<String>,
    pub local_file_fingerprint: Option<String>,
    pub fingerprint: String,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputationalState {
    pub model: TheoreticalModel,
    pub reaction: Option<Reaction>,
    pub kinematics: Option<KinematicConfig>,
    pub seed: u64,
    pub version: String,
    #[serde(default)]
    pub pdg_provenance: Option<SessionPdgProvenance>,
}

#[allow(missing_docs)]
pub type ProvenanceState = ComputationalState;

#[allow(missing_docs)]
impl ComputationalState {
    pub fn new(
        model: TheoreticalModel,
        reaction: Option<Reaction>,
        kinematics: Option<KinematicConfig>,
        seed: u64,
    ) -> Self {
        let pdg_provenance = model.pdg_provenance.as_ref().map(from_model_pdg_provenance);
        Self {
            model,
            reaction,
            kinematics,
            seed,
            version: env!("CARGO_PKG_VERSION").to_string(),
            pdg_provenance,
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub sha256: String,
    pub payload: String,
}

#[allow(missing_docs)]
pub trait ProvenanceProvider {
    fn enrich_state(&self, state: &mut ComputationalState) -> SpireResult<()>;
}

#[allow(missing_docs)]
pub trait StateHasher {
    fn algorithm(&self) -> &'static str;
    fn hash_bytes(&self, bytes: &[u8]) -> String;
}

#[allow(missing_docs)]
pub struct Sha256StateHasher;

impl StateHasher for Sha256StateHasher {
    fn algorithm(&self) -> &'static str {
        "sha256"
    }

    fn hash_bytes(&self, bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct PdgFilesystemProvenanceProvider {
    search_roots: Vec<PathBuf>,
}

impl Default for PdgFilesystemProvenanceProvider {
    fn default() -> Self {
        Self {
            search_roots: vec![PathBuf::from("data/pdg"), PathBuf::from("data")],
        }
    }
}

impl PdgFilesystemProvenanceProvider {
    fn enrich_fingerprint(&self, pdg: &mut SessionPdgProvenance) -> SpireResult<()> {
        let source = pdg
            .source_path
            .clone()
            .or_else(|| pdg.origin.clone())
            .or_else(|| {
                resolve_pdg_sqlite_path(&self.search_roots, Some(&pdg.edition))
                    .map(|p| p.to_string_lossy().to_string())
            })
            .or_else(|| {
                resolve_pdg_sqlite_path(&self.search_roots, None)
                    .map(|p| p.to_string_lossy().to_string())
            });

        let Some(source_path) = source else {
            return Ok(());
        };

        let normalized = normalize_path(&source_path);
        if !normalized.exists() {
            return Ok(());
        }

        let local_file_fingerprint = compute_file_sha256(&normalized)?;
        pdg.source_path = Some(normalized.to_string_lossy().to_string());
        pdg.local_file_fingerprint = Some(local_file_fingerprint.clone());

        if pdg.fingerprint.trim().is_empty() {
            pdg.fingerprint = local_file_fingerprint;
        }

        Ok(())
    }
}

impl ProvenanceProvider for PdgFilesystemProvenanceProvider {
    fn enrich_state(&self, state: &mut ComputationalState) -> SpireResult<()> {
        if state.pdg_provenance.is_none() {
            state.pdg_provenance = state
                .model
                .pdg_provenance
                .as_ref()
                .map(from_model_pdg_provenance);
        }

        if let Some(ref mut pdg) = state.pdg_provenance {
            self.enrich_fingerprint(pdg)?;
        }

        Ok(())
    }
}

#[allow(missing_docs)]
pub fn compute_provenance_checked(state: &ComputationalState) -> SpireResult<ProvenanceRecord> {
    let provider = PdgFilesystemProvenanceProvider::default();
    let hasher = Sha256StateHasher;
    compute_provenance_with(state, &provider, &hasher)
}

#[allow(missing_docs)]
pub fn compute_provenance_with(
    state: &ComputationalState,
    provider: &dyn ProvenanceProvider,
    hasher: &dyn StateHasher,
) -> SpireResult<ProvenanceRecord> {
    let mut enriched = state.clone();
    provider.enrich_state(&mut enriched)?;
    let payload = canonical_json_string(&enriched)?;
    Ok(ProvenanceRecord {
        sha256: hasher.hash_bytes(payload.as_bytes()),
        payload,
    })
}

#[allow(missing_docs)]
pub fn compute_provenance(state: &ComputationalState) -> ProvenanceRecord {
    compute_provenance_checked(state).unwrap_or_else(|e| {
        let payload = format!(
            "{{\"error\":\"{}\",\"version\":\"{}\"}}",
            e,
            env!("CARGO_PKG_VERSION")
        );
        ProvenanceRecord {
            sha256: hex::encode(Sha256::digest(payload.as_bytes())),
            payload,
        }
    })
}

#[allow(missing_docs)]
pub fn verify_provenance(record: &ProvenanceRecord) -> bool {
    let hash = Sha256::digest(record.payload.as_bytes());
    hex::encode(hash) == record.sha256
}

#[allow(missing_docs)]
pub fn load_provenance(record: &ProvenanceRecord) -> SpireResult<ComputationalState> {
    if !verify_provenance(record) {
        return Err(SpireError::InternalError(
            "Provenance hash mismatch: the payload has been modified or corrupted".into(),
        ));
    }

    let state: ComputationalState = serde_json::from_str(&record.payload).map_err(|e| {
        SpireError::InternalError(format!("Failed to deserialize provenance state: {e}"))
    })?;

    if let Some(saved) = &state.pdg_provenance {
        let local = crate::session::probe_local_pdg_environment(Some(&saved.edition))?;
        crate::session::validate_session_provenance(saved, &local)?;
    }

    Ok(state)
}

#[allow(missing_docs)]
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

    if let Ok(state) = serde_json::from_str::<ComputationalState>(&record.payload) {
        if let Some(pdg) = state.pdg_provenance {
            out.push_str(&format!(
                "{} PDG EDITION: {}{}\n",
                prefix, pdg.edition, suffix
            ));
            out.push_str(&format!(
                "{} PDG SOURCE ID: {}{}\n",
                prefix, pdg.source_id, suffix
            ));
            out.push_str(&format!(
                "{} PDG EXTRACTION POLICY: {}{}\n",
                prefix,
                pdg.extraction_policy
                    .unwrap_or_else(|| "unspecified".to_string()),
                suffix
            ));
            out.push_str(&format!(
                "{} PDG LOCAL FINGERPRINT: {}{}\n",
                prefix,
                pdg.local_file_fingerprint
                    .unwrap_or_else(|| "unavailable".to_string()),
                suffix
            ));
        }
    }

    out.push_str(&format!("{} BEGIN PROVENANCE PAYLOAD{}\n", prefix, suffix));
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

#[allow(missing_docs)]
pub fn compute_file_sha256(path: &Path) -> SpireResult<String> {
    let bytes = fs::read(path).map_err(|e| {
        SpireError::InternalError(format!(
            "Failed to read PDG source '{}': {e}",
            path.display()
        ))
    })?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

#[allow(missing_docs)]
pub fn resolve_pdg_sqlite_path(
    search_roots: &[PathBuf],
    preferred_edition: Option<&str>,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    for root in search_roots {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("sqlite") {
                    continue;
                }
                let lower = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_lowercase();
                if lower.contains("pdg") {
                    candidates.push(path);
                }
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort();

    if let Some(edition) = preferred_edition {
        let needle = edition.to_lowercase().replace(' ', "-");
        if let Some(hit) = candidates
            .iter()
            .rev()
            .find(|p| p.to_string_lossy().to_lowercase().contains(&needle))
        {
            return Some(hit.clone());
        }
    }

    candidates.pop()
}

fn canonical_json_string<T: Serialize>(value: &T) -> SpireResult<String> {
    let json = serde_json::to_value(value)
        .map_err(|e| SpireError::InternalError(format!("Canonical serialization failed: {e}")))?;
    let canonical = canonicalize_value(json);
    serde_json::to_string(&canonical)
        .map_err(|e| SpireError::InternalError(format!("Canonical JSON encoding failed: {e}")))
}

fn canonicalize_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize_value).collect())
        }
        serde_json::Value::Object(map) => {
            let mut ordered = BTreeMap::new();
            for (k, v) in map {
                ordered.insert(k, canonicalize_value(v));
            }
            let mut out = serde_json::Map::new();
            for (k, v) in ordered {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        scalar => scalar,
    }
}

fn from_model_pdg_provenance(value: &PdgProvenance) -> SessionPdgProvenance {
    SessionPdgProvenance {
        edition: value.edition.clone(),
        release_timestamp: value.release_timestamp.clone(),
        source_id: value.source_id.clone(),
        origin: value.origin.clone(),
        source_path: value.source_path.clone(),
        extraction_policy: value.extraction_policy.clone(),
        source_arbitration_outcome: value.source_arbitration_outcome.clone(),
        local_file_fingerprint: value.local_file_fingerprint.clone(),
        fingerprint: value.fingerprint.clone(),
    }
}

fn normalize_path(input: &str) -> PathBuf {
    let candidate = PathBuf::from(input);
    if candidate.is_absolute() {
        candidate
    } else {
        PathBuf::from(".").join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrangian::TheoreticalModel;

    fn test_model() -> TheoreticalModel {
        TheoreticalModel::default()
    }

    #[test]
    fn hash_is_64_hex_chars() {
        let state = ProvenanceState::new(test_model(), None, None, 42);
        let record = compute_provenance(&state);
        assert_eq!(record.sha256.len(), 64);
    }

    #[test]
    fn identical_state_has_identical_hash() {
        let a = ProvenanceState::new(test_model(), None, None, 42);
        let b = ProvenanceState::new(test_model(), None, None, 42);
        let ra = compute_provenance(&a);
        let rb = compute_provenance(&b);
        assert_eq!(ra.sha256, rb.sha256);
        assert_eq!(ra.payload, rb.payload);
    }

    #[test]
    fn canonicalization_sorts_map_keys() {
        let value = serde_json::json!({"b": 2, "a": {"d": 4, "c": 3}});
        let encoded = serde_json::to_string(&canonicalize_value(value)).unwrap();
        assert_eq!(encoded, "{\"a\":{\"c\":3,\"d\":4},\"b\":2}");
    }

    #[test]
    fn format_block_contains_hash_and_payload() {
        let state = ProvenanceState::new(test_model(), None, None, 7);
        let record = compute_provenance(&state);
        let block = format_provenance_block(&record, "%", "");
        assert!(block.contains("SPIRE PROVENANCE HASH:"));
        assert!(block.contains("BEGIN PROVENANCE PAYLOAD"));
        assert!(block.contains("END PROVENANCE PAYLOAD"));
    }
}
