//! # UFO (Universal FeynRules Output) Bridge
//!
//! Pure-Rust parser for UFO model files that maps external particle, vertex,
//! coupling, and parameter definitions into the SPIRE ontology and
//! [`TheoreticalModel`].
//!
//! ## Format Overview
//!
//! A UFO model is a Python package with these key files:
//!
//! - `particles.py` — Particle definitions: `Particle(pdg_code=21, name='g', ...)`
//! - `vertices.py` — Vertex definitions: `Vertex(name='V_1', particles=[...], ...)`
//! - `couplings.py` — Coupling definitions: `Coupling(name='GC_3', value='...', ...)`
//! - `parameters.py` — Parameter definitions: `Parameter(name='aS', ...)`
//! - `lorentz.py` — Lorentz structures for vertices.
//!
//! This module uses lightweight string/regex-style parsing (no Python interpreter)
//! to be WASM-compatible.
//!
//! ## References
//!
//! - C. Degrande *et al.*, "UFO – The Universal FeynRules Output",
//!   Comput. Phys. Commun. **183** (2012) 1201

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lagrangian::{LagrangianTerm, LagrangianTermKind, TheoreticalModel, VertexFactor};
use crate::ontology::{
    BaryonNumber, ChargeConjugation, ColorRepresentation, ElectricCharge, Field, Hypercharge,
    InteractionType, LeptonNumbers, Parity, QuantumNumbers, Spin, WeakIsospin, WeakMultiplet,
};
use crate::SpireResult;

// ---------------------------------------------------------------------------
// UFO Data Structures
// ---------------------------------------------------------------------------

/// A particle definition extracted from a UFO `particles.py` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoParticle {
    /// PDG Monte Carlo ID.
    pub pdg_code: i32,
    /// Internal name (e.g., `"g"`, `"e__minus__"`).
    pub name: String,
    /// Antiparticle name (e.g., `"g"`, `"e__plus__"`).
    pub antiname: String,
    /// UFO spin convention: $2S + 1$ (1 = scalar, 2 = fermion, 3 = vector).
    pub spin: i32,
    /// UFO color convention: 1 = singlet, 3 = triplet, -3 = anti-triplet, 8 = octet.
    pub color: i32,
    /// Mass parameter name (e.g., `"MZ"`, `"ZERO"`).
    pub mass_name: String,
    /// Width parameter name (e.g., `"WZ"`, `"ZERO"`).
    pub width_name: String,
    /// Electric charge in units of $|e|$ (as a float string, e.g., `"-1"`, `"2/3"`).
    pub charge: f64,
    /// Additional key-value attributes not explicitly parsed.
    pub extra: HashMap<String, String>,
}

/// A coupling definition extracted from `couplings.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoCoupling {
    /// Coupling name (e.g., `"GC_3"`, `"GC_100"`).
    pub name: String,
    /// Symbolic expression for the coupling value (Python expression string).
    pub value: String,
    /// Perturbative order: `{"QED": 1}`, `{"QCD": 2}`, etc.
    pub order: HashMap<String, i32>,
}

/// A Lorentz structure definition extracted from `lorentz.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoLorentz {
    /// Structure name (e.g., `"FFV1"`, `"SSS1"`).
    pub name: String,
    /// Number of external particles (spins).
    pub spins: Vec<i32>,
    /// Symbolic structure string.
    pub structure: String,
}

/// A vertex definition extracted from `vertices.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoVertex {
    /// Vertex name (e.g., `"V_1"`).
    pub name: String,
    /// Particle references (names from `particles.py`).
    pub particles: Vec<String>,
    /// Color structure strings.
    pub color: Vec<String>,
    /// Lorentz structure references.
    pub lorentz: Vec<String>,
    /// Coupling map: `{(color_idx, lorentz_idx): coupling_name}`.
    pub couplings: HashMap<(usize, usize), String>,
}

/// An external parameter definition from `parameters.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoParameter {
    /// Parameter name (e.g., `"aS"`, `"MZ"`, `"WZ"`).
    pub name: String,
    /// Nature: `"external"` or `"internal"`.
    pub nature: String,
    /// Type: `"real"` or `"complex"`.
    pub param_type: String,
    /// Numerical value (for external parameters).
    pub value: Option<f64>,
    /// Symbolic expression (for internal/derived parameters).
    pub expression: Option<String>,
    /// LaTeX representation.
    pub texname: Option<String>,
    /// SLHA block this parameter lives in (if any).
    pub lhablock: Option<String>,
    /// SLHA block index (if any).
    pub lhacode: Option<Vec<i32>>,
}

/// A complete UFO model parsed from a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfoModel {
    /// All particles in the model.
    pub particles: Vec<UfoParticle>,
    /// All vertices.
    pub vertices: Vec<UfoVertex>,
    /// All coupling definitions.
    pub couplings: Vec<UfoCoupling>,
    /// All parameters (external + internal).
    pub parameters: Vec<UfoParameter>,
    /// All Lorentz structures.
    pub lorentz_structures: Vec<UfoLorentz>,
}

// ---------------------------------------------------------------------------
// Python Expression Parser Utilities
// ---------------------------------------------------------------------------

