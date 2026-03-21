use std::path::Path;

use crate::theory::pdg::contracts::{
    PdgParticleRecord, PdgProvenance, PdgQuantumNumbers, PdgDecayTable, PdgMetadata,
};
use crate::theory::pdg::database::PdgDatabase;
use crate::theory::pdg::resolution::ResolvedParticle;
use crate::theory::pdg::decay_query::extract_decay_channels;
use crate::theory::pdg::policy::{PdgExtractionPolicy, apply_policy};
use crate::SpireResult;

/// High-level kernel adapter for PDG SQLite lookups.
pub struct PdgAdapter {
    database: PdgDatabase,
    provenance: PdgProvenance,
}

impl PdgAdapter {
    /// Open an adapter against a PDG SQLite database path.
    pub fn open(path: &Path, provenance: PdgProvenance) -> SpireResult<Self> {
        let database = PdgDatabase::open(path)?;
        Ok(Self {
            database,
            provenance,
        })
    }

    /// Resolve and extract particle properties by MCID.
    pub fn lookup_particle_by_mcid(&self, mcid: i32) -> SpireResult<PdgParticleRecord> {
        let resolved = self.database.resolve_particle_by_mcid(mcid)?;
        self.build_particle_record(resolved)
    }

    /// Resolve and extract particle properties by canonical or alias name.
    pub fn lookup_particle_by_name(&self, name: &str) -> SpireResult<PdgParticleRecord> {
        let resolved = self.database.resolve_particle_by_name(name)?;
        self.build_particle_record(resolved)
    }

    /// Canonical Phase-71 API required by the integration roadmap.
    pub fn get_particle_properties(&self, mcid: i32) -> SpireResult<PdgParticleRecord> {
        self.lookup_particle_by_mcid(mcid)
    }

    /// Extract decay table for a parent particle.
    ///
    /// Returns all decay channels, filtered according to the specified policy.
    /// `StrictPhysical` returns only concrete, Monte-Carlo-samplable channels.
    /// `Catalog` returns all channels including generic placeholders.
    pub fn get_decay_table(
        &self,
        mcid: i32,
        policy: PdgExtractionPolicy,
    ) -> SpireResult<PdgDecayTable> {
        // Extract raw decay channels
        let raw_channels = extract_decay_channels(self.database.connection(), mcid)?;

        // Apply policy filter
        let filtered_channels = apply_policy(raw_channels, policy);

        Ok(PdgDecayTable {
            parent_pdg_id: mcid,
            channels: filtered_channels,
            edition: self.provenance.edition.clone(),
        })
    }

    /// Get metadata about this PDG database instance.
    pub fn get_metadata(&self) -> SpireResult<PdgMetadata> {
        Ok(PdgMetadata {
            edition: self.provenance.edition.clone(),
            version: "0.2.2".to_string(),
            timestamp: "2025-03-20T00:00:00Z".to_string(),
            source_files: self.provenance.origin.iter().cloned().collect(),
        })
    }

    /// Convenience constructor using the default PDG database path.
    ///
    /// Looks for the database in `data/pdg-2025-v0.2.2.sqlite`.
    pub fn with_default_path() -> SpireResult<Self> {
        let path = std::path::Path::new("data/pdg-2025-v0.2.2.sqlite");
        let provenance = PdgProvenance {
            edition: "PDG 2025".to_string(),
            release_timestamp: Some("2025-03-20T00:00:00Z".to_string()),
            source_id: "local_sqlite".to_string(),
            origin: Some(path.to_string_lossy().to_string()),
            source_path: Some(path.to_string_lossy().to_string()),
            extraction_policy: Some("Catalog".to_string()),
            source_arbitration_outcome: Some("local".to_string()),
            local_file_fingerprint: None,
            fingerprint: "v0.2.2".to_string(),
        };
        Self::open(path, provenance)
    }

    fn build_particle_record(&self, resolved: ResolvedParticle) -> SpireResult<PdgParticleRecord> {
        let mass = self.database.extract_core_quantity(resolved.root_pdgid_id, "M")?;
        let width = self.database.extract_core_quantity(resolved.root_pdgid_id, "G")?;
        let lifetime = self.database.extract_core_quantity(resolved.root_pdgid_id, "T")?;
        let branching_fractions = self
            .database
            .extract_branching_fractions(resolved.root_pdgid_id)?;

        Ok(PdgParticleRecord {
            pdg_id: resolved.mcid,
            label: Some(resolved.name),
            mass,
            width,
            lifetime,
            branching_fractions,
            quantum_numbers: PdgQuantumNumbers {
                charge: resolved.charge,
                spin_j: resolved.quantum_j,
                parity: resolved.quantum_p,
                c_parity: resolved.quantum_c,
            },
            provenance: self.provenance.clone(),
        })
    }
}
