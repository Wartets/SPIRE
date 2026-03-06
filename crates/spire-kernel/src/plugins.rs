//! # SPIRE Plugin Host Engine
//!
//! Sandboxed WASM runtime for loading and executing third-party plugins.
//! Uses [`wasmtime`] as the embedding engine with strict fuel-metered
//! execution, memory limits, and no WASI (filesystem/network) access
//! by default.
//!
//! ## Architecture
//!
//! ```text
//!  ┌─────────────┐     JSON      ┌─────────────────┐
//!  │  SPIRE Host  │ ←──────────→ │  WASM Guest      │
//!  │  (this mod)  │  serde_json  │  (plugin .wasm)  │
//!  └─────────────┘               └─────────────────┘
//! ```
//!
//! All data exchange across the WASM boundary uses JSON serialization
//! via guest-allocated linear memory.  The host writes JSON bytes into
//! guest memory (via `spire_alloc`), calls the hook function, then
//! reads the JSON result back out.

use serde::{Deserialize, Serialize};
use spire_plugin_api::{
    HookResult, KinematicEvent, PluginCapability, PluginMetadata, CURRENT_API_VERSION,
    EXPORT_ALLOC, EXPORT_CUSTOM_MATRIX_ELEMENT, EXPORT_CUSTOM_OBSERVABLE, EXPORT_KINEMATIC_CUT,
    EXPORT_METADATA,
};
use std::collections::HashMap;
use wasmtime::*;

use crate::SpireError;

// ===========================================================================
// Plugin Instance
// ===========================================================================

/// A loaded and validated plugin instance.
///
/// Holds the compiled WASM module, its instantiated state, and the
/// extracted metadata.  Each plugin runs in its own `Store` with
/// independent fuel metering.
pub struct PluginInstance {
    /// Metadata extracted at load time.
    pub metadata: PluginMetadata,
    /// Compiled WASM module (can be cheaply cloned / re-instantiated).
    module: Module,
    /// Wasmtime store holding instance state + fuel.
    store: Store<()>,
    /// Live instance of the module.
    instance: Instance,
}

/// Summary of a loaded plugin (safe to send across IPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}

impl From<&PluginInstance> for PluginSummary {
    fn from(p: &PluginInstance) -> Self {
        Self {
            name: p.metadata.name.clone(),
            version: p.metadata.version.to_string(),
            description: p.metadata.description.clone(),
            author: p.metadata.author.clone(),
            capabilities: p
                .metadata
                .capabilities
                .iter()
                .map(|c| format!("{:?}", c))
                .collect(),
            enabled: true,
        }
    }
}

// ===========================================================================
// Plugin Host
// ===========================================================================

/// Maximum fuel units granted per plugin call (prevents infinite loops).
const DEFAULT_FUEL: u64 = 500_000_000;

/// Maximum linear memory per plugin instance (64 MiB).
const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;

/// The WASM plugin host engine.
///
/// Manages the wasmtime [`Engine`], loaded plugins, and hook dispatch.
pub struct PluginHost {
    engine: Engine,
    plugins: HashMap<String, PluginInstance>,
}

