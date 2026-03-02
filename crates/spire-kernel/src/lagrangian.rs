//! # Lagrangian — Model Parsing and Feynman Rule Derivation
//!
//! This module parses theoretical models from data files (TOML) and derives the
//! corresponding vertex rules, propagators, and Feynman rules from Lagrangian
//! density terms.
//!
//! It is the engine that translates abstract theory (a Lagrangian density) into
//! computable rules (vertex factors, propagator kernels) that the `graph` and
//! `algebra` modules consume.

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::ontology::{Field, InteractionType, Spin};
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Core Data Structures
// ---------------------------------------------------------------------------

/// A single term in the Lagrangian density.
///
/// Each term encodes the fields involved, the coupling structure, and the
/// Lorentz/gauge index structure. Kinetic terms, mass terms, and interaction
/// terms are all representable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LagrangianTerm {
    /// Unique identifier for this term (e.g., `"qed_vertex"`, `"higgs_yukawa_electron"`).
    pub id: String,
    /// Human-readable description (e.g., "QED electron-photon vertex").
    pub description: String,
    /// The coupling constant symbol (e.g., `"e"`, `"g_s"`, `"y_e"`).
    pub coupling_symbol: String,
    /// Numerical value of the coupling constant (if known).
    pub coupling_value: Option<f64>,
    /// Identifiers of the fields participating in this interaction.
    pub field_ids: Vec<String>,
    /// The Lorentz structure of the vertex (e.g., `"gamma_mu"`, `"scalar"`, `"tensor"`).
    pub lorentz_structure: String,
    /// The interaction type this term belongs to.
    pub interaction_type: InteractionType,
    /// Whether this is a kinetic, mass, or interaction term.
    pub term_kind: LagrangianTermKind,
    /// Mass dimension of the operator (4 = renormalisable, 5/6 = EFT).
    /// `None` defaults to dimension-4 (standard renormalisable interaction).
    #[serde(default)]
    pub operator_dimension: Option<u8>,
}

/// Classification of a Lagrangian term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LagrangianTermKind {
    /// Kinetic term ($\bar\psi i\gamma^\mu \partial_\mu \psi$, etc.).
    Kinetic,
    /// Mass term ($m \bar\psi \psi$, $\frac12 m^2 \phi^2$, etc.).
    Mass,
    /// Interaction vertex (3-point, 4-point couplings).
    Interaction,
    /// Gauge-fixing term.
    GaugeFixing,
    /// Ghost term (Faddeev–Popov ghosts for non-Abelian theories).
    Ghost,
    /// Effective contact interaction (dimension-5 or dimension-6 operators).
    /// Used for 4-fermion operators and other EFT terms above dimension-4.
    ContactInteraction,
}

/// Structured mathematical form of a Feynman propagator.
///
/// Rather than storing the propagator as an opaque string, this enum
/// encodes the exact functional form so downstream modules (algebra,
/// graph) can manipulate propagators symbolically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropagatorForm {
    /// Scalar propagator: $\frac{i}{p^2 - m^2 + i\epsilon}$
    Scalar,
    /// Dirac fermion propagator: $\frac{i(\gamma^\mu p_\mu + m)}{p^2 - m^2 + i\epsilon}$
    DiracFermion,
    /// Massless vector boson in Feynman gauge: $\frac{-i g_{\mu\nu}}{p^2 + i\epsilon}$
    MasslessVector,
    /// Massive vector boson (unitary gauge): $\frac{-i(g_{\mu\nu} - p_\mu p_\nu / m^2)}{p^2 - m^2 + i\epsilon}$
    MassiveVector,
}

/// A **VertexFactor** is the Feynman rule derived from an interaction term in the
/// Lagrangian. It maps directly to the mathematical expression assigned to a
/// vertex in a Feynman diagram.
///
/// For example, the QED vertex yields $-ie\gamma^\mu$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexFactor {
    /// Reference to the originating `LagrangianTerm`.
    pub term_id: String,
    /// The field identifiers at the vertex (ordered).
    pub field_ids: Vec<String>,
    /// Symbolic expression for the vertex factor (e.g., `"-i e gamma^mu"`).
    pub expression: String,
    /// The coupling constant numerical factor.
    pub coupling_value: Option<f64>,
    /// Number of legs (fields) at this vertex.
    pub n_legs: u8,
}

