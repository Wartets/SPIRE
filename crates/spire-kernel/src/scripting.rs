//! # Scripting — Embedded Rhai Scripting Engine
//!
//! This module embeds the [Rhai](https://rhai.rs) scripting engine into the
//! SPIRE kernel, enabling researchers to define custom observables, kinematic
//! cuts, and momentum-dependent form factors at runtime — without recompiling
//! the core engine.
//!
//! ## Performance
//!
//! All user scripts are **pre-compiled to an Abstract Syntax Tree (AST)** before
//! any Monte Carlo integration loop begins. The compiled AST is then evaluated
//! millions of times with near-native overhead, avoiding any string parsing
//! inside hot loops.
//!
//! ## Type Registration
//!
//! The engine registers core physics types so that Rhai scripts can manipulate
//! them natively:
//!
//! - [`SpacetimeVector`] — with methods `.e()`, `.px()`, `.py()`, `.pz()`,
//!   `.pt()`, `.eta()`, `.phi()`, `.m()`, `.rapidity()`.
//! - [`PhaseSpacePoint`] — with property `.momenta` (array of vectors) and
//!   `.weight` (phase-space weight).
//!
//! ## Examples
//!
//! ```no_run
//! use spire_kernel::scripting::SpireScriptEngine;
//! use spire_kernel::kinematics::PhaseSpacePoint;
//! use spire_kernel::algebra::SpacetimeVector;
//!
//! let engine = SpireScriptEngine::new();
//!
//! // Compile an observable: invariant mass of particles 0 and 1
//! let obs = engine.compile_observable(r#"
//!     let p1 = event.momenta[0];
//!     let p2 = event.momenta[1];
//!     let total = p1 + p2;
//!     total.m()
//! "#).unwrap();
//!
//! // Compile a kinematic cut: pT > 50 GeV
//! let cut = engine.compile_cut(r#"
//!     let p = event.momenta[0];
//!     p.pt() > 50.0
//! "#).unwrap();
//! ```

use std::sync::Arc;

use rhai::{Array, Dynamic, Engine, AST};

use crate::algebra::SpacetimeVector;
use crate::kinematics::PhaseSpacePoint;
use crate::reco::clustering::Jet;
use crate::reco::detector::ReconstructedEvent;
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// SpacetimeVector — Rhai helper functions
// ---------------------------------------------------------------------------

/// Energy component $v^0$.
fn sv_e(v: &mut SpacetimeVector) -> f64 {
    v[0]
}

/// Spatial $x$-component $v^1$.
fn sv_px(v: &mut SpacetimeVector) -> f64 {
    v[1]
}

/// Spatial $y$-component $v^2$.
fn sv_py(v: &mut SpacetimeVector) -> f64 {
    v[2]
}

/// Spatial $z$-component $v^3$.
fn sv_pz(v: &mut SpacetimeVector) -> f64 {
    v[3]
}

/// Transverse momentum $p_T = \sqrt{p_x^2 + p_y^2}$.
fn sv_pt(v: &mut SpacetimeVector) -> f64 {
    let px = v[1];
    let py = v[2];
    (px * px + py * py).sqrt()
}

/// Pseudo-rapidity $\eta = \mathrm{atanh}(p_z / |\vec{p}|)$.
///
/// Returns ±∞ for forward/backward particles, 0 for transverse.
fn sv_eta(v: &mut SpacetimeVector) -> f64 {
    let px = v[1];
    let py = v[2];
    let pz = v[3];
    let p_mag = (px * px + py * py + pz * pz).sqrt();
    if p_mag < 1e-300 {
        return 0.0;
    }
    let cos_theta = pz / p_mag;
    // eta = -ln(tan(theta/2)) = atanh(cos_theta)
    cos_theta.atanh()
}

/// Azimuthal angle $\phi = \mathrm{atan2}(p_y, p_x)$.
fn sv_phi(v: &mut SpacetimeVector) -> f64 {
    v[2].atan2(v[1])
}

