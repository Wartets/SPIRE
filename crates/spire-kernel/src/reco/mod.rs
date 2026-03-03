//! # Reco — Reconstruction & Detector Simulation
//!
//! This module bridges parton-level Monte Carlo truth and experimentally
//! comparable reconstructed observables. It provides:
//!
//! - **Jet Clustering** ([`clustering`]): The Anti-$k_t$ sequential
//!   recombination algorithm for grouping hadronic energy deposits
//!   into jets.
//! - **Detector Response** ([`detector`]): Phenomenological simulation
//!   of energy smearing, reconstruction efficiency, and missing
//!   transverse energy computation.
//!
//! ## Workflow
//!
//! A typical reconstruction chain:
//!
//! 1. Generate a `PhaseSpacePoint` (truth-level 4-momenta).
//! 2. Classify each particle (electron, muon, photon, hadron, invisible).
//! 3. Call [`detector::reconstruct_event`] to apply the detector model.
//! 4. The result is a [`detector::ReconstructedEvent`] containing
//!    jets, isolated leptons, photons, and $E_T^{\mathrm{miss}}$.
//!
//! ## Example
//!
//! ```no_run
//! use spire_kernel::reco::detector::{DetectorModel, ParticleKind, reconstruct_event};
//! use spire_kernel::kinematics::PhaseSpacePoint;
//! use spire_kernel::algebra::SpacetimeVector;
//!
//! let event = PhaseSpacePoint {
//!     momenta: vec![
//!         SpacetimeVector::new_4d(100.0, 50.0, 50.0, 50.0),
//!         SpacetimeVector::new_4d(100.0, -50.0, -50.0, -50.0),
//!     ],
//!     weight: 1.0,
//! };
//!
//! let detector = DetectorModel::lhc_like();
//! let kinds = vec![ParticleKind::Hadron, ParticleKind::Hadron];
//! let mut rng = rand::thread_rng();
//!
//! let reco = reconstruct_event(&event, &kinds, &detector, &mut rng);
//! println!("Jets: {}, MET: {:.1} GeV", reco.jets.len(), reco.met_pt());
//! ```

pub mod clustering;
pub mod detector;
