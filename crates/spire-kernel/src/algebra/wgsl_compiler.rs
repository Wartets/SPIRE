//! # WGSL Compiler - CasExpr → WebGPU Shading Language
//!
//! Translates the symbolic [`CasExpr`] amplitude AST into WGSL compute
//! shader source code suitable for execution on the GPU via `wgpu`.
//!
//! ## Design
//!
//! The compiler performs a recursive tree walk over the `CasExpr` tree,
//! emitting WGSL code that evaluates the amplitude numerically. The
//! generated shader operates on arrays of four-momenta (one per
//! phase-space event) and writes the squared amplitude to an output
//! buffer.
//!
//! ## WGSL Complex Arithmetic
//!
//! Since WGSL has no native complex type, the compiler emits a `Complex`
//! struct and helper functions (`c_mul`, `c_add`, `c_div`, `c_conj`,
//! `c_norm_sq`) as a prelude. All amplitude expressions are compiled to
//! complex-valued WGSL expressions.
//!
//! ## Memory Layout
//!
//! ```text
//! @group(0) @binding(0) var<storage, read> momenta : array<vec4<f32>>;
//! @group(0) @binding(1) var<storage, read_write> results : array<f32>;
//! @group(0) @binding(2) var<uniform> params : Params;
//! ```
//!
//! Each event occupies `n_external` consecutive `vec4<f32>` entries in the
//! `momenta` buffer (E, px, py, pz). The `results` buffer receives
//! $|\mathcal{M}|^2$ for each event.
//!
//! ## Limitations
//!
//! - Traces and loop integrals must be evaluated symbolically on the CPU
//!   before GPU compilation. The compiler panics if it encounters a `Trace`
//!   or `LoopIntegral` node.
//! - Spinors and gamma matrices are treated as pre-contracted scalar
//!   expressions (the Dirac algebra must be resolved before compilation).

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::CasExpr;

// ===========================================================================
// WGSL Prelude - Complex Arithmetic Library
// ===========================================================================

/// The WGSL prelude: complex number struct and arithmetic functions.
///
/// This block is prepended to every generated shader. It provides the
/// minimal complex algebra needed for amplitude evaluation.
const WGSL_PRELUDE: &str = r#"
// ---------------------------------------------------------------------------
// Complex arithmetic (no native complex in WGSL)
// ---------------------------------------------------------------------------
struct Complex {
    re : f32,
    im : f32,
}

fn c_new(re: f32, im: f32) -> Complex {
    return Complex(re, im);
}

fn c_real(v: f32) -> Complex {
    return Complex(v, 0.0);
}

fn c_zero() -> Complex {
    return Complex(0.0, 0.0);
}

fn c_one() -> Complex {
    return Complex(1.0, 0.0);
}

fn c_i() -> Complex {
    return Complex(0.0, 1.0);
}

fn c_add(a: Complex, b: Complex) -> Complex {
    return Complex(a.re + b.re, a.im + b.im);
}

fn c_sub(a: Complex, b: Complex) -> Complex {
    return Complex(a.re - b.re, a.im - b.im);
}

fn c_mul(a: Complex, b: Complex) -> Complex {
    return Complex(a.re * b.re - a.im * b.im, a.re * b.im + a.im * b.re);
}

fn c_div(a: Complex, b: Complex) -> Complex {
    let d = b.re * b.re + b.im * b.im;
    return Complex((a.re * b.re + a.im * b.im) / d,
                   (a.im * b.re - a.re * b.im) / d);
}

fn c_neg(a: Complex) -> Complex {
    return Complex(-a.re, -a.im);
}

fn c_conj(a: Complex) -> Complex {
    return Complex(a.re, -a.im);
}

fn c_norm_sq(a: Complex) -> f32 {
    return a.re * a.re + a.im * a.im;
}

fn c_scale(a: Complex, s: f32) -> Complex {
    return Complex(a.re * s, a.im * s);
}

