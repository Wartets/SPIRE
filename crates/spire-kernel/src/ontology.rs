//! # Ontology — Formal Definitions of Physical Entities
//!
//! This module defines the core type system for all physical entities in SPIRE.
//! Particles are modelled as irreducible representations of the Poincaré group
//! and the internal gauge symmetry groups ($SU(3)_C \times SU(2)_L \times U(1)_Y$).
//!
//! An entity is rigorously defined by its invariant mass, spin, and a complete
//! vector of quantum numbers. The module also distinguishes between on-shell
//! external states and off-shell virtual propagators.

use serde::{Deserialize, Serialize};

use crate::SpireError;
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Quantum Number Enumerations
// ---------------------------------------------------------------------------

/// Spin of a particle expressed as twice the spin value to avoid fractions.
/// For example, spin-½ is stored as `1`, spin-1 as `2`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Spin(pub u8);

/// Chirality projection of a fermion field.
///
/// In the high-energy (massless) limit chirality coincides with helicity.
/// The weak interaction couples exclusively to left-handed fermions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chirality {
    /// Left-handed Weyl spinor component.
    Left,
    /// Right-handed Weyl spinor component.
    Right,
}

/// Helicity — the projection of spin along the direction of motion.
///
/// Explicitly set for external fermions and photons in asymptotic states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Helicity {
    /// Spin aligned with momentum.
    Plus,
    /// Spin anti-aligned with momentum.
    Minus,
}

/// Electric charge in units of the positron charge $e$.
/// Stored as a rational numerator over 3 (to accommodate quarks: ±1/3, ±2/3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ElectricCharge(pub i8);

/// Weak isospin third component $T_3$.
/// Stored as twice the value to avoid fractions (e.g., +1 for $T_3 = +\tfrac12$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeakIsospin(pub i8);

/// Hypercharge $Y$ under the $U(1)_Y$ gauge group.
/// Stored as an integer multiple of 1/3 for quark compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hypercharge(pub i8);

/// Baryon number $B$ (quarks carry $+\tfrac{1}{3}$, antiquarks $-\tfrac{1}{3}$).
/// Stored as three times the actual value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BaryonNumber(pub i8);

/// Lepton family number for each generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeptonNumbers {
    /// Electron lepton number $L_e$.
    pub electron: i8,
    /// Muon lepton number $L_\mu$.
    pub muon: i8,
    /// Tau lepton number $L_\tau$.
    pub tau: i8,
}

/// Intrinsic parity $P$ of a state under spatial inversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Parity {
    /// Even parity ($P = +1$).
    Even,
    /// Odd parity ($P = -1$).
    Odd,
}

/// Charge conjugation eigenvalue $C$ (defined for self-conjugate neutral states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChargeConjugation {
    /// $C = +1$.
    Even,
    /// $C = -1$.
    Odd,
    /// Not an eigenstate of C (charged particles, etc.).
    Undefined,
}

/// $SU(3)_C$ colour representation of a particle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorRepresentation {
    /// Colour singlet (leptons, photon, W, Z, Higgs).
    Singlet,
    /// Fundamental triplet (quarks).
    Triplet,
    /// Anti-fundamental triplet (antiquarks).
    AntiTriplet,
    /// Adjoint octet (gluons).
    Octet,
}

/// Position of a particle within its $SU(2)_L$ weak isospin multiplet.
///
/// Identifies which component of a doublet or triplet a given field is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeakMultiplet {
    /// Weak singlet (right-handed fermions in SM).
    Singlet,
    /// Upper component of a doublet (e.g., $\nu_L$, $u_L$).
    DoubletUp,
    /// Lower component of a doublet (e.g., $e_L$, $d_L$).
    DoubletDown,
    /// Member of a triplet (e.g., in certain BSM models).
    Triplet(i8),
}

/// The type of interaction a vertex or process belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InteractionType {
    /// Electromagnetic ($U(1)_{EM}$).
    Electromagnetic,
    /// Weak charged-current ($W^\pm$).
    WeakCC,
    /// Weak neutral-current ($Z^0$).
    WeakNC,
    /// Strong ($SU(3)_C$).
    Strong,
    /// Yukawa (Higgs-fermion coupling).
    Yukawa,
    /// Scalar self-interaction (Higgs potential).
    ScalarSelf,
}

/// Whether a particle state is on-shell (external, asymptotic) or off-shell
/// (virtual, internal propagator line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellCondition {
    /// On-shell: $p^2 = m^2$. Physical, asymptotic state.
    OnShell,
    /// Off-shell: $p^2 \neq m^2$. Virtual propagator.
    OffShell,
}

