use crate::theory::pdg::contracts::PdgParticleRecord;

/// Source priority used by arbitration.
///
/// Lower values are preferred.
pub type SourcePriority = u16;

/// Runtime arbitration mode for optional REST enrichment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrationMode {
    /// Prefer local authoritative sources first.
    PreferLocal,
    /// Prefer API source first with automatic local fallback.
    PreferApi,
}

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
    /// Effective arbitration mode used for this lookup.
    pub mode: ArbitrationMode,
}

/// Resolve the highest-priority record from available sources.
pub fn arbitrate_particle_record(
    sources: &[&dyn PdgDataSource],
    pdg_id: i32,
) -> ArbitrationOutcome {
    arbitrate_particle_record_with_mode(sources, pdg_id, ArbitrationMode::PreferLocal)
}

/// Resolve a source record while honoring the selected arbitration mode.
pub fn arbitrate_particle_record_with_mode(
    sources: &[&dyn PdgDataSource],
    pdg_id: i32,
    mode: ArbitrationMode,
) -> ArbitrationOutcome {
    let mut hits: Vec<(SourcePriority, String, PdgParticleRecord)> = Vec::new();

    for source in sources {
        if let Some(record) = source.particle_record(pdg_id) {
            hits.push((source.priority(), source.source_id().to_string(), record));
        }
    }

    match mode {
        ArbitrationMode::PreferLocal => hits.sort_by_key(|(priority, _, _)| *priority),
        ArbitrationMode::PreferApi => hits.sort_by_key(|(priority, source_id, _)| {
            let rest_bias: SourcePriority = if source_id == "pdg_rest" { 0 } else { 1 };
            (rest_bias, *priority)
        }),
    }

    let candidates = hits.iter().map(|(_, src, _)| src.clone()).collect();
    let selected = hits.into_iter().next().map(|(_, _, record)| record);

    ArbitrationOutcome {
        selected,
        candidates,
        mode,
    }
}
