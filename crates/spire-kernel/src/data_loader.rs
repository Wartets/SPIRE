//! # Data Loader — TOML Ingestion for Particle and Vertex Databases
//!
//! This module provides the deserialization layer that reads external TOML data
//! files (`particles.toml`, `sm_vertices.toml`, BSM extensions) into the kernel's
//! internal type system.
//!
//! ## Design
//!
//! Raw TOML structures (`RawParticle`, `RawVertex`) mirror the exact schema of the
//! data files. Conversion routines then transform these flat records into the
//! richer kernel types (`Field`, `QuantumNumbers`, `LagrangianTerm`, `VertexFactor`).
//! This two-stage approach decouples the on-disk format from the internal
//! representations, making schema evolution straightforward.

use serde::Deserialize;
use std::collections::HashSet;

use crate::lagrangian::{LagrangianTerm, LagrangianTermKind, TheoreticalModel, VertexFactor};
use crate::ontology::{
    BaryonNumber, ChargeConjugation, ColorRepresentation, ElectricCharge, Field, Hypercharge,
    InteractionType, LeptonNumbers, Parity, QuantumNumbers, Spin, WeakIsospin, WeakMultiplet,
};
use crate::SpireError;
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Raw TOML Deserialization Structures — Particles
// ---------------------------------------------------------------------------

/// Top-level wrapper for `particles.toml`.
///
/// The file contains a TOML array-of-tables `[[particle]]`.
#[derive(Debug, Clone, Deserialize)]
pub struct ParticleDatabase {
    /// List of particle (field) definitions.
    pub particle: Vec<RawParticle>,
}