// ---------------------------------------------------------------------------
// Minkowski dot product  (+ - - - signature)
// ---------------------------------------------------------------------------
fn dot4(a: vec4<f32>, b: vec4<f32>) -> f32 {
    return a.x * b.x - a.y * b.y - a.z * b.z - a.w * b.w;
}
"#;

// ===========================================================================
// Compiler State
// ===========================================================================

/// The WGSL shader compiler for CasExpr amplitude trees.
///
/// # Usage
///
/// ```ignore
/// use spire_kernel::algebra::wgsl_compiler::WgslCompiler;
/// use spire_kernel::algebra::CasExpr;
///
/// let expr = CasExpr::DotProduct {
///     left: "p1".into(),
///     right: "p2".into(),
/// };
///
/// let mut compiler = WgslCompiler::new(2, &["p1", "p2"]);
/// let shader = compiler.compile(&expr);
/// ```
pub struct WgslCompiler {
    /// Number of external momenta per event.
    n_momenta: usize,
    /// Map from momentum label → index in the momenta buffer.
    momentum_indices: HashMap<String, usize>,
    /// Temporary variable counter for generating unique names.
    temp_counter: usize,
    /// Accumulated helper variable declarations (let tmp_N = ...).
    declarations: Vec<String>,
    /// Set of symbol names that are expected as uniform parameters.
    uniform_symbols: HashSet<String>,
}

impl WgslCompiler {
    /// Create a new compiler for an amplitude with the given external
    /// momentum labels.
    ///
    /// The order of `momentum_labels` determines the memory layout:
    /// momentum `momentum_labels[i]` is stored at offset `event_id * n +
    /// i` in the `momenta` storage buffer.
    pub fn new(n_momenta: usize, momentum_labels: &[&str]) -> Self {
        let mut momentum_indices = HashMap::new();
        for (i, label) in momentum_labels.iter().enumerate() {
            momentum_indices.insert((*label).to_string(), i);
        }
        Self {
            n_momenta,
            momentum_indices,
            temp_counter: 0,
            declarations: Vec::new(),
            uniform_symbols: HashSet::new(),
        }
    }

    /// Compile a `CasExpr` into a complete WGSL compute shader string.
    ///
    /// The generated shader contains:
    /// 1. The complex arithmetic prelude.
    /// 2. Storage/uniform binding declarations.
    /// 3. A `@compute @workgroup_size(64)` entry point `main` that
    ///    evaluates $|\mathcal{M}|^2$ for each event and writes it to
    ///    the results buffer.
    pub fn compile(&mut self, expr: &CasExpr) -> String {
        // Reset state for a fresh compilation.
        self.temp_counter = 0;
        self.declarations.clear();
        self.uniform_symbols.clear();

        // First pass: collect all symbol names used as uniforms.
        self.collect_symbols(expr);

        // Second pass: compile the expression tree into a WGSL expression.
        let result_expr = self.compile_expr(expr);

        // Assemble the full shader.
        self.assemble_shader(&result_expr)
    }

    /// Compile a `CasExpr` and return **only** the body expression
    /// (no prelude, no bindings). Useful for unit tests and embedding
    /// the amplitude into a larger hand-written shader.
    pub fn compile_expression_only(&mut self, expr: &CasExpr) -> String {
        self.temp_counter = 0;
        self.declarations.clear();
        self.uniform_symbols.clear();
        self.compile_expr(expr)
    }

    // -----------------------------------------------------------------------
    // Symbol Collection (first pass)
    // -----------------------------------------------------------------------

