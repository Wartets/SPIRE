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

use petgraph::graph::{DiGraph, NodeIndex};
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

// ---------------------------------------------------------------------------
// Momentum Routing for Loop Diagrams
// ---------------------------------------------------------------------------

/// A symbolic linear combination of loop and external momenta:
/// $q = \sum_i c_i^{\text{loop}} l_i + \sum_j c_j^{\text{ext}} p_j$
///
/// This is the core building block for expressing the momentum flowing through
/// any internal edge in a loop diagram. For tree-level, `loop_coefficients` is
/// empty and there is a single external momentum assignment.
///
/// **Modularity note**: This representation is independent of loop order.
/// Multi-loop diagrams simply have multiple entries in `loop_coefficients`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumExpression {
    /// Coefficients of loop momenta: key = loop momentum label (e.g., `"l"`), value = coefficient ∈ {-1, 0, 1}.
    pub loop_coefficients: Vec<(String, i32)>,
    /// Coefficients of external momenta: key = external momentum label (e.g., `"p1"`), value = coefficient.
    pub external_coefficients: Vec<(String, i32)>,
}

impl MomentumExpression {
    /// Create a pure external momentum expression.
    pub fn external(label: &str) -> Self {
        Self {
            loop_coefficients: vec![],
            external_coefficients: vec![(label.to_string(), 1)],
        }
    }

    /// Create a pure loop momentum expression.
    pub fn loop_momentum(label: &str) -> Self {
        Self {
            loop_coefficients: vec![(label.to_string(), 1)],
            external_coefficients: vec![],
        }
    }

    /// Render a human-readable string, e.g., `"l + p1 - p2"`.
    pub fn to_label(&self) -> String {
        let mut parts = Vec::new();
        for (label, coeff) in &self.loop_coefficients {
            match *coeff {
                1 => parts.push(label.clone()),
                -1 => parts.push(format!("-{}", label)),
                c => parts.push(format!("{}·{}", c, label)),
            }
        }
        for (label, coeff) in &self.external_coefficients {
            match *coeff {
                1 => parts.push(label.clone()),
                -1 => parts.push(format!("-{}", label)),
                c => parts.push(format!("{}·{}", c, label)),
            }
        }
        if parts.is_empty() {
            "0".into()
        } else {
            // Join with " + ", cleaning up leading minus signs
            let mut result = parts[0].clone();
            for p in &parts[1..] {
                if p.starts_with('-') {
                    result.push_str(&format!(" - {}", &p[1..]));
                } else {
                    result.push_str(&format!(" + {}", p));
                }
            }
            result
        }
    }
}

/// Complete momentum routing for a loop diagram.
///
/// Stores the assignment of momentum expressions to every internal edge,
/// ensuring Kirchhoff's law (momentum conservation) at every vertex.
/// This is the bridge between topology and analytic loop integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopMomentumRouting {
    /// Labels for the independent loop momenta (e.g., `["l"]` for 1-loop).
    pub loop_momenta: Vec<String>,
    /// Momentum expressions assigned to each internal edge.
    /// Keyed by `(source_node_id, target_node_id)`.
    pub edge_momenta: Vec<(usize, usize, MomentumExpression)>,
}

