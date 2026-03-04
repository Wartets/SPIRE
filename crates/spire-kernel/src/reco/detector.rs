//! # Detector Response — Phenomenological Simulation
//!
//! This module provides a modular, configurable detector simulation that
//! bridges parton-level Monte Carlo truth and reconstructed experimental
//! observables. It models the key effects of a general-purpose collider
//! detector:
//!
//! 1. **Particle Identification**: Classifies final-state particles into
//!    leptons, photons, hadronic deposits, and invisible (neutrinos, BSM).
//! 2. **Efficiency Cuts**: Applies acceptance × efficiency maps
//!    $\epsilon(p_T, \eta)$ to each particle, stochastically dropping
//!    those that fail.
//! 3. **Energy Smearing**: Applies Gaussian energy resolution of the form
//!    $$\frac{\sigma_E}{E} = \frac{a}{\sqrt{E}} \oplus b$$
//!    where $a$ is the stochastic term and $b$ is the constant term,
//!    combined in quadrature.
//! 4. **Missing Transverse Energy**: Computes $\vec{E}_T^{\,\text{miss}}$
//!    from invisible particles and detector inefficiencies.
//! 5. **Jet Clustering**: Runs the Anti-$k_t$ algorithm on hadronic
//!    energy deposits to produce reconstructed jets.
//!
//! ## Detector Presets
//!
//! Three built-in detector configurations are provided:
//! - **Perfect**: No smearing, 100% efficiency (truth-level).
//! - **LHC-like**: Typical ATLAS/CMS resolution and acceptance.
//! - **ILC-like**: International Linear Collider-style detector.

use rand::Rng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};

use crate::algebra::SpacetimeVector;
use crate::kinematics::PhaseSpacePoint;
use crate::reco::clustering::{self, Jet, JetAlgorithm};

// ===========================================================================
// Particle Classification
// ===========================================================================

/// Classification of a final-state particle for detector simulation.
///
/// In a full simulation, this would be determined by PDG ID. At parton
/// level, we use mass-based heuristics or explicit user tagging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticleKind {
    /// Electron or positron (electromagnetically interacting lepton).
    Electron,
    /// Muon or antimuon (minimum-ionising lepton).
    Muon,
    /// Photon (electromagnetic shower).
    Photon,
    /// Hadronic deposit (quarks, gluons → jets).
    Hadron,
    /// Invisible particle (neutrino, dark matter candidate).
    /// Does not interact with the detector; contributes to MET.
    Invisible,
}

// ===========================================================================
// Resolution & Efficiency Profiles
// ===========================================================================

/// Energy resolution profile for a calorimeter sub-system.
///
/// The fractional energy resolution is parametrised as:
/// $$\frac{\sigma_E}{E} = \sqrt{\frac{a^2}{E} + b^2}$$
///
/// where $a$ is the stochastic (sampling) term and $b$ is the constant
/// (calibration) term.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResolutionProfile {
    /// Stochastic term $a$ in the resolution formula.
    /// Typical values: 0.5 (hadronic), 0.1 (EM), 0.01 (muon tracker).
    pub stochastic: f64,
    /// Constant term $b$ in the resolution formula.
    /// Typical values: 0.03 (hadronic), 0.007 (EM), 0.001 (muon).
    pub constant: f64,
}

impl ResolutionProfile {
    /// Compute the fractional energy resolution $\sigma_E / E$ at energy $E$.
    #[inline]
    pub fn sigma_over_e(&self, energy: f64) -> f64 {
        if energy <= 0.0 {
            return 0.0;
        }
        let a2_over_e = self.stochastic * self.stochastic / energy;
        let b2 = self.constant * self.constant;
        (a2_over_e + b2).sqrt()
    }

    /// A "perfect" resolution (no smearing).
    pub fn perfect() -> Self {
        Self {
            stochastic: 0.0,
            constant: 0.0,
        }
    }
}

/// Efficiency map defining the geometric and reconstruction acceptance
/// for a given particle type.
///
/// The acceptance is defined as a flat efficiency within a fiducial region
/// ($p_T > p_T^{\min}$, $|\eta| < \eta^{\max}$). Particles outside the
/// fiducial region are lost (efficiency = 0).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EfficiencyMap {
    /// Minimum transverse momentum for detection (GeV).
    pub pt_min: f64,
    /// Maximum pseudorapidity for detector acceptance.
    pub eta_max: f64,
    /// Flat efficiency within the fiducial region (0.0 to 1.0).
    pub flat_efficiency: f64,
}