/// Extract the string content between matching delimiters starting at `start`.
///
/// Handles nested parentheses/brackets. Returns the content and the position
/// after the closing delimiter.
fn extract_balanced(input: &str, start: usize, open: char, close: char) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if start >= bytes.len() || bytes[start] as char != open {
        return None;
    }

    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = '"';
    let mut prev_escape = false;

    for (i, ch) in input[start..].char_indices() {
        if in_string {
            if !prev_escape && ch == string_char {
                in_string = false;
            }
            prev_escape = ch == '\\';
            continue;
        }

        if ch == '\'' || ch == '"' {
            in_string = true;
            string_char = ch;
            prev_escape = false;
            continue;
        }

        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let content = &input[start + 1..start + i];
                return Some((content.to_string(), start + i + 1));
            }
        }
    }
    None
}

/// Parse a Python keyword argument like `key=value` or `key='string_value'`.
///
/// Returns `(key, raw_value_string)` pairs from a constructor call body.
fn parse_kwargs(body: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut chars = body.char_indices().peekable();
    let mut current_key = String::new();
    let mut current_val = String::new();
    let mut in_key = true;
    let mut depth = 0i32; // Track nested brackets/parens
    let mut in_string = false;
    let mut string_char = '"';

    while let Some(&(_, ch)) = chars.peek() {
        chars.next();

        if in_string {
            current_val.push(ch);
            if ch == string_char {
                in_string = false;
            }
            continue;
        }

        if (ch == '\'' || ch == '"') && !in_key {
            in_string = true;
            string_char = ch;
            current_val.push(ch);
            continue;
        }

        if ch == '(' || ch == '[' || ch == '{' {
            depth += 1;
            if !in_key {
                current_val.push(ch);
            }
            continue;
        }
        if ch == ')' || ch == ']' || ch == '}' {
            depth -= 1;
            if !in_key {
                current_val.push(ch);
            }
            continue;
        }

        if ch == '=' && in_key && depth == 0 {
            in_key = false;
            continue;
        }

        if ch == ',' && depth == 0 && !in_key {
            let key = current_key.trim().to_string();
            let val = current_val.trim().to_string();
            if !key.is_empty() {
                result.push((key, val));
            }
            current_key.clear();
            current_val.clear();
            in_key = true;
            continue;
        }

        if ch == '\n' || ch == '\r' {
            continue; // Skip newlines inside constructor calls.
        }

        if in_key {
            current_key.push(ch);
        } else {
            current_val.push(ch);
        }
    }

    // Flush last pair.
    let key = current_key.trim().to_string();
    let val = current_val.trim().to_string();
    if !key.is_empty() && !val.is_empty() {
        result.push((key, val));
    }

    result
}

/// Strip Python string quotes from a value.
fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Parse a Python float/int literal, handling simple fractions like `"2/3"`.
fn parse_py_number(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(pos) = s.find('/') {
        let num: f64 = s[..pos].trim().parse().ok()?;
        let den: f64 = s[pos + 1..].trim().parse().ok()?;
        Some(num / den)
    } else {
        s.parse().ok()
    }
}

/// Parse a Python list literal `[item1, item2, ...]` into string items.
fn parse_py_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return vec![s.to_string()];
    }
    let inner = &s[1..s.len() - 1];
    inner
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// File Parsers
// ---------------------------------------------------------------------------

/// Find all constructor calls of `ClassName(...)` in a Python source string.
///
/// Returns `(variable_name, constructor_body)` pairs for assignments like:
/// ```python
/// foo = ClassName(key1=val1, key2=val2)
/// ```
fn find_constructor_calls<'a>(content: &'a str, class_name: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let pattern = format!("{}(", class_name);

    for line_start in 0..content.len() {
        // Look for "variable = ClassName("
        if let Some(ctor_pos) = content[line_start..].find(&pattern) {
            let abs_pos = line_start + ctor_pos;

            // Find the variable name (look backwards for '=').
            let before = &content[..abs_pos];
            if let Some(eq_pos) = before.rfind('=') {
                // Make sure this isn't inside a string or a !=, ==, etc.
                if eq_pos > 0
                    && !matches!(
                        before.as_bytes().get(eq_pos - 1),
                        Some(b'!' | b'<' | b'>' | b'=')
                    )
                {
                    let var_name = before
                        [before[..eq_pos].rfind('\n').map(|p| p + 1).unwrap_or(0)..eq_pos]
                        .trim()
                        .to_string();

                    // Extract the balanced parenthesized body.
                    let paren_start = abs_pos + class_name.len();
                    if let Some((body, end_pos)) = extract_balanced(content, paren_start, '(', ')')
                    {
                        results.push((var_name, body));
                        // Skip ahead to avoid re-matching.
                        if end_pos > line_start {
                            continue;
                        }
                    }
                }
            }
        } else {
            break; // No more matches.
        }
    }

    // Deduplicate (the scanning can hit overlaps).
    let mut seen = std::collections::HashSet::new();
    results.retain(|(name, _)| seen.insert(name.clone()));
    results
}

