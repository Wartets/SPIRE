//! # Decay — Automated Width & Branching Ratio Calculator
//!
//! This module discovers kinematically allowed decay channels for any particle
//! in a [`TheoreticalModel`], computes partial widths via phase-space integration,
//! and assembles the results into a [`DecayTable`] that can be serialised for
//! UI display or exported in SLHA format.
//!
//! ## Channel Discovery
//!
//! Given a parent particle $A$ with mass $m_A$, the engine iterates all
//! [`VertexFactor`]s in the model, identifies vertices containing $A$, and
//! checks whether the remaining daughters are kinematically accessible:
//! $m_A > \sum_i m_i$ (daughters).
//!
//! ## Partial Width Formula (2-body: $A \to B + C$)
//!
//! For a 2-body decay the partial width is:
//!
//! $$\Gamma(A \to BC) = \frac{|\vec{p}_{\text{cm}}|}{8\pi m_A^2}\,|\mathcal{M}|^2$$
//!
//! where $|\vec{p}_{\text{cm}}|$ is the CM 3-momentum of either daughter:
//!
//! $$|\vec{p}_{\text{cm}}| = \frac{\sqrt{\lambda(m_A^2,\,m_B^2,\,m_C^2)}}{2\,m_A}$$
//!
//! and $|\mathcal{M}|^2$ is the spin-averaged squared matrix element.
//! For vertices with coupling $g$ and lorentz structure $\gamma^\mu$ (vector
//! coupling to fermion pair), the spin-summed/averaged result is:
//!
//! $$\overline{|\mathcal{M}|^2} = N_c \cdot g^2 \cdot F_{\text{spin}}(m_A, m_B, m_C)$$
//!
//! where $N_c$ is the colour factor and $F_{\text{spin}}$ depends on the
//! spin structure of the vertex.
//!
//! ## SLHA Export
//!
//! The [`DecayTable::to_slha_string`] method formats the result in the
//! standard SUSY Les Houches Accord `DECAY` block format.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::kinematics::{cm_momentum, kallen_lambda};
use crate::lagrangian::{LagrangianTermKind, PropagatorForm, TheoreticalModel, VertexFactor};
use crate::ontology::Field;
use crate::SpireResult;

// ===========================================================================
// Data Structures
// ===========================================================================

/// A single decay channel with its partial width and branching ratio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayChannel {
    /// Identifiers of the final-state daughters.
    pub final_state: Vec<String>,
    /// Human-readable display names of the daughters.
    pub final_state_names: Vec<String>,
    /// Partial width $\Gamma_i$ in GeV.
    pub partial_width: f64,
    /// Branching ratio $\text{BR}_i = \Gamma_i / \Gamma_{\text{tot}}$.
    pub branching_ratio: f64,
    /// The vertex factor ID that mediates this channel.
    pub vertex_id: String,
}

/// Complete decay table for a single parent particle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayTable {
    /// The parent particle identifier.
    pub parent_id: String,
    /// The parent particle display name.
    pub parent_name: String,
    /// The parent particle mass in GeV.
    pub parent_mass: f64,
    /// Total decay width $\Gamma_{\text{tot}}$ in GeV.
    pub total_width: f64,
    /// Mean lifetime $\tau = \hbar / \Gamma_{\text{tot}}$ in seconds.
    pub lifetime_seconds: f64,
    /// All kinematically allowed decay channels with their partial widths.
    pub channels: Vec<DecayChannel>,
}

/// Discovery result: a candidate decay channel before width calculation.
#[derive(Debug, Clone)]
struct CandidateChannel {
    /// IDs of the daughter particles.
    daughter_ids: Vec<String>,
    /// The vertex factor mediating this decay.
    vertex: VertexFactor,
}

// ===========================================================================
// Constants
// ===========================================================================

/// Reduced Planck constant in GeV·s: $\hbar = 6.582119569 \times 10^{-25}$ GeV·s.
const HBAR_GEV_S: f64 = 6.582119569e-25;

// ===========================================================================
// Channel Discovery
// ===========================================================================