/// Invariant mass $m = \sqrt{|E^2 - |\vec{p}|^2|}$.
///
/// The absolute value inside the square root handles slight numerical
/// violations of the on-shell condition that arise from Monte Carlo sampling.
fn sv_m(v: &mut SpacetimeVector) -> f64 {
    let e = v[0];
    let px = v[1];
    let py = v[2];
    let pz = v[3];
    let m_sq = e * e - px * px - py * py - pz * pz;
    m_sq.abs().sqrt()
}

/// Rapidity $y = \frac{1}{2} \ln\frac{E + p_z}{E - p_z}$.
fn sv_rapidity(v: &mut SpacetimeVector) -> f64 {
    let e = v[0];
    let pz = v[3];
    let plus = e + pz;
    let minus = e - pz;
    if minus.abs() < 1e-300 || plus.abs() < 1e-300 {
        return if pz >= 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
    }
    0.5 * (plus / minus).abs().ln()
}

/// SpacetimeVector addition for Rhai operator overloading.
fn sv_add(a: &mut SpacetimeVector, b: SpacetimeVector) -> SpacetimeVector {
    a.clone() + b
}

/// SpacetimeVector subtraction for Rhai operator overloading.
fn sv_sub(a: &mut SpacetimeVector, b: SpacetimeVector) -> SpacetimeVector {
    a.clone() - b
}

/// Scale a SpacetimeVector by a scalar (v * f).
fn sv_mul_scalar(v: &mut SpacetimeVector, factor: f64) -> SpacetimeVector {
    v.scale(factor)
}

/// Display a SpacetimeVector as a string.
fn sv_to_string(v: &mut SpacetimeVector) -> String {
    format!("{}", v)
}

// ---------------------------------------------------------------------------
// PhaseSpacePoint — Rhai helper functions
// ---------------------------------------------------------------------------

/// Get the momenta array from a PhaseSpacePoint.
fn psp_get_momenta(psp: &mut PhaseSpacePoint) -> Array {
    psp.momenta
        .iter()
        .map(|v| Dynamic::from(v.clone()))
        .collect()
}

/// Get the phase-space weight.
fn psp_get_weight(psp: &mut PhaseSpacePoint) -> f64 {
    psp.weight
}

// ---------------------------------------------------------------------------
// Jet — Rhai helper functions
// ---------------------------------------------------------------------------

/// Jet transverse momentum $p_T$.
fn jet_pt(j: &mut Jet) -> f64 {
    j.pt()
}

/// Jet pseudorapidity $\eta$.
fn jet_eta(j: &mut Jet) -> f64 {
    j.eta()
}

/// Jet azimuthal angle $\phi$.
fn jet_phi(j: &mut Jet) -> f64 {
    j.phi()
}

/// Jet energy.
fn jet_e(j: &mut Jet) -> f64 {
    j.energy()
}

/// Jet invariant mass.
fn jet_m(j: &mut Jet) -> f64 {
    j.mass()
}

/// Jet 4-momentum as a SpacetimeVector.
fn jet_momentum(j: &mut Jet) -> SpacetimeVector {
    j.momentum.clone()
}

/// Number of constituents in the jet.
fn jet_n_constituents(j: &mut Jet) -> i64 {
    j.n_constituents() as i64
}

/// Jet $p_x$.
fn jet_px(j: &mut Jet) -> f64 {
    j.momentum[1]
}

/// Jet $p_y$.
fn jet_py(j: &mut Jet) -> f64 {
    j.momentum[2]
}

/// Jet $p_z$.
fn jet_pz(j: &mut Jet) -> f64 {
    j.momentum[3]
}

// ---------------------------------------------------------------------------
// ReconstructedEvent — Rhai helper functions
// ---------------------------------------------------------------------------