/// A single particle entry as it appears in `particles.toml`.
///
/// Field names and types match the TOML schema exactly.
/// Integer-encoded quantum numbers are stored as-is and converted to
/// the kernel's newtype wrappers during the transform step.
#[derive(Debug, Clone, Deserialize)]
pub struct RawParticle {
    /// Unique identifier (e.g., `"e-"`, `"photon"`, `"u"`).
    pub id: String,
    /// Human-readable name (e.g., `"Electron"`).
    pub name: String,
    /// LaTeX symbol (e.g., `"e^-"`).
    pub symbol: String,
    /// Rest mass in GeV/$c^2$.
    pub mass: f64,
    /// Total decay width in GeV.
    pub width: f64,
    /// Twice the spin quantum number.
    pub spin: u8,
    /// Electric charge in units of $e/3$ (e.g., $-3$ for $Q = -1$).
    pub electric_charge: i8,
    /// Twice the weak isospin third component $T_3$.
    pub weak_isospin: i8,
    /// Hypercharge $Y$ in units of $1/3$.
    pub hypercharge: i8,
    /// Three times the baryon number $B$.
    pub baryon_number: i8,
    /// Lepton family numbers $[L_e, L_\mu, L_\tau]$.
    pub lepton_numbers: [i8; 3],
    /// Parity: `"Even"` or `"Odd"`.
    pub parity: String,
    /// Charge conjugation: `"Even"`, `"Odd"`, or `"Undefined"`.
    pub charge_conjugation: String,
    /// Colour representation: `"Singlet"`, `"Triplet"`, `"AntiTriplet"`, `"Octet"`.
    pub color: String,
    /// Weak multiplet: `"Singlet"`, `"DoubletUp"`, `"DoubletDown"`.
    pub weak_multiplet: String,
    /// List of interaction type strings.
    pub interactions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Raw TOML Deserialization Structures — Vertices
// ---------------------------------------------------------------------------

/// Top-level wrapper for `sm_vertices.toml` (or a BSM vertex file).
///
/// The file contains a TOML array-of-tables `[[vertex]]`.
#[derive(Debug, Clone, Deserialize)]
pub struct VertexDatabase {
    /// List of vertex (interaction) definitions.
    pub vertex: Vec<RawVertex>,
}

/// A single vertex entry as it appears in `sm_vertices.toml`.
///
/// Mirrors the TOML schema. The `expression` field carries the symbolic
/// Feynman rule and is stored as-is; the full derivation logic lives in the
/// `lagrangian` module.
#[derive(Debug, Clone, Deserialize)]
pub struct RawVertex {
    /// Unique identifier (e.g., `"qed_eea"`).
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Ordered list of field IDs at this vertex.
    pub field_ids: Vec<String>,
    /// Coupling constant symbol (e.g., `"e"`, `"g_s"`).
    pub coupling_symbol: String,
    /// Numerical coupling value (optional).
    pub coupling_value: Option<f64>,
    /// Lorentz structure of the vertex (e.g., `"gamma_mu"`, `"scalar"`).
    pub lorentz_structure: String,
    /// Interaction type string (e.g., `"Electromagnetic"`).
    pub interaction_type: String,
    /// Term kind string (e.g., `"Interaction"`).
    pub term_kind: String,
    /// Symbolic Feynman rule expression (LaTeX-style).
    pub expression: String,
    /// Number of legs at this vertex.
    pub n_legs: u8,
    /// Mass dimension of the operator (default 4 = renormalisable).
    /// Set to 5 or 6 for effective field theory operators.
    pub operator_dimension: Option<u8>,
}

// ---------------------------------------------------------------------------
// Conversion: RawParticle → Field
// ---------------------------------------------------------------------------

/// Convert a [`RawParticle`] from the TOML data into the kernel's [`Field`] type.
///
/// Parses string-encoded enum values and wraps integer quantum numbers in
/// their newtype wrappers.
pub fn raw_particle_to_field(raw: &RawParticle) -> SpireResult<Field> {
    let parity = parse_parity(&raw.parity)?;
    let charge_conj = parse_charge_conjugation(&raw.charge_conjugation)?;
    let color = parse_color(&raw.color)?;
    let weak_mult = parse_weak_multiplet(&raw.weak_multiplet)?;

    let interactions: Vec<InteractionType> = raw
        .interactions
        .iter()
        .map(|s| parse_interaction_type(s))
        .collect::<SpireResult<Vec<_>>>()?;

    let quantum_numbers = QuantumNumbers {
        electric_charge: ElectricCharge(raw.electric_charge),
        weak_isospin: WeakIsospin(raw.weak_isospin),
        hypercharge: Hypercharge(raw.hypercharge),
        baryon_number: BaryonNumber(raw.baryon_number),
        lepton_numbers: LeptonNumbers {
            electron: raw.lepton_numbers[0],
            muon: raw.lepton_numbers[1],
            tau: raw.lepton_numbers[2],
        },
        spin: Spin(raw.spin),
        parity,
        charge_conjugation: charge_conj,
        color,
        weak_multiplet: weak_mult,
        representations: legacy_sm_to_representations(color, weak_mult, raw.hypercharge),
    };

    Ok(Field {
        id: raw.id.clone(),
        name: raw.name.clone(),
        symbol: raw.symbol.clone(),
        mass: raw.mass,
        width: raw.width,
        quantum_numbers,
        interactions,
    })
}

// ---------------------------------------------------------------------------
// Conversion: RawVertex → LagrangianTerm + VertexFactor
// ---------------------------------------------------------------------------

/// Convert a [`RawVertex`] into a [`LagrangianTerm`] for the internal model.
pub fn raw_vertex_to_term(raw: &RawVertex) -> SpireResult<LagrangianTerm> {
    let interaction_type = parse_interaction_type(&raw.interaction_type)?;
    let term_kind = parse_term_kind(&raw.term_kind)?;

    Ok(LagrangianTerm {
        id: raw.id.clone(),
        description: raw.description.clone(),
        coupling_symbol: raw.coupling_symbol.clone(),
        coupling_value: raw.coupling_value,
        field_ids: raw.field_ids.clone(),
        lorentz_structure: raw.lorentz_structure.clone(),
        interaction_type,
        term_kind,
        operator_dimension: raw.operator_dimension,
    })
}

/// Convert a [`RawVertex`] directly into a [`VertexFactor`] Feynman rule.
pub fn raw_vertex_to_factor(raw: &RawVertex) -> SpireResult<VertexFactor> {
    Ok(VertexFactor {
        term_id: raw.id.clone(),
        field_ids: raw.field_ids.clone(),
        expression: raw.expression.clone(),
        coupling_value: raw.coupling_value,
        n_legs: raw.n_legs,
    })
}

// ---------------------------------------------------------------------------
// Public API — Database Loading
// ---------------------------------------------------------------------------

/// Load and parse a particle database from a TOML string.
///
/// Reads the contents of a `particles.toml` file (or equivalent) and returns
/// a [`ParticleDatabase`] containing all particle definitions.
///
/// # Arguments
/// * `toml_content` — The full contents of the particles TOML file.
pub fn load_particle_database(toml_content: &str) -> SpireResult<ParticleDatabase> {
    toml::from_str(toml_content).map_err(|e| {
        SpireError::ModelParseError(format!("Failed to parse particle database TOML: {}", e))
    })
}

/// Load and parse a vertex database from a TOML string.
///
/// Reads the contents of an `sm_vertices.toml` file (or BSM extension) and
/// returns a [`VertexDatabase`] containing all vertex definitions.
///
/// # Arguments
/// * `toml_content` — The full contents of the vertices TOML file.
pub fn load_vertex_database(toml_content: &str) -> SpireResult<VertexDatabase> {
    toml::from_str(toml_content).map_err(|e| {
        SpireError::ModelParseError(format!("Failed to parse vertex database TOML: {}", e))
    })
}

/// Construct a complete [`TheoreticalModel`] from particle and vertex TOML data.
///
/// This is the primary high-level entry point for model initialization.
/// It calls [`load_particle_database`], [`load_vertex_database`], performs
/// conversions via [`raw_particle_to_field`] and [`raw_vertex_to_term`], and
/// assembles the full model.
///
/// # Arguments
/// * `particles_toml` — Contents of the particles definition file.
/// * `vertices_toml` — Contents of the vertices/interactions definition file.
/// * `model_name` — Name to assign to the constructed model.
pub fn build_model(
    particles_toml: &str,
    vertices_toml: &str,
    model_name: &str,
) -> SpireResult<TheoreticalModel> {
    let particle_db = load_particle_database(particles_toml)?;
    let vertex_db = load_vertex_database(vertices_toml)?;

    let fields: Vec<Field> = particle_db
        .particle
        .iter()
        .map(raw_particle_to_field)
        .collect::<SpireResult<Vec<_>>>()?;

    // ------------------------------------------------------------------
    // Referential integrity check: every field_id referenced by a vertex
    // must correspond to a defined particle.
    // ------------------------------------------------------------------
    let known_ids: HashSet<&str> = fields.iter().map(|f| f.id.as_str()).collect();

    for raw_vertex in &vertex_db.vertex {
        for fid in &raw_vertex.field_ids {
            if !known_ids.contains(fid.as_str()) {
                return Err(SpireError::ModelParseError(format!(
                    "Vertex '{}' references undefined particle '{}'. \
                     Defined particle IDs: [{}]",
                    raw_vertex.id,
                    fid,
                    fields
                        .iter()
                        .map(|f| f.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
    }

    let terms: Vec<LagrangianTerm> = vertex_db
        .vertex
        .iter()
        .map(raw_vertex_to_term)
        .collect::<SpireResult<Vec<_>>>()?;

    let vertex_factors: Vec<VertexFactor> = vertex_db
        .vertex
        .iter()
        .map(raw_vertex_to_factor)
        .collect::<SpireResult<Vec<_>>>()?;

    // Propagators are derived from kinetic/mass terms later.
    let propagators = Vec::new();

    Ok(TheoreticalModel {
        name: model_name.to_string(),
        description: format!(
            "Model '{}' with {} fields and {} interaction terms.",
            model_name,
            fields.len(),
            terms.len()
        ),
        fields,
        terms,
        vertex_factors,
        propagators,
        gauge_symmetry: None,
        spacetime: crate::algebra::SpacetimeConfig::default(),
        constants: crate::ontology::PhysicalConstants::default(),
    })
}

// ---------------------------------------------------------------------------
// Spacetime Configuration Parsing
// ---------------------------------------------------------------------------

/// Raw TOML representation of the optional `[spacetime]` table.
///
/// # Example TOML
///
/// ```toml
/// [spacetime]
/// dimension = 4
/// metric_signature = ["+", "-", "-", "-"]
///
/// [spacetime.constants]
/// speed_of_light = 1.0
/// hbar = 1.0
/// gravitational_constant = 6.709e-39
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct RawSpacetimeConfig {
    /// Number of spacetime dimensions (default: 4).
    pub dimension: Option<usize>,
    /// Metric signature as an array of "+" or "-" strings.
    pub metric_signature: Option<Vec<String>>,
    /// Optional physical constants override.
    pub constants: Option<RawPhysicalConstants>,
}

/// Raw TOML representation for physical constants.
#[derive(Debug, Clone, Deserialize)]
pub struct RawPhysicalConstants {
    /// Speed of light (default: 1.0).
    pub speed_of_light: Option<f64>,
    /// Reduced Planck constant (default: 1.0).
    pub hbar: Option<f64>,
    /// Gravitational constant in GeV^-2 (default: 6.709e-39).
    pub gravitational_constant: Option<f64>,
    /// Boltzmann constant (default: 1.0).
    pub boltzmann_constant: Option<f64>,
}

/// Wrapper for TOML files that may contain a `[spacetime]` table.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfigFile {
    /// Optional spacetime configuration.
    pub spacetime: Option<RawSpacetimeConfig>,
}

/// Parse an optional `[spacetime]` TOML table into the kernel's spacetime
/// configuration types.
///
/// If the input TOML does not contain a `[spacetime]` table, returns the
/// standard 4D Minkowski defaults.
///
/// # Arguments
/// * `toml_str` — Raw TOML string that may contain a `[spacetime]` table.
pub fn parse_spacetime_config(
    toml_str: &str,
) -> SpireResult<(
    crate::algebra::SpacetimeConfig,
    crate::ontology::PhysicalConstants,
)> {
    use crate::algebra::{
        FlatMetric, MetricSign, MetricSignature, SpacetimeConfig, SpacetimeDimension,
    };
    use crate::ontology::PhysicalConstants;

    // Try parsing the TOML — if it fails or has no [spacetime], use defaults.
    let config: ModelConfigFile =
        toml::from_str(toml_str).unwrap_or(ModelConfigFile { spacetime: None });

    let raw = match config.spacetime {
        Some(r) => r,
        None => {
            return Ok((SpacetimeConfig::default(), PhysicalConstants::default()));
        }
    };

    // Build metric signature
    let signature = if let Some(ref sig_strs) = raw.metric_signature {
        let signs: Vec<MetricSign> = sig_strs
            .iter()
            .map(|s| match s.as_str() {
                "+" => Ok(MetricSign::Plus),
                "-" => Ok(MetricSign::Minus),
                other => Err(SpireError::ModelParseError(format!(
                    "Invalid metric signature entry '{}'. Expected '+' or '-'.",
                    other
                ))),
            })
            .collect::<SpireResult<Vec<_>>>()?;

        if let Some(dim) = raw.dimension {
            if signs.len() != dim {
                return Err(SpireError::ModelParseError(format!(
                    "Metric signature has {} entries but dimension is specified as {}.",
                    signs.len(),
                    dim
                )));
            }
        }

        MetricSignature::new(signs)
    } else if let Some(dim) = raw.dimension {
        MetricSignature::minkowski(dim)
    } else {
        MetricSignature::minkowski_4d()
    };

    let dim_val = signature.dimension() as u32;
    let spacetime_config =
        SpacetimeConfig::custom(FlatMetric { signature }, SpacetimeDimension::Fixed(dim_val));

    // Build physical constants
    let defaults = PhysicalConstants::default();
    let constants = if let Some(ref raw_c) = raw.constants {
        PhysicalConstants {
            speed_of_light: raw_c.speed_of_light.unwrap_or(defaults.speed_of_light),
            hbar: raw_c.hbar.unwrap_or(defaults.hbar),
            gravitational_constant: raw_c
                .gravitational_constant
                .unwrap_or(defaults.gravitational_constant),
            boltzmann_constant: raw_c
                .boltzmann_constant
                .unwrap_or(defaults.boltzmann_constant),
        }
    } else {
        defaults
    };

    Ok((spacetime_config, constants))
}

/// Build a theoretical model with optional spacetime configuration.
///
/// Extends [`build_model`] by parsing an optional `[spacetime]` table from
/// a separate TOML string (or the same file).
///
/// # Arguments
/// * `particles_toml` — Contents of the particles definition file.
/// * `vertices_toml` — Contents of the vertices/interactions definition file.
/// * `model_name` — Name to assign to the constructed model.
/// * `config_toml` — Optional TOML string containing a `[spacetime]` table.
pub fn build_model_with_config(
    particles_toml: &str,
    vertices_toml: &str,
    model_name: &str,
    config_toml: Option<&str>,
) -> SpireResult<TheoreticalModel> {
    let mut model = build_model(particles_toml, vertices_toml, model_name)?;

    if let Some(config_str) = config_toml {
        let (spacetime, constants) = parse_spacetime_config(config_str)?;
        model.spacetime = spacetime;
        model.constants = constants;
    }

    Ok(model)
}

// ---------------------------------------------------------------------------
// Helper: String → Enum Parsers
// ---------------------------------------------------------------------------

/// Parse a parity string (`"Even"` / `"Odd"`) into the kernel enum.
fn parse_parity(s: &str) -> SpireResult<Parity> {
    match s {
        "Even" => Ok(Parity::Even),
        "Odd" => Ok(Parity::Odd),
        other => Err(SpireError::ModelParseError(format!(
            "Unknown parity value: '{}'. Expected 'Even' or 'Odd'.",
            other
        ))),
    }
}

/// Parse a charge conjugation string into the kernel enum.
fn parse_charge_conjugation(s: &str) -> SpireResult<ChargeConjugation> {
    match s {
        "Even" => Ok(ChargeConjugation::Even),
        "Odd" => Ok(ChargeConjugation::Odd),
        "Undefined" => Ok(ChargeConjugation::Undefined),
        other => Err(SpireError::ModelParseError(format!(
            "Unknown charge conjugation value: '{}'. Expected 'Even', 'Odd', or 'Undefined'.",
            other
        ))),
    }
}

/// Parse a colour representation string into the kernel enum.
fn parse_color(s: &str) -> SpireResult<ColorRepresentation> {
    match s {
        "Singlet" => Ok(ColorRepresentation::Singlet),
        "Triplet" => Ok(ColorRepresentation::Triplet),
        "AntiTriplet" => Ok(ColorRepresentation::AntiTriplet),
        "Octet" => Ok(ColorRepresentation::Octet),
        other => Err(SpireError::ModelParseError(format!(
            "Unknown color representation: '{}'. Expected 'Singlet', 'Triplet', 'AntiTriplet', or 'Octet'.",
            other
        ))),
    }
}

/// Parse a weak multiplet string into the kernel enum.
fn parse_weak_multiplet(s: &str) -> SpireResult<WeakMultiplet> {
    match s {
        "Singlet" => Ok(WeakMultiplet::Singlet),
        "DoubletUp" => Ok(WeakMultiplet::DoubletUp),
        "DoubletDown" => Ok(WeakMultiplet::DoubletDown),
        other => {
            if let Some(rest) = other.strip_prefix("Triplet(") {
                if let Some(num_str) = rest.strip_suffix(')') {
                    let n: i8 = num_str.parse().map_err(|_| {
                        SpireError::ModelParseError(format!(
                            "Invalid triplet index in weak multiplet: '{}'",
                            other
                        ))
                    })?;
                    return Ok(WeakMultiplet::Triplet(n));
                }
            }
            Err(SpireError::ModelParseError(format!(
                "Unknown weak multiplet: '{}'. Expected 'Singlet', 'DoubletUp', 'DoubletDown', or 'Triplet(N)'.",
                other
            )))
        }
    }
}

/// Parse an interaction type string into the kernel enum.
fn parse_interaction_type(s: &str) -> SpireResult<InteractionType> {
    match s {
        "Electromagnetic" => Ok(InteractionType::Electromagnetic),
        "WeakCC" => Ok(InteractionType::WeakCC),
        "WeakNC" => Ok(InteractionType::WeakNC),
        "Strong" => Ok(InteractionType::Strong),
        "Yukawa" => Ok(InteractionType::Yukawa),
        "ScalarSelf" => Ok(InteractionType::ScalarSelf),
        other => Err(SpireError::ModelParseError(format!(
            "Unknown interaction type: '{}'. Expected one of: Electromagnetic, WeakCC, WeakNC, Strong, Yukawa, ScalarSelf.",
            other
        ))),
    }
}

/// Parse a Lagrangian term kind string into the kernel enum.
fn parse_term_kind(s: &str) -> SpireResult<LagrangianTermKind> {
    match s {
        "Kinetic" => Ok(LagrangianTermKind::Kinetic),
        "Mass" => Ok(LagrangianTermKind::Mass),
        "Interaction" => Ok(LagrangianTermKind::Interaction),
        "GaugeFixing" => Ok(LagrangianTermKind::GaugeFixing),
        "Ghost" => Ok(LagrangianTermKind::Ghost),
        "ContactInteraction" => Ok(LagrangianTermKind::ContactInteraction),
        other => Err(SpireError::ModelParseError(format!(
            "Unknown Lagrangian term kind: '{}'. Expected one of: Kinetic, Mass, Interaction, GaugeFixing, Ghost, ContactInteraction.",
            other
        ))),
    }
}

// ---------------------------------------------------------------------------
// Legacy SM → Generalized Representation Adapter (Phase 14)
// ---------------------------------------------------------------------------

/// Build a `Vec<LieGroupRepresentation>` from legacy Standard-Model quantum
/// numbers (colour, weak multiplet, hypercharge).
///
/// This adapter ensures that particles loaded from old-format TOML files
/// automatically gain the generalized representation entries needed by the
/// Phase-14 gauge-conservation engine.
///
/// # Mapping rules
///
/// | SM field          | Gauge group      | Representation            |
/// |-------------------|------------------|---------------------------|
/// | `color`           | $SU(3)_C$        | **3** / **3̄** / **8** / **1** |
/// | `weak_multiplet`  | $SU(2)_L$        | **2** / **1**             |
/// | `hypercharge`     | $U(1)_Y$         | charge = hypercharge / 6  |
fn legacy_sm_to_representations(
    color: ColorRepresentation,
    weak_multiplet: WeakMultiplet,
    hypercharge_raw: i8,
) -> Vec<crate::groups::LieGroupRepresentation> {
    use crate::groups::{LieGroup, LieGroupRepresentation};

    let mut reps = Vec::new();

    // SU(3)_C representation.
    let su3 = LieGroup::SU {
        n: 3,
        label: "C".into(),
    };
    match color {
        ColorRepresentation::Triplet => reps.push(LieGroupRepresentation {
            group: su3,
            dimension: 3,
            charge: None,
            is_conjugate: false,
            dynkin_labels: vec![1, 0],
            label: "3".into(),
        }),
        ColorRepresentation::AntiTriplet => reps.push(LieGroupRepresentation {
            group: su3,
            dimension: 3,
            charge: None,
            is_conjugate: true,
            dynkin_labels: vec![0, 1],
            label: "3̄".into(),
        }),
        ColorRepresentation::Octet => reps.push(LieGroupRepresentation {
            group: su3,
            dimension: 8,
            charge: None,
            is_conjugate: false,
            dynkin_labels: vec![1, 1],
            label: "8".into(),
        }),
        ColorRepresentation::Singlet => reps.push(LieGroupRepresentation {
            group: su3,
            dimension: 1,
            charge: None,
            is_conjugate: false,
            dynkin_labels: vec![0, 0],
            label: "1".into(),
        }),
    }

    // SU(2)_L representation.
    let su2 = LieGroup::SU {
        n: 2,
        label: "L".into(),
    };
    match weak_multiplet {
        WeakMultiplet::DoubletUp | WeakMultiplet::DoubletDown => {
            reps.push(LieGroupRepresentation {
                group: su2,
                dimension: 2,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![1],
                label: "2".into(),
            });
        }
        WeakMultiplet::Singlet => {
            reps.push(LieGroupRepresentation {
                group: su2,
                dimension: 1,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![0],
                label: "1".into(),
            });
        }
        WeakMultiplet::Triplet(_) => {
            reps.push(LieGroupRepresentation {
                group: su2,
                dimension: 3,
                charge: None,
                is_conjugate: false,
                dynkin_labels: vec![2],
                label: "3".into(),
            });
        }
    }

    // U(1)_Y charge.
    let u1y = LieGroup::U1 { label: "Y".into() };
    let hypercharge_val = hypercharge_raw as f64 / 6.0;
    reps.push(LieGroupRepresentation {
        group: u1y,
        dimension: 1,
        charge: Some(hypercharge_val),
        is_conjugate: false,
        dynkin_labels: vec![],
        label: format!("{:.4}", hypercharge_val),
    });

    reps
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PARTICLES_TOML: &str = r#"
[[particle]]
id             = "e-"
name           = "Electron"
symbol         = "e^-"
mass           = 0.000511
width          = 0.0
spin           = 1
electric_charge = -3
weak_isospin   = -1
hypercharge    = -3
baryon_number  = 0
lepton_numbers = [1, 0, 0]
parity         = "Even"
charge_conjugation = "Undefined"
color          = "Singlet"
weak_multiplet = "DoubletDown"
interactions   = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id             = "photon"
name           = "Photon"
symbol         = "γ"
mass           = 0.0
width          = 0.0
spin           = 2
electric_charge = 0
weak_isospin   = 0
hypercharge    = 0
baryon_number  = 0
lepton_numbers = [0, 0, 0]
parity         = "Odd"
charge_conjugation = "Odd"
color          = "Singlet"
weak_multiplet = "Singlet"
interactions   = ["Electromagnetic"]
"#;

    const SAMPLE_VERTICES_TOML: &str = r#"
[[vertex]]
id                = "qed_eea"
description       = "QED vertex: electron-photon coupling"
field_ids         = ["e-", "photon", "e-"]
coupling_symbol   = "e"
coupling_value    = 0.30282212
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i e \\gamma^{\\mu}"
n_legs            = 3
"#;

    #[test]
    fn deserialize_particle_database() {
        let db: ParticleDatabase = toml::from_str(SAMPLE_PARTICLES_TOML).unwrap();
        assert_eq!(db.particle.len(), 2);
        assert_eq!(db.particle[0].id, "e-");
        assert_eq!(db.particle[0].spin, 1);
        assert_eq!(db.particle[0].electric_charge, -3);
        assert_eq!(db.particle[1].id, "photon");
    }

    #[test]
    fn deserialize_vertex_database() {
        let db: VertexDatabase = toml::from_str(SAMPLE_VERTICES_TOML).unwrap();
        assert_eq!(db.vertex.len(), 1);
        assert_eq!(db.vertex[0].id, "qed_eea");
        assert_eq!(db.vertex[0].n_legs, 3);
    }

    #[test]
    fn load_particle_database_api() {
        let db = load_particle_database(SAMPLE_PARTICLES_TOML).unwrap();
        assert_eq!(db.particle.len(), 2);
        assert_eq!(db.particle[0].id, "e-");
    }

    #[test]
    fn load_vertex_database_api() {
        let db = load_vertex_database(SAMPLE_VERTICES_TOML).unwrap();
        assert_eq!(db.vertex.len(), 1);
        assert_eq!(db.vertex[0].id, "qed_eea");
    }

    #[test]
    fn raw_particle_to_field_electron() {
        let db = load_particle_database(SAMPLE_PARTICLES_TOML).unwrap();
        let field = raw_particle_to_field(&db.particle[0]).unwrap();
        assert_eq!(field.id, "e-");
        assert_eq!(field.name, "Electron");
        assert_eq!(field.mass, 0.000511);
        assert_eq!(field.quantum_numbers.spin, Spin(1));
        assert_eq!(field.quantum_numbers.electric_charge, ElectricCharge(-3));
        assert_eq!(field.quantum_numbers.weak_isospin, WeakIsospin(-1));
        assert_eq!(field.quantum_numbers.hypercharge, Hypercharge(-3));
        assert_eq!(field.quantum_numbers.baryon_number, BaryonNumber(0));
        assert_eq!(field.quantum_numbers.lepton_numbers.electron, 1);
        assert_eq!(field.quantum_numbers.lepton_numbers.muon, 0);
        assert_eq!(field.quantum_numbers.lepton_numbers.tau, 0);
        assert_eq!(field.quantum_numbers.parity, Parity::Even);
        assert_eq!(
            field.quantum_numbers.charge_conjugation,
            ChargeConjugation::Undefined
        );
        assert_eq!(field.quantum_numbers.color, ColorRepresentation::Singlet);
        assert_eq!(
            field.quantum_numbers.weak_multiplet,
            WeakMultiplet::DoubletDown
        );
        assert_eq!(field.interactions.len(), 3);
        assert_eq!(field.interactions[0], InteractionType::Electromagnetic);
    }

    #[test]
    fn raw_vertex_to_term_and_factor() {
        let db = load_vertex_database(SAMPLE_VERTICES_TOML).unwrap();
        let term = raw_vertex_to_term(&db.vertex[0]).unwrap();
        assert_eq!(term.id, "qed_eea");
        assert_eq!(term.interaction_type, InteractionType::Electromagnetic);
        assert_eq!(term.term_kind, LagrangianTermKind::Interaction);

        let factor = raw_vertex_to_factor(&db.vertex[0]).unwrap();
        assert_eq!(factor.term_id, "qed_eea");
        assert_eq!(factor.n_legs, 3);
        assert!(factor.coupling_value.is_some());
    }

    #[test]
    fn build_model_from_toml() {
        let model = build_model(SAMPLE_PARTICLES_TOML, SAMPLE_VERTICES_TOML, "QED Test").unwrap();
        assert_eq!(model.name, "QED Test");
        assert_eq!(model.fields.len(), 2);
        assert_eq!(model.terms.len(), 1);
        assert_eq!(model.vertex_factors.len(), 1);
        assert!(model.propagators.is_empty());
    }

    #[test]
    fn parse_invalid_parity_returns_error() {
        let result = parse_parity("Wrong");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_interaction_returns_error() {
        let result = parse_interaction_type("Gravity");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_toml_returns_model_error() {
        let result = load_particle_database("this is not valid toml [[[");
        assert!(result.is_err());
        if let Err(SpireError::ModelParseError(msg)) = result {
            assert!(msg.contains("Failed to parse"));
        } else {
            panic!("Expected ModelParseError");
        }
    }

    // --- Referential integrity tests ---

    #[test]
    fn build_model_rejects_dangling_vertex_reference() {
        // Vertex references "graviton" which is NOT defined in the particles TOML.
        let dangling_vertices = r#"
[[vertex]]
id                = "bad_vertex"
description       = "References undefined graviton"
field_ids         = ["e-", "graviton", "e-"]
coupling_symbol   = "g"
coupling_value    = 0.1
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i g gamma^mu"
n_legs            = 3
"#;

        let result = build_model(SAMPLE_PARTICLES_TOML, dangling_vertices, "Bad Model");
        assert!(
            result.is_err(),
            "Should reject vertex referencing undefined particle"
        );
        if let Err(SpireError::ModelParseError(msg)) = result {
            assert!(
                msg.contains("graviton"),
                "Error should mention the undefined particle ID 'graviton', got: {}",
                msg
            );
            assert!(
                msg.contains("bad_vertex"),
                "Error should mention the offending vertex ID 'bad_vertex', got: {}",
                msg
            );
        } else {
            panic!("Expected ModelParseError for dangling vertex reference");
        }
    }

    #[test]
    fn build_model_accepts_valid_vertex_references() {
        // Both "e-" is defined in particles — vertex referencing only "e-" should work.
        let valid_vertices = r#"
[[vertex]]
id                = "self_coupling"
description       = "Electron self-coupling (fictional)"
field_ids         = ["e-", "e-", "e-"]
coupling_symbol   = "g"
coupling_value    = 0.1
lorentz_structure = "scalar"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i g"
n_legs            = 3
"#;

        let result = build_model(SAMPLE_PARTICLES_TOML, valid_vertices, "Valid Model");
        assert!(
            result.is_ok(),
            "Should accept vertex with all defined particles"
        );
    }

    // --- EFT / ContactInteraction tests ---

    #[test]
    fn parse_contact_interaction_term_kind() {
        let result = parse_term_kind("ContactInteraction");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), LagrangianTermKind::ContactInteraction);
    }

    #[test]
    fn build_model_with_contact_interaction() {
        // Two particles + a dimension-6 contact vertex between them.
        let particles = r#"
[[particle]]
id             = "e-"
name           = "Electron"
symbol         = "e^-"
mass           = 0.000511
width          = 0.0
spin           = 1
electric_charge = -3
weak_isospin   = -1
hypercharge    = -3
baryon_number  = 0
lepton_numbers = [1, 0, 0]
parity         = "Even"
charge_conjugation = "Undefined"
color          = "Singlet"
weak_multiplet = "DoubletDown"
interactions   = ["Electromagnetic"]

[[particle]]
id             = "mu-"
name           = "Muon"
symbol         = "mu^-"
mass           = 0.10566
width          = 0.0
spin           = 1
electric_charge = -3
weak_isospin   = -1
hypercharge    = -3
baryon_number  = 0
lepton_numbers = [0, 1, 0]
parity         = "Even"
charge_conjugation = "Undefined"
color          = "Singlet"
weak_multiplet = "DoubletDown"
interactions   = ["Electromagnetic"]
"#;

        let vertices = r#"
[[vertex]]
id                = "eft_4f_eemm"
description       = "Dimension-6 four-fermion contact: e-e-mu-mu-"
field_ids         = ["e-", "e-", "mu-", "mu-"]
coupling_symbol   = "C_LL"
coupling_value    = 1.0
lorentz_structure = "contact_VmA"
interaction_type  = "Electromagnetic"
term_kind         = "ContactInteraction"
expression        = "-i C_LL / Lambda^2 (gamma^mu P_L) x (gamma_mu P_L)"
n_legs            = 4
operator_dimension = 6
"#;

        let model = build_model(particles, vertices, "EFT Test").unwrap();
        assert_eq!(model.name, "EFT Test");
        assert_eq!(model.fields.len(), 2);
        assert_eq!(model.terms.len(), 1);
        assert_eq!(
            model.terms[0].term_kind,
            LagrangianTermKind::ContactInteraction
        );
        assert_eq!(model.terms[0].operator_dimension, Some(6));
        assert_eq!(model.vertex_factors.len(), 1);
        assert_eq!(model.vertex_factors[0].n_legs, 4);
    }

    // ===================================================================
    // Spacetime Config TOML Parser Tests
    // ===================================================================

    #[test]
    fn parse_spacetime_config_defaults() {
        // No [spacetime] table → standard 4D Minkowski + natural units
        let (st, c) = parse_spacetime_config("").unwrap();
        assert_eq!(st.dimension(), 4);
        assert_eq!(st.metric.signature.display_signature(), "(+,-,-,-)");
        assert_eq!(c.speed_of_light, 1.0);
        assert_eq!(c.hbar, 1.0);
    }

    #[test]
    fn parse_spacetime_config_dimension_only() {
        let toml = r#"
[spacetime]
dimension = 10
"#;
        let (st, _) = parse_spacetime_config(toml).unwrap();
        assert_eq!(st.dimension(), 10);
        assert_eq!(st.metric.signature.time_dimensions(), 1);
        assert_eq!(st.metric.signature.space_dimensions(), 9);
    }

    #[test]
    fn parse_spacetime_config_explicit_signature() {
        let toml = r#"
[spacetime]
metric_signature = ["+", "+", "-", "-"]
"#;
        let (st, _) = parse_spacetime_config(toml).unwrap();
        assert_eq!(st.dimension(), 4);
        assert_eq!(st.metric.signature.display_signature(), "(+,+,-,-)");
    }

    #[test]
    fn parse_spacetime_config_signature_dimension_mismatch() {
        let toml = r#"
[spacetime]
dimension = 5
metric_signature = ["+", "-", "-"]
"#;
        assert!(parse_spacetime_config(toml).is_err());
    }

    #[test]
    fn parse_spacetime_config_invalid_sign() {
        let toml = r#"
[spacetime]
metric_signature = ["+", "-", "x"]
"#;
        assert!(parse_spacetime_config(toml).is_err());
    }

    #[test]
    fn parse_spacetime_config_with_constants() {
        let toml = r#"
[spacetime]
dimension = 4

[spacetime.constants]
speed_of_light = 2.998e8
hbar = 1.055e-34
"#;
        let (st, c) = parse_spacetime_config(toml).unwrap();
        assert_eq!(st.dimension(), 4);
        assert!((c.speed_of_light - 2.998e8).abs() < 1.0);
        assert!((c.hbar - 1.055e-34).abs() < 1e-37);
        // Unspecified constants → defaults
        assert_eq!(c.boltzmann_constant, 1.0);
    }

    #[test]
    fn build_model_with_config_no_spacetime() {
        let particles = r#"
[[particle]]
id = "photon"
name = "Photon"
symbol = "gamma"
spin = 2
mass = 0.0
width = 0.0
electric_charge = 0
weak_isospin = 0
hypercharge = 0
baryon_number = 0
lepton_numbers = [0, 0, 0]
parity = "Odd"
charge_conjugation = "Odd"
color = "Singlet"
weak_multiplet = "Singlet"
interactions = ["Electromagnetic"]
is_self_conjugate = true
"#;
        let vertices = "vertex = []";
        let model = build_model_with_config(particles, vertices, "Test", None).unwrap();
        // Standard defaults
        assert_eq!(model.spacetime.dimension(), 4);
        assert_eq!(model.constants.speed_of_light, 1.0);
    }

    #[test]
    fn build_model_with_config_custom_spacetime() {
        let particles = r#"
[[particle]]
id = "photon"
name = "Photon"
symbol = "gamma"
spin = 2
mass = 0.0
width = 0.0
electric_charge = 0
weak_isospin = 0
hypercharge = 0
baryon_number = 0
lepton_numbers = [0, 0, 0]
parity = "Odd"
charge_conjugation = "Odd"
color = "Singlet"
weak_multiplet = "Singlet"
interactions = ["Electromagnetic"]
is_self_conjugate = true
"#;
        let vertices = "vertex = []";
        let config = r#"
[spacetime]
dimension = 10

[spacetime.constants]
gravitational_constant = 1e-30
"#;
        let model = build_model_with_config(particles, vertices, "String", Some(config)).unwrap();
        assert_eq!(model.spacetime.dimension(), 10);
        assert!((model.constants.gravitational_constant - 1e-30).abs() < 1e-40);
        assert_eq!(model.constants.speed_of_light, 1.0); // default preserved
    }
}