/// A **Propagator** is the Feynman rule for an internal line, derived from the
/// kinetic + mass terms of the Lagrangian.
///
/// Encodes the Green's function appropriate for the particle's spin:
/// scalar, fermion (Dirac), or vector boson propagator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Propagator {
    /// The field this propagator describes.
    pub field_id: String,
    /// Spin of the propagating field (determines the functional form).
    pub spin: Spin,
    /// Mass of the propagating particle in GeV.
    pub mass: f64,
    /// Decay width in GeV (used for Breit–Wigner regulation near resonance).
    pub width: f64,
    /// Symbolic expression for the propagator (e.g.,
    /// `"i (gamma^mu p_mu + m) / (p^2 - m^2 + i epsilon)"` for a fermion).
    pub expression: String,
    /// Gauge parameter $\xi$ (relevant for gauge boson propagators in $R_\xi$ gauges).
    pub gauge_parameter: Option<f64>,
    /// Structured mathematical form of the propagator (scalar, fermion, vector).
    pub form: PropagatorForm,
}

/// A complete theoretical model loaded from data files.
///
/// Bundles all fields, Lagrangian terms, vertex factors, and propagators
/// that define the theory (e.g., the Standard Model, or a BSM extension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoreticalModel {
    /// Name of the model (e.g., `"Standard Model"`, `"MSSM"`).
    pub name: String,
    /// Description of the theoretical framework.
    pub description: String,
    /// All fields (particles) defined in this model.
    pub fields: Vec<Field>,
    /// All Lagrangian terms.
    pub terms: Vec<LagrangianTerm>,
    /// Derived vertex factors (computed from interaction terms).
    pub vertex_factors: Vec<VertexFactor>,
    /// Derived propagators (computed from kinetic + mass terms).
    pub propagators: Vec<Propagator>,
    /// The gauge symmetry of this model (Phase 14).
    ///
    /// `None` for legacy models that have not specified an explicit gauge group;
    /// `Some(gs)` for models with a fully-specified gauge symmetry
    /// (e.g., $SU(3)_C \times SU(2)_L \times U(1)_Y$ for the Standard Model).
    #[serde(default)]
    pub gauge_symmetry: Option<crate::groups::GaugeSymmetry>,
}

// ---------------------------------------------------------------------------
// Public API — Propagator and Vertex Rule Derivation
// ---------------------------------------------------------------------------

/// Derive the Feynman propagator for a given field from its spin and mass.
///
/// Maps the field's intrinsic properties to the correct Green's function:
/// - Spin-0 → scalar propagator
/// - Spin-½ → Dirac fermion propagator
/// - Spin-1 massless → Feynman-gauge vector propagator
/// - Spin-1 massive → unitary-gauge vector propagator
///
/// # Arguments
/// * `field` — The field definition supplying spin and mass.
pub fn derive_propagator(field: &Field) -> SpireResult<Propagator> {
    let twice_spin = field.quantum_numbers.spin.0;
    let mass = field.mass;

    let (form, expression) = match twice_spin {
        0 => (
            PropagatorForm::Scalar,
            format!("i / (p² - ({})² + iε)", mass),
        ),
        1 => (
            PropagatorForm::DiracFermion,
            format!("i(γ·p + {}) / (p² - ({})² + iε)", mass, mass),
        ),
        2 => {
            if mass.abs() < 1e-10 {
                (
                    PropagatorForm::MasslessVector,
                    "(-i g_μν) / (p² + iε)".to_string(),
                )
            } else {
                (
                    PropagatorForm::MassiveVector,
                    format!(
                        "-i(g_μν - p_μ p_ν / ({})²) / (p² - ({})² + iε)",
                        mass, mass
                    ),
                )
            }
        }
        other => {
            return Err(SpireError::InternalError(format!(
                "Unsupported spin 2s={} for propagator derivation of field '{}'",
                other, field.id
            )));
        }
    };

    // Gauge parameter ξ = 1 (Feynman gauge) for vector bosons.
    let gauge_parameter = if twice_spin == 2 { Some(1.0) } else { None };

    Ok(Propagator {
        field_id: field.id.clone(),
        spin: field.quantum_numbers.spin,
        mass,
        width: field.width,
        expression,
        gauge_parameter,
        form,
    })
}

