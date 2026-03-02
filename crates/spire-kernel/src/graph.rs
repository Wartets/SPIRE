//! # Graph — Feynman Diagram Topology
//!
//! This module handles the generation, manipulation, and validation of Feynman
//! diagrams represented as directed graphs. Nodes represent interaction vertices
//! and external legs; edges represent propagators or external particle lines.
//!
//! Built on `petgraph`, it implements algorithms for:
//! - **Tree-level** topological enumeration of all distinct diagrams for
//!   $1 \to 2$, $2 \to 1$, and $2 \to 2$ processes.
//! - Channel identification and isolation (s, t, u Mandelstam channels).
//! - Symmetry factor calculation ($1/n!$ for identical final-state particles).
//! - Loop-order classification.
//!
//! # Physical Mapping
//!
//! A Feynman diagram is a **directed graph** (`DiGraph<Node, Edge>`):
//! - **Nodes** are either *external states* (incoming/outgoing particles) or
//!   *interaction vertices* (carrying a [`VertexFactor`] from the Lagrangian).
//! - **Edges** represent the flow of a particle: external legs carry on-shell
//!   particles, internal lines carry off-shell virtual propagators.
//!
//! The `TheoreticalModel` dictates which vertices are allowed. Two nodes may
//! only be connected if a [`VertexFactor`] exists in the model for the fields
//! meeting at each vertex.

use petgraph::graph::DiGraph;
use serde::{Deserialize, Serialize};

use crate::lagrangian::{Propagator, TheoreticalModel, VertexFactor};
use crate::ontology::{Field, Particle};
use crate::s_matrix::Reaction;
use crate::SpireResult;

// ---------------------------------------------------------------------------
// Core Data Structures
// ---------------------------------------------------------------------------

/// Classification of a node in the Feynman graph.
///
/// Each node is either an external asymptotic state or an internal interaction
/// vertex governed by a Feynman rule derived from the Lagrangian.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    /// An external leg — an asymptotic **incoming** particle.
    ExternalIncoming(Particle),
    /// An external leg — an asymptotic **outgoing** particle.
    ExternalOutgoing(Particle),
    /// An interaction vertex carrying a specific vertex factor from the Lagrangian.
    InternalVertex(VertexFactor),
}

/// A **Node** in the Feynman diagram graph.
///
/// Represents either an external particle leg or an interaction vertex.
/// External nodes have exactly one edge; vertex nodes have edges equal to
/// the number of legs in the vertex factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique index of this node within the diagram.
    pub id: usize,
    /// What kind of node this is (external leg or vertex).
    pub kind: NodeKind,
    /// Position hint for rendering (x, y). Not physically meaningful.
    pub position: Option<(f64, f64)>,
}

/// An **Edge** in the Feynman diagram graph, representing a particle line.
///
/// Carries the propagating field identity, the Feynman-rule propagator
/// (for internal lines), and momentum labelling.
///
/// - **External edges** connect an external-state node to a vertex node.
///   They carry the on-shell particle and have `propagator = None`.
/// - **Internal edges** connect two vertex nodes via a virtual propagator.
///   They carry the off-shell field and have a derived [`Propagator`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// The field propagating along this line.
    pub field: Field,
    /// The propagator Feynman rule (present for internal lines, absent for external).
    pub propagator: Option<Propagator>,
    /// Symbolic label for the 4-momentum flowing through this edge (e.g., `"p1"`, `"k"`).
    pub momentum_label: String,
    /// Whether this is an external (on-shell) or internal (off-shell) line.
    pub is_external: bool,
}

/// The Mandelstam channel classification of a diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    /// s-channel: total energy flows through a single propagator (annihilation/resonance).
    S,
    /// t-channel: momentum transfer between non-annihilating particles (exchange).
    T,
    /// u-channel: crossed exchange (identical final-state particles interchanged).
    U,
}

/// The loop order classification of a diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopOrder {
    /// Tree-level (Born approximation, no loops).
    Tree,
    /// One-loop correction.
    OneLoop,
    /// Two-loop correction.
    TwoLoop,
    /// Higher-order ($n$-loop).
    NLoop(u32),
}

/// A complete **FeynmanGraph** — the topological representation of a single
/// Feynman diagram.
///
/// Wraps a `petgraph::DiGraph` of [`Node`]s and [`Edge`]s, together with
/// metadata about channel classification, loop order, and symmetry factors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeynmanGraph {
    /// Unique identifier for this diagram within a set of topologies.
    pub id: usize,
    /// The underlying directed graph structure.
    ///
    /// Serialization note: `petgraph::DiGraph` does not implement Serde traits,
    /// so this field is skipped during serialization. The `nodes` and `edges`
    /// vectors provide a serializable snapshot.
    #[serde(skip)]
    pub graph: DiGraph<Node, Edge>,
    /// Serializable list of nodes (for transport to frontend).
    pub nodes: Vec<Node>,
    /// Serializable list of edges with `(source_id, target_id, edge_data)`.
    pub edges: Vec<(usize, usize, Edge)>,
    /// The Mandelstam channel(s) this diagram contributes to.
    pub channels: Vec<Channel>,
    /// The loop order of this diagram.
    pub loop_order: LoopOrder,
    /// The symmetry factor $1/S$ for this diagram.
    ///
    /// For tree-level diagrams, $S = n!$ where $n$ is the number of identical
    /// particles in the final state, giving a factor of $1/n!$.
    pub symmetry_factor: f64,
    /// Whether this is a connected diagram.
    pub is_connected: bool,
}

/// A collection of all topologically distinct Feynman diagrams generated for
/// a specific reaction at a given perturbative order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySet {
    /// Descriptive label for the reaction these diagrams correspond to.
    pub reaction_id: String,
    /// The maximum loop order included.
    pub max_loop_order: LoopOrder,
    /// All generated diagrams.
    pub diagrams: Vec<FeynmanGraph>,
    /// Total number of diagrams found.
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Internal Helpers — Vertex Matching
// ---------------------------------------------------------------------------

/// Check whether a set of field IDs can meet at a vertex defined in the model.
///
/// Vertex matching is **order-independent**: the field ID lists are sorted
/// before comparison. This mirrors the physics: vertices are symmetric
/// under permutation of the attached legs.
fn find_matching_vertex<'a>(
    model: &'a TheoreticalModel,
    field_ids: &[&str],
) -> Option<&'a VertexFactor> {
    let mut sorted_query: Vec<&str> = field_ids.to_vec();
    sorted_query.sort();

    model.vertex_factors.iter().find(|vf| {
        let mut sorted_fields: Vec<&str> = vf.field_ids.iter().map(|s| s.as_str()).collect();
        sorted_fields.sort();
        sorted_fields == sorted_query
    })
}

