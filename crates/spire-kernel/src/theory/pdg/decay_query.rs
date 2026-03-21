//! PDG decay channel extraction and resolution.
//!
//! This module provides SQL-based traversal of the PDG decay hierarchy, resolving
//! decay products from concrete MCIDs or generic placeholders, and reconstructing
//! flat product lists from tree structures.

use crate::theory::pdg::contracts::{PdgDecayChannel, PdgDecayProduct};
use crate::SpireError;
use rusqlite::{Connection, OptionalExtension, Row};

/// Low-level query abstraction for decay channel extraction.
pub trait DecayQueryBuilder {
    /// Fetch all decay modes for a given parent MCID.
    fn get_decay_modes(&self, parent_mcid: i32) -> Result<Vec<DecayModeRow>, SpireError>;

    /// Fetch all products for a specific decay mode (pdgid_id).
    fn get_decay_products(&self, pdgid_id: i32) -> Result<Vec<DecayProductRow>, SpireError>;

    /// Resolve a product name/alias to its concrete MCID, if possible.
    fn resolve_product_mcid(
        &self,
        pdgitem_id: Option<i32>,
        product_name: &str,
    ) -> Result<Option<i32>, SpireError>;

    /// Check if a product is generic (non-concrete).
    fn is_product_generic(&self, pdgitem_id: Option<i32>) -> Result<bool, SpireError>;

    /// Recursively fetch subdecay products if `subdecay_id` is set.
    fn get_subdecay_products(&self, subdecay_id: i32) -> Result<Vec<DecayProductRow>, SpireError>;
}

/// Raw database row for a decay mode.
#[derive(Debug, Clone)]
pub struct DecayModeRow {
    /// Internal pdgid.id primary key
    pub pdgid_id: i32,
    /// PDG particle code (e.g., "23")
    pub pdgid: String,
    /// Optional mode enumeration number
    pub mode_number: Option<i32>,
    /// Human-readable description (e.g., "Z0 --> e+ e-")
    pub description: String,
}

/// Raw database row for a decay product.
#[derive(Debug, Clone)]
pub struct DecayProductRow {
    /// pdgdecay.id primary key
    pub product_id: i32,
    /// pdgitem.id foreign key (may be NULL)
    pub pdgitem_id: Option<i32>,
    /// Product name (e.g., "e+", "X", "hadrons")
    pub name: String,
    /// Product multiplier (e.g., 2 for "2 π⁰")
    pub multiplier: i32,
    /// Subdecay reference (may be NULL)
    pub subdecay_id: Option<i32>,
    /// Sort order within the mode
    pub sort: i32,
}

/// Standard implementation of DecayQueryBuilder.
pub struct StandardDecayQueryBuilder<'conn> {
    conn: &'conn Connection,
}

impl<'conn> StandardDecayQueryBuilder<'conn> {
    /// Create a new query builder.
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }
}

impl<'conn> DecayQueryBuilder for StandardDecayQueryBuilder<'conn> {
    fn get_decay_modes(&self, parent_mcid: i32) -> Result<Vec<DecayModeRow>, SpireError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, pdgid, mode_number, description \
             FROM pdgid \
             WHERE EXISTS (
               SELECT 1 FROM pdgparticle p WHERE p.pdgid_id = pdgid.id AND p.mcid = ?
             ) \
             ORDER BY mode_number ASC, sort ASC, id ASC",
            )
            .map_err(|e| SpireError::DatabaseError(e.to_string()))?;

        let modes = stmt
            .query_map([parent_mcid], |row: &Row| {
                Ok(DecayModeRow {
                    pdgid_id: row.get(0)?,
                    pdgid: row.get(1)?,
                    mode_number: row.get(2)?,
                    description: row.get(3)?,
                })
            })
            .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?;

        Ok(modes)
    }

    fn get_decay_products(&self, pdgid_id: i32) -> Result<Vec<DecayProductRow>, SpireError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, pdgitem_id, name, multiplier, subdecay_id, sort \
             FROM pdgdecay \
             WHERE pdgid_id = ? \
             ORDER BY sort ASC, id ASC",
            )
            .map_err(|e| SpireError::DatabaseError(e.to_string()))?;

        let products = stmt
            .query_map([pdgid_id], |row: &Row| {
                Ok(DecayProductRow {
                    product_id: row.get(0)?,
                    pdgitem_id: row.get(1)?,
                    name: row.get(2)?,
                    multiplier: row.get(3)?,
                    subdecay_id: row.get(4)?,
                    sort: row.get(5)?,
                })
            })
            .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?;

        Ok(products)
    }

    fn resolve_product_mcid(
        &self,
        pdgitem_id: Option<i32>,
        product_name: &str,
    ) -> Result<Option<i32>, SpireError> {
        // Try direct pdgitem → pdgparticle mapping
        if let Some(item_id) = pdgitem_id {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT p.mcid FROM pdgparticle p \
                 WHERE p.pdgitem_id = ? \
                 ORDER BY ABS(p.mcid) ASC, p.mcid ASC \
                 LIMIT 1",
                )
                .map_err(|e| SpireError::DatabaseError(e.to_string()))?;

            let mcid: Option<i32> = stmt
                .query_row([item_id], |row: &Row| row.get(0))
                .optional()
                .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?;

            if mcid.is_some() {
                return Ok(mcid);
            }
        }

        // Try alias-based fallback via pdgitem name matching
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.mcid FROM pdgparticle p \
             JOIN pdgitem i ON p.pdgitem_id = i.id \
             WHERE i.name = ? \
             ORDER BY i.item_type DESC, ABS(p.mcid) ASC, p.mcid ASC \
             LIMIT 1",
            )
            .map_err(|e| SpireError::DatabaseError(e.to_string()))?;

        let mcid: Option<i32> = stmt
            .query_row([product_name], |row: &Row| row.get(0))
            .optional()
            .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?;

        Ok(mcid)
    }

    fn is_product_generic(&self, pdgitem_id: Option<i32>) -> Result<bool, SpireError> {
        if let Some(item_id) = pdgitem_id {
            let mut stmt = self
                .conn
                .prepare("SELECT item_type FROM pdgitem WHERE id = ?")
                .map_err(|e| SpireError::DatabaseError(e.to_string()))?;

            let item_type: String = stmt
                .query_row([item_id], |row: &Row| row.get(0))
                .optional()
                .map_err(|e: rusqlite::Error| SpireError::DatabaseError(e.to_string()))?
                .unwrap_or_else(|| "P".to_string());

            // Generic if item_type = 'G' (generic) or 'B' (both)
            return Ok(item_type == "G" || item_type == "B");
        }

        // If no pdgitem_id, check if product name looks generic
        Ok(false)
    }

    fn get_subdecay_products(&self, subdecay_id: i32) -> Result<Vec<DecayProductRow>, SpireError> {
        // Recursively fetch products for a subdecay mode
        self.get_decay_products(subdecay_id)
    }
}