/// Discover all kinematically allowed decay channels for a parent particle.
///
/// Iterates through every [`VertexFactor`] in the model, identifies those
/// containing the parent field, and filters out channels where the daughter
/// masses exceed the parent mass.
///
/// # Arguments
/// * `model` — The theoretical model containing fields and vertex factors.
/// * `parent_id` — The identifier of the decaying particle.
///
/// # Returns
/// A vector of candidate channels (before width calculation).
fn find_candidate_channels(model: &TheoreticalModel, parent_id: &str) -> Vec<CandidateChannel> {
    let parent_field = match model.fields.iter().find(|f| f.id == parent_id) {
        Some(f) => f,
        None => return vec![],
    };

    let parent_mass = parent_field.mass;

    // Only consider interaction vertices (not kinetic/mass terms).
    let interaction_vertices: Vec<&VertexFactor> = model
        .vertex_factors
        .iter()
        .filter(|vf| {
            // Check that the corresponding Lagrangian term is an interaction.
            model
                .terms
                .iter()
                .any(|t| t.id == vf.term_id && t.term_kind == LagrangianTermKind::Interaction)
        })
        .collect();

    let mut candidates = Vec::new();

    for vf in &interaction_vertices {
        // Check if this vertex contains the parent field.
        if !vf.field_ids.contains(&parent_id.to_string()) {
            continue;
        }

        // The daughters are all fields in the vertex except one instance of the parent.
        let mut daughter_ids: Vec<String> = vf.field_ids.clone();
        if let Some(pos) = daughter_ids.iter().position(|id| id == parent_id) {
            daughter_ids.remove(pos);
        } else {
            continue;
        }

        // We need at least 2 daughters for a decay.
        if daughter_ids.len() < 2 {
            continue;
        }

        // Compute the daughter mass sum.
        let daughter_mass_sum: f64 = daughter_ids
            .iter()
            .filter_map(|id| model.fields.iter().find(|f| f.id == *id))
            .map(|f| f.mass)
            .sum();

        // Kinematic filter: parent must be heavier than the sum of daughters.
        if parent_mass <= daughter_mass_sum {
            continue;
        }

        candidates.push(CandidateChannel {
            daughter_ids,
            vertex: (*vf).clone(),
        });
    }

    candidates
}

/// Public wrapper: discover all kinematically allowed decay channels.
///
/// Returns the daughter field IDs and vertex ID for each channel.
pub fn find_decay_channels(
    model: &TheoreticalModel,
    parent_id: &str,
) -> Vec<(Vec<String>, String)> {
    find_candidate_channels(model, parent_id)
        .into_iter()
        .map(|c| (c.daughter_ids, c.vertex.term_id))
        .collect()
}

// ===========================================================================
// Spin-averaged |M|^2 Evaluation
// ===========================================================================

/// Compute the colour multiplicity factor $N_c$ for a decay vertex.
///
/// For a parent decaying to a colour-charged pair (e.g., $Z \to q\bar{q}$),
/// this returns 3 (one for each colour). For colour-singlet daughters, returns 1.
fn colour_factor(model: &TheoreticalModel, daughter_ids: &[String]) -> f64 {
    use crate::ontology::ColorRepresentation;
    let mut nc = 1.0;
    for id in daughter_ids {
        if let Some(field) = model.fields.iter().find(|f| f.id == *id) {
            match field.quantum_numbers.color {
                ColorRepresentation::Triplet | ColorRepresentation::AntiTriplet => {
                    nc = 3.0;
                    break;
                }
                ColorRepresentation::Octet => {
                    nc = 8.0;
                    break;
                }
                _ => {}
            }
        }
    }
    nc
}

/// Determine the propagator form of a field from the model.
fn field_propagator_form(model: &TheoreticalModel, field_id: &str) -> Option<PropagatorForm> {
    model
        .propagators
        .iter()
        .find(|p| p.field_id == field_id)
        .map(|p| p.form)
}