/// Parse a UFO `particles.py` file content into `UfoParticle` structs.
///
/// # Example
///
/// ```
/// use spire_kernel::theory::ufo::parse_ufo_particles;
///
/// let content = r#"
/// g = Particle(pdg_code=21, name='g', antiname='g', spin=3, color=8,
///              mass='ZERO', width='ZERO', charge=0)
/// "#;
///
/// let particles = parse_ufo_particles(content).unwrap();
/// assert_eq!(particles.len(), 1);
/// assert_eq!(particles[0].pdg_code, 21);
/// assert_eq!(particles[0].name, "g");
/// assert_eq!(particles[0].spin, 3);
/// assert_eq!(particles[0].color, 8);
/// ```
pub fn parse_ufo_particles(content: &str) -> SpireResult<Vec<UfoParticle>> {
    let calls = find_constructor_calls(content, "Particle");
    let mut particles = Vec::new();

    for (_var_name, body) in &calls {
        let kwargs = parse_kwargs(body);
        let map: HashMap<String, String> = kwargs.into_iter().collect();

        let pdg_code = map
            .get("pdg_code")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        let name = map.get("name").map(|s| strip_quotes(s)).unwrap_or_default();

        let antiname = map
            .get("antiname")
            .map(|s| strip_quotes(s))
            .unwrap_or_else(|| name.clone());

        let spin = map
            .get("spin")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);

        let color = map
            .get("color")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);

        let mass_name = map
            .get("mass")
            .map(|s| strip_quotes(s))
            .unwrap_or_else(|| "ZERO".into());

        let width_name = map
            .get("width")
            .map(|s| strip_quotes(s))
            .unwrap_or_else(|| "ZERO".into());

        let charge = map
            .get("charge")
            .and_then(|s| parse_py_number(s))
            .unwrap_or(0.0);

        // Collect remaining kwargs as extra.
        let known_keys = [
            "pdg_code", "name", "antiname", "spin", "color", "mass", "width", "charge",
        ];
        let extra: HashMap<String, String> = map
            .iter()
            .filter(|(k, _)| !known_keys.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), strip_quotes(v)))
            .collect();

        particles.push(UfoParticle {
            pdg_code,
            name,
            antiname,
            spin,
            color,
            mass_name,
            width_name,
            charge,
            extra,
        });
    }

    Ok(particles)
}

/// Parse a UFO `couplings.py` file content into `UfoCoupling` structs.
pub fn parse_ufo_couplings(content: &str) -> SpireResult<Vec<UfoCoupling>> {
    let calls = find_constructor_calls(content, "Coupling");
    let mut couplings = Vec::new();

    for (_var_name, body) in &calls {
        let kwargs = parse_kwargs(body);
        let map: HashMap<String, String> = kwargs.into_iter().collect();

        let name = map.get("name").map(|s| strip_quotes(s)).unwrap_or_default();

        let value = map
            .get("value")
            .map(|s| strip_quotes(s))
            .unwrap_or_default();

        // Parse order dict: {'QED':1, 'QCD':2}
        let order = if let Some(order_str) = map.get("order") {
            parse_py_order_dict(order_str)
        } else {
            HashMap::new()
        };

        couplings.push(UfoCoupling { name, value, order });
    }

    Ok(couplings)
}

/// Parse a Python order dict like `{'QED':1, 'QCD':2}`.
fn parse_py_order_dict(s: &str) -> HashMap<String, i32> {
    let mut result = HashMap::new();
    let s = s.trim();

    // Strip outer braces.
    let inner = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    for pair in inner.split(',') {
        let pair = pair.trim();
        if let Some(colon_pos) = pair.find(':') {
            let key = strip_quotes(pair[..colon_pos].trim());
            if let Ok(val) = pair[colon_pos + 1..].trim().parse::<i32>() {
                result.insert(key, val);
            }
        }
    }

    result
}

/// Parse a UFO `parameters.py` file content into `UfoParameter` structs.
pub fn parse_ufo_parameters(content: &str) -> SpireResult<Vec<UfoParameter>> {
    let calls = find_constructor_calls(content, "Parameter");
    let mut parameters = Vec::new();

    for (_var_name, body) in &calls {
        let kwargs = parse_kwargs(body);
        let map: HashMap<String, String> = kwargs.into_iter().collect();

        let name = map.get("name").map(|s| strip_quotes(s)).unwrap_or_default();

        let nature = map
            .get("nature")
            .map(|s| strip_quotes(s))
            .unwrap_or_else(|| "external".into());

        let param_type = map
            .get("type")
            .map(|s| strip_quotes(s))
            .unwrap_or_else(|| "real".into());

        let value = map.get("value").and_then(|s| parse_py_number(s));

        let expression = map.get("value").and_then(|s| {
            if parse_py_number(s).is_none() {
                Some(strip_quotes(s))
            } else {
                None
            }
        });

        let texname = map.get("texname").map(|s| strip_quotes(s));

        let lhablock = map.get("lhablock").map(|s| strip_quotes(s));

        let lhacode = map.get("lhacode").map(|s| {
            parse_py_list(s)
                .iter()
                .filter_map(|item| item.parse::<i32>().ok())
                .collect()
        });

        parameters.push(UfoParameter {
            name,
            nature,
            param_type,
            value,
            expression,
            texname,
            lhablock,
            lhacode,
        });
    }

    Ok(parameters)
}

