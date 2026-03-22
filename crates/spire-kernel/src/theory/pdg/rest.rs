use serde::Deserialize;

use crate::io::network::{
    NetworkDiagnostics, NetworkThrottleConfig, ResponseMode, ThrottledHttpClient,
};
use crate::theory::pdg::arbiter::{PdgDataSource, SourcePriority};
use crate::theory::pdg::contracts::{PdgParticleRecord, PdgProvenance};

/// Configuration for the optional PDG REST data source.
#[derive(Debug, Clone)]
pub struct PdgRestConfig {
    /// Enables outbound REST calls when true.
    pub enabled: bool,
    /// Base URL for the REST endpoint.
    pub base_url: String,
    /// Stable source identifier exposed in provenance.
    pub source_id: String,
    /// Source precedence in arbitration (lower is preferred).
    pub priority: SourcePriority,
    /// Embedded network throttling configuration.
    pub throttle: NetworkThrottleConfig,
}

impl Default for PdgRestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: "https://pdgapi.lbl.gov".to_string(),
            source_id: "pdg_rest".to_string(),
            priority: 10,
            throttle: NetworkThrottleConfig::default(),
        }
    }
}

/// Optional REST-backed PDG particle source.
#[derive(Debug)]
pub struct PdgRestDataSource {
    config: PdgRestConfig,
    client: ThrottledHttpClient,
}

impl PdgRestDataSource {
    /// Build a REST source from configuration.
    pub fn new(config: PdgRestConfig) -> Self {
        let client = ThrottledHttpClient::new(config.throttle.clone());
        Self { config, client }
    }

    /// Replace runtime configuration and reset network client state.
    pub fn reconfigure(&mut self, config: PdgRestConfig) {
        self.client = ThrottledHttpClient::new(config.throttle.clone());
        self.config = config;
    }

    /// Return current source configuration.
    pub fn config(&self) -> &PdgRestConfig {
        &self.config
    }

    /// Return a telemetry snapshot from the throttled network layer.
    pub fn diagnostics(&self) -> NetworkDiagnostics {
        self.client.diagnostics()
    }

    /// Attempt to fetch a particle record from REST.
    pub fn fetch_particle_record(&self, pdg_id: i32) -> Option<PdgParticleRecord> {
        if !self.config.enabled {
            return None;
        }

        let url = format!(
            "{}/api/v1/particles/{}",
            self.config.base_url.trim_end_matches('/'),
            pdg_id
        );

        let mut record = match self
            .client
            .get_json_with_mode::<RestParticlePayload>(&url, ResponseMode::Optional404)
        {
            Ok(Some(payload)) => payload.into_record(),
            Ok(None) => return None,
            Err(_) => return None,
        };

        record.provenance.source_id = self.config.source_id.clone();
        record.provenance.source_arbitration_outcome = Some("api".to_string());
        Some(record)
    }
}

impl PdgDataSource for PdgRestDataSource {
    fn source_id(&self) -> &str {
        &self.config.source_id
    }

    fn priority(&self) -> SourcePriority {
        self.config.priority
    }

    fn particle_record(&self, pdg_id: i32) -> Option<PdgParticleRecord> {
        self.fetch_particle_record(pdg_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RestParticlePayload {
    pdg_id: i32,
    label: Option<String>,
    mass: Option<crate::theory::pdg::contracts::PdgValue>,
    width: Option<crate::theory::pdg::contracts::PdgValue>,
    lifetime: Option<crate::theory::pdg::contracts::PdgValue>,
    #[serde(default)]
    branching_fractions: Vec<crate::theory::pdg::contracts::PdgBranchingFraction>,
    #[serde(default)]
    quantum_numbers: crate::theory::pdg::contracts::PdgQuantumNumbers,
    provenance: Option<PdgProvenance>,
}

impl RestParticlePayload {
    fn into_record(self) -> PdgParticleRecord {
        PdgParticleRecord {
            pdg_id: self.pdg_id,
            label: self.label,
            mass: self.mass,
            width: self.width,
            lifetime: self.lifetime,
            branching_fractions: self.branching_fractions,
            quantum_numbers: self.quantum_numbers,
            provenance: self.provenance.unwrap_or(PdgProvenance {
                edition: "PDG REST".to_string(),
                release_timestamp: None,
                source_id: "pdg_rest".to_string(),
                origin: None,
                source_path: None,
                extraction_policy: Some("Catalog".to_string()),
                source_arbitration_outcome: Some("api".to_string()),
                local_file_fingerprint: None,
                fingerprint: "pdg-rest".to_string(),
            }),
        }
    }
}
