//! Extraction policy engine for PDG decay channels.
//!
//! Provides runtime policy flags to control whether generic/unresolved products
//! are included or filtered out from decay tables.

use crate::theory::pdg::contracts::PdgDecayChannel;

/// Runtime policy for PDG decay channel extraction.
///
/// Controls whether generic (non-Monte-Carlo-samplable) decay channels are included
/// in the final output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdgExtractionPolicy {
    /// Strict physical mode: only Monte-Carlo-samplable channels.
    ///
    /// Filters out any channel containing a generic product (e.g., "X", "hadrons").
    /// Remaining channels are guaranteed to have fully concrete, kinematically reconstructible
    /// final states. Branching ratios are left as-is (may not sum to 1.0 due to filtering).
    StrictPhysical,

    /// Catalog mode: all reported channels.
    ///
    /// Includes all decay channels as recorded in the PDG, including those with
    /// generic or incomplete product descriptions. Useful for theoretical reference
    /// and UI browsing.
    Catalog,
}

/// Apply an extraction policy to a set of decay channels.
///
/// # Arguments
/// * `channels` - Raw decay channels from the PDG database
/// * `policy` - Which channels to keep/filter
///
/// # Returns
/// Filtered channel list according to the policy.
pub fn apply_policy(
    channels: Vec<PdgDecayChannel>,
    policy: PdgExtractionPolicy,
) -> Vec<PdgDecayChannel> {
    match policy {
        PdgExtractionPolicy::StrictPhysical => {
            channels.into_iter().filter(|ch| !ch.is_generic).collect()
        }
        PdgExtractionPolicy::Catalog => channels,
    }
}

/// Compute effective branching fractions after filtering.
///
/// If channels are filtered out, the remaining fractions may not sum to 1.0.
/// This function optionally re-normalizes them to sum to 1.0 for MC generation.
///
/// # Arguments
/// * `channels` - Already-filtered channels (result of `apply_policy`)
/// * `renormalize` - If true, scale BRs so they sum to 1.0; if false, leave as partial widths
///
/// # Returns
/// Channels with optionally renormalized branching fractions.
pub fn effective_branching_fractions(
    mut channels: Vec<PdgDecayChannel>,
    renormalize: bool,
) -> Vec<PdgDecayChannel> {
    if !renormalize {
        return channels;
    }

    // Sum existing branching fractions
    let total_br: f64 = channels
        .iter()
        .filter_map(|ch| ch.branching_fraction())
        .sum();

    if total_br <= 0.0 || (total_br - 1.0).abs() < 1e-6 {
        // Already normalized or no branching data
        return channels;
    }

    // Scale each channel's branching fraction
    for ch in &mut channels {
        if let Some(ref mut br_val) = ch.branching_ratio {
            let scaled_br = br_val.central() / total_br;
            *br_val = match br_val {
                crate::theory::pdg::contracts::PdgValue::Exact { is_limit, .. } => {
                    crate::theory::pdg::contracts::PdgValue::Exact {
                        value: scaled_br,
                        is_limit: *is_limit,
                    }
                }
                crate::theory::pdg::contracts::PdgValue::Symmetric {
                    error, is_limit, ..
                } => crate::theory::pdg::contracts::PdgValue::Symmetric {
                    value: scaled_br,
                    error: *error / total_br,
                    is_limit: *is_limit,
                },
                crate::theory::pdg::contracts::PdgValue::Asymmetric {
                    error, is_limit, ..
                } => crate::theory::pdg::contracts::PdgValue::Asymmetric {
                    value: scaled_br,
                    error: crate::theory::pdg::contracts::AsymmetricError {
                        minus: error.minus / total_br,
                        plus: error.plus / total_br,
                    },
                    is_limit: *is_limit,
                },
            };
        }
    }

    channels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::pdg::contracts::{PdgDecayChannel, PdgDecayProduct, PdgValue};

    fn sample_concrete_channel(mcid: i32) -> PdgDecayChannel {
        PdgDecayChannel {
            mode_id: 1,
            products: vec![(PdgDecayProduct::Concrete { mcid }, 1)],
            branching_ratio: Some(PdgValue::Exact {
                value: 0.5,
                is_limit: false,
            }),
            is_generic: false,
            description: Some("test".to_string()),
        }
    }

    fn sample_generic_channel() -> PdgDecayChannel {
        PdgDecayChannel {
            mode_id: 2,
            products: vec![(
                PdgDecayProduct::Generic {
                    description: "X".to_string(),
                },
                1,
            )],
            branching_ratio: Some(PdgValue::Exact {
                value: 0.5,
                is_limit: false,
            }),
            is_generic: true,
            description: Some("inclusive".to_string()),
        }
    }

    #[test]
    fn test_strict_physical_filters_generic() {
        let channels = vec![
            sample_concrete_channel(11),
            sample_generic_channel(),
            sample_concrete_channel(13),
        ];

        let filtered = apply_policy(channels, PdgExtractionPolicy::StrictPhysical);

        assert_eq!(filtered.len(), 2);
        assert!(!filtered.iter().any(|ch| ch.is_generic));
    }

    #[test]
    fn test_catalog_keeps_all() {
        let channels = vec![
            sample_concrete_channel(11),
            sample_generic_channel(),
            sample_concrete_channel(13),
        ];

        let filtered = apply_policy(channels.clone(), PdgExtractionPolicy::Catalog);

        assert_eq!(filtered.len(), channels.len());
    }

    #[test]
    fn test_renormalization() {
        let mut channels = vec![sample_concrete_channel(11), sample_concrete_channel(13)];

        // Set branching fractions that don't sum to 1.0
        channels[0].branching_ratio = Some(PdgValue::Exact {
            value: 0.3,
            is_limit: false,
        });
        channels[1].branching_ratio = Some(PdgValue::Exact {
            value: 0.2,
            is_limit: false,
        });

        let normalized = effective_branching_fractions(channels, true);

        let sum: f64 = normalized
            .iter()
            .filter_map(|ch| ch.branching_fraction())
            .sum();

        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_no_renormalization() {
        let mut channels = vec![sample_concrete_channel(11)];
        channels[0].branching_ratio = Some(PdgValue::Exact {
            value: 0.7,
            is_limit: false,
        });

        let unchanged = effective_branching_fractions(channels.clone(), false);

        assert_eq!(unchanged[0].branching_fraction(), Some(0.7));
    }
}
