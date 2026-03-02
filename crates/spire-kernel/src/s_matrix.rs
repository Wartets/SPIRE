//! # S-Matrix — Process Construction and Validation
//!
//! This module implements the high-level logic for constructing and validating
//! physical scattering and decay processes within the S-matrix framework.
//!
//! A physical process is defined as a transition between asymptotic states in
//! Hilbert space. This module serves as the primary entry point for a
//! calculation: the user specifies initial and final states, and the engine
//! orchestrates validation (via `groups`), diagram generation (via `graph`),
//! and amplitude construction (via `algebra`).

use serde::{Deserialize, Serialize};

use crate::groups;
use crate::lagrangian::TheoreticalModel;
use crate::ontology::{InteractionType, Particle, QuantumState};
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Core Data Structures
// ---------------------------------------------------------------------------

/// An **AsymptoticState** represents the complete collection of particles in
/// either the infinite past (initial state) or infinite future (final state).
///
/// Corresponds to a multi-particle Fock-space state
/// $|p_1, \sigma_1; p_2, \sigma_2; \ldots\rangle$.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsymptoticState {
    /// The ordered list of quantum states comprising this asymptotic state.
    pub states: Vec<QuantumState>,
    /// Total invariant mass squared $s = (p_1 + p_2 + \cdots)^2$ in GeV².
    pub invariant_mass_sq: f64,
}

/// A **Reaction** is the full specification of a physical process:
/// initial state → final state, together with all derived information
/// (validity, interaction type, mediating bosons).
///
/// This is the top-level serializable result object that flows from the
/// kernel to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    /// The initial (incoming) asymptotic state.
    pub initial: AsymptoticState,
    /// The final (outgoing) asymptotic state.
    pub final_state: AsymptoticState,
    /// Whether the reaction passes all conservation-law checks.
    pub is_valid: bool,
    /// If invalid, detailed diagnostics listing every violated symmetry.
    pub violation_diagnostics: Vec<String>,
    /// The interaction type(s) that can mediate this process.
    pub interaction_types: Vec<InteractionType>,
    /// Mediating gauge bosons identified for this transition.
    pub mediating_bosons: Vec<Particle>,
    /// The perturbative order requested for diagram generation.
    pub perturbative_order: u32,
}

/// A summary of a possible final state returned by the reaction reconstruction
/// engine when the user provides an incomplete process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedFinalState {
    /// The list of particles completing the final state.
    pub particles: Vec<Particle>,
    /// Relative probability ranking (based on phase-space volume and coupling).
    pub weight: f64,
    /// The interaction chain required to produce this final state.
    pub interaction_chain: Vec<InteractionType>,
}

/// Enumeration of Standard Model gauge bosons that can mediate a transition.
///
/// Returned by [`identify_mediating_bosons`] after analysing the quantum-number
/// flow between initial and final fermion states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MediatingBoson {
    /// Photon ($\gamma$) — mediates electromagnetic interactions.
    Photon,
    /// $W^+$ boson — mediates weak charged-current interactions ($\Delta Q > 0$).
    WPlus,
    /// $W^-$ boson — mediates weak charged-current interactions ($\Delta Q < 0$).
    WMinus,
    /// $Z^0$ boson — mediates weak neutral-current interactions.
    Z0,
    /// Gluon ($g$) — mediates strong interactions (colour change).
    Gluon,
    /// Higgs boson ($H$) — mediates Yukawa interactions.
    Higgs,
}

impl MediatingBoson {
    /// Return the canonical particle-database identifier for this boson.
    pub fn field_id(&self) -> &'static str {
        match self {
            MediatingBoson::Photon => "photon",
            MediatingBoson::WPlus => "W+",
            MediatingBoson::WMinus => "W-",
            MediatingBoson::Z0 => "Z0",
            MediatingBoson::Gluon => "g",
            MediatingBoson::Higgs => "H",
        }
    }
}

// ---------------------------------------------------------------------------
// Reaction Validation
// ---------------------------------------------------------------------------