/// Compute the spin-averaged squared matrix element $\overline{|\mathcal{M}|^2}$
/// for a 2-body decay $A \to B + C$.
///
/// Uses the vertex coupling value and the spin structure to compute the
/// appropriate Lorentz-contracted result.
///
/// ## Spin Structure Cases
///
/// 1. **Vector → fermion pair** ($V \to f\bar{f}$, e.g. $Z \to e^+e^-$):
///    $\overline{|\mathcal{M}|^2} = g^2 \cdot \frac{1}{3} \cdot (2 m_A^2 + m_B^2 + m_C^2 - (m_B^2 - m_C^2)^2 / m_A^2)$
///    The $1/3$ is from averaging over 3 polarisations of the massive vector.
///
/// 2. **Scalar → fermion pair** ($H \to f\bar{f}$):
///    $\overline{|\mathcal{M}|^2} = g^2 \cdot 2 m_A^2 (1 - 4 m_f^2 / m_A^2)$
///    (no spin average for scalar parent).
///
/// 3. **Fermion → fermion + vector** ($t \to b W$):
///    $\overline{|\mathcal{M}|^2} = g^2 \cdot \frac{1}{2} \cdot ((m_A^2 - m_B^2)^2/m_C^2 + m_A^2 + m_B^2 - 2 m_C^2)$
///
/// 4. **Fallback**: $g^2 \cdot 2 m_A^2$ (generic constant matrix element).
fn compute_m_squared_2body(
    model: &TheoreticalModel,
    parent: &Field,
    daughter_ids: &[String],
    coupling: f64,
) -> f64 {
    let m_a = parent.mass;
    let m_a_sq = m_a * m_a;

    let d0 = model.fields.iter().find(|f| f.id == daughter_ids[0]);
    let d1 = model.fields.iter().find(|f| f.id == daughter_ids[1]);

    let (m_b, m_c) = match (d0, d1) {
        (Some(b), Some(c)) => (b.mass, c.mass),
        _ => return coupling * coupling * 2.0 * m_a_sq,
    };

    let m_b_sq = m_b * m_b;
    let m_c_sq = m_c * m_c;
    let g_sq = coupling * coupling;

    let parent_form = field_propagator_form(model, &parent.id);
    let d0_form = field_propagator_form(model, &daughter_ids[0]);
    let d1_form = field_propagator_form(model, &daughter_ids[1]);

    let is_fermion = |form: Option<PropagatorForm>| form == Some(PropagatorForm::DiracFermion);
    let is_vector = |form: Option<PropagatorForm>| {
        matches!(
            form,
            Some(PropagatorForm::MassiveVector) | Some(PropagatorForm::MasslessVector)
        )
    };
    let is_scalar = |form: Option<PropagatorForm>| form == Some(PropagatorForm::Scalar);

    // Case 1: Vector → fermion pair (V → f f̄)
    if is_vector(parent_form) && is_fermion(d0_form) && is_fermion(d1_form) {
        // Spin-summed then averaged over 3 parent polarisations:
        // |M|² = g² · (1/3) · (2 m_A² + m_B² + m_C² - (m_B² - m_C²)²/m_A²)
        let kinematic =
            2.0 * m_a_sq + m_b_sq + m_c_sq - (m_b_sq - m_c_sq) * (m_b_sq - m_c_sq) / m_a_sq;
        return g_sq * kinematic / 3.0;
    }

    // Case 2: Scalar → fermion pair (H → f f̄)
    if is_scalar(parent_form) && is_fermion(d0_form) && is_fermion(d1_form) {
        // |M|² = g² · 2 m_A² · (1 - 4 m_f²/m_A²) — beta² threshold factor.
        // Both daughters are the same fermion flavour typically.
        let m_f_sq = m_b_sq; // Take the first daughter mass.
        let beta_sq = (1.0 - 4.0 * m_f_sq / m_a_sq).max(0.0);
        return g_sq * 2.0 * m_a_sq * beta_sq;
    }

    // Case 3: Fermion → fermion + massive vector (t → b W)
    if is_fermion(parent_form) && is_fermion(d0_form) && is_vector(d1_form) {
        // |M|² = g² · (1/2) · ((m_A² - m_B²)²/m_C² + m_A² + m_B² - 2 m_C²)
        if m_c_sq > 0.0 {
            let kinematic =
                (m_a_sq - m_b_sq) * (m_a_sq - m_b_sq) / m_c_sq + m_a_sq + m_b_sq - 2.0 * m_c_sq;
            return g_sq * kinematic / 2.0;
        }
    }

    // Case 3b: Fermion → fermion + massive vector (swapped daughter order)
    if is_fermion(parent_form) && is_vector(d0_form) && is_fermion(d1_form) && m_b_sq > 0.0 {
        let kinematic =
            (m_a_sq - m_c_sq) * (m_a_sq - m_c_sq) / m_b_sq + m_a_sq + m_c_sq - 2.0 * m_b_sq;
        return g_sq * kinematic / 2.0;
    }

    // Case 4: Vector → scalar pair (V → S S)
    if is_vector(parent_form) && is_scalar(d0_form) && is_scalar(d1_form) {
        // |M|² = g² · (1/3) · λ(m_A², m_B², m_C²) / m_A²
        let lam = kallen_lambda(m_a_sq, m_b_sq, m_c_sq);
        return g_sq * lam / (3.0 * m_a_sq);
    }

    // Fallback: constant matrix element.
    g_sq * 2.0 * m_a_sq
}

