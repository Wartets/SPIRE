//! # Cosmology
//!
//! Modules for cosmological calculations, primarily dark matter relic
//! density via thermal freeze-out of the early universe.
//!
//! ## Background Expansion and Freeze-Out Regime
//!
//! The thermal relic calculation assumes a radiation-dominated expansion
//! around freeze-out, with Hubble rate
//!
//! $$H(T) = \sqrt{\frac{4\pi^3}{45}}\,\frac{\sqrt{g_*}\,T^2}{M_{\mathrm{Pl}}}$$
//!
//! and entropy density
//!
//! $$s(T) = \frac{2\pi^2}{45}\,g_{*s}\,T^3.$$
//!
//! Writing the Boltzmann equation in terms of the yield
//! $Y \equiv n/s$ and $x \equiv m_\chi/T$ isolates stiff early-time
//! behaviour and enables stable semi-analytic and numerical solvers.
//!
//! ## Physical Assumptions
//!
//! The current implementation targets the canonical WIMP-like scenario:
//! - standard cosmological thermal history,
//! - no late entropy injection,
//! - annihilation expansion $\langle\sigma v\rangle \simeq a + 6b/x$.
//!
//! These assumptions can be relaxed in future modules (coannihilation,
//! non-standard expansion, Sommerfeld enhancement, etc.).
//!
//! ## Modules
//!
//! - [`relic`] - Boltzmann equation solver for computing the cosmological
//!   relic abundance $\Omega h^2$ of a dark matter candidate.

pub mod relic;
