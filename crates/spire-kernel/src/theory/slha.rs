//! # SLHA (SUSY Les Houches Accord) Spectrum Parser
//!
//! Parses SLHA-format files (v1 and v2) into a structured `SlhaDocument`,
//! then merges the mass spectrum, decay widths, and mixing matrices into a
//! [`TheoreticalModel`].
//!
//! ## Format Overview
//!
//! SLHA is a plain-text, line-oriented format used by spectrum generators
//! (SPheno, SoftSUSY, ISAJET) and Monte Carlo tools. Key blocks:
//!
//! - `BLOCK MASS` — PDG ID → pole mass mapping
//! - `BLOCK NMIX`, `UMIX`, `VMIX`, `STOPMIX`, … — mixing matrices ($N_{ij}$)
//! - `DECAY <pdg_id> <width>` — decay table with branching ratios
//!
//! Comments begin with `#` and extend to end of line.
//!
//! ## References
//!
//! - P. Skands *et al.*, "SUSY Les Houches Accord", JHEP **0407** (2004) 036
//! - B. Allanach *et al.*, "SLHA2", Comput. Phys. Commun. **180** (2009) 8

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lagrangian::TheoreticalModel;
use crate::{SpireError, SpireResult};

// ---------------------------------------------------------------------------
// Data Structures
// ---------------------------------------------------------------------------

/// A single entry in an SLHA block.
///
/// Indices are stored as a vector of integers (length 1 for simple blocks
/// like `MASS`, length 2 for matrix blocks like `NMIX`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlhaEntry {
    /// Integer indices (e.g., `[25]` for Higgs mass, `[1, 2]` for $N_{12}$).
    pub indices: Vec<i32>,
    /// Numerical value.
    pub value: f64,
    /// Trailing comment (if any).
    pub comment: Option<String>,
}

/// A named SLHA block (e.g., `MASS`, `NMIX`, `ALPHA`, `HMIX`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlhaBlock {
    /// Block name in uppercase (e.g., `"MASS"`, `"NMIX"`).
    pub name: String,
    /// Optional scale $Q$ at which the block is evaluated (in GeV).
    pub scale: Option<f64>,
    /// Entries indexed by their integer key tuple.
    pub entries: HashMap<Vec<i32>, f64>,
    /// Comments associated with entries (same key).
    pub comments: HashMap<Vec<i32>, String>,
}

/// A single decay channel: branching ratio and daughter PDG IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayChannel {
    /// Branching ratio (0.0–1.0).
    pub branching_ratio: f64,
    /// Number of daughter particles.
    pub n_daughters: u8,
    /// PDG codes of the daughter particles.
    pub daughter_pdg_ids: Vec<i32>,
    /// Trailing comment (if any).
    pub comment: Option<String>,
}

/// A `DECAY` block for a single particle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlhaDecay {
    /// PDG ID of the decaying particle.
    pub pdg_id: i32,
    /// Total decay width in GeV.
    pub total_width: f64,
    /// List of decay channels.
    pub channels: Vec<DecayChannel>,
}

/// A complete parsed SLHA document.
///
/// Contains all blocks and decay tables found in the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlhaDocument {
    /// Named blocks (key = uppercase block name).
    pub blocks: HashMap<String, SlhaBlock>,
    /// Decay tables (key = PDG ID).
    pub decays: HashMap<i32, SlhaDecay>,
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse state machine for the line-by-line SLHA reader.
enum ParseContext {
    /// Not inside any block.
    None,
    /// Inside a `BLOCK <name>`.
    Block(String),
    /// Inside a `DECAY <pdg_id> <width>`.
    Decay(i32),
}

