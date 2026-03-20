//! # Numerical Stability & Precision Module
//!
//! This module provides stable accumulation strategies for large-scale
//! summations in Monte Carlo integration and histogram binning, preventing
//! catastrophic cancellation and floating-point drift.
//!
//! ## Kahan Compensated Summation
//!
//! The standard accumulation loop `sum += x` loses precision when adding
//! small values to large sums due to floating-point rounding. The Kahan
//! algorithm maintains a compensation term to recover the lost bits.
//!
//! **Algorithm**:
//! ```ignore
//! y = x - c                    // Subtract compensation
//! t = sum + y                  // Add to accumulator
//! c = (t - sum) - y            // Compute new compensation
//! sum = t                      // Update sum
//! ```
//!
//! This reduces the accumulated error from O(n·ε) to O(ε) for n additions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Detects and flags catastrophic cancellation in floating-point subtraction.
///
/// Returns `true` if the relative error of `(a - b)` is likely to exceed
/// machine epsilon due to cancellation of significant digits.
///
/// # Arguments
/// * `a`, `b` - Operands to check
/// * `epsilon` - Machine epsilon (e.g., `f64::EPSILON`)
///
/// # Example
/// ```ignore
/// if detect_cancellation(1e20, 1e20 - 1.0, f64::EPSILON) {
///     // Re-evaluate with higher precision
///     eprintln!("Catastrophic cancellation detected");
/// }
/// ```
pub fn detect_cancellation(a: f64, b: f64, epsilon: f64) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }

    // If subtraction rounded two large nearby values to exact equality,
    // treat this as a cancellation hotspot.
    if a == b && a != 0.0 {
        return true;
    }

    let diff = (a - b).abs();
    let scale = a.abs() + b.abs();
    if scale == 0.0 {
        return false;
    }

    // Relative cancellation check with an absolute floor to avoid
    // false positives in tiny-safe regions.
    let rel = diff / scale;
    let abs_floor = (a.abs().max(b.abs()) * 1e-30).max(f64::MIN_POSITIVE * 16.0);
    diff > abs_floor && rel < epsilon.sqrt() * 8.0
}

/// Kahan compensated summation accumulator.
///
/// Maintains two f64 fields to preserve precision in iterative summation:
/// - `sum`: The accumulated value
/// - `c`: The compensation term (accumulated rounding error)
///
/// # Examples
///
/// ```ignore
/// let mut acc = KahanAccumulator::new();
/// for value in large_dataset {
///     acc.add(value);
/// }
/// let result = acc.sum();
/// ```
///
/// # Performance Note
///
/// Kahan summation has ~3x the instruction count of naive summation,
/// but is essential when precision is critical (e.g., cross-section
/// totals where O(n) terms are summed).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KahanAccumulator {
    /// The accumulated sum.
    pub sum: f64,
    /// The compensation term (accumulated rounding error).
    c: f64,
}

impl KahanAccumulator {
    /// Create a new accumulator starting at zero.
    #[inline]
    pub fn new() -> Self {
        Self { sum: 0.0, c: 0.0 }
    }

    /// Create an accumulator with an initial value.
    #[inline]
    pub fn with_initial(initial: f64) -> Self {
        Self {
            sum: initial,
            c: 0.0,
        }
    }

    /// Add a value using Kahan compensated summation.
    ///
    /// This operation is numerically stable even when adding many
    /// small values to a large sum.
    #[inline]
    pub fn add(&mut self, value: f64) {
        let y = value - self.c;
        let t = self.sum + y;
        self.c = (t - self.sum) - y;
        self.sum = t;
    }

    /// Add a value using Neumaier compensation (robust when |value| > |sum|).
    #[inline]
    pub fn add_neumaier(&mut self, value: f64) {
        let t = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.c += (self.sum - t) + value;
        } else {
            self.c += (value - t) + self.sum;
        }
        self.sum = t;
    }

    /// Add multiple values in sequence.
    #[inline]
    pub fn add_batch(&mut self, values: &[f64]) {
        for &v in values {
            self.add(v);
        }
    }

    /// Get the accumulated sum.
    #[inline]
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Reset to zero.
    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.c = 0.0;
    }

    /// Get the current compensation term (for diagnostics).
    #[inline]
    pub fn compensation(&self) -> f64 {
        self.c
    }

    /// Compensated total value (sum + compensation).
    #[inline]
    pub fn compensated_sum(&self) -> f64 {
        self.sum + self.c
    }
}

impl Default for KahanAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KahanAccumulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sum)
    }
}