/// Map a particle to its field ID for vertex lookup.
fn base_field_id(particle: &Particle) -> &str {
    &particle.field.id
}

/// Look up a field in the model by its string ID.
fn lookup_field<'a>(model: &'a TheoreticalModel, id: &str) -> Option<&'a Field> {
    model.fields.iter().find(|f| f.id == id)
}

/// Get the electric charge (in 3Q units) of a field by its ID.
fn field_charge(model: &TheoreticalModel, id: &str) -> i32 {
    lookup_field(model, id)
        .map(|f| f.quantum_numbers.electric_charge.0 as i32)
        .unwrap_or(0)
}

/// Find all valid s-channel mediator fields.
///
/// For an s-channel $A + B \to X \to C + D$:
/// - Vertex 1: fields `[A, B, X]` must exist in the model.
/// - Vertex 2: fields `[X, C, D]` must exist in the model.
/// - Charge conservation: $Q(X) = Q(A) + Q(B)$ (mediator carries total
///   charge of initial state).
///
/// Returns all valid mediator fields and their vertex factors.
fn find_s_channel_mediators<'a>(
    model: &'a TheoreticalModel,
    in1: &str,
    in2: &str,
    out1: &str,
    out2: &str,
) -> Vec<(&'a Field, &'a VertexFactor, &'a VertexFactor)> {
    let mut results = Vec::new();
    let q_total = field_charge(model, in1) + field_charge(model, in2);

    for field in &model.fields {
        let mediator_id = field.id.as_str();

        // s-channel: mediator carries total initial-state charge
        if field.quantum_numbers.electric_charge.0 as i32 != q_total {
            continue;
        }

        // Check if vertex (in1, in2, mediator) exists
        if let Some(vf1) = find_matching_vertex(model, &[in1, in2, mediator_id]) {
            // Check if vertex (mediator, out1, out2) exists
            if let Some(vf2) = find_matching_vertex(model, &[mediator_id, out1, out2]) {
                results.push((field, vf1, vf2));
            }
        }
    }

    results
}

/// Find all valid exchange mediator fields for t-channel or u-channel diagrams.
///
/// For a t-channel $A + B \to C + D$ with exchange between $A \leftrightarrow C$:
/// - Vertex 1: fields `[A, C, X]` must exist.
/// - Vertex 2: fields `[B, D, X]` must exist.
///
/// Returns all valid exchange fields and their vertex factors.
fn find_exchange_mediators<'a>(
    model: &'a TheoreticalModel,
    leg_a: &str,
    leg_b: &str,
    leg_c: &str,
    leg_d: &str,
) -> Vec<(&'a Field, &'a VertexFactor, &'a VertexFactor)> {
    let mut results = Vec::new();

    for field in &model.fields {
        let exchg_id = field.id.as_str();

        // Vertex 1: (leg_a, leg_c, exchg)
        if let Some(vf1) = find_matching_vertex(model, &[leg_a, leg_c, exchg_id]) {
            // Vertex 2: (leg_b, leg_d, exchg)
            if let Some(vf2) = find_matching_vertex(model, &[leg_b, leg_d, exchg_id]) {
                results.push((field, vf1, vf2));
            }
        }
    }

    results
}

/// Build a `FeynmanGraph` for an **s-channel** topology.
///
/// ```text
///   [In1] ──→ (V1) ──propagator──→ (V2) ──→ [Out1]
///   [In2] ──→ (V1)                 (V2) ──→ [Out2]
/// ```
fn build_s_channel_graph(
    diagram_id: usize,
    in1: &Particle,
    in2: &Particle,
    out1: &Particle,
    out2: &Particle,
    mediator: &Field,
    mediator_propagator: Option<&Propagator>,
    vf1: &VertexFactor,
    vf2: &VertexFactor,
) -> FeynmanGraph {
    let mut graph = DiGraph::<Node, Edge>::new();

    // External nodes
    let n_in1 = graph.add_node(Node {
        id: 0,
        kind: NodeKind::ExternalIncoming(in1.clone()),
        position: Some((-2.0, 1.0)),
    });
    let n_in2 = graph.add_node(Node {
        id: 1,
        kind: NodeKind::ExternalIncoming(in2.clone()),
        position: Some((-2.0, -1.0)),
    });

    // Internal vertex nodes
    let n_v1 = graph.add_node(Node {
        id: 2,
        kind: NodeKind::InternalVertex(vf1.clone()),
        position: Some((-1.0, 0.0)),
    });
    let n_v2 = graph.add_node(Node {
        id: 3,
        kind: NodeKind::InternalVertex(vf2.clone()),
        position: Some((1.0, 0.0)),
    });

    // External outgoing nodes
    let n_out1 = graph.add_node(Node {
        id: 4,
        kind: NodeKind::ExternalOutgoing(out1.clone()),
        position: Some((2.0, 1.0)),
    });
    let n_out2 = graph.add_node(Node {
        id: 5,
        kind: NodeKind::ExternalOutgoing(out2.clone()),
        position: Some((2.0, -1.0)),
    });

    // External edges: incoming → vertex 1
    graph.add_edge(n_in1, n_v1, Edge {
        field: in1.field.clone(),
        propagator: None,
        momentum_label: "p1".into(),
        is_external: true,
    });
    graph.add_edge(n_in2, n_v1, Edge {
        field: in2.field.clone(),
        propagator: None,
        momentum_label: "p2".into(),
        is_external: true,
    });

    // Internal propagator: vertex 1 → vertex 2
    graph.add_edge(n_v1, n_v2, Edge {
        field: mediator.clone(),
        propagator: mediator_propagator.cloned(),
        momentum_label: "q".into(),
        is_external: false,
    });

    // External edges: vertex 2 → outgoing
    graph.add_edge(n_v2, n_out1, Edge {
        field: out1.field.clone(),
        propagator: None,
        momentum_label: "p3".into(),
        is_external: true,
    });
    graph.add_edge(n_v2, n_out2, Edge {
        field: out2.field.clone(),
        propagator: None,
        momentum_label: "p4".into(),
        is_external: true,
    });

    // Snapshot for serialization
    let nodes: Vec<Node> = graph.node_weights().cloned().collect();
    let edges: Vec<(usize, usize, Edge)> = graph
        .edge_indices()
        .map(|ei| {
            let (src, tgt) = graph.edge_endpoints(ei).unwrap();
            (src.index(), tgt.index(), graph[ei].clone())
        })
        .collect();

    FeynmanGraph {
        id: diagram_id,
        graph,
        nodes,
        edges,
        channels: vec![Channel::S],
        loop_order: LoopOrder::Tree,
        symmetry_factor: 1.0, // will be set by caller
        is_connected: true,
    }
}

