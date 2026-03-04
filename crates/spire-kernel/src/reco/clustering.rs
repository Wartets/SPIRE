//! # Jet Clustering — Anti-$k_t$ Algorithm
//!
//! This module implements the **Anti-$k_t$** sequential recombination
//! jet clustering algorithm (Cacciari, Salam, Soyez 2008), the
//! industry standard for hadron-collider jet finding.
//!
//! ## Distance Metrics
//!
//! The Anti-$k_t$ algorithm uses two distance measures:
//!
//! **Pairwise distance** between pseudo-jets $i$ and $j$:
//! $$d_{ij} = \min(k_{ti}^{2p},\, k_{tj}^{2p}) \cdot \frac{\Delta R_{ij}^2}{R^2}$$
//!
//! **Beam distance** for pseudo-jet $i$:
//! $$d_{iB} = k_{ti}^{2p}$$
//!
//! where $p = -1$ for Anti-$k_t$, $k_{ti} = p_{T,i}$, and
//! $\Delta R_{ij}^2 = (\eta_i - \eta_j)^2 + (\phi_i - \phi_j)^2$.
//!
//! ## Recombination Scheme
//!
//! Uses the **E-scheme** (4-momentum addition): when merging pseudo-jets
//! $i$ and $j$, the combined 4-momentum is $p^\mu_{ij} = p^\mu_i + p^\mu_j$.
//!
//! ## Complexity
//!
//! The naive implementation is $O(N^3)$ per event. Since parton-level
//! events typically have $N \lesssim 6$ final-state partons, this is
//! negligible compared to the matrix-element evaluation cost.

use serde::{Deserialize, Serialize};

use crate::algebra::SpacetimeVector;

// ===========================================================================
// Jet
// ===========================================================================

/// A reconstructed jet: a collimated spray of hadronic energy.
///
/// Wraps the combined 4-momentum of all constituent partons plus
/// a list of constituent indices (into the original parton list).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jet {
    /// Combined 4-momentum of the jet (E-scheme recombination).
    pub momentum: SpacetimeVector,
    /// Indices of the original input pseudo-jets that were clustered
    /// into this jet. Useful for constituent-level analysis.
    pub constituents: Vec<usize>,
}

impl Jet {
    /// Transverse momentum $p_T = \sqrt{p_x^2 + p_y^2}$.
    #[inline]
    pub fn pt(&self) -> f64 {
        let px = self.momentum[1];
        let py = self.momentum[2];
        (px * px + py * py).sqrt()
    }

    /// Pseudorapidity $\eta = \mathrm{atanh}(p_z / |\vec{p}|)$.
    #[inline]
    pub fn eta(&self) -> f64 {
        let px = self.momentum[1];
        let py = self.momentum[2];
        let pz = self.momentum[3];
        let p_mag = (px * px + py * py + pz * pz).sqrt();
        if p_mag < 1e-300 {
            return 0.0;
        }
        (pz / p_mag).atanh()
    }

    /// Azimuthal angle $\phi = \mathrm{atan2}(p_y, p_x)$.
    #[inline]
    pub fn phi(&self) -> f64 {
        self.momentum[2].atan2(self.momentum[1])
    }

    /// Energy component.
    #[inline]
    pub fn energy(&self) -> f64 {
        self.momentum[0]
    }

    /// Invariant mass $m = \sqrt{|E^2 - |\vec{p}|^2|}$.
    #[inline]
    pub fn mass(&self) -> f64 {
        let e = self.momentum[0];
        let px = self.momentum[1];
        let py = self.momentum[2];
        let pz = self.momentum[3];
        let m_sq = e * e - px * px - py * py - pz * pz;
        m_sq.abs().sqrt()
    }

    /// Number of constituents.
    #[inline]
    pub fn n_constituents(&self) -> usize {
        self.constituents.len()
    }
}

// ===========================================================================
// Jet Algorithm Enum
// ===========================================================================

