use std::path::PathBuf;

use spire_kernel::theory::pdg::adapter::PdgAdapter;
use spire_kernel::theory::pdg::contracts::{PdgProvenance, PdgValue};

fn pdg_database_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("pdg")
        .join("pdg-2025-v0.2.2.sqlite")
}

fn test_provenance() -> PdgProvenance {
    PdgProvenance {
        edition: "2025-v0.2.2".to_string(),
        release_timestamp: None,
        source_id: "local_sqlite".to_string(),
        origin: Some(pdg_database_path().display().to_string()),
        source_path: Some(pdg_database_path().display().to_string()),
        extraction_policy: None,
        source_arbitration_outcome: None,
        local_file_fingerprint: None,
        fingerprint: "phase71-test-local".to_string(),
    }
}

fn central(value: &PdgValue) -> f64 {
    value.central()
}

#[test]
fn electron_mass_lookup_from_mcid() {
    let adapter =
        PdgAdapter::open(&pdg_database_path(), test_provenance()).expect("open pdg adapter");
    let record = adapter
        .get_particle_properties(11)
        .expect("lookup electron by mcid");

    let mass = record.mass.as_ref().expect("electron mass present");
    assert!((central(mass) - 0.00051099895).abs() < 1.0e-12);
}

#[test]
fn z_mass_and_width_lookup_from_mcid() {
    let adapter =
        PdgAdapter::open(&pdg_database_path(), test_provenance()).expect("open pdg adapter");
    let record = adapter
        .get_particle_properties(23)
        .expect("lookup Z by mcid");

    let mass = record.mass.as_ref().expect("Z mass present");
    let width = record.width.as_ref().expect("Z width present");

    assert!((central(mass) - 91.1876).abs() < 0.01);
    assert!((central(width) - 2.4952).abs() < 0.01);
}

#[test]
fn positron_and_electron_mass_agree() {
    let adapter =
        PdgAdapter::open(&pdg_database_path(), test_provenance()).expect("open pdg adapter");

    let electron = adapter
        .get_particle_properties(11)
        .expect("lookup electron by mcid");
    let positron = adapter
        .get_particle_properties(-11)
        .expect("lookup positron by mcid");

    let me = electron
        .mass
        .as_ref()
        .expect("electron mass present")
        .central();
    let mp = positron
        .mass
        .as_ref()
        .expect("positron mass present")
        .central();

    assert!((me - mp).abs() < 1.0e-15);
}