/// Parse an SLHA-format string into a structured `SlhaDocument`.
///
/// # Errors
///
/// Returns `SpireError::ModelParseError` if the input contains malformed
/// lines (e.g., non-numeric values where numbers are expected).
///
/// # Example
///
/// ```
/// use spire_kernel::theory::slha::parse_slha;
///
/// let input = r#"
/// BLOCK MASS   # Mass Spectrum
///    6    1.73200000E+02  # Mt
///   25    1.25000000E+02  # Mh
///
/// DECAY  25   4.07000000E-03  # Gamma(h)
///    2  2.23000000E-01   5  -5  # BR(h -> b bbar)
/// "#;
///
/// let doc = parse_slha(input).unwrap();
/// assert!(doc.blocks.contains_key("MASS"));
/// assert_eq!(doc.blocks["MASS"].entries[&vec![25]], 1.25e2);
/// assert!(doc.decays.contains_key(&25));
/// ```
pub fn parse_slha(input: &str) -> SpireResult<SlhaDocument> {
    let mut blocks: HashMap<String, SlhaBlock> = HashMap::new();
    let mut decays: HashMap<i32, SlhaDecay> = HashMap::new();
    let mut context = ParseContext::None;

    for (line_num, raw_line) in input.lines().enumerate() {
        // Strip comments: everything after '#' is a comment.
        let comment_text = raw_line
            .find('#')
            .map(|pos| raw_line[pos + 1..].trim().to_string());
        let line = match raw_line.find('#') {
            Some(pos) => raw_line[..pos].trim(),
            None => raw_line.trim(),
        };

        if line.is_empty() {
            continue;
        }

        let upper = line.to_uppercase();

        // --- BLOCK header ---
        if upper.starts_with("BLOCK") {
            let rest = line[5..].trim();
            let mut parts = rest.split_whitespace();
            let block_name = parts
                .next()
                .ok_or_else(|| {
                    SpireError::ModelParseError(format!(
                        "Line {}: BLOCK without name",
                        line_num + 1
                    ))
                })?
                .to_uppercase();

            // Optional scale Q=<value>
            let scale = parse_block_scale(rest);

            blocks.entry(block_name.clone()).or_insert_with(|| SlhaBlock {
                name: block_name.clone(),
                scale,
                entries: HashMap::new(),
                comments: HashMap::new(),
            });

            context = ParseContext::Block(block_name);
            continue;
        }

        // --- DECAY header ---
        if upper.starts_with("DECAY") {
            let rest = line[5..].trim();
            let mut parts = rest.split_whitespace();

            let pdg_id: i32 = parts
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| {
                    SpireError::ModelParseError(format!(
                        "Line {}: DECAY without valid PDG ID",
                        line_num + 1
                    ))
                })?;

            let width: f64 = parts
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| {
                    SpireError::ModelParseError(format!(
                        "Line {}: DECAY without valid width",
                        line_num + 1
                    ))
                })?;

            decays.entry(pdg_id).or_insert_with(|| SlhaDecay {
                pdg_id,
                total_width: width,
                channels: Vec::new(),
            });

            context = ParseContext::Decay(pdg_id);
            continue;
        }

        // --- Entry lines (depend on context) ---
        match &context {
            ParseContext::None => {
                // Ignore lines outside any block (e.g., stray header comments).
            }
            ParseContext::Block(block_name) => {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.is_empty() {
                    continue;
                }

                // The last token is the value; all preceding tokens are indices.
                if tokens.len() < 2 {
                    // Single-value blocks (e.g., BLOCK ALPHA with one entry)
                    if let Ok(val) = tokens[0].parse::<f64>() {
                        if let Some(block) = blocks.get_mut(block_name) {
                            block.entries.insert(vec![], val);
                            if let Some(c) = &comment_text {
                                block.comments.insert(vec![], c.clone());
                            }
                        }
                    }
                    continue;
                }

                let value: f64 = tokens.last().unwrap().parse().map_err(|_| {
                    SpireError::ModelParseError(format!(
                        "Line {}: non-numeric value '{}'",
                        line_num + 1,
                        tokens.last().unwrap()
                    ))
                })?;

                let indices: Result<Vec<i32>, _> = tokens[..tokens.len() - 1]
                    .iter()
                    .map(|s| s.parse::<i32>())
                    .collect();

                let indices = indices.map_err(|_| {
                    SpireError::ModelParseError(format!(
                        "Line {}: non-integer index in block {}",
                        line_num + 1,
                        block_name
                    ))
                })?;

                if let Some(block) = blocks.get_mut(block_name) {
                    block.entries.insert(indices.clone(), value);
                    if let Some(c) = &comment_text {
                        block.comments.insert(indices, c.clone());
                    }
                }
            }
            ParseContext::Decay(pdg_id) => {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.len() < 3 {
                    continue; // Need at least: n_daughters BR daughter1
                }

                // Format: BR  n_daughters  pdg1  pdg2  ...
                // Note: some SLHA files have n_daughters first, others have BR first.
                // Standard format: n_daughters BR pdg1 pdg2 ...
                // But many generators use: BR n_daughters pdg1 pdg2 ...
                // We detect based on whether first token is integer or float.

                let (branching_ratio, n_daughters, daughter_start) =
                    parse_decay_line(&tokens, line_num)?;

                let daughter_pdg_ids: Result<Vec<i32>, _> = tokens[daughter_start..]
                    .iter()
                    .map(|s| s.parse::<i32>())
                    .collect();

                let daughter_pdg_ids = daughter_pdg_ids.map_err(|_| {
                    SpireError::ModelParseError(format!(
                        "Line {}: non-integer PDG ID in decay channel",
                        line_num + 1
                    ))
                })?;

                if let Some(decay) = decays.get_mut(pdg_id) {
                    decay.channels.push(DecayChannel {
                        branching_ratio,
                        n_daughters,
                        daughter_pdg_ids,
                        comment: comment_text.clone(),
                    });
                }
            }
        }
    }

    Ok(SlhaDocument { blocks, decays })
}