/// Get the jets array from a ReconstructedEvent.
fn reco_get_jets(reco: &mut ReconstructedEvent) -> Array {
    reco.jets.iter().map(|j| Dynamic::from(j.clone())).collect()
}

/// Get the electrons array from a ReconstructedEvent.
fn reco_get_electrons(reco: &mut ReconstructedEvent) -> Array {
    reco.electrons.iter().map(|v| Dynamic::from(v.clone())).collect()
}

/// Get the muons array from a ReconstructedEvent.
fn reco_get_muons(reco: &mut ReconstructedEvent) -> Array {
    reco.muons.iter().map(|v| Dynamic::from(v.clone())).collect()
}

/// Get the photons array from a ReconstructedEvent.
fn reco_get_photons(reco: &mut ReconstructedEvent) -> Array {
    reco.photons.iter().map(|v| Dynamic::from(v.clone())).collect()
}

/// Get the MET 4-vector from a ReconstructedEvent.
fn reco_get_met(reco: &mut ReconstructedEvent) -> SpacetimeVector {
    reco.met.clone()
}

/// Number of jets.
fn reco_n_jets(reco: &mut ReconstructedEvent) -> i64 {
    reco.n_jets() as i64
}

/// Number of leptons (electrons + muons).
fn reco_n_leptons(reco: &mut ReconstructedEvent) -> i64 {
    reco.n_leptons() as i64
}

/// MET magnitude $E_T^{\text{miss}}$.
fn reco_met_pt(reco: &mut ReconstructedEvent) -> f64 {
    reco.met_pt()
}

// ---------------------------------------------------------------------------
// Script Engine
// ---------------------------------------------------------------------------

/// The SPIRE scripting engine — a pre-configured Rhai runtime with all
/// physics types registered.
///
/// Scripts are compiled to an AST once, then evaluated repeatedly in the
/// Monte Carlo integration loop with zero re-parsing overhead.
pub struct SpireScriptEngine {
    /// The underlying Rhai engine with all type registrations.
    engine: Arc<Engine>,
}

impl SpireScriptEngine {
    /// Create a new scripting engine with all SPIRE physics types registered.
    ///
    /// This performs one-time setup: type registration, method binding,
    /// and operator overloading. The resulting engine can then compile
    /// arbitrarily many scripts.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // --- Register SpacetimeVector ---
        engine.register_type_with_name::<SpacetimeVector>("SpacetimeVector");
        engine.register_fn("e", sv_e);
        engine.register_fn("px", sv_px);
        engine.register_fn("py", sv_py);
        engine.register_fn("pz", sv_pz);
        engine.register_fn("pt", sv_pt);
        engine.register_fn("eta", sv_eta);
        engine.register_fn("phi", sv_phi);
        engine.register_fn("m", sv_m);
        engine.register_fn("rapidity", sv_rapidity);
        engine.register_fn("+", sv_add);
        engine.register_fn("-", sv_sub);
        engine.register_fn("*", sv_mul_scalar);
        engine.register_fn("to_string", sv_to_string);

        // Constructor: vec4(e, px, py, pz) for script-side vector creation.
        engine.register_fn("vec4", |e: f64, px: f64, py: f64, pz: f64| {
            SpacetimeVector::new_4d(e, px, py, pz)
        });

        // --- Register PhaseSpacePoint ---
        engine.register_type_with_name::<PhaseSpacePoint>("PhaseSpacePoint");
        engine.register_get("momenta", psp_get_momenta);
        engine.register_get("weight", psp_get_weight);

        // --- Register Jet ---
        engine.register_type_with_name::<Jet>("Jet");
        engine.register_fn("pt", jet_pt);
        engine.register_fn("eta", jet_eta);
        engine.register_fn("phi", jet_phi);
        engine.register_fn("e", jet_e);
        engine.register_fn("m", jet_m);
        engine.register_fn("px", jet_px);
        engine.register_fn("py", jet_py);
        engine.register_fn("pz", jet_pz);
        engine.register_get("momentum", jet_momentum);
        engine.register_fn("n_constituents", jet_n_constituents);

