//! # Session — Persistent Notebook Execution Engine
//!
//! Manages stateful notebook sessions where variables, models, and reactions
//! persist across sequential cell executions.  Each session encapsulates a
//! Rhai [`Engine`] and [`Scope`], allowing scripts executed in Cell 1 to
//! define variables that Cell 2 can read and modify.
//!
//! ## Architecture
//!
//! - [`NotebookSession`] — A single session holding the Rhai runtime state,
//!   the active theoretical model, and the active reaction.
//! - [`SessionManager`] — A thread-safe registry of active sessions, keyed
//!   by UUID strings.  Designed to be stored in Tauri's managed state or
//!   used from any multi-threaded context.
//!
//! ## Execution Model
//!
//! Two cell types produce executable payloads:
//!
//! - **Config cells** (TOML): Parsed to build a `TheoreticalModel` that is
//!   stored in the session and made available to subsequent script cells.
//! - **Script cells** (Rhai): Evaluated against the session's persistent
//!   `Scope`.  The script can access a `model` variable (if loaded) and
//!   all previously defined variables.
//!
//! The `print()` function in Rhai is overridden to capture output into a
//! buffer, which is returned to the frontend as the cell's text output.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use rhai::{Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};

use crate::data_loader;
use crate::lagrangian::TheoreticalModel;
use crate::s_matrix::Reaction;
use crate::scripting::SpireScriptEngine;


// ---------------------------------------------------------------------------
// Execution Result
// ---------------------------------------------------------------------------

/// The result of executing a single notebook cell.
///
/// Returned to the frontend for display beneath the cell editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution completed without errors.
    pub success: bool,
    /// Captured stdout from `print()` calls, joined by newlines.
    pub output: String,
    /// Error message, if the execution failed.
    pub error: Option<String>,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: f64,
    /// A structured return value (JSON-serializable) for rich display.
    /// For config cells this may contain model metadata; for script cells
    /// it contains the final expression value (if any).
    pub return_value: Option<serde_json::Value>,
}