/// Parse a UFO `lorentz.py` file content into `UfoLorentz` structs.
pub fn parse_ufo_lorentz(content: &str) -> SpireResult<Vec<UfoLorentz>> {
    let calls = find_constructor_calls(content, "Lorentz");
    let mut structures = Vec::new();

    for (_var_name, body) in &calls {
        let kwargs = parse_kwargs(body);
        let map: HashMap<String, String> = kwargs.into_iter().collect();

        let name = map.get("name").map(|s| strip_quotes(s)).unwrap_or_default();

        let spins = map
            .get("spins")
            .map(|s| {
                parse_py_list(s)
                    .iter()
                    .filter_map(|item| item.parse::<i32>().ok())
                    .collect()
            })
            .unwrap_or_default();

        let structure = map
            .get("structure")
            .map(|s| strip_quotes(s))
            .unwrap_or_default();

        structures.push(UfoLorentz {
            name,
            spins,
            structure,
        });
    }

    Ok(structures)
}

/// Parse a UFO `vertices.py` file content into `UfoVertex` structs.
pub fn parse_ufo_vertices(content: &str) -> SpireResult<Vec<UfoVertex>> {
    let calls = find_constructor_calls(content, "Vertex");
    let mut vertices = Vec::new();

    for (_var_name, body) in &calls {
        let kwargs = parse_kwargs(body);
        let map: HashMap<String, String> = kwargs.into_iter().collect();

        let name = map.get("name").map(|s| strip_quotes(s)).unwrap_or_default();

        // Particles: [P.e__minus__, P.e__plus__, P.a]
        let particles = map
            .get("particles")
            .map(|s| {
                parse_py_list(s)
                    .iter()
                    .map(|item| {
                        let item = item.trim();
                        // Strip the "P." prefix if present.
                        if let Some(stripped) = item.strip_prefix("P.") {
                            stripped.to_string()
                        } else {
                            item.to_string()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let color = map
            .get("color")
            .map(|s| parse_py_list(s).iter().map(|i| strip_quotes(i)).collect())
            .unwrap_or_default();

        let lorentz = map
            .get("lorentz")
            .map(|s| {
                parse_py_list(s)
                    .iter()
                    .map(|item| {
                        let item = item.trim();
                        if let Some(stripped) = item.strip_prefix("L.") {
                            stripped.to_string()
                        } else {
                            strip_quotes(item)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Couplings: {(0,0):C.GC_3, (1,0):C.GC_5}
        let couplings = map
            .get("couplings")
            .map(|s| parse_couplings_dict(s))
            .unwrap_or_default();

        vertices.push(UfoVertex {
            name,
            particles,
            color,
            lorentz,
            couplings,
        });
    }

    Ok(vertices)
}

/// Parse a UFO couplings dictionary like `{(0,0):C.GC_3}`.
fn parse_couplings_dict(s: &str) -> HashMap<(usize, usize), String> {
    let mut result = HashMap::new();
    let s = s.trim();
    let inner = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    // Split on "),", being careful about nested tuples.
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in inner.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
                if depth == 0 {
                    // Find the colon after the closing paren.
                    // We'll process this entry after collecting all chars up to next comma at depth 0.
                }
            }
            ',' if depth == 0 => {
                // Check if previous was the value part (after `:`)
                if current.contains(':') {
                    entries.push(current.trim().to_string());
                    current = String::new();
                } else {
                    current.push(ch);
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.trim().is_empty() {
        entries.push(current.trim().to_string());
    }

    for entry in &entries {
        // Expected format: "(i,j):C.GC_N" or "(i,j):'GC_N'"
        if let Some(colon_pos) = entry.find("):") {
            let tuple_str = &entry[..colon_pos + 1]; // includes the ')'
            let value_str = entry[colon_pos + 2..].trim();

            // Parse the tuple.
            let inner_tuple = tuple_str.trim_start_matches('(').trim_end_matches(')');
            let parts: Vec<&str> = inner_tuple.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(i), Ok(j)) = (
                    parts[0].trim().parse::<usize>(),
                    parts[1].trim().parse::<usize>(),
                ) {
                    let coupling_name = if let Some(stripped) = value_str.strip_prefix("C.") {
                        stripped.to_string()
                    } else {
                        strip_quotes(value_str)
                    };
                    result.insert((i, j), coupling_name);
                }
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// UFO Model Assembly
// ---------------------------------------------------------------------------

/// Collection of raw UFO file contents for parsing.
///
/// Each field holds the string content of the corresponding `.py` file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UfoFileContents {
    pub particles_py: Option<String>,
    pub vertices_py: Option<String>,
    pub couplings_py: Option<String>,
    pub parameters_py: Option<String>,
    pub lorentz_py: Option<String>,
}

/// Parse all UFO files and assemble a complete `UfoModel`.
///
/// Missing files are treated as empty (no particles, vertices, etc.).
///
/// # Example
///
/// ```
/// use spire_kernel::theory::ufo::{parse_ufo_model, UfoFileContents};
///
/// let files = UfoFileContents {
///     particles_py: Some(r#"
///         g = Particle(pdg_code=21, name='g', antiname='g', spin=3, color=8,
///                      mass='ZERO', width='ZERO', charge=0)
///     "#.into()),
///     ..Default::default()
/// };
///
/// let model = parse_ufo_model(&files).unwrap();
/// assert_eq!(model.particles.len(), 1);
/// assert_eq!(model.particles[0].pdg_code, 21);
/// ```
pub fn parse_ufo_model(files: &UfoFileContents) -> SpireResult<UfoModel> {
    let particles = match &files.particles_py {
        Some(content) => parse_ufo_particles(content)?,
        None => Vec::new(),
    };
    let vertices = match &files.vertices_py {
        Some(content) => parse_ufo_vertices(content)?,
        None => Vec::new(),
    };
    let couplings = match &files.couplings_py {
        Some(content) => parse_ufo_couplings(content)?,
        None => Vec::new(),
    };
    let parameters = match &files.parameters_py {
        Some(content) => parse_ufo_parameters(content)?,
        None => Vec::new(),
    };
    let lorentz_structures = match &files.lorentz_py {
        Some(content) => parse_ufo_lorentz(content)?,
        None => Vec::new(),
    };

    Ok(UfoModel {
        particles,
        vertices,
        couplings,
        parameters,
        lorentz_structures,
    })
}

// ---------------------------------------------------------------------------
// Conversion to TheoreticalModel
// ---------------------------------------------------------------------------

/// Convert a UFO spin value ($2S+1$) to a SPIRE `Spin` (twice the spin).
fn ufo_spin_to_spire(ufo_spin: i32) -> Spin {
    // UFO: 1=scalar, 2=fermion, 3=vector, 4=spin-3/2, 5=spin-2
    // SPIRE Spin(2*S): 0=scalar, 1=fermion, 2=vector, 3=spin-3/2, 4=spin-2
    let twice_spin = (ufo_spin - 1).max(0) as u8;
    Spin(twice_spin)
}

/// Convert a UFO color code to a SPIRE `ColorRepresentation`.
fn ufo_color_to_spire(ufo_color: i32) -> ColorRepresentation {
    match ufo_color {
        1 => ColorRepresentation::Singlet,
        3 => ColorRepresentation::Triplet,
        -3 => ColorRepresentation::AntiTriplet,
        8 => ColorRepresentation::Octet,
        // For exotic representations (6, -6, etc.) fall back to singlet.
        _ => ColorRepresentation::Singlet,
    }
}

/// Build the parameter value lookup table from `UfoParameter` definitions.
fn build_param_values(params: &[UfoParameter]) -> HashMap<String, f64> {
    let mut values = HashMap::new();
    values.insert("ZERO".to_string(), 0.0);

    for p in params {
        if let Some(val) = p.value {
            values.insert(p.name.clone(), val);
        }
    }

    values
}

/// Convert a `UfoModel` into a SPIRE `TheoreticalModel`.
///
/// Maps particles to `Field`s, vertices to `LagrangianTerm`s and
/// `VertexFactor`s.
///
/// # Arguments
///
/// * `ufo` — The parsed UFO model.
/// * `model_name` — Human-readable name for the model.
pub fn ufo_to_theoretical_model(ufo: &UfoModel, model_name: &str) -> SpireResult<TheoreticalModel> {
    let param_values = build_param_values(&ufo.parameters);

    // --- Convert particles to Fields ---
    let fields: Vec<Field> = ufo
        .particles
        .iter()
        .map(|p| {
            let mass = param_values.get(&p.mass_name).copied().unwrap_or(0.0);
            let width = param_values.get(&p.width_name).copied().unwrap_or(0.0);

            // Map charge: UFO gives float; SPIRE stores i8 in units of e/3.
            let charge_thirds = (p.charge * 3.0).round() as i8;

            Field {
                id: p.name.clone(),
                name: p.name.replace("__", " ").replace("_", ""),
                symbol: p.name.clone(),
                mass,
                width,
                quantum_numbers: QuantumNumbers {
                    electric_charge: ElectricCharge(charge_thirds),
                    weak_isospin: WeakIsospin(0),
                    hypercharge: Hypercharge(0),
                    baryon_number: BaryonNumber(0),
                    lepton_numbers: LeptonNumbers {
                        electron: 0,
                        muon: 0,
                        tau: 0,
                    },
                    spin: ufo_spin_to_spire(p.spin),
                    parity: Parity::Even,
                    charge_conjugation: ChargeConjugation::Undefined,
                    color: ufo_color_to_spire(p.color),
                    weak_multiplet: WeakMultiplet::Singlet,
                    representations: vec![],
                },
                interactions: vec![],
            }
        })
        .collect();

    // --- Convert vertices to LagrangianTerms + VertexFactors ---
    let mut terms = Vec::new();
    let mut vertex_factors = Vec::new();

    for (_idx, vtx) in ufo.vertices.iter().enumerate() {
        let term_id = format!("ufo_{}", vtx.name);

        // Determine coupling value from first coupling entry.
        let coupling_name = vtx.couplings.values().next().cloned().unwrap_or_default();
        let coupling_value = ufo
            .couplings
            .iter()
            .find(|c| c.name == coupling_name)
            .and_then(|_| None::<f64>); // Symbolic; no numeric value easily available.

        let interaction_type = infer_interaction_type(&vtx.particles, &ufo.particles);

        terms.push(LagrangianTerm {
            id: term_id.clone(),
            description: format!("UFO vertex {}", vtx.name),
            coupling_symbol: coupling_name.clone(),
            coupling_value,
            field_ids: vtx.particles.clone(),
            lorentz_structure: vtx.lorentz.first().cloned().unwrap_or_default(),
            interaction_type,
            term_kind: LagrangianTermKind::Interaction,
            operator_dimension: Some(4),
        });

        let lorentz_str = vtx.lorentz.join(" × ");
        vertex_factors.push(VertexFactor {
            term_id,
            field_ids: vtx.particles.clone(),
            expression: format!(
                "i {} [{}]",
                coupling_name,
                if lorentz_str.is_empty() {
                    "1".to_string()
                } else {
                    lorentz_str
                }
            ),
            coupling_value,
            n_legs: vtx.particles.len() as u8,
        });
    }

    Ok(TheoreticalModel {
        name: model_name.to_string(),
        description: format!(
            "Model imported from UFO format ({} particles, {} vertices)",
            ufo.particles.len(),
            ufo.vertices.len()
        ),
        fields,
        terms,
        vertex_factors,
        propagators: vec![], // Propagators derived separately from fields.
        gauge_symmetry: None,
        spacetime: Default::default(),
        constants: Default::default(),
    })
}

/// Heuristic: infer the `InteractionType` from the particles at a vertex.
fn infer_interaction_type(
    particle_names: &[String],
    all_particles: &[UfoParticle],
) -> InteractionType {
    let has_gluon = particle_names
        .iter()
        .any(|n| all_particles.iter().any(|p| &p.name == n && p.color == 8));
    let has_photon = particle_names.iter().any(|n| n == "a" || n == "A");
    let has_w = particle_names.iter().any(|n| n.contains("W"));
    let has_z = particle_names.iter().any(|n| n.contains("Z"));
    let has_higgs = particle_names
        .iter()
        .any(|n| n.contains("H") || n.contains("h"));

    if has_gluon {
        InteractionType::Strong
    } else if has_w {
        InteractionType::WeakCC
    } else if has_z {
        InteractionType::WeakNC
    } else if has_photon {
        InteractionType::Electromagnetic
    } else if has_higgs {
        InteractionType::Yukawa
    } else {
        InteractionType::Electromagnetic // Default fallback.
    }
}

// ---------------------------------------------------------------------------
// TheoryImporter Trait
// ---------------------------------------------------------------------------

/// Generic trait for importing complete theory models into SPIRE.
///
/// Implementors translate external model formats (UFO, FeynRules, SARAH)
/// into a SPIRE `TheoreticalModel`.
pub trait TheoryImporter {
    /// The configuration/input type for this importer.
    type Input;

    /// Import a theory model and return a `TheoreticalModel`.
    fn import(&self, input: &Self::Input) -> SpireResult<TheoreticalModel>;

    /// A human-readable name for this importer format.
    fn format_name(&self) -> &str;
}

/// UFO model importer implementing the [`TheoryImporter`] trait.
pub struct UfoImporter {
    /// Name to assign to the imported model.
    pub model_name: String,
}

impl TheoryImporter for UfoImporter {
    type Input = UfoFileContents;

    fn import(&self, input: &UfoFileContents) -> SpireResult<TheoreticalModel> {
        let ufo = parse_ufo_model(input)?;
        ufo_to_theoretical_model(&ufo, &self.model_name)
    }

    fn format_name(&self) -> &str {
        "Universal FeynRules Output (UFO)"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PARTICLES_PY: &str = r#"
# UFO particles.py -- Standard Model (subset)

a = Particle(pdg_code = 22, name = 'a', antiname = 'a', spin = 3, color = 1,
             mass = 'ZERO', width = 'ZERO', charge = 0, texname = 'A')

g = Particle(pdg_code = 21, name = 'g', antiname = 'g', spin = 3, color = 8,
             mass = 'ZERO', width = 'ZERO', charge = 0)

e__minus__ = Particle(pdg_code = 11, name = 'e__minus__', antiname = 'e__plus__',
                      spin = 2, color = 1, mass = 'Me', width = 'ZERO', charge = -1)

u = Particle(pdg_code = 2, name = 'u', antiname = 'u__tilde__', spin = 2, color = 3,
             mass = 'MU', width = 'ZERO', charge = 2/3)

Z = Particle(pdg_code = 23, name = 'Z', antiname = 'Z', spin = 3, color = 1,
             mass = 'MZ', width = 'WZ', charge = 0)

H = Particle(pdg_code = 25, name = 'H', antiname = 'H', spin = 1, color = 1,
             mass = 'MH', width = 'WH', charge = 0)
"#;

    const TEST_COUPLINGS_PY: &str = r#"
GC_1 = Coupling(name = 'GC_1', value = '-(ee*complex(0,1))/3.', order = {'QED':1})
GC_2 = Coupling(name = 'GC_2', value = '(2*ee*complex(0,1))/3.', order = {'QED':1})
GC_3 = Coupling(name = 'GC_3', value = '-(ee*complex(0,1))', order = {'QED':1})
GC_10 = Coupling(name = 'GC_10', value = '-G', order = {'QCD':1})
GC_50 = Coupling(name = 'GC_50', value = '-(yb/cmath.sqrt(2))', order = {'QED':1})
"#;

    const TEST_PARAMETERS_PY: &str = r#"
aS = Parameter(name = 'aS', nature = 'external', type = 'real', value = 0.1184,
               texname = '\\alpha_s', lhablock = 'SMINPUTS', lhacode = [3])
MZ = Parameter(name = 'MZ', nature = 'external', type = 'real', value = 91.1876,
               texname = 'M_Z', lhablock = 'MASS', lhacode = [23])
MH = Parameter(name = 'MH', nature = 'external', type = 'real', value = 125.09,
               texname = 'M_H', lhablock = 'MASS', lhacode = [25])
WZ = Parameter(name = 'WZ', nature = 'external', type = 'real', value = 2.4952,
               texname = '\\Gamma_Z', lhablock = 'DECAY', lhacode = [23])
WH = Parameter(name = 'WH', nature = 'external', type = 'real', value = 0.00407,
               texname = '\\Gamma_H', lhablock = 'DECAY', lhacode = [25])
Me = Parameter(name = 'Me', nature = 'external', type = 'real', value = 0.000511,
               texname = 'M_e', lhablock = 'MASS', lhacode = [11])
MU = Parameter(name = 'MU', nature = 'external', type = 'real', value = 0.00255,
               texname = 'M_u', lhablock = 'MASS', lhacode = [2])
"#;

    const TEST_LORENTZ_PY: &str = r#"
FFV1 = Lorentz(name = 'FFV1', spins = [2, 2, 3], structure = 'Gamma(3,2,1)')
SSS1 = Lorentz(name = 'SSS1', spins = [1, 1, 1], structure = '1')
VVV1 = Lorentz(name = 'VVV1', spins = [3, 3, 3], structure = 'P(3,1)*Metric(1,2)')
"#;

    const TEST_VERTICES_PY: &str = r#"
V_1 = Vertex(name = 'V_1', particles = [P.e__minus__, P.e__plus__, P.a],
             color = ['1'], lorentz = [L.FFV1],
             couplings = {(0,0):C.GC_3})

V_5 = Vertex(name = 'V_5', particles = [P.g, P.g, P.g],
             color = ['f(1,2,3)'], lorentz = [L.VVV1],
             couplings = {(0,0):C.GC_10})
"#;

    #[test]
    fn parse_ufo_particles_basic() {
        let particles = parse_ufo_particles(TEST_PARTICLES_PY).unwrap();

        assert!(
            particles.len() >= 4,
            "Expected at least 4 particles, got {}",
            particles.len()
        );

        let photon = particles.iter().find(|p| p.name == "a").unwrap();
        assert_eq!(photon.pdg_code, 22);
        assert_eq!(photon.spin, 3);
        assert_eq!(photon.color, 1);
        assert_eq!(photon.charge, 0.0);

        let gluon = particles.iter().find(|p| p.name == "g").unwrap();
        assert_eq!(gluon.pdg_code, 21);
        assert_eq!(gluon.color, 8);

        let electron = particles.iter().find(|p| p.name == "e__minus__").unwrap();
        assert_eq!(electron.pdg_code, 11);
        assert_eq!(electron.spin, 2);
        assert_eq!(electron.charge, -1.0);
        assert_eq!(electron.antiname, "e__plus__");

        let higgs = particles.iter().find(|p| p.name == "H").unwrap();
        assert_eq!(higgs.pdg_code, 25);
        assert_eq!(higgs.spin, 1); // scalar
    }

    #[test]
    fn parse_ufo_quark_charge() {
        let particles = parse_ufo_particles(TEST_PARTICLES_PY).unwrap();
        let u_quark = particles.iter().find(|p| p.name == "u").unwrap();
        assert_eq!(u_quark.pdg_code, 2);
        assert_eq!(u_quark.color, 3); // triplet
        assert!((u_quark.charge - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn parse_ufo_couplings_basic() {
        let couplings = parse_ufo_couplings(TEST_COUPLINGS_PY).unwrap();
        assert!(couplings.len() >= 3);

        let gc3 = couplings.iter().find(|c| c.name == "GC_3").unwrap();
        assert_eq!(gc3.value, "-(ee*complex(0,1))");
        assert_eq!(gc3.order.get("QED"), Some(&1));

        let gc10 = couplings.iter().find(|c| c.name == "GC_10").unwrap();
        assert_eq!(gc10.order.get("QCD"), Some(&1));
    }

    #[test]
    fn parse_ufo_parameters_basic() {
        let params = parse_ufo_parameters(TEST_PARAMETERS_PY).unwrap();
        assert!(params.len() >= 4);

        let mz = params.iter().find(|p| p.name == "MZ").unwrap();
        assert_eq!(mz.nature, "external");
        assert!((mz.value.unwrap() - 91.1876).abs() < 1e-4);
        assert_eq!(mz.lhablock.as_deref(), Some("MASS"));
        assert_eq!(mz.lhacode.as_deref(), Some(&[23][..]));
    }

    #[test]
    fn parse_ufo_lorentz_basic() {
        let lorentz = parse_ufo_lorentz(TEST_LORENTZ_PY).unwrap();
        assert!(lorentz.len() >= 2);

        let ffv1 = lorentz.iter().find(|l| l.name == "FFV1").unwrap();
        assert_eq!(ffv1.spins, vec![2, 2, 3]);
        assert_eq!(ffv1.structure, "Gamma(3,2,1)");
    }

    #[test]
    fn parse_ufo_vertices_basic() {
        let vertices = parse_ufo_vertices(TEST_VERTICES_PY).unwrap();
        assert!(vertices.len() >= 2);

        let v1 = vertices.iter().find(|v| v.name == "V_1").unwrap();
        assert_eq!(v1.particles, vec!["e__minus__", "e__plus__", "a"]);
        assert_eq!(v1.lorentz, vec!["FFV1"]);
        assert_eq!(v1.couplings.get(&(0, 0)), Some(&"GC_3".to_string()));
    }

    #[test]
    fn ufo_spin_mapping() {
        assert_eq!(ufo_spin_to_spire(1), Spin(0)); // scalar
        assert_eq!(ufo_spin_to_spire(2), Spin(1)); // fermion
        assert_eq!(ufo_spin_to_spire(3), Spin(2)); // vector
        assert_eq!(ufo_spin_to_spire(4), Spin(3)); // spin-3/2
        assert_eq!(ufo_spin_to_spire(5), Spin(4)); // spin-2
    }

    #[test]
    fn ufo_color_mapping() {
        assert_eq!(ufo_color_to_spire(1), ColorRepresentation::Singlet);
        assert_eq!(ufo_color_to_spire(3), ColorRepresentation::Triplet);
        assert_eq!(ufo_color_to_spire(-3), ColorRepresentation::AntiTriplet);
        assert_eq!(ufo_color_to_spire(8), ColorRepresentation::Octet);
        assert_eq!(ufo_color_to_spire(6), ColorRepresentation::Singlet); // fallback
    }

    #[test]
    fn full_ufo_model_parse() {
        let files = UfoFileContents {
            particles_py: Some(TEST_PARTICLES_PY.to_string()),
            vertices_py: Some(TEST_VERTICES_PY.to_string()),
            couplings_py: Some(TEST_COUPLINGS_PY.to_string()),
            parameters_py: Some(TEST_PARAMETERS_PY.to_string()),
            lorentz_py: Some(TEST_LORENTZ_PY.to_string()),
        };

        let model = parse_ufo_model(&files).unwrap();
        assert!(model.particles.len() >= 4);
        assert!(model.vertices.len() >= 2);
        assert!(model.couplings.len() >= 3);
    }

    #[test]
    fn ufo_to_theoretical_model_conversion() {
        let files = UfoFileContents {
            particles_py: Some(TEST_PARTICLES_PY.to_string()),
            vertices_py: Some(TEST_VERTICES_PY.to_string()),
            couplings_py: Some(TEST_COUPLINGS_PY.to_string()),
            parameters_py: Some(TEST_PARAMETERS_PY.to_string()),
            lorentz_py: Some(TEST_LORENTZ_PY.to_string()),
        };

        let ufo = parse_ufo_model(&files).unwrap();
        let model = ufo_to_theoretical_model(&ufo, "SM_UFO").unwrap();

        assert_eq!(model.name, "SM_UFO");
        assert!(!model.fields.is_empty());
        assert!(!model.terms.is_empty());
        assert!(!model.vertex_factors.is_empty());

        // Check Z boson mass was resolved from parameters.
        let z_field = model.fields.iter().find(|f| f.id == "Z").unwrap();
        assert!((z_field.mass - 91.1876).abs() < 1e-4);
        assert!((z_field.width - 2.4952).abs() < 1e-4);

        // Check Higgs.
        let h_field = model.fields.iter().find(|f| f.id == "H").unwrap();
        assert!((h_field.mass - 125.09).abs() < 1e-2);
    }

    #[test]
    fn ufo_importer_trait() {
        let importer = UfoImporter {
            model_name: "Test_UFO".to_string(),
        };
        assert_eq!(importer.format_name(), "Universal FeynRules Output (UFO)");

        let files = UfoFileContents {
            particles_py: Some(TEST_PARTICLES_PY.to_string()),
            parameters_py: Some(TEST_PARAMETERS_PY.to_string()),
            ..Default::default()
        };

        let model = importer.import(&files).unwrap();
        assert_eq!(model.name, "Test_UFO");
    }

    #[test]
    fn empty_ufo_model() {
        let files = UfoFileContents::default();
        let model = parse_ufo_model(&files).unwrap();
        assert!(model.particles.is_empty());
        assert!(model.vertices.is_empty());
    }

    #[test]
    fn kwarg_parser_handles_nested() {
        let body = "name = 'test', value = '-(ee*complex(0,1))', order = {'QED':1}";
        let kwargs = parse_kwargs(body);
        assert_eq!(kwargs.len(), 3);
        assert_eq!(kwargs[0].0, "name");
        assert_eq!(kwargs[0].1, "'test'");
    }

    #[test]
    fn strip_quotes_works() {
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("\"world\""), "world");
        assert_eq!(strip_quotes("no_quotes"), "no_quotes");
    }

    #[test]
    fn parse_py_number_fractions() {
        assert!((parse_py_number("2/3").unwrap() - 2.0 / 3.0).abs() < 1e-10);
        assert!((parse_py_number("-1").unwrap() - (-1.0)).abs() < 1e-10);
        assert!((parse_py_number("91.1876").unwrap() - 91.1876).abs() < 1e-10);
    }
}