        // --- Register ReconstructedEvent ---
        engine.register_type_with_name::<ReconstructedEvent>("ReconstructedEvent");
        engine.register_get("jets", reco_get_jets);
        engine.register_get("electrons", reco_get_electrons);
        engine.register_get("muons", reco_get_muons);
        engine.register_get("photons", reco_get_photons);
        engine.register_get("met", reco_get_met);
        engine.register_fn("n_jets", reco_n_jets);
        engine.register_fn("n_leptons", reco_n_leptons);
        engine.register_fn("met_pt", reco_met_pt);

        Self {
            engine: Arc::new(engine),
        }
    }

    /// Get a reference to the underlying Rhai engine.
    pub fn engine(&self) -> &Arc<Engine> {
        &self.engine
    }

    /// Compile a user script into a reusable AST.
    ///
    /// This is the raw compilation step. Prefer [`SpireScriptEngine::compile_observable`] or
    /// [`SpireScriptEngine::compile_cut`] for type-safe wrappers.
    ///
    /// # Errors
    /// Returns a `SpireError::InternalError` if the script has syntax errors.
    pub fn compile(&self, script: &str) -> SpireResult<AST> {
        self.engine
            .compile(script)
            .map_err(|e| SpireError::InternalError(format!("Script compilation error: {}", e)))
    }

    /// Compile a script intended to be evaluated as an observable.
    ///
    /// The script receives a `PhaseSpacePoint` bound to the variable `event`
    /// and must return a numeric (`f64`) value.
    ///
    /// # Performance
    ///
    /// The AST is compiled once; the returned `RhaiObservable` can be
    /// evaluated millions of times with no re-parsing.
    pub fn compile_observable(&self, script: &str) -> SpireResult<RhaiObservable> {
        let ast = self.compile(script)?;
        Ok(RhaiObservable {
            engine: Arc::clone(&self.engine),
            ast,
            name: "custom_observable".into(),
        })
    }

    /// Compile a script intended to be evaluated as a kinematic cut.
    ///
    /// The script receives a `PhaseSpacePoint` bound to the variable `event`
    /// and must return a boolean. Events failing the cut are discarded.
    pub fn compile_cut(&self, script: &str) -> SpireResult<RhaiCut> {
        let ast = self.compile(script)?;
        Ok(RhaiCut {
            engine: Arc::clone(&self.engine),
            ast,
            name: "custom_cut".into(),
        })
    }

    /// Compile a script for a momentum-dependent form factor.
    ///
    /// The script receives the squared momentum transfer as the variable `q2`
    /// and must return a dimensionless suppression factor $F(Q^2) \in [0, 1]$.
    pub fn compile_form_factor(&self, script: &str) -> SpireResult<CompiledFormFactor> {
        let ast = self.compile(script)?;
        Ok(CompiledFormFactor {
            engine: Arc::clone(&self.engine),
            ast,
            source: script.to_string(),
        })
    }

    /// Compile a script intended to be evaluated as a reconstructed-level observable.
    ///
    /// The script receives a `ReconstructedEvent` bound to the variable `reco`
    /// and a `PhaseSpacePoint` bound to the variable `event`, and must return
    /// a numeric (`f64`) value.
    ///
    /// Allows scripts like `reco.jets[0].pt()` or `reco.met.pt()`.
    pub fn compile_reco_observable(&self, script: &str) -> SpireResult<RhaiRecoObservable> {
        let ast = self.compile(script)?;
        Ok(RhaiRecoObservable {
            engine: Arc::clone(&self.engine),
            ast,
            name: "reco_observable".into(),
        })
    }

    /// Compile a script for a reconstructed-level cut.
    ///
    /// The script receives both `reco` (ReconstructedEvent) and `event`
    /// (PhaseSpacePoint) and must return a boolean.
    pub fn compile_reco_cut(&self, script: &str) -> SpireResult<RhaiRecoCut> {
        let ast = self.compile(script)?;
        Ok(RhaiRecoCut {
            engine: Arc::clone(&self.engine),
            ast,
            name: "reco_cut".into(),
        })
    }

    /// Validate a script string without creating a typed wrapper.
    ///
    /// Useful for frontend "lint as you type" functionality.
    pub fn validate_script(&self, script: &str) -> Result<(), String> {
        self.engine
            .compile(script)
            .map(|_| ())
            .map_err(|e| format!("{}", e))
    }
}

