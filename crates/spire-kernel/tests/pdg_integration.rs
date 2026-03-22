use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::theory::pdg::adapter::PdgAdapter;
use spire_kernel::theory::pdg::contracts::{PdgDecayProduct, PdgProvenance, PdgValue};
use spire_kernel::theory::pdg::database::PdgDatabase;
use spire_kernel::theory::pdg::guards::EditionMismatchPolicy;
use spire_kernel::theory::pdg::policy::PdgExtractionPolicy;
use spire_kernel::SpireError;

trait AliasResolutionHarness {
    fn resolve_alias_candidates(&self, alias: &str) -> Vec<i32>;
}

#[derive(Debug, Default)]
struct MockPdgDataSource {
    alias_map: HashMap<String, Vec<i32>>,
}

impl MockPdgDataSource {
    fn with_alias(mut self, alias: &str, candidates: Vec<i32>) -> Self {
        self.alias_map
            .insert(alias.to_ascii_lowercase(), candidates);
        self
    }
}

impl AliasResolutionHarness for MockPdgDataSource {
    fn resolve_alias_candidates(&self, alias: &str) -> Vec<i32> {
        self.alias_map
            .get(&alias.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct SqliteAliasHarness {
    connection: Connection,
}

impl SqliteAliasHarness {
    fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        Ok(Self { connection })
    }
}

impl AliasResolutionHarness for SqliteAliasHarness {
    fn resolve_alias_candidates(&self, alias: &str) -> Vec<i32> {
        let mut statement = match self.connection.prepare(
            "
            SELECT DISTINCT p.mcid
            FROM pdgitem_map m
            JOIN pdgparticle p ON p.pdgitem_id = m.target_id
            WHERE lower(m.name) = lower(?1)
            ORDER BY abs(p.mcid) ASC, p.mcid ASC
            ",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let rows = match statement.query_map([alias], |row| row.get::<_, i32>(0)) {
            Ok(iter) => iter,
            Err(_) => return Vec::new(),
        };

        rows.flatten().collect()
    }
}

fn resolve_decay_alias(alias: &str, resolver: &dyn AliasResolutionHarness) -> PdgDecayProduct {
    let candidates = resolver.resolve_alias_candidates(alias);
    match candidates.as_slice() {
        [only] => PdgDecayProduct::Concrete { mcid: *only },
        _ => PdgDecayProduct::Generic {
            description: alias.to_string(),
        },
    }
}

fn pdg_database_path() -> Option<PathBuf> {
    if let Ok(override_path) = env::var("SPIRE_PDG_DB_PATH") {
        let candidate = PathBuf::from(override_path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let from_workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("pdg")
        .join("pdg-2025-v0.2.2.sqlite");

    if from_workspace.exists() {
        return Some(from_workspace);
    }

    None
}

fn local_provenance(path: &Path) -> PdgProvenance {
    PdgProvenance {
        edition: "2025-v0.2.2".to_string(),
        release_timestamp: None,
        source_id: "local_sqlite".to_string(),
        origin: Some(path.display().to_string()),
        source_path: Some(path.display().to_string()),
        extraction_policy: Some("Catalog".to_string()),
        source_arbitration_outcome: Some("local".to_string()),
        local_file_fingerprint: None,
        fingerprint: "phase79-pdg-integration-tests".to_string(),
    }
}

fn find_alias(conn: &Connection) -> Option<String> {
    const PREFERRED_ALIASES: [&str; 4] = ["ell", "lep", "l", "hadrons"];

    for alias in PREFERRED_ALIASES {
        let mut stmt = conn
            .prepare(
                "
                SELECT COUNT(DISTINCT p.mcid)
                FROM pdgitem_map m
                JOIN pdgparticle p ON p.pdgitem_id = m.target_id
                WHERE lower(m.name) = lower(?1)
                ",
            )
            .ok()?;

        let count = stmt
            .query_row([alias], |row| row.get::<_, i64>(0))
            .unwrap_or(0);
        if count > 1 {
            return Some(alias.to_string());
        }
    }

    let mut stmt = conn
        .prepare(
            "
            SELECT m.name
            FROM pdgitem_map m
            JOIN pdgparticle p ON p.pdgitem_id = m.target_id
            GROUP BY m.name
            HAVING COUNT(DISTINCT p.mcid) > 1
            ORDER BY COUNT(DISTINCT p.mcid) DESC, m.name ASC
            LIMIT 1
            ",
        )
        .ok()?;

    stmt.query_row([], |row| row.get::<_, String>(0)).ok()
}

fn find_generic_decay_parent(conn: &Connection) -> Option<(i32, String)> {
    let mut stmt = conn
        .prepare(
            "
            SELECT
                p.mcid,
                lower(d.name) AS alias
            FROM pdgdecay d
            JOIN pdgid g ON g.id = d.pdgid_id
            JOIN pdgparticle p ON p.pdgid_id = g.id
            LEFT JOIN pdgitem i ON i.id = d.pdgitem_id
            WHERE
                lower(d.name) IN ('ell', 'lep', 'hadrons', 'x', 'inclusive', 'invisible')
                OR lower(d.name) LIKE 'ell%'
                OR lower(d.name) LIKE '%hadr%'
                OR i.item_type IN ('G', 'B')
            ORDER BY
                CASE
                    WHEN lower(d.name) = 'ell' THEN 0
                    WHEN lower(d.name) = 'hadrons' THEN 1
                    ELSE 2
                END,
                abs(p.mcid) ASC,
                p.mcid ASC
            LIMIT 1
            ",
        )
        .ok()?;

    stmt.query_row([], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
    })
    .ok()
}

fn create_temp_db_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("spire-phase79-{name}-{unique}.sqlite"))
}

fn create_schema_join_fixture(path: &Path) -> rusqlite::Result<()> {
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "
        CREATE TABLE pdgid (
            id INTEGER PRIMARY KEY,
            pdgid TEXT NOT NULL,
            parent_pdgid TEXT,
            data_type TEXT,
            description TEXT,
            sort INTEGER DEFAULT 0
        );

        CREATE TABLE pdgdata (
            id INTEGER PRIMARY KEY,
            pdgid_id INTEGER NOT NULL,
            value REAL NOT NULL,
            error_positive REAL,
            error_negative REAL,
            limit_type TEXT,
            value_type TEXT,
            unit_text TEXT,
            display_in_percent INTEGER DEFAULT 0,
            edition TEXT,
            in_summary_table INTEGER DEFAULT 1,
            sort INTEGER DEFAULT 0
        );

        CREATE TABLE pdgparticle (
            id INTEGER PRIMARY KEY,
            pdgid_id INTEGER NOT NULL,
            pdgid TEXT,
            mcid INTEGER NOT NULL,
            name TEXT,
            cc_type TEXT,
            charge REAL,
            quantum_j TEXT,
            quantum_p TEXT,
            quantum_c TEXT,
            pdgitem_id INTEGER
        );

        CREATE TABLE pdgitem (
            id INTEGER PRIMARY KEY,
            name TEXT,
            item_type TEXT
        );

        CREATE TABLE pdgitem_map (
            id INTEGER PRIMARY KEY,
            name TEXT,
            target_id INTEGER,
            sort INTEGER DEFAULT 0
        );
        ",
    )?;

