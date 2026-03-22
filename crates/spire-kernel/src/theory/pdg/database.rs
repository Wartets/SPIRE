use rusqlite::{Connection, OpenFlags};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

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

    /// Count particle rows matching a search query.
    fn count_particle_rows_sql(&self) -> &'static str;

    /// Fetch particle rows matching a search query with pagination.
    fn search_particle_rows_sql(&self) -> &'static str;
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

    fn count_particle_rows_sql(&self) -> &'static str {
        "
        SELECT COUNT(*)
        FROM pdgparticle p
        WHERE (?1 = '')
           OR lower(p.name) LIKE ?2
           OR CAST(p.mcid AS TEXT) LIKE ?2
           OR lower(COALESCE(p.pdgid, '')) LIKE ?2
        "
    }

    fn search_particle_rows_sql(&self) -> &'static str {
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
        WHERE (?1 = '')
           OR lower(p.name) LIKE ?2
           OR CAST(p.mcid AS TEXT) LIKE ?2
           OR lower(COALESCE(p.pdgid, '')) LIKE ?2
        ORDER BY abs(p.mcid) ASC, p.mcid ASC, p.id ASC
        LIMIT ?3 OFFSET ?4
        "
    }
}

/// Read-only PDG SQLite database handle.
pub struct PdgDatabase {
    connection: Connection,
    query_builder: Box<dyn PdgQueryBuilder + Send + Sync>,
    source_path: PathBuf,
    active_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
struct RequiredIndex {
    name: &'static str,
    create_sql: &'static str,
}

const REQUIRED_INDICES: [RequiredIndex; 3] = [
    RequiredIndex {
        name: "idx_mcid",
        create_sql: "CREATE INDEX IF NOT EXISTS idx_mcid ON pdgparticle(mcid)",
    },
    RequiredIndex {
        name: "idx_pdgid_id",
        create_sql: "CREATE INDEX IF NOT EXISTS idx_pdgid_id ON pdgdata(pdgid_id)",
    },
    RequiredIndex {
        name: "idx_data_type",
        create_sql: "CREATE INDEX IF NOT EXISTS idx_data_type ON pdgid(data_type)",
    },
];

static PDG_CACHE_COPY_COUNTER: AtomicU64 = AtomicU64::new(0);

impl PdgDatabase {
    /// Open a PDG SQLite database and validate required schema/index contracts.
    pub fn open(path: &Path) -> SpireResult<Self> {
        Self::open_with_query_builder(path, Box::<StandardPdgQueryBuilder>::default())
    }