/// Build a `FeynmanGraph` for a **t-channel or u-channel** exchange topology.
///
/// ```text
///   [In1] ──→ (V1) ──→ [Out1]
///                |
///            propagator (exchange)
///                |
///   [In2] ──→ (V2) ──→ [Out2]
/// ```
fn build_exchange_graph(
    diagram_id: usize,
    in1: &Particle,
    in2: &Particle,
    out1: &Particle,
    out2: &Particle,
    exchange: &Field,
    exchange_propagator: Option<&Propagator>,
    vf1: &VertexFactor,
    vf2: &VertexFactor,
    channel: Channel,
) -> FeynmanGraph {
    let mut graph = DiGraph::<Node, Edge>::new();

    // External nodes
    let n_in1 = graph.add_node(Node {
        id: 0,
        kind: NodeKind::ExternalIncoming(in1.clone()),
        position: Some((-2.0, 1.0)),
    });
    let n_in2 = graph.add_node(Node {
        id: 1,
        kind: NodeKind::ExternalIncoming(in2.clone()),
        position: Some((-2.0, -1.0)),
    });

    // Vertex nodes
    let n_v1 = graph.add_node(Node {
        id: 2,
        kind: NodeKind::InternalVertex(vf1.clone()),
        position: Some((0.0, 1.0)),
    });
    let n_v2 = graph.add_node(Node {
        id: 3,
        kind: NodeKind::InternalVertex(vf2.clone()),
        position: Some((0.0, -1.0)),
    });

    // Outgoing external nodes
    let n_out1 = graph.add_node(Node {
        id: 4,
        kind: NodeKind::ExternalOutgoing(out1.clone()),
        position: Some((2.0, 1.0)),
    });
    let n_out2 = graph.add_node(Node {
        id: 5,
        kind: NodeKind::ExternalOutgoing(out2.clone()),
        position: Some((2.0, -1.0)),
    });

    // Incoming → vertex
    graph.add_edge(n_in1, n_v1, Edge {
        field: in1.field.clone(),
        propagator: None,
        momentum_label: "p1".into(),
        is_external: true,
    });
    graph.add_edge(n_in2, n_v2, Edge {
        field: in2.field.clone(),
        propagator: None,
        momentum_label: "p2".into(),
        is_external: true,
    });

    // Exchange propagator: V1 → V2
    graph.add_edge(n_v1, n_v2, Edge {
        field: exchange.clone(),
        propagator: exchange_propagator.cloned(),
        momentum_label: "q".into(),
        is_external: false,
    });

    // Vertex → outgoing
    graph.add_edge(n_v1, n_out1, Edge {
        field: out1.field.clone(),
        propagator: None,
        momentum_label: "p3".into(),
        is_external: true,
    });
    graph.add_edge(n_v2, n_out2, Edge {
        field: out2.field.clone(),
        propagator: None,
        momentum_label: "p4".into(),
        is_external: true,
    });

    let nodes: Vec<Node> = graph.node_weights().cloned().collect();
    let edges: Vec<(usize, usize, Edge)> = graph
        .edge_indices()
        .map(|ei| {
            let (src, tgt) = graph.edge_endpoints(ei).unwrap();
            (src.index(), tgt.index(), graph[ei].clone())
        })
        .collect();

    FeynmanGraph {
        id: diagram_id,
        graph,
        nodes,
        edges,
        channels: vec![channel],
        loop_order: LoopOrder::Tree,
        symmetry_factor: 1.0,
        is_connected: true,
    }
}

/// Build a `FeynmanGraph` for a **1 → 2** decay or **2 → 1** fusion (single vertex).
///
/// ```text
///   [In] ──→ (V) ──→ [Out 1]
///                ──→ [Out 2]
/// ```
fn build_single_vertex_graph(
    diagram_id: usize,
    incoming: &[Particle],
    outgoing: &[Particle],
    vf: &VertexFactor,
) -> FeynmanGraph {
    let mut graph = DiGraph::<Node, Edge>::new();
    let mut node_id = 0usize;

    // Incoming external nodes
    let mut incoming_indices = Vec::new();
    for (i, p) in incoming.iter().enumerate() {
        let ni = graph.add_node(Node {
            id: node_id,
            kind: NodeKind::ExternalIncoming(p.clone()),
            position: Some((-2.0, i as f64)),
        });
        incoming_indices.push(ni);
        node_id += 1;
    }

    // Single vertex node
    let n_v = graph.add_node(Node {
        id: node_id,
        kind: NodeKind::InternalVertex(vf.clone()),
        position: Some((0.0, 0.0)),
    });
    node_id += 1;

    // Outgoing external nodes
    let mut outgoing_indices = Vec::new();
    for (i, p) in outgoing.iter().enumerate() {
        let ni = graph.add_node(Node {
            id: node_id,
            kind: NodeKind::ExternalOutgoing(p.clone()),
            position: Some((2.0, i as f64)),
        });
        outgoing_indices.push(ni);
        node_id += 1;
    }

    // Incoming edges → vertex
    for (i, &ni) in incoming_indices.iter().enumerate() {
        graph.add_edge(ni, n_v, Edge {
            field: incoming[i].field.clone(),
            propagator: None,
            momentum_label: format!("p{}", i + 1),
            is_external: true,
        });
    }

    // Vertex → outgoing edges
    for (i, &ni) in outgoing_indices.iter().enumerate() {
        graph.add_edge(n_v, ni, Edge {
            field: outgoing[i].field.clone(),
            propagator: None,
            momentum_label: format!("p{}", incoming.len() + i + 1),
            is_external: true,
        });
    }

    let nodes: Vec<Node> = graph.node_weights().cloned().collect();
    let edges: Vec<(usize, usize, Edge)> = graph
        .edge_indices()
        .map(|ei| {
            let (src, tgt) = graph.edge_endpoints(ei).unwrap();
            (src.index(), tgt.index(), graph[ei].clone())
        })
        .collect();

    FeynmanGraph {
        id: diagram_id,
        graph,
        nodes,
        edges,
        channels: vec![Channel::S], // single-vertex diagrams are s-channel
        loop_order: LoopOrder::Tree,
        symmetry_factor: 1.0,
        is_connected: true,
    }
}

/// Create a deduplication key for a diagram to avoid duplicate topologies.
fn diagram_key(channel: Channel, mediator_id: &str, vf1_id: &str, vf2_id: &str) -> String {
    format!("{:?}|{}|{}|{}", channel, mediator_id, vf1_id, vf2_id)
}

/// Compute the $1/n!$ symmetry factor from a list of final-state particles.
fn compute_final_state_symmetry_factor(particles: &[&Particle]) -> f64 {
    if particles.is_empty() {
        return 1.0;
    }

    let mut counts = std::collections::HashMap::<&str, u32>::new();
    for p in particles {
        *counts.entry(p.field.id.as_str()).or_insert(0) += 1;
    }

    let mut factor = 1.0;
    for &n in counts.values() {
        if n > 1 {
            factor /= factorial(n) as f64;
        }
    }
    factor
}