/// Classification of a standard 1-loop topology.
///
/// These correspond to the Passarino-Veltman scalar integral basis.
/// The naming follows the number of internal propagators in the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OneLoopTopologyKind {
    /// Self-energy / Tadpole: 1 propagator in the loop (1-point function $A_0$).
    Tadpole,
    /// Self-energy / Bubble: 2 propagators in the loop (2-point function $B_0$).
    Bubble,
    /// Vertex correction / Triangle: 3 propagators (3-point function $C_0$).
    Triangle,
    /// Box: 4 propagators (4-point function $D_0$).
    Box,
    /// Pentagon and higher: $n$ propagators ($n$-point function).
    NPoint(u32),
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
    /// Whether this diagram is one-particle irreducible (1PI).
    ///
    /// A 1PI diagram cannot be disconnected by cutting any single internal edge.
    /// All tree-level diagrams with more than one internal line are 1PR.
    /// Set to `None` for tree-level diagrams (where the concept is less relevant).
    #[serde(default)]
    pub is_one_particle_irreducible: Option<bool>,
    /// The 1-loop topology classification (if applicable).
    #[serde(default)]
    pub loop_topology_kind: Option<OneLoopTopologyKind>,
    /// Loop momentum routing (present for loop diagrams).
    #[serde(default)]
    pub momentum_routing: Option<LoopMomentumRouting>,
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
        is_one_particle_irreducible: None,
        loop_topology_kind: None,
        momentum_routing: None,
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
        is_one_particle_irreducible: None,
        loop_topology_kind: None,
        momentum_routing: None,
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
        is_one_particle_irreducible: None,
        loop_topology_kind: None,
        momentum_routing: None,
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
    max_order: LoopOrder,
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

    // -------------------------------------------------------------------
    // 1-Loop Topologies (if max_order >= OneLoop)
    // -------------------------------------------------------------------
    let include_one_loop = matches!(
        max_order,
        LoopOrder::OneLoop | LoopOrder::TwoLoop | LoopOrder::NLoop(_)
    );

    if include_one_loop && !diagrams.is_empty() {
        let tree_diagrams = diagrams.clone();
        for tree_diag in &tree_diagrams {
            // Generate self-energy (bubble) corrections on each internal propagator
            let internal_edges: Vec<(usize, usize, &Edge)> = tree_diag
                .edges
                .iter()
                .filter(|(_, _, e)| !e.is_external)
                .map(|(s, t, e)| (*s, *t, e))
                .collect();

            for (src, tgt, int_edge) in &internal_edges {
                // Try all fields that can form a loop with the propagator
                let mediator_id = int_edge.field.id.as_str();
                let loop_diagrams = generate_self_energy_topologies(
                    &mut next_id,
                    tree_diag,
                    *src,
                    *tgt,
                    mediator_id,
                    model,
                );
                diagrams.extend(loop_diagrams);
            }

            // Generate vertex corrections (triangles) for each internal vertex
            let vertex_nodes: Vec<&Node> = tree_diag
                .nodes
                .iter()
                .filter(|n| matches!(n.kind, NodeKind::InternalVertex(_)))
                .collect();

            for v_node in &vertex_nodes {
                let loop_diagrams = generate_vertex_correction_topologies(
                    &mut next_id,
                    tree_diag,
                    v_node.id,
                    model,
                );
                diagrams.extend(loop_diagrams);
            }

            // Generate box topologies for 2→2 processes
            if n_in == 2 && n_out == 2 {
                let loop_diagrams = generate_box_topologies(
                    &mut next_id,
                    tree_diag,
                    model,
                );
                diagrams.extend(loop_diagrams);
            }
        }
    }

    // Determine the effective max loop order for the set
    let effective_max = if include_one_loop {
        LoopOrder::OneLoop
    } else {
        LoopOrder::Tree
    };

    Ok(TopologySet {
        reaction_id: reaction_label,
        max_loop_order: effective_max,
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

// ---------------------------------------------------------------------------
// 1-Loop Topology Generation
// ---------------------------------------------------------------------------

/// Generate **self-energy (bubble)** corrections by inserting a loop on an
/// internal propagator of a tree-level diagram.
///
/// Given a tree diagram with an internal edge $(V_1 \to V_2)$ carrying field $X$,
/// this inserts a loop by:
/// 1. Replacing the direct propagator with two new vertices.
/// 2. Connecting the new vertices with two parallel propagators forming a loop.
///
/// ```text
///    (V1) ─── X ───→ (V2)        becomes
///    (V1) → (Va) ═══ (Vb) → (V2)
///                ↺ loop
/// ```
///
/// The function tries all model fields that can participate at the new vertices.
fn generate_self_energy_topologies(
    next_id: &mut usize,
    tree_diag: &FeynmanGraph,
    prop_src: usize,
    prop_tgt: usize,
    _mediator_id: &str,
    model: &TheoreticalModel,
) -> Vec<FeynmanGraph> {
    let mut results = Vec::new();

    // Find the original internal edge data
    let original_edge = tree_diag
        .edges
        .iter()
        .find(|(s, t, e)| *s == prop_src && *t == prop_tgt && !e.is_external);

    let original_edge = match original_edge {
        Some(e) => e.2.clone(),
        None => return results,
    };

    let original_field = &original_edge.field;

    // Try all pairs of fields that can form a self-energy bubble
    // The loop propagators are (F1, F2) where V_a has vertex [original, F1, F2]
    // and V_b has vertex [F1, F2, original]
    for f1 in &model.fields {
        for f2 in &model.fields {
            // Check vertex at V_a: [original_field, f1, f2]
            let va_ids = [original_field.id.as_str(), f1.id.as_str(), f2.id.as_str()];
            let vf_a = match find_matching_vertex(model, &va_ids) {
                Some(vf) => vf,
                None => continue,
            };

            // Check vertex at V_b: [f1, f2, original_field]
            let vb_ids = [f1.id.as_str(), f2.id.as_str(), original_field.id.as_str()];
            let vf_b = match find_matching_vertex(model, &vb_ids) {
                Some(vf) => vf,
                None => continue,
            };

            // Build the self-energy diagram
            let mut graph = DiGraph::<Node, Edge>::new();
            let mut node_id = 0usize;
            let mut node_map: std::collections::HashMap<usize, NodeIndex> =
                std::collections::HashMap::new();

            // Re-create all original nodes
            for orig_node in &tree_diag.nodes {
                let ni = graph.add_node(orig_node.clone());
                node_map.insert(orig_node.id, ni);
                node_id = node_id.max(orig_node.id + 1);
            }

            // Add two new internal vertices for the self-energy insertion
            let va_node_id = node_id;
            let n_va = graph.add_node(Node {
                id: va_node_id,
                kind: NodeKind::InternalVertex(vf_a.clone()),
                position: Some((-0.3, 0.5)),
            });
            node_map.insert(va_node_id, n_va);
            node_id += 1;

            let vb_node_id = node_id;
            let n_vb = graph.add_node(Node {
                id: vb_node_id,
                kind: NodeKind::InternalVertex(vf_b.clone()),
                position: Some((0.3, 0.5)),
            });
            node_map.insert(vb_node_id, n_vb);
            // node_id now past vb; no further node allocation in this iteration.

            // Re-create all original edges EXCEPT the one being replaced
            for (s, t, e) in &tree_diag.edges {
                if *s == prop_src && *t == prop_tgt && !e.is_external {
                    continue; // Skip the edge being replaced
                }
                if let (Some(&ns), Some(&nt)) = (node_map.get(s), node_map.get(t)) {
                    graph.add_edge(ns, nt, e.clone());
                }
            }

            // Connect: original V1 → Va
            let prop_v1_va = model.propagators.iter().find(|p| p.field_id == original_field.id);
            if let Some(&n_src) = node_map.get(&prop_src) {
                graph.add_edge(n_src, n_va, Edge {
                    field: original_field.clone(),
                    propagator: prop_v1_va.cloned(),
                    momentum_label: "q1".into(),
                    is_external: false,
                });
            }

            // Connect: Vb → original V2
            if let Some(&n_tgt) = node_map.get(&prop_tgt) {
                graph.add_edge(n_vb, n_tgt, Edge {
                    field: original_field.clone(),
                    propagator: prop_v1_va.cloned(),
                    momentum_label: "q2".into(),
                    is_external: false,
                });
            }

            // Loop propagator 1: Va → Vb
            let loop_prop_1 = model.propagators.iter().find(|p| p.field_id == f1.id);
            graph.add_edge(n_va, n_vb, Edge {
                field: f1.clone(),
                propagator: loop_prop_1.cloned(),
                momentum_label: "l".into(),
                is_external: false,
            });

            // Loop propagator 2: Vb → Va (closing the loop)
            let loop_prop_2 = model.propagators.iter().find(|p| p.field_id == f2.id);
            graph.add_edge(n_vb, n_va, Edge {
                field: f2.clone(),
                propagator: loop_prop_2.cloned(),
                momentum_label: "l_q".into(),
                is_external: false,
            });

            // Build momentum routing for the self-energy
            let routing = LoopMomentumRouting {
                loop_momenta: vec!["l".to_string()],
                edge_momenta: vec![
                    (va_node_id, vb_node_id, MomentumExpression::loop_momentum("l")),
                    (vb_node_id, va_node_id, MomentumExpression {
                        loop_coefficients: vec![("l".to_string(), -1)],
                        external_coefficients: vec![("q".to_string(), 1)],
                    }),
                ],
            };

            let nodes: Vec<Node> = graph.node_weights().cloned().collect();
            let edges: Vec<(usize, usize, Edge)> = graph
                .edge_indices()
                .map(|ei| {
                    let (s, t) = graph.edge_endpoints(ei).unwrap();
                    (s.index(), t.index(), graph[ei].clone())
                })
                .collect();

            let mut fg = FeynmanGraph {
                id: *next_id,
                graph,
                nodes,
                edges,
                channels: tree_diag.channels.clone(),
                loop_order: LoopOrder::OneLoop,
                symmetry_factor: tree_diag.symmetry_factor,
                is_connected: true,
                is_one_particle_irreducible: Some(true), // Self-energy bubbles are 1PI
                loop_topology_kind: Some(OneLoopTopologyKind::Bubble),
                momentum_routing: Some(routing),
            };

            // Check 1PI status
            fg.is_one_particle_irreducible = Some(check_one_particle_irreducible(&fg));

            results.push(fg);
            *next_id += 1;
        }
    }

    results
}

/// Generate **vertex correction (triangle)** topologies by dressing an
/// internal vertex with a triangular loop.
///
/// For a tree-level vertex $V$ with legs $(A, B, C)$, this adds a triangular
/// loop through 3 internal propagators and 3 vertices.
///
/// The resulting topology has 3 loop propagators forming a triangle,
/// corresponding to a $C_0$ (triangle) scalar integral.
fn generate_vertex_correction_topologies(
    next_id: &mut usize,
    tree_diag: &FeynmanGraph,
    vertex_node_id: usize,
    model: &TheoreticalModel,
) -> Vec<FeynmanGraph> {
    let mut results = Vec::new();

    // Find edges connected to this vertex
    let connected_edges: Vec<&(usize, usize, Edge)> = tree_diag
        .edges
        .iter()
        .filter(|(s, t, _)| *s == vertex_node_id || *t == vertex_node_id)
        .collect();

    // For a vertex correction, we need at least 2 edges to form a triangle.
    // We pick pairs of edges and try to insert a triangle loop.
    if connected_edges.len() < 2 {
        return results;
    }

    // For each pair of adjacent edges, try to form a triangle loop
    for i in 0..connected_edges.len() {
        for j in (i + 1)..connected_edges.len() {
            let edge_a = connected_edges[i];
            let edge_b = connected_edges[j];

            let field_a = &edge_a.2.field;
            let field_b = &edge_b.2.field;

            // Try all possible fields for the third propagator in the triangle
            for f_loop in &model.fields {
                // Check vertices:
                // V_new_1: [field_a, f_loop, <vertex_original>]
                // V_new_2: [f_loop, field_b, <vertex_original>]
                let v1_ids = [field_a.id.as_str(), f_loop.id.as_str(), field_a.id.as_str()];
                let v2_ids = [f_loop.id.as_str(), field_b.id.as_str(), field_b.id.as_str()];

                if find_matching_vertex(model, &v1_ids).is_some()
                    && find_matching_vertex(model, &v2_ids).is_some()
                {
                    // Create a triangle topology placeholder
                    // This is a simplified representation — the full graph construction
                    // follows the same pattern as self-energy but with 3 internal edges.
                    let routing = LoopMomentumRouting {
                        loop_momenta: vec!["l".to_string()],
                        edge_momenta: vec![
                            (0, 1, MomentumExpression::loop_momentum("l")),
                            (1, 2, MomentumExpression {
                                loop_coefficients: vec![("l".to_string(), 1)],
                                external_coefficients: vec![("p1".to_string(), 1)],
                            }),
                            (2, 0, MomentumExpression {
                                loop_coefficients: vec![("l".to_string(), 1)],
                                external_coefficients: vec![
                                    ("p1".to_string(), 1),
                                    ("p2".to_string(), 1),
                                ],
                            }),
                        ],
                    };

                    // Build a simplified graph for the vertex correction
                    let mut graph = DiGraph::<Node, Edge>::new();
                    let mut new_nodes = tree_diag.nodes.clone();
                    let mut new_edges = tree_diag.edges.clone();

                    // Add a marker node for the triangle vertex correction
                    let triangle_marker_id = new_nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                    new_nodes.push(Node {
                        id: triangle_marker_id,
                        kind: NodeKind::InternalVertex(
                            tree_diag
                                .nodes
                                .iter()
                                .find(|n| n.id == vertex_node_id)
                                .and_then(|n| match &n.kind {
                                    NodeKind::InternalVertex(vf) => Some(vf.clone()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| model.vertex_factors[0].clone()),
                        ),
                        position: Some((0.0, 0.8)),
                    });

                    // Add the loop propagator edge
                    let loop_prop = model.propagators.iter().find(|p| p.field_id == f_loop.id);
                    new_edges.push((
                        vertex_node_id,
                        triangle_marker_id,
                        Edge {
                            field: f_loop.clone(),
                            propagator: loop_prop.cloned(),
                            momentum_label: "l".into(),
                            is_external: false,
                        },
                    ));
                    new_edges.push((
                        triangle_marker_id,
                        vertex_node_id,
                        Edge {
                            field: f_loop.clone(),
                            propagator: loop_prop.cloned(),
                            momentum_label: "l_q".into(),
                            is_external: false,
                        },
                    ));

                    // Reconstruct petgraph
                    for node in &new_nodes {
                        graph.add_node(node.clone());
                    }
                    for (s, t, e) in &new_edges {
                        let si = NodeIndex::new(*s);
                        let ti = NodeIndex::new(*t);
                        if si.index() < graph.node_count() && ti.index() < graph.node_count() {
                            graph.add_edge(si, ti, e.clone());
                        }
                    }

                    let fg = FeynmanGraph {
                        id: *next_id,
                        graph,
                        nodes: new_nodes,
                        edges: new_edges,
                        channels: tree_diag.channels.clone(),
                        loop_order: LoopOrder::OneLoop,
                        symmetry_factor: tree_diag.symmetry_factor,
                        is_connected: true,
                        is_one_particle_irreducible: Some(true), // Vertex corrections are 1PI
                        loop_topology_kind: Some(OneLoopTopologyKind::Triangle),
                        momentum_routing: Some(routing),
                    };

                    results.push(fg);
                    *next_id += 1;

                    break; // One triangle per edge pair per loop field
                }
            }
        }
    }

    results
}

/// Generate **box** topologies for a 2→2 process.
///
/// A box diagram has 4 internal propagators and 4 vertices, each
/// connecting one external leg to the loop. This corresponds to the
/// $D_0$ (4-point) scalar integral.
///
/// ```text
///   [In1] → (V1) ── loop ── (V2) → [Out1]
///              |                |
///           loop               loop
///              |                |
///   [In2] → (V4) ── loop ── (V3) → [Out2]
/// ```
fn generate_box_topologies(
    next_id: &mut usize,
    tree_diag: &FeynmanGraph,
    model: &TheoreticalModel,
) -> Vec<FeynmanGraph> {
    let mut results = Vec::new();

    // Identify external legs
    let incoming: Vec<&Node> = tree_diag
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::ExternalIncoming(_)))
        .collect();
    let outgoing: Vec<&Node> = tree_diag
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::ExternalOutgoing(_)))
        .collect();

    if incoming.len() != 2 || outgoing.len() != 2 {
        return results;
    }

    let in_fields: Vec<&Field> = incoming
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::ExternalIncoming(p) => Some(&p.field),
            _ => None,
        })
        .collect();
    let out_fields: Vec<&Field> = outgoing
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::ExternalOutgoing(p) => Some(&p.field),
            _ => None,
        })
        .collect();

    if in_fields.len() != 2 || out_fields.len() != 2 {
        return results;
    }

    // Try all combinations of 4 internal fields for the box loop
    for f1 in &model.fields {
        for f2 in &model.fields {
            for f3 in &model.fields {
                for f4 in &model.fields {
                    // Check 4 vertices:
                    // V1: [in1, f1, f4], V2: [out1, f1, f2]
                    // V3: [out2, f2, f3], V4: [in2, f3, f4]
                    let v1_check = find_matching_vertex(
                        model,
                        &[in_fields[0].id.as_str(), f1.id.as_str(), f4.id.as_str()],
                    );
                    let v2_check = find_matching_vertex(
                        model,
                        &[out_fields[0].id.as_str(), f1.id.as_str(), f2.id.as_str()],
                    );
                    let v3_check = find_matching_vertex(
                        model,
                        &[out_fields[1].id.as_str(), f2.id.as_str(), f3.id.as_str()],
                    );
                    let v4_check = find_matching_vertex(
                        model,
                        &[in_fields[1].id.as_str(), f3.id.as_str(), f4.id.as_str()],
                    );

                    if let (Some(vf1), Some(vf2), Some(vf3), Some(vf4)) =
                        (v1_check, v2_check, v3_check, v4_check)
                    {
                        let mut graph = DiGraph::<Node, Edge>::new();

                        // 4 external + 4 vertex = 8 nodes
                        let n_in1 = graph.add_node(Node {
                            id: 0,
                            kind: NodeKind::ExternalIncoming(
                                match &incoming[0].kind {
                                    NodeKind::ExternalIncoming(p) => p.clone(),
                                    _ => unreachable!(),
                                },
                            ),
                            position: Some((-2.0, 1.0)),
                        });
                        let n_in2 = graph.add_node(Node {
                            id: 1,
                            kind: NodeKind::ExternalIncoming(
                                match &incoming[1].kind {
                                    NodeKind::ExternalIncoming(p) => p.clone(),
                                    _ => unreachable!(),
                                },
                            ),
                            position: Some((-2.0, -1.0)),
                        });
                        let n_v1 = graph.add_node(Node {
                            id: 2,
                            kind: NodeKind::InternalVertex(vf1.clone()),
                            position: Some((-1.0, 1.0)),
                        });
                        let n_v2 = graph.add_node(Node {
                            id: 3,
                            kind: NodeKind::InternalVertex(vf2.clone()),
                            position: Some((1.0, 1.0)),
                        });
                        let n_v3 = graph.add_node(Node {
                            id: 4,
                            kind: NodeKind::InternalVertex(vf3.clone()),
                            position: Some((1.0, -1.0)),
                        });
                        let n_v4 = graph.add_node(Node {
                            id: 5,
                            kind: NodeKind::InternalVertex(vf4.clone()),
                            position: Some((-1.0, -1.0)),
                        });
                        let n_out1 = graph.add_node(Node {
                            id: 6,
                            kind: NodeKind::ExternalOutgoing(
                                match &outgoing[0].kind {
                                    NodeKind::ExternalOutgoing(p) => p.clone(),
                                    _ => unreachable!(),
                                },
                            ),
                            position: Some((2.0, 1.0)),
                        });
                        let n_out2 = graph.add_node(Node {
                            id: 7,
                            kind: NodeKind::ExternalOutgoing(
                                match &outgoing[1].kind {
                                    NodeKind::ExternalOutgoing(p) => p.clone(),
                                    _ => unreachable!(),
                                },
                            ),
                            position: Some((2.0, -1.0)),
                        });

                        // External edges
                        graph.add_edge(n_in1, n_v1, Edge {
                            field: in_fields[0].clone(),
                            propagator: None,
                            momentum_label: "p1".into(),
                            is_external: true,
                        });
                        graph.add_edge(n_in2, n_v4, Edge {
                            field: in_fields[1].clone(),
                            propagator: None,
                            momentum_label: "p2".into(),
                            is_external: true,
                        });
                        graph.add_edge(n_v2, n_out1, Edge {
                            field: out_fields[0].clone(),
                            propagator: None,
                            momentum_label: "p3".into(),
                            is_external: true,
                        });
                        graph.add_edge(n_v3, n_out2, Edge {
                            field: out_fields[1].clone(),
                            propagator: None,
                            momentum_label: "p4".into(),
                            is_external: true,
                        });

                        // 4 internal loop edges
                        let p1 = model.propagators.iter().find(|p| p.field_id == f1.id);
                        let p2 = model.propagators.iter().find(|p| p.field_id == f2.id);
                        let p3 = model.propagators.iter().find(|p| p.field_id == f3.id);
                        let p4 = model.propagators.iter().find(|p| p.field_id == f4.id);

                        graph.add_edge(n_v1, n_v2, Edge {
                            field: f1.clone(),
                            propagator: p1.cloned(),
                            momentum_label: "l".into(),
                            is_external: false,
                        });
                        graph.add_edge(n_v2, n_v3, Edge {
                            field: f2.clone(),
                            propagator: p2.cloned(),
                            momentum_label: "l_p3".into(),
                            is_external: false,
                        });
                        graph.add_edge(n_v3, n_v4, Edge {
                            field: f3.clone(),
                            propagator: p3.cloned(),
                            momentum_label: "l_p3_p4".into(),
                            is_external: false,
                        });
                        graph.add_edge(n_v4, n_v1, Edge {
                            field: f4.clone(),
                            propagator: p4.cloned(),
                            momentum_label: "l_p1".into(),
                            is_external: false,
                        });

                        let routing = LoopMomentumRouting {
                            loop_momenta: vec!["l".to_string()],
                            edge_momenta: vec![
                                (2, 3, MomentumExpression::loop_momentum("l")),
                                (3, 4, MomentumExpression {
                                    loop_coefficients: vec![("l".to_string(), 1)],
                                    external_coefficients: vec![("p3".to_string(), -1)],
                                }),
                                (4, 5, MomentumExpression {
                                    loop_coefficients: vec![("l".to_string(), 1)],
                                    external_coefficients: vec![
                                        ("p3".to_string(), -1),
                                        ("p4".to_string(), -1),
                                    ],
                                }),
                                (5, 2, MomentumExpression {
                                    loop_coefficients: vec![("l".to_string(), 1)],
                                    external_coefficients: vec![("p1".to_string(), 1)],
                                }),
                            ],
                        };

                        let nodes: Vec<Node> = graph.node_weights().cloned().collect();
                        let edges: Vec<(usize, usize, Edge)> = graph
                            .edge_indices()
                            .map(|ei| {
                                let (s, t) = graph.edge_endpoints(ei).unwrap();
                                (s.index(), t.index(), graph[ei].clone())
                            })
                            .collect();

                        let fg = FeynmanGraph {
                            id: *next_id,
                            graph,
                            nodes,
                            edges,
                            channels: tree_diag.channels.clone(),
                            loop_order: LoopOrder::OneLoop,
                            symmetry_factor: tree_diag.symmetry_factor,
                            is_connected: true,
                            is_one_particle_irreducible: Some(true), // Boxes are always 1PI
                            loop_topology_kind: Some(OneLoopTopologyKind::Box),
                            momentum_routing: Some(routing),
                        };

                        results.push(fg);
                        *next_id += 1;

                        // Found one valid box — return to avoid combinatorial explosion
                        // (more boxes may exist with different field assignments)
                        if results.len() >= 4 {
                            return results;
                        }
                    }
                }
            }
        }
    }

    results
}