impl EfficiencyMap {
    /// Evaluate whether a particle passes the efficiency filter.
    ///
    /// Returns `true` if the particle survives (is reconstructed).
    #[inline]
    pub fn passes(&self, pt: f64, eta: f64, rng: &mut impl Rng) -> bool {
        if pt < self.pt_min || eta.abs() > self.eta_max {
            return false;
        }
        if self.flat_efficiency >= 1.0 {
            return true;
        }
        rng.gen::<f64>() < self.flat_efficiency
    }

    /// A "perfect" efficiency (100% acceptance everywhere).
    pub fn perfect() -> Self {
        Self {
            pt_min: 0.0,
            eta_max: f64::INFINITY,
            flat_efficiency: 1.0,
        }
    }
}

// ===========================================================================
// Detector Model
// ===========================================================================

/// A configurable phenomenological detector model.
///
/// Bundles resolution profiles and efficiency maps for each particle
/// sub-system (electrons, muons, photons, jets) along with the jet
/// clustering algorithm configuration.
///
/// # Presets
///
/// Use [`DetectorModel::perfect`], [`DetectorModel::lhc_like`], or
/// [`DetectorModel::ilc_like`] for standard configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorModel {
    /// Human-readable name for this detector configuration.
    pub name: String,
    /// Jet radius parameter $R$ for the Anti-$k_t$ algorithm.
    pub jet_radius: f64,
    /// Jet algorithm selection.
    pub jet_algorithm: JetAlgorithm,
    /// Hadronic calorimeter energy resolution.
    pub hadronic_resolution: ResolutionProfile,
    /// Electromagnetic calorimeter energy resolution.
    pub em_resolution: ResolutionProfile,
    /// Muon spectrometer momentum resolution.
    pub muon_resolution: ResolutionProfile,
    /// Electron reconstruction efficiency.
    pub electron_efficiency: EfficiencyMap,
    /// Muon reconstruction efficiency.
    pub muon_efficiency: EfficiencyMap,
    /// Photon reconstruction efficiency.
    pub photon_efficiency: EfficiencyMap,
    /// Jet reconstruction efficiency.
    pub jet_efficiency: EfficiencyMap,
}

impl DetectorModel {
    /// A "perfect" detector: no smearing, 100% efficiency.
    ///
    /// Useful for truth-level comparisons and debugging.
    pub fn perfect() -> Self {
        Self {
            name: "Perfect Detector".into(),
            jet_radius: 0.4,
            jet_algorithm: JetAlgorithm::AntiKt,
            hadronic_resolution: ResolutionProfile::perfect(),
            em_resolution: ResolutionProfile::perfect(),
            muon_resolution: ResolutionProfile::perfect(),
            electron_efficiency: EfficiencyMap::perfect(),
            muon_efficiency: EfficiencyMap::perfect(),
            photon_efficiency: EfficiencyMap::perfect(),
            jet_efficiency: EfficiencyMap::perfect(),
        }
    }

    /// An LHC-like detector (typical ATLAS/CMS parameters).
    ///
    /// - Hadronic calorimeter: $a = 0.50$, $b = 0.03$.
    /// - EM calorimeter: $a = 0.10$, $b = 0.007$.
    /// - Muon spectrometer: $a = 0.01$, $b = 0.001$.
    /// - Lepton/photon acceptance: $p_T > 25\,\text{GeV}$, $|\eta| < 2.5$.
    /// - Jet acceptance: $p_T > 25\,\text{GeV}$, $|\eta| < 4.5$.
    /// - Jet radius: $R = 0.4$.
    pub fn lhc_like() -> Self {
        Self {
            name: "LHC-like Detector".into(),
            jet_radius: 0.4,
            jet_algorithm: JetAlgorithm::AntiKt,
            hadronic_resolution: ResolutionProfile {
                stochastic: 0.50,
                constant: 0.03,
            },
            em_resolution: ResolutionProfile {
                stochastic: 0.10,
                constant: 0.007,
            },
            muon_resolution: ResolutionProfile {
                stochastic: 0.01,
                constant: 0.001,
            },
            electron_efficiency: EfficiencyMap {
                pt_min: 25.0,
                eta_max: 2.5,
                flat_efficiency: 0.95,
            },
            muon_efficiency: EfficiencyMap {
                pt_min: 25.0,
                eta_max: 2.5,
                flat_efficiency: 0.98,
            },
            photon_efficiency: EfficiencyMap {
                pt_min: 25.0,
                eta_max: 2.5,
                flat_efficiency: 0.90,
            },
            jet_efficiency: EfficiencyMap {
                pt_min: 25.0,
                eta_max: 4.5,
                flat_efficiency: 0.99,
            },
        }
    }

