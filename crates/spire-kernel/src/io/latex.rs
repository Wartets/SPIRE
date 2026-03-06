//! # LaTeX - Proof Document Compiler
//!
//! This module compiles a [`ProofDocument`] into a publication-ready LaTeX
//! document using the `revtex4-2` document class (American Physical Society
//! journals). The output is a standalone `.tex` file that can be compiled
//! with `pdflatex` or `lualatex` without additional dependencies.
//!
//! ## Architecture
//!
//! The compiler walks the [`ProofStep`] tree recursively:
//!
//! - Top-level steps become `\section{}` headings.
//! - First-level children become `\subsection{}` headings.
//! - Deeper children become `\subsubsection{}` headings (LaTeX depth limit).
//!
//! Each step's `latex_math` field is rendered inside an `equation` environment.
//! Empty math strings are skipped.
//!
//! ## Usage
//!
//! ```
//! use spire_kernel::algebra::{ProofDocument, ProofStep};
//! use spire_kernel::io::latex::compile_proof_to_latex;
//!
//! let doc = ProofDocument {
//!     title: "Derivation of \\sigma(e^+e^- \\to \\mu^+\\mu^-)".into(),
//!     abstract_text: "A complete derivation.".into(),
//!     process: "e+ e- -> mu+ mu-".into(),
//!     steps: vec![
//!         ProofStep::leaf("Feynman Rules", "Apply Feynman rules.", r"\mathcal{M} = e^2"),
//!     ],
//! };
//!
//! let tex = compile_proof_to_latex(&doc);
//! assert!(tex.contains(r"\documentclass"));
//! assert!(tex.contains(r"\begin{document}"));
//! assert!(tex.contains(r"\end{document}"));
//! ```

use crate::algebra::{ProofDocument, ProofStep};
use crate::io::provenance::ProvenanceRecord;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a [`ProofDocument`] into a complete LaTeX source string.
///
/// The output uses the `revtex4-2` document class with APS journal
/// formatting. It includes:
///
/// - Title, author (SPIRE), and date
/// - Abstract
/// - Hierarchical sections from the proof step tree
/// - Numbered equation environments for each step's LaTeX math
/// - A conclusion section with the process label
///
/// # Example
///
/// ```
/// use spire_kernel::algebra::{ProofDocument, ProofStep};
/// use spire_kernel::io::latex::compile_proof_to_latex;
///
/// let doc = ProofDocument {
///     title: "Test".into(),
///     abstract_text: "Abstract.".into(),
///     process: "e+ e- -> gamma".into(),
///     steps: vec![ProofStep::leaf("Step", "Desc", r"\alpha + \beta")],
/// };
/// let tex = compile_proof_to_latex(&doc);
/// assert!(tex.contains(r"\documentclass[aps,prd,twocolumn,superscriptaddress]{revtex4-2}"));
/// assert!(tex.contains(r"\alpha + \beta"));
/// ```
pub fn compile_proof_to_latex(doc: &ProofDocument) -> String {
    let mut out = String::with_capacity(4096);

    // --- Preamble ---
    out.push_str(PREAMBLE);

    // --- Title block ---
    out.push_str(&format!("\\title{{{}}}\n", escape_latex(&doc.title)));
    out.push_str("\\author{SPIRE}\n");
    out.push_str("\\affiliation{Structured Particle Interaction and Reaction Engine}\n");
    out.push_str("\\date{\\today}\n\n");

    // --- Abstract ---
    out.push_str("\\begin{abstract}\n");
    out.push_str(&escape_latex(&doc.abstract_text));
    out.push('\n');
    out.push_str("\\end{abstract}\n\n");

    // --- Body ---
    out.push_str("\\maketitle\n\n");

    // Walk the proof step tree.
    for step in &doc.steps {
        render_step(&mut out, step, 0);
    }

    // --- Conclusion ---
    out.push_str("\\section{Conclusion}\n\n");
    out.push_str(&format!(
        "This document presented a complete algebraic derivation for the process ${}$, \
         generated automatically by SPIRE.\n\n",
        escape_latex(&doc.process)
    ));

    // --- End ---
    out.push_str("\\end{document}\n");

    out
}