/// Derive the Feynman vertex factor from a Lagrangian interaction term.
///
/// Translates the coupling constant and Lorentz structure of an interaction
/// term into the symbolic vertex factor used in amplitude construction.
/// For example, a QED interaction term yields $-ie\gamma^\mu$.
///
/// # Arguments
/// * `term` — The Lagrangian interaction term.
pub fn derive_vertex_factor(term: &LagrangianTerm) -> SpireResult<VertexFactor> {
    let expression = match term.lorentz_structure.as_str() {
        "gamma_mu" => format!("-i {} γ^μ", term.coupling_symbol),
        "scalar" => format!("-i {}", term.coupling_symbol),
        "tensor" => format!("-i {} T^a γ^μ", term.coupling_symbol),
        // Dimension-6 four-fermion contact interactions: (V−A)×(V−A)
        "contact_VmA" => format!(
            "-i {}/Λ² (γ^μ P_L) ⊗ (γ_μ P_L)",
            term.coupling_symbol
        ),
        // Dimension-6 four-fermion contact interactions: (V+A)×(V+A)
        "contact_VpA" => format!(
            "-i {}/Λ² (γ^μ P_R) ⊗ (γ_μ P_R)",
            term.coupling_symbol
        ),
        // Dimension-6 scalar four-fermion contact interaction
        "contact_SS" => format!(
            "-i {}/Λ² (1) ⊗ (1)",
            term.coupling_symbol
        ),
        // Generic dimension-6 four-fermion contact interaction
        "contact_4f" => format!(
            "-i {}/Λ² (γ^μ) ⊗ (γ_μ)",
            term.coupling_symbol
        ),
        other => format!("-i {} [{}]", term.coupling_symbol, other),
    };

    Ok(VertexFactor {
        term_id: term.id.clone(),
        field_ids: term.field_ids.clone(),
        expression,
        coupling_value: term.coupling_value,
        n_legs: term.field_ids.len() as u8,
    })
}

// ---------------------------------------------------------------------------
// Public API — Model Parsing and Lookup
// ---------------------------------------------------------------------------

/// Parse a theoretical model from TOML data files.
///
/// Delegates to [`crate::data_loader::build_model`] for TOML deserialization,
/// then derives propagators for every field and vertex factors for every
/// interaction term, populating the complete [`TheoreticalModel`].
///
/// # Arguments
/// * `particles_toml` — Contents of the particles definition file.
/// * `vertices_toml` — Contents of the vertices/interactions definition file.
pub fn parse_model(particles_toml: &str, vertices_toml: &str) -> SpireResult<TheoreticalModel> {
    let mut model = crate::data_loader::build_model(particles_toml, vertices_toml, "Parsed Model")?;

    // Derive propagators from each field's spin and mass.
    model.propagators = model
        .fields
        .iter()
        .map(derive_propagator)
        .collect::<SpireResult<Vec<_>>>()?;

    // If vertex factors were already loaded from TOML they are kept;
    // otherwise derive them from interaction terms.
    if model.vertex_factors.is_empty() {
        model.vertex_factors = model
            .terms
            .iter()
            .filter(|t| {
                t.term_kind == LagrangianTermKind::Interaction
                    || t.term_kind == LagrangianTermKind::ContactInteraction
            })
            .map(derive_vertex_factor)
            .collect::<SpireResult<Vec<_>>>()?;
    }

    Ok(model)
}

/// Derive vertex factors (Feynman rules) from the interaction terms in a model.
///
/// Iterates over all `LagrangianTerm`s of kind `Interaction` and produces
/// the corresponding [`VertexFactor`] via [`derive_vertex_factor`].
pub fn derive_vertex_rules(model: &TheoreticalModel) -> SpireResult<Vec<VertexFactor>> {
    model
        .terms
        .iter()
        .filter(|t| {
            t.term_kind == LagrangianTermKind::Interaction
                || t.term_kind == LagrangianTermKind::ContactInteraction
        })
        .map(derive_vertex_factor)
        .collect()
}

/// Derive propagators for every field defined in the model.
///
/// Iterates over all fields and produces the appropriate Feynman propagator
/// via [`derive_propagator`].
pub fn derive_propagators(model: &TheoreticalModel) -> SpireResult<Vec<Propagator>> {
    model.fields.iter().map(derive_propagator).collect()
}

/// Look up the vertex factor for a specific combination of field identifiers.
///
/// Searches the model's derived vertex factors for a vertex that matches the
/// given set of fields. The comparison is **order-independent**: the field
/// lists are sorted before matching.
///
/// # Arguments
/// * `model` — The active theoretical model.
/// * `field_ids` — The field identifiers meeting at the vertex.
pub fn lookup_vertex(model: &TheoreticalModel, field_ids: &[&str]) -> SpireResult<VertexFactor> {
    let mut sorted_query: Vec<&str> = field_ids.to_vec();
    sorted_query.sort();

    for vf in &model.vertex_factors {
        let mut sorted_fields: Vec<&str> = vf.field_ids.iter().map(|s| s.as_str()).collect();
        sorted_fields.sort();
        if sorted_fields == sorted_query {
            return Ok(vf.clone());
        }
    }

    Err(SpireError::InvalidVertex(format!(
        "No vertex found for fields: {:?}",
        field_ids
    )))
}

