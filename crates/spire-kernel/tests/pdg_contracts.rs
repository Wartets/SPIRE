use spire_kernel::lagrangian::TheoreticalModel;
use spire_kernel::theory::pdg::arbiter::{
    arbitrate_particle_record, PdgDataSource, SourcePriority,
};
use spire_kernel::theory::pdg::contracts::{
    AsymmetricError, PdgParticleRecord, PdgProvenance, PdgQuantumNumbers, PdgValue,
};
use spire_kernel::theory::pdg::guards::EditionMismatchPolicy;
use spire_kernel::SpireError;

#[derive(Clone)]
struct MockSource {
    id: String,
    priority: SourcePriority,
    record: Option<PdgParticleRecord>,
}

impl PdgDataSource for MockSource {
    fn source_id(&self) -> &str {
        &self.id
    }

    fn priority(&self) -> SourcePriority {
        self.priority
    }

    fn particle_record(&self, _pdg_id: i32) -> Option<PdgParticleRecord> {
        self.record.clone()
    }
}

fn provenance(edition: &str, source_id: &str) -> PdgProvenance {
    PdgProvenance {
        edition: edition.to_string(),
        release_timestamp: None,
        source_id: source_id.to_string(),
        origin: Some(format!("{source_id}.sqlite")),
        source_path: Some(format!("{source_id}.sqlite")),
        extraction_policy: None,
        source_arbitration_outcome: None,
        local_file_fingerprint: None,
        fingerprint: format!("{edition}-{source_id}"),
    }
}

#[test]
fn asymmetric_value_serde_roundtrip() {
    let value = PdgValue::Asymmetric {
        value: 91.1876,
        error: AsymmetricError::new(0.0021, 0.0023),
        is_limit: false,
    };

    let json = serde_json::to_string(&value).expect("serialize asymmetric pdg value");
    let parsed: PdgValue = serde_json::from_str(&json).expect("deserialize asymmetric pdg value");

    assert_eq!(parsed, value);
    assert!((parsed.central() - 91.1876).abs() < 1e-12);
}

#[test]
fn arbiter_prefers_higher_precedence_source() {
    let local = MockSource {
        id: "local_sqlite".to_string(),
        priority: 10,
        record: Some(PdgParticleRecord {
            pdg_id: 23,
            label: Some("Z0".to_string()),
            mass: Some(PdgValue::Exact {
                value: 91.1876,
                is_limit: false,
            }),
            width: None,
            lifetime: None,
            branching_fractions: vec![],
            quantum_numbers: PdgQuantumNumbers::default(),
            provenance: provenance("2025-v0.2.2", "local_sqlite"),
        }),
    };

    let remote = MockSource {
        id: "pdg_rest".to_string(),
        priority: 50,
        record: Some(PdgParticleRecord {
            pdg_id: 23,
            label: Some("Z0".to_string()),
            mass: Some(PdgValue::Exact {
                value: 91.2,
                is_limit: false,
            }),
            width: None,
            lifetime: None,
            branching_fractions: vec![],
            quantum_numbers: PdgQuantumNumbers::default(),
            provenance: provenance("2025-v0.2.2", "pdg_rest"),
        }),
    };

    let sources: [&dyn PdgDataSource; 2] = [&remote, &local];
    let outcome = arbitrate_particle_record(&sources, 23);

    let selected = outcome.selected.expect("expected selected source");
    assert_eq!(selected.provenance.source_id, "local_sqlite");
    assert_eq!(outcome.candidates.len(), 2);
}

#[test]
fn edition_mismatch_blocks_then_override_allows_update() {
    let mut model = TheoreticalModel::default();

    model
        .apply_pdg_provenance(
            provenance("2025-v0.2.2", "local_sqlite"),
            EditionMismatchPolicy::Strict,
        )
        .expect("first provenance lock should succeed");

    let err = model
        .apply_pdg_provenance(
            provenance("2024-v0.1.4", "pdg_rest"),
            EditionMismatchPolicy::Strict,
        )
        .expect_err("strict policy should reject edition mismatch");

    match err {
        SpireError::EditionMismatch {
            locked_edition,
            incoming_edition,
            source_id,
        } => {
            assert_eq!(locked_edition, "2025-v0.2.2");
            assert_eq!(incoming_edition, "2024-v0.1.4");
            assert_eq!(source_id, "pdg_rest");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    model
        .apply_pdg_provenance(
            provenance("2024-v0.1.4", "pdg_rest"),
            EditionMismatchPolicy::AllowOverride,
        )
        .expect("override policy should allow edition change");

    assert_eq!(model.pdg_edition(), Some("2024-v0.1.4"));
}