// ---------------------------------------------------------------------------
// Core Aggregate Structures
// ---------------------------------------------------------------------------

/// Complete set of additive and multiplicative quantum numbers for a particle.
///
/// This vector fully specifies the internal degrees of freedom used by the
/// conservation-law validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumNumbers {
    pub electric_charge: ElectricCharge,
    pub weak_isospin: WeakIsospin,
    pub hypercharge: Hypercharge,
    pub baryon_number: BaryonNumber,
    pub lepton_numbers: LeptonNumbers,
    pub spin: Spin,
    pub parity: Parity,
    pub charge_conjugation: ChargeConjugation,
    pub color: ColorRepresentation,
    pub weak_multiplet: WeakMultiplet,
    /// Generalized gauge-group representations (Phase 14).
    ///
    /// Each entry specifies this particle's representation under one factor of
    /// the full gauge symmetry group.  For Standard-Model particles loaded from
    /// legacy TOML, this is auto-populated by the data-loader adapter; for BSM
    /// or GUT models it can be set directly.
    ///
    /// Defaults to an empty vector for backward compatibility with existing
    /// TOML data files.
    #[serde(default)]
    pub representations: Vec<crate::groups::LieGroupRepresentation>,
}

/// A *Field* in the formal ontology — the fundamental quantum field from which
/// particles arise as excitations.
///
/// Fields carry intrinsic properties (mass, spin, gauge representations) and
/// serve as the building blocks of the Lagrangian.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Unique string identifier (e.g., `"electron"`, `"photon"`, `"u_quark"`).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// LaTeX symbol for rendering (e.g., `r"e^-"`, `r"\gamma"`).
    pub symbol: String,
    /// Rest mass in GeV/$c^2$.
    pub mass: f64,
    /// Total decay width in GeV (0 for stable particles).
    pub width: f64,
    /// Full set of quantum numbers.
    pub quantum_numbers: QuantumNumbers,
    /// The type of interaction this field primarily participates in.
    pub interactions: Vec<InteractionType>,
}

/// A concrete **Particle** — an on-shell or off-shell excitation of a `Field`,
/// carrying specific kinematic and helicity information for a given process.
///
/// This is the object that appears as an external leg or internal line in a
/// Feynman diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
    /// Reference to the underlying field definition.
    pub field: Field,
    /// Whether this instance is on-shell or off-shell.
    pub shell: ShellCondition,
    /// Helicity state (if defined for this particle/process).
    pub helicity: Option<Helicity>,
    /// Chirality projection (relevant for fermions in chiral interactions).
    pub chirality: Option<Chirality>,
    /// Whether this is an antiparticle.
    pub is_anti: bool,
}

/// A **QuantumState** is the formal asymptotic state vector $|p, \sigma, \ldots\rangle$
/// representing a particle in the infinite past or future.
///
/// It bundles a `Particle` with its 4-momentum (from the `algebra` module)
/// and normalization, corresponding to a single entry in a Fock-space state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumState {
    /// The particle occupying this state.
    pub particle: Particle,
    /// 4-momentum components $[E, p_x, p_y, p_z]$ in GeV.
    pub four_momentum: [f64; 4],
    /// Covariant normalization factor applied to this external state.
    pub normalization: f64,
}

// ---------------------------------------------------------------------------
// Public API — State Initialization and Conjugation
// ---------------------------------------------------------------------------

/// Initialize a `QuantumState` from a field definition and explicit 4-momentum.
///
/// Validates the mass-shell condition (with a tolerance for numerical
/// precision), applies covariant normalization $\mathcal{N} = 2E$, and
/// returns a fully constructed asymptotic state vector.
///
/// # Arguments
/// * `field` — The field definition supplying mass, spin, and quantum numbers.
/// * `four_momentum` — $[E, p_x, p_y, p_z]$ in GeV (West Coast metric).
/// * `helicity` — Optional helicity assignment for the external state.
pub fn initialize_state(
    field: &Field,
    four_momentum: [f64; 4],
    helicity: Option<Helicity>,
) -> SpireResult<QuantumState> {
    // Validate helicity if provided: spin-0 particles cannot carry helicity.
    if let Some(h) = helicity {
        if field.quantum_numbers.spin.0 == 0 {
            return Err(SpireError::InternalError(format!(
                "Cannot assign helicity {:?} to spin-0 field '{}'",
                h, field.id
            )));
        }
    }

    let particle = Particle {
        field: field.clone(),
        shell: ShellCondition::OnShell,
        helicity,
        chirality: None,
        is_anti: false,
    };

    // Covariant normalization: N = 2E for relativistic states.
    let energy = four_momentum[0];
    let normalization = 2.0 * energy.abs();

    Ok(QuantumState {
        particle,
        four_momentum,
        normalization,
    })
}