    /// An ILC-like detector (International Linear Collider).
    ///
    /// Typically has better resolution and larger acceptance than hadron
    /// colliders due to the cleaner $e^+e^-$ environment.
    pub fn ilc_like() -> Self {
        Self {
            name: "ILC-like Detector".into(),
            jet_radius: 0.7,
            jet_algorithm: JetAlgorithm::AntiKt,
            hadronic_resolution: ResolutionProfile {
                stochastic: 0.30,
                constant: 0.01,
            },
            em_resolution: ResolutionProfile {
                stochastic: 0.05,
                constant: 0.005,
            },
            muon_resolution: ResolutionProfile {
                stochastic: 0.005,
                constant: 0.0005,
            },
            electron_efficiency: EfficiencyMap {
                pt_min: 10.0,
                eta_max: 3.0,
                flat_efficiency: 0.99,
            },
            muon_efficiency: EfficiencyMap {
                pt_min: 10.0,
                eta_max: 3.0,
                flat_efficiency: 0.99,
            },
            photon_efficiency: EfficiencyMap {
                pt_min: 10.0,
                eta_max: 3.0,
                flat_efficiency: 0.98,
            },
            jet_efficiency: EfficiencyMap {
                pt_min: 10.0,
                eta_max: 4.0,
                flat_efficiency: 0.99,
            },
        }
    }

    /// Resolve a detector preset by name.
    ///
    /// Recognised names: `"perfect"`, `"lhc"`, `"ilc"`.
    pub fn from_preset(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "perfect" | "none" => Some(Self::perfect()),
            "lhc" | "lhc-like" | "lhc_like" | "atlas" | "cms" => Some(Self::lhc_like()),
            "ilc" | "ilc-like" | "ilc_like" | "lepton-collider" => Some(Self::ilc_like()),
            _ => None,
        }
    }
}

// ===========================================================================
// Reconstructed Event
// ===========================================================================

/// A fully reconstructed event after detector simulation.
///
/// Contains all physics objects visible to the experiment:
/// jets, isolated leptons, photons, and missing transverse energy.
/// All objects have been through efficiency cuts and energy smearing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedEvent {
    /// Reconstructed jets (Anti-$k_t$, $p_T$-ordered).
    pub jets: Vec<Jet>,
    /// Reconstructed electrons/positrons ($p_T$-ordered).
    pub electrons: Vec<SpacetimeVector>,
    /// Reconstructed muons/antimuons ($p_T$-ordered).
    pub muons: Vec<SpacetimeVector>,
    /// Reconstructed photons ($p_T$-ordered).
    pub photons: Vec<SpacetimeVector>,
    /// Missing transverse energy 4-vector.
    ///
    /// The spatial components $(p_x, p_y)$ are the negative vector sum
    /// of all visible transverse momenta. The energy component is
    /// $|\vec{p}_T^{\,\mathrm{miss}}|$ and $p_z = 0$.
    pub met: SpacetimeVector,
}

impl ReconstructedEvent {
    /// The missing transverse energy magnitude $E_T^{\text{miss}}$ (GeV).
    #[inline]
    pub fn met_pt(&self) -> f64 {
        let px = self.met[1];
        let py = self.met[2];
        (px * px + py * py).sqrt()
    }

    /// Total number of reconstructed jets.
    #[inline]
    pub fn n_jets(&self) -> usize {
        self.jets.len()
    }

    /// Total number of reconstructed leptons (electrons + muons).
    #[inline]
    pub fn n_leptons(&self) -> usize {
        self.electrons.len() + self.muons.len()
    }
}

// ===========================================================================
// Smearing & Reconstruction
// ===========================================================================

/// Compute transverse momentum from a 4-vector.
#[inline]
fn compute_pt(v: &SpacetimeVector) -> f64 {
    let px = v[1];
    let py = v[2];
    (px * px + py * py).sqrt()
}

