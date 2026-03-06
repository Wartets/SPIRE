//! # Telemetry - Kernel-Level Performance Profiling
//!
//! This module provides lightweight instrumentation for tracking execution
//! time across distinct computation stages. The design prioritises minimal
//! overhead in hot paths (Monte Carlo event loops) while capturing
//! meaningful performance breakdowns for the user interface.
//!
//! ## Architecture
//!
//! - [`ComputeProfile`] is a generic, serializable container mapping
//!   arbitrary stage names to wall-clock milliseconds. The frontend
//!   renders whatever keys the backend provides - no hardcoded stage names.
//!
//! - [`ScopedTimer`] is a RAII guard that records `Instant::now()` on
//!   creation and writes the elapsed time to a `ComputeProfile` on drop.
//!   It is used to bracket top-level pipeline stages, **not** individual
//!   Monte Carlo events. For the MC hot loop, the entire integration is
//!   wrapped in a single timer; convergence snapshots are appended at
//!   logarithmically-spaced intervals from within the integrator.
//!
//! ## Performance Considerations
//!
//! `ScopedTimer` adds one `Instant::now()` call at stage entry and one
//! `HashMap::insert` at stage exit. This amounts to ~50ns overhead per
//! stage boundary - negligible compared to the millisecond-scale
//! computation it measures. The convergence snapshot vector is pre-allocated
//! to avoid reallocation during integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

// ===========================================================================
// Compute Profile
// ===========================================================================

/// A generic performance profile for any computation pipeline.
///
/// The `stage_timings` map uses arbitrary string keys so that the
/// frontend UI can dynamically render whatever stages the backend
/// reports - including stages injected by future external solvers.
///
/// # Serialization
///
/// Serializes to JSON with the following shape:
/// ```json
/// {
///   "stage_timings": { "Phase-Space Sampling": 12.5, "Matrix Element": 85.3 },
///   "total_time_ms": 100.2,
///   "peak_memory_mb": 42.0,
///   "threads_used": 8,
///   "convergence_data": [[1000, 0.15], [2000, 0.11], [5000, 0.07]]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeProfile {
    /// Mapping from stage name to wall-clock execution time in milliseconds.
    ///
    /// Keys are arbitrary strings like `"Topology Generation"`,
    /// `"Monte Carlo Integration"`, `"Amplitude Derivation"`, etc.
    /// The frontend iterates over these dynamically.
    pub stage_timings: HashMap<String, f64>,

    /// Total wall-clock time for the entire computation in milliseconds.
    ///
    /// This may exceed the sum of `stage_timings` due to overhead between
    /// stages (serialization, setup, etc.).
    pub total_time_ms: f64,

    /// Peak resident memory usage in megabytes.
    ///
    /// Set to `0.0` when memory tracking is unavailable (e.g., on
    /// platforms without `/proc/self/status` or equivalent).
    pub peak_memory_mb: f64,

    /// Number of threads used during computation.
    ///
    /// `1` for sequential pipelines, rayon thread pool size for parallel
    /// Monte Carlo integration.
    pub threads_used: u32,

    /// Convergence snapshots from Monte Carlo integration.
    ///
    /// Each entry is `(events_evaluated, current_relative_error)`.
    /// Collected at logarithmically-spaced intervals during integration
    /// to power the $1/\sqrt{N}$ convergence plot in the UI.
    ///
    /// Empty for non-integration computations.
    pub convergence_data: Vec<(usize, f64)>,
}

impl Default for ComputeProfile {
    fn default() -> Self {
        Self {
            stage_timings: HashMap::new(),
            total_time_ms: 0.0,
            peak_memory_mb: 0.0,
            threads_used: 1,
            convergence_data: Vec::new(),
        }
    }
}

impl ComputeProfile {
    /// Create a new empty profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a timing entry for a named stage.
    pub fn record_stage(&mut self, name: impl Into<String>, elapsed_ms: f64) {
        self.stage_timings.insert(name.into(), elapsed_ms);
    }

    /// Add a convergence snapshot.
    pub fn push_convergence(&mut self, events: usize, relative_error: f64) {
        self.convergence_data.push((events, relative_error));
    }

    /// Finalize the profile by computing total time from a start instant.
    pub fn finalize(&mut self, start: Instant) {
        self.total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    }