impl PluginHost {
    /// Create a new plugin host with default configuration.
    pub fn new() -> Result<Self, SpireError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        // Cranelift is the default compiler; no extra config needed.
        let engine = Engine::new(&config).map_err(|e| {
            SpireError::InternalError(format!("Failed to create WASM engine: {}", e))
        })?;
        Ok(Self {
            engine,
            plugins: HashMap::new(),
        })
    }

    /// Load a plugin from raw `.wasm` bytes.
    ///
    /// The module is compiled, instantiated, and its `spire_plugin_metadata`
    /// export is called to extract metadata.  ABI version compatibility is
    /// verified before the plugin is accepted.
    pub fn load_plugin_from_bytes(&mut self, bytes: &[u8]) -> Result<PluginMetadata, SpireError> {
        // Compile the module
        let module = Module::new(&self.engine, bytes).map_err(|e| {
            SpireError::InternalError(format!("Failed to compile WASM module: {}", e))
        })?;

        // Validate memory limits
        for import in module.imports() {
            if let ExternType::Memory(mem_ty) = import.ty() {
                let max_pages = mem_ty.maximum().unwrap_or(u64::MAX);
                if max_pages as usize * 65536 > MAX_MEMORY_BYTES {
                    return Err(SpireError::InternalError(format!(
                        "Plugin requests too much memory: {} pages (max {} bytes)",
                        max_pages, MAX_MEMORY_BYTES
                    )));
                }
            }
        }

        // Create a store with fuel metering
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(DEFAULT_FUEL)
            .map_err(|e| SpireError::InternalError(format!("Failed to set fuel: {}", e)))?;

        // Instantiate with an empty linker (no WASI = full sandbox)
        let linker = Linker::new(&self.engine);
        let instance = linker.instantiate(&mut store, &module).map_err(|e| {
            SpireError::InternalError(format!("Failed to instantiate WASM module: {}", e))
        })?;

        // Extract metadata
        let metadata = Self::extract_metadata(&mut store, &instance)?;

        // Version check: plugin's api_version must be compatible
        if metadata.api_version.major != CURRENT_API_VERSION.major {
            return Err(SpireError::InternalError(format!(
                "Plugin '{}' requires API v{} but host provides v{} (major version mismatch)",
                metadata.name, metadata.api_version, CURRENT_API_VERSION,
            )));
        }
        if metadata.api_version.minor > CURRENT_API_VERSION.minor {
            return Err(SpireError::InternalError(format!(
                "Plugin '{}' requires API v{} but host only provides v{} (minor version too new)",
                metadata.name, metadata.api_version, CURRENT_API_VERSION,
            )));
        }

        let name = metadata.name.clone();
        let result = metadata.clone();

        self.plugins.insert(
            name,
            PluginInstance {
                metadata,
                module,
                store,
                instance,
            },
        );

        Ok(result)
    }

    /// Extract plugin metadata by calling the guest's `spire_plugin_metadata` export.
    fn extract_metadata(
        store: &mut Store<()>,
        instance: &Instance,
    ) -> Result<PluginMetadata, SpireError> {
        // The metadata function returns a pointer to a JSON string in guest memory.
        // Convention: returns (ptr << 32) | len packed into an i64.
        let metadata_fn = instance
            .get_typed_func::<(), i64>(store, EXPORT_METADATA)
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Plugin missing '{}' export: {}",
                    EXPORT_METADATA, e
                ))
            })?;

        let packed = metadata_fn.call(store, ()).map_err(|e| {
            SpireError::InternalError(format!("Failed to call {}: {}", EXPORT_METADATA, e))
        })?;

        let ptr = (packed >> 32) as u32 as usize;
        let len = (packed & 0xFFFF_FFFF) as u32 as usize;

        let memory = instance
            .get_memory(store, "memory")
            .ok_or_else(|| SpireError::InternalError("Plugin has no 'memory' export".into()))?;

        let data = memory.data(store);
        if ptr + len > data.len() {
            return Err(SpireError::InternalError(
                "Plugin metadata pointer out of bounds".into(),
            ));
        }

        let json_bytes = &data[ptr..ptr + len];
        let json_str = std::str::from_utf8(json_bytes).map_err(|e| {
            SpireError::InternalError(format!("Plugin metadata is not valid UTF-8: {}", e))
        })?;

        serde_json::from_str(json_str).map_err(|e| {
            SpireError::InternalError(format!("Failed to parse plugin metadata JSON: {}", e))
        })
    }

    /// List all loaded plugins.
    pub fn list_plugins(&self) -> Vec<PluginSummary> {
        self.plugins.values().map(PluginSummary::from).collect()
    }

    /// Unload a plugin by name.
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), SpireError> {
        self.plugins.remove(name).ok_or_else(|| {
            SpireError::InternalError(format!("Plugin '{}' not found", name))
        })?;
        Ok(())
    }

    /// Check if a specific plugin has a given capability.
    pub fn has_capability(&self, name: &str, cap: &PluginCapability) -> bool {
        self.plugins
            .get(name)
            .is_some_and(|p| p.metadata.capabilities.contains(cap))
    }

    /// Dispatch the kinematic-cut hook to all plugins that declare the
    /// [`PluginCapability::KinematicCut`] capability.
    ///
    /// Returns `true` if the event is accepted by **all** plugins
    /// (logical AND). If any plugin rejects, returns `false`.
    pub fn dispatch_kinematic_cut(&mut self, event: &KinematicEvent) -> Result<bool, SpireError> {
        let json = serde_json::to_string(event).map_err(|e| {
            SpireError::InternalError(format!("Failed to serialize event: {}", e))
        })?;

        let names: Vec<String> = self
            .plugins
            .iter()
            .filter(|(_, p)| {
                p.metadata
                    .capabilities
                    .contains(&PluginCapability::KinematicCut)
            })
            .map(|(n, _)| n.clone())
            .collect();

        for name in &names {
            let result = self.call_hook(name, EXPORT_KINEMATIC_CUT, &json)?;
            match result {
                HookResult::Reject => return Ok(false),
                HookResult::Accept => {}
                HookResult::Error(msg) => {
                    return Err(SpireError::InternalError(format!(
                        "Plugin '{}' kinematic cut error: {}",
                        name, msg
                    )));
                }
                _ => {}
            }
        }

        Ok(true)
    }

    /// Dispatch the custom-observable hook to a specific plugin.
    ///
    /// Returns the computed observable value.
    pub fn dispatch_custom_observable(
        &mut self,
        plugin_name: &str,
        event: &KinematicEvent,
    ) -> Result<f64, SpireError> {
        let json = serde_json::to_string(event).map_err(|e| {
            SpireError::InternalError(format!("Failed to serialize event: {}", e))
        })?;

        let result = self.call_hook(plugin_name, EXPORT_CUSTOM_OBSERVABLE, &json)?;
        match result {
            HookResult::Value(v) => Ok(v),
            HookResult::Error(msg) => Err(SpireError::InternalError(format!(
                "Plugin '{}' observable error: {}",
                plugin_name, msg
            ))),
            other => Err(SpireError::InternalError(format!(
                "Plugin '{}' returned unexpected result for observable: {:?}",
                plugin_name, other
            ))),
        }
    }

    /// Dispatch the custom matrix element hook to a specific plugin.
    pub fn dispatch_custom_matrix_element(
        &mut self,
        plugin_name: &str,
        event: &KinematicEvent,
    ) -> Result<f64, SpireError> {
        let json = serde_json::to_string(event).map_err(|e| {
            SpireError::InternalError(format!("Failed to serialize event: {}", e))
        })?;

        let result = self.call_hook(plugin_name, EXPORT_CUSTOM_MATRIX_ELEMENT, &json)?;
        match result {
            HookResult::Value(v) => Ok(v),
            HookResult::Error(msg) => Err(SpireError::InternalError(format!(
                "Plugin '{}' matrix element error: {}",
                plugin_name, msg
            ))),
            other => Err(SpireError::InternalError(format!(
                "Plugin '{}' returned unexpected result for matrix element: {:?}",
                plugin_name, other
            ))),
        }
    }

    // -----------------------------------------------------------------------
    // Internal: Generic hook calling
    // -----------------------------------------------------------------------

    /// Call a named hook function in a specific plugin.
    ///
    /// The calling convention is:
    /// 1. Serialize the input as JSON bytes.
    /// 2. Allocate guest memory via `spire_alloc(len)`.
    /// 3. Copy JSON bytes into guest memory at the returned pointer.
    /// 4. Call `hook_name(ptr, len)` which returns `(result_ptr << 32) | result_len`.
    /// 5. Read result JSON from guest memory and deserialize.
    fn call_hook(
        &mut self,
        plugin_name: &str,
        hook_name: &str,
        json_input: &str,
    ) -> Result<HookResult, SpireError> {
        let plugin = self.plugins.get_mut(plugin_name).ok_or_else(|| {
            SpireError::InternalError(format!("Plugin '{}' not found", plugin_name))
        })?;

        // Refuel the store for this call
        let remaining = plugin.store.get_fuel().unwrap_or(0);
        if remaining < DEFAULT_FUEL / 2 {
            plugin
                .store
                .set_fuel(DEFAULT_FUEL)
                .map_err(|e| SpireError::InternalError(format!("Failed to refuel: {}", e)))?;
        }

        let input_bytes = json_input.as_bytes();

        // Allocate guest memory for input
        let alloc_fn = plugin
            .instance
            .get_typed_func::<i32, i32>(&mut plugin.store, EXPORT_ALLOC)
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Plugin '{}' missing '{}' export: {}",
                    plugin_name, EXPORT_ALLOC, e
                ))
            })?;

        let input_ptr = alloc_fn
            .call(&mut plugin.store, input_bytes.len() as i32)
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Plugin '{}' alloc failed: {}",
                    plugin_name, e
                ))
            })?;

        // Write input JSON into guest memory
        let memory = plugin
            .instance
            .get_memory(&mut plugin.store, "memory")
            .ok_or_else(|| {
                SpireError::InternalError(format!(
                    "Plugin '{}' has no 'memory' export",
                    plugin_name
                ))
            })?;

        memory
            .data_mut(&mut plugin.store)
            .get_mut(input_ptr as usize..input_ptr as usize + input_bytes.len())
            .ok_or_else(|| {
                SpireError::InternalError("Input pointer out of bounds in guest memory".into())
            })?
            .copy_from_slice(input_bytes);

        // Call the hook function
        let hook_fn = plugin
            .instance
            .get_typed_func::<(i32, i32), i64>(&mut plugin.store, hook_name)
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Plugin '{}' missing hook '{}': {}",
                    plugin_name, hook_name, e
                ))
            })?;

        let packed_result = hook_fn
            .call(
                &mut plugin.store,
                (input_ptr, input_bytes.len() as i32),
            )
            .map_err(|e| {
                SpireError::InternalError(format!(
                    "Plugin '{}' hook '{}' trapped: {}",
                    plugin_name, hook_name, e
                ))
            })?;

        // Unpack result pointer and length
        let result_ptr = (packed_result >> 32) as u32 as usize;
        let result_len = (packed_result & 0xFFFF_FFFF) as u32 as usize;

        let mem_data = memory.data(&plugin.store);
        if result_ptr + result_len > mem_data.len() {
            return Err(SpireError::InternalError(
                "Plugin hook result pointer out of bounds".into(),
            ));
        }

        let result_bytes = &mem_data[result_ptr..result_ptr + result_len];
        let result_str = std::str::from_utf8(result_bytes).map_err(|e| {
            SpireError::InternalError(format!("Hook result is not valid UTF-8: {}", e))
        })?;

        serde_json::from_str(result_str).map_err(|e| {
            SpireError::InternalError(format!("Failed to parse hook result JSON: {}", e))
        })
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_plugin_host() {
        let host = PluginHost::new().expect("Failed to create plugin host");
        assert!(host.plugins.is_empty());
        assert_eq!(host.list_plugins().len(), 0);
    }

    #[test]
    fn load_invalid_wasm_fails() {
        let mut host = PluginHost::new().unwrap();
        let result = host.load_plugin_from_bytes(b"not a wasm module");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Failed to compile"),
            "Expected compile error, got: {}",
            err
        );
    }

    #[test]
    fn unload_missing_plugin_fails() {
        let mut host = PluginHost::new().unwrap();
        let result = host.unload_plugin("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn plugin_summary_from_metadata() {
        let summary = PluginSummary {
            name: "test".into(),
            version: "1.0.0".into(),
            description: "A test".into(),
            author: "Tester".into(),
            capabilities: vec!["KinematicCut".into()],
            enabled: true,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: PluginSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
    }
}