/// Extract decay channels for a parent particle.
///
/// This is the high-level API that orchestrates SQL queries and product resolution.
pub fn extract_decay_channels(
    conn: &Connection,
    parent_mcid: i32,
) -> Result<Vec<PdgDecayChannel>, SpireError> {
    let qb = StandardDecayQueryBuilder::new(conn);

    let modes = qb.get_decay_modes(parent_mcid)?;
    let mut channels = Vec::new();

    for mode in modes {
        let products_raw = qb.get_decay_products(mode.pdgid_id)?;
        let mut products: Vec<(PdgDecayProduct, u32)> = Vec::new();
        let mut is_generic = false;

        for prod in products_raw {
            let multiplier = prod.multiplier.max(1) as u32;

            // Try to resolve to concrete MCID first
            if let Some(mcid) = qb.resolve_product_mcid(prod.pdgitem_id, &prod.name)? {
                products.push((PdgDecayProduct::Concrete { mcid }, multiplier));
            } else if qb.is_product_generic(prod.pdgitem_id)? {
                // Mark as generic if we can't resolve and it looks generic
                products.push((
                    PdgDecayProduct::Generic {
                        description: prod.name.clone(),
                    },
                    multiplier,
                ));
                is_generic = true;
            } else {
                // Unknown product: mark as generic to be safe
                products.push((
                    PdgDecayProduct::Generic {
                        description: prod.name.clone(),
                    },
                    multiplier,
                ));
                // Only set is_generic if it's truly a placeholder (e.g., "X", "hadrons")
                if is_likely_generic_placeholder(&prod.name) {
                    is_generic = true;
                }
            }

            // Handle subdecays recursively if present
            if let Some(subdecay_id) = prod.subdecay_id {
                let subdecay_products = qb.get_subdecay_products(subdecay_id)?;
                for sprod in subdecay_products {
                    let mult = sprod.multiplier.max(1) as u32;
                    if let Some(mcid) = qb.resolve_product_mcid(sprod.pdgitem_id, &sprod.name)? {
                        products.push((PdgDecayProduct::Concrete { mcid }, mult));
                    } else {
                        products.push((
                            PdgDecayProduct::Generic {
                                description: sprod.name.clone(),
                            },
                            mult,
                        ));
                        is_generic = true;
                    }
                }
            }
        }

        let channel = PdgDecayChannel {
            mode_id: mode.pdgid_id,
            products,
            branching_ratio: None, // Will be populated from pdgdata if available
            is_generic,
            description: Some(mode.description),
        };

        channels.push(channel);
    }

    Ok(channels)
}

/// Check if a product name looks like a generic placeholder.
fn is_likely_generic_placeholder(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("x")
        || lower.contains("hadrons")
        || lower.contains("invisible")
        || lower.contains("unknown")
        || lower.contains("inclusive")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_placeholder_detection() {
        assert!(is_likely_generic_placeholder("X"));
        assert!(is_likely_generic_placeholder("hadrons"));
        assert!(is_likely_generic_placeholder("invisible"));
        assert!(!is_likely_generic_placeholder("e+"));
        assert!(!is_likely_generic_placeholder("pi"));
    }
}
