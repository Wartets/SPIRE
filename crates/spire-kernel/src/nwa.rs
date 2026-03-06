//! # Narrow Width Approximation (NWA) - Cascade Decay Simulation
//!
//! This module implements the **Narrow Width Approximation** for simulating
//! decay cascades in Monte Carlo event generation. When a final-state
//! particle is unstable ($\Gamma / m \ll 1$), it can be decayed in its
//! rest frame, with the daughter momenta boosted back to the lab frame.
//!
//! ## Algorithm
//!
//! 1. For each final-state particle, check if it is unstable
//!    ($\Gamma_{\text{tot}} > 0$ and mass $> 0$).
//! 2. Sample a decay channel from the branching ratio distribution.
//! 3. Generate isotropic 2-body decay momenta in the parent rest frame.
//! 4. Lorentz-boost the daughter momenta from the parent rest frame
//!    to the lab frame using $\vec{\beta} = \vec{p}_{\text{parent}} / E_{\text{parent}}$.
//! 5. Recursively apply to any unstable daughters.
//!
//! ## Lorentz Boost Convention
//!
//! The boost from the parent rest frame to the lab frame uses:
//! $$\vec{\beta} = \frac{\vec{p}_{\text{parent}}}{E_{\text{parent}}}$$
//! $$\gamma = \frac{1}{\sqrt{1 - \beta^2}}$$
//!
//! ## Invariant Mass Preservation
//!
//! After the boost, the invariant mass of the daughter system equals the
//! parent mass: $(p_{d_1} + p_{d_2})^2 = m_{\text{parent}}^2$.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::algebra::SpacetimeVector;
use crate::decay::{calculate_decay_table, DecayTable};
use crate::kinematics::{apply_lorentz_boost, cm_momentum, LorentzBoost, PhaseSpacePoint};
use crate::lagrangian::TheoreticalModel;
use crate::SpireResult;

// ===========================================================================
// Data Structures
// ===========================================================================

/// A single particle in a cascade event, carrying its 4-momentum and identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeParticle {
    /// The field identifier (e.g., "e-", "mu+", "photon").
    pub field_id: String,
    /// The 4-momentum in the lab frame: $(E, p_x, p_y, p_z)$.
    pub momentum: SpacetimeVector,
    /// Whether this particle is stable (true) or was decayed (false).
    pub is_stable: bool,
}

/// The result of applying cascade decays to a phase-space event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeEvent {
    /// All final-state stable particles after all decays are resolved.
    pub particles: Vec<CascadeParticle>,
    /// Number of cascade steps performed.
    pub n_decays: usize,
}

/// Configuration for the NWA cascade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadeConfig {
    /// Maximum recursion depth for cascade decays (0 = no cascading).
    pub max_depth: usize,
    /// Minimum branching ratio to consider a channel (prune rare decays).
    pub min_branching_ratio: f64,
    /// Random seed for reproducibility (None = random).
    pub seed: Option<u64>,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            min_branching_ratio: 1e-6,
            seed: None,
        }
    }
}

// ===========================================================================
// 2-Body Rest-Frame Decay
// ===========================================================================

/// Generate isotropic 2-body decay momenta in the parent rest frame.
///
/// The daughter momenta are generated with uniform angular distribution
/// $(\cos\theta, \phi)$ on the unit sphere.
///
/// # Arguments
/// * `parent_mass` - The invariant mass of the parent particle (GeV).
/// * `m1` - Mass of daughter 1 (GeV).
/// * `m2` - Mass of daughter 2 (GeV).
/// * `rng` - Random number generator.
///
/// # Returns
/// A pair of 4-momenta $(p_1, p_2)$ in the parent rest frame,
/// satisfying $p_1 + p_2 = (m_{\text{parent}}, 0, 0, 0)$.
pub fn generate_two_body_decay(
    parent_mass: f64,
    m1: f64,
    m2: f64,
    rng: &mut StdRng,
) -> (SpacetimeVector, SpacetimeVector) {
    let s = parent_mass * parent_mass;
    let p_star = cm_momentum(s, m1, m2);

    // Isotropic angular distribution.
    let cos_theta: f64 = 2.0 * rng.gen::<f64>() - 1.0;
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
    let phi: f64 = 2.0 * PI * rng.gen::<f64>();

    let px = p_star * sin_theta * phi.cos();
    let py = p_star * sin_theta * phi.sin();
    let pz = p_star * cos_theta;

    let e1 = (m1 * m1 + p_star * p_star).sqrt();
    let e2 = (m2 * m2 + p_star * p_star).sqrt();

    let p1 = SpacetimeVector::new_4d(e1, px, py, pz);
    let p2 = SpacetimeVector::new_4d(e2, -px, -py, -pz);

    (p1, p2)
}

