//! # SPIRE Plugin API
//!
//! Lightweight, standalone crate defining the public interface between
//! SPIRE and its WASM extension plugins.  This crate has **minimal
//! dependencies** (only `serde`) so that plugin authors can compile
//! against it without pulling in the full kernel.
//!
//! ## Design Principles
//!
//! - **Stability**: Types here form a versioned ABI contract.  Breaking
//!   changes require a semver bump.
//! - **Serialization**: All types crossing the WASM boundary are
//!   serialized as JSON via `serde`.  This avoids raw pointer exchange
//!   and guarantees crash isolation.
//! - **Capability-based**: Plugins declare which hooks they implement
//!   via [`PluginCapability`] flags in their metadata.

use serde::{Deserialize, Serialize};

// ===========================================================================
// Plugin Metadata
// ===========================================================================

/// Semantic version triple for ABI compatibility checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Capabilities that a plugin can declare.
///
/// The host engine only dispatches hooks that the plugin has explicitly
/// registered, minimising overhead for lightweight extensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCapability {
    /// Apply kinematic cuts to phase-space events.
    KinematicCut,
    /// Compute a custom observable from event kinematics.
    CustomObservable,
    /// Provide a custom matrix element squared contribution.
    CustomMatrixElement,
    /// Post-process analysis histograms.
    AnalysisPostProcess,
}

/// Metadata block exported by every conforming plugin.
///
/// The host reads this from the WASM module's `spire_plugin_metadata`
/// exported function which returns a JSON string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version.
    pub version: PluginVersion,
    /// Minimum SPIRE API version this plugin was compiled against.
    pub api_version: PluginVersion,
    /// One-line description.
    pub description: String,
    /// Plugin author(s).
    pub author: String,
    /// Declared capabilities (hooks the plugin implements).
    pub capabilities: Vec<PluginCapability>,
}

// ===========================================================================
// Hook Data Types
// ===========================================================================

/// A single four-momentum vector `(E, px, py, pz)` in GeV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FourMomentum {
    pub e: f64,
    pub px: f64,
    pub py: f64,
    pub pz: f64,
}

/// A kinematic event passed to plugin hooks.
///
/// Contains the list of final-state four-momenta and optional
/// per-particle identifiers (PDG codes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicEvent {
    /// Final-state four-momenta.
    pub momenta: Vec<FourMomentum>,
    /// PDG particle IDs corresponding to each momentum (if known).
    pub pdg_ids: Vec<i32>,
    /// Centre-of-mass energy √s in GeV.
    pub sqrt_s: f64,
    /// Event weight (for weighted MC).
    pub weight: f64,
}

/// Result returned by plugin hook functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookResult {
    /// The event passes the cut.
    Accept,
    /// The event is rejected by the cut.
    Reject,
    /// The plugin returns a floating-point observable value.
    Value(f64),
    /// The plugin encountered an error.
    Error(String),
}

// ===========================================================================
// Current API Version
// ===========================================================================

/// The current SPIRE Plugin API version.
///
/// Plugins compiled against a newer API than the host will be rejected
/// at load time with a clear version-mismatch diagnostic.
pub const CURRENT_API_VERSION: PluginVersion = PluginVersion {
    major: 0,
    minor: 1,
    patch: 0,
};

// ===========================================================================
// Well-Known Export Names
// ===========================================================================

/// Name of the WASM exported function that returns plugin metadata as
/// a JSON string allocated in the guest's linear memory.
pub const EXPORT_METADATA: &str = "spire_plugin_metadata";

/// Name of the WASM exported function for the kinematic-cut hook.
/// Signature: `(ptr: i32, len: i32) -> i32`
/// Input: JSON-encoded [`KinematicEvent`]. Returns a pointer to a
/// JSON-encoded [`HookResult`].
pub const EXPORT_KINEMATIC_CUT: &str = "spire_kinematic_cut";

/// Name of the WASM exported function for custom observable computation.
/// Same calling convention as the kinematic-cut hook.
pub const EXPORT_CUSTOM_OBSERVABLE: &str = "spire_custom_observable";

/// Name of the WASM exported function for custom matrix element squared.
/// Same calling convention as the kinematic-cut hook.
pub const EXPORT_CUSTOM_MATRIX_ELEMENT: &str = "spire_custom_matrix_element";

/// Name of the WASM exported function for analysis post-processing.
/// Same calling convention as the kinematic-cut hook.
pub const EXPORT_ANALYSIS_POST_PROCESS: &str = "spire_analysis_post_process";

/// Name of the WASM exported function the guest uses to allocate memory
/// for the host to write into.  Signature: `(size: i32) -> i32` (returns ptr).
pub const EXPORT_ALLOC: &str = "spire_alloc";

/// Name of the WASM exported function the guest uses to deallocate memory.
/// Signature: `(ptr: i32, size: i32)`.
pub const EXPORT_DEALLOC: &str = "spire_dealloc";

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_roundtrip_json() {
        let meta = PluginMetadata {
            name: "test-plugin".into(),
            version: PluginVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            api_version: CURRENT_API_VERSION,
            description: "A test plugin".into(),
            author: "SPIRE Team".into(),
            capabilities: vec![PluginCapability::KinematicCut],
        };
        let json = serde_json::to_string(&meta).unwrap();
        let parsed: PluginMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-plugin");
        assert_eq!(parsed.capabilities.len(), 1);
    }

    #[test]
    fn hook_result_roundtrip() {
        let cases = vec![
            HookResult::Accept,
            HookResult::Reject,
            HookResult::Value(3.14),
            HookResult::Error("boom".into()),
        ];
        for case in &cases {
            let json = serde_json::to_string(case).unwrap();
            let _parsed: HookResult = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn kinematic_event_roundtrip() {
        let evt = KinematicEvent {
            momenta: vec![
                FourMomentum {
                    e: 100.0,
                    px: 0.0,
                    py: 0.0,
                    pz: 100.0,
                },
                FourMomentum {
                    e: 100.0,
                    px: 0.0,
                    py: 0.0,
                    pz: -100.0,
                },
            ],
            pdg_ids: vec![11, -11],
            sqrt_s: 200.0,
            weight: 1.0,
        };
        let json = serde_json::to_string(&evt).unwrap();
        let parsed: KinematicEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.momenta.len(), 2);
        assert_eq!(parsed.pdg_ids, vec![11, -11]);
    }

    #[test]
    fn version_display() {
        assert_eq!(CURRENT_API_VERSION.to_string(), "0.1.0");
    }
}