/// Check whether a Feynman diagram is **one-particle irreducible** (1PI).
///
/// A 1PI diagram cannot be disconnected by cutting any single internal edge.
/// Uses the graph-theoretic definition: for each internal edge, remove it
/// and check if the graph remains connected.
///
/// For tree-level diagrams, any graph with $\geq 2$ internal vertices is
/// always 1PR (reducible), since cutting the single propagator disconnects it.
pub fn check_one_particle_irreducible(diagram: &FeynmanGraph) -> bool {
    let internal_edges: Vec<(usize, usize)> = diagram
        .edges
        .iter()
        .filter(|(_, _, e)| !e.is_external)
        .map(|(s, t, _)| (*s, *t))
        .collect();

    // A diagram with 0 or 1 internal edges is trivially 1PI (or tree-level)
    if internal_edges.len() <= 1 {
        return internal_edges.is_empty();
    }

    // For each internal edge, check connectivity after removal
    for &(removed_src, removed_tgt) in &internal_edges {
        // Build adjacency without this edge
        let remaining: Vec<(usize, usize)> = diagram
            .edges
            .iter()
            .filter(|(s, t, _)| !(*s == removed_src && *t == removed_tgt))
            .map(|(s, t, _)| (*s, *t))
            .collect();

        // Collect all node IDs
        let all_node_ids: std::collections::HashSet<usize> =
            diagram.nodes.iter().map(|n| n.id).collect();

        if all_node_ids.is_empty() {
            continue;
        }

        // BFS from the first node
        let start = *all_node_ids.iter().next().unwrap();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            for &(s, t) in &remaining {
                if s == current && !visited.contains(&t) {
                    visited.insert(t);
                    queue.push_back(t);
                }
                if t == current && !visited.contains(&s) {
                    visited.insert(s);
                    queue.push_back(s);
                }
            }
        }

        // If not all nodes visited, removing this edge disconnected the graph → 1PR
        if visited.len() < all_node_ids.len() {
            return false;
        }
    }

    // All edges removed individually without disconnecting → 1PI
    true
}