impl Reaction {
    /// Construct and validate a `Reaction` from initial and final asymptotic states.
    ///
    /// Tries every fundamental interaction type against
    /// [`groups::validate_conservation_laws`]. If **any** interaction type
    /// satisfies all conservation laws the reaction is considered valid
    /// (pending a kinematic threshold check on $\sqrt{s}$).
    ///
    /// # Returns
    /// A fully populated `Reaction` with validity flag, violation diagnostics,
    /// and the list of compatible interaction types.
    pub fn validate(
        initial: AsymptoticState,
        final_state: AsymptoticState,
    ) -> SpireResult<Self> {
        let interaction_types = [
            InteractionType::Electromagnetic,
            InteractionType::Strong,
            InteractionType::WeakNC,
            InteractionType::WeakCC,
            InteractionType::Yukawa,
            InteractionType::ScalarSelf,
        ];

        let mut valid_interactions = Vec::new();
        let mut all_violations = Vec::new();

        for &it in &interaction_types {
            let result = groups::validate_conservation_laws(
                &initial.states,
                &final_state.states,
                it,
            )?;
            if result.is_valid {
                valid_interactions.push(it);
            } else {
                for v in &result.violations {
                    if !all_violations.contains(v) {
                        all_violations.push(v.clone());
                    }
                }
            }
        }

        let mut violation_diagnostics = Vec::new();
        let is_valid = if valid_interactions.is_empty() {
            // No interaction type can mediate this process.
            violation_diagnostics = all_violations;
            false
        } else {
            // At least one interaction type works — check kinematics.
            let final_mass_sum: f64 = final_state
                .states
                .iter()
                .map(|s| s.particle.field.mass)
                .sum();
            let cms_energy = initial.invariant_mass_sq.sqrt();

            if cms_energy > 1e-10 && final_mass_sum > cms_energy {
                violation_diagnostics.push(format!(
                    "Kinematically forbidden: \u{221a}s = {:.6} GeV < sum of final masses = {:.6} GeV",
                    cms_energy, final_mass_sum
                ));
                false
            } else {
                true
            }
        };

        Ok(Reaction {
            initial,
            final_state,
            is_valid,
            violation_diagnostics,
            interaction_types: valid_interactions,
            mediating_bosons: vec![],
            perturbative_order: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Public API — Reaction Construction
// ---------------------------------------------------------------------------

/// Construct and fully validate a `Reaction` by resolving particle identifiers
/// from a [`TheoreticalModel`].
///
/// 1. Looks up each ID in the model's field list.
/// 2. Builds [`AsymptoticState`]s with simplified 4-momenta.
/// 3. Delegates to [`Reaction::validate`].
///
/// # Arguments
/// * `initial_ids` — Particle identifier strings for the initial state.
/// * `final_ids`   — Particle identifier strings for the final state.
/// * `model`       — The active theoretical model providing field definitions.
/// * `cms_energy`  — Centre-of-mass energy in GeV (optional).
pub fn construct_reaction(
    initial_ids: &[&str],
    final_ids: &[&str],
    model: &TheoreticalModel,
    cms_energy: Option<f64>,
) -> SpireResult<Reaction> {
    let lookup = |id: &str| -> SpireResult<&crate::ontology::Field> {
        model
            .fields
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| SpireError::UnknownParticle(id.to_string()))
    };

    let energy = cms_energy.unwrap_or(0.0);
    let n_init = initial_ids.len().max(1) as f64;
    let n_final = final_ids.len().max(1) as f64;

    let initial_states: Vec<QuantumState> = initial_ids
        .iter()
        .map(|&id| {
            let field = lookup(id)?;
            crate::ontology::initialize_state(field, [energy / n_init, 0.0, 0.0, 0.0], None)
        })
        .collect::<SpireResult<Vec<_>>>()?;

    let final_states: Vec<QuantumState> = final_ids
        .iter()
        .map(|&id| {
            let field = lookup(id)?;
            crate::ontology::initialize_state(field, [energy / n_final, 0.0, 0.0, 0.0], None)
        })
        .collect::<SpireResult<Vec<_>>>()?;

    let s = energy * energy;
    let initial = AsymptoticState {
        states: initial_states,
        invariant_mass_sq: s,
    };
    let final_state = AsymptoticState {
        states: final_states,
        invariant_mass_sq: s,
    };

    Reaction::validate(initial, final_state)
}

/// Reconstruct all kinematically and dynamically permissible **two-body**
/// final states given an initial state, a theoretical model, and the
/// available centre-of-mass energy.
///
/// The engine iterates through all ordered pairs of fields defined in the
/// model, checks conservation laws under every fundamental interaction type,
/// and filters out kinematically forbidden channels.
///
/// # Arguments
/// * `initial_states`   — The initial-state particles.
/// * `model`            — The active theoretical model.
/// * `available_energy` — Centre-of-mass energy in GeV.
pub fn reconstruct_reaction(
    initial_states: &[Particle],
    model: &TheoreticalModel,
    available_energy: f64,
) -> SpireResult<Vec<ReconstructedFinalState>> {
    let mut results = Vec::new();

    // Build QuantumStates for the initial particles (mock momenta).
    let e_per = available_energy / initial_states.len().max(1) as f64;
    let initial_qs: Vec<QuantumState> = initial_states
        .iter()
        .map(|p| QuantumState {
            particle: p.clone(),
            four_momentum: [e_per, 0.0, 0.0, 0.0],
            normalization: 2.0 * e_per,
        })
        .collect();

    let n_fields = model.fields.len();

    // Iterate all pairs (i, j) with j >= i (avoids double-counting).
    for i in 0..n_fields {
        for j in i..n_fields {
            let f1 = &model.fields[i];
            let f2 = &model.fields[j];

            // --- Cheapest check first: kinematic threshold ---
            let mass_sum = f1.mass + f2.mass;
            if mass_sum > available_energy {
                continue;
            }

            let p1 = crate::ontology::particle_from_field(f1.clone());
            let p2 = crate::ontology::particle_from_field(f2.clone());

            // Build mock final-state QuantumStates.
            let final_qs = vec![
                QuantumState {
                    particle: p1.clone(),
                    four_momentum: [available_energy / 2.0, 0.0, 0.0, 0.0],
                    normalization: available_energy,
                },
                QuantumState {
                    particle: p2.clone(),
                    four_momentum: [available_energy / 2.0, 0.0, 0.0, 0.0],
                    normalization: available_energy,
                },
            ];

            // Try each fundamental interaction type.
            let interactions = [
                InteractionType::Electromagnetic,
                InteractionType::WeakCC,
                InteractionType::WeakNC,
                InteractionType::Strong,
            ];

            let mut valid_interactions = Vec::new();
            for &interaction in &interactions {
                let result =
                    groups::validate_conservation_laws(&initial_qs, &final_qs, interaction)?;
                if result.is_valid {
                    valid_interactions.push(interaction);
                }
            }

            if !valid_interactions.is_empty() {
                let weight = (available_energy - mass_sum).max(0.0);
                results.push(ReconstructedFinalState {
                    particles: vec![p1, p2],
                    weight,
                    interaction_chain: valid_interactions,
                });
            }
        }
    }

    // Sort by phase-space weight descending.
    results.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

/// Identify the mediating gauge bosons for a single fermion-line transition.
///
/// Compares the quantum numbers of the initial and final particles on a
/// single fermion line to determine which intermediate boson(s) are
/// required to balance the quantum-number flow.
///
/// # Rules
/// * $\Delta Q \neq 0$ → $W^\pm$ (charged current).
/// * $\Delta Q = 0$, colour change → gluon.
/// * $\Delta Q = 0$, charged particle → photon **and/or** $Z^0$.
/// * $\Delta Q = 0$, neutral particle → $Z^0$ only.
///
/// # Arguments
/// * `initial`        — The incoming fermion.
/// * `final_particle` — The outgoing fermion.
pub fn identify_mediating_bosons(
    initial: &Particle,
    final_particle: &Particle,
) -> SpireResult<Vec<MediatingBoson>> {
    let qi = initial.field.quantum_numbers.electric_charge.0 as i32;
    let qf = final_particle.field.quantum_numbers.electric_charge.0 as i32;
    let ci = initial.field.quantum_numbers.color;
    let cf = final_particle.field.quantum_numbers.color;

    let delta_q = qi - qf; // in units of e/3

    let mut bosons = Vec::new();

    // Colour change requires a gluon.
    if ci != cf {
        bosons.push(MediatingBoson::Gluon);
    }

    // Charge change requires a W boson.
    if delta_q != 0 {
        if delta_q > 0 {
            bosons.push(MediatingBoson::WPlus);
        } else {
            bosons.push(MediatingBoson::WMinus);
        }
    } else {
        // ΔQ = 0: electromagnetic (photon) and/or weak neutral current (Z).
        if qi != 0 {
            // Charged particle → photon is possible.
            bosons.push(MediatingBoson::Photon);
        }
        // Z couples to all SM fermions (charged and neutral).
        bosons.push(MediatingBoson::Z0);
    }

    Ok(bosons)
}

/// Classify the interaction type(s) that can mediate a proposed transition.
///
/// Tries every fundamental interaction type with the conservation-law
/// validator and returns those that satisfy all selection rules.
pub fn classify_interaction(
    initial: &AsymptoticState,
    final_state: &AsymptoticState,
) -> SpireResult<Vec<InteractionType>> {
    let interaction_types = [
        InteractionType::Electromagnetic,
        InteractionType::Strong,
        InteractionType::WeakNC,
        InteractionType::WeakCC,
        InteractionType::Yukawa,
        InteractionType::ScalarSelf,
    ];

    let mut compatible = Vec::new();
    for &it in &interaction_types {
        let result =
            groups::validate_conservation_laws(&initial.states, &final_state.states, it)?;
        if result.is_valid {
            compatible.push(it);
        }
    }

    Ok(compatible)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrangian::TheoreticalModel;
    use crate::ontology::*;

    // -----------------------------------------------------------------------
    // Test helpers — field constructors
    // -----------------------------------------------------------------------

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
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakCC],
        }
    }

    fn positron_field() -> Field {
        Field {
            id: "e+".into(),
            name: "Positron".into(),
            symbol: "e^+".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: -1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakCC],
        }
    }