/// Parse the optional scale `Q=<value>` from a BLOCK header line.
fn parse_block_scale(rest: &str) -> Option<f64> {
    let upper = rest.to_uppercase();
    if let Some(pos) = upper.find("Q=") {
        let after_q = rest[pos + 2..].trim();
        after_q.split_whitespace().next()?.parse().ok()
    } else {
        None
    }
}

/// Parse a decay channel line, handling both common SLHA orderings.
///
/// Returns `(branching_ratio, n_daughters, daughter_start_index)`.
fn parse_decay_line(tokens: &[&str], line_num: usize) -> SpireResult<(f64, u8, usize)> {
    // Standard SLHA format: BR  n_daughters  pdg1  pdg2  ...
    // where BR is a float like 2.23000000E-01 and n_daughters is a small int.

    // Try: first token is BR (float), second is n_daughters (int)
    if let Ok(br) = tokens[0].parse::<f64>() {
        if let Ok(nd) = tokens[1].parse::<u8>() {
            // Heuristic: if nd <= 10 and matches remaining token count, it's n_daughters
            if nd as usize <= 10 && tokens.len() >= 2 + nd as usize {
                return Ok((br, nd, 2));
            }
        }
    }

    // Fallback: first token is n_daughters, second is BR
    if let Ok(nd) = tokens[0].parse::<u8>() {
        if let Ok(br) = tokens[1].parse::<f64>() {
            return Ok((br, nd, 2));
        }
    }

    Err(SpireError::ModelParseError(format!(
        "Line {}: cannot parse decay channel format",
        line_num + 1
    )))
}

// ---------------------------------------------------------------------------
// SlhaDocument API
// ---------------------------------------------------------------------------