/// Look up the propagator for a specific field by its identifier.
///
/// # Arguments
/// * `model` — The active theoretical model.
/// * `field_id` — The identifier of the propagating field.
pub fn lookup_propagator(model: &TheoreticalModel, field_id: &str) -> SpireResult<Propagator> {
    model
        .propagators
        .iter()
        .find(|p| p.field_id == field_id)
        .cloned()
        .ok_or_else(|| {
            SpireError::UnknownParticle(format!("No propagator found for field: '{}'", field_id))
        })
}

// ---------------------------------------------------------------------------
// UFO Export — Simplified Universal FeynRules Output
// ---------------------------------------------------------------------------

impl TheoreticalModel {
    /// Generate a simplified **UFO `particles.py`** file from this model.
    ///
    /// Each field is emitted as a `Particle(...)` constructor call following
    /// the UFO specification used by MadGraph5 / Pythia.
    ///
    /// # Notes
    /// - PDG codes are approximated from spin and charge.
    /// - Antiparticles are auto-generated with negated PDG code.
    /// - This is a *simplified* export; a full UFO would include lorentz.py,
    ///   couplings.py, coupling_orders.py, vertices.py, etc.
    pub fn to_ufo_particles_py(&self) -> String {
        let mut lines = Vec::new();
        lines.push("# UFO particles.py — auto-generated by SPIRE".into());
        lines.push(format!("# Model: {}", self.name));
        lines.push(String::new());
        lines.push("from object_library import all_particles, Particle".into());
        lines.push(String::new());

        for (i, field) in self.fields.iter().enumerate() {
            let pdg = ufo_pdg_code(field, i);
            let spin_2j_plus_1 = field.quantum_numbers.spin.0 as i32 + 1;
            let color = ufo_color_code(field);
            let charge_thirds = field.quantum_numbers.electric_charge.0;
            let charge_str = format!("{}/{}", charge_thirds, 3);

            let var_name = ufo_variable_name(&field.id);
            lines.push(format!(
                "{var_name} = Particle(pdg_code={pdg}, name='{name}', antiname='{anti}', \
                 spin={spin}, color={color}, mass='{mass}', width='{width}', \
                 charge={charge})",
                name = field.id,
                anti = ufo_antiname(&field.id),
                spin = spin_2j_plus_1,
                mass = format!("M{}", var_name.to_uppercase()),
                width = if field.width > 0.0 {
                    format!("W{}", var_name.to_uppercase())
                } else {
                    "ZERO".into()
                },
                charge = charge_str,
            ));
            lines.push(String::new());
        }

        lines.join("\n")
    }

    /// Generate a simplified **UFO `parameters.py`** file.
    ///
    /// Contains external parameters (masses, widths) and internal parameters
    /// (coupling constants) derived from the model.
    pub fn to_ufo_parameters_py(&self) -> String {
        let mut lines = Vec::new();
        lines.push("# UFO parameters.py — auto-generated by SPIRE".into());
        lines.push(format!("# Model: {}", self.name));
        lines.push(String::new());
        lines.push("from object_library import all_parameters, Parameter".into());
        lines.push(String::new());
        lines.push("# --- External Parameters (masses and widths) ---".into());
        lines.push(String::new());

        for field in &self.fields {
            let var_name = ufo_variable_name(&field.id);
            let mass_name = format!("M{}", var_name.to_uppercase());
            lines.push(format!(
                "{mass_name} = Parameter(name='{mass_name}', nature='external', \
                 type='real', value={:.6}, block='MASS')",
                field.mass
            ));

            if field.width > 0.0 {
                let width_name = format!("W{}", var_name.to_uppercase());
                lines.push(format!(
                    "{width_name} = Parameter(name='{width_name}', nature='external', \
                     type='real', value={:.6}, block='DECAY')",
                    field.width
                ));
            }
        }

        lines.push(String::new());
        lines.push("# --- Internal Parameters (couplings) ---".into());
        lines.push(String::new());

        let mut seen_couplings = std::collections::HashSet::new();
        for term in &self.terms {
            if seen_couplings.contains(&term.coupling_symbol) {
                continue;
            }
            seen_couplings.insert(term.coupling_symbol.clone());
            let value = term.coupling_value.unwrap_or(0.0);
            lines.push(format!(
                "{sym} = Parameter(name='{sym}', nature='internal', \
                 type='real', value={value:.6})",
                sym = term.coupling_symbol,
            ));
        }

        lines.push(String::new());
        lines.join("\n")
    }

