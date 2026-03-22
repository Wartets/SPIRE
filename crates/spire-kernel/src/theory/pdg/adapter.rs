use std::sync::{Arc, OnceLock};
use std::time::Instant;

use std::path::Path;

use crate::theory::pdg::cache::PdgCacheSet;
use crate::theory::pdg::contracts::{
    PdgCacheDiagnostics, PdgDecayTable, PdgMetadata, PdgParticleRecord, PdgProvenance,
    PdgQuantumNumbers,
};
use crate::theory::pdg::database::PdgDatabase;
use crate::theory::pdg::decay_query::extract_decay_channels;
use crate::theory::pdg::policy::{apply_policy, PdgExtractionPolicy};
use crate::theory::pdg::resolution::ResolvedParticle;
use crate::SpireResult;

static DEFAULT_PDG_CACHE: OnceLock<Arc<PdgCacheSet>> = OnceLock::new();

fn default_cache_set() -> Arc<PdgCacheSet> {
    DEFAULT_PDG_CACHE
        .get_or_init(|| Arc::new(PdgCacheSet::new_default()))
        .clone()
}

/// High-level kernel adapter for PDG SQLite lookups.
pub struct PdgAdapter {
    database: PdgDatabase,
    provenance: PdgProvenance,
    caches: Arc<PdgCacheSet>,
}

impl PdgAdapter {
    /// Open an adapter against a PDG SQLite database path.
    pub fn open(path: &Path, provenance: PdgProvenance) -> SpireResult<Self> {
        Self::open_with_cache(path, provenance, default_cache_set())
    }

    /// Open an adapter against a PDG SQLite database path with explicit caches.
    pub(crate) fn open_with_cache(
        path: &Path,
        provenance: PdgProvenance,
        caches: Arc<PdgCacheSet>,
    ) -> SpireResult<Self> {
        let database = PdgDatabase::open(path)?;
        Ok(Self {
            database,
            provenance,
            caches,
        })
    }

    /// Resolve and extract particle properties by MCID.
    pub fn lookup_particle_by_mcid(&self, mcid: i32) -> SpireResult<PdgParticleRecord> {
        if let Some(cached) = self.caches.get_particle_record(mcid) {
            return Ok(cached);
        }

        let record = self.timed_db_query(|| {
            let resolved = self.database.resolve_particle_by_mcid(mcid)?;
            self.build_particle_record(resolved)
        })?;

        self.caches.put_particle_record(mcid, record.clone());
        if let Some(label) = &record.label {
            self.caches.put_id_resolution(label, mcid);
        }
        self.caches.put_id_resolution(&mcid.to_string(), mcid);

        Ok(record)
    }

    /// Resolve and extract particle properties by canonical or alias name.
    pub fn lookup_particle_by_name(&self, name: &str) -> SpireResult<PdgParticleRecord> {
        let normalized = name.trim().to_ascii_lowercase();

        if let Some(mcid) = self.caches.get_id_resolution(&normalized) {
            if let Some(cached) = self.caches.get_particle_record(mcid) {
                return Ok(cached);
            }
        }

        let record = self.timed_db_query(|| {
            let resolved = self.database.resolve_particle_by_name(name)?;
            self.build_particle_record(resolved)
        })?;

        self.caches.put_particle_record(record.pdg_id, record.clone());
        self.caches.put_id_resolution(&normalized, record.pdg_id);
        if let Some(label) = &record.label {
            self.caches.put_id_resolution(label, record.pdg_id);
        }

        Ok(record)
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
        let policy_label = match policy {
            PdgExtractionPolicy::StrictPhysical => "StrictPhysical",
            PdgExtractionPolicy::Catalog => "Catalog",
        };

        if let Some(cached) = self.caches.get_decay_table(mcid, policy_label) {
            return Ok(cached);
        }

        // Extract raw decay channels
        let raw_channels =
            self.timed_db_query(|| extract_decay_channels(self.database.connection(), mcid))?;

        // Apply policy filter
        let filtered_channels = apply_policy(raw_channels, policy);

        let table = PdgDecayTable {
            parent_pdg_id: mcid,
            channels: filtered_channels,
            edition: self.provenance.edition.clone(),
        };

        self.caches
            .put_decay_table(mcid, policy_label, table.clone());

        Ok(table)
    }

    /// Get metadata about this PDG database instance.
    pub fn get_metadata(&self) -> SpireResult<PdgMetadata> {
        Ok(PdgMetadata {
            edition: self.provenance.edition.clone(),
            version: "0.2.2".to_string(),
            timestamp: "2025-03-20T00:00:00Z".to_string(),
            source_files: vec![
                self.database.source_path().display().to_string(),
                self.database.active_path().display().to_string(),
            ],
        })
    }

    /// Fetch a paginated chunk of particle records for catalog/search views.
    pub fn search_particle_records_chunk(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> SpireResult<(Vec<PdgParticleRecord>, usize)> {
        let total = self.timed_db_query(|| self.database.count_particles_matching(query))?;
        if limit == 0 || offset >= total {
            return Ok((Vec::new(), total));
        }

        let page =
            self.timed_db_query(|| self.database.resolve_particle_page(query, offset, limit))?;
        let mut out = Vec::with_capacity(page.len());
        for resolved in page {
            if let Some(cached) = self.caches.get_particle_record(resolved.mcid) {
                out.push(cached);
                continue;
            }

            let record = self.timed_db_query(|| self.build_particle_record(resolved.clone()))?;
            self.caches.put_particle_record(record.pdg_id, record.clone());
            if let Some(label) = &record.label {
                self.caches.put_id_resolution(label, record.pdg_id);
            }
            self.caches
                .put_id_resolution(&record.pdg_id.to_string(), record.pdg_id);
            out.push(record);
        }

        Ok((out, total))
    }

    /// Return aggregate PDG cache and DB query diagnostics.
    pub fn get_cache_diagnostics(&self) -> PdgCacheDiagnostics {
        self.caches.diagnostics()
    }

    /// Clear cache entries while keeping diagnostic counters.
    pub fn clear_cache_entries(&self) {
        self.caches.clear();
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

    fn timed_db_query<T>(&self, query: impl FnOnce() -> SpireResult<T>) -> SpireResult<T> {
        let started = Instant::now();
        let result = query();
        self.caches.record_db_query(started.elapsed());
        result
    }

    fn build_particle_record(&self, resolved: ResolvedParticle) -> SpireResult<PdgParticleRecord> {
        let mass = self
            .database
            .extract_core_quantity(resolved.root_pdgid_id, "M")?;
        let width = self
            .database
            .extract_core_quantity(resolved.root_pdgid_id, "G")?;
        let lifetime = self
            .database
            .extract_core_quantity(resolved.root_pdgid_id, "T")?;
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