impl Default for SpireScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Observable Trait & Rhai Implementation
// ---------------------------------------------------------------------------

/// A computed physical quantity evaluated on each Monte Carlo event.
///
/// Observables map a phase-space configuration to a scalar value.
/// Examples: invariant mass, transverse momentum, rapidity.
///
/// The trait is object-safe, enabling heterogeneous collections of
/// both native Rust observables and dynamically defined Rhai scripts.
pub trait Observable: Send + Sync {
    /// Evaluate this observable on the given phase-space point.
    fn evaluate(&self, event: &PhaseSpacePoint) -> f64;

    /// A human-readable name for this observable.
    fn name(&self) -> &str;
}

/// A custom observable defined by a Rhai script.
///
/// The script is pre-compiled to an AST at construction time. Each call
/// to `evaluate` binds the event to the `event` variable and runs the
/// AST, returning the final expression value.
pub struct RhaiObservable {
    engine: Arc<Engine>,
    ast: AST,
    name: String,
}

impl RhaiObservable {
    /// Set a descriptive name for this observable.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Observable for RhaiObservable {
    fn evaluate(&self, event: &PhaseSpacePoint) -> f64 {
        let mut scope = rhai::Scope::new();
        scope.push("event", event.clone());
        self.engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &self.ast)
            .and_then(|v| v.as_float().map_err(|_t| {
                rhai::EvalAltResult::ErrorMismatchOutputType(
                    "f64".into(),
                    "non-numeric".into(),
                    rhai::Position::NONE,
                ).into()
            }))
            .unwrap_or(f64::NAN)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Safety: Engine and AST are Send+Sync when the `sync` feature is enabled.
unsafe impl Send for RhaiObservable {}
unsafe impl Sync for RhaiObservable {}

// ---------------------------------------------------------------------------
// Kinematic Cut Trait & Rhai Implementation
// ---------------------------------------------------------------------------

/// A boolean predicate applied to each Monte Carlo event.
///
/// Events failing the cut are discarded from the integration (their
/// contribution is set to zero). This enables detector-level acceptance
/// cuts such as $p_T > 50\,\text{GeV}$ or $|\eta| < 2.5$.
pub trait KinematicCut: Send + Sync {
    /// Returns `true` if the event passes this cut.
    fn is_passed(&self, event: &PhaseSpacePoint) -> bool;

