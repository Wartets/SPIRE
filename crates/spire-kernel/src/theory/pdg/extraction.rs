use rusqlite::{params, OptionalExtension};

use crate::theory::pdg::contracts::{AsymmetricError, PdgBranchingFraction, PdgValue};
use crate::theory::pdg::database::PdgDatabase;
use crate::{SpireError, SpireResult};

#[derive(Debug, Clone)]
struct QuantityRow {
    pdgid: String,
    description: String,
    value: f64,
    error_positive: Option<f64>,
    error_negative: Option<f64>,
    limit_type: Option<String>,
    value_type: Option<String>,
    unit_text: Option<String>,
    display_in_percent: Option<i64>,
}

impl PdgDatabase {
    /// Extract a single quantity by PDG `data_type` from the resolved root `pdgid.id`.
    pub fn extract_core_quantity(
        &self,
        root_pdgid_id: i64,
        data_type: &str,
    ) -> SpireResult<Option<PdgValue>> {
        let sql = self.query_builder().quantity_lookup_sql();
        let mut stmt = self.connection().prepare(sql).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to prepare quantity extraction query for data_type '{}': {}",
                data_type, err
            ))
        })?;

        let row = stmt
            .query_row(params![root_pdgid_id, data_type], |row| {
                Ok(QuantityRow {
                    pdgid: row.get(0)?,
                    description: row.get(1)?,
                    value: row.get(3)?,
                    error_positive: row.get(4)?,
                    error_negative: row.get(5)?,
                    limit_type: row.get(6)?,
                    value_type: row.get(7)?,
                    unit_text: row.get(8)?,
                    display_in_percent: row.get(9)?,
                })
            })
            .optional()
            .map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Quantity extraction failed for root_id={} and data_type='{}': {}",
                    root_pdgid_id, data_type, err
                ))
            })?;

        row.map(|r| quantity_row_to_value(&r, data_type))
            .transpose()
    }

    /// Extract branching-fraction rows from the resolved root `pdgid.id`.
    pub fn extract_branching_fractions(
        &self,
        root_pdgid_id: i64,
    ) -> SpireResult<Vec<PdgBranchingFraction>> {
        let sql = self.query_builder().branching_fraction_lookup_sql();
        let mut stmt = self.connection().prepare(sql).map_err(|err| {
            SpireError::DatabaseError(format!(
                "Failed to prepare branching-fraction extraction query: {}",
                err
            ))
        })?;

        let rows = stmt
            .query_map([root_pdgid_id], |row| {
                Ok(QuantityRow {
                    pdgid: row.get(0)?,
                    description: row.get(1)?,
                    value: row.get(2)?,
                    error_positive: row.get(3)?,
                    error_negative: row.get(4)?,
                    limit_type: row.get(5)?,
                    value_type: row.get(6)?,
                    unit_text: row.get(7)?,
                    display_in_percent: row.get(8)?,
                })
            })
            .map_err(|err| {
                SpireError::DatabaseError(format!(
                    "Branching-fraction extraction failed for root_id={}: {}",
                    root_pdgid_id, err
                ))
            })?;

        let mut out = Vec::new();
        for row in rows {
            let parsed = row.map_err(|err| {
                SpireError::DatabaseError(format!("Failed to read branching-fraction row: {}", err))
            })?;
            out.push(PdgBranchingFraction {
                pdgid: parsed.pdgid.clone(),
                description: parsed.description.clone(),
                value: quantity_row_to_value(&parsed, "BR")?,
            });
        }

        Ok(out)
    }
}

fn quantity_row_to_value(row: &QuantityRow, data_type: &str) -> SpireResult<PdgValue> {
    let value = normalize_value(
        row.value,
        row.unit_text.as_deref(),
        data_type,
        row.display_in_percent,
    )?;

    let err_plus = row
        .error_positive
        .map(|v| {
            normalize_value(
                v,
                row.unit_text.as_deref(),
                data_type,
                row.display_in_percent,
            )
        })
        .transpose()?;
    let err_minus = row
        .error_negative
        .map(|v| {
            normalize_value(
                v,
                row.unit_text.as_deref(),
                data_type,
                row.display_in_percent,
            )
        })
        .transpose()?;

    let is_limit =
        row.limit_type.is_some() || matches!(row.value_type.as_deref(), Some("L") | Some("U"));

    let result = match (err_plus, err_minus) {
        (None, None) => PdgValue::Exact { value, is_limit },
        (Some(ep), Some(em)) if (ep - em).abs() <= f64::EPSILON => PdgValue::Symmetric {
            value,
            error: ep,
            is_limit,
        },
        (Some(ep), Some(em)) => PdgValue::Asymmetric {
            value,
            error: AsymmetricError::new(em, ep),
            is_limit,
        },
        (Some(ep), None) | (None, Some(ep)) => PdgValue::Symmetric {
            value,
            error: ep,
            is_limit,
        },
    };

    Ok(result)
}

fn normalize_value(
    raw: f64,
    unit_text: Option<&str>,
    data_type: &str,
    display_in_percent: Option<i64>,
) -> SpireResult<f64> {
    let unit = unit_text.unwrap_or("").trim().to_ascii_lowercase();

    if display_in_percent == Some(1) {
        return Ok(raw / 100.0);
    }

    let normalized = match data_type {
        // Mass and total width are normalized to GeV.
        "M" | "G" => match unit.as_str() {
            "ev" => raw * 1.0e-9,
            "kev" => raw * 1.0e-6,
            "mev" => raw * 1.0e-3,
            "gev" | "" => raw,
            "tev" => raw * 1.0e3,
            other => {
                return Err(SpireError::DatabaseError(format!(
                    "Unsupported energy unit '{}' for data_type '{}'",
                    other, data_type
                )));
            }
        },
        // Lifetime is normalized to seconds.
        "T" => match unit.as_str() {
            "s" | "sec" | "" => raw,
            "ms" => raw * 1.0e-3,
            "us" | "μs" => raw * 1.0e-6,
            "ns" => raw * 1.0e-9,
            "ps" => raw * 1.0e-12,
            "fs" => raw * 1.0e-15,
            other => {
                return Err(SpireError::DatabaseError(format!(
                    "Unsupported lifetime unit '{}' for data_type '{}'",
                    other, data_type
                )));
            }
        },
        // BR values are dimensionless (or percentages already transformed above).
        _ => raw,
    };

    Ok(normalized)
}