    /// Generate the full simplified UFO export as a `HashMap<filename, content>`.
    ///
    /// Returns `{ "particles.py": ..., "parameters.py": ..., "__init__.py": ... }`.
    pub fn to_ufo(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();

        files.insert("particles.py".into(), self.to_ufo_particles_py());
        files.insert("parameters.py".into(), self.to_ufo_parameters_py());

        // __init__.py with model metadata
        let init = format!(
            "# UFO Model: {}\n# Auto-generated by SPIRE\n\nfrom . import particles\nfrom . import parameters\n",
            self.name
        );
        files.insert("__init__.py".into(), init);

        files
    }
}

/// Approximate a PDG code for a field based on its properties.
fn ufo_pdg_code(field: &Field, index: usize) -> i32 {
    // Use well-known PDG codes for common particles.
    match field.id.as_str() {
        "e-" => 11,
        "e+" => -11,
        "mu-" => 13,
        "mu+" => -13,
        "tau-" => 15,
        "tau+" => -15,
        "nu_e" => 12,
        "anti_nu_e" | "nu_e_bar" => -12,
        "nu_mu" => 14,
        "anti_nu_mu" | "nu_mu_bar" => -14,
        "nu_tau" => 16,
        "anti_nu_tau" | "nu_tau_bar" => -16,
        "u" => 2,
        "u_bar" | "anti_u" => -2,
        "d" => 1,
        "d_bar" | "anti_d" => -1,
        "photon" | "a" | "gamma" => 22,
        "Z0" | "Z" => 23,
        "W+" => 24,
        "W-" => -24,
        "g" | "gluon" => 21,
        "H" | "higgs" => 25,
        _ => 9000000 + (index as i32 + 1), // BSM / unknown
    }
}

/// Map a colour representation to the UFO colour code (1 = singlet, 3 = triplet, etc.).
fn ufo_color_code(field: &Field) -> i32 {
    match field.quantum_numbers.color {
        crate::ontology::ColorRepresentation::Singlet => 1,
        crate::ontology::ColorRepresentation::Triplet => 3,
        crate::ontology::ColorRepresentation::AntiTriplet => -3,
        crate::ontology::ColorRepresentation::Octet => 8,
    }
}

/// Create a Python-safe variable name from a field ID.
fn ufo_variable_name(id: &str) -> String {
    id.replace('-', "m")
        .replace('+', "p")
        .replace('_', "_")
        .replace(' ', "_")
}