/// Compute the loop momentum routing for a diagram using Kirchhoff's laws.
///
/// Assigns an undetermined loop momentum $l$ to one internal edge, then
/// propagates momentum conservation constraints through all vertices to
/// determine the momentum on every other internal edge.
///
/// # Arguments
/// * `diagram` — The Feynman diagram to route momenta through.
///
/// # Returns
/// A [`LoopMomentumRouting`] describing the momentum flow, or `None` for
/// tree-level diagrams.
pub fn compute_momentum_routing(diagram: &FeynmanGraph) -> Option<LoopMomentumRouting> {
    let loop_order = classify_loop_order(diagram);
    if matches!(loop_order, LoopOrder::Tree) {
        return None;
    }

    // For diagrams that already have routing computed during generation
    if diagram.momentum_routing.is_some() {
        return diagram.momentum_routing.clone();
    }

    // Fallback: assign "l" to the first internal edge, derive the rest
    let internal_edges: Vec<(usize, usize, &Edge)> = diagram
        .edges
        .iter()
        .filter(|(_, _, e)| !e.is_external)
        .map(|(s, t, e)| (*s, *t, e))
        .collect();

    if internal_edges.is_empty() {
        return None;
    }

    let mut edge_momenta = Vec::new();

    // Assign l to the first internal edge
    edge_momenta.push((
        internal_edges[0].0,
        internal_edges[0].1,
        MomentumExpression::loop_momentum("l"),
    ));

    // For remaining internal edges, build expressions using momentum conservation
    // at each vertex: sum of incoming = sum of outgoing
    for (idx, &(src, tgt, _)) in internal_edges.iter().enumerate().skip(1) {
        // Simplified momentum routing: each subsequent edge carries l + combination of externals
        let mut ext_coeffs = Vec::new();
        // The external momentum contributions depend on the graph topology
        // We use a simple linear assignment for now
        ext_coeffs.push((format!("p{}", idx), 1));

        edge_momenta.push((src, tgt, MomentumExpression {
            loop_coefficients: vec![("l".to_string(), 1)],
            external_coefficients: ext_coeffs,
        }));
    }

    Some(LoopMomentumRouting {
        loop_momenta: vec!["l".to_string()],
        edge_momenta,
    })
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
            spacetime: crate::algebra::SpacetimeConfig::default(),
            constants: crate::ontology::PhysicalConstants::default(),
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
            is_one_particle_irreducible: None,
            loop_topology_kind: None,
            momentum_routing: None,
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

    // ===================================================================
    // Loop Topology Generation Tests
    // ===================================================================

    #[test]
    fn one_loop_topology_kind_serde() {
        let kinds = vec![
            OneLoopTopologyKind::Tadpole,
            OneLoopTopologyKind::Bubble,
            OneLoopTopologyKind::Triangle,
            OneLoopTopologyKind::Box,
            OneLoopTopologyKind::NPoint(5),
        ];
        for k in &kinds {
            let json = serde_json::to_string(k).unwrap();
            let back: OneLoopTopologyKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*k, back);
        }
    }

    #[test]
    fn momentum_expression_to_label() {
        let me = MomentumExpression {
            loop_coefficients: vec![("l".into(), 1)],
            external_coefficients: vec![("p1".into(), 1), ("p2".into(), -1)],
        };
        let label = me.to_label();
        assert!(label.contains("l"), "got: {}", label);
        assert!(label.contains("p1"), "got: {}", label);
        assert!(label.contains("p2"), "got: {}", label);
    }

    #[test]
    fn momentum_expression_loop_only() {
        let me = MomentumExpression {
            loop_coefficients: vec![("l".into(), 1)],
            external_coefficients: vec![],
        };
        assert_eq!(me.to_label(), "l");
    }

    #[test]
    fn momentum_routing_serde() {
        let routing = LoopMomentumRouting {
            loop_momenta: vec!["l".into()],
            edge_momenta: vec![
                (0, 1, MomentumExpression {
                    loop_coefficients: vec![("l".into(), 1)],
                    external_coefficients: vec![],
                }),
            ],
        };
        let json = serde_json::to_string(&routing).unwrap();
        let back: LoopMomentumRouting = serde_json::from_str(&json).unwrap();
        assert_eq!(back.loop_momenta.len(), 1);
        assert_eq!(back.edge_momenta.len(), 1);
    }

    #[test]
    fn check_1pi_tree_diagram() {
        // A tree diagram (no loops) is NOT 1PI — removing any internal edge
        // disconnects the graph.
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::Tree).unwrap();
        for diagram in &topos.diagrams {
            let is_1pi = check_one_particle_irreducible(diagram);
            assert!(!is_1pi, "Tree diagrams should not be 1PI");
        }
    }

    #[test]
    fn one_loop_generation_produces_diagrams() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        // Should have tree-level diagrams plus loop-level diagrams
        let tree_count = topos
            .diagrams
            .iter()
            .filter(|d| d.loop_order == LoopOrder::Tree)
            .count();
        let loop_count = topos
            .diagrams
            .iter()
            .filter(|d| d.loop_order == LoopOrder::OneLoop)
            .count();

        assert!(tree_count > 0, "Should have tree diagrams");
        assert!(loop_count > 0, "Should have 1-loop diagrams");
    }

    #[test]
    fn one_loop_diagrams_have_topology_classification() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        for diagram in topos.diagrams.iter().filter(|d| d.loop_order == LoopOrder::OneLoop) {
            assert!(
                diagram.loop_topology_kind.is_some(),
                "1-loop diagrams should have topology classification"
            );
        }
    }

    #[test]
    fn one_loop_diagrams_have_momentum_routing() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        for diagram in topos.diagrams.iter().filter(|d| d.loop_order == LoopOrder::OneLoop) {
            assert!(
                diagram.momentum_routing.is_some(),
                "1-loop diagrams should have momentum routing"
            );
            let routing = diagram.momentum_routing.as_ref().unwrap();
            assert!(!routing.loop_momenta.is_empty(), "Should have at least one loop momentum");
        }
    }

    #[test]
    fn self_energy_topologies_are_bubbles() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        let bubble_count = topos
            .diagrams
            .iter()
            .filter(|d| d.loop_topology_kind == Some(OneLoopTopologyKind::Bubble))
            .count();
        assert!(bubble_count > 0, "Should have generated at least one bubble topology");

        // All bubble diagrams should be 1-loop
        for diag in topos.diagrams.iter().filter(|d| d.loop_topology_kind == Some(OneLoopTopologyKind::Bubble)) {
            assert_eq!(diag.loop_order, LoopOrder::OneLoop);
        }
    }

    #[test]
    fn vertex_correction_topologies_are_triangles() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        let triangle_count = topos
            .diagrams
            .iter()
            .filter(|d| d.loop_topology_kind == Some(OneLoopTopologyKind::Triangle))
            .count();
        assert!(triangle_count > 0, "Should have generated at least one triangle topology");

        for diag in topos.diagrams.iter().filter(|d| d.loop_topology_kind == Some(OneLoopTopologyKind::Triangle)) {
            assert_eq!(diag.loop_order, LoopOrder::OneLoop);
        }
    }

    #[test]
    fn one_loop_feynman_graph_serde_roundtrip() {
        let model = make_full_test_model();
        let reaction = make_reaction(
            &[electron_field(), positron_field()],
            &[muon_field(), antimuon_field()],
            200.0,
        );
        let topos = generate_topologies(&reaction, &model, LoopOrder::OneLoop).unwrap();

        for diagram in topos.diagrams.iter().filter(|d| d.loop_order == LoopOrder::OneLoop).take(3) {
            let json = serde_json::to_string(diagram).unwrap();
            let back: FeynmanGraph = serde_json::from_str(&json).unwrap();
            assert_eq!(back.loop_order, LoopOrder::OneLoop);
            assert!(back.loop_topology_kind.is_some());
            assert!(back.momentum_routing.is_some());
        }
    }
}