/// Generalised $k_t$-family jet algorithm selection.
///
/// The parameter $p$ controls the distance metric behaviour:
/// - $p = -1$ (Anti-$k_t$): hard seeds absorb soft radiation → circular jets.
/// - $p = 0$ (Cambridge/Aachen): purely geometric clustering.
/// - $p = +1$ (Inclusive $k_t$): soft particles cluster first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum JetAlgorithm {
    /// Anti-$k_t$ ($p = -1$). The standard at the LHC.
    AntiKt,
    /// Cambridge/Aachen ($p = 0$).
    CambridgeAachen,
    /// Inclusive $k_t$ ($p = +1$).
    Kt,
}

impl JetAlgorithm {
    /// The exponent $p$ used in the distance metric.
    #[inline]
    pub fn power(&self) -> f64 {
        match self {
            JetAlgorithm::AntiKt => -1.0,
            JetAlgorithm::CambridgeAachen => 0.0,
            JetAlgorithm::Kt => 1.0,
        }
    }
}

// ===========================================================================
// Cluster Sequence
// ===========================================================================

/// Internal pseudo-jet used during the clustering sequence.
///
/// Caches kinematic quantities to avoid recomputation.
#[derive(Debug, Clone)]
struct PseudoJet {
    /// 4-momentum.
    momentum: SpacetimeVector,
    /// Transverse momentum.
    pt: f64,
    /// Pseudorapidity.
    eta: f64,
    /// Azimuthal angle.
    phi: f64,
    /// Original constituent indices (into the input parton list).
    constituents: Vec<usize>,
}

impl PseudoJet {
    /// Create a pseudo-jet from a 4-momentum with a single constituent index.
    fn from_momentum(momentum: SpacetimeVector, index: usize) -> Self {
        let px = momentum[1];
        let py = momentum[2];
        let pz = momentum[3];
        let pt = (px * px + py * py).sqrt();
        let p_mag = (px * px + py * py + pz * pz).sqrt();
        let eta = if p_mag > 1e-300 {
            (pz / p_mag).atanh()
        } else {
            0.0
        };
        let phi = py.atan2(px);

        Self {
            momentum,
            pt,
            eta,
            phi,
            constituents: vec![index],
        }
    }

    /// Merge two pseudo-jets using the E-scheme (4-momentum addition).
    fn merge(a: &PseudoJet, b: &PseudoJet) -> Self {
        let combined = a.momentum.clone() + b.momentum.clone();
        let px = combined[1];
        let py = combined[2];
        let pz = combined[3];
        let pt = (px * px + py * py).sqrt();
        let p_mag = (px * px + py * py + pz * pz).sqrt();
        let eta = if p_mag > 1e-300 {
            (pz / p_mag).atanh()
        } else {
            0.0
        };
        let phi = py.atan2(px);

        let mut constituents = a.constituents.clone();
        constituents.extend_from_slice(&b.constituents);

        Self {
            momentum: combined,
            pt,
            eta,
            phi,
            constituents,
        }
    }
}

/// Compute $\Delta R^2$ between two pseudo-jets in ($\eta$, $\phi$) space.
///
/// Handles the $2\pi$ periodicity of $\phi$.
#[inline]
fn delta_r_sq(a: &PseudoJet, b: &PseudoJet) -> f64 {
    let d_eta = a.eta - b.eta;
    let mut d_phi = a.phi - b.phi;
    // Wrap to [-π, π].
    if d_phi > std::f64::consts::PI {
        d_phi -= 2.0 * std::f64::consts::PI;
    } else if d_phi < -std::f64::consts::PI {
        d_phi += 2.0 * std::f64::consts::PI;
    }
    d_eta * d_eta + d_phi * d_phi
}