/// Merge two Kahan accumulators using Neumaier's improved algorithm.
///
/// When combining results from parallel threads, the compensation terms
/// must be carefully merged to preserve precision. This uses the
/// Neumaier variant to correctly handle the merging of two compensation
/// terms without loss.
///
/// # Algorithm
///
/// For two accumulators with sums s₁, s₂ and compensations c₁, c₂:
/// 1. Combine both compensations into the result compensation
/// 2. Use improved Kahan (Neumaier) logic for the merge itself
///
/// This ensures that precision lost in one thread's accumulation
/// is not lost again when merging.
impl std::ops::Add for KahanAccumulator {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        // Merge fully compensated values to preserve low-order bits across
        // parallel reduction boundaries.
        let mut result = KahanAccumulator::new();
        result.add_neumaier(self.sum);
        result.add_neumaier(self.c);
        result.add_neumaier(other.sum);
        result.add_neumaier(other.c);
        result
    }
}

/// Support parallel reduction in Rayon by implementing Sum trait
impl std::iter::Sum for KahanAccumulator {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(KahanAccumulator::new(), |a, b| a + b)
    }
}

/// Adaptive precision escalation flag.
///
/// When catastrophic cancellation is detected in a critical region,
/// the integrator may escalate to higher precision for that bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecisionEscalation {
    /// Standard f64 precision sufficient.
    Standard,
    /// Cancellation detected; re-evaluated with fallback strategy.
    EscalatedFallback,
}

/// Statistics for a Kahan accumulation session.
///
/// Tracks the number of add operations and any precision escalations,
/// useful for profiling and diagnostics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AccumulationStats {
    /// Total number of additions performed.
    pub add_count: usize,
    /// Number of cancellation detection events.
    pub cancellation_events: usize,
    /// Accumulated compensation term (absolute value).
    pub total_compensation: f64,
}

/// Stateful Kahan accumulator with diagnostics.
///
/// Extends `KahanAccumulator` with tracking of add operations
/// and escalation events, useful for profiling high-frequency
/// summation loops.
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticKahanAccumulator {
    inner: KahanAccumulator,
    stats: AccumulationStats,
}

/// Result of adaptive precision evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdaptivePrecisionResult {
    /// Final value used in accumulation.
    pub value: f64,
    /// Whether fallback path was required.
    pub escalation: PrecisionEscalation,
}

/// Evaluate a numerically sensitive expression with optional fallback.
///
/// `eval_primary` should be the normal fast path (`f64`), while
/// `eval_fallback` is a slower but more stable evaluation path.
pub fn evaluate_with_precision_fallback<P, F>(
    a: f64,
    b: f64,
    epsilon: f64,
    eval_primary: P,
    eval_fallback: F,
) -> AdaptivePrecisionResult
where
    P: FnOnce() -> f64,
    F: FnOnce() -> f64,
{
    let primary = eval_primary();
    if detect_cancellation(a, b, epsilon) || !primary.is_finite() {
        AdaptivePrecisionResult {
            value: eval_fallback(),
            escalation: PrecisionEscalation::EscalatedFallback,
        }
    } else {
        AdaptivePrecisionResult {
            value: primary,
            escalation: PrecisionEscalation::Standard,
        }
    }
}

impl DiagnosticKahanAccumulator {
    /// Create a new diagnostic accumulator.
    pub fn new() -> Self {
        Self {
            inner: KahanAccumulator::new(),
            stats: AccumulationStats::default(),
        }
    }

    /// Add a value and check for cancellation.
    pub fn add_with_check(&mut self, value: f64) {
        let old_sum = self.inner.sum;
        self.inner.add(value);

        self.stats.add_count += 1;

        if detect_cancellation(old_sum, value, f64::EPSILON) {
            self.stats.cancellation_events += 1;
        }

        self.stats.total_compensation += self.inner.c.abs();
    }

    /// Get the accumulated sum.
    pub fn sum(&self) -> f64 {
        self.inner.sum
    }

    /// Get the accumulated statistics.
    pub fn stats(&self) -> AccumulationStats {
        self.stats
    }

    /// Reset both accumulator and statistics.
    pub fn reset(&mut self) {
        self.inner.reset();
        self.stats = AccumulationStats::default();
    }
}