// ===========================================================================
// Lorentz Boost Utility
// ===========================================================================

/// Compute the boost parameters from a parent 4-momentum.
///
/// Returns $\vec{\beta} = \vec{p} / E$ and $\gamma = 1/\sqrt{1-\beta^2}$.
fn boost_from_momentum(parent: &SpacetimeVector) -> Option<LorentzBoost> {
    let e = parent[0];
    if e.abs() < 1e-15 {
        return None;
    }

    let bx = parent[1] / e;
    let by = parent[2] / e;
    let bz = parent[3] / e;

    let beta_sq = bx * bx + by * by + bz * bz;
    if beta_sq >= 1.0 {
        return None;
    }

    let gamma = 1.0 / (1.0 - beta_sq).sqrt();

    Some(LorentzBoost {
        beta: [bx, by, bz],
        gamma,
    })
}

/// Boost a SpacetimeVector from the parent rest frame to the lab frame.
///
/// Since `apply_lorentz_boost` transforms *to* the frame with velocity β,
/// we must negate β to perform the inverse transformation (rest→lab).
fn boost_to_lab(
    momentum_rest: &SpacetimeVector,
    parent_lab: &SpacetimeVector,
) -> SpireResult<SpacetimeVector> {
    let boost_fwd = match boost_from_momentum(parent_lab) {
        Some(b) => b,
        None => return Ok(momentum_rest.clone()),
    };

    // Inverse boost: negate β to go from rest frame → lab frame.
    let inverse_boost = LorentzBoost {
        beta: [-boost_fwd.beta[0], -boost_fwd.beta[1], -boost_fwd.beta[2]],
        gamma: boost_fwd.gamma,
    };

    use crate::algebra::FourMomentum;
    let p = FourMomentum::new(
        momentum_rest[0],
        momentum_rest[1],
        momentum_rest[2],
        momentum_rest[3],
    );

    let boosted = apply_lorentz_boost(&p, &inverse_boost)?;

    Ok(SpacetimeVector::new_4d(
        boosted.e, boosted.px, boosted.py, boosted.pz,
    ))
}

// ===========================================================================
// Recursive Cascade Decay
// ===========================================================================

/// Check whether a field is unstable (has positive width and nonzero mass).
fn is_unstable(model: &TheoreticalModel, field_id: &str) -> bool {
    model
        .fields
        .iter()
        .find(|f| f.id == field_id)
        .map(|f| f.width > 0.0 && f.mass > 0.0)
        .unwrap_or(false)
}

/// Recursively decay a single particle.
///
/// If the particle is stable, it is returned as-is. If unstable, a decay
/// channel is sampled from the branching ratios, 2-body daughter momenta
/// are generated in the rest frame, boosted to the lab frame, and each
/// daughter is recursively decayed.
fn decay_particle_recursive(
    model: &TheoreticalModel,
    field_id: &str,
    lab_momentum: &SpacetimeVector,
    config: &CascadeConfig,
    rng: &mut StdRng,
    depth: usize,
    n_decays: &mut usize,
) -> SpireResult<Vec<CascadeParticle>> {
    // Base cases: stable particle or max depth reached.
    if !is_unstable(model, field_id) || depth >= config.max_depth {
        return Ok(vec![CascadeParticle {
            field_id: field_id.to_string(),
            momentum: lab_momentum.clone(),
            is_stable: !is_unstable(model, field_id),
        }]);
    }

    // Compute decay table.
    let table = calculate_decay_table(model, field_id)?;

    if table.channels.is_empty() {
        return Ok(vec![CascadeParticle {
            field_id: field_id.to_string(),
            momentum: lab_momentum.clone(),
            is_stable: true,
        }]);
    }

    // Sample a decay channel from the BR distribution.
    let channel = sample_channel(&table, config.min_branching_ratio, rng);
    let channel = match channel {
        Some(ch) => ch,
        None => {
            return Ok(vec![CascadeParticle {
                field_id: field_id.to_string(),
                momentum: lab_momentum.clone(),
                is_stable: true,
            }]);
        }
    };

    // Only handle 2-body decays.
    if channel.final_state.len() != 2 {
        return Ok(vec![CascadeParticle {
            field_id: field_id.to_string(),
            momentum: lab_momentum.clone(),
            is_stable: true,
        }]);
    }

    // Resolve daughter masses.
    let m1 = model
        .fields
        .iter()
        .find(|f| f.id == channel.final_state[0])
        .map(|f| f.mass)
        .unwrap_or(0.0);
    let m2 = model
        .fields
        .iter()
        .find(|f| f.id == channel.final_state[1])
        .map(|f| f.mass)
        .unwrap_or(0.0);

    // Generate rest-frame momenta.
    let (p1_rest, p2_rest) = generate_two_body_decay(table.parent_mass, m1, m2, rng);

    // Boost to lab frame.
    let p1_lab = boost_to_lab(&p1_rest, lab_momentum)?;
    let p2_lab = boost_to_lab(&p2_rest, lab_momentum)?;

    *n_decays += 1;

    // Recursively decay the daughters.
    let mut result = Vec::new();
    result.extend(decay_particle_recursive(
        model,
        &channel.final_state[0],
        &p1_lab,
        config,
        rng,
        depth + 1,
        n_decays,
    )?);
    result.extend(decay_particle_recursive(
        model,
        &channel.final_state[1],
        &p2_lab,
        config,
        rng,
        depth + 1,
        n_decays,
    )?);

    Ok(result)
}