/// Compute $n!$ for small $n$.
fn factorial(n: u32) -> u32 {
    (1..=n).product()
}

// ---------------------------------------------------------------------------
// Public API — Diagram Generation and Manipulation
// ---------------------------------------------------------------------------

/// Generate all topologically distinct **tree-level** Feynman diagrams for a
/// given reaction and theoretical model.
///
/// Supports $1 \to 2$, $2 \to 1$, and $2 \to 2$ processes at tree level.
/// For $2 \to 2$ scattering, the algorithm systematically tries:
/// 1. **s-channel**: Merge incoming legs → single propagator → split to outgoing.
/// 2. **t-channel**: Exchange a virtual particle between the two fermion lines
///    ($\text{In}_1 \to \text{Out}_1$ emission, $\text{In}_2 \to \text{Out}_2$ absorption).
/// 3. **u-channel**: Crossed exchange ($\text{In}_1 \to \text{Out}_2$,
///    $\text{In}_2 \to \text{Out}_1$).
///
/// Each candidate topology is validated by checking that the meeting fields
/// at every vertex correspond to an allowed [`VertexFactor`] in the model.
///
/// # Arguments
/// * `reaction`  — The validated reaction defining the external legs.
/// * `model`     — The active theoretical model providing the allowed vertices.
/// * `max_order` — Maximum loop order (only `LoopOrder::Tree` currently supported).
///
/// # Returns
/// A [`TopologySet`] containing all valid, topologically distinct diagrams.
pub fn generate_topologies(
    reaction: &Reaction,
    model: &TheoreticalModel,
    _max_order: LoopOrder,
) -> SpireResult<TopologySet> {
    let initial = &reaction.initial.states;
    let final_st = &reaction.final_state.states;
    let n_in = initial.len();
    let n_out = final_st.len();

    let mut diagrams: Vec<FeynmanGraph> = Vec::new();
    let mut seen_keys = std::collections::HashSet::<String>::new();
    let mut next_id = 0usize;

    // Build label for the reaction
    let in_labels: Vec<&str> = initial.iter().map(|s| s.particle.field.id.as_str()).collect();
    let out_labels: Vec<&str> = final_st.iter().map(|s| s.particle.field.id.as_str()).collect();
    let reaction_label = format!("{} → {}", in_labels.join(" + "), out_labels.join(" + "));

    // Extract Particle references
    let in_particles: Vec<&Particle> = initial.iter().map(|s| &s.particle).collect();
    let out_particles: Vec<&Particle> = final_st.iter().map(|s| &s.particle).collect();

    match (n_in, n_out) {
        // ---------------------------------------------------------------
        // 1 → 2 decay (single vertex)
        // ---------------------------------------------------------------
        (1, 2) => {
            let ids = [
                base_field_id(in_particles[0]),
                base_field_id(out_particles[0]),
                base_field_id(out_particles[1]),
            ];
            if let Some(vf) = find_matching_vertex(model, &ids) {
                let incoming_ps: Vec<Particle> = in_particles.iter().map(|p| (*p).clone()).collect();
                let outgoing_ps: Vec<Particle> = out_particles.iter().map(|p| (*p).clone()).collect();
                let mut fg = build_single_vertex_graph(next_id, &incoming_ps, &outgoing_ps, vf);
                fg.symmetry_factor = compute_final_state_symmetry_factor(
                    &out_particles.iter().copied().collect::<Vec<_>>(),
                );
                diagrams.push(fg);
            }
        }

        // ---------------------------------------------------------------
        // 2 → 1 fusion (single vertex)
        // ---------------------------------------------------------------
        (2, 1) => {
            let ids = [
                base_field_id(in_particles[0]),
                base_field_id(in_particles[1]),
                base_field_id(out_particles[0]),
            ];
            if let Some(vf) = find_matching_vertex(model, &ids) {
                let incoming_ps: Vec<Particle> = in_particles.iter().map(|p| (*p).clone()).collect();
                let outgoing_ps: Vec<Particle> = out_particles.iter().map(|p| (*p).clone()).collect();
                let mut fg = build_single_vertex_graph(next_id, &incoming_ps, &outgoing_ps, vf);
                fg.symmetry_factor = 1.0;
                diagrams.push(fg);
            }
        }

        // ---------------------------------------------------------------
        // 2 → 2 scattering (s, t, u channels)
        // ---------------------------------------------------------------
        (2, 2) => {
            let id_in1 = base_field_id(in_particles[0]);
            let id_in2 = base_field_id(in_particles[1]);
            let id_out1 = base_field_id(out_particles[0]);
            let id_out2 = base_field_id(out_particles[1]);

            let sym_factor = compute_final_state_symmetry_factor(
                &out_particles.iter().copied().collect::<Vec<_>>(),
            );

            // --- s-channel ---
            // (In1 + In2) → X → (Out1 + Out2)
            for (med, vf1, vf2) in find_s_channel_mediators(model, id_in1, id_in2, id_out1, id_out2)
            {
                let key = diagram_key(Channel::S, &med.id, &vf1.term_id, &vf2.term_id);
                if seen_keys.insert(key) {
                    let prop = model.propagators.iter().find(|p| p.field_id == med.id);
                    let mut fg = build_s_channel_graph(
                        next_id,
                        in_particles[0],
                        in_particles[1],
                        out_particles[0],
                        out_particles[1],
                        med,
                        prop,
                        vf1,
                        vf2,
                    );
                    fg.symmetry_factor = sym_factor;
                    diagrams.push(fg);
                    next_id += 1;
                }
            }

            // --- t-channel ---
            // In1 emits exchange → Out1; In2 absorbs → Out2
            // Vertex 1: (In1, Out1, X), Vertex 2: (In2, Out2, X)
            for (exch, vf1, vf2) in
                find_exchange_mediators(model, id_in1, id_in2, id_out1, id_out2)
            {
                let key = diagram_key(Channel::T, &exch.id, &vf1.term_id, &vf2.term_id);
                if seen_keys.insert(key) {
                    let prop = model.propagators.iter().find(|p| p.field_id == exch.id);
                    let mut fg = build_exchange_graph(
                        next_id,
                        in_particles[0],
                        in_particles[1],
                        out_particles[0],
                        out_particles[1],
                        exch,
                        prop,
                        vf1,
                        vf2,
                        Channel::T,
                    );
                    fg.symmetry_factor = sym_factor;
                    diagrams.push(fg);
                    next_id += 1;
                }
            }

            // --- u-channel ---
            // In1 emits exchange → Out2; In2 absorbs → Out1
            // Vertex 1: (In1, Out2, X), Vertex 2: (In2, Out1, X)
            //
            // For the u-channel exchange, the vertex pair is
            //   V1 = [in1, out2, X], V2 = [in2, out1, X].
            // When this produces the SAME sorted vertex pair as the
            // t-channel (V1=[in1,out1,X], V2=[in2,out2,X]), both channels
            // are generated and kept — they differ by momentum routing.
            //
            // However, when the u-channel vertex pair is equivalent to an
            // s-channel annihilation pair and the graph topology would be
            // an exchange graph (not an annihilation graph), we skip it.
            // This happens when (in1,out2) and (in2,out1) represent the
            // same particle-antiparticle combination as the s-channel
            // vertices (in1,in2) and (out1,out2), AND they don't also
            // match an exchange pattern.
            //
            // Concretely: skip u-channel only for Bhabha-like processes
            // where the u-channel exchange vertices are identical (as sets)
            // to the s-channel annihilation vertices.
            let u_vertices_same_as_s = {
                let mut s_v1: Vec<&str> = vec![id_in1, id_in2];
                s_v1.sort();
                let mut s_v2: Vec<&str> = vec![id_out1, id_out2];
                s_v2.sort();
                let mut u_v1: Vec<&str> = vec![id_in1, id_out2];
                u_v1.sort();
                let mut u_v2: Vec<&str> = vec![id_in2, id_out1];
                u_v2.sort();
                // u-channel has same vertex external legs as s-channel
                (u_v1 == s_v1 && u_v2 == s_v2) || (u_v1 == s_v2 && u_v2 == s_v1)
            };

            // Also skip if u-channel exchange is identical to t-channel
            // (same vertex structure AND same vertex factor IDs — this
            // only matters when all four particles are identical)
            let u_vertices_same_as_t = {
                let mut t_v1: Vec<&str> = vec![id_in1, id_out1];
                t_v1.sort();
                let mut t_v2: Vec<&str> = vec![id_in2, id_out2];
                t_v2.sort();
                let mut u_v1: Vec<&str> = vec![id_in1, id_out2];
                u_v1.sort();
                let mut u_v2: Vec<&str> = vec![id_in2, id_out1];
                u_v2.sort();
                (u_v1 == t_v1 && u_v2 == t_v2) || (u_v1 == t_v2 && u_v2 == t_v1)
            };

            // If u has same vertex structure as t, they are BOTH generated
            // since they have different momentum routing. If u has same
            // vertex structure as s AND is not also a valid exchange topology
            // independent of s, skip it.
            if !u_vertices_same_as_s || u_vertices_same_as_t {
                for (exch, vf1, vf2) in
                    find_exchange_mediators(model, id_in1, id_in2, id_out2, id_out1)
                {
                    let key = diagram_key(Channel::U, &exch.id, &vf1.term_id, &vf2.term_id);
                    if seen_keys.insert(key) {
                        let prop = model.propagators.iter().find(|p| p.field_id == exch.id);
                        let mut fg = build_exchange_graph(
                            next_id,
                            in_particles[0],
                            in_particles[1],
                            out_particles[1],
                            out_particles[0],
                            exch,
                            prop,
                            vf1,
                            vf2,
                            Channel::U,
                        );
                        fg.symmetry_factor = sym_factor;
                        diagrams.push(fg);
                        next_id += 1;
                    }
                }
            }
        }

        _ => {
            // Unsupported topology (N>2 legs on either side) — no diagrams.
        }
    }

    Ok(TopologySet {
        reaction_id: reaction_label,
        max_loop_order: LoopOrder::Tree,
        count: diagrams.len(),
        diagrams,
    })
}