impl Default for DiagnosticKahanAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kahan_simple_sum() {
        let mut acc = KahanAccumulator::new();
        acc.add(1.0);
        acc.add(2.0);
        acc.add(3.0);
        assert!((acc.sum() - 6.0).abs() < 1e-15);
    }

    #[test]
    fn test_kahan_precision_vs_naive() {
        // Naive summation loses tiny increments near a huge baseline.
        let mut naive_sum = 0.0f64;
        naive_sum += 1.0e16;
        for _ in 0..10 {
            naive_sum += 1.0;
        }
        naive_sum -= 1.0e16;

        // Kahan summation preserves low-order bits through compensation.
        let mut acc = KahanAccumulator::new();
        acc.add(1.0e16);
        for _ in 0..10 {
            acc.add(1.0);
        }
        acc.add(-1.0e16);

        assert!(acc.compensated_sum() > naive_sum);
        assert!((acc.compensated_sum() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_kahan_initial_value() {
        let mut acc = KahanAccumulator::with_initial(100.0);
        acc.add(50.0);
        assert!((acc.sum() - 150.0).abs() < 1e-14);
    }

    #[test]
    fn test_kahan_batch_add() {
        let mut acc = KahanAccumulator::new();
        acc.add_batch(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((acc.sum() - 15.0).abs() < 1e-15);
    }

    #[test]
    fn test_detect_cancellation() {
        let large = 1e20;
        let small_diff = 1.0;

        assert!(detect_cancellation(large, large - small_diff, f64::EPSILON));
        assert!(!detect_cancellation(1.0, 2.0, f64::EPSILON));
    }

    #[test]
    fn test_diagnostic_accumulator() {
        let mut acc = DiagnosticKahanAccumulator::new();
        acc.add_with_check(1.0);
        acc.add_with_check(2.0);
        acc.add_with_check(3.0);

        assert_eq!(acc.stats().add_count, 3);
        assert!((acc.sum() - 6.0).abs() < 1e-15);
    }

    #[test]
    fn test_kahan_accumulator_addition() {
        let mut acc1 = KahanAccumulator::new();
        acc1.add(1.0);
        acc1.add(2.0);
        acc1.add(3.0);

        let mut acc2 = KahanAccumulator::new();
        acc2.add(4.0);
        acc2.add(5.0);
        acc2.add(6.0);

        let combined = acc1 + acc2;
        assert!((combined.sum() - 21.0).abs() < 1e-14);
    }

    #[test]
    fn test_kahan_parallel_reduction_neumaier() {
        // Simulate a parallel reduction with multiple accumulators
        let mut acc1 = KahanAccumulator::new();
        for i in 0..1000 {
            acc1.add((i as f64) * 1e-10);
        }

        let mut acc2 = KahanAccumulator::new();
        for i in 1000..2000 {
            acc2.add((i as f64) * 1e-10);
        }

        let mut acc3 = KahanAccumulator::new();
        for i in 2000..3000 {
            acc3.add((i as f64) * 1e-10);
        }

        // Merge using the Add operator (which uses Neumaier's algorithm)
        let combined = acc1 + acc2 + acc3;

        // Sequential reference
        let mut sequential = KahanAccumulator::new();
        for i in 0..3000 {
            sequential.add((i as f64) * 1e-10);
        }

        // The combined parallel result should match the sequential one
        // within machine epsilon (no loss of precision across merges)
        let relative_error =
            (combined.sum() - sequential.sum()).abs() / sequential.sum().abs().max(1.0);
        assert!(
            relative_error < 1e-14,
            "Parallel merge lost precision: parallel={}, sequential={}, rel_error={}",
            combined.sum(),
            sequential.sum(),
            relative_error
        );
    }

    #[test]
    fn test_kahan_sum_iterator_trait() {
        // Test that Rayon can use Sum trait for parallel reductions
        let accumulators = vec![
            {
                let mut acc = KahanAccumulator::new();
                acc.add(1.0);
                acc.add(2.0);
                acc
            },
            {
                let mut acc = KahanAccumulator::new();
                acc.add(3.0);
                acc.add(4.0);
                acc
            },
            {
                let mut acc = KahanAccumulator::new();
                acc.add(5.0);
                acc.add(6.0);
                acc
            },
        ];

        let sum: KahanAccumulator = accumulators.into_iter().sum();
        assert!((sum.sum() - 21.0).abs() < 1e-14);
    }

    #[test]
    fn test_kahan_rayon_parallel_sum() {
        // This test verifies that rayon can correctly reduce KahanAccumulators
        // in parallel using the Sum trait.
        use rayon::prelude::*;

        let data: Vec<f64> = (0..10000).map(|i| (i as f64) * 1e-12).collect();

        // Sequential reference using Kahan
        let mut sequential = KahanAccumulator::new();
        for &v in &data {
            sequential.add(v);
        }

        // Parallel reduction using Rayon's Sum trait
        let chunk_size = 1000;
        let parallel_result: KahanAccumulator = data
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut acc = KahanAccumulator::new();
                for &v in chunk {
                    acc.add(v);
                }
                acc
            })
            .sum();

        // Verify no precision loss from parallel merging
        let relative_error =
            (parallel_result.sum() - sequential.sum()).abs() / sequential.sum().abs().max(1.0);
        assert!(
            relative_error < 1e-13,
            "Rayon parallel reduction lost precision: parallel={}, sequential={}, rel_error={}",
            parallel_result.sum(),
            sequential.sum(),
            relative_error
        );
    }

    #[test]
    fn test_parallel_merge_10k_chunks_stability() {
        use rayon::prelude::*;

        let values: Vec<f64> = (0..100_000)
            .map(|i| if i % 2 == 0 { 1e12 } else { -1e12 + 1e-4 })
            .collect();

        let mut seq = KahanAccumulator::new();
        for &v in &values {
            seq.add(v);
        }

        let par: KahanAccumulator = values
            .par_chunks(10)
            .map(|chunk| {
                let mut local = KahanAccumulator::new();
                for &v in chunk {
                    local.add(v);
                }
                local
            })
            .sum();

        let rel = (par.compensated_sum() - seq.compensated_sum()).abs()
            / seq.compensated_sum().abs().max(1.0);
        assert!(rel < 1e-12, "10k merge instability: rel={}", rel);
    }

    #[test]
    fn test_adaptive_precision_fallback_trigger() {
        let out = evaluate_with_precision_fallback(
            1.0e16,
            1.0e16 - 1.0,
            f64::EPSILON,
            || (1.0e16 + 1.0) - 1.0e16,
            || 1.0,
        );

        assert_eq!(out.escalation, PrecisionEscalation::EscalatedFallback);
        assert!((out.value - 1.0).abs() < 1e-15);
    }

    #[cfg(test)]
    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_parallel_merge_exact(
                values_1 in prop::collection::vec(prop::num::f64::NORMAL, 10..100),
                values_2 in prop::collection::vec(prop::num::f64::NORMAL, 10..100),
            ) {
                let values_1: Vec<f64> = values_1.into_iter().filter(|v| v.is_finite()).collect();
                let values_2: Vec<f64> = values_2.into_iter().filter(|v| v.is_finite()).collect();

                prop_assume!(!values_1.is_empty() && !values_2.is_empty());

                // Sequential
                let mut sequential = KahanAccumulator::new();
                for v in &values_1 {
                    sequential.add(*v);
                }
                for v in &values_2 {
                    sequential.add(*v);
                }

                // Parallel merge
                let mut acc1 = KahanAccumulator::new();
                for v in &values_1 {
                    acc1.add(*v);
                }

                let mut acc2 = KahanAccumulator::new();
                for v in &values_2 {
                    acc2.add(*v);
                }

                let parallel = acc1 + acc2;

                  let sequential_sum = sequential.compensated_sum();
                  let parallel_sum = parallel.compensated_sum();

                  // Overflow/underflow edge cases can yield non-finite totals in
                  // both paths; only compare finite cases for relative precision.
                  prop_assume!(sequential_sum.is_finite() && parallel_sum.is_finite());

                // Check that parallel merge preserves precision
                  let relative_error = (parallel_sum - sequential_sum).abs()
                            / sequential_sum.abs().max(1.0);
                prop_assert!(relative_error < 1e-13,
                            "Parallel merge lost precision: parallel={}, sequential={}, rel_error={}",
                        parallel_sum, sequential_sum, relative_error);
            }

            #[test]
            fn prop_parallel_merge_many_chunks_preserves_precision(
                values in prop::collection::vec(-1e8f64..1e8f64, 10_000..11_000)
            ) {
                use rayon::prelude::*;

                let mut seq = KahanAccumulator::new();
                for &v in &values {
                    seq.add(v);
                }

                let par: KahanAccumulator = values
                    .par_chunks(1)
                    .map(|chunk| {
                        let mut local = KahanAccumulator::new();
                        local.add(chunk[0]);
                        local
                    })
                    .sum();

                let rel = (par.compensated_sum() - seq.compensated_sum()).abs()
                    / seq.compensated_sum().abs().max(1.0);
                prop_assert!(rel < 1e-11, "many-chunk merge drift: rel={}", rel);
            }
        }
    }
}