impl SlhaDocument {
    /// Retrieve a single value from a block by its integer indices.
    ///
    /// # Example
    ///
    /// ```
    /// use spire_kernel::theory::slha::parse_slha;
    ///
    /// let doc = parse_slha("BLOCK MASS\n  25  125.0\n").unwrap();
    /// assert_eq!(doc.get_block_entry("MASS", &[25]), Some(125.0));
    /// ```
    pub fn get_block_entry(&self, block_name: &str, indices: &[i32]) -> Option<f64> {
        self.blocks
            .get(&block_name.to_uppercase())
            .and_then(|b| b.entries.get(&indices.to_vec()).copied())
    }

    /// Retrieve a row of a mixing matrix block (e.g., row 1 of `NMIX`).
    ///
    /// Returns pairs of `(column_index, value)` sorted by column.
    pub fn get_matrix_row(&self, block_name: &str, row: i32) -> Vec<(i32, f64)> {
        let block = match self.blocks.get(&block_name.to_uppercase()) {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut entries: Vec<(i32, f64)> = block
            .entries
            .iter()
            .filter(|(k, _)| k.len() == 2 && k[0] == row)
            .map(|(k, &v)| (k[1], v))
            .collect();

        entries.sort_by_key(|(col, _)| *col);
        entries
    }

    /// Extract the full mixing matrix from a block as a row-major 2D array.
    ///
    /// Returns `(dimension, matrix_elements)` where `dimension` is the
    /// inferred matrix size and `matrix_elements` is a flat `dim × dim` vector.
    pub fn get_mixing_matrix(&self, block_name: &str) -> Option<(usize, Vec<f64>)> {
        let block = self.blocks.get(&block_name.to_uppercase())?;

        // Determine dimension from maximum index.
        let max_idx = block
            .entries
            .keys()
            .filter(|k| k.len() == 2)
            .flat_map(|k| k.iter())
            .max()?;

        let dim = *max_idx as usize;
        let mut matrix = vec![0.0; dim * dim];

        for (indices, &value) in &block.entries {
            if indices.len() == 2 {
                let i = (indices[0] - 1) as usize; // SLHA uses 1-based indexing
                let j = (indices[1] - 1) as usize;
                if i < dim && j < dim {
                    matrix[i * dim + j] = value;
                }
            }
        }

        Some((dim, matrix))
    }

    /// Get the mass of a particle by its PDG code from the `MASS` block.
    pub fn get_mass(&self, pdg_id: i32) -> Option<f64> {
        self.get_block_entry("MASS", &[pdg_id])
    }

    /// Get the total decay width of a particle by PDG code.
    pub fn get_decay_width(&self, pdg_id: i32) -> Option<f64> {
        self.decays.get(&pdg_id).map(|d| d.total_width)
    }

    /// List all PDG IDs present in the `MASS` block.
    pub fn mass_pdg_ids(&self) -> Vec<i32> {
        match self.blocks.get("MASS") {
            Some(block) => block
                .entries
                .keys()
                .filter(|k| k.len() == 1)
                .map(|k| k[0])
                .collect(),
            None => Vec::new(),
        }
    }

    /// List all block names present in the document.
    pub fn block_names(&self) -> Vec<&str> {
        self.blocks.keys().map(|s| s.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Model Integration
// ---------------------------------------------------------------------------

/// Merge an SLHA spectrum into an existing `TheoreticalModel`.
///
/// Updates the masses and widths of fields whose PDG codes match entries
/// in the `MASS` block and `DECAY` tables. Fields are matched by their
/// `id` field, which should correspond to a stringified PDG code or a
/// known particle name. A `pdg_map` provides the mapping from PDG IDs
/// to field IDs.
///
/// # Returns
///
/// A new `TheoreticalModel` with updated masses and widths, plus a
/// summary of what was changed.
pub fn merge_slha_into_model(
    doc: &SlhaDocument,
    model: &TheoreticalModel,
    pdg_map: &HashMap<i32, String>,
) -> SpireResult<(TheoreticalModel, SlhaMergeSummary)> {
    let mut merged = model.clone();
    let mut summary = SlhaMergeSummary {
        masses_updated: Vec::new(),
        widths_updated: Vec::new(),
        unmatched_pdg_ids: Vec::new(),
    };

    // Update masses from MASS block.
    if let Some(mass_block) = doc.blocks.get("MASS") {
        for (indices, &mass) in &mass_block.entries {
            if indices.len() == 1 {
                let pdg_id = indices[0];
                if let Some(field_id) = pdg_map.get(&pdg_id) {
                    if let Some(field) = merged.fields.iter_mut().find(|f| &f.id == field_id) {
                        field.mass = mass.abs(); // SLHA masses can be negative (sign = CP phase)
                        summary.masses_updated.push((pdg_id, field_id.clone(), mass));
                    } else {
                        summary.unmatched_pdg_ids.push(pdg_id);
                    }
                } else {
                    summary.unmatched_pdg_ids.push(pdg_id);
                }
            }
        }
    }

    // Update widths from DECAY blocks.
    for (&pdg_id, decay) in &doc.decays {
        if let Some(field_id) = pdg_map.get(&pdg_id) {
            if let Some(field) = merged.fields.iter_mut().find(|f| &f.id == field_id) {
                field.width = decay.total_width;
                summary
                    .widths_updated
                    .push((pdg_id, field_id.clone(), decay.total_width));
            }
        }
    }

    // Update propagator masses and widths.
    for prop in merged.propagators.iter_mut() {
        if let Some(field) = merged.fields.iter().find(|f| f.id == prop.field_id) {
            prop.mass = field.mass;
            prop.width = field.width;
        }
    }

    Ok((merged, summary))
}

/// Summary of changes made by [`merge_slha_into_model`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlhaMergeSummary {
    /// `(pdg_id, field_id, new_mass)` for each updated mass.
    pub masses_updated: Vec<(i32, String, f64)>,
    /// `(pdg_id, field_id, new_width)` for each updated width.
    pub widths_updated: Vec<(i32, String, f64)>,
    /// PDG IDs found in SLHA but not matched to any field.
    pub unmatched_pdg_ids: Vec<i32>,
}

// ---------------------------------------------------------------------------
// SpectrumImporter Trait
// ---------------------------------------------------------------------------

/// Generic trait for importing external spectrum data into SPIRE.
///
/// Implementors parse a specific format (SLHA, SLHA2, custom) and produce
/// a `TheoreticalModel` or modify an existing one.
pub trait SpectrumImporter {
    /// Import spectrum data from a string and return a modified `TheoreticalModel`.
    fn import_from_str(
        &self,
        input: &str,
        base_model: &TheoreticalModel,
    ) -> SpireResult<TheoreticalModel>;

    /// A human-readable name for this importer (e.g., "SLHA v1").
    fn format_name(&self) -> &str;
}

/// SLHA v1 spectrum importer implementing the [`SpectrumImporter`] trait.
pub struct SlhaImporter {
    /// Mapping from PDG IDs to field IDs in the target model.
    pub pdg_map: HashMap<i32, String>,
}

impl SpectrumImporter for SlhaImporter {
    fn import_from_str(
        &self,
        input: &str,
        base_model: &TheoreticalModel,
    ) -> SpireResult<TheoreticalModel> {
        let doc = parse_slha(input)?;
        let (model, _summary) = merge_slha_into_model(&doc, base_model, &self.pdg_map)?;
        Ok(model)
    }

    fn format_name(&self) -> &str {
        "SLHA v1"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Full MSSM-like SLHA spectrum string for testing.
    const TEST_SLHA: &str = r#"
# SUSY Les Houches Accord - MSSM spectrum
# SPheno v4.0.5
BLOCK MODSEL  # Model selection
   1    1      # mSUGRA

BLOCK SMINPUTS  # Standard Model inputs
   1    1.27934000E+02  # alpha_em^-1(MZ) SM MSbar
   2    1.16637000E-05  # G_Fermi
   3    1.17200000E-01  # alpha_s(MZ) SM MSbar
   4    9.11876000E+01  # MZ(pole)
   5    4.25000000E+00  # Mb(mb) SM MSbar
   6    1.73200000E+02  # Mtop(pole)
   7    1.77700000E+00  # Mtau(pole)

BLOCK MASS  # Mass spectrum
#  PDG code      mass        particle
    6    1.73200000E+02  # Mt
   24    8.03970000E+01  # MW
   25    1.25090000E+02  # h0
   35    1.50000000E+03  # H0
   36    1.50000000E+03  # A0
   37    1.50300000E+03  # H+
 1000001  5.68441109E+02  # ~d_L
 1000002  5.61119014E+02  # ~u_L
 1000006  3.99668493E+02  # ~t_1
 1000021  6.07713704E+02  # ~g
 1000022  9.71956453E+01  # ~chi_10
 1000023  1.80337040E+02  # ~chi_20
 1000025 -3.63756027E+02  # ~chi_30
 1000035  3.81729382E+02  # ~chi_40

BLOCK NMIX  # Neutralino mixing matrix
   1  1     9.86371540E-01  # N_11
   1  2    -5.31103553E-02  # N_12
   1  3     1.46345439E-01  # N_13
   1  4    -5.31186117E-02  # N_14
   2  1     9.93505358E-02  # N_21
   2  2     9.44681685E-01  # N_22
   2  3    -2.69846720E-01  # N_23
   2  4     1.56150698E-01  # N_24
   3  1    -6.03388002E-02  # N_31
   3  2     8.77461387E-02  # N_32
   3  3     6.95956432E-01  # N_33
   3  4     7.10227009E-01  # N_34
   4  1    -1.16507132E-01  # N_41
   4  2     3.12305454E-01  # N_42
   4  3     6.44263088E-01  # N_43
   4  4    -6.84377823E-01  # N_44

BLOCK HMIX Q=  4.67034270E+02  # Higgs mixing parameters
   1    3.57680977E+02  # mu(Q)
   2    9.75139550E+00  # tan(beta)(Q)
   3    2.44923506E+02  # v(Q)
   4    2.24352080E+06  # m_A^2(Q)

DECAY  6    1.49100000E+00  # Gamma(t)

DECAY  25   4.07000000E-03  # Gamma(h0)
   2  2.23000000E-01   5  -5  # BR(h -> b bbar)
   2  1.57000000E-02  15 -15  # BR(h -> tau+ tau-)
   2  6.32000000E-01  24 -24  # BR(h -> W+ W-)
   2  8.57000000E-02  23  23  # BR(h -> Z Z)
   2  2.28000000E-03  22  22  # BR(h -> gamma gamma)
   2  3.94000000E-02  21  21  # BR(h -> g g)

DECAY  1000022  0.00000000E+00  # Gamma(~chi_10) -- stable LSP
"#;

    #[test]
    fn parse_slha_blocks() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        // Check MASS block exists.
        assert!(doc.blocks.contains_key("MASS"));
        let mass_block = &doc.blocks["MASS"];

        // Top quark mass.
        assert!((mass_block.entries[&vec![6]] - 173.2).abs() < 1e-6);

        // Higgs mass.
        assert!((mass_block.entries[&vec![25]] - 125.09).abs() < 1e-6);

        // Stop squark mass.
        assert!((mass_block.entries[&vec![1000006]] - 399.668493).abs() < 1e-3);

        // Negative mass (CP phase) — neutralino 3.
        assert!((mass_block.entries[&vec![1000025]] - (-363.756027)).abs() < 1e-3);
    }

    #[test]
    fn parse_slha_sm_inputs() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        assert!(doc.blocks.contains_key("SMINPUTS"));
        let sm = &doc.blocks["SMINPUTS"];

        // alpha_em^-1(MZ)
        assert!((sm.entries[&vec![1]] - 127.934).abs() < 1e-3);

        // alpha_s(MZ)
        assert!((sm.entries[&vec![3]] - 0.1172).abs() < 1e-4);

        // MZ(pole)
        assert!((sm.entries[&vec![4]] - 91.1876).abs() < 1e-4);
    }

    #[test]
    fn parse_slha_mixing_matrix() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        assert!(doc.blocks.contains_key("NMIX"));

        // N_11 ≈ 0.986
        assert!((doc.get_block_entry("NMIX", &[1, 1]).unwrap() - 0.986371540).abs() < 1e-6);

        // N_23 ≈ -0.270
        assert!((doc.get_block_entry("NMIX", &[2, 3]).unwrap() - (-0.269846720)).abs() < 1e-6);

        // Full matrix extraction.
        let (dim, matrix) = doc.get_mixing_matrix("NMIX").unwrap();
        assert_eq!(dim, 4);
        assert_eq!(matrix.len(), 16);
        // N_44 ≈ -0.684
        assert!((matrix[3 * 4 + 3] - (-0.684377823)).abs() < 1e-6);
    }

    #[test]
    fn parse_slha_matrix_row() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        let row1 = doc.get_matrix_row("NMIX", 1);
        assert_eq!(row1.len(), 4);
        assert_eq!(row1[0].0, 1); // column 1
        assert!((row1[0].1 - 0.986371540).abs() < 1e-6);
    }

