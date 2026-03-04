#![allow(dead_code)]
//! # Proptest Strategies — Reusable Generators for Physics Types
//!
//! This module defines `proptest` strategies for generating random instances
//! of the SPIRE kernel's core types:
//!
//! - [`arbitrary_mass`]: positive rest mass in $[0.001, 500]$ GeV
//! - [`arbitrary_four_momentum`]: on-shell 4-momentum with random spatial direction
//! - [`arbitrary_boost`]: subluminal Lorentz boost with consistent $\gamma$
//! - [`arbitrary_complex`]: complex number with bounded real/imaginary parts
//! - [`arbitrary_gamma_indices`]: even-length sequences of Lorentz indices $\in [0, 3]$
//! - [`arbitrary_garbage_string`]: random ASCII strings for fuzzing

use proptest::prelude::*;
use spire_kernel::algebra::{Complex, FourMomentum, MetricSignature, SpacetimeVector};
use spire_kernel::kinematics::LorentzBoost;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Mass & Energy
// ---------------------------------------------------------------------------

/// Generate a positive rest mass in $[0.001, 500]$ GeV.
///
/// Avoids exact zero (which triggers edge cases in many kinematic formulas)
/// and extreme values that would overflow f64 when squared.
pub fn arbitrary_mass() -> impl Strategy<Value = f64> {
    0.001_f64..=500.0
}

/// Generate a CM energy that is safely above the sum of two masses.
///
/// Returns $E_\text{cm} \in [1.01 \cdot (m_a + m_b),\ 100 \cdot (m_a + m_b)]$.
pub fn arbitrary_cms_energy(m_a: f64, m_b: f64) -> impl Strategy<Value = f64> {
    let mass_sum = m_a + m_b;
    let lo = mass_sum * 1.01;
    let hi = mass_sum * 100.0;
    lo..=hi
}

// ---------------------------------------------------------------------------
// Four-Momentum
// ---------------------------------------------------------------------------

/// Generate an on-shell `FourMomentum` with mass `m`.
///
/// The spatial direction is uniform on the sphere and the 3-momentum
/// magnitude is drawn from $[0.01, 1000]$ GeV.
pub fn arbitrary_four_momentum(m: f64) -> impl Strategy<Value = FourMomentum> {
    (0.01_f64..1000.0, -1.0_f64..1.0, 0.0_f64..2.0 * PI).prop_map(move |(p_mag, cos_theta, phi)| {
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let px = p_mag * sin_theta * phi.cos();
        let py = p_mag * sin_theta * phi.sin();
        let pz = p_mag * cos_theta;
        let e = (p_mag * p_mag + m * m).sqrt();
        FourMomentum::new(e, px, py, pz)
    })
}

/// Generate an on-shell `SpacetimeVector` (4D) with mass `m`.
pub fn arbitrary_spacetime_vector_4d(m: f64) -> impl Strategy<Value = SpacetimeVector> {
    arbitrary_four_momentum(m).prop_map(SpacetimeVector::from)
}

// ---------------------------------------------------------------------------
// Lorentz Boost
// ---------------------------------------------------------------------------

/// Generate a valid (subluminal) Lorentz boost.
///
/// Each component $\beta_i$ is drawn from $[-0.99, 0.99]$ with the constraint
/// $|\vec{\beta}|^2 < 1$. The Lorentz factor $\gamma$ is computed consistently.
pub fn arbitrary_boost() -> impl Strategy<Value = LorentzBoost> {
    // Draw each beta component from a restricted range so that even when all
    // three are at maximum, |β|² < 1.  The cube diagonal of 0.57 is ~0.987.
    (-0.57_f64..=0.57, -0.57_f64..=0.57, -0.57_f64..=0.57).prop_map(|(bx, by, bz)| {
        let beta_sq = bx * bx + by * by + bz * bz;
        // Guarantee subluminal (the range already ensures this, but be safe).
        let scale = if beta_sq >= 0.999 {
            0.999_f64.sqrt() / beta_sq.sqrt()
        } else {
            1.0
        };
        let bx = bx * scale;
        let by = by * scale;
        let bz = bz * scale;
        let beta_sq = bx * bx + by * by + bz * bz;
        let gamma = 1.0 / (1.0 - beta_sq).sqrt();
        LorentzBoost {
            beta: [bx, by, bz],
            gamma,
        }
    })
}

// ---------------------------------------------------------------------------
// Complex Numbers
// ---------------------------------------------------------------------------

/// Generate a complex number with bounded real and imaginary parts.
pub fn arbitrary_complex() -> impl Strategy<Value = Complex> {
    (-1000.0_f64..=1000.0, -1000.0_f64..=1000.0).prop_map(|(re, im)| Complex::new(re, im))
}

/// Generate a non-zero complex number (for division tests).
pub fn arbitrary_nonzero_complex() -> impl Strategy<Value = Complex> {
    arbitrary_complex().prop_filter("non-zero complex", |z| z.norm_sq() > 1e-20)
}

// ---------------------------------------------------------------------------
// Gamma Matrix Indices
// ---------------------------------------------------------------------------

/// Generate an even-length sequence of gamma matrix Lorentz indices.
///
/// Length is in $[2, 8]$ (even), indices in $[0, 3]$.
pub fn arbitrary_even_gamma_indices() -> impl Strategy<Value = Vec<u8>> {
    // Pick an even length, then fill with random indices.
    (1_usize..=4).prop_flat_map(|half_len| proptest::collection::vec(0_u8..4, half_len * 2))
}

/// Generate an odd-length sequence of gamma matrix Lorentz indices.
///
/// Length is in $[1, 7]$ (odd), indices in $[0, 3]$.
pub fn arbitrary_odd_gamma_indices() -> impl Strategy<Value = Vec<u8>> {
    (0_usize..=3).prop_flat_map(|half_len| proptest::collection::vec(0_u8..4, half_len * 2 + 1))
}

// ---------------------------------------------------------------------------
// Garbage Strings (Fuzzing)
// ---------------------------------------------------------------------------

/// Generate a random ASCII string up to 4096 bytes for fuzzing parsers.
pub fn arbitrary_garbage_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::num::u8::ANY, 0..4096)
        .prop_map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// Generate a random string that is valid UTF-8 (printable ASCII) up to 2048 chars.
pub fn arbitrary_ascii_string() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[[:print:]]{0,2048}").unwrap()
}

// ---------------------------------------------------------------------------
// Metric Signatures
// ---------------------------------------------------------------------------

/// Generate a Minkowski-like metric in $D$ dimensions ($D \in [1, 10]$).
pub fn arbitrary_minkowski_metric() -> impl Strategy<Value = MetricSignature> {
    (1_usize..=10).prop_map(MetricSignature::minkowski)
}