impl ExecutionResult {
    /// Create a successful result with captured output.
    fn ok(output: String, duration_ms: f64, return_value: Option<serde_json::Value>) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms,
            return_value,
        }
    }

    /// Create an error result.
    fn err(error: String, output: String, duration_ms: f64) -> Self {
        Self {
            success: false,
            output,
            error: Some(error),
            duration_ms,
            return_value: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Notebook Session
// ---------------------------------------------------------------------------

/// A persistent notebook session.
///
/// Holds the Rhai engine, a mutable scope (retaining variables across cell
/// executions), and optional physics context (model + reaction).
pub struct NotebookSession {
    /// The Rhai engine with all SPIRE physics types registered.
    engine: Engine,
    /// Persistent variable scope — survives across cell executions.
    scope: Scope<'static>,
    /// The active theoretical model (set via Config cells).
    model: Option<TheoreticalModel>,
    /// The active reaction (can be set via script commands).
    reaction: Option<Reaction>,
    /// Monotonically increasing execution counter.
    execution_count: u32,
    /// Shared print output buffer, captured during script evaluation.
    print_buffer: Arc<Mutex<Vec<String>>>,
}

impl NotebookSession {
    /// Create a new session with a fresh Rhai engine and empty scope.
    pub fn new() -> Self {
        let print_buffer: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // Build a full SPIRE-aware Rhai engine using the scripting module,
        // then extract the inner Engine and add session-specific extensions.
        let base = SpireScriptEngine::new();
        let mut engine = Engine::new();

        // Re-register all types that SpireScriptEngine registers.
        // We clone the approach rather than sharing the Arc<Engine> because
        // we need a mutable engine to override `print` and add session functions.
        // Instead, create a new engine from scratch with the same registrations.

        // Register SpacetimeVector, PhaseSpacePoint, Jet, ReconstructedEvent
        // by constructing a fresh SpireScriptEngine and extracting its config.
        // Since SpireScriptEngine wraps an Arc<Engine> that's immutable,
        // we'll build our own engine with the same registrations.
        drop(base);

        // --- SpacetimeVector ---
        use crate::algebra::SpacetimeVector;
        use crate::kinematics::PhaseSpacePoint;

        engine.register_type_with_name::<SpacetimeVector>("SpacetimeVector");
        engine.register_fn("e", |v: &mut SpacetimeVector| -> f64 { v[0] });
        engine.register_fn("px", |v: &mut SpacetimeVector| -> f64 { v[1] });
        engine.register_fn("py", |v: &mut SpacetimeVector| -> f64 { v[2] });
        engine.register_fn("pz", |v: &mut SpacetimeVector| -> f64 { v[3] });
        engine.register_fn("pt", |v: &mut SpacetimeVector| -> f64 {
            (v[1] * v[1] + v[2] * v[2]).sqrt()
        });
        engine.register_fn("m", |v: &mut SpacetimeVector| -> f64 {
            let m_sq = v[0] * v[0] - v[1] * v[1] - v[2] * v[2] - v[3] * v[3];
            m_sq.abs().sqrt()
        });
        engine.register_fn(
            "+",
            |a: &mut SpacetimeVector, b: SpacetimeVector| -> SpacetimeVector { a.clone() + b },
        );
        engine.register_fn(
            "-",
            |a: &mut SpacetimeVector, b: SpacetimeVector| -> SpacetimeVector { a.clone() - b },
        );
        engine.register_fn(
            "*",
            |v: &mut SpacetimeVector, f: f64| -> SpacetimeVector { v.scale(f) },
        );
        engine.register_fn(
            "to_string",
            |v: &mut SpacetimeVector| -> String { format!("{}", v) },
        );
        engine.register_fn("vec4", |e: f64, px: f64, py: f64, pz: f64| {
            SpacetimeVector::new_4d(e, px, py, pz)
        });

        // --- PhaseSpacePoint ---
        engine.register_type_with_name::<PhaseSpacePoint>("PhaseSpacePoint");
        engine.register_get("momenta", |psp: &mut PhaseSpacePoint| -> rhai::Array {
            psp.momenta
                .iter()
                .map(|v| Dynamic::from(v.clone()))
                .collect()
        });
        engine.register_get("weight", |psp: &mut PhaseSpacePoint| -> f64 {
            psp.weight
        });

        // --- Session-specific: math helpers ---
        engine.register_fn("sqrt", |x: f64| -> f64 { x.sqrt() });
        engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
        engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
        engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
        engine.register_fn("exp", |x: f64| -> f64 { x.exp() });
        engine.register_fn("ln", |x: f64| -> f64 { x.ln() });
        engine.register_fn("log10", |x: f64| -> f64 { x.log10() });
        engine.register_fn("pow", |base: f64, exp: f64| -> f64 { base.powf(exp) });
        engine.register_fn("PI", || -> f64 { std::f64::consts::PI });

        // --- Override print to capture output ---
        let buf_clone = Arc::clone(&print_buffer);
        engine.on_print(move |text| {
            if let Ok(mut buf) = buf_clone.lock() {
                buf.push(text.to_string());
            }
        });

        let buf_clone2 = Arc::clone(&print_buffer);
        engine.on_debug(move |text, _source, _pos| {
            if let Ok(mut buf) = buf_clone2.lock() {
                buf.push(format!("[debug] {}", text));
            }
        });

        Self {
            engine,
            scope: Scope::new(),
            model: None,
            reaction: None,
            execution_count: 0,
            print_buffer,
        }
    }

    /// Execute a Rhai script cell within this session's persistent scope.
    ///
    /// Variables defined in the script are retained for subsequent cells.
    /// The `model` variable is automatically injected if a model is loaded.
    pub fn execute_script(&mut self, script: &str) -> ExecutionResult {
        self.execution_count += 1;

        // Clear the print buffer
        if let Ok(mut buf) = self.print_buffer.lock() {
            buf.clear();
        }

        // Inject current model metadata into scope if available
        if let Some(ref model) = self.model {
            self.scope
                .set_or_push("model_name", model.name.clone());
            self.scope
                .set_or_push("n_fields", model.fields.len() as i64);
            self.scope
                .set_or_push("n_vertices", model.vertex_factors.len() as i64);

            // Inject field names as an array
            let field_names: rhai::Array = model
                .fields
                .iter()
                .map(|f| Dynamic::from(f.id.clone()))
                .collect();
            self.scope.set_or_push("field_names", field_names);

            // Inject field masses as a map-like array of [name, mass] pairs
            let field_masses: rhai::Array = model
                .fields
                .iter()
                .map(|f| {
                    let pair: rhai::Array =
                        vec![Dynamic::from(f.id.clone()), Dynamic::from(f.mass)];
                    Dynamic::from(pair)
                })
                .collect();
            self.scope.set_or_push("field_masses", field_masses);
        }

        // Inject reaction metadata if available
        if let Some(ref rxn) = self.reaction {
            self.scope.set_or_push(
                "reaction_initial",
                Dynamic::from(
                    rxn.initial
                        .states
                        .iter()
                        .map(|s| Dynamic::from(s.particle.field.id.clone()))
                        .collect::<rhai::Array>(),
                ),
            );
            self.scope.set_or_push(
                "reaction_final",
                Dynamic::from(
                    rxn.final_state
                        .states
                        .iter()
                        .map(|s| Dynamic::from(s.particle.field.id.clone()))
                        .collect::<rhai::Array>(),
                ),
            );
            self.scope
                .set_or_push("perturbative_order", rxn.perturbative_order as i64);
        }

        let start = Instant::now();
        let result = self.engine.eval_with_scope::<Dynamic>(&mut self.scope, script);
        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        // Collect captured print output
        let output = if let Ok(buf) = self.print_buffer.lock() {
            buf.join("\n")
        } else {
            String::new()
        };

        match result {
            Ok(value) => {
                // Convert the Rhai Dynamic to a JSON value for the frontend
                let return_value = dynamic_to_json(&value);
                ExecutionResult::ok(output, duration_ms, return_value)
            }
            Err(err) => {
                ExecutionResult::err(format!("{}", err), output, duration_ms)
            }
        }
    }

    /// Execute a TOML config cell to load a theoretical model.
    ///
    /// The TOML must contain `[particles]` and `[vertices]` tables, or
    /// the raw particle/vertex TOML content that `data_loader::build_model`
    /// can parse.
    pub fn execute_config(&mut self, toml_content: &str) -> ExecutionResult {
        self.execution_count += 1;

        // Clear print buffer
        if let Ok(mut buf) = self.print_buffer.lock() {
            buf.clear();
        }

        let start = Instant::now();

        // Try to parse the TOML as a combined config with particles + vertices sections
        let parse_result: Result<toml::Value, _> = toml::from_str(toml_content);

        match parse_result {
            Ok(config) => {
                // Extract particles and vertices TOML sections
                let particles_toml = if let Some(particles) = config.get("particles") {
                    // Re-serialize just the particles section
                    let mut wrapper = toml::map::Map::new();
                    if let toml::Value::Table(ref table) = particles {
                        for (k, v) in table {
                            wrapper.insert(k.clone(), v.clone());
                        }
                    }
                    // The data_loader expects the particle table at the top level
                    toml::to_string(&toml::Value::Table(wrapper)).unwrap_or_default()
                } else {
                    // Assume the entire TOML is a particles definition
                    toml_content.to_string()
                };

                let vertices_toml = if let Some(vertices) = config.get("vertices") {
                    let mut wrapper = toml::map::Map::new();
                    if let toml::Value::Table(ref table) = vertices {
                        for (k, v) in table {
                            wrapper.insert(k.clone(), v.clone());
                        }
                    }
                    toml::to_string(&toml::Value::Table(wrapper)).unwrap_or_default()
                } else {
                    String::new()
                };

                let model_name = config
                    .get("model")
                    .and_then(|m| m.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("Notebook Model")
                    .to_string();

                match data_loader::build_model(&particles_toml, &vertices_toml, &model_name) {
                    Ok(model) => {
                        let n_fields = model.fields.len();
                        let n_vertices = model.vertex_factors.len();
                        let name = model.name.clone();
                        self.model = Some(model);

                        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                        let meta = serde_json::json!({
                            "model_name": name,
                            "n_fields": n_fields,
                            "n_vertices": n_vertices,
                        });

                        ExecutionResult::ok(
                            format!(
                                "Model \"{}\" loaded: {} fields, {} vertices",
                                name, n_fields, n_vertices
                            ),
                            duration_ms,
                            Some(meta),
                        )
                    }
                    Err(e) => {
                        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                        ExecutionResult::err(format!("{}", e), String::new(), duration_ms)
                    }
                }
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                ExecutionResult::err(
                    format!("TOML parse error: {}", e),
                    String::new(),
                    duration_ms,
                )
            }
        }
    }

    /// Get the current execution count.
    pub fn execution_count(&self) -> u32 {
        self.execution_count
    }

    /// Check whether a model is currently loaded in this session.
    pub fn has_model(&self) -> bool {
        self.model.is_some()
    }

    /// Clear the session scope and reset all state.
    pub fn reset(&mut self) {
        self.scope.clear();
        self.model = None;
        self.reaction = None;
        self.execution_count = 0;
    }
}

impl Default for NotebookSession {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Session Manager
// ---------------------------------------------------------------------------

/// Thread-safe registry of active notebook sessions.
///
/// Sessions are identified by UUID strings and stored behind `RwLock`
/// for concurrent read access and exclusive write access during execution.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Mutex<NotebookSession>>>,
}

impl SessionManager {
    /// Create a new empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session and return its unique ID.
    pub fn create_session(&self) -> String {
        let id = generate_session_id();
        let session = NotebookSession::new();
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.insert(id.clone(), Mutex::new(session));
        }
        id
    }

    /// Execute a Rhai script in the specified session.
    pub fn execute_script(
        &self,
        session_id: &str,
        script: &str,
    ) -> Result<ExecutionResult, String> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| "Session manager lock poisoned".to_string())?;

        let session_mutex = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut session = session_mutex
            .lock()
            .map_err(|_| "Session lock poisoned".to_string())?;

        Ok(session.execute_script(script))
    }

    /// Execute a TOML config cell in the specified session.
    pub fn execute_config(
        &self,
        session_id: &str,
        toml_content: &str,
    ) -> Result<ExecutionResult, String> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| "Session manager lock poisoned".to_string())?;

        let session_mutex = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut session = session_mutex
            .lock()
            .map_err(|_| "Session lock poisoned".to_string())?;

        Ok(session.execute_config(toml_content))
    }

    /// Reset a session's scope and state.
    pub fn reset_session(&self, session_id: &str) -> Result<(), String> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| "Session manager lock poisoned".to_string())?;

        let session_mutex = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        let mut session = session_mutex
            .lock()
            .map_err(|_| "Session lock poisoned".to_string())?;

        session.reset();
        Ok(())
    }

    /// Destroy a session and free its resources.
    pub fn destroy_session(&self, session_id: &str) -> bool {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(session_id).is_some()
        } else {
            false
        }
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions
            .read()
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a unique session ID.
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Simple hash-like ID: timestamp + counter
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    format!("session-{:x}-{:x}", timestamp, count)
}