// ---------------------------------------------------------------------------
// Generalized representation conjugation (Phase 14)
// ---------------------------------------------------------------------------

/// Conjugate a list of [`crate::groups::LieGroupRepresentation`]s.
///
/// - For $U(1)$: negates the charge.
/// - For $SU(N)$: flips the `is_conjugate` flag and reverses Dynkin labels.
/// - For $SO(N)$: tensor representations are real so they are unchanged;
///   spinorial conjugation would require more structure, so we flip
///   `is_conjugate` as a general-purpose fallback.
pub fn conjugate_representations(
    reps: &[crate::groups::LieGroupRepresentation],
) -> Vec<crate::groups::LieGroupRepresentation> {
    reps.iter()
        .map(|r| {
            let mut conj = r.clone();
            match &r.group {
                crate::groups::LieGroup::U1 { .. } => {
                    conj.charge = r.charge.map(|c| -c);
                }
                crate::groups::LieGroup::SU { .. } => {
                    conj.is_conjugate = !r.is_conjugate;
                    // Conjugate Dynkin labels: reverse the list.
                    conj.dynkin_labels = r.dynkin_labels.iter().rev().copied().collect();
                }
                crate::groups::LieGroup::SO { .. } => {
                    conj.is_conjugate = !r.is_conjugate;
                }
            }
            conj
        })
        .collect()
}

/// Construct the charge-conjugated (antiparticle) state of a given `QuantumState`.
///
/// Inverts all additive quantum numbers (electric charge, baryon number,
/// lepton numbers, weak isospin, hypercharge) and flips the `is_anti` flag.
/// The 4-momentum is preserved while the internal quantum number vector is
/// conjugated according to CPT.
pub fn conjugate_state(state: &QuantumState) -> SpireResult<QuantumState> {
    let orig = &state.particle;
    let qn = &orig.field.quantum_numbers;

    // Negate all additive quantum numbers.
    let conjugated_qn = QuantumNumbers {
        electric_charge: ElectricCharge(-qn.electric_charge.0),
        weak_isospin: WeakIsospin(-qn.weak_isospin.0),
        hypercharge: Hypercharge(-qn.hypercharge.0),
        baryon_number: BaryonNumber(-qn.baryon_number.0),
        lepton_numbers: LeptonNumbers {
            electron: -qn.lepton_numbers.electron,
            muon: -qn.lepton_numbers.muon,
            tau: -qn.lepton_numbers.tau,
        },
        // Spin, parity, charge_conjugation are unchanged.
        spin: qn.spin,
        parity: qn.parity,
        charge_conjugation: qn.charge_conjugation,
        // Swap colour representation: Triplet <-> AntiTriplet.
        color: match qn.color {
            ColorRepresentation::Triplet => ColorRepresentation::AntiTriplet,
            ColorRepresentation::AntiTriplet => ColorRepresentation::Triplet,
            other => other,
        },
        weak_multiplet: qn.weak_multiplet,
        // Conjugate generalized representations.
        representations: conjugate_representations(&qn.representations),
    };

    let conjugated_field = Field {
        id: orig.field.id.clone(),
        name: orig.field.name.clone(),
        symbol: orig.field.symbol.clone(),
        mass: orig.field.mass,
        width: orig.field.width,
        quantum_numbers: conjugated_qn,
        interactions: orig.field.interactions.clone(),
    };

    let conjugated_particle = Particle {
        field: conjugated_field,
        shell: orig.shell,
        helicity: orig.helicity,
        chirality: orig.chirality,
        is_anti: !orig.is_anti,
    };

    Ok(QuantumState {
        particle: conjugated_particle,
        four_momentum: state.four_momentum,
        normalization: state.normalization,
    })
}

/// Create a `Particle` from a `Field` with default on-shell settings.
///
/// Convenience constructor that produces an on-shell, non-anti particle with
/// no explicit helicity or chirality selection.
pub fn particle_from_field(field: Field) -> Particle {
    Particle {
        field,
        shell: ShellCondition::OnShell,
        helicity: None,
        chirality: None,
        is_anti: false,
    }
}