    #[test]
    fn parse_slha_decays() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        // Top decay.
        assert!(doc.decays.contains_key(&6));
        assert!((doc.decays[&6].total_width - 1.491).abs() < 1e-6);

        // Higgs decays.
        assert!(doc.decays.contains_key(&25));
        let higgs = &doc.decays[&25];
        assert!((higgs.total_width - 4.07e-3).abs() < 1e-6);
        assert_eq!(higgs.channels.len(), 6);

        // h -> b bbar.
        let bb_channel = &higgs.channels[0];
        assert!((bb_channel.branching_ratio - 0.223).abs() < 1e-6);
        assert_eq!(bb_channel.n_daughters, 2);
        assert_eq!(bb_channel.daughter_pdg_ids, vec![5, -5]);

        // h -> gamma gamma.
        let gg_channel = &higgs.channels[4];
        assert!((gg_channel.branching_ratio - 2.28e-3).abs() < 1e-6);
        assert_eq!(gg_channel.daughter_pdg_ids, vec![22, 22]);

        // Stable LSP (zero width).
        assert!(doc.decays.contains_key(&1000022));
        assert_eq!(doc.decays[&1000022].total_width, 0.0);
        assert_eq!(doc.decays[&1000022].channels.len(), 0);
    }

    #[test]
    fn slha_convenience_methods() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        // get_mass
        assert!((doc.get_mass(25).unwrap() - 125.09).abs() < 1e-6);
        assert!(doc.get_mass(9999999).is_none());

        // get_decay_width
        assert!((doc.get_decay_width(25).unwrap() - 4.07e-3).abs() < 1e-6);
        assert!(doc.get_decay_width(9999999).is_none());

        // mass_pdg_ids
        let ids = doc.mass_pdg_ids();
        assert!(ids.contains(&6));
        assert!(ids.contains(&25));
        assert!(ids.contains(&1000006));
    }

    #[test]
    fn parse_slha_block_with_scale() {
        let doc = parse_slha(TEST_SLHA).unwrap();

        let hmix = doc.blocks.get("HMIX").unwrap();
        assert!(hmix.scale.is_some());
        assert!((hmix.scale.unwrap() - 467.034270).abs() < 1e-3);

        // mu(Q)
        assert!((hmix.entries[&vec![1]] - 357.680977).abs() < 1e-3);
    }

    #[test]
    fn parse_empty_slha() {
        let doc = parse_slha("").unwrap();
        assert!(doc.blocks.is_empty());
        assert!(doc.decays.is_empty());
    }

    #[test]
    fn parse_slha_comments_only() {
        let doc = parse_slha("# Just a comment\n# Another line\n").unwrap();
        assert!(doc.blocks.is_empty());
    }

    #[test]
    fn merge_slha_updates_masses() {
        let slha_str = "BLOCK MASS\n  25  126.0\nDECAY  25  5.0E-03\n";
        let doc = parse_slha(slha_str).unwrap();

        let model = TheoreticalModel {
            name: "Test".into(),
            description: "".into(),
            fields: vec![crate::ontology::Field {
                id: "higgs".into(),
                name: "Higgs".into(),
                symbol: "h".into(),
                mass: 125.0,
                width: 0.004,
                quantum_numbers: default_qn(),
                interactions: vec![],
            }],
            terms: vec![],
            vertex_factors: vec![],
            propagators: vec![],
            gauge_symmetry: None,
            spacetime: Default::default(),
            constants: Default::default(),
        };

        let mut pdg_map = HashMap::new();
        pdg_map.insert(25, "higgs".into());

        let (merged, summary) = merge_slha_into_model(&doc, &model, &pdg_map).unwrap();

        assert!((merged.fields[0].mass - 126.0).abs() < 1e-6);
        assert!((merged.fields[0].width - 5.0e-3).abs() < 1e-6);
        assert_eq!(summary.masses_updated.len(), 1);
        assert_eq!(summary.widths_updated.len(), 1);
        assert!(summary.unmatched_pdg_ids.is_empty());
    }

    #[test]
    fn merge_slha_tracks_unmatched() {
        let slha_str = "BLOCK MASS\n  999999  100.0\n";
        let doc = parse_slha(slha_str).unwrap();

        let model = TheoreticalModel {
            name: "Test".into(),
            description: "".into(),
            fields: vec![],
            terms: vec![],
            vertex_factors: vec![],
            propagators: vec![],
            gauge_symmetry: None,
            spacetime: Default::default(),
            constants: Default::default(),
        };

        let pdg_map = HashMap::new();
        let (_merged, summary) = merge_slha_into_model(&doc, &model, &pdg_map).unwrap();

        assert_eq!(summary.unmatched_pdg_ids, vec![999999]);
    }

    #[test]
    fn slha_importer_trait() {
        let slha_str = "BLOCK MASS\n  25  126.0\n";

        let mut pdg_map = HashMap::new();
        pdg_map.insert(25, "higgs".into());

        let importer = SlhaImporter { pdg_map };
        assert_eq!(importer.format_name(), "SLHA v1");

        let model = TheoreticalModel {
            name: "Test".into(),
            description: "".into(),
            fields: vec![crate::ontology::Field {
                id: "higgs".into(),
                name: "Higgs".into(),
                symbol: "h".into(),
                mass: 125.0,
                width: 0.0,
                quantum_numbers: default_qn(),
                interactions: vec![],
            }],
            terms: vec![],
            vertex_factors: vec![],
            propagators: vec![],
            gauge_symmetry: None,
            spacetime: Default::default(),
            constants: Default::default(),
        };

        let result = importer.import_from_str(slha_str, &model).unwrap();
        assert!((result.fields[0].mass - 126.0).abs() < 1e-6);
    }

    /// Helper: create default quantum numbers for test fields.
    fn default_qn() -> crate::ontology::QuantumNumbers {
        use crate::ontology::*;
        QuantumNumbers {
            electric_charge: ElectricCharge(0),
            weak_isospin: WeakIsospin(0),
            hypercharge: Hypercharge(0),
            baryon_number: BaryonNumber(0),
            lepton_numbers: LeptonNumbers {
                electron: 0,
                muon: 0,
                tau: 0,
            },
            spin: Spin(0),
            parity: Parity::Even,
            charge_conjugation: ChargeConjugation::Even,
            color: ColorRepresentation::Singlet,
            weak_multiplet: WeakMultiplet::Singlet,
            representations: vec![],
        }
    }
}
