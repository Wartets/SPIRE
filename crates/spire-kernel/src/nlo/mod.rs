//! # NLO - Next-to-Leading Order Corrections
//!
//! This module implements the theoretical infrastructure for computing NLO
//! QCD corrections to scattering cross-sections using the **Catani–Seymour
//! dipole subtraction** method.
//!
//! ## Infrared-Safe Observable Structure
//!
//! For an IR-safe observable $\mathcal{O}$, the NLO contribution can be cast
//! into finite Monte Carlo integrals by adding and subtracting a local
//! counterterm $d\sigma^A$ with the same soft/collinear limits as the real
//! matrix element:
//!
//! $$\langle \mathcal{O} \rangle_{\mathrm{NLO}} =
//! \int_{m+1} \left[d\sigma^R\,\mathcal{O}_{m+1} - d\sigma^A\,\mathcal{O}_m\right]
//! + \int_m \left[d\sigma^V + \int_1 d\sigma^A\right] \mathcal{O}_m$$
//!
//! The two bracketed terms are separately finite in four dimensions and are
//! therefore suitable for numerical integration.
//!
//! ## Architecture
//!
//! The subtraction method factorises soft and collinear singularities of
//! real-emission matrix elements into universal dipole terms that can be
//! analytically integrated over the unresolved one-particle phase space.
//!
//! $$\sigma^{\text{NLO}} = \int_{m+1} \left[ d\sigma^R - d\sigma^A \right]
//!     + \int_m \left[ d\sigma^V + \int_1 d\sigma^A \right]$$
//!
//! where $d\sigma^A$ is built from a sum of dipole contributions, each
//! constructed from a **kinematic mapping** and a **splitting function**.
//!
//! ## Implementation Scope
//!
//! The current implementation focuses on **massless final-final dipoles**.
//! This captures the dominant structure needed for many benchmark QCD
//! processes and defines extension points for:
//! - initial-state dipoles,
//! - massive emitter/spectator variants,
//! - integrated subtraction operators ($\mathbf{I}$, $\mathbf{K}$, $\mathbf{P}$).
//!
//! ## Submodules
//!
//! - [`dipole`] - Catani–Seymour kinematic mappings, splitting functions,
//!   dipole contributions, and the `SubtractionScheme` trait.

pub mod dipole;