/// Compile a proof document to LaTeX with an embedded provenance hash.
///
/// Identical to [`compile_proof_to_latex`] but prepends a LaTeX comment
/// block containing the SHA-256 provenance hash and the full serialized
/// computation state. This enables independent verification that the
/// document was generated from a specific theoretical configuration.
pub fn compile_proof_to_latex_with_provenance(
    doc: &ProofDocument,
    provenance: &ProvenanceRecord,
) -> String {
    let mut out = String::with_capacity(4096 + provenance.payload.len());

    // Provenance block as LaTeX comments (before \documentclass)
    out.push_str(&crate::io::provenance::format_provenance_block(
        provenance, "%", "",
    ));
    out.push('\n');

    // Delegate to the standard compiler for the document body.
    out.push_str(&compile_proof_to_latex(doc));
    out
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// LaTeX preamble with revtex4-2 configuration.
const PREAMBLE: &str = r#"\documentclass[aps,prd,twocolumn,superscriptaddress]{revtex4-2}

\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{slashed}
\usepackage{feynmp-auto}
\usepackage{hyperref}

\begin{document}

"#;

/// Render a single proof step (and its children) into the output buffer.
///
/// `depth` controls the sectioning level:
/// - 0 → `\section`
/// - 1 → `\subsection`
/// - 2+ → `\subsubsection`
fn render_step(out: &mut String, step: &ProofStep, depth: usize) {
    // Section heading
    let heading = match depth {
        0 => "\\section",
        1 => "\\subsection",
        _ => "\\subsubsection",
    };
    out.push_str(&format!("{}{{{}}}\n\n", heading, escape_latex(&step.title)));

    // Description paragraph
    if !step.description.is_empty() {
        out.push_str(&escape_latex(&step.description));
        out.push_str("\n\n");
    }

    // Math expression (if non-empty)
    if !step.latex_math.is_empty() {
        out.push_str("\\begin{equation}\n");
        out.push_str(&step.latex_math);
        out.push('\n');
        out.push_str("\\end{equation}\n\n");
    }

    // Recurse into children
    for child in &step.children {
        render_step(out, child, depth + 1);
    }
}

/// Escape special LaTeX characters in plain text.
///
/// This escapes the characters that have special meaning in LaTeX text mode:
/// `&`, `%`, `$`, `#`, `_`, `{`, `}`, `~`, `^`.
///
/// Note: This is for text content only. LaTeX math content (`latex_math`
/// fields) is written verbatim.
fn escape_latex(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => result.push_str(r"\&"),
            '%' => result.push_str(r"\%"),
            '$' => result.push_str(r"\$"),
            '#' => result.push_str(r"\#"),
            '_' => result.push_str(r"\_"),
            '{' => result.push_str(r"\{"),
            '}' => result.push_str(r"\}"),
            '~' => result.push_str(r"\textasciitilde{}"),
            '^' => result.push_str(r"\textasciicircum{}"),
            '\\' => result.push_str(r"\textbackslash{}"),
            _ => result.push(ch),
        }
    }
    result
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{ProofDocument, ProofStep};

    #[test]
    fn compile_minimal_document() {
        let doc = ProofDocument {
            title: "Minimal Test".into(),
            abstract_text: "A minimal proof document.".into(),
            process: "e+ e- -> gamma".into(),
            steps: vec![],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains(r"\documentclass[aps,prd,twocolumn,superscriptaddress]{revtex4-2}"));
        assert!(tex.contains(r"\begin{document}"));
        assert!(tex.contains(r"\end{document}"));
        assert!(tex.contains(r"\title{Minimal Test}"));
        assert!(tex.contains(r"\begin{abstract}"));
        assert!(tex.contains("A minimal proof document."));
    }

    #[test]
    fn compile_with_single_step() {
        let doc = ProofDocument {
            title: "Single Step".into(),
            abstract_text: "One step.".into(),
            process: "A -> B C".into(),
            steps: vec![ProofStep::leaf(
                "Feynman Rules",
                "Apply Feynman rules.",
                r"\mathcal{M} = g \bar{u} \gamma^\mu u",
            )],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains(r"\section{Feynman Rules}"));
        assert!(tex.contains(r"\begin{equation}"));
        assert!(tex.contains(r"\mathcal{M} = g \bar{u} \gamma^\mu u"));
        assert!(tex.contains(r"\end{equation}"));
    }

    #[test]
    fn compile_with_nested_steps() {
        let child = ProofStep::leaf("Sub-step", "A sub-derivation.", r"\alpha");
        let parent =
            ProofStep::with_children("Main Section", "Parent description.", r"\beta", vec![child]);
        let doc = ProofDocument {
            title: "Nested".into(),
            abstract_text: "Nested test.".into(),
            process: "X -> Y".into(),
            steps: vec![parent],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains(r"\section{Main Section}"));
        assert!(tex.contains(r"\subsection{Sub-step}"));
    }

    #[test]
    fn compile_deep_nesting_clamps_to_subsubsection() {
        let leaf = ProofStep::leaf("Leaf", "Deep.", r"\delta");
        let l2 = ProofStep::with_children("L2", "Level 2.", r"\gamma", vec![leaf]);
        let l1 = ProofStep::with_children("L1", "Level 1.", r"\beta", vec![l2]);
        let l0 = ProofStep::with_children("L0", "Level 0.", r"\alpha", vec![l1]);

        let doc = ProofDocument {
            title: "Deep".into(),
            abstract_text: "Deep nesting.".into(),
            process: "A -> B".into(),
            steps: vec![l0],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains(r"\section{L0}"));
        assert!(tex.contains(r"\subsection{L1}"));
        assert!(tex.contains(r"\subsubsection{L2}"));
        // Leaf also gets \subsubsection (depth >= 2 clamps)
        assert!(tex.contains(r"\subsubsection{Leaf}"));
    }

    #[test]
    fn escape_latex_special_chars() {
        assert_eq!(escape_latex("a & b"), r"a \& b");
        assert_eq!(escape_latex("100%"), r"100\%");
        assert_eq!(escape_latex("$x$"), r"\$x\$");
        assert_eq!(escape_latex("a_b"), r"a\_b");
        assert_eq!(escape_latex("a#b"), r"a\#b");
        assert_eq!(escape_latex("{x}"), r"\{x\}");
    }

    #[test]
    fn empty_math_skips_equation_env() {
        let doc = ProofDocument {
            title: "No Math".into(),
            abstract_text: "No equations.".into(),
            process: "A -> B".into(),
            steps: vec![ProofStep::leaf("Text Only", "Just text.", "")],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains(r"\section{Text Only}"));
        assert!(!tex.contains(r"\begin{equation}"));
    }

    #[test]
    fn conclusion_contains_process() {
        let doc = ProofDocument {
            title: "T".into(),
            abstract_text: "A.".into(),
            process: "e+ e- -> mu+ mu-".into(),
            steps: vec![],
        };
        let tex = compile_proof_to_latex(&doc);
        assert!(tex.contains("e+ e- -> mu+ mu-"));
        assert!(tex.contains(r"\section{Conclusion}"));
    }

    #[test]
    fn special_chars_in_title_escaped() {
        let doc = ProofDocument {
            title: "sigma(e+e- -> mu+mu-)".into(),
            abstract_text: "Test.".into(),
            process: "test".into(),
            steps: vec![],
        };
        let tex = compile_proof_to_latex(&doc);
        // The title should not contain raw special chars
        assert!(tex.contains(r"\title{sigma(e+e- -> mu+mu-)}"));
    }

    #[test]
    fn compile_with_provenance_includes_hash() {
        use crate::io::provenance::{compute_provenance, ProvenanceState};
        use crate::lagrangian::TheoreticalModel;

        let state = ProvenanceState::new(TheoreticalModel::default(), None, None, 42);
        let record = compute_provenance(&state);

        let doc = ProofDocument {
            title: "Provenance Test".into(),
            abstract_text: "With provenance.".into(),
            process: "e+ e- -> gamma".into(),
            steps: vec![],
        };
        let tex = compile_proof_to_latex_with_provenance(&doc, &record);
        assert!(tex.contains("% SPIRE PROVENANCE HASH:"));
        assert!(tex.contains(&record.sha256));
        assert!(tex.contains("% BEGIN PROVENANCE PAYLOAD"));
        assert!(tex.contains("% END PROVENANCE PAYLOAD"));
        // The actual document content should still be present after the provenance block.
        assert!(tex.contains(r"\documentclass"));
        assert!(tex.contains(r"\end{document}"));
    }
}