    /// A human-readable name for this cut.
    fn name(&self) -> &str;
}

/// A kinematic cut defined by a Rhai script.
///
/// The script must evaluate to a boolean. Events for which the script
/// returns `false` are discarded.
pub struct RhaiCut {
    engine: Arc<Engine>,
    ast: AST,
    name: String,
}

impl RhaiCut {
    /// Set a descriptive name for this cut.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl KinematicCut for RhaiCut {
    fn is_passed(&self, event: &PhaseSpacePoint) -> bool {
        let mut scope = rhai::Scope::new();
        scope.push("event", event.clone());
        self.engine
            .eval_ast_with_scope::<bool>(&mut scope, &self.ast)
            .unwrap_or(false)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

unsafe impl Send for RhaiCut {}
unsafe impl Sync for RhaiCut {}

// ---------------------------------------------------------------------------
// Compiled Form Factor
// ---------------------------------------------------------------------------

/// A momentum-dependent form factor defined by a Rhai script.
///
/// The script receives $Q^2$ as the variable `q2` and must return a
/// dimensionless suppression factor $F(Q^2)$.
///
/// This is used by `FormFactor::Scripted` to allow user-defined
/// momentum-dependent vertex corrections without recompilation.
pub struct CompiledFormFactor {
    engine: Arc<Engine>,
    ast: AST,
    /// The original script source, retained for serialization.
    pub source: String,
}

impl CompiledFormFactor {
    /// Evaluate the form factor at a given $Q^2$.
    pub fn evaluate(&self, q2: f64) -> f64 {
        let mut scope = rhai::Scope::new();
        scope.push("q2", q2);
        self.engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &self.ast)
            .and_then(|v| v.as_float().map_err(|_t| {
                rhai::EvalAltResult::ErrorMismatchOutputType(
                    "f64".into(),
                    "non-numeric".into(),
                    rhai::Position::NONE,
                ).into()
            }))
            .unwrap_or(1.0)
    }
}

unsafe impl Send for CompiledFormFactor {}
unsafe impl Sync for CompiledFormFactor {}

// ---------------------------------------------------------------------------
// Reconstructed-Level Observable & Cut
// ---------------------------------------------------------------------------

/// A reco-level observable that receives both `reco` (`ReconstructedEvent`)
/// and `event` (`PhaseSpacePoint`) in the Rhai scope.
///
/// Use [`SpireScriptEngine::compile_reco_observable`] to construct.
pub struct RhaiRecoObservable {
    engine: Arc<Engine>,
    ast: AST,
    name: String,
}

impl RhaiRecoObservable {
    /// Set a descriptive name for this observable.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Evaluate this observable on a reconstructed event.
    pub fn evaluate(&self, reco: &ReconstructedEvent, event: &PhaseSpacePoint) -> f64 {
        let mut scope = rhai::Scope::new();
        scope.push("reco", reco.clone());
        scope.push("event", event.clone());
        self.engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &self.ast)
            .and_then(|v| {
                v.as_float().map_err(|_t| {
                    rhai::EvalAltResult::ErrorMismatchOutputType(
                        "f64".into(),
                        "non-numeric".into(),
                        rhai::Position::NONE,
                    )
                    .into()
                })
            })
            .unwrap_or(f64::NAN)
    }

    /// Human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

unsafe impl Send for RhaiRecoObservable {}
unsafe impl Sync for RhaiRecoObservable {}

/// A reco-level boolean cut receiving both `reco` and `event`.
pub struct RhaiRecoCut {
    engine: Arc<Engine>,
    ast: AST,
    name: String,
}

impl RhaiRecoCut {
    /// Set a descriptive name for this cut.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Returns `true` if the event passes this cut.
    pub fn is_passed(&self, reco: &ReconstructedEvent, event: &PhaseSpacePoint) -> bool {
        let mut scope = rhai::Scope::new();
        scope.push("reco", reco.clone());
        scope.push("event", event.clone());
        self.engine
            .eval_ast_with_scope::<bool>(&mut scope, &self.ast)
            .unwrap_or(false)
    }

    /// Human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

unsafe impl Send for RhaiRecoCut {}
unsafe impl Sync for RhaiRecoCut {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::SpacetimeVector;
    use crate::kinematics::PhaseSpacePoint;

    fn sample_event() -> PhaseSpacePoint {
        // e+ e- → mu+ mu- at 200 GeV CM
        // Particle 0: e- (100, 0, 0, 100)   — massless, along +z
        // Particle 1: e+ (100, 0, 0, -100)   — massless, along -z
        // Particle 2: mu+ (100, 30, 40, 86.6) — massive final state
        // Particle 3: mu- (100, -30, -40, -86.6)
        PhaseSpacePoint {
            momenta: vec![
                SpacetimeVector::new_4d(100.0, 0.0, 0.0, 100.0),
                SpacetimeVector::new_4d(100.0, 0.0, 0.0, -100.0),
                SpacetimeVector::new_4d(100.0, 30.0, 40.0, 86.6),
                SpacetimeVector::new_4d(100.0, -30.0, -40.0, -86.6),
            ],
            weight: 1.0,
        }
    }