/// Isolate diagrams belonging to a specific Mandelstam channel.
///
/// Filters the topology set to return only diagrams classified under the
/// requested channel (s, t, or u).
///
/// # Arguments
/// * `topologies` — The full set of generated topologies.
/// * `channel`    — The Mandelstam channel to isolate.
pub fn isolate_channels<'a>(
    topologies: &'a TopologySet,
    channel: Channel,
) -> SpireResult<Vec<&'a FeynmanGraph>> {
    let filtered: Vec<&FeynmanGraph> = topologies
        .diagrams
        .iter()
        .filter(|d| d.channels.contains(&channel))
        .collect();
    Ok(filtered)
}

/// Calculate the symmetry factor for a Feynman diagram based on identical
/// particles in the final state.
///
/// For tree-level diagrams, $n$ identical final-state particles give a
/// factor of $1/n!$.
///
/// # Arguments
/// * `diagram` — The Feynman diagram to compute the symmetry factor for.
pub fn calculate_symmetry_factors(diagram: &FeynmanGraph) -> SpireResult<f64> {
    let outgoing: Vec<&Particle> = diagram
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            NodeKind::ExternalOutgoing(p) => Some(p),
            _ => None,
        })
        .collect();

    Ok(compute_final_state_symmetry_factor(&outgoing))
}

/// Apply a crossing symmetry transformation to a diagram.
///
/// Reconfigures a scattering diagram into a decay or annihilation diagram
/// by swapping an incoming leg with an outgoing leg (and conjugating the
/// corresponding particle).
///
/// # Arguments
/// * `diagram` — The diagram to transform.
/// * `leg_in`  — Index of the incoming external leg to cross.
/// * `leg_out` — Index of the outgoing external leg to cross.
pub fn apply_crossing_symmetry(
    _diagram: &FeynmanGraph,
    _leg_in: usize,
    _leg_out: usize,
) -> SpireResult<FeynmanGraph> {
    // Crossing symmetry transformation is planned for a future phase.
    todo!("Crossing symmetry transformation not yet implemented")
}

/// Classify the loop order of a given diagram.
///
/// For a connected graph, the number of independent loops is:
/// $$L = E_{\text{internal}} - V_{\text{internal}} + 1$$
/// where $E_{\text{internal}}$ is the number of internal edges (propagators)
/// and $V_{\text{internal}}$ is the number of internal vertex nodes.
pub fn classify_loop_order(diagram: &FeynmanGraph) -> LoopOrder {
    let n_internal_edges = diagram
        .edges
        .iter()
        .filter(|(_, _, e)| !e.is_external)
        .count();

    let n_internal_vertices = diagram
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::InternalVertex(_)))
        .count();

    if n_internal_vertices == 0 {
        return LoopOrder::Tree;
    }

    // L = E_internal - V_internal + 1
    let loops = (n_internal_edges as i32) - (n_internal_vertices as i32) + 1;

    match loops {
        l if l <= 0 => LoopOrder::Tree,
        1 => LoopOrder::OneLoop,
        2 => LoopOrder::TwoLoop,
        n => LoopOrder::NLoop(n as u32),
    }
}