// ===========================================================================
// Partial Width Calculation
// ===========================================================================

/// Compute the partial width for a 2-body decay $A \to B + C$.
///
/// $$\Gamma(A \to BC) = \frac{|\vec{p}_{\text{cm}}|}{8\pi m_A^2} \,
///   N_c \cdot \overline{|\mathcal{M}|^2}$$
///
/// # Arguments
/// * `model` — The theoretical model.
/// * `parent` — The decaying particle field.
/// * `daughter_ids` — IDs of the daughter particles.
/// * `vertex` — The vertex factor mediating this decay.
fn compute_partial_width_2body(
    model: &TheoreticalModel,
    parent: &Field,
    daughter_ids: &[String],
    vertex: &VertexFactor,
) -> f64 {
    let m_a = parent.mass;
    let m_a_sq = m_a * m_a;

    if m_a < 1e-15 {
        return 0.0;
    }

    // Resolve daughter masses.
    let daughter_masses: Vec<f64> = daughter_ids
        .iter()
        .filter_map(|id| model.fields.iter().find(|f| f.id == *id))
        .map(|f| f.mass)
        .collect();

    if daughter_masses.len() != 2 {
        return 0.0;
    }

    let m_b = daughter_masses[0];
    let m_c = daughter_masses[1];

    // CM 3-momentum.
    let p_cm = cm_momentum(m_a_sq, m_b, m_c);
    if p_cm <= 0.0 {
        return 0.0;
    }

    // Coupling constant.
    let coupling = vertex.coupling_value.unwrap_or(0.0);
    if coupling.abs() < 1e-30 {
        return 0.0;
    }

    // Colour factor.
    let nc = colour_factor(model, daughter_ids);

    // Spin-averaged |M|².
    let m_sq = compute_m_squared_2body(model, parent, daughter_ids, coupling);

    // Γ = |p_cm| / (8π m_A²) × N_c × |M|²
    nc * p_cm * m_sq / (8.0 * PI * m_a_sq)
}

// ===========================================================================
// Decay Table Assembly
// ===========================================================================