/// Compute pseudorapidity from a 4-vector.
#[inline]
fn compute_eta(v: &SpacetimeVector) -> f64 {
    let px = v[1];
    let py = v[2];
    let pz = v[3];
    let p_mag = (px * px + py * py + pz * pz).sqrt();
    if p_mag < 1e-300 {
        return 0.0;
    }
    (pz / p_mag).atanh()
}

/// Apply Gaussian energy smearing to a 4-momentum.
///
/// Scales all components by the factor $(1 + z)$ where
/// $z \sim \mathcal{N}(0, \sigma)$ and $\sigma = \sigma_E / E$.
///
/// The direction is preserved (all components scaled equally).
fn smear_momentum(
    momentum: &SpacetimeVector,
    resolution: &ResolutionProfile,
    rng: &mut impl Rng,
) -> SpacetimeVector {
    let energy = momentum[0];
    if energy <= 0.0 || (resolution.stochastic == 0.0 && resolution.constant == 0.0) {
        return momentum.clone();
    }

    let sigma = resolution.sigma_over_e(energy);
    if sigma <= 0.0 {
        return momentum.clone();
    }

    let normal = Normal::new(0.0, sigma).unwrap_or_else(|_| Normal::new(0.0, 1e-10).unwrap());
    let z: f64 = normal.sample(rng);
    let scale = (1.0 + z).max(0.01); // Prevent negative energy.

    momentum.scale(scale)
}

