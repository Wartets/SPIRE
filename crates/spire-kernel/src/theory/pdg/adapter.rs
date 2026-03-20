use std::path::Path;

use crate::theory::pdg::contracts::{
    PdgParticleRecord, PdgProvenance, PdgQuantumNumbers,
};
use crate::theory::pdg::database::PdgDatabase;
use crate::theory::pdg::resolution::ResolvedParticle;
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
