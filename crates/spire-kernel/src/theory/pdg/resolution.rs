use rusqlite::OptionalExtension;

use crate::theory::pdg::database::PdgDatabase;
use crate::{SpireError, SpireResult};

/// Deterministic resolution output for a particle lookup.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedParticle {
    /// Root PDG quantity id (`pdgid.id`) used for downstream joins.
    pub root_pdgid_id: i64,
    /// Root PDG key (`pdgid`, e.g. `S003`).
    pub root_pdgid: String,
    /// Resolved Monte Carlo id.
    pub mcid: i32,
    /// Canonical particle label.
    pub name: String,
    /// Charge-conjugation class from PDG (`P`, `A`, `S`, ...).
    pub cc_type: String,
    /// Electric charge in units of $e$.
    pub charge: Option<f64>,
    /// Total spin representation text.
    pub quantum_j: Option<String>,
    /// Parity text.
    pub quantum_p: Option<String>,
    /// C-parity text.
    pub quantum_c: Option<String>,
}

impl PdgDatabase {
    /// Resolve a particle deterministically by MCID.
    pub fn resolve_particle_by_mcid(&self, mcid: i32) -> SpireResult<ResolvedParticle> {
        let sql = self.query_builder().resolve_particle_by_mcid_sql();
        let mut stmt = self.connection().prepare(sql).map_err(|err| {
            SpireError::DatabaseError(format!("Failed to prepare MCID resolution query: {}", err))
        })?;

        let direct = stmt
            .query_row([mcid], |row| {
                Ok(ResolvedParticle {
                    root_pdgid_id: row.get(0)?,
                    root_pdgid: row.get(1)?,
                    mcid: row.get(2)?,
                    name: row.get(3)?,
                    cc_type: row.get(4)?,
                    charge: row.get(5)?,
                    quantum_j: row.get(6)?,
                    quantum_p: row.get(7)?,
                    quantum_c: row.get(8)?,
                })
            })
            .optional()
            .map_err(|err| {
                SpireError::DatabaseError(format!("MCID resolution query failed for {}: {}", mcid, err))
            })?;

        if let Some(found) = direct {
            return Ok(found);
        }

        let fallback_sql = "
            SELECT
                p.pdgid_id,
                p.pdgid,
                p.mcid,
                p.name,
                p.cc_type,
                p.charge,
                p.quantum_j,
                p.quantum_p,
                p.quantum_c
            FROM pdgparticle p
            WHERE abs(p.mcid) = abs(?1)
            ORDER BY
                CASE
                    WHEN ?1 < 0 AND p.mcid < 0 THEN 0
                    WHEN ?1 > 0 AND p.mcid > 0 THEN 0
                    ELSE 1
                END,
                abs(p.mcid) ASC,
                p.id ASC
            LIMIT 1
        ";

        let mut fallback_stmt = self.connection().prepare(fallback_sql).map_err(|err| {
            SpireError::DatabaseError(format!("Failed to prepare MCID fallback query: {}", err))
        })?;

        fallback_stmt
            .query_row([mcid], |row| {
                Ok(ResolvedParticle {
                    root_pdgid_id: row.get(0)?,
                    root_pdgid: row.get(1)?,
                    mcid: row.get(2)?,
                    name: row.get(3)?,
                    cc_type: row.get(4)?,
                    charge: row.get(5)?,
                    quantum_j: row.get(6)?,
                    quantum_p: row.get(7)?,
                    quantum_c: row.get(8)?,
                })
            })
            .optional()
            .map_err(|err| {
                SpireError::DatabaseError(format!(
                    "MCID fallback resolution query failed for {}: {}",
                    mcid, err
                ))
            })?
            .ok_or_else(|| {
                SpireError::UnknownParticle(format!(
                    "No PDG particle entry found for MCID {}",
                    mcid
                ))
            })
    }

    /// Resolve a particle by canonical or alias name.
    pub fn resolve_particle_by_name(&self, name: &str) -> SpireResult<ResolvedParticle> {
        let sql = self.query_builder().resolve_particle_by_name_sql();
        let mut stmt = self.connection().prepare(sql).map_err(|err| {
            SpireError::DatabaseError(format!("Failed to prepare name resolution query: {}", err))
        })?;

        stmt.query_row([name], |row| {
            Ok(ResolvedParticle {
                root_pdgid_id: row.get(0)?,
                root_pdgid: row.get(1)?,
                mcid: row.get(2)?,
                name: row.get(3)?,
                cc_type: row.get(4)?,
                charge: row.get(5)?,
                quantum_j: row.get(6)?,
                quantum_p: row.get(7)?,
                quantum_c: row.get(8)?,
            })
        })
        .optional()
        .map_err(|err| {
            SpireError::DatabaseError(format!(
                "Name resolution query failed for '{}': {}",
                name, err
            ))
        })?
        .ok_or_else(|| {
            SpireError::UnknownParticle(format!(
                "No PDG particle entry found for name '{}'",
                name
            ))
        })
    }
}