    conn.execute(
        "INSERT INTO pdgid (id, pdgid, parent_pdgid, data_type, description, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            1_i64,
            "XROOT",
            Option::<String>::None,
            Option::<String>::None,
            "root",
            0_i64
        ],
    )?;

    conn.execute(
        "INSERT INTO pdgid (id, pdgid, parent_pdgid, data_type, description, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![2_i64, "XMASS", "XROOT", "M", "mass", 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgid (id, pdgid, parent_pdgid, data_type, description, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![3_i64, "XWIDTH", "XROOT", "G", "width", 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgid (id, pdgid, parent_pdgid, data_type, description, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![4_i64, "XLIFE", "XROOT", "T", "lifetime", 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgdata
            (id, pdgid_id, value, error_positive, error_negative, limit_type, value_type, unit_text, display_in_percent, edition, in_summary_table, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![1_i64, 2_i64, 125.0_f64, 1.5_f64, 0.7_f64, Option::<String>::None, "W", "GeV", 0_i64, "2025", 1_i64, 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgdata
            (id, pdgid_id, value, error_positive, error_negative, limit_type, value_type, unit_text, display_in_percent, edition, in_summary_table, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![2_i64, 3_i64, 4.2_f64, 0.2_f64, 0.2_f64, Option::<String>::None, "M", "GeV", 0_i64, "2025", 1_i64, 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgdata
            (id, pdgid_id, value, error_positive, error_negative, limit_type, value_type, unit_text, display_in_percent, edition, in_summary_table, sort)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![3_i64, 4_i64, 1.0_f64, Option::<f64>::None, Option::<f64>::None, "U", "L", "s", 0_i64, "2025", 1_i64, 0_i64],
    )?;

    conn.execute(
        "INSERT INTO pdgparticle (id, pdgid_id, pdgid, mcid, name, cc_type, charge, quantum_j, quantum_p, quantum_c, pdgitem_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![1_i64, 1_i64, "XROOT", 9900015_i64, "X", "S", 0.0_f64, "0", "+", Option::<String>::None, Option::<i64>::None],
    )?;

    Ok(())
}

#[test]
fn alias_resolution_harness_marks_ambiguous_alias_as_generic() {
    let mock_source = MockPdgDataSource::default().with_alias("ell", vec![11, 13]);
    let resolved = resolve_decay_alias("ell", &mock_source);

    match resolved {
        PdgDecayProduct::Generic { description } => {
            assert_eq!(description, "ell");
        }
        other => panic!("expected generic decay product for ambiguous alias, got {other:?}"),
    }
}

#[test]
fn sqlite_alias_lookup_is_deterministic_for_ambiguous_aliases() {
    let Some(path) = pdg_database_path() else {
        eprintln!("Skipping sqlite alias lookup test: PDG database not available");
        return;
    };

    let alias_harness = SqliteAliasHarness::open(&path).expect("open sqlite alias harness");
    let Some(alias) = find_alias(&alias_harness.connection) else {
        eprintln!("Skipping sqlite alias lookup test: no ambiguous alias found in dataset");
        return;
    };

    let candidates = alias_harness.resolve_alias_candidates(&alias);
    assert!(
        candidates.len() > 1,
        "expected at least two candidates for ambiguous alias '{}'",
        alias
    );

    let candidates_second_pass = alias_harness.resolve_alias_candidates(&alias);
    assert_eq!(
        candidates, candidates_second_pass,
        "ambiguous alias candidate ordering should be deterministic for '{}'",
        alias
    );

    assert!(
        candidates.windows(2).all(|pair| {
            let left = pair[0];
            let right = pair[1];
            (left.abs(), left) <= (right.abs(), right)
        }),
        "alias candidates must be sorted by abs(mcid), mcid for deterministic tie-breaking: {:?}",
        candidates
    );

    let resolved = resolve_decay_alias(&alias, &alias_harness);
    assert!(
        matches!(resolved, PdgDecayProduct::Generic { .. }),
        "ambiguous alias '{}' must be represented as generic in decay context",
        alias
    );
}

#[test]
fn generic_decay_aliases_are_tagged_and_filtered_by_policy() {
    let Some(path) = pdg_database_path() else {
        eprintln!("Skipping generic decay alias test: PDG database not available");
        return;
    };

    let conn = Connection::open(&path).expect("open pdg sqlite file");
    let Some((parent_mcid, alias_hint)) = find_generic_decay_parent(&conn) else {
        eprintln!("Skipping generic decay alias test: no generic alias decay rows found");
        return;
    };

    let adapter = PdgAdapter::open(&path, local_provenance(&path)).expect("open adapter");

    let catalog = adapter
        .get_decay_table(parent_mcid, PdgExtractionPolicy::Catalog)
        .expect("fetch catalog decay table");

    let alias_lower = alias_hint.to_ascii_lowercase();
    let has_generic_alias = catalog.channels.iter().any(|channel| {
        channel.products.iter().any(|(product, _)| {
            matches!(
                product,
                PdgDecayProduct::Generic { description }
                    if description.to_ascii_lowercase().contains(&alias_lower)
                        || description.to_ascii_lowercase().contains("hadr")
                        || description.to_ascii_lowercase() == "x"
                        || description.to_ascii_lowercase().contains("inclusive")
            )
        })
    });

    assert!(
        has_generic_alias,
        "expected at least one generic decay product carrying alias hint '{}'; parent MCID {}",
        alias_hint, parent_mcid
    );

    let strict = adapter
        .get_decay_table(parent_mcid, PdgExtractionPolicy::StrictPhysical)
        .expect("fetch strict decay table");

    assert!(
        strict.channels.iter().all(|channel| !channel.is_generic),
        "strict physical policy must exclude generic channels"
    );
    assert!(
        strict.channels.len() <= catalog.channels.len(),
        "strict physical channels should be a subset of catalog channels"
    );
}

#[test]
fn quantity_extraction_uses_pdgid_data_type_with_asymmetric_and_limit_semantics() {
    let temp_path = create_temp_db_path("quantity-semantics");
    create_schema_join_fixture(&temp_path).expect("create sqlite fixture");

    let database = PdgDatabase::open(&temp_path).expect("open fixture database");

    let mass = database
        .extract_core_quantity(1, "M")
        .expect("extract mass quantity")
        .expect("mass value present");

    match mass {
        PdgValue::Asymmetric {
            value,
            error,
            is_limit,
        } => {
            assert!((value - 125.0).abs() < 1e-12);
            assert!((error.minus - 0.7).abs() < 1e-12);
            assert!((error.plus - 1.5).abs() < 1e-12);
            assert!(!is_limit);
        }
        other => panic!("expected asymmetric mass value, got {other:?}"),
    }

    let width = database
        .extract_core_quantity(1, "G")
        .expect("extract width quantity")
        .expect("width value present");

    match width {
        PdgValue::Symmetric {
            value,
            error,
            is_limit,
        } => {
            assert!((value - 4.2).abs() < 1e-12);
            assert!((error - 0.2).abs() < 1e-12);
            assert!(!is_limit);
        }
        other => panic!("expected symmetric width value, got {other:?}"),
    }

    let lifetime = database
        .extract_core_quantity(1, "T")
        .expect("extract lifetime quantity")
        .expect("lifetime value present");

    match lifetime {
        PdgValue::Exact { value, is_limit } => {
            assert!((value - 1.0).abs() < 1e-12);
            assert!(is_limit, "expected limit flag from limit_type/value_type");
        }
        other => panic!("expected exact lifetime value with limit flag, got {other:?}"),
    }

    let _ = fs::remove_file(temp_path);
}

fn test_provenance(edition: &str) -> PdgProvenance {
    PdgProvenance {
        edition: edition.to_string(),
        release_timestamp: None,
        source_id: "test_source".to_string(),
        origin: Some("unit-test".to_string()),
        source_path: Some("unit-test".to_string()),
        extraction_policy: Some("Catalog".to_string()),
        source_arbitration_outcome: Some("local".to_string()),
        local_file_fingerprint: None,
        fingerprint: format!("fingerprint-{edition}"),
    }
}

#[test]
fn edition_lock_rejects_cross_edition_injection_in_strict_mode() {
    let mut model = TheoreticalModel::default();

    model
        .apply_pdg_provenance(
            test_provenance("2024-v0.1.4"),
            EditionMismatchPolicy::Strict,
        )
        .expect("initial lock should succeed");

    let err = model
        .apply_pdg_provenance(
            test_provenance("2025-v0.2.2"),
            EditionMismatchPolicy::Strict,
        )
        .expect_err("strict lock should reject mixed editions");

    match err {
        SpireError::EditionMismatch {
            locked_edition,
            incoming_edition,
            source_id,
        } => {
            assert_eq!(locked_edition, "2024-v0.1.4");
            assert_eq!(incoming_edition, "2025-v0.2.2");
            assert_eq!(source_id, "test_source");
        }
        other => panic!("expected SpireError::EditionMismatch, got {other:?}"),
    }
}