    #[test]
    fn engine_creation() {
        let _engine = SpireScriptEngine::new();
    }

    #[test]
    fn compile_valid_script() {
        let engine = SpireScriptEngine::new();
        let result = engine.compile("let x = 42; x");
        assert!(result.is_ok());
    }

    #[test]
    fn compile_invalid_script() {
        let engine = SpireScriptEngine::new();
        let result = engine.compile("let x = ;; bad");
        assert!(result.is_err());
    }

    #[test]
    fn observable_energy() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[2].e()")
            .unwrap();
        let event = sample_event();
        let val = obs.evaluate(&event);
        assert!((val - 100.0).abs() < 1e-10);
    }

    #[test]
    fn observable_transverse_momentum() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[2].pt()")
            .unwrap();
        let event = sample_event();
        // pt = sqrt(30^2 + 40^2) = 50
        let val = obs.evaluate(&event);
        assert!((val - 50.0).abs() < 1e-10);
    }

    #[test]
    fn observable_invariant_mass_pair() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable(
                r#"
                let p1 = event.momenta[2];
                let p2 = event.momenta[3];
                let total = p1 + p2;
                total.m()
                "#,
            )
            .unwrap();
        let event = sample_event();
        // total = (200, 0, 0, 0) → m = 200
        let val = obs.evaluate(&event);
        assert!((val - 200.0).abs() < 1e-8);
    }

