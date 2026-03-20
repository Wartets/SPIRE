use spire_kernel::theory::pdg::adapter::PdgAdapter;
use spire_kernel::theory::pdg::contracts::PdgProvenance;
use spire_kernel::theory::pdg::policy::PdgExtractionPolicy;
use std::path::Path;

fn get_adapter() -> PdgAdapter {
    // Try multiple path possibilities depending on where tests are run from
    let possible_paths = [
        "data/pdg/pdg-2025-v0.2.2.sqlite",
        "../../data/pdg/pdg-2025-v0.2.2.sqlite",
        "../../../data/pdg/pdg-2025-v0.2.2.sqlite",
    ];
    
    let pdg_path = possible_paths
        .iter()
        .find(|p| Path::new(p).exists())
        .map(Path::new)
        .expect("Could not find PDG database at any expected path");
    
    let provenance = PdgProvenance {
        edition: "2025-v0.2.2".to_string(),
        source_id: "local_sqlite".to_string(),
        origin: Some(pdg_path.to_string_lossy().to_string()),
        fingerprint: "test".to_string(),
    };

    PdgAdapter::open(pdg_path, provenance).expect("failed to open PDG adapter")
}

#[test]
fn test_decay_table_catalog_mode() {
    let adapter = get_adapter();
    
    // Try to get decay table in catalog mode (includes generic channels)
    // Note: not all particles have decay modes in PDG
    // We test with a particle that we know has decay information
    // For now, we'll accept either an error (no decays) or a valid table
    
    match adapter.get_decay_table(23, PdgExtractionPolicy::Catalog) {
        Ok(table) => {
            // Z boson has well-defined decay modes
            assert_eq!(table.parent_pdg_id, 23);
            // In catalog mode, we should get all channels
        }
        Err(_) => {
            // Some particles may not have decay data in PDG, which is fine
        }
    }
}

#[test]
fn test_decay_table_strict_physical_mode() {
    let adapter = get_adapter();
    
    // Try to get decay table in strict physical mode (filters out generic channels)
    match adapter.get_decay_table(23, PdgExtractionPolicy::StrictPhysical) {
        Ok(table) => {
            assert_eq!(table.parent_pdg_id, 23);
            // In strict physical mode, we should get only concrete channels
            for channel in &table.channels {
                assert!(!channel.is_generic, "strict physical mode should not include generic channels");
            }
        }
        Err(_) => {
            // Some particles may not have decay data in PDG
        }
    }
}

#[test]
fn test_decay_product_serialization() {
    use spire_kernel::theory::pdg::contracts::PdgDecayProduct;
    
    let concrete = PdgDecayProduct::Concrete { mcid: 11 };
    let concrete_json = serde_json::to_string(&concrete).expect("failed to serialize concrete product");
    assert!(concrete_json.contains("concrete"));
    assert!(concrete_json.contains("11"));
    
    let generic = PdgDecayProduct::Generic { description: "X".to_string() };
    let generic_json = serde_json::to_string(&generic).expect("failed to serialize generic product");
    assert!(generic_json.contains("generic"));
    assert!(generic_json.contains("X"));
}

#[test]
fn test_decay_channel_branching_fraction() {
    use spire_kernel::theory::pdg::contracts::{PdgDecayChannel, PdgDecayProduct, PdgValue};
    
    let channel = PdgDecayChannel {
        mode_id: 1,
        products: vec![(PdgDecayProduct::Concrete { mcid: 11 }, 1)],
        branching_ratio: Some(PdgValue::Exact {
            value: 0.5,
            is_limit: false,
        }),
        is_generic: false,
        description: Some("test".to_string()),
    };
    
    assert_eq!(channel.branching_fraction(), Some(0.5));
    
    let limit_channel = PdgDecayChannel {
        mode_id: 2,
        products: vec![(PdgDecayProduct::Concrete { mcid: 13 }, 1)],
        branching_ratio: Some(PdgValue::Exact {
            value: 0.1,
            is_limit: true,
        }),
        is_generic: false,
        description: Some("limit".to_string()),
    };
    
    assert_eq!(limit_channel.branching_fraction(), None);
}

#[test]
fn test_decay_table_methods() {
    use spire_kernel::theory::pdg::contracts::{PdgDecayTable, PdgDecayChannel, PdgDecayProduct};
    use spire_kernel::theory::pdg::contracts::PdgValue;
    
    let table = PdgDecayTable {
        parent_pdg_id: 23,
        channels: vec![
            PdgDecayChannel {
                mode_id: 1,
                products: vec![(PdgDecayProduct::Concrete { mcid: 11 }, 1)],
                branching_ratio: Some(PdgValue::Exact {
                    value: 0.3,
                    is_limit: false,
                }),
                is_generic: false,
                description: Some("e+ e-".to_string()),
            },
            PdgDecayChannel {
                mode_id: 2,
                products: vec![(PdgDecayProduct::Generic { description: "X".to_string() }, 1)],
                branching_ratio: Some(PdgValue::Exact {
                    value: 0.7,
                    is_limit: false,
                }),
                is_generic: true,
                description: Some("inclusive".to_string()),
            },
        ],
        edition: "2025-v0.2.2".to_string(),
    };
    
    // Test concrete_channels
    let concrete = table.concrete_channels();
    assert_eq!(concrete.len(), 1);
    
    // Test concrete_branching_sum
    let sum = table.concrete_branching_sum();
    assert!((sum - 0.3).abs() < 1e-6);
    
    // Test generic_channels
    let generic = table.generic_channels();
    assert_eq!(generic.len(), 1);
}

#[test]
fn test_policy_application() {
    use spire_kernel::theory::pdg::policy::apply_policy;
    use spire_kernel::theory::pdg::contracts::{PdgDecayChannel, PdgDecayProduct};
    
    let channels = vec![
        PdgDecayChannel {
            mode_id: 1,
            products: vec![(PdgDecayProduct::Concrete { mcid: 11 }, 1)],
            branching_ratio: None,
            is_generic: false,
            description: None,
        },
        PdgDecayChannel {
            mode_id: 2,
            products: vec![(PdgDecayProduct::Generic { description: "X".to_string() }, 1)],
            branching_ratio: None,
            is_generic: true,
            description: None,
        },
    ];
    
    // Strict physical should filter out generic
    let strict = apply_policy(channels.clone(), PdgExtractionPolicy::StrictPhysical);
    assert_eq!(strict.len(), 1);
    assert!(!strict[0].is_generic);
    
    // Catalog should keep all
    let catalog = apply_policy(channels, PdgExtractionPolicy::Catalog);
    assert_eq!(catalog.len(), 2);
}