/// Convert a Rhai `Dynamic` value to a `serde_json::Value`.
///
/// This enables structured display of script return values in the
/// notebook UI (numbers, strings, arrays, maps).
fn dynamic_to_json(value: &Dynamic) -> Option<serde_json::Value> {
    if value.is_unit() {
        return None;
    }
    if let Ok(v) = value.as_float() {
        return Some(serde_json::json!(v));
    }
    if let Ok(v) = value.as_int() {
        return Some(serde_json::json!(v));
    }
    if let Ok(v) = value.as_bool() {
        return Some(serde_json::json!(v));
    }
    if let Ok(v) = value.clone().into_string() {
        return Some(serde_json::json!(v));
    }
    if value.is_array() {
        if let Ok(arr) = value.clone().into_typed_array::<Dynamic>() {
            let json_arr: Vec<serde_json::Value> = arr
                .iter()
                .filter_map(dynamic_to_json)
                .collect();
            return Some(serde_json::json!(json_arr));
        }
    }
    // Fallback: use Rhai's debug format
    Some(serde_json::json!(format!("{:?}", value)))
}

// ---------------------------------------------------------------------------
// Global Session Manager (lazy static)
// ---------------------------------------------------------------------------

/// Get the global session manager instance.
///
/// Uses `std::sync::OnceLock` for a one-time initialized global.
pub fn global_session_manager() -> &'static SessionManager {
    static INSTANCE: std::sync::OnceLock<SessionManager> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(SessionManager::new)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_create_and_execute_script() {
        let mut session = NotebookSession::new();
        let result = session.execute_script("let x = 42; x");
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::json!(42)));
    }

    #[test]
    fn session_persistent_scope() {
        let mut session = NotebookSession::new();
        // Cell 1: define variable
        let r1 = session.execute_script("let mass = 125.0;");
        assert!(r1.success);
        // Cell 2: use variable from cell 1
        let r2 = session.execute_script("mass * 2.0");
        assert!(r2.success);
        assert_eq!(r2.return_value, Some(serde_json::json!(250.0)));
    }

    #[test]
    fn session_print_capture() {
        let mut session = NotebookSession::new();
        let result = session.execute_script(r#"print("Hello SPIRE"); print("Line 2");"#);
        assert!(result.success);
        assert!(result.output.contains("Hello SPIRE"));
        assert!(result.output.contains("Line 2"));
    }

    #[test]
    fn session_script_error() {
        let mut session = NotebookSession::new();
        let result = session.execute_script("let x = ;");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn session_math_functions() {
        let mut session = NotebookSession::new();
        let result = session.execute_script("sqrt(144.0)");
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::json!(12.0)));
    }

    #[test]
    fn session_vec4_construction() {
        let mut session = NotebookSession::new();
        let result = session.execute_script(r#"
            let p = vec4(100.0, 30.0, 40.0, 0.0);
            p.e()
        "#);
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::json!(100.0)));
    }

    #[test]
    fn session_manager_lifecycle() {
        let manager = SessionManager::new();
        let id = manager.create_session();
        assert_eq!(manager.session_count(), 1);

        let result = manager.execute_script(&id, "1 + 2").unwrap();
        assert!(result.success);
        assert_eq!(result.return_value, Some(serde_json::json!(3)));

        assert!(manager.destroy_session(&id));
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn session_manager_unknown_session() {
        let manager = SessionManager::new();
        let result = manager.execute_script("nonexistent", "1");
        assert!(result.is_err());
    }

    #[test]
    fn session_reset_clears_scope() {
        let mut session = NotebookSession::new();
        session.execute_script("let x = 10;");
        session.reset();
        let result = session.execute_script("x");
        assert!(!result.success); // x should not be defined after reset
    }

    #[test]
    fn execution_count_increments() {
        let mut session = NotebookSession::new();
        assert_eq!(session.execution_count(), 0);
        session.execute_script("1");
        assert_eq!(session.execution_count(), 1);
        session.execute_script("2");
        assert_eq!(session.execution_count(), 2);
    }
}