/// Reconstruct a Monte Carlo truth event through a detector model.
///
/// This is the main detector simulation function. It processes each
/// final-state particle through the following pipeline:
///
/// 1. **Classification**: Determine particle kind from `particle_kinds`.
/// 2. **Invisible Accumulation**: Neutrinos and other invisible particles
///    contribute directly to the MET calculation.
/// 3. **Efficiency Filter**: Apply $\epsilon(p_T, \eta)$ acceptance cuts.
/// 4. **Energy Smearing**: Apply Gaussian resolution degradation.
/// 5. **Jet Clustering**: Group hadronic deposits into jets.
/// 6. **MET Computation**: Sum visible momenta and negate for MET.
///
/// # Arguments
///
/// * `event` — The truth-level phase-space point.
/// * `particle_kinds` — Classification of each final-state particle.
///   Must have the same length as `event.momenta`.
/// * `detector` — The detector model to apply.
/// * `rng` — Random number generator for efficiency and smearing.
///
/// # Returns
///
/// A [`ReconstructedEvent`] with reconstructed jets, leptons, photons, and MET.
pub fn reconstruct_event(
    event: &PhaseSpacePoint,
    particle_kinds: &[ParticleKind],
    detector: &DetectorModel,
    rng: &mut impl Rng,
) -> ReconstructedEvent {
    assert_eq!(
        event.momenta.len(),
        particle_kinds.len(),
        "Particle kinds must match momenta count"
    );

    let mut electrons: Vec<SpacetimeVector> = Vec::new();
    let mut muons: Vec<SpacetimeVector> = Vec::new();
    let mut photons: Vec<SpacetimeVector> = Vec::new();
    let mut hadronic_deposits: Vec<SpacetimeVector> = Vec::new();

    // Accumulate visible transverse momentum for MET computation.
    let mut sum_px_visible = 0.0_f64;
    let mut sum_py_visible = 0.0_f64;

    for (momentum, &kind) in event.momenta.iter().zip(particle_kinds.iter()) {
        match kind {
            ParticleKind::Invisible => {
                // Invisible particles contribute to MET (not to visible sum).
                // They are *not* added to sum_px_visible / sum_py_visible.
                continue;
            }
            ParticleKind::Electron => {
                let pt = compute_pt(momentum);
                let eta = compute_eta(momentum);

                if !detector.electron_efficiency.passes(pt, eta, rng) {
                    continue; // Lost → contributes to MET deficit.
                }

                let smeared = smear_momentum(momentum, &detector.em_resolution, rng);
                sum_px_visible += smeared[1];
                sum_py_visible += smeared[2];
                electrons.push(smeared);
            }
            ParticleKind::Muon => {
                let pt = compute_pt(momentum);
                let eta = compute_eta(momentum);

                if !detector.muon_efficiency.passes(pt, eta, rng) {
                    continue;
                }

                let smeared = smear_momentum(momentum, &detector.muon_resolution, rng);
                sum_px_visible += smeared[1];
                sum_py_visible += smeared[2];
                muons.push(smeared);
            }
            ParticleKind::Photon => {
                let pt = compute_pt(momentum);
                let eta = compute_eta(momentum);

                if !detector.photon_efficiency.passes(pt, eta, rng) {
                    continue;
                }

                let smeared = smear_momentum(momentum, &detector.em_resolution, rng);
                sum_px_visible += smeared[1];
                sum_py_visible += smeared[2];
                photons.push(smeared);
            }
            ParticleKind::Hadron => {
                let pt = compute_pt(momentum);
                let eta = compute_eta(momentum);

                // Hadronic deposits go to jet clustering.
                // Apply calorimeter smearing first.
                let smeared = smear_momentum(momentum, &detector.hadronic_resolution, rng);

                // Check basic calorimeter acceptance.
                let smeared_pt = compute_pt(&smeared);
                let smeared_eta = compute_eta(&smeared);

                // Soft deposits below a minimal threshold are lost.
                if smeared_pt < 0.5 || smeared_eta.abs() > detector.jet_efficiency.eta_max {
                    let _ = (pt, eta); // Suppress unused warnings.
                    continue;
                }

                sum_px_visible += smeared[1];
                sum_py_visible += smeared[2];
                hadronic_deposits.push(smeared);
            }
        }
    }

    // --- Jet Clustering ---
    let mut jets = clustering::cluster_jets(
        &hadronic_deposits,
        detector.jet_algorithm,
        detector.jet_radius,
    );

    // Apply jet-level efficiency cuts.
    jets.retain(|jet| {
        let pt = jet.pt();
        let eta = jet.eta();
        pt >= detector.jet_efficiency.pt_min && eta.abs() <= detector.jet_efficiency.eta_max
    });

    // --- Sort all collections by descending pT ---
    electrons.sort_by(|a, b| {
        compute_pt(b)
            .partial_cmp(&compute_pt(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    muons.sort_by(|a, b| {
        compute_pt(b)
            .partial_cmp(&compute_pt(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    photons.sort_by(|a, b| {
        compute_pt(b)
            .partial_cmp(&compute_pt(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Jets already sorted by cluster_jets.

    // --- MET Computation ---
    // MET = negative sum of all visible transverse momenta.
    // For a perfect detector with no invisible particles, MET ≈ 0.
    // Truth-level invisible particles (not added to sum_px/py_visible)
    // create genuine MET.
    let met_px = -sum_px_visible;
    let met_py = -sum_py_visible;
    let met_pt = (met_px * met_px + met_py * met_py).sqrt();
    let met = SpacetimeVector::new_4d(met_pt, met_px, met_py, 0.0);

    ReconstructedEvent {
        jets,
        electrons,
        muons,
        photons,
        met,
    }
}

/// Convenience function: reconstruct an event treating all particles as hadrons.
///
/// This is the simplest use case: all final-state partons are clustered
/// into jets with no lepton isolation or MET.
pub fn reconstruct_all_hadronic(
    event: &PhaseSpacePoint,
    detector: &DetectorModel,
    rng: &mut impl Rng,
) -> ReconstructedEvent {
    let kinds = vec![ParticleKind::Hadron; event.momenta.len()];
    reconstruct_event(event, &kinds, detector, rng)
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_test_event() -> PhaseSpacePoint {
        // 2→2 event at √s = 200 GeV.
        PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 30.0, 40.0, 86.6),
                SpacetimeVector::new_4d(100.0, -30.0, -40.0, -86.6),
            ],
            weight: 1.0,
        }
    }

    #[test]
    fn perfect_detector_preserves_momenta() {
        let event = make_test_event();
        let detector = DetectorModel::perfect();
        let kinds = vec![ParticleKind::Electron, ParticleKind::Muon];
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);

        assert_eq!(reco.electrons.len(), 1);
        assert_eq!(reco.muons.len(), 1);
        assert_eq!(reco.jets.len(), 0);
        // Perfect detector: momenta should be unchanged.
        assert!((compute_pt(&reco.electrons[0]) - 50.0).abs() < 1e-10);
        assert!((compute_pt(&reco.muons[0]) - 50.0).abs() < 1e-10);
    }

    #[test]
    fn invisible_particles_create_met() {
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0), // visible
                SpacetimeVector::new_4d(50.0, -30.0, -40.0, 0.0), // invisible (neutrino)
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Electron, ParticleKind::Invisible];
        let detector = DetectorModel::perfect();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);

        assert_eq!(reco.electrons.len(), 1);
        // MET should be non-zero (the invisible neutrino).
        // Visible: px=30, py=40. MET: px=-30, py=-40, pt=50.
        assert!((reco.met_pt() - 50.0).abs() < 1e-8);
    }

    #[test]
    fn efficiency_drops_particles() {
        // Use a detector with 0% electron efficiency → all electrons lost.
        let mut detector = DetectorModel::perfect();
        detector.electron_efficiency.flat_efficiency = 0.0;

        let event = make_test_event();
        let kinds = vec![ParticleKind::Electron, ParticleKind::Electron];
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert_eq!(
            reco.electrons.len(),
            0,
            "Zero efficiency should drop all electrons"
        );
    }

    #[test]
    fn efficiency_50_percent_drops_some() {
        let mut detector = DetectorModel::perfect();
        detector.electron_efficiency.flat_efficiency = 0.5;

        // Generate many events and count surviving electrons.
        let mut rng = StdRng::seed_from_u64(12345);
        let mut total_survived = 0usize;
        let n_trials = 2000;

        for _ in 0..n_trials {
            let event = PhaseSpacePoint {
                momenta: vec![SpacetimeVector::new_4d(100.0, 50.0, 0.0, 0.0)],
                weight: 1.0,
            };
            let kinds = vec![ParticleKind::Electron];
            let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
            total_survived += reco.electrons.len();
        }

        let fraction = total_survived as f64 / n_trials as f64;
        assert!(
            (fraction - 0.5).abs() < 0.05,
            "~50% of electrons should survive, got {:.2}%",
            fraction * 100.0
        );
    }

    #[test]
    fn pt_cut_rejects_soft_particles() {
        let mut detector = DetectorModel::perfect();
        detector.electron_efficiency.pt_min = 25.0;

        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(30.0, 20.0, 15.0, 0.0), // pT = 25 → passes
                SpacetimeVector::new_4d(10.0, 5.0, 3.0, 0.0),   // pT ≈ 5.8 → fails
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Electron, ParticleKind::Electron];
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert_eq!(reco.electrons.len(), 1, "Soft electron should be rejected");
    }

    #[test]
    fn eta_cut_rejects_forward_particles() {
        let mut detector = DetectorModel::perfect();
        detector.muon_efficiency.eta_max = 2.5;

        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 50.0, 0.0, 0.0), // η = 0 → passes
                SpacetimeVector::new_4d(100.0, 1.0, 0.0, 99.99), // η ≈ 5.3 → fails
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Muon, ParticleKind::Muon];
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert_eq!(reco.muons.len(), 1, "Forward muon should be rejected");
    }

    #[test]
    fn hadronic_deposits_form_jets() {
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 100.0, 0.0, 0.0),
                SpacetimeVector::new_4d(100.0, -100.0, 0.0, 0.0),
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Hadron, ParticleKind::Hadron];
        let detector = DetectorModel::perfect();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert_eq!(reco.jets.len(), 2, "Two back-to-back partons → 2 jets");
    }

    #[test]
    fn smearing_changes_energy() {
        let detector = DetectorModel::lhc_like();
        let mut rng = StdRng::seed_from_u64(42);

        // Smear a 50 GeV electron many times — the energy should fluctuate.
        let original = SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0);
        let mut energies: Vec<f64> = Vec::new();
        for _ in 0..100 {
            let smeared = smear_momentum(&original, &detector.em_resolution, &mut rng);
            energies.push(smeared[0]);
        }

        let mean_e: f64 = energies.iter().sum::<f64>() / energies.len() as f64;
        // Mean should be close to 50 GeV.
        assert!(
            (mean_e - 50.0).abs() < 3.0,
            "Mean smeared energy {} should be near 50 GeV",
            mean_e
        );
        // Variance should be non-zero (smearing is happening).
        let variance: f64 =
            energies.iter().map(|e| (e - mean_e).powi(2)).sum::<f64>() / energies.len() as f64;
        assert!(
            variance > 0.01,
            "Smearing should produce energy fluctuations"
        );
    }

    #[test]
    fn resolution_profile_sigma() {
        // LHC hadronic: a=0.5, b=0.03.
        let res = ResolutionProfile {
            stochastic: 0.5,
            constant: 0.03,
        };
        // At 100 GeV: σ/E = sqrt(0.25/100 + 0.0009) = sqrt(0.0025 + 0.0009) = sqrt(0.0034)
        let sigma = res.sigma_over_e(100.0);
        let expected = (0.25 / 100.0 + 0.0009_f64).sqrt();
        assert!((sigma - expected).abs() < 1e-10);
    }

    #[test]
    fn detector_model_presets() {
        let perfect = DetectorModel::from_preset("perfect").unwrap();
        assert_eq!(perfect.name, "Perfect Detector");

        let lhc = DetectorModel::from_preset("lhc").unwrap();
        assert_eq!(lhc.jet_radius, 0.4);
        assert!((lhc.hadronic_resolution.stochastic - 0.5).abs() < 1e-12);

        let ilc = DetectorModel::from_preset("ilc").unwrap();
        assert_eq!(ilc.jet_radius, 0.7);

        assert!(DetectorModel::from_preset("unknown").is_none());
    }

    #[test]
    fn met_conservation_perfect_visible() {
        // All visible particles, perfect detector → MET ≈ 0.
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0),
                SpacetimeVector::new_4d(50.0, -30.0, -40.0, 0.0),
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Electron, ParticleKind::Muon];
        let detector = DetectorModel::perfect();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert!(
            reco.met_pt() < 1e-10,
            "MET should be ~0 for fully visible events with perfect detector"
        );
    }

    #[test]
    fn lhc_detector_full_event() {
        // Simulate a Z → e+ e- event.
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(45.6, 30.0, 25.0, 20.0), // electron
                SpacetimeVector::new_4d(45.6, -30.0, -25.0, -20.0), // positron
            ],
            weight: 1.0,
        };
        let kinds = vec![ParticleKind::Electron, ParticleKind::Electron];
        let detector = DetectorModel::lhc_like();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        // Both electrons should survive (pT ≈ 39 > 25 GeV, |η| < 2.5).
        assert_eq!(reco.electrons.len(), 2);
    }

    #[test]
    fn reconstruct_all_hadronic_convenience() {
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 100.0, 0.0, 0.0),
                SpacetimeVector::new_4d(100.0, -100.0, 0.0, 0.0),
            ],
            weight: 1.0,
        };
        let detector = DetectorModel::perfect();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_all_hadronic(&event, &detector, &mut rng);
        assert_eq!(reco.jets.len(), 2);
        assert_eq!(reco.electrons.len(), 0);
        assert_eq!(reco.muons.len(), 0);
    }

    #[test]
    fn reconstructed_event_serde() {
        let reco = ReconstructedEvent {
            jets: vec![Jet {
                momentum: SpacetimeVector::new_4d(100.0, 50.0, 50.0, 50.0),
                constituents: vec![0, 1],
            }],
            electrons: vec![SpacetimeVector::new_4d(50.0, 30.0, 40.0, 0.0)],
            muons: vec![],
            photons: vec![],
            met: SpacetimeVector::new_4d(10.0, -8.0, -6.0, 0.0),
        };

        let json = serde_json::to_string(&reco).unwrap();
        let back: ReconstructedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.jets.len(), 1);
        assert_eq!(back.electrons.len(), 1);
        assert!((back.met_pt() - 10.0).abs() < 1e-8);
    }

    #[test]
    fn mixed_event_classification() {
        // e + ν + q + q̄ (like a W → eν event with hadronic recoil).
        let event = PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(50.0, 40.0, 30.0, 0.0), // electron
                SpacetimeVector::new_4d(50.0, -40.0, -30.0, 0.0), // neutrino
                SpacetimeVector::new_4d(50.0, 50.0, 0.0, 0.0),  // quark
                SpacetimeVector::new_4d(50.0, -50.0, 0.0, 0.0), // antiquark
            ],
            weight: 1.0,
        };
        let kinds = vec![
            ParticleKind::Electron,
            ParticleKind::Invisible,
            ParticleKind::Hadron,
            ParticleKind::Hadron,
        ];
        let detector = DetectorModel::perfect();
        let mut rng = StdRng::seed_from_u64(42);

        let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
        assert_eq!(reco.electrons.len(), 1);
        assert_eq!(reco.jets.len(), 2);
        // MET from the neutrino: px=40, py=30 (negative of invisible).
        // Visible: electron(40,30) + jet(50,0) + jet(-50,0) → sum_px=40, sum_py=30.
        // MET: (-40, -30) → pt = 50.
        assert!((reco.met_pt() - 50.0).abs() < 1e-8);
    }
}