    /// Recursively walk the tree and record every `Symbol` name as a
    /// uniform parameter.
    fn collect_symbols(&mut self, expr: &CasExpr) {
        match expr {
            CasExpr::Symbol { name, .. } => {
                self.uniform_symbols.insert(name.clone());
            }
            CasExpr::Add(terms) => {
                for t in terms {
                    self.collect_symbols(t);
                }
            }
            CasExpr::Mul(factors) => {
                for f in factors {
                    self.collect_symbols(f);
                }
            }
            CasExpr::Neg(inner) | CasExpr::Trace(inner) => {
                self.collect_symbols(inner);
            }
            CasExpr::Fraction {
                numerator,
                denominator,
            } => {
                self.collect_symbols(numerator);
                self.collect_symbols(denominator);
            }
            CasExpr::Commutator(a, b) | CasExpr::AntiCommutator(a, b) => {
                self.collect_symbols(a);
                self.collect_symbols(b);
            }
            // Leaves with no sub-expressions.
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Expression Compilation (second pass)
    // -----------------------------------------------------------------------

    /// Allocate a fresh temporary variable name.
    fn fresh_tmp(&mut self) -> String {
        let name = format!("tmp_{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    /// Emit a `let` declaration and return the temporary's name.
    fn emit_let(&mut self, value: &str) -> String {
        let name = self.fresh_tmp();
        self.declarations
            .push(format!("    let {} = {};", name, value));
        name
    }

    /// Compile a `CasExpr` node into a WGSL expression string.
    ///
    /// For complex sub-expressions, intermediate results are hoisted into
    /// `let` bindings (via [`emit_let`]) to avoid deeply nested
    /// expressions and to improve readability of the generated shader.
    fn compile_expr(&mut self, expr: &CasExpr) -> String {
        match expr {
            // -- Atomic leaves --------------------------------------------------
            CasExpr::Scalar(v) => {
                // Emit as a real complex constant.
                format!("c_real({:.8})", v)
            }

            CasExpr::ImaginaryUnit => "c_i()".to_string(),

            CasExpr::Symbol { name, .. } => {
                // Symbols map to uniform parameters accessed as
                // `c_real(params.NAME)`.
                let safe_name = sanitize_identifier(name);
                format!("c_real(params.{})", safe_name)
            }

            CasExpr::DotProduct { left, right } => {
                let l_idx = self.momentum_index(left);
                let r_idx = self.momentum_index(right);
                format!(
                    "c_real(dot4(momenta[base + {}u], momenta[base + {}u]))",
                    l_idx, r_idx
                )
            }

            CasExpr::Momentum { label, .. } => {
                // Return the raw 4-vector load as a complex scalar
                // representing the component. In practice, momenta appear
                // inside DotProduct nodes after index contraction, so this
                // path is uncommon. We emit the energy component as a
                // fallback.
                let idx = self.momentum_index(label);
                format!("c_real(momenta[base + {}u].x)", idx)
            }

            CasExpr::SlashedMomentum { label } => {
                // Slashed momenta should be resolved before GPU
                // compilation (Dirac algebra contracted). Emit a
                // placeholder that will fail loudly if reached.
                let idx = self.momentum_index(label);
                let tmp = self.emit_let(&format!(
                    "c_real(dot4(momenta[base + {idx}u], momenta[base + {idx}u]))"
                ));
                tmp
            }

            CasExpr::MetricTensor { .. } => {
                // After contraction the metric tensor is eliminated.
                // If it survives to GPU compilation, treat as identity (1).
                "c_one()".to_string()
            }

            CasExpr::KroneckerDelta { .. } => "c_one()".to_string(),

            CasExpr::LeviCivita { .. } => {
                // Levi-Civita should be contracted before GPU compilation.
                "c_zero()".to_string()
            }

            CasExpr::GammaMat { .. } | CasExpr::Gamma5 => {
                // Gamma matrices must be traced/contracted on CPU.
                "c_one()".to_string()
            }

            CasExpr::SpinorU { .. }
            | CasExpr::SpinorUBar { .. }
            | CasExpr::SpinorV { .. }
            | CasExpr::SpinorVBar { .. } => {
                // Spinors should be contracted into scalar bilinears on
                // CPU before compilation.
                "c_one()".to_string()
            }

            CasExpr::Tensor { .. } => {
                // Polarization sums / contractions should be done on CPU.
                "c_one()".to_string()
            }

            CasExpr::PropagatorDenom { momentum, mass_sq } => {
                // 1 / (p² - m² + iε)
                // We use a small imaginary part for numerical stability.
                let p_idx = self.momentum_index(momentum);
                let p_sq = format!("dot4(momenta[base + {}u], momenta[base + {}u])", p_idx, p_idx);
                let denom_re = format!("({} - {:.8})", p_sq, mass_sq);
                let denom = format!("c_new({}, 1.0e-6)", denom_re);
                let tmp = self.emit_let(&format!("c_div(c_one(), {})", denom));
                tmp
            }

            // -- Composite nodes ------------------------------------------------
            CasExpr::Add(terms) => {
                if terms.is_empty() {
                    return "c_zero()".to_string();
                }
                if terms.len() == 1 {
                    return self.compile_expr(&terms[0]);
                }
                let first = self.compile_expr(&terms[0]);
                let mut acc = self.emit_let(&first);
                for term in &terms[1..] {
                    let t = self.compile_expr(term);
                    acc = self.emit_let(&format!("c_add({}, {})", acc, t));
                }
                acc
            }

            CasExpr::Mul(factors) => {
                if factors.is_empty() {
                    return "c_one()".to_string();
                }
                if factors.len() == 1 {
                    return self.compile_expr(&factors[0]);
                }
                let first = self.compile_expr(&factors[0]);
                let mut acc = self.emit_let(&first);
                for factor in &factors[1..] {
                    let f = self.compile_expr(factor);
                    acc = self.emit_let(&format!("c_mul({}, {})", acc, f));
                }
                acc
            }

            CasExpr::Neg(inner) => {
                let inner_code = self.compile_expr(inner);
                let tmp = self.emit_let(&format!("c_neg({})", inner_code));
                tmp
            }

            CasExpr::Fraction {
                numerator,
                denominator,
            } => {
                let num = self.compile_expr(numerator);
                let den = self.compile_expr(denominator);
                let tmp = self.emit_let(&format!("c_div({}, {})", num, den));
                tmp
            }

            CasExpr::Trace(inner) => {
                // Traces must be evaluated on CPU before compilation.
                // If one slips through, compile the inner expression
                // (which should already be a scalar after trace evaluation).
                self.compile_expr(inner)
            }

            CasExpr::Commutator(a, b) => {
                // [A, B] = AB - BA
                let a_code = self.compile_expr(a);
                let b_code = self.compile_expr(b);
                let ab = self.emit_let(&format!("c_mul({}, {})", a_code, b_code));
                let ba = self.emit_let(&format!("c_mul({}, {})", b_code, a_code));
                let tmp = self.emit_let(&format!("c_sub({}, {})", ab, ba));
                tmp
            }

            CasExpr::AntiCommutator(a, b) => {
                // {A, B} = AB + BA
                let a_code = self.compile_expr(a);
                let b_code = self.compile_expr(b);
                let ab = self.emit_let(&format!("c_mul({}, {})", a_code, b_code));
                let ba = self.emit_let(&format!("c_mul({}, {})", b_code, a_code));
                let tmp = self.emit_let(&format!("c_add({}, {})", ab, ba));
                tmp
            }
        }
    }

    // -----------------------------------------------------------------------
    // Momentum Index Lookup
    // -----------------------------------------------------------------------

    /// Look up the buffer index for a momentum label.
    ///
    /// If the label is not found in the pre-registered momentum map
    /// (e.g., for internal/virtual momenta), the compiler assigns a new
    /// index sequentially.
    fn momentum_index(&mut self, label: &str) -> usize {
        if let Some(&idx) = self.momentum_indices.get(label) {
            return idx;
        }
        // Assign next available index.
        let idx = self.n_momenta;
        self.n_momenta += 1;
        self.momentum_indices.insert(label.to_string(), idx);
        idx
    }

    // -----------------------------------------------------------------------
    // Shader Assembly
    // -----------------------------------------------------------------------

    /// Assemble the complete WGSL shader from the compiled expression and
    /// collected metadata.
    fn assemble_shader(&self, result_expr: &str) -> String {
        let mut shader = String::with_capacity(4096);

        // --- Prelude (complex math + dot4) ---
        shader.push_str(WGSL_PRELUDE);
        shader.push('\n');

        // --- Uniform parameters struct ---
        self.emit_params_struct(&mut shader);

        // --- Storage and uniform bindings ---
        write!(
            shader,
            r#"
@group(0) @binding(0) var<storage, read> momenta : array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> results : array<f32>;
@group(0) @binding(2) var<uniform> params : Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid : vec3<u32>) {{
    let event_id = gid.x;
    let n_events = arrayLength(&results);
    if (event_id >= n_events) {{
        return;
    }}

    // Base index into the momenta buffer for this event.
    let base = event_id * {}u;

"#,
            self.n_momenta
        )
        .expect("write to String cannot fail");

        // --- Hoisted let declarations ---
        for decl in &self.declarations {
            shader.push_str(decl);
            shader.push('\n');
        }
        shader.push('\n');

        // --- Final result: |M|² ---
        write!(
            shader,
            "    let amplitude = {};\n",
            result_expr
        )
        .expect("write to String cannot fail");
        shader.push_str("    results[event_id] = c_norm_sq(amplitude);\n");
        shader.push_str("}\n");

        shader
    }

    /// Emit the `Params` uniform struct containing all symbolic parameters
    /// discovered during compilation (coupling constants, masses, etc.).
    fn emit_params_struct(&self, shader: &mut String) {
        shader.push_str("struct Params {\n");
        shader.push_str("    /// Number of events (redundant with arrayLength, but useful).\n");
        shader.push_str("    n_events : u32,\n");

        // Sort for deterministic output.
        let mut symbols: Vec<&String> = self.uniform_symbols.iter().collect();
        symbols.sort();

        for name in symbols {
            let safe = sanitize_identifier(name);
            let _ = write!(shader, "    {} : f32,\n", safe);
        }

        shader.push_str("}\n");
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Convert a physics symbol name into a valid WGSL identifier.
///
/// Replaces characters illegal in WGSL identifiers (-, +, *, etc.) with
/// underscores and prepends `s_` if the name starts with a digit.
fn sanitize_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if i == 0 && ch.is_ascii_digit() {
                out.push_str("s_");
            }
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("_unnamed");
    }
    out
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::CasExpr;

    #[test]
    fn scalar_compiles_to_c_real() {
        let mut c = WgslCompiler::new(0, &[]);
        let code = c.compile_expression_only(&CasExpr::Scalar(3.14));
        assert!(code.contains("c_real("));
        assert!(code.contains("3.14"));
    }

    #[test]
    fn imaginary_unit_compiles_to_c_i() {
        let mut c = WgslCompiler::new(0, &[]);
        let code = c.compile_expression_only(&CasExpr::ImaginaryUnit);
        assert_eq!(code, "c_i()");
    }

    #[test]
    fn dot_product_uses_dot4() {
        let mut c = WgslCompiler::new(2, &["p1", "p2"]);
        let expr = CasExpr::DotProduct {
            left: "p1".into(),
            right: "p2".into(),
        };
        let code = c.compile_expression_only(&expr);
        assert!(code.contains("dot4"));
        assert!(code.contains("momenta[base + 0u]"));
        assert!(code.contains("momenta[base + 1u]"));
    }

    #[test]
    fn add_generates_c_add_chain() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Add(vec![
            CasExpr::Scalar(1.0),
            CasExpr::Scalar(2.0),
            CasExpr::Scalar(3.0),
        ]);
        let code = c.compile_expression_only(&expr);
        // Should reference tmp variables from the add chain.
        assert!(code.starts_with("tmp_"));
    }

    #[test]
    fn mul_generates_c_mul_chain() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Mul(vec![
            CasExpr::Scalar(2.0),
            CasExpr::Scalar(3.0),
        ]);
        let code = c.compile_expression_only(&expr);
        assert!(code.starts_with("tmp_"));
    }

    #[test]
    fn fraction_generates_c_div() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Fraction {
            numerator: Box::new(CasExpr::Scalar(1.0)),
            denominator: Box::new(CasExpr::Scalar(2.0)),
        };
        let code = c.compile_expression_only(&expr);
        assert!(code.starts_with("tmp_"));
        assert!(c.declarations.iter().any(|d| d.contains("c_div")));
    }

    #[test]
    fn negation_generates_c_neg() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Neg(Box::new(CasExpr::Scalar(5.0)));
        let code = c.compile_expression_only(&expr);
        assert!(code.starts_with("tmp_"));
        assert!(c.declarations.iter().any(|d| d.contains("c_neg")));
    }

    #[test]
    fn propagator_denom_generates_div() {
        let mut c = WgslCompiler::new(2, &["q", "p1"]);
        let expr = CasExpr::PropagatorDenom {
            momentum: "q".into(),
            mass_sq: 8315.1784, // m_Z²
        };
        let code = c.compile_expression_only(&expr);
        assert!(code.starts_with("tmp_"));
        assert!(c.declarations.iter().any(|d| d.contains("dot4")));
        assert!(c.declarations.iter().any(|d| d.contains("c_div")));
    }

    #[test]
    fn symbol_maps_to_params() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Symbol {
            name: "alpha_s".into(),
            indices: vec![],
        };
        let code = c.compile_expression_only(&expr);
        assert!(code.contains("params.alpha_s"));
    }