/// Run the sequential recombination jet clustering algorithm.
///
/// This is the main entry point for jet finding. It implements the
/// generalised $k_t$-family algorithm with configurable exponent $p$
/// and jet radius $R$.
///
/// # Algorithm
///
/// 1. Initialise pseudo-jets from input 4-momenta.
/// 2. Compute all pairwise distances $d_{ij}$ and beam distances $d_{iB}$.
/// 3. Find the global minimum.
///    - If $d_{ij}$: merge pseudo-jets $i$ and $j$ (E-scheme).
///    - If $d_{iB}$: declare pseudo-jet $i$ a jet and remove it.
/// 4. Repeat until no pseudo-jets remain.
/// 5. Return jets sorted by descending $p_T$.
///
/// # Arguments
///
/// * `inputs` — 4-momenta of the input particles (partons or calorimeter deposits).
/// * `algorithm` — Jet algorithm selection (Anti-$k_t$, C/A, or $k_t$).
/// * `radius` — The jet radius parameter $R$.
///
/// # Returns
///
/// A vector of [`Jet`] objects sorted by descending transverse momentum.
pub fn cluster_jets(inputs: &[SpacetimeVector], algorithm: JetAlgorithm, radius: f64) -> Vec<Jet> {
    if inputs.is_empty() {
        return Vec::new();
    }

    let p = algorithm.power();
    let r_sq = radius * radius;
    let two_p = 2.0 * p;

    // Initialise pseudo-jets.
    let mut pseudo_jets: Vec<PseudoJet> = inputs
        .iter()
        .enumerate()
        .map(|(i, mom)| PseudoJet::from_momentum(mom.clone(), i))
        .collect();

    let mut jets: Vec<Jet> = Vec::new();

    // Main clustering loop.
    while !pseudo_jets.is_empty() {
        let n = pseudo_jets.len();

        if n == 1 {
            // Only one pseudo-jet left → declare it a jet.
            let pj = pseudo_jets.remove(0);
            jets.push(Jet {
                momentum: pj.momentum,
                constituents: pj.constituents,
            });
            break;
        }

        // Find the minimum distance.
        let mut min_d = f64::INFINITY;
        let mut min_i = 0;
        let mut min_j: Option<usize> = None; // None means beam distance.

        // Compute beam distances d_{iB}.
        for (i, jet) in pseudo_jets.iter().enumerate().take(n) {
            let pt_i = jet.pt;
            let d_ib = if pt_i > 1e-300 {
                pt_i.powf(two_p)
            } else {
                // Very soft particle: d_iB → ∞ for Anti-k_t (p=-1).
                if p < 0.0 {
                    f64::INFINITY
                } else {
                    0.0
                }
            };

            if d_ib < min_d {
                min_d = d_ib;
                min_i = i;
                min_j = None;
            }
        }

        // Compute pairwise distances d_{ij}.
        for i in 0..n {
            let pt_i = pseudo_jets[i].pt;
            let kti_2p = if pt_i > 1e-300 {
                pt_i.powf(two_p)
            } else if p < 0.0 {
                f64::INFINITY
            } else {
                0.0
            };

            for j in (i + 1)..n {
                let pt_j = pseudo_jets[j].pt;
                let ktj_2p = if pt_j > 1e-300 {
                    pt_j.powf(two_p)
                } else if p < 0.0 {
                    f64::INFINITY
                } else {
                    0.0
                };

                let min_kt = kti_2p.min(ktj_2p);
                let dr2 = delta_r_sq(&pseudo_jets[i], &pseudo_jets[j]);
                let d_ij = min_kt * dr2 / r_sq;

                if d_ij < min_d {
                    min_d = d_ij;
                    min_i = i;
                    min_j = Some(j);
                }
            }
        }

        match min_j {
            Some(j) => {
                // Merge pseudo-jets i and j.
                let merged = PseudoJet::merge(&pseudo_jets[min_i], &pseudo_jets[j]);
                // Remove j first (larger index), then i.
                pseudo_jets.remove(j);
                pseudo_jets.remove(min_i);
                pseudo_jets.push(merged);
            }
            None => {
                // Beam clustering: declare i a jet.
                let pj = pseudo_jets.remove(min_i);
                jets.push(Jet {
                    momentum: pj.momentum,
                    constituents: pj.constituents,
                });
            }
        }
    }

    // Sort jets by descending pT.
    jets.sort_by(|a, b| {
        b.pt()
            .partial_cmp(&a.pt())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    jets
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Helper: create a massless 4-vector from (pT, eta, phi).
    fn make_massless(pt: f64, eta: f64, phi: f64) -> SpacetimeVector {
        let px = pt * phi.cos();
        let py = pt * phi.sin();
        let pz = pt * eta.sinh();
        let e = (px * px + py * py + pz * pz).sqrt();
        SpacetimeVector::new_4d(e, px, py, pz)
    }

    #[test]
    fn empty_input_returns_no_jets() {
        let jets = cluster_jets(&[], JetAlgorithm::AntiKt, 0.4);
        assert!(jets.is_empty());
    }

    #[test]
    fn single_parton_becomes_one_jet() {
        let inputs = vec![make_massless(50.0, 0.0, 0.0)];
        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 1);
        assert!((jets[0].pt() - 50.0).abs() < 1e-10);
        assert_eq!(jets[0].constituents, vec![0]);
    }

    #[test]
    fn two_collinear_partons_merge() {
        // Two partons at nearly the same (eta, phi) → should merge.
        let p1 = make_massless(30.0, 0.0, 0.0);
        let p2 = make_massless(20.0, 0.1, 0.05); // ΔR ≈ 0.11 < R=0.4
        let inputs = vec![p1, p2];

        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 1, "Collinear partons should merge into one jet");
        assert_eq!(jets[0].n_constituents(), 2);
        // Combined pT ≈ 30 + 20 = 50 (approximately, since directions differ slightly).
        assert!(jets[0].pt() > 45.0);
    }

    #[test]
    fn two_separated_partons_stay_separate() {
        // Two partons far apart in η → should remain separate jets.
        let p1 = make_massless(50.0, 2.0, 0.0);
        let p2 = make_massless(40.0, -2.0, PI);
        let inputs = vec![p1, p2];

        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 2, "Well-separated partons should form two jets");
        // Should be pT-ordered.
        assert!(jets[0].pt() >= jets[1].pt());
        assert!((jets[0].pt() - 50.0).abs() < 1e-10);
        assert!((jets[1].pt() - 40.0).abs() < 1e-10);
    }

    #[test]
    fn anti_kt_hard_seed_absorbs_soft() {
        // Hard parton (100 GeV) with a soft companion (5 GeV) nearby.
        // Anti-k_t should merge the soft into the hard jet, not vice versa.
        let hard = make_massless(100.0, 0.0, 0.0);
        let soft = make_massless(5.0, 0.2, 0.1); // ΔR ≈ 0.22 < R=0.4
        let inputs = vec![hard, soft];

        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 1);
        assert_eq!(jets[0].n_constituents(), 2);
        assert!(jets[0].pt() > 100.0); // hard + soft
    }

    #[test]
    fn three_partons_two_close_one_far() {
        // Partons 0 and 1 close together; parton 2 far away.
        let p0 = make_massless(40.0, 1.0, 0.5);
        let p1 = make_massless(30.0, 1.1, 0.6); // ΔR ≈ 0.14 < R=0.4
        let p2 = make_massless(60.0, -1.0, 2.5); // far away
        let inputs = vec![p0, p1, p2];

        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(
            jets.len(),
            2,
            "Should get 2 jets: merged(0,1) and isolated(2)"
        );

        // Leading jet should be parton 2 (60 GeV) or merged(0+1, ~70 GeV).
        let total_01 = jets.iter().find(|j| j.n_constituents() == 2);
        let isolated = jets.iter().find(|j| j.n_constituents() == 1);
        assert!(total_01.is_some());
        assert!(isolated.is_some());
    }

    #[test]
    fn jets_sorted_by_descending_pt() {
        let inputs = vec![
            make_massless(10.0, 0.0, 0.0),
            make_massless(50.0, 2.0, 1.0),
            make_massless(30.0, -2.0, -1.0),
        ];
        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        for i in 0..jets.len().saturating_sub(1) {
            assert!(
                jets[i].pt() >= jets[i + 1].pt(),
                "Jets should be pT-ordered"
            );
        }
    }

    #[test]
    fn jet_kinematic_accessors() {
        let mom = make_massless(50.0, 1.5, 0.8);
        let jet = Jet {
            momentum: mom.clone(),
            constituents: vec![0],
        };
        assert!((jet.pt() - 50.0).abs() < 1e-8);
        assert!((jet.eta() - 1.5).abs() < 0.01);
        assert!((jet.phi() - 0.8).abs() < 0.01);
        assert!(jet.energy() > 0.0);
    }

    #[test]
    fn cambridge_aachen_algorithm() {
        // C/A should still cluster nearby partons.
        let p1 = make_massless(50.0, 0.0, 0.0);
        let p2 = make_massless(45.0, 0.15, 0.1);
        let jets = cluster_jets(&[p1, p2], JetAlgorithm::CambridgeAachen, 0.4);
        assert_eq!(jets.len(), 1, "C/A should merge close partons");
    }

    #[test]
    fn kt_algorithm() {
        // k_t should also merge nearby partons.
        let p1 = make_massless(50.0, 0.0, 0.0);
        let p2 = make_massless(45.0, 0.15, 0.1);
        let jets = cluster_jets(&[p1, p2], JetAlgorithm::Kt, 0.4);
        assert_eq!(jets.len(), 1, "k_t should merge close partons");
    }

    #[test]
    fn four_partons_back_to_back() {
        // Two pairs of partons at opposite η: should get 2 jets.
        let inputs = vec![
            make_massless(50.0, 2.0, 0.0),
            make_massless(30.0, 2.1, 0.05), // close to parton 0
            make_massless(40.0, -2.0, PI),
            make_massless(20.0, -1.9, PI - 0.05), // close to parton 2
        ];
        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 2);
    }

    #[test]
    fn jet_mass_from_merged_partons() {
        // Two massless partons at right angles should produce a massive jet.
        let p1 = SpacetimeVector::new_4d(50.0, 50.0, 0.0, 0.0);
        let p2 = SpacetimeVector::new_4d(50.0, 0.0, 50.0, 0.0);
        // ΔR between these: both at η≈0 but different φ.
        // φ1 = atan2(0, 50) = 0, φ2 = atan2(50, 0) = π/2 ≈ 1.57.
        // ΔR = sqrt(0 + (π/2)²) ≈ 1.57 > R=0.4 → separate jets with R=0.4.
        // Use R=2.0 to force merge.
        let jets = cluster_jets(&[p1, p2], JetAlgorithm::AntiKt, 2.0);
        assert_eq!(jets.len(), 1);
        // Merged: (100, 50, 50, 0) → m = sqrt(10000 - 2500 - 2500) = sqrt(5000) ≈ 70.7
        assert!((jets[0].mass() - 5000.0_f64.sqrt()).abs() < 1.0);
    }

    #[test]
    fn phi_wrapping_merges_correctly() {
        // Two partons near φ = ±π should recognise they are close.
        let p1 = make_massless(50.0, 0.0, PI - 0.1);
        let p2 = make_massless(40.0, 0.0, -(PI - 0.1));
        // Δφ = (π-0.1) - (-(π-0.1)) = 2π - 0.2, wrapped → 0.2
        // ΔR = sqrt(0 + 0.04) = 0.2 < R=0.4 → should merge.
        let jets = cluster_jets(&[p1, p2], JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 1, "Phi wrapping should merge these partons");
    }

    #[test]
    fn constituent_indices_preserved() {
        let inputs = vec![
            make_massless(50.0, 0.0, 0.0),
            make_massless(30.0, 0.1, 0.05), // merges with 0
            make_massless(60.0, 3.0, 2.0),  // isolated
        ];
        let jets = cluster_jets(&inputs, JetAlgorithm::AntiKt, 0.4);
        assert_eq!(jets.len(), 2);

        // Find the merged jet.
        let merged = jets.iter().find(|j| j.n_constituents() == 2).unwrap();
        let mut idx = merged.constituents.clone();
        idx.sort();
        assert_eq!(idx, vec![0, 1]);
    }

    #[test]
    fn jet_serde_roundtrip() {
        let jet = Jet {
            momentum: SpacetimeVector::new_4d(100.0, 30.0, 40.0, 50.0),
            constituents: vec![0, 1, 2],
        };
        let json = serde_json::to_string(&jet).unwrap();
        let back: Jet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.constituents, vec![0, 1, 2]);
        assert!((back.pt() - jet.pt()).abs() < 1e-10);
    }
}