/// Sample a decay channel from the branching ratio distribution.
fn sample_channel<'a>(
    table: &'a DecayTable,
    min_br: f64,
    rng: &mut StdRng,
) -> Option<&'a crate::decay::DecayChannel> {
    // Filter channels above minimum BR.
    let eligible: Vec<&crate::decay::DecayChannel> = table
        .channels
        .iter()
        .filter(|c| c.branching_ratio >= min_br)
        .collect();

    if eligible.is_empty() {
        return None;
    }

    // Normalise BRs over eligible channels.
    let br_sum: f64 = eligible.iter().map(|c| c.branching_ratio).sum();
    if br_sum <= 0.0 {
        return None;
    }

    let r: f64 = rng.gen::<f64>() * br_sum;
    let mut cumulative = 0.0;
    for ch in &eligible {
        cumulative += ch.branching_ratio;
        if r <= cumulative {
            return Some(ch);
        }
    }

    eligible.last().copied()
}

// ===========================================================================
// Public API
// ===========================================================================

/// Apply cascade decays to a phase-space event.
///
/// For each final-state particle in the event, if it is unstable in the
/// given model, its 4-momentum is decayed via the NWA: daughter momenta
/// are generated in the parent rest frame and boosted to the lab frame.
/// This is applied recursively until all particles are stable.
///
/// # Arguments
/// * `event` - The original phase-space event (e.g., from RAMBO).
/// * `particle_ids` - The field IDs of the final-state particles (one per momentum).
/// * `model` - The theoretical model containing field definitions and vertices.
/// * `config` - Cascade configuration (max depth, min BR, seed).
///
/// # Returns
/// A [`CascadeEvent`] containing all stable final-state particles.
pub fn cascade_decay(
    event: &PhaseSpacePoint,
    particle_ids: &[String],
    model: &TheoreticalModel,
    config: &CascadeConfig,
) -> SpireResult<CascadeEvent> {
    if event.momenta.len() != particle_ids.len() {
        return Err(crate::SpireError::InternalError(format!(
            "Mismatch: {} momenta vs {} particle IDs",
            event.momenta.len(),
            particle_ids.len()
        )));
    }

    let mut rng = match config.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    };

    let mut all_particles = Vec::new();
    let mut total_decays = 0usize;

    for (momentum, field_id) in event.momenta.iter().zip(particle_ids.iter()) {
        let particles = decay_particle_recursive(
            model,
            field_id,
            momentum,
            config,
            &mut rng,
            0,
            &mut total_decays,
        )?;
        all_particles.extend(particles);
    }

    Ok(CascadeEvent {
        particles: all_particles,
        n_decays: total_decays,
    })
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{MetricSignature, SpacetimeVector};
    use crate::data_loader;

    fn load_sm_model() -> TheoreticalModel {
        let particles_toml = include_str!("../../../data/particles.toml");
        let vertices_toml = include_str!("../../../data/sm_vertices.toml");
        data_loader::build_model(particles_toml, vertices_toml, "Standard Model").unwrap()
    }

    #[test]
    fn test_two_body_decay_momentum_conservation() {
        let mut rng = StdRng::seed_from_u64(42);
        let parent_mass = 91.1876; // Z boson
        let m1 = 0.000511; // electron
        let m2 = 0.000511; // positron

        let (p1, p2) = generate_two_body_decay(parent_mass, m1, m2, &mut rng);

        // Sum should equal (M, 0, 0, 0) in the rest frame.
        let e_total = p1[0] + p2[0];
        let px_total = p1[1] + p2[1];
        let py_total = p1[2] + p2[2];
        let pz_total = p1[3] + p2[3];

        assert!(
            (e_total - parent_mass).abs() < 1e-6,
            "Energy conservation: {} vs {}",
            e_total,
            parent_mass
        );
        assert!(px_total.abs() < 1e-10, "px conservation: {}", px_total);
        assert!(py_total.abs() < 1e-10, "py conservation: {}", py_total);
        assert!(pz_total.abs() < 1e-10, "pz conservation: {}", pz_total);
    }

    #[test]
    fn test_two_body_decay_invariant_mass() {
        let mut rng = StdRng::seed_from_u64(123);
        let parent_mass = 125.1; // Higgs
        let m1 = 0.000511; // electron
        let m2 = 0.000511; // positron

        let (p1, p2) = generate_two_body_decay(parent_mass, m1, m2, &mut rng);

        // Each daughter should be on-shell.
        let metric = MetricSignature::minkowski_4d();
        let m1_sq = p1.invariant_mass_sq(&metric).unwrap();
        let m2_sq = p2.invariant_mass_sq(&metric).unwrap();

        assert!(
            (m1_sq - m1 * m1).abs() < 1e-6,
            "Daughter 1 on-shell: {} vs {}",
            m1_sq,
            m1 * m1
        );
        assert!(
            (m2_sq - m2 * m2).abs() < 1e-6,
            "Daughter 2 on-shell: {} vs {}",
            m2_sq,
            m2 * m2
        );
    }

    #[test]
    fn test_boost_from_momentum_at_rest() {
        let p = SpacetimeVector::new_4d(91.1876, 0.0, 0.0, 0.0);
        let boost = boost_from_momentum(&p).unwrap();
        assert!(boost.beta[0].abs() < 1e-15);
        assert!(boost.beta[1].abs() < 1e-15);
        assert!(boost.beta[2].abs() < 1e-15);
        assert!((boost.gamma - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_boost_from_momentum_moving() {
        // Z boson with pz = 100 GeV.
        let m_z: f64 = 91.1876;
        let pz: f64 = 100.0;
        let e = (m_z * m_z + pz * pz).sqrt();
        let p = SpacetimeVector::new_4d(e, 0.0, 0.0, pz);
        let boost = boost_from_momentum(&p).unwrap();

        assert!((boost.beta[2] - pz / e).abs() < 1e-12);
        assert!(boost.gamma > 1.0);
    }

    #[test]
    fn test_boost_preserves_invariant_mass() {
        let mut rng = StdRng::seed_from_u64(7);
        let m_z: f64 = 91.1876;
        let m_e: f64 = 0.000511;

        // Generate decay in rest frame.
        let (p1_rest, p2_rest) = generate_two_body_decay(m_z, m_e, m_e, &mut rng);

        // Parent is moving with momentum (E, 0, 0, 200).
        let pz_lab: f64 = 200.0;
        let e_lab = (m_z * m_z + pz_lab * pz_lab).sqrt();
        let parent_lab = SpacetimeVector::new_4d(e_lab, 0.0, 0.0, pz_lab);

        // Boost daughters.
        let p1_lab = boost_to_lab(&p1_rest, &parent_lab).unwrap();
        let p2_lab = boost_to_lab(&p2_rest, &parent_lab).unwrap();

        // Sum of boosted daughters should have invariant mass ≈ m_Z.
        let metric = MetricSignature::minkowski_4d();
        let sum_e = p1_lab[0] + p2_lab[0];
        let sum_px = p1_lab[1] + p2_lab[1];
        let sum_py = p1_lab[2] + p2_lab[2];
        let sum_pz = p1_lab[3] + p2_lab[3];
        let sum_vec = SpacetimeVector::new_4d(sum_e, sum_px, sum_py, sum_pz);
        let inv_mass_sq: f64 = sum_vec.invariant_mass_sq(&metric).unwrap();

        let rel_error = (inv_mass_sq - m_z * m_z).abs() / (m_z * m_z);
        assert!(
            rel_error < 1e-8,
            "Invariant mass preservation: √(s) = {:.6} vs m_Z = {:.6} (rel err = {:.2e})",
            inv_mass_sq.sqrt(),
            m_z,
            rel_error
        );
    }

    #[test]
    fn test_cascade_stable_particles_unchanged() {
        let model = load_sm_model();
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 0.0, 0.0, 50.0),
                SpacetimeVector::new_4d(50.0, 0.0, 0.0, -50.0),
            ],
            weight: 1.0,
        };
        let particle_ids = vec!["e-".to_string(), "e+".to_string()];
        let config = CascadeConfig {
            max_depth: 5,
            min_branching_ratio: 1e-6,
            seed: Some(42),
        };

        let result = cascade_decay(&event, &particle_ids, &model, &config).unwrap();

        // Electrons are stable - should pass through unchanged.
        assert_eq!(result.particles.len(), 2);
        assert_eq!(result.n_decays, 0);
        assert_eq!(result.particles[0].field_id, "e-");
        assert_eq!(result.particles[1].field_id, "e+");
    }

    #[test]
    fn test_cascade_z_decays() {
        let model = load_sm_model();

        // Z boson at rest.
        let m_z = 91.1876;
        let event = PhaseSpacePoint {
            momenta: vec![SpacetimeVector::new_4d(m_z, 0.0, 0.0, 0.0)],
            weight: 1.0,
        };
        let particle_ids = vec!["Z0".to_string()];
        let config = CascadeConfig {
            max_depth: 5,
            min_branching_ratio: 1e-6,
            seed: Some(42),
        };

        let result = cascade_decay(&event, &particle_ids, &model, &config).unwrap();

        // Z should have decayed into 2 stable particles.
        assert!(result.n_decays >= 1, "Z should have decayed");
        assert!(
            result.particles.len() >= 2,
            "Should have at least 2 daughters, got {}",
            result.particles.len()
        );
    }

    #[test]
    fn test_cascade_energy_conservation() {
        let model = load_sm_model();

        let m_z: f64 = 91.1876;
        let pz: f64 = 150.0;
        let e_z = (m_z * m_z + pz * pz).sqrt();
        let event = PhaseSpacePoint {
            momenta: vec![SpacetimeVector::new_4d(e_z, 0.0, 0.0, pz)],
            weight: 1.0,
        };
        let particle_ids = vec!["Z0".to_string()];
        let config = CascadeConfig {
            max_depth: 1,
            min_branching_ratio: 1e-6,
            seed: Some(99),
        };

        let result = cascade_decay(&event, &particle_ids, &model, &config).unwrap();

        // Total energy and momentum should be conserved.
        let total_e: f64 = result.particles.iter().map(|p| p.momentum[0]).sum();
        let total_px: f64 = result.particles.iter().map(|p| p.momentum[1]).sum();
        let total_py: f64 = result.particles.iter().map(|p| p.momentum[2]).sum();
        let total_pz: f64 = result.particles.iter().map(|p| p.momentum[3]).sum();

        assert!(
            (total_e - e_z).abs() / e_z < 1e-8,
            "Energy conservation: {} vs {}",
            total_e,
            e_z
        );
        assert!(total_px.abs() < 1e-6, "px conservation: {}", total_px);
        assert!(total_py.abs() < 1e-6, "py conservation: {}", total_py);
        assert!(
            (total_pz - pz).abs() / pz < 1e-8,
            "pz conservation: {} vs {}",
            total_pz,
            pz
        );
    }

    #[test]
    fn test_cascade_max_depth_zero() {
        let model = load_sm_model();

        let m_z = 91.1876;
        let event = PhaseSpacePoint {
            momenta: vec![SpacetimeVector::new_4d(m_z, 0.0, 0.0, 0.0)],
            weight: 1.0,
        };
        let particle_ids = vec!["Z0".to_string()];
        let config = CascadeConfig {
            max_depth: 0, // No cascading.
            min_branching_ratio: 1e-6,
            seed: Some(42),
        };

        let result = cascade_decay(&event, &particle_ids, &model, &config).unwrap();

        // With max_depth=0, the Z should not be decayed.
        assert_eq!(result.n_decays, 0);
        assert_eq!(result.particles.len(), 1);
        assert_eq!(result.particles[0].field_id, "Z0");
    }

    #[test]
    fn test_cascade_mismatched_lengths_error() {
        let model = load_sm_model();
        let event = PhaseSpacePoint {
            momenta: vec![SpacetimeVector::new_4d(100.0, 0.0, 0.0, 0.0)],
            weight: 1.0,
        };
        let particle_ids = vec!["e-".to_string(), "e+".to_string()]; // 2 IDs but 1 momentum.
        let config = CascadeConfig::default();

        let result = cascade_decay(&event, &particle_ids, &model, &config);
        assert!(result.is_err(), "Should error on mismatched lengths");
    }

    #[test]
    fn test_sample_channel_deterministic() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();

        let mut rng = StdRng::seed_from_u64(42);
        let ch = sample_channel(&table, 1e-6, &mut rng);
        assert!(ch.is_some(), "Should find a channel for Z0");
    }
}