    #[test]
    fn observable_pseudorapidity() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[0].eta()")
            .unwrap();
        let event = sample_event();
        // Particle 0: (100, 0, 0, 100) → massless along +z, eta → +∞
        let val = obs.evaluate(&event);
        assert!(val > 10.0); // very forward
    }

    #[test]
    fn observable_phi() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[2].phi()")
            .unwrap();
        let event = sample_event();
        // Particle 2: px=30, py=40 → phi = atan2(40, 30) ≈ 0.927 rad
        let val = obs.evaluate(&event);
        assert!((val - 40.0_f64.atan2(30.0)).abs() < 1e-10);
    }

    #[test]
    fn observable_rapidity() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[2].rapidity()")
            .unwrap();
        let event = sample_event();
        // y = 0.5 * ln((E + pz) / (E - pz)) = 0.5 * ln((100+86.6)/(100-86.6))
        let expected = 0.5 * ((100.0_f64 + 86.6) / (100.0_f64 - 86.6)).ln();
        let val = obs.evaluate(&event);
        assert!((val - expected).abs() < 1e-6);
    }

    #[test]
    fn observable_weight() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.weight")
            .unwrap();
        let event = sample_event();
        let val = obs.evaluate(&event);
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cut_passes() {
        let engine = SpireScriptEngine::new();
        let cut = engine
            .compile_cut("event.momenta[2].pt() > 40.0")
            .unwrap();
        let event = sample_event();
        // pt = 50, so 50 > 40 → true
        assert!(cut.is_passed(&event));
    }

    #[test]
    fn cut_fails() {
        let engine = SpireScriptEngine::new();
        let cut = engine
            .compile_cut("event.momenta[2].pt() > 60.0")
            .unwrap();
        let event = sample_event();
        // pt = 50, so 50 > 60 → false
        assert!(!cut.is_passed(&event));
    }

    #[test]
    fn cut_combined_pt_eta() {
        let engine = SpireScriptEngine::new();
        let cut = engine
            .compile_cut(
                r#"
                let p = event.momenta[2];
                p.pt() > 25.0 && p.eta().abs() < 5.0
                "#,
            )
            .unwrap();
        let event = sample_event();
        assert!(cut.is_passed(&event));
    }

    #[test]
    fn vec4_constructor_in_script() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable(
                r#"
                let v = vec4(10.0, 3.0, 4.0, 0.0);
                v.m()
                "#,
            )
            .unwrap();
        // m = sqrt(100 - 9 - 16) = sqrt(75) ≈ 8.66
        let event = sample_event();
        let val = obs.evaluate(&event);
        assert!((val - 75.0_f64.sqrt()).abs() < 1e-8);
    }

    #[test]
    fn vector_addition_in_script() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable(
                r#"
                let v1 = vec4(10.0, 1.0, 0.0, 0.0);
                let v2 = vec4(20.0, -1.0, 0.0, 0.0);
                let sum = v1 + v2;
                sum.e()
                "#,
            )
            .unwrap();
        let event = sample_event();
        let val = obs.evaluate(&event);
        assert!((val - 30.0).abs() < 1e-10);
    }

    #[test]
    fn vector_subtraction_in_script() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable(
                r#"
                let v1 = event.momenta[0];
                let v2 = event.momenta[1];
                let diff = v1 - v2;
                diff.pz()
                "#,
            )
            .unwrap();
        let event = sample_event();
        // pz1 - pz2 = 100 - (-100) = 200
        let val = obs.evaluate(&event);
        assert!((val - 200.0).abs() < 1e-10);
    }

    #[test]
    fn form_factor_dipole() {
        let engine = SpireScriptEngine::new();
        let ff = engine
            .compile_form_factor(
                r#"
                let r = 1.0 + q2 / 0.71;
                1.0 / (r * r)
                "#,
            )
            .unwrap();
        // At q2 = 0: F = 1.0
        assert!((ff.evaluate(0.0) - 1.0).abs() < 1e-10);
        // At q2 = 0.71: F = (1 + 1)^{-2} = 0.25
        assert!((ff.evaluate(0.71) - 0.25).abs() < 1e-10);
    }

    #[test]
    fn form_factor_exponential() {
        let engine = SpireScriptEngine::new();
        let ff = engine
            .compile_form_factor("(-q2 / 1.0).exp()")
            .unwrap();
        // At q2 = 0: F = 1.0
        assert!((ff.evaluate(0.0) - 1.0).abs() < 1e-10);
        // At q2 = 1: F = e^{-1}
        assert!((ff.evaluate(1.0) - (-1.0_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn validate_good_script() {
        let engine = SpireScriptEngine::new();
        assert!(engine.validate_script("let x = 1; x + 2").is_ok());
    }

    #[test]
    fn validate_bad_script() {
        let engine = SpireScriptEngine::new();
        assert!(engine.validate_script("let = ;").is_err());
    }

    #[test]
    fn observable_with_name() {
        let engine = SpireScriptEngine::new();
        let obs = engine
            .compile_observable("event.momenta[0].e()")
            .unwrap()
            .with_name("Leading Lepton Energy");
        assert_eq!(obs.name(), "Leading Lepton Energy");
    }

    #[test]
    fn cut_with_name() {
        let engine = SpireScriptEngine::new();
        let cut = engine
            .compile_cut("event.momenta[0].pt() > 10.0")
            .unwrap()
            .with_name("Trigger pT");
        assert_eq!(cut.name(), "Trigger pT");
    }

    #[test]
    fn observable_trait_object() {
        // Verify Observable is object-safe.
        let engine = SpireScriptEngine::new();
        let obs: Box<dyn Observable> = Box::new(
            engine.compile_observable("event.momenta[0].e()").unwrap(),
        );
        let event = sample_event();
        let val = obs.evaluate(&event);
        assert!((val - 100.0).abs() < 1e-10);
    }

    #[test]
    fn cut_trait_object() {
        // Verify KinematicCut is object-safe.
        let engine = SpireScriptEngine::new();
        let cut: Box<dyn KinematicCut> = Box::new(
            engine.compile_cut("true").unwrap(),
        );
        let event = sample_event();
        assert!(cut.is_passed(&event));
    }
}