/// Generate an antiparticle name from the field ID.
fn ufo_antiname(id: &str) -> String {
    // Common patterns
    if id.ends_with('-') {
        return format!("{}+", &id[..id.len() - 1]);
    }
    if id.ends_with('+') {
        return format!("{}-", &id[..id.len() - 1]);
    }
    if id.starts_with("anti_") {
        return id[5..].to_string();
    }
    format!("anti_{}", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::*;

    // --- Helper: build minimal Fields for testing ---

    fn electron_field() -> Field {
        Field {
            id: "e-".into(),
            name: "Electron".into(),
            symbol: "e^-".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn photon_field() -> Field {
        Field {
            id: "photon".into(),
            name: "Photon".into(),
            symbol: "γ".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(2),
                parity: Parity::Odd,
                charge_conjugation: ChargeConjugation::Odd,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn z_boson_field() -> Field {
        Field {
            id: "Z0".into(),
            name: "Z Boson".into(),
            symbol: "Z^0".into(),
            mass: 91.1876,
            width: 2.4952,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(2),
                parity: Parity::Odd,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::WeakNC],
        }
    }

    fn higgs_field() -> Field {
        Field {
            id: "H".into(),
            name: "Higgs".into(),
            symbol: "H".into(),
            mass: 125.1,
            width: 0.00407,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(0),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Even,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::Yukawa],
        }
    }

    fn make_test_model() -> TheoreticalModel {
        let fields = vec![electron_field(), photon_field(), z_boson_field(), higgs_field()];
        let terms = vec![LagrangianTerm {
            id: "qed_eea".into(),
            description: "QED vertex".into(),
            coupling_symbol: "e".into(),
            coupling_value: Some(0.303),
            field_ids: vec!["e-".into(), "photon".into(), "e-".into()],
            lorentz_structure: "gamma_mu".into(),
            interaction_type: InteractionType::Electromagnetic,
            term_kind: LagrangianTermKind::Interaction,
            operator_dimension: None,
        }];
        // Build model with derived propagators and vertex factors.
        let propagators = fields.iter().map(|f| derive_propagator(f).unwrap()).collect();
        let vertex_factors = terms.iter().map(|t| derive_vertex_factor(t).unwrap()).collect();
        TheoreticalModel {
            name: "Test Model".into(),
            description: "Minimal QED for unit tests".into(),
            fields,
            terms,
            vertex_factors,
            propagators,
            gauge_symmetry: None,
        }
    }

    // --- Propagator derivation tests ---

    #[test]
    fn propagator_scalar() {
        let prop = derive_propagator(&higgs_field()).unwrap();
        assert_eq!(prop.form, PropagatorForm::Scalar);
        assert_eq!(prop.spin, Spin(0));
        assert!(prop.gauge_parameter.is_none());
    }

    #[test]
    fn propagator_dirac_fermion() {
        let prop = derive_propagator(&electron_field()).unwrap();
        assert_eq!(prop.form, PropagatorForm::DiracFermion);
        assert_eq!(prop.spin, Spin(1));
        assert!(prop.expression.contains("γ·p"));
    }

    #[test]
    fn propagator_massless_vector() {
        let prop = derive_propagator(&photon_field()).unwrap();
        assert_eq!(prop.form, PropagatorForm::MasslessVector);
        assert_eq!(prop.spin, Spin(2));
        assert!(prop.gauge_parameter.is_some());
        assert!((prop.gauge_parameter.unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn propagator_massive_vector() {
        let prop = derive_propagator(&z_boson_field()).unwrap();
        assert_eq!(prop.form, PropagatorForm::MassiveVector);
        assert!(prop.expression.contains("p_μ p_ν"));
    }

    #[test]
    fn massive_vs_massless_vector_differ() {
        let photon_prop = derive_propagator(&photon_field()).unwrap();
        let z_prop = derive_propagator(&z_boson_field()).unwrap();
        assert_ne!(photon_prop.form, z_prop.form);
    }

    // --- Vertex factor derivation tests ---

    #[test]
    fn vertex_factor_qed() {
        let term = LagrangianTerm {
            id: "qed".into(),
            description: "QED".into(),
            coupling_symbol: "e".into(),
            coupling_value: Some(0.303),
            field_ids: vec!["e-".into(), "photon".into(), "e-".into()],
            lorentz_structure: "gamma_mu".into(),
            interaction_type: InteractionType::Electromagnetic,
            term_kind: LagrangianTermKind::Interaction,
            operator_dimension: None,
        };
        let vf = derive_vertex_factor(&term).unwrap();
        assert_eq!(vf.n_legs, 3);
        assert!(vf.expression.contains("e"));
        assert!(vf.expression.contains("γ^μ"));
    }

    #[test]
    fn vertex_factor_scalar_coupling() {
        let term = LagrangianTerm {
            id: "yuk".into(),
            description: "Yukawa".into(),
            coupling_symbol: "y_e".into(),
            coupling_value: Some(2.94e-6),
            field_ids: vec!["e-".into(), "e-".into(), "H".into()],
            lorentz_structure: "scalar".into(),
            interaction_type: InteractionType::Yukawa,
            term_kind: LagrangianTermKind::Interaction,
            operator_dimension: None,
        };
        let vf = derive_vertex_factor(&term).unwrap();
        assert!(vf.expression.contains("y_e"));
        assert!(!vf.expression.contains("γ"));
    }

    // --- Model-level tests ---

    #[test]
    fn derive_propagators_for_model() {
        let model = make_test_model();
        let props = derive_propagators(&model).unwrap();
        assert_eq!(props.len(), model.fields.len());
    }

    #[test]
    fn derive_vertex_rules_for_model() {
        let model = make_test_model();
        let rules = derive_vertex_rules(&model).unwrap();
        assert_eq!(rules.len(), 1); // only one interaction term in test model
    }

    #[test]
    fn lookup_vertex_found() {
        let model = make_test_model();
        let vf = lookup_vertex(&model, &["e-", "photon", "e-"]).unwrap();
        assert_eq!(vf.term_id, "qed_eea");
    }

    #[test]
    fn lookup_vertex_order_independent() {
        let model = make_test_model();
        let vf = lookup_vertex(&model, &["photon", "e-", "e-"]).unwrap();
        assert_eq!(vf.term_id, "qed_eea");
    }

    #[test]
    fn lookup_vertex_not_found() {
        let model = make_test_model();
        let result = lookup_vertex(&model, &["u", "g", "u"]);
        assert!(result.is_err());
    }

    #[test]
    fn lookup_propagator_found() {
        let model = make_test_model();
        let prop = lookup_propagator(&model, "photon").unwrap();
        assert_eq!(prop.form, PropagatorForm::MasslessVector);
    }

    #[test]
    fn lookup_propagator_not_found() {
        let model = make_test_model();
        let result = lookup_propagator(&model, "graviton");
        assert!(result.is_err());
    }

    #[test]
    fn term_kind_variants() {
        assert_ne!(LagrangianTermKind::Kinetic, LagrangianTermKind::Interaction);
    }

    #[test]
    fn vertex_factor_serde() {
        let vf = VertexFactor {
            term_id: "test".into(),
            field_ids: vec!["e-".into(), "photon".into()],
            expression: "-ie gamma^mu".into(),
            coupling_value: Some(0.303),
            n_legs: 3,
        };
        let json = serde_json::to_string(&vf).unwrap();
        let vf2: VertexFactor = serde_json::from_str(&json).unwrap();
        assert_eq!(vf2.n_legs, 3);
    }

    // --- EFT / Contact interaction vertex factor tests ---

    #[test]
    fn vertex_factor_contact_vma() {
        let term = LagrangianTerm {
            id: "eft_4f".into(),
            description: "4-fermion V-A contact".into(),
            coupling_symbol: "C_LL".into(),
            coupling_value: Some(1.0),
            field_ids: vec!["e-".into(), "e-".into(), "mu-".into(), "mu-".into()],
            lorentz_structure: "contact_VmA".into(),
            interaction_type: InteractionType::Electromagnetic,
            term_kind: LagrangianTermKind::ContactInteraction,
            operator_dimension: Some(6),
        };
        let vf = derive_vertex_factor(&term).unwrap();
        assert_eq!(vf.n_legs, 4);
        assert!(vf.expression.contains("C_LL"));
        assert!(vf.expression.contains("Λ²"));
        assert!(vf.expression.contains("P_L"));
    }

    #[test]
    fn vertex_factor_contact_4f_generic() {
        let term = LagrangianTerm {
            id: "eft_generic".into(),
            description: "Generic 4-fermion contact".into(),
            coupling_symbol: "G_F".into(),
            coupling_value: Some(1.166e-5),
            field_ids: vec!["e-".into(), "e-".into(), "e-".into(), "e-".into()],
            lorentz_structure: "contact_4f".into(),
            interaction_type: InteractionType::Electromagnetic,
            term_kind: LagrangianTermKind::ContactInteraction,
            operator_dimension: Some(6),
        };
        let vf = derive_vertex_factor(&term).unwrap();
        assert_eq!(vf.n_legs, 4);
        assert!(vf.expression.contains("G_F"));
        assert!(vf.expression.contains("Λ²"));
    }

    #[test]
    fn contact_interaction_included_in_derive_vertex_rules() {
        let fields = vec![electron_field(), photon_field()];
        let terms = vec![
            LagrangianTerm {
                id: "qed_eea".into(),
                description: "QED vertex".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["e-".into(), "photon".into(), "e-".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            LagrangianTerm {
                id: "eft_contact".into(),
                description: "EFT contact".into(),
                coupling_symbol: "C".into(),
                coupling_value: Some(1.0),
                field_ids: vec!["e-".into(), "e-".into(), "e-".into(), "e-".into()],
                lorentz_structure: "contact_4f".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::ContactInteraction,
                operator_dimension: Some(6),
            },
        ];
        let propagators = fields.iter().map(|f| derive_propagator(f).unwrap()).collect();
        let model = TheoreticalModel {
            name: "EFT Test".into(),
            description: "Test".into(),
            fields,
            terms,
            vertex_factors: vec![], // empty — should be derived
            propagators,
            gauge_symmetry: None,
        };
        let rules = derive_vertex_rules(&model).unwrap();
        assert_eq!(rules.len(), 2, "Both Interaction and ContactInteraction should produce vertex rules");
    }

    #[test]
    fn propagator_form_serde() {
        let prop = derive_propagator(&electron_field()).unwrap();
        let json = serde_json::to_string(&prop).unwrap();
        let prop2: Propagator = serde_json::from_str(&json).unwrap();
        assert_eq!(prop2.form, PropagatorForm::DiracFermion);
    }

    // --- UFO Export Tests ---

    #[test]
    fn ufo_particles_py_header() {
        let model = make_test_model();
        let output = model.to_ufo_particles_py();
        assert!(output.contains("from object_library import all_particles, Particle"));
        assert!(output.contains("# Model: Test Model"));
    }

    #[test]
    fn ufo_particles_py_contains_all_fields() {
        let model = make_test_model();
        let output = model.to_ufo_particles_py();
        // 4 fields in the test model: electron, photon, Z0, higgs
        assert!(output.contains("name='e-'"), "should contain electron");
        assert!(output.contains("name='photon'"), "should contain photon");
        assert!(output.contains("name='Z0'"), "should contain Z boson");
        assert!(output.contains("name='H'"), "should contain Higgs");
    }

    #[test]
    fn ufo_particles_py_pdg_codes() {
        let model = make_test_model();
        let output = model.to_ufo_particles_py();
        assert!(output.contains("pdg_code=11"), "electron should have PDG 11: {}", output);
        assert!(output.contains("pdg_code=22"), "photon should have PDG 22");
        assert!(output.contains("pdg_code=23"), "Z should have PDG 23");
        assert!(output.contains("pdg_code=25"), "Higgs should have PDG 25");
    }

    #[test]
    fn ufo_particles_py_spin_codes() {
        let model = make_test_model();
        let output = model.to_ufo_particles_py();
        // Higgs: spin=0 → 2J+1=1, Electron: spin=1(=2*1/2) → 2J+1=2,
        // Photon/Z: spin=2(=2*1) → 2J+1=3
        assert!(output.contains("name='H'") && output.contains("spin=1"), "Higgs spin=1: {}", output);
    }

    #[test]
    fn ufo_particles_py_width() {
        let model = make_test_model();
        let output = model.to_ufo_particles_py();
        // Z has width > 0, photon has width = 0
        assert!(output.contains("width='ZERO'"), "massless particles should have ZERO width");
    }

    #[test]
    fn ufo_parameters_py_header() {
        let model = make_test_model();
        let output = model.to_ufo_parameters_py();
        assert!(output.contains("from object_library import all_parameters, Parameter"));
        assert!(output.contains("# Model: Test Model"));
    }

    #[test]
    fn ufo_parameters_py_masses() {
        let model = make_test_model();
        let output = model.to_ufo_parameters_py();
        assert!(output.contains("block='MASS'"), "should have MASS block parameters");
        // Electron mass
        assert!(output.contains("0.000511"), "should contain electron mass");
        // Higgs mass
        assert!(output.contains("125.1"), "should contain Higgs mass: {}", output);
    }

    #[test]
    fn ufo_parameters_py_couplings() {
        let model = make_test_model();
        let output = model.to_ufo_parameters_py();
        assert!(output.contains("nature='internal'"), "should have internal coupling params");
        assert!(output.contains("name='e'"), "should contain QED coupling 'e': {}", output);
    }

    #[test]
    fn ufo_parameters_py_width_for_z() {
        let model = make_test_model();
        let output = model.to_ufo_parameters_py();
        assert!(output.contains("block='DECAY'"), "Z boson should have DECAY block: {}", output);
        assert!(output.contains("2.4952"), "should contain Z width");
    }

    #[test]
    fn ufo_full_export_returns_three_files() {
        let model = make_test_model();
        let files = model.to_ufo();
        assert!(files.contains_key("particles.py"));
        assert!(files.contains_key("parameters.py"));
        assert!(files.contains_key("__init__.py"));
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn ufo_init_py_content() {
        let model = make_test_model();
        let files = model.to_ufo();
        let init = &files["__init__.py"];
        assert!(init.contains("from . import particles"));
        assert!(init.contains("from . import parameters"));
        assert!(init.contains("Test Model"));
    }

    #[test]
    fn ufo_antiname_patterns() {
        assert_eq!(ufo_antiname("e-"), "e+");
        assert_eq!(ufo_antiname("W+"), "W-");
        assert_eq!(ufo_antiname("anti_u"), "u");
        assert_eq!(ufo_antiname("photon"), "anti_photon");
    }

    #[test]
    fn ufo_variable_name_sanitization() {
        assert_eq!(ufo_variable_name("e-"), "em");
        assert_eq!(ufo_variable_name("W+"), "Wp");
        assert_eq!(ufo_variable_name("Z0"), "Z0");
    }
}
