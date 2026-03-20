use rusqlite::{Connection, OpenFlags};
use std::path::Path;

use crate::{SpireError, SpireResult};

/// Query contract for the PDG SQLite adapter.
///
/// This abstraction isolates raw SQL strings from extraction logic so schema
/// touch-ups can be handled in one place.
pub trait PdgQueryBuilder {
    /// Required tables for schema validation.
    fn required_tables(&self) -> &'static [&'static str];

    /// Resolve a particle row from MCID.
    fn resolve_particle_by_mcid_sql(&self) -> &'static str;

    /// Resolve a particle row from item name or alias.
    fn resolve_particle_by_name_sql(&self) -> &'static str;

    /// Extract a single quantity (`M`, `G`, `T`) for a resolved particle root.
    fn quantity_lookup_sql(&self) -> &'static str;

    /// Extract branching-fraction rows for a resolved particle root.
    fn branching_fraction_lookup_sql(&self) -> &'static str;
}

/// Default query set for the PDG 2024/2025 SQLite schema family.
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardPdgQueryBuilder;

impl PdgQueryBuilder for StandardPdgQueryBuilder {
    fn required_tables(&self) -> &'static [&'static str] {
        &["pdgparticle", "pdgid", "pdgdata", "pdgitem", "pdgitem_map"]
    }

    fn resolve_particle_by_mcid_sql(&self) -> &'static str {
        "
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
        WHERE p.mcid = ?1
        ORDER BY
            CASE p.cc_type WHEN 'P' THEN 0 WHEN 'A' THEN 1 ELSE 2 END,
            p.id ASC
        LIMIT 1
        "
    }

    fn resolve_particle_by_name_sql(&self) -> &'static str {
        "
        WITH candidate_items AS (
            SELECT i.id AS item_id, 0 AS alias_rank, 0 AS alias_sort
            FROM pdgitem i
            WHERE lower(i.name) = lower(?1)

            UNION ALL

            SELECT m.target_id AS item_id, 1 AS alias_rank, m.sort AS alias_sort
            FROM pdgitem_map m
            WHERE lower(m.name) = lower(?1)
        )
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
        FROM candidate_items ci
        JOIN pdgparticle p ON p.pdgitem_id = ci.item_id
        ORDER BY
            ci.alias_rank ASC,
            ci.alias_sort ASC,
            abs(p.mcid) ASC,
            p.mcid ASC,
            p.id ASC
        LIMIT 1
        "
    }

    fn quantity_lookup_sql(&self) -> &'static str {
        "
        SELECT
            d.pdgid,
            d.description,
            d.data_type,
            da.value,
            da.error_positive,
            da.error_negative,
            da.limit_type,
            da.value_type,
            da.unit_text,
            da.display_in_percent
        FROM pdgid root
        JOIN pdgid d ON d.parent_pdgid = root.pdgid
        JOIN pdgdata da ON da.pdgid_id = d.id
        WHERE root.id = ?1
          AND d.data_type = ?2
        ORDER BY
            CAST(COALESCE(da.edition, '0') AS INTEGER) DESC,
            da.in_summary_table DESC,
            CASE lower(COALESCE(da.unit_text, ''))
                WHEN 'gev' THEN 0
                WHEN 'mev' THEN 1
                WHEN 'kev' THEN 2
                WHEN 'ev' THEN 3
                WHEN 'tev' THEN 4
                ELSE 9
            END ASC,
            CASE
                WHEN lower(COALESCE(d.description, '')) LIKE '%atomic mass units%'
                THEN 1 ELSE 0
            END ASC,
            da.sort ASC,
            da.id ASC
        LIMIT 1
        "
    }

    fn branching_fraction_lookup_sql(&self) -> &'static str {
        "
        SELECT
            d.pdgid,
            d.description,
            da.value,
            da.error_positive,
            da.error_negative,
            da.limit_type,
            da.value_type,
            da.unit_text,
            da.display_in_percent
        FROM pdgid root
        JOIN pdgid d ON d.parent_pdgid = root.pdgid
        JOIN pdgdata da ON da.pdgid_id = d.id
        WHERE root.id = ?1
          AND d.data_type = 'BR'
        ORDER BY
            CAST(COALESCE(da.edition, '0') AS INTEGER) DESC,
            da.in_summary_table DESC,
            d.sort ASC,
            da.sort ASC,
            da.id ASC
        "
    }
}

/// Read-only PDG SQLite database handle.
pub struct PdgDatabase {
    connection: Connection,
    query_builder: Box<dyn PdgQueryBuilder + Send + Sync>,
}

impl PdgDatabase {
    /// Open a PDG SQLite database in read-only mode and validate required tables.
    pub fn open(path: &Path) -> SpireResult<Self> {
        Self::open_with_query_builder(path, Box::<StandardPdgQueryBuilder>::default())
    }

    /// Open with an explicit query-builder strategy.
    pub fn open_with_query_builder(
        path: &Path,
        query_builder: Box<dyn PdgQueryBuilder + Send + Sync>,
    ) -> SpireResult<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
        let connection = Connection::open_with_flags(path, flags).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to open PDG database '{}' (read-only): {}",
                path.display(),
                err
            ))
        })?;

        let db = Self {
            connection,
            query_builder,
        };

        db.verify_schema()?;
        Ok(db)
    }

    /// Access the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Access the active query-builder strategy.
    pub fn query_builder(&self) -> &(dyn PdgQueryBuilder + Send + Sync) {
        self.query_builder.as_ref()
    }

    fn verify_schema(&self) -> SpireResult<()> {
        let mut stmt = self
            .connection
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1 LIMIT 1")
            .map_err(|err| SpireError::DatabaseError(format!("Failed to prepare schema probe: {}", err)))?;

        for table in self.query_builder.required_tables() {
            let found = stmt
                .query_row([*table], |row| row.get::<_, i64>(0))
                .map(|_| true)
                .unwrap_or(false);
            if !found {
                return Err(SpireError::DatabaseError(format!(
                    "PDG schema validation failed: required table '{}' not found",
                    table
                )));
            }
        }

        Ok(())
    }
}