/// Compute the complete decay table for a particle.
///
/// Discovers all kinematically allowed 2-body channels, computes partial
/// widths, and assembles the branching ratios.
///
/// # Arguments
/// * `model` — The theoretical model.
/// * `parent_id` — Identifier of the decaying particle.
///
/// # Returns
/// A [`DecayTable`] with all channels, partial widths, and BRs.
pub fn calculate_decay_table(model: &TheoreticalModel, parent_id: &str) -> SpireResult<DecayTable> {
    let parent = model
        .fields
        .iter()
        .find(|f| f.id == parent_id)
        .ok_or_else(|| {
            crate::SpireError::UnknownParticle(format!(
                "Parent particle '{}' not found in model",
                parent_id
            ))
        })?;

    let candidates = find_candidate_channels(model, parent_id);

    // Compute partial widths.
    let mut channels: Vec<DecayChannel> = Vec::new();
    for candidate in &candidates {
        if candidate.daughter_ids.len() != 2 {
            // Skip non-2-body decays for now.
            continue;
        }

        let partial_width =
            compute_partial_width_2body(model, parent, &candidate.daughter_ids, &candidate.vertex);

        if partial_width <= 0.0 {
            continue;
        }

        let final_state_names: Vec<String> = candidate
            .daughter_ids
            .iter()
            .map(|id| {
                model
                    .fields
                    .iter()
                    .find(|f| f.id == *id)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();

        channels.push(DecayChannel {
            final_state: candidate.daughter_ids.clone(),
            final_state_names,
            partial_width,
            branching_ratio: 0.0, // Set after total width is known.
            vertex_id: candidate.vertex.term_id.clone(),
        });
    }

    // Total width.
    let total_width: f64 = channels.iter().map(|c| c.partial_width).sum();

    // Compute branching ratios.
    if total_width > 0.0 {
        for ch in &mut channels {
            ch.branching_ratio = ch.partial_width / total_width;
        }
    }

    // Sort by BR descending.
    channels.sort_by(|a, b| b.branching_ratio.partial_cmp(&a.branching_ratio).unwrap());

    // Lifetime.
    let lifetime_seconds = if total_width > 0.0 {
        HBAR_GEV_S / total_width
    } else {
        f64::INFINITY
    };

    Ok(DecayTable {
        parent_id: parent_id.to_string(),
        parent_name: parent.name.clone(),
        parent_mass: parent.mass,
        total_width,
        lifetime_seconds,
        channels,
    })
}

// ===========================================================================
// SLHA Export
// ===========================================================================

impl DecayTable {
    /// Format this decay table as an SLHA `DECAY` block string.
    ///
    /// # Arguments
    /// * `pdg_code` — The PDG Monte Carlo particle numbering code.
    ///
    /// # Example Output
    /// ```text
    /// DECAY  23  2.495200e+00   # Z width
    ///     1.520000e-01   2    -11   11   # BR(Z -> e+ e-)
    ///     1.520000e-01   2    -13   13   # BR(Z -> mu+ mu-)
    /// ```
    pub fn to_slha_string(&self, pdg_code: i32) -> String {
        let mut out = String::new();

        // Header line.
        out.push_str(&format!(
            "DECAY  {}  {:.6e}   # {} width\n",
            pdg_code, self.total_width, self.parent_name
        ));

        // Channel lines.
        for ch in &self.channels {
            let n_daughters = ch.final_state.len();
            let daughter_list = ch
                .final_state
                .iter()
                .map(|id| format!("  {}", id))
                .collect::<Vec<_>>()
                .join("");
            let comment = ch.final_state_names.join(" ");
            out.push_str(&format!(
                "    {:.6e}   {}{}   # BR({} -> {})\n",
                ch.branching_ratio, n_daughters, daughter_list, self.parent_name, comment
            ));
        }

        out
    }
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_loader;

    /// Build the Standard Model from the bundled TOML data files.
    fn load_sm_model() -> TheoreticalModel {
        let particles_toml = include_str!("../../../data/particles.toml");
        let vertices_toml = include_str!("../../../data/sm_vertices.toml");
        data_loader::build_model(particles_toml, vertices_toml, "Standard Model").unwrap()
    }

    #[test]
    fn test_z_boson_has_decay_channels() {
        let model = load_sm_model();
        let channels = find_decay_channels(&model, "Z0");
        // Z should decay to e+e-, mu+mu-, nu_e nu_e, u u_bar, d d_bar at minimum.
        assert!(
            !channels.is_empty(),
            "Z boson should have at least one decay channel"
        );
    }

    #[test]
    fn test_photon_is_stable() {
        let model = load_sm_model();
        let channels = find_decay_channels(&model, "photon");
        assert!(
            channels.is_empty(),
            "Photon should have no decay channels (massless)"
        );
    }

    #[test]
    fn test_electron_is_stable() {
        let model = load_sm_model();
        let channels = find_decay_channels(&model, "e-");
        assert!(
            channels.is_empty(),
            "Electron should be stable (lightest charged lepton)"
        );
    }

    #[test]
    fn test_z_decay_table_has_positive_width() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        assert!(
            table.total_width > 0.0,
            "Z total width should be positive, got {}",
            table.total_width
        );
        assert!(!table.channels.is_empty(), "Z should have decay channels");
    }

    #[test]
    fn test_z_branching_ratios_sum_to_one() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        let br_sum: f64 = table.channels.iter().map(|c| c.branching_ratio).sum();
        assert!(
            (br_sum - 1.0).abs() < 1e-10,
            "Z BRs should sum to 1.0, got {}",
            br_sum
        );
    }

    #[test]
    fn test_z_decay_to_electron_pair_exists() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        let ee_channel = table
            .channels
            .iter()
            .find(|c| c.final_state.contains(&"e-".to_string()));
        assert!(ee_channel.is_some(), "Z → e+e- / e-e- channel should exist");
    }

    #[test]
    fn test_higgs_decay_table() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "H").unwrap();
        // Higgs should decay to at least e+e- (Yukawa coupling exists).
        assert!(table.total_width > 0.0, "Higgs should have positive width");
    }

    #[test]
    fn test_stable_particle_decay_table() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "photon").unwrap();
        assert_eq!(table.total_width, 0.0, "Photon should have zero width");
        assert!(table.channels.is_empty(), "Photon should have no channels");
        assert!(
            table.lifetime_seconds.is_infinite(),
            "Photon should have infinite lifetime"
        );
    }

    #[test]
    fn test_unknown_particle_error() {
        let model = load_sm_model();
        let result = calculate_decay_table(&model, "squark_L");
        assert!(result.is_err(), "Unknown particle should return error");
    }

    #[test]
    fn test_slha_output_format() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        let slha = table.to_slha_string(23);
        assert!(slha.starts_with("DECAY"), "SLHA should start with DECAY");
        assert!(slha.contains("23"), "SLHA should contain PDG code");
        assert!(slha.contains("# BR("), "SLHA should have BR comments");
    }

    #[test]
    fn test_decay_table_sorted_by_br() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        for i in 1..table.channels.len() {
            assert!(
                table.channels[i - 1].branching_ratio >= table.channels[i].branching_ratio,
                "Channels should be sorted by BR descending"
            );
        }
    }

    #[test]
    fn test_colour_factor_quarks() {
        let model = load_sm_model();
        let nc = colour_factor(&model, &["u".to_string(), "u".to_string()]);
        assert_eq!(nc, 3.0, "Quark pair should give N_c = 3");
    }

    #[test]
    fn test_colour_factor_leptons() {
        let model = load_sm_model();
        let nc = colour_factor(&model, &["e-".to_string(), "e-".to_string()]);
        assert_eq!(nc, 1.0, "Lepton pair should give N_c = 1");
    }

    #[test]
    fn test_w_boson_has_decay_channels() {
        let model = load_sm_model();
        let channels = find_decay_channels(&model, "W-");
        assert!(
            !channels.is_empty(),
            "W- boson should have decay channels (e.g. W- → e- nu_e)"
        );
    }

    #[test]
    fn test_z_lifetime_positive() {
        let model = load_sm_model();
        let table = calculate_decay_table(&model, "Z0").unwrap();
        assert!(
            table.lifetime_seconds > 0.0 && table.lifetime_seconds.is_finite(),
            "Z lifetime should be positive and finite, got {}",
            table.lifetime_seconds
        );
    }

    #[test]
    fn test_find_decay_channels_public_api() {
        let model = load_sm_model();
        let channels = find_decay_channels(&model, "Z0");
        for (daughters, vertex_id) in &channels {
            assert!(daughters.len() >= 2, "Each channel must have ≥ 2 daughters");
            assert!(!vertex_id.is_empty(), "Vertex ID must not be empty");
        }
    }
}