    fn muon_field() -> Field {
        Field {
            id: "mu-".into(),
            name: "Muon".into(),
            symbol: "\u{03bc}^-".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakCC],
        }
    }

    fn antimuon_field() -> Field {
        Field {
            id: "mu+".into(),
            name: "Antimuon".into(),
            symbol: "\u{03bc}^+".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: -1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakCC],
        }
    }

    fn photon_field() -> Field {
        Field {
            id: "photon".into(),
            name: "Photon".into(),
            symbol: "\u{03b3}".into(),
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

    fn nu_e_field() -> Field {
        Field {
            id: "nu_e".into(),
            name: "Electron Neutrino".into(),
            symbol: "\u{03bd}_e".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::WeakCC, InteractionType::WeakNC],
        }
    }

    fn up_field() -> Field {
        Field {
            id: "u".into(),
            name: "Up Quark".into(),
            symbol: "u".into(),
            mass: 0.0023,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(2),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(1),
                baryon_number: BaryonNumber(1),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Triplet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::Strong],
        }
    }

    fn down_field() -> Field {
        Field {
            id: "d".into(),
            name: "Down Quark".into(),
            symbol: "d".into(),
            mass: 0.0048,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-1),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(1),
                baryon_number: BaryonNumber(1),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Triplet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::Strong],
        }
    }

    /// Build a minimal test model containing fundamental SM particles.
    fn make_test_model() -> TheoreticalModel {
        TheoreticalModel {
            name: "Test SM".into(),
            description: "Minimal SM for s_matrix tests".into(),
            fields: vec![
                electron_field(),
                positron_field(),
                muon_field(),
                antimuon_field(),
                photon_field(),
                nu_e_field(),
                up_field(),
                down_field(),
            ],
            terms: vec![],
            vertex_factors: vec![],
            propagators: vec![],
            gauge_symmetry: None,
            spacetime: crate::algebra::SpacetimeConfig::default(),
            constants: crate::ontology::PhysicalConstants::default(),
        }
    }

    // -----------------------------------------------------------------------
    // Reaction validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn reaction_struct_is_serializable() {
        fn _assert_serialize<T: serde::Serialize + serde::de::DeserializeOwned>() {}
        _assert_serialize::<Reaction>();
    }

    #[test]
    fn validate_ee_to_mumu_valid() {
        // e+ e- → μ+ μ- at 10 GeV — valid EM process.
        let model = make_test_model();
        let rxn =
            construct_reaction(&["e-", "e+"], &["mu-", "mu+"], &model, Some(10.0)).unwrap();
        assert!(rxn.is_valid, "e+e- → μ+μ- should be valid: {:?}", rxn.violation_diagnostics);
        assert!(
            rxn.interaction_types.contains(&InteractionType::Electromagnetic),
            "Should be compatible with EM"
        );
    }

    #[test]
    fn validate_ee_to_photons_valid() {
        let model = make_test_model();
        let rxn =
            construct_reaction(&["e-", "e+"], &["photon", "photon"], &model, Some(10.0)).unwrap();
        assert!(rxn.is_valid, "e+e- → γγ should be valid: {:?}", rxn.violation_diagnostics);
    }

    #[test]
    fn validate_charge_violation_rejected() {
        // e- → γ alone violates charge conservation.
        let model = make_test_model();
        let rxn = construct_reaction(&["e-"], &["photon"], &model, None).unwrap();
        assert!(!rxn.is_valid);
        assert!(
            rxn.violation_diagnostics
                .iter()
                .any(|v| v.contains("Electric charge")),
            "Should report charge violation"
        );
    }

    #[test]
    fn validate_kinematically_forbidden() {
        // e+ e- at 0.001 GeV (1 MeV) cannot produce μ+ μ- (mass sum ~0.211 GeV).
        let model = make_test_model();
        let rxn =
            construct_reaction(&["e-", "e+"], &["mu-", "mu+"], &model, Some(0.001)).unwrap();
        assert!(!rxn.is_valid);
        assert!(
            rxn.violation_diagnostics
                .iter()
                .any(|v| v.contains("Kinematically forbidden")),
            "Should report kinematic threshold: {:?}",
            rxn.violation_diagnostics
        );
    }

    #[test]
    fn validate_unknown_particle_error() {
        let model = make_test_model();
        let result = construct_reaction(&["e-"], &["graviton"], &model, None);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Reconstruction tests
    // -----------------------------------------------------------------------

    #[test]
    fn reconstruct_ee_10gev_finds_mumu() {
        let model = make_test_model();
        let electron = particle_from_field(electron_field());
        let positron = particle_from_field(positron_field());

        let results = reconstruct_reaction(&[electron, positron], &model, 10.0).unwrap();

        // μ+ μ- should appear.
        let has_mumu = results.iter().any(|r| {
            let ids: Vec<&str> = r.particles.iter().map(|p| p.field.id.as_str()).collect();
            ids.contains(&"mu-") && ids.contains(&"mu+")
        });
        assert!(has_mumu, "Reconstruction should find \u{03bc}+\u{03bc}- at 10 GeV");
    }

    #[test]
    fn reconstruct_ee_10gev_finds_photon_pair() {
        let model = make_test_model();
        let electron = particle_from_field(electron_field());
        let positron = particle_from_field(positron_field());

        let results = reconstruct_reaction(&[electron, positron], &model, 10.0).unwrap();

        // γγ should appear.
        let has_gg = results.iter().any(|r| {
            r.particles.iter().all(|p| p.field.id == "photon")
                && r.particles.len() == 2
        });
        assert!(has_gg, "Reconstruction should find \u{03b3}\u{03b3} at 10 GeV");
    }

    #[test]
    fn reconstruct_ee_10gev_rejects_e_muplus() {
        // e- \u{03bc}+ as final state violates lepton number.
        let model = make_test_model();
        let electron = particle_from_field(electron_field());
        let positron = particle_from_field(positron_field());

        let results = reconstruct_reaction(&[electron, positron], &model, 10.0).unwrap();

        let has_e_mu = results.iter().any(|r| {
            let ids: Vec<&str> = r.particles.iter().map(|p| p.field.id.as_str()).collect();
            ids.contains(&"e-") && ids.contains(&"mu+")
        });
        assert!(!has_e_mu, "e- \u{03bc}+ should be rejected (lepton number violation)");
    }

    #[test]
    fn reconstruct_ee_1mev_rejects_mumu() {
        // At 1 MeV (0.001 GeV), \u{03bc}+\u{03bc}- is kinematically forbidden.
        let model = make_test_model();
        let electron = particle_from_field(electron_field());
        let positron = particle_from_field(positron_field());

        let results = reconstruct_reaction(&[electron, positron], &model, 0.001).unwrap();

        let has_mumu = results.iter().any(|r| {
            let ids: Vec<&str> = r.particles.iter().map(|p| p.field.id.as_str()).collect();
            ids.contains(&"mu-") && ids.contains(&"mu+")
        });
        assert!(!has_mumu, "\u{03bc}+\u{03bc}- should be rejected at 1 MeV");
    }

    #[test]
    fn reconstruct_ee_results_sorted_by_weight() {
        let model = make_test_model();
        let electron = particle_from_field(electron_field());
        let positron = particle_from_field(positron_field());

        let results = reconstruct_reaction(&[electron, positron], &model, 10.0).unwrap();

        // Verify descending weight order.
        for w in results.windows(2) {
            assert!(
                w[0].weight >= w[1].weight,
                "Results should be sorted by phase-space weight"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Mediating boson identification tests
    // -----------------------------------------------------------------------

    #[test]
    fn mediator_e_to_nue_is_w_minus() {
        // e- → \u{03bd}_e requires W- (\u{0394}Q = -3 in scaled units).
        let e = particle_from_field(electron_field());
        let nu = particle_from_field(nu_e_field());
        let bosons = identify_mediating_bosons(&e, &nu).unwrap();
        assert!(
            bosons.contains(&MediatingBoson::WMinus),
            "e- → \u{03bd}_e should require W-: {:?}",
            bosons
        );
    }

    #[test]
    fn mediator_u_to_d_is_w_plus() {
        // u → d requires W+ (\u{0394}Q = +3 in scaled units).
        let u = particle_from_field(up_field());
        let d = particle_from_field(down_field());
        let bosons = identify_mediating_bosons(&u, &d).unwrap();
        assert!(
            bosons.contains(&MediatingBoson::WPlus),
            "u → d should require W+: {:?}",
            bosons
        );
    }

    #[test]
    fn mediator_e_to_e_is_photon_and_z() {
        // e- → e- (elastic): photon and Z are both possible.
        let e = particle_from_field(electron_field());
        let bosons = identify_mediating_bosons(&e, &e).unwrap();
        assert!(
            bosons.contains(&MediatingBoson::Photon),
            "e- → e- should allow photon: {:?}",
            bosons
        );
        assert!(
            bosons.contains(&MediatingBoson::Z0),
            "e- → e- should allow Z0: {:?}",
            bosons
        );
    }

    #[test]
    fn mediator_nu_to_nu_is_z_only() {
        // \u{03bd}_e → \u{03bd}_e: neutral particle, only Z.
        let nu = particle_from_field(nu_e_field());
        let bosons = identify_mediating_bosons(&nu, &nu).unwrap();
        assert!(
            bosons.contains(&MediatingBoson::Z0),
            "\u{03bd}_e → \u{03bd}_e should allow Z0"
        );
        assert!(
            !bosons.contains(&MediatingBoson::Photon),
            "\u{03bd}_e → \u{03bd}_e should NOT allow photon (neutral)"
        );
    }

    #[test]
    fn mediator_quark_color_change_includes_gluon() {
        // Triplet → Singlet colour change requires a gluon.
        let u = particle_from_field(up_field()); // Triplet
        let photon_p = particle_from_field(photon_field()); // Singlet
        let bosons = identify_mediating_bosons(&u, &photon_p).unwrap();
        assert!(
            bosons.contains(&MediatingBoson::Gluon),
            "Colour change should require gluon: {:?}",
            bosons
        );
    }

    // -----------------------------------------------------------------------
    // classify_interaction tests
    // -----------------------------------------------------------------------

    #[test]
    fn classify_ee_to_mumu() {
        let model = make_test_model();
        let rxn = construct_reaction(&["e-", "e+"], &["mu-", "mu+"], &model, Some(10.0)).unwrap();
        // e+e- → μ+μ- is compatible with EM and WeakNC.
        let types = classify_interaction(&rxn.initial, &rxn.final_state).unwrap();
        assert!(
            types.contains(&InteractionType::Electromagnetic),
            "Should be EM-compatible: {:?}",
            types
        );
    }

    // -----------------------------------------------------------------------
    // MediatingBoson enum tests
    // -----------------------------------------------------------------------

    #[test]
    fn mediating_boson_field_ids() {
        assert_eq!(MediatingBoson::Photon.field_id(), "photon");
        assert_eq!(MediatingBoson::WPlus.field_id(), "W+");
        assert_eq!(MediatingBoson::WMinus.field_id(), "W-");
        assert_eq!(MediatingBoson::Z0.field_id(), "Z0");
        assert_eq!(MediatingBoson::Gluon.field_id(), "g");
        assert_eq!(MediatingBoson::Higgs.field_id(), "H");
    }

    #[test]
    fn mediating_boson_serde() {
        let mb = MediatingBoson::WMinus;
        let json = serde_json::to_string(&mb).unwrap();
        let mb2: MediatingBoson = serde_json::from_str(&json).unwrap();
        assert_eq!(mb, mb2);
    }
}