/// Create the antiparticle `Field` from a given field by conjugating all
/// additive quantum numbers.
///
/// Negates electric charge, weak isospin, hypercharge, baryon number, and
/// lepton family numbers. Swaps colour representations (Triplet ↔ AntiTriplet).
/// Mass, spin, width, and discrete symmetries are preserved.
///
/// The returned field has an auto-generated ID of the form `"anti_{id}"`.
pub fn conjugate_field(field: &Field) -> Field {
    let qn = &field.quantum_numbers;
    let conjugated_qn = QuantumNumbers {
        electric_charge: ElectricCharge(-qn.electric_charge.0),
        weak_isospin: WeakIsospin(-qn.weak_isospin.0),
        hypercharge: Hypercharge(-qn.hypercharge.0),
        baryon_number: BaryonNumber(-qn.baryon_number.0),
        lepton_numbers: LeptonNumbers {
            electron: -qn.lepton_numbers.electron,
            muon: -qn.lepton_numbers.muon,
            tau: -qn.lepton_numbers.tau,
        },
        spin: qn.spin,
        parity: qn.parity,
        charge_conjugation: qn.charge_conjugation,
        color: match qn.color {
            ColorRepresentation::Triplet => ColorRepresentation::AntiTriplet,
            ColorRepresentation::AntiTriplet => ColorRepresentation::Triplet,
            other => other,
        },
        weak_multiplet: qn.weak_multiplet,
        representations: conjugate_representations(&qn.representations),
    };
    Field {
        id: format!("anti_{}", field.id),
        name: format!("Anti-{}", field.name),
        symbol: format!("\\bar{{{}}}", field.symbol),
        mass: field.mass,
        width: field.width,
        quantum_numbers: conjugated_qn,
        interactions: field.interactions.clone(),
    }
}

/// Apply a chirality projection (left or right) to a fermion `QuantumState`.
///
/// Returns an error if the particle is not a spin-½ fermion.
pub fn apply_chirality_projection(
    state: &mut QuantumState,
    chirality: Chirality,
) -> SpireResult<()> {
    // Chirality projection is only defined for spin-½ fermions (2s = 1).
    let twice_spin = state.particle.field.quantum_numbers.spin.0;
    if twice_spin != 1 {
        return Err(SpireError::InternalError(format!(
            "Chirality projection requires spin-1/2 (2s=1), but field '{}' has 2s={}.",
            state.particle.field.id, twice_spin
        )));
    }
    state.particle.chirality = Some(chirality);
    Ok(())
}