    /// Open with an explicit query-builder strategy.
    pub fn open_with_query_builder(
        path: &Path,
        query_builder: Box<dyn PdgQueryBuilder + Send + Sync>,
    ) -> SpireResult<Self> {
        let source_path = path.to_path_buf();
        let source_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
        let source_connection = Connection::open_with_flags(path, source_flags).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to open PDG database '{}' (read-only): {}",
                path.display(),
                err
            ))
        })?;

        Self::verify_schema_with(&source_connection, query_builder.as_ref())?;
        Self::apply_startup_pragmas(&source_connection, false)?;

        let missing_indices = Self::missing_indices(&source_connection)?;
        if missing_indices.is_empty() {
            return Ok(Self {
                connection: source_connection,
                query_builder,
                source_path: source_path.clone(),
                active_path: source_path,
            });
        }

        drop(source_connection);

        let optimized_path = Self::prepare_optimized_copy(path)?;
        let optimized_flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE;
        let mut optimized_connection =
            Connection::open_with_flags(&optimized_path, optimized_flags).map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to open optimized PDG cache copy '{}' (read-write): {}",
                    optimized_path.display(),
                    err
                ))
            })?;

        Self::apply_startup_pragmas(&optimized_connection, true)?;
        if let Err(schema_err) =
            Self::verify_schema_with(&optimized_connection, query_builder.as_ref())
        {
            drop(optimized_connection);

            let _ = fs::remove_file(&optimized_path);
            fs::copy(path, &optimized_path).map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to refresh invalid PDG cache copy '{}' from source '{}': {}",
                    optimized_path.display(),
                    path.display(),
                    err
                ))
            })?;

            optimized_connection = Connection::open_with_flags(&optimized_path, optimized_flags)
                .map_err(|err| {
                    SpireError::DatabaseError(format!(
                        "Failed to reopen refreshed PDG cache copy '{}' (read-write): {}",
                        optimized_path.display(),
                        err
                    ))
                })?;

            Self::apply_startup_pragmas(&optimized_connection, true)?;
            Self::verify_schema_with(&optimized_connection, query_builder.as_ref())
                .map_err(|_| schema_err)?;
        }

        Self::ensure_required_indices(&optimized_connection)?;

        Ok(Self {
            connection: optimized_connection,
            query_builder,
            source_path,
            active_path: optimized_path,
        })
    }

    /// Access the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Access the active query-builder strategy.
    pub fn query_builder(&self) -> &(dyn PdgQueryBuilder + Send + Sync) {
        self.query_builder.as_ref()
    }

    /// Original PDG source path requested by the caller.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// Effective database path currently opened for queries.
    pub fn active_path(&self) -> &Path {
        &self.active_path
    }

    fn verify_schema_with(
        connection: &Connection,
        query_builder: &(dyn PdgQueryBuilder + Send + Sync),
    ) -> SpireResult<()> {
        let mut stmt = connection
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name = ?1 LIMIT 1")
            .map_err(|err| {
                SpireError::DatabaseError(format!("Failed to prepare schema probe: {}", err))
            })?;

        for table in query_builder.required_tables() {
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

    fn apply_startup_pragmas(connection: &Connection, write_mode: bool) -> SpireResult<()> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to configure SQLite busy timeout for PDG database: {}",
                    err
                ))
            })?;

        if write_mode {
            connection
                .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .map_err(|err| {
                    SpireError::DatabaseError(format!(
                        "Failed to apply write-mode PDG PRAGMAs: {}",
                        err
                    ))
                })?;
        } else {
            let _ = connection.execute_batch("PRAGMA journal_mode=WAL;");
            let _ = connection.execute_batch("PRAGMA synchronous=NORMAL;");
        }

        connection
            .execute_batch("PRAGMA temp_store=MEMORY; PRAGMA cache_size=-64000;")
            .map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to apply read-throughput PDG PRAGMAs: {}",
                    err
                ))
            })?;

        Ok(())
    }

    fn missing_indices(connection: &Connection) -> SpireResult<Vec<RequiredIndex>> {
        let mut stmt = connection
            .prepare("SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1 LIMIT 1")
            .map_err(|err| {
                SpireError::DatabaseError(format!("Failed to prepare index probe query: {}", err))
            })?;

        let mut missing = Vec::new();
        for index in REQUIRED_INDICES {
            let exists = stmt
                .query_row([index.name], |row| row.get::<_, i64>(0))
                .map(|_| true)
                .unwrap_or(false);
            if !exists {
                missing.push(index);
            }
        }

        Ok(missing)
    }

    fn ensure_required_indices(connection: &Connection) -> SpireResult<()> {
        let missing = Self::missing_indices(connection)?;
        for index in missing {
            connection.execute(index.create_sql, []).map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to create PDG index '{}' on optimized cache copy: {}",
                    index.name, err
                ))
            })?;
        }

        Ok(())
    }

    fn prepare_optimized_copy(source_path: &Path) -> SpireResult<PathBuf> {
        let cache_root = Self::cache_root_dir()?;
        fs::create_dir_all(&cache_root).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to create PDG cache directory '{}': {}",
                cache_root.display(),
                err
            ))
        })?;

        source_path.file_name().ok_or_else(|| {
            SpireError::DatabaseError(format!(
                "Cannot derive SQLite file name from PDG source path '{}'",
                source_path.display()
            ))
        })?;

        let unique_copy_name = {
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("pdg-cache");
            let ext = source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("sqlite");
            let counter = PDG_CACHE_COPY_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!("{}-pid{}-copy{}.{}", stem, std::process::id(), counter, ext)
        };

        let cached_path = cache_root.join(unique_copy_name);

        fs::copy(source_path, &cached_path).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to copy PDG SQLite source '{}' to cache '{}': {}",
                source_path.display(),
                cached_path.display(),
                err
            ))
        })?;

        let metadata = fs::metadata(&cached_path).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to inspect copied PDG cache file '{}': {}",
                cached_path.display(),
                err
            ))
        })?;
        let mut permissions = metadata.permissions();
        if permissions.readonly() {
            #[cfg(windows)]
            {
                #[allow(clippy::permissions_set_readonly_false)]
                permissions.set_readonly(false);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let current_mode = permissions.mode();
                permissions.set_mode(current_mode | 0o200);
            }

            fs::set_permissions(&cached_path, permissions).map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Failed to make PDG cache copy writable '{}': {}",
                    cached_path.display(),
                    err
                ))
            })?;
        }

        Ok(cached_path)
    }

    fn cache_root_dir() -> SpireResult<PathBuf> {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(local_app_data)
                .join("SPIRE")
                .join("cache")
                .join("pdg"));
        }

        if let Ok(xdg_cache) = env::var("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(xdg_cache).join("spire").join("pdg"));
        }

        if let Ok(home) = env::var("HOME") {
            return Ok(PathBuf::from(home).join(".cache").join("spire").join("pdg"));
        }

        let temp_dir = env::temp_dir();
        if temp_dir.as_os_str().is_empty() {
            return Err(SpireError::DatabaseError(
                "Could not resolve a writable cache directory for PDG optimization".to_string(),
            ));
        }

        Ok(temp_dir.join("spire").join("pdg"))
    }
}