/// Detect and remove vacuum bubble (disconnected) sub-diagrams.
///
/// Vacuum bubbles do not contribute to the S-matrix and are excluded from
/// physical amplitude calculations. Returns the number of removed diagrams.
pub fn remove_vacuum_bubbles(topologies: &mut TopologySet) -> SpireResult<usize> {
    let before = topologies.diagrams.len();
    topologies.diagrams.retain(|d| d.is_connected);
    let removed = before - topologies.diagrams.len();
    topologies.count = topologies.diagrams.len();
    Ok(removed)
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lagrangian::{
        derive_propagator, derive_vertex_factor, LagrangianTerm, LagrangianTermKind,
    };
    use crate::ontology::*;
    use crate::s_matrix::{AsymptoticState, Reaction};

    // -----------------------------------------------------------------------
    // Field helper constructors
    // -----------------------------------------------------------------------

    fn electron_field() -> Field {
        Field {
            id: "e-".into(),
            name: "Electron".into(),
            symbol: "e^-".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakNC],
        }
    }

    fn positron_field() -> Field {
        Field {
            id: "e+".into(),
            name: "Positron".into(),
            symbol: "e^+".into(),
            mass: 0.000511,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: -1, muon: 0, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakNC],
        }
    }

    fn muon_field() -> Field {
        Field {
            id: "mu-".into(),
            name: "Muon".into(),
            symbol: "μ^-".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(-3),
                weak_isospin: WeakIsospin(-1),
                hypercharge: Hypercharge(-3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletDown,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakNC],
        }
    }

    fn antimuon_field() -> Field {
        Field {
            id: "mu+".into(),
            name: "Antimuon".into(),
            symbol: "μ^+".into(),
            mass: 0.10566,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(3),
                weak_isospin: WeakIsospin(1),
                hypercharge: Hypercharge(3),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: -1, tau: 0 },
                spin: Spin(1),
                parity: Parity::Even,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::DoubletUp,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic, InteractionType::WeakNC],
        }
    }

    fn photon_field() -> Field {
        Field {
            id: "photon".into(),
            name: "Photon".into(),
            symbol: "γ".into(),
            mass: 0.0,
            width: 0.0,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(2),
                parity: Parity::Odd,
                charge_conjugation: ChargeConjugation::Odd,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::Electromagnetic],
        }
    }

    fn z_boson_field() -> Field {
        Field {
            id: "Z0".into(),
            name: "Z Boson".into(),
            symbol: "Z^0".into(),
            mass: 91.1876,
            width: 2.4952,
            quantum_numbers: QuantumNumbers {
                electric_charge: ElectricCharge(0),
                weak_isospin: WeakIsospin(0),
                hypercharge: Hypercharge(0),
                baryon_number: BaryonNumber(0),
                lepton_numbers: LeptonNumbers { electron: 0, muon: 0, tau: 0 },
                spin: Spin(2),
                parity: Parity::Odd,
                charge_conjugation: ChargeConjugation::Undefined,
                color: ColorRepresentation::Singlet,
                weak_multiplet: WeakMultiplet::Singlet,
                representations: vec![],
            },
            interactions: vec![InteractionType::WeakNC],
        }
    }

    // -----------------------------------------------------------------------
    // Test model construction
    //
    // In QFT a vertex like ēeγ handles all crossing configurations. The TOML
    // vertex definitions use canonical field IDs. For our topology generator
    // to correctly find diagrams for processes involving antiparticles (e+),
    // we must include explicit vertex entries with the antiparticle IDs.
    //
    // Vertices included:
    //   QED:  [e-,photon,e-], [e+,photon,e+]
    //   Annihilation: [e-,e+,photon], [e-,e+,Z0]
    //   NC:   [e-,e-,Z0], [e+,e+,Z0]
    //   Muon QED: [mu-,photon,mu-], [mu+,photon,mu+]
    //   Muon annihilation: [mu-,mu+,photon], [mu-,mu+,Z0]
    //   Muon NC: [mu-,mu-,Z0], [mu+,mu+,Z0]
    // -----------------------------------------------------------------------

    fn make_full_test_model() -> TheoreticalModel {
        let fields = vec![
            electron_field(),
            positron_field(),
            muon_field(),
            antimuon_field(),
            photon_field(),
            z_boson_field(),
        ];

        let terms = vec![
            // QED: e−γe−
            LagrangianTerm {
                id: "qed_eea".into(),
                description: "QED e-γ vertex".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["e-".into(), "photon".into(), "e-".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // QED: e+γe+
            LagrangianTerm {
                id: "qed_epa".into(),
                description: "QED e+γ vertex".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["e+".into(), "photon".into(), "e+".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // Annihilation: e−e+γ
            LagrangianTerm {
                id: "qed_annihilation_a".into(),
                description: "e+e− annihilation (photon)".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["e-".into(), "e+".into(), "photon".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // Annihilation: e−e+Z
            LagrangianTerm {
                id: "nc_annihilation_z".into(),
                description: "e+e− annihilation (Z)".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["e-".into(), "e+".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // NC: e−e−Z
            LagrangianTerm {
                id: "nc_eez".into(),
                description: "NC e−Z vertex".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["e-".into(), "e-".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // NC: e+e+Z
            LagrangianTerm {
                id: "nc_epz".into(),
                description: "NC e+Z vertex".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["e+".into(), "e+".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // QED: μ−γμ−
            LagrangianTerm {
                id: "qed_mma".into(),
                description: "QED μ-γ vertex".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["mu-".into(), "photon".into(), "mu-".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // QED: μ+γμ+
            LagrangianTerm {
                id: "qed_mpa".into(),
                description: "QED μ+γ vertex".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["mu+".into(), "photon".into(), "mu+".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // Annihilation: μ−μ+γ
            LagrangianTerm {
                id: "qed_mu_annihilation_a".into(),
                description: "μ+μ− annihilation (photon)".into(),
                coupling_symbol: "e".into(),
                coupling_value: Some(0.303),
                field_ids: vec!["mu-".into(), "mu+".into(), "photon".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::Electromagnetic,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // Annihilation: μ−μ+Z
            LagrangianTerm {
                id: "nc_mu_annihilation_z".into(),
                description: "μ+μ− annihilation (Z)".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["mu-".into(), "mu+".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // NC: μ−μ−Z
            LagrangianTerm {
                id: "nc_mmz".into(),
                description: "NC μ−Z vertex".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["mu-".into(), "mu-".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
            // NC: μ+μ+Z
            LagrangianTerm {
                id: "nc_mpz".into(),
                description: "NC μ+Z vertex".into(),
                coupling_symbol: "g_z".into(),
                coupling_value: Some(0.7397),
                field_ids: vec!["mu+".into(), "mu+".into(), "Z0".into()],
                lorentz_structure: "gamma_mu".into(),
                interaction_type: InteractionType::WeakNC,
                term_kind: LagrangianTermKind::Interaction,
                operator_dimension: None,
            },
        ];

        let propagators = fields.iter().map(|f| derive_propagator(f).unwrap()).collect();
        let vertex_factors = terms.iter().map(|t| derive_vertex_factor(t).unwrap()).collect();

        TheoreticalModel {
            name: "Full QED+WeakNC Test".into(),
            description: "Test model with all crossing vertices".into(),
            fields,
            terms,
            vertex_factors,
            propagators,
            gauge_symmetry: None,
        }
    }

    // -----------------------------------------------------------------------
    // Helper: build a Reaction from field references
    // -----------------------------------------------------------------------

    fn make_reaction(in_fields: &[Field], out_fields: &[Field], cms_energy: f64) -> Reaction {
        let n_in = in_fields.len().max(1) as f64;
        let n_out = out_fields.len().max(1) as f64;
        let e_in = cms_energy / n_in;
        let e_out = cms_energy / n_out;

        let initial_states: Vec<QuantumState> = in_fields
            .iter()
            .map(|f| initialize_state(f, [e_in, 0.0, 0.0, 0.0], None).unwrap())
            .collect();

        let final_states: Vec<QuantumState> = out_fields
            .iter()
            .map(|f| initialize_state(f, [e_out, 0.0, 0.0, 0.0], None).unwrap())
            .collect();

        let s = cms_energy * cms_energy;

        Reaction {
            initial: AsymptoticState {
                states: initial_states,
                invariant_mass_sq: s,
            },
            final_state: AsymptoticState {
                states: final_states,
                invariant_mass_sq: s,
            },
            is_valid: true,
            violation_diagnostics: vec![],
            interaction_types: vec![InteractionType::Electromagnetic],
            mediating_bosons: vec![],
            perturbative_order: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Structural / serde tests
    // -----------------------------------------------------------------------

    #[test]
    fn channel_equality() {
        assert_eq!(Channel::S, Channel::S);
        assert_ne!(Channel::S, Channel::T);
        assert_ne!(Channel::T, Channel::U);
    }

    #[test]
    fn loop_order_serde() {
        let lo = LoopOrder::Tree;
        let json = serde_json::to_string(&lo).unwrap();
        assert_eq!(json, "\"Tree\"");
        let lo2: LoopOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(lo, lo2);
    }

    #[test]
    fn feynman_graph_serde_roundtrip() {
        let fg = FeynmanGraph {
            id: 0,
            graph: DiGraph::new(),
            nodes: vec![],
            edges: vec![],
            channels: vec![Channel::S],
            loop_order: LoopOrder::Tree,
            symmetry_factor: 0.5,
            is_connected: true,
        };
        let json = serde_json::to_string(&fg).unwrap();
        let fg2: FeynmanGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(fg2.id, 0);
        assert_eq!(fg2.channels, vec![Channel::S]);
        assert!((fg2.symmetry_factor - 0.5).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // 5.4.1: Bhabha Scattering (e+ e- → e+ e-)
    //
    // Tree-level diagrams (4 total):
    //   s-channel γ: V1=[e-,e+,photon], V2=[e-,e+,photon]  → mediator=photon
    //   s-channel Z: V1=[e-,e+,Z0],     V2=[e-,e+,Z0]      → mediator=Z
    //   t-channel γ: V1=[e-,e-,photon], V2=[e+,e+,photon]   → exchange=photon
    //   t-channel Z: V1=[e-,e-,Z0],     V2=[e+,e+,Z0]       → exchange=Z
    // -----------------------------------------------------------------------

    #[test]
    fn bhabha_topology_count() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert_eq!(
            topos.count, 4,
            "Bhabha: expected 4 diagrams (2s + 2t), got {}",
            topos.count
        );
    }

    #[test]
    fn bhabha_has_s_channel() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let s_diagrams = isolate_channels(&topos, Channel::S).unwrap();
        assert_eq!(s_diagrams.len(), 2, "Bhabha: expected 2 s-channel (γ,Z)");
    }

    #[test]
    fn bhabha_has_t_channel() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let t_diagrams = isolate_channels(&topos, Channel::T).unwrap();
        assert_eq!(t_diagrams.len(), 2, "Bhabha: expected 2 t-channel (γ,Z)");
    }

    // -----------------------------------------------------------------------
    // 5.4.2: Møller Scattering (e- e- → e- e-)
    //
    // Tree-level diagrams (4 total):
    //   t-channel γ: V1=[e-,e-,photon], V2=[e-,e-,photon]
    //   t-channel Z: V1=[e-,e-,Z0],     V2=[e-,e-,Z0]
    //   u-channel γ: same vertices, crossed outgoing assignment
    //   u-channel Z: same
    // No s-channel (no e−e−→X vertex, would violate charge).
    // -----------------------------------------------------------------------

    #[test]
    fn moller_topology_count() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), electron_field()],
            &[electron_field(), electron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert_eq!(topos.count, 4, "Møller: expected 4 diagrams (2t + 2u)");
    }

    #[test]
    fn moller_has_t_and_u_channels() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), electron_field()],
            &[electron_field(), electron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let t = isolate_channels(&topos, Channel::T).unwrap().len();
        let u = isolate_channels(&topos, Channel::U).unwrap().len();
        assert_eq!(t, 2, "Møller: expected 2 t-channel diagrams");
        assert_eq!(u, 2, "Møller: expected 2 u-channel diagrams");
    }

    #[test]
    fn moller_has_no_s_channel() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), electron_field()],
            &[electron_field(), electron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let s = isolate_channels(&topos, Channel::S).unwrap();
        assert_eq!(s.len(), 0, "Møller should have no s-channel diagrams");
    }

    // -----------------------------------------------------------------------
    // 5.4.3: Electron-Positron Annihilation (e+ e- → γ γ)
    //
    // t-channel: V1=[e-,photon,e-], V2=[e+,photon,e+] with exchange=e-
    //   → find_exchange_mediators(e-, e+, photon, photon)
    //     X=e-: V1=[e-,photon,e-]✓, V2=[e+,photon,e-] sorted=[e+,e-,photon]
    //            matches [e-,e+,photon] (annihilation vertex) ✓
    //   → Also X=e+: V1=[e-,photon,e+] sorted=[e+,e-,photon] matches annihilation ✓
    //                 V2=[e+,photon,e+] ✓
    //
    // u-channel: same with outgoing swapped (photon,photon identical so same)
    //
    // Both e- and e+ exchange produce physically equivalent diagrams (same
    // fermion propagator, different arrow direction). We accept both as
    // distinct topology entries since they have different vertex factor combos.
    // -----------------------------------------------------------------------

    #[test]
    fn ee_annihilation_to_photons_has_t_and_u() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[photon_field(), photon_field()],
            10.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let t_count = isolate_channels(&topos, Channel::T).unwrap().len();
        let u_count = isolate_channels(&topos, Channel::U).unwrap().len();

        assert!(t_count >= 1, "e+e−→γγ: need ≥1 t-channel, got {}", t_count);
        assert!(u_count >= 1, "e+e−→γγ: need ≥1 u-channel, got {}", u_count);
        assert!(topos.count >= 2, "e+e−→γγ: need ≥2 total diagrams, got {}", topos.count);
    }

    #[test]
    fn ee_annihilation_to_photons_no_s_channel() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[photon_field(), photon_field()],
            10.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let s_count = isolate_channels(&topos, Channel::S).unwrap().len();
        assert_eq!(s_count, 0, "e+e−→γγ should have no s-channel (no γγX vertex)");
    }

    // -----------------------------------------------------------------------
    // 5.4.4: Symmetry Factor Tests
    // -----------------------------------------------------------------------

    #[test]
    fn symmetry_factor_identical_photons() {
        // e+e− → γγ: 2 identical photons → 1/2! = 0.5
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[photon_field(), photon_field()],
            10.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert!(topos.count > 0, "Should generate at least one diagram");

        for diagram in &topos.diagrams {
            assert!(
                (diagram.symmetry_factor - 0.5).abs() < 1e-12,
                "Diagram {}: symmetry factor should be 0.5, got {}",
                diagram.id, diagram.symmetry_factor
            );
        }
    }

    #[test]
    fn symmetry_factor_distinct_final_state() {
        // e+e− → e+e−: distinct particles → factor 1.0
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            assert!(
                (diagram.symmetry_factor - 1.0).abs() < 1e-12,
                "Bhabha diagram {}: factor should be 1.0, got {}",
                diagram.id, diagram.symmetry_factor
            );
        }
    }

    #[test]
    fn symmetry_factor_moller_identical_electrons() {
        // e-e- → e-e-: identical electrons → 1/2! = 0.5
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), electron_field()],
            &[electron_field(), electron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            assert!(
                (diagram.symmetry_factor - 0.5).abs() < 1e-12,
                "Møller diagram {}: factor should be 0.5, got {}",
                diagram.id, diagram.symmetry_factor
            );
        }
    }

    #[test]
    fn calculate_symmetry_factors_fn() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[photon_field(), photon_field()],
            10.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            let sf = calculate_symmetry_factors(diagram).unwrap();
            assert!((sf - 0.5).abs() < 1e-12, "calculate_symmetry_factors: expected 0.5, got {}", sf);
        }
    }

    // -----------------------------------------------------------------------
    // Loop order classification
    // -----------------------------------------------------------------------

    #[test]
    fn classify_tree_level_diagrams() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            assert_eq!(classify_loop_order(diagram), LoopOrder::Tree);
        }
    }

    // -----------------------------------------------------------------------
    // e+e− → μ+μ− (pure s-channel)
    // -----------------------------------------------------------------------

    #[test]
    fn ee_to_mumu_s_channel_only() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let s = isolate_channels(&topos, Channel::S).unwrap().len();
        let t = isolate_channels(&topos, Channel::T).unwrap().len();
        let u = isolate_channels(&topos, Channel::U).unwrap().len();

        assert_eq!(s, 2, "e+e−→μ+μ−: expected 2 s-channel (γ,Z)");
        assert_eq!(t, 0, "e+e−→μ+μ−: expected no t-channel");
        assert_eq!(u, 0, "e+e−→μ+μ−: expected no u-channel");
    }

    // -----------------------------------------------------------------------
    // Graph structure validation
    // -----------------------------------------------------------------------

    #[test]
    fn s_channel_graph_structure() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert!(topos.count > 0);

        let diagram = &topos.diagrams[0];
        assert_eq!(diagram.nodes.len(), 6, "s-channel: 4 external + 2 vertex = 6 nodes");
        assert_eq!(diagram.edges.len(), 5, "s-channel: 4 external + 1 propagator = 5 edges");

        let incoming = diagram.nodes.iter()
            .filter(|n| matches!(n.kind, NodeKind::ExternalIncoming(_))).count();
        let outgoing = diagram.nodes.iter()
            .filter(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_))).count();
        let vertices = diagram.nodes.iter()
            .filter(|n| matches!(n.kind, NodeKind::InternalVertex(_))).count();

        assert_eq!(incoming, 2);
        assert_eq!(outgoing, 2);
        assert_eq!(vertices, 2);

        let internal_edges = diagram.edges.iter().filter(|e| !e.2.is_external).count();
        assert_eq!(internal_edges, 1, "s-channel: exactly 1 propagator");
    }

    #[test]
    fn t_channel_graph_structure() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let t_diagrams = isolate_channels(&topos, Channel::T).unwrap();
        assert!(!t_diagrams.is_empty());

        let diagram = t_diagrams[0];
        assert_eq!(diagram.nodes.len(), 6);
        assert_eq!(diagram.edges.len(), 5);
    }

    // -----------------------------------------------------------------------
    // Isolation and topology set
    // -----------------------------------------------------------------------

    #[test]
    fn isolate_channels_partitions_diagrams() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[electron_field(), positron_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let s = isolate_channels(&topos, Channel::S).unwrap().len();
        let t = isolate_channels(&topos, Channel::T).unwrap().len();
        let u = isolate_channels(&topos, Channel::U).unwrap().len();

        assert_eq!(s + t + u, topos.count, "Channel isolation should partition all diagrams");
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn no_diagrams_for_invalid_vertex() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), electron_field()],
            &[muon_field(), muon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert_eq!(topos.count, 0, "e−e−→μ−μ− should produce no diagrams");
    }

    #[test]
    fn remove_vacuum_bubbles_no_change() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let mut topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        let original = topos.count;
        let removed = remove_vacuum_bubbles(&mut topos).unwrap();
        assert_eq!(removed, 0, "Tree-level: no vacuum bubbles to remove");
        assert_eq!(topos.count, original);
    }

    #[test]
    fn topology_set_reaction_label() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        assert!(topos.reaction_id.contains("→"));
        assert!(topos.reaction_id.contains("e-"));
        assert!(topos.reaction_id.contains("mu-"));
    }

    #[test]
    fn internal_propagator_correct_field() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            let internal: Vec<_> = diagram.edges.iter().filter(|e| !e.2.is_external).collect();
            assert_eq!(internal.len(), 1);
            let mediator_id = &internal[0].2.field.id;
            assert!(
                mediator_id == "photon" || mediator_id == "Z0",
                "Mediator should be photon or Z, got '{}'", mediator_id
            );
        }
    }

    #[test]
    fn propagator_attached_to_internal_edge() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );

        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            for (_, _, edge) in &diagram.edges {
                if !edge.is_external {
                    assert!(
                        edge.propagator.is_some(),
                        "Internal edges should have a propagator attached"
                    );
                }
            }
        }
    }
}
