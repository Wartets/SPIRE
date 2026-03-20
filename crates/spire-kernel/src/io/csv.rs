//! Robust CSV/TSV/semicolon parser utilities for experimental datasets.

use crate::{SpireError, SpireResult};

/// Parsed numerical table with optional header metadata.
#[derive(Debug, Clone)]
pub struct ParsedNumericTable {
    /// Parsed numeric rows.
    pub rows: Vec<Vec<f64>>,
    /// Number of skipped header/comment lines.
    pub skipped_lines: usize,
    /// Delimiter used by the parser.
    pub delimiter: char,
}

fn detect_delimiter(line: &str) -> char {
    let candidates = [',', ';', '\t'];
    let mut best = ',';
    let mut best_count = 0usize;
    for &d in &candidates {
        let c = line.matches(d).count();
        if c > best_count {
            best = d;
            best_count = c;
        }
    }
    best
}

fn looks_like_header(fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|f| f.chars().any(|c| c.is_ascii_alphabetic()))
}

/// Parse a text table with auto delimiter detection and resilient header skipping.
pub fn parse_numeric_table(text: &str) -> SpireResult<ParsedNumericTable> {
    let mut delimiter = ',';

    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        delimiter = detect_delimiter(line);
        break;
    }

    let mut rows = Vec::new();
    let mut skipped = 0usize;

    for (line_idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            skipped += 1;
            continue;
        }

        let fields: Vec<&str> = line.split(delimiter).map(str::trim).collect();
        if fields.is_empty() {
            skipped += 1;
            continue;
        }

        if looks_like_header(&fields) {
            skipped += 1;
            continue;
        }

        let mut parsed = Vec::with_capacity(fields.len());
        for (col_idx, v) in fields.iter().enumerate() {
            if v.is_empty() {
                parsed.push(0.0);
                continue;
            }
            let val = v.parse::<f64>().map_err(|_| {
                SpireError::DataParseError(format!(
                    "Row {} column {} contains invalid floating-point data: '{}'",
                    line_idx + 1,
                    col_idx + 1,
                    v
                ))
            })?;
            parsed.push(val);
        }

        if parsed.len() < 2 {
            return Err(SpireError::DataParseError(format!(
                "Row {} must contain at least x,y columns",
                line_idx + 1
            )));
        }

        rows.push(parsed);
    }

    if rows.is_empty() {
        return Err(SpireError::DataParseError(
            "No numeric data rows found in input file".to_string(),
        ));
    }

    Ok(ParsedNumericTable {
        rows,
        skipped_lines: skipped,
        delimiter,
    })
}

/// Validate overlap between experimental x-range and theoretical histogram support.
pub fn validate_axis_overlap(
    theory_edges: &[f64],
    x_values: &[f64],
    observable: &str,
) -> SpireResult<()> {
    if theory_edges.len() < 2 || x_values.is_empty() {
        return Err(SpireError::DataMismatch(format!(
            "Cannot validate axis overlap for observable '{}' with empty inputs",
            observable
        )));
    }

    let theory_min = theory_edges.first().copied().unwrap_or(0.0);
    let theory_max = theory_edges.last().copied().unwrap_or(0.0);

    let mut exp_min = f64::INFINITY;
    let mut exp_max = f64::NEG_INFINITY;
    for &x in x_values {
        exp_min = exp_min.min(x);
        exp_max = exp_max.max(x);
    }

    let overlap_min = theory_min.max(exp_min);
    let overlap_max = theory_max.min(exp_max);

    if overlap_max <= overlap_min {
        return Err(SpireError::DataMismatch(format!(
            "Observable '{}' has no X-axis overlap: theory=[{:.6}, {:.6}] experimental=[{:.6}, {:.6}]",
            observable, theory_min, theory_max, exp_min, exp_max
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delimiter_detection_semicolon() {
        let txt = "x;y;dy\n1;2;0.1\n2;3;0.2";
        let p = parse_numeric_table(txt).unwrap();
        assert_eq!(p.delimiter, ';');
        assert_eq!(p.rows.len(), 2);
    }

    #[test]
    fn header_and_comment_skipping() {
        let txt = "# c\nx,y\n0.1,1\n0.2,2";
        let p = parse_numeric_table(txt).unwrap();
        assert!(p.skipped_lines >= 2);
        assert_eq!(p.rows.len(), 2);
    }

    #[test]
    fn axis_overlap_validation() {
        assert!(validate_axis_overlap(&[0.0, 1.0, 2.0], &[1.5, 1.7], "pt").is_ok());
        assert!(validate_axis_overlap(&[0.0, 1.0], &[2.0, 3.0], "eta").is_err());
    }
}