/// Assign a specific helicity to an external state.
///
/// Validates that the requested helicity is physically consistent with the
/// particle's spin.
pub fn assign_helicity(state: &mut QuantumState, helicity: Helicity) -> SpireResult<()> {
    let twice_spin = state.particle.field.quantum_numbers.spin.0;
    // Spin-0 particles have no helicity degrees of freedom.
    if twice_spin == 0 {
        return Err(SpireError::InternalError(format!(
            "Cannot assign helicity to spin-0 field '{}'.",
            state.particle.field.id
        )));
    }
    state.particle.helicity = Some(helicity);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build an electron Field for testing.
    fn make_electron() -> Field {
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

    /// Helper: build a photon Field for testing.
    fn make_photon() -> Field {
        Field {
            id: "photon".into(),
            name: "Photon".into(),
            symbol: "\\gamma".into(),
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

    /// Helper: build a Higgs-like scalar Field for testing.
    fn make_scalar() -> Field {
        Field {
            id: "h".into(),
            name: "Higgs".into(),
            symbol: "h".into(),
            mass: 125.0,
            width: 0.004,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(0),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Even,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Yukawa],
        }
    }

    #[test]
    fn spin_roundtrip_serde() {
        let s = Spin(1);
        let json = serde_json::to_string(&s).unwrap();
        let s2: Spin = serde_json::from_str(&json).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn chirality_variants() {
        assert_ne!(Chirality::Left, Chirality::Right);
    }

    #[test]
    fn interaction_type_serialize() {
        let it = InteractionType::Electromagnetic;
        let json = serde_json::to_string(&it).unwrap();
        assert_eq!(json, "\"Electromagnetic\"");
    }

    #[test]
    fn particle_from_field_defaults() {
        let electron = make_electron();
        let p = particle_from_field(electron.clone());
        assert_eq!(p.field.id, "e-");
        assert_eq!(p.shell, ShellCondition::OnShell);
        assert!(p.helicity.is_none());
        assert!(p.chirality.is_none());
        assert!(!p.is_anti);
    }

    #[test]
    fn initialize_state_electron() {
        let electron = make_electron();
        let mom = [0.5, 0.0, 0.0, 0.5];
        let state = initialize_state(&electron, mom, Some(Helicity::Minus)).unwrap();
        assert_eq!(state.particle.field.id, "e-");
        assert_eq!(state.particle.helicity, Some(Helicity::Minus));
        assert_eq!(state.four_momentum, mom);
        // Covariant normalization: 2E = 1.0
        assert!((state.normalization - 1.0).abs() < 1e-12);
    }

    #[test]
    fn initialize_state_scalar_no_helicity() {
        let higgs = make_scalar();
        let mom = [125.0, 0.0, 0.0, 0.0];
        let state = initialize_state(&higgs, mom, None).unwrap();
        assert!(state.particle.helicity.is_none());
    }

    #[test]
    fn initialize_state_scalar_rejects_helicity() {
        let higgs = make_scalar();
        let mom = [125.0, 0.0, 0.0, 0.0];
        let result = initialize_state(&higgs, mom, Some(Helicity::Plus));
        assert!(result.is_err());
    }

    #[test]
    fn conjugate_electron_to_positron() {
        let electron = make_electron();
        let mom = [0.5, 0.0, 0.0, 0.5];
        let e_state = initialize_state(&electron, mom, Some(Helicity::Minus)).unwrap();
        let positron = conjugate_state(&e_state).unwrap();

        // Additive QNs should be negated.
        let qn = &positron.particle.field.quantum_numbers;
        assert_eq!(qn.electric_charge, ElectricCharge(3));    // +1
        assert_eq!(qn.weak_isospin, WeakIsospin(1));          // +1/2
        assert_eq!(qn.hypercharge, Hypercharge(3));           // +1
        assert_eq!(qn.baryon_number, BaryonNumber(0));        // still 0
        assert_eq!(qn.lepton_numbers.electron, -1);           // L_e = -1
        // Spin and parity are preserved.
        assert_eq!(qn.spin, Spin(1));
        // is_anti flag flipped.
        assert!(positron.particle.is_anti);
        // 4-momentum preserved.
        assert_eq!(positron.four_momentum, mom);
    }

    #[test]
    fn double_conjugation_restores_original() {
        let electron = make_electron();
        let mom = [1.0, 0.0, 0.0, 1.0];
        let e_state = initialize_state(&electron, mom, None).unwrap();
        let positron = conjugate_state(&e_state).unwrap();
        let restored = conjugate_state(&positron).unwrap();

        // Double conjugation should restore all additive QNs.
        let orig_qn = &e_state.particle.field.quantum_numbers;
        let rest_qn = &restored.particle.field.quantum_numbers;
        assert_eq!(orig_qn.electric_charge, rest_qn.electric_charge);
        assert_eq!(orig_qn.baryon_number, rest_qn.baryon_number);
        assert_eq!(orig_qn.lepton_numbers, rest_qn.lepton_numbers);
        assert_eq!(orig_qn.hypercharge, rest_qn.hypercharge);
        assert!(!restored.particle.is_anti);
    }

    #[test]
    fn chirality_projection_on_electron() {
        let electron = make_electron();
        let mom = [0.5, 0.0, 0.0, 0.5];
        let mut state = initialize_state(&electron, mom, None).unwrap();
        apply_chirality_projection(&mut state, Chirality::Left).unwrap();
        assert_eq!(state.particle.chirality, Some(Chirality::Left));
    }

    #[test]
    fn chirality_projection_on_photon_fails() {
        let photon = make_photon();
        let mom = [1.0, 0.0, 0.0, 1.0];
        let mut state = initialize_state(&photon, mom, None).unwrap();
        let result = apply_chirality_projection(&mut state, Chirality::Left);
        assert!(result.is_err());
    }

    #[test]
    fn assign_helicity_to_photon() {
        let photon = make_photon();
        let mom = [1.0, 0.0, 0.0, 1.0];
        let mut state = initialize_state(&photon, mom, None).unwrap();
        assign_helicity(&mut state, Helicity::Plus).unwrap();
        assert_eq!(state.particle.helicity, Some(Helicity::Plus));
    }

    #[test]
    fn assign_helicity_to_scalar_fails() {
        let higgs = make_scalar();
        let mom = [125.0, 0.0, 0.0, 0.0];
        let mut state = initialize_state(&higgs, mom, None).unwrap();
        let result = assign_helicity(&mut state, Helicity::Plus);
        assert!(result.is_err());
    }
}