    /// Attempt to read peak resident memory from the OS.
    ///
    /// On Windows, uses `GetProcessMemoryInfo`. On Linux, reads
    /// `/proc/self/status` for `VmHWM`. Falls back to 0.0 elsewhere.
    pub fn capture_memory(&mut self) {
        self.peak_memory_mb = read_peak_memory_mb();
    }

    /// Set the thread count from the current rayon pool.
    pub fn capture_threads(&mut self) {
        self.threads_used = rayon::current_num_threads() as u32;
    }
}

// ===========================================================================
// Scoped Timer (RAII)
// ===========================================================================

/// A RAII timer that records elapsed wall-clock time into a
/// [`ComputeProfile`] when dropped.
///
/// # Usage
///
/// ```rust,ignore
/// let mut profile = ComputeProfile::new();
/// {
///     let _timer = ScopedTimer::new("Topology Generation", &mut profile);
///     // ... expensive computation ...
/// } // _timer drops here, recording elapsed time
/// ```
///
/// # Performance
///
/// Overhead is approximately 50ns (two `Instant::now()` calls and one
/// `HashMap::insert`). Suitable for stage-level granularity, **not** for
/// per-event instrumentation in Monte Carlo hot loops.
pub struct ScopedTimer<'a> {
    label: String,
    start: Instant,
    profile: &'a mut ComputeProfile,
}

impl<'a> ScopedTimer<'a> {
    /// Start a new scoped timer for the given stage name.
    pub fn new(label: impl Into<String>, profile: &'a mut ComputeProfile) -> Self {
        Self {
            label: label.into(),
            start: Instant::now(),
            profile,
        }
    }
}

impl Drop for ScopedTimer<'_> {
    fn drop(&mut self) {
        let elapsed_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        self.profile
            .stage_timings
            .insert(self.label.clone(), elapsed_ms);
    }
}

// ===========================================================================
// Platform-Specific Memory Reading
// ===========================================================================

/// Read peak resident memory in megabytes from the operating system.
///
/// Returns `0.0` on unsupported platforms.
#[cfg(target_os = "windows")]
fn read_peak_memory_mb() -> f64 {
    use std::mem;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct ProcessMemoryCounters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }

    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(
            process: isize,
            ppsmemCounters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }

    unsafe {
        let mut counters: ProcessMemoryCounters = mem::zeroed();
        counters.cb = mem::size_of::<ProcessMemoryCounters>() as u32;
        let handle = GetCurrentProcess();
        if K32GetProcessMemoryInfo(handle, &mut counters, counters.cb) != 0 {
            counters.PeakWorkingSetSize as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }
}

#[cfg(target_os = "linux")]
fn read_peak_memory_mb() -> f64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmHWM:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn read_peak_memory_mb() -> f64 {
    0.0
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn scoped_timer_records_timing() {
        let mut profile = ComputeProfile::new();
        {
            let _timer = ScopedTimer::new("test_stage", &mut profile);
            thread::sleep(Duration::from_millis(10));
        }
        assert!(profile.stage_timings.contains_key("test_stage"));
        assert!(profile.stage_timings["test_stage"] >= 5.0); // at least 5ms
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let mut profile = ComputeProfile::new();
        profile.record_stage("Phase-Space Sampling", 12.5);
        profile.record_stage("Matrix Element", 85.3);
        profile.total_time_ms = 100.2;
        profile.peak_memory_mb = 42.0;
        profile.threads_used = 8;
        profile.push_convergence(1000, 0.15);
        profile.push_convergence(5000, 0.07);

        let json = serde_json::to_string(&profile).unwrap();
        let restored: ComputeProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.stage_timings.len(), 2);
        assert!((restored.stage_timings["Phase-Space Sampling"] - 12.5).abs() < 1e-10);
        assert!((restored.total_time_ms - 100.2).abs() < 1e-10);
        assert_eq!(restored.threads_used, 8);
        assert_eq!(restored.convergence_data.len(), 2);
        assert_eq!(restored.convergence_data[0].0, 1000);
    }

    #[test]
    fn default_profile_is_empty() {
        let profile = ComputeProfile::new();
        assert!(profile.stage_timings.is_empty());
        assert_eq!(profile.total_time_ms, 0.0);
        assert_eq!(profile.threads_used, 1);
        assert!(profile.convergence_data.is_empty());
    }

    #[test]
    fn memory_capture_does_not_panic() {
        let mut profile = ComputeProfile::new();
        profile.capture_memory();
        // On any platform, this should not panic and should return >= 0
        assert!(profile.peak_memory_mb >= 0.0);
    }
}