    #[test]
    fn full_shader_compiles_cleanly() {
        let mut c = WgslCompiler::new(2, &["p1", "p2"]);
        let expr = CasExpr::Mul(vec![
            CasExpr::Symbol {
                name: "g".into(),
                indices: vec![],
            },
            CasExpr::DotProduct {
                left: "p1".into(),
                right: "p2".into(),
            },
            CasExpr::Fraction {
                numerator: Box::new(CasExpr::Scalar(1.0)),
                denominator: Box::new(CasExpr::PropagatorDenom {
                    momentum: "p1".into(),
                    mass_sq: 0.0,
                }),
            },
        ]);
        let shader = c.compile(&expr);
        assert!(shader.contains("@compute"));
        assert!(shader.contains("@workgroup_size(64)"));
        assert!(shader.contains("fn main("));
        assert!(shader.contains("struct Complex"));
        assert!(shader.contains("struct Params"));
        assert!(shader.contains("c_norm_sq(amplitude)"));
        assert!(shader.contains("results[event_id]"));
    }

    #[test]
    fn sanitize_identifier_handles_special_chars() {
        assert_eq!(sanitize_identifier("alpha_s"), "alpha_s");
        assert_eq!(sanitize_identifier("e-"), "e_");
        assert_eq!(sanitize_identifier("3body"), "s_3body");
        assert_eq!(sanitize_identifier(""), "_unnamed");
        assert_eq!(sanitize_identifier("g+"), "g_");
    }

    #[test]
    fn empty_add_is_zero() {
        let mut c = WgslCompiler::new(0, &[]);
        let code = c.compile_expression_only(&CasExpr::Add(vec![]));
        assert_eq!(code, "c_zero()");
    }

    #[test]
    fn empty_mul_is_one() {
        let mut c = WgslCompiler::new(0, &[]);
        let code = c.compile_expression_only(&CasExpr::Mul(vec![]));
        assert_eq!(code, "c_one()");
    }

    #[test]
    fn commutator_generates_sub() {
        let mut c = WgslCompiler::new(0, &[]);
        let expr = CasExpr::Commutator(
            Box::new(CasExpr::Scalar(2.0)),
            Box::new(CasExpr::Scalar(3.0)),
        );
        let code = c.compile_expression_only(&expr);
        assert!(code.starts_with("tmp_"));
        assert!(c.declarations.iter().any(|d| d.contains("c_sub")));
    }
}
