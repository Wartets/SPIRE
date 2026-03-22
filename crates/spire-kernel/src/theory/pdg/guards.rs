use crate::lagrangian::TheoreticalModel;
use crate::theory::pdg::contracts::PdgProvenance;
use crate::{SpireError, SpireResult};

/// Controls behavior when incoming PDG provenance edition differs from model lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditionMismatchPolicy {
    /// Reject mixed-edition updates.
    Strict,
    /// Permit mixed-edition updates and replace lock with incoming provenance.
    AllowOverride,
}

/// Validate and apply model-level PDG provenance lock.
///
/// - If no lock exists, incoming provenance becomes the lock.
/// - If lock edition matches incoming, provenance is updated.
/// - If editions differ, behavior depends on `policy`.
pub fn enforce_model_edition_lock(
    model: &mut TheoreticalModel,
    incoming: PdgProvenance,
    policy: EditionMismatchPolicy,
) -> SpireResult<()> {
    if let Some(existing) = &model.pdg_provenance {
        if existing.edition != incoming.edition && policy == EditionMismatchPolicy::Strict {
            return Err(SpireError::EditionMismatch {
                locked_edition: existing.edition.clone(),
                incoming_edition: incoming.edition,
                source_id: incoming.source_id,
            });
        }
    }

    model.pdg_provenance = Some(incoming);
    Ok(())
}
