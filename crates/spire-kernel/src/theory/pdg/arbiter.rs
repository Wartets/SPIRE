use crate::theory::pdg::contracts::PdgParticleRecord;

/// Source priority used by arbitration.
///
/// Lower values are preferred.
pub type SourcePriority = u16;

/// Abstract PDG data source interface.
pub trait PdgDataSource {
    /// Stable source identifier (`local_sqlite`, `local_text`, `pdg_rest`, ...).
    fn source_id(&self) -> &str;

    /// Source precedence: lower is stronger.
    fn priority(&self) -> SourcePriority;

    /// Retrieve a mass/width record for `pdg_id` if available.
    fn particle_record(&self, pdg_id: i32) -> Option<PdgParticleRecord>;
}

/// Arbitration decision details.
#[derive(Debug, Clone, PartialEq)]
pub struct ArbitrationOutcome {
    /// Winning record, if any source had one.
    pub selected: Option<PdgParticleRecord>,
    /// Source ids that provided candidate records.
    pub candidates: Vec<String>,
}

/// Resolve the highest-priority record from available sources.
pub fn arbitrate_particle_record(
    sources: &[&dyn PdgDataSource],
    pdg_id: i32,
) -> ArbitrationOutcome {
    let mut hits: Vec<(SourcePriority, String, PdgParticleRecord)> = Vec::new();

    for source in sources {
        if let Some(record) = source.particle_record(pdg_id) {
            hits.push((source.priority(), source.source_id().to_string(), record));
        }
    }

    hits.sort_by_key(|(priority, _, _)| *priority);

    let candidates = hits.iter().map(|(_, src, _)| src.clone()).collect();
    let selected = hits.into_iter().next().map(|(_, _, record)| record);

    ArbitrationOutcome {
        selected,
        candidates,
    }
}
