/**
 * Advanced Layout Engine for Feynman Diagrams
 *
 * Features:
 * - Sugiyama-style hierarchical layout with left-to-right orientation
 * - Planar edge routing to minimize crossings
 * - Multi-edge support (parallel edges between same node pairs)
 * - Proper incoming/outgoing particle positioning
 * - Grid-based node positioning to avoid overlaps
 */

import type { FeynmanDiagram, FeynmanNode, FeynmanEdge, NodeKind } from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Core Data Structures
// ---------------------------------------------------------------------------

export interface LayoutNode {
  id: number;
  x: number;
  y: number;
  kind: NodeKind;
  label: string;
  layer: number; // Hierarchical layer (0 = incoming, max = outgoing)
  orderInLayer: number; // Position within layer for better ordering
}

export interface LayoutEdge {
  srcId: number;
  tgtId: number;
  edgeIndex: number; // Index in original edges array
  laneOffset: number; // For parallel edges
  edge: FeynmanEdge;
}

export interface DiagramLayout {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  width: number;
  height: number;
}

/** Validation report for diagram structure */
export interface DiagramValidation {
  isValid: boolean;
  warnings: string[];
  errors: string[];
}

// ---------------------------------------------------------------------------
// Layer Assignment (Sugiyama Phase 1)
// ---------------------------------------------------------------------------

function assignLayers(
  nodes: FeynmanNode[],
  edges: Array<[number, number, FeynmanEdge]>,
): Map<number, number> {
  const layers = new Map<number, number>();
  const outgoing = new Map<number, number[]>();

  // Initialize
  for (const node of nodes) {
    outgoing.set(node.id, []);
  }

  for (const [src, tgt] of edges) {
    outgoing.get(src)?.push(tgt);
  }

  // Layer assignment: stable breadth-first propagation from incoming sources.
  // Important: we only relax to *smaller* layer values, which guarantees
  // convergence even for cyclic graphs (e.g. one-loop topologies).
  const queue: number[] = [];
  const queued = new Set<number>();
  for (const node of nodes) {
    if ("ExternalIncoming" in node.kind) {
      layers.set(node.id, 0);
      queue.push(node.id);
      queued.add(node.id);
    }
  }

  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    queued.delete(nodeId);
    const currentLayer = layers.get(nodeId) ?? 0;

    for (const targetId of outgoing.get(nodeId) ?? []) {
      const candidate = currentLayer + 1;
      const previous = layers.get(targetId);
      if (previous === undefined || candidate < previous) {
        layers.set(targetId, candidate);
        if (!queued.has(targetId)) {
          queue.push(targetId);
          queued.add(targetId);
        }
      }
    }
  }

  // Assign remaining nodes (unconnected or only reachable through cycle direction)
  // to reasonable fallback layers.
  let maxAssignedLayer = 0;
  for (const value of layers.values()) {
    maxAssignedLayer = Math.max(maxAssignedLayer, value);
  }

  for (const node of nodes) {
    if (!layers.has(node.id)) {
      if ("ExternalOutgoing" in node.kind) {
        layers.set(node.id, maxAssignedLayer + 1); // Far right but bounded
      } else {
        layers.set(node.id, 1); // Middle
      }
    }
  }

  // Ensure outgoing nodes are to the right of any non-outgoing content.
  let maxNonOutgoing = 0;
  for (const node of nodes) {
    if (!("ExternalOutgoing" in node.kind)) {
      maxNonOutgoing = Math.max(maxNonOutgoing, layers.get(node.id) ?? 0);
    }
  }
  for (const node of nodes) {
    if ("ExternalOutgoing" in node.kind) {
      const current = layers.get(node.id) ?? maxNonOutgoing + 1;
      layers.set(node.id, Math.max(current, maxNonOutgoing + 1));
    }
  }

  return layers;
}

// ---------------------------------------------------------------------------
// Node Ordering within Layers (Sugiyama Phase 2)
// ---------------------------------------------------------------------------

function orderNodesInLayers(
  nodes: FeynmanNode[],
  edges: Array<[number, number, FeynmanEdge]>,
  layers: Map<number, number>,
): Map<number, number> {
  // Group nodes by layer
  const nodesByLayer = new Map<number, FeynmanNode[]>();
  for (const node of nodes) {
    const layer = layers.get(node.id) ?? 0;
    if (!nodesByLayer.has(layer)) nodesByLayer.set(layer, []);
    nodesByLayer.get(layer)!.push(node);
  }

  // Build edge connectivity and degree information
  const edgeCount = new Map<number, number>();
  for (const [src, tgt] of edges) {
    edgeCount.set(src, (edgeCount.get(src) ?? 0) + 1);
    edgeCount.set(tgt, (edgeCount.get(tgt) ?? 0) + 1);
  }

  // Build adjacency for better ordering
  const adjacency = new Map<number, Set<number>>();
  for (const [src, tgt] of edges) {
    if (!adjacency.has(src)) adjacency.set(src, new Set());
    if (!adjacency.has(tgt)) adjacency.set(tgt, new Set());
    adjacency.get(src)!.add(tgt);
    adjacency.get(tgt)!.add(src);
  }

  // Order within each layer: minimize edge crossings and group related nodes
  const order = new Map<number, number>();
  let globalOrder = 0;

  // Process layers in order
  const sortedLayers = Array.from(nodesByLayer.keys()).sort((a, b) => a - b);

  for (const layer of sortedLayers) {
    const layerNodes = nodesByLayer.get(layer) ?? [];

    // Improved ordering: external nodes on edges, internal by degree
    layerNodes.sort((a, b) => {
      const aIsIncoming = "ExternalIncoming" in a.kind;
      const bIsIncoming = "ExternalIncoming" in b.kind;
      const aIsOutgoing = "ExternalOutgoing" in a.kind;
      const bIsOutgoing = "ExternalOutgoing" in b.kind;
      const aIsInternal = "InternalVertex" in a.kind;
      const bIsInternal = "InternalVertex" in b.kind;

      // Incoming particles first (sorted by degree, higher first)
      if (aIsIncoming && bIsIncoming) {
        return (edgeCount.get(b.id) ?? 0) - (edgeCount.get(a.id) ?? 0);
      }
      if (aIsIncoming && !bIsIncoming) return -1;
      if (!aIsIncoming && bIsIncoming) return 1;

      // Internal vertices by degree (higher degree first for better layout)
      if (aIsInternal && bIsInternal) {
        return (edgeCount.get(b.id) ?? 0) - (edgeCount.get(a.id) ?? 0);
      }
      if (aIsInternal && !bIsInternal && !bIsOutgoing) return -1;
      if (!aIsInternal && !aIsOutgoing && bIsInternal) return 1;

      // Outgoing particles last (sorted by degree, higher first)
      if (aIsOutgoing && bIsOutgoing) {
        return (edgeCount.get(b.id) ?? 0) - (edgeCount.get(a.id) ?? 0);
      }
      if (aIsOutgoing && !bIsOutgoing) return 1;
      if (!bIsOutgoing && bIsOutgoing) return -1;

      // Default: by ID
      return a.id - b.id;
    });

    for (let i = 0; i < layerNodes.length; i++) {
      order.set(layerNodes[i].id, i);
    }
  }

  return order;
}

// ---------------------------------------------------------------------------
// Edge Lane Assignment (for parallel edges)
// ---------------------------------------------------------------------------

function assignEdgeLanes(
  edges: Array<[number, number, FeynmanEdge]>,
): number[] {
  // Group edges by (source, target) pair
  const edgeGroups = new Map<string, number[]>();

  for (let i = 0; i < edges.length; i++) {
    const [src, tgt] = edges[i];
    const key = `${Math.min(src, tgt)}-${Math.max(src, tgt)}`; // Canonical form
    if (!edgeGroups.has(key)) edgeGroups.set(key, []);
    edgeGroups.get(key)!.push(i);
  }

  const lanes = new Array(edges.length).fill(0);

  // Assign lanes within each group with adaptive spacing for high-multiplicity edges
  for (const indices of edgeGroups.values()) {
    if (indices.length === 1) {
      lanes[indices[0]] = 0;
    } else {
      const n = indices.length;
      // For many parallel edges, use non-linear spacing to avoid crowding
      const spreadFactor = Math.min(1.0, 12 / n); // Adaptive spread factor
      for (let i = 0; i < n; i++) {
        const normalized = i / (n - 1); // 0 to 1
        const offset = (normalized - 0.5) * 2 * (n - 1) * spreadFactor;
        lanes[indices[i]] = offset;
      }
    }
  }

  return lanes;
}

// ---------------------------------------------------------------------------
// Position Calculation
// ---------------------------------------------------------------------------

interface LayoutConfig {
  width: number;
  height: number;
  padding: number;
  minLayerSpacing: number;
  minNodeSpacing: number;
}

function calculatePositions(
  nodes: FeynmanNode[],
  layers: Map<number, number>,
  order: Map<number, number>,
  config: LayoutConfig,
): Map<number, { x: number; y: number }> {
  const positions = new Map<number, { x: number; y: number }>();
  const { width, height, padding, minLayerSpacing, minNodeSpacing } = config;

  // Calculate available space
  const availableWidth = width - 2 * padding;
  const availableHeight = height - 2 * padding;

  // Count layers and nodes per layer
  const maxLayer = Math.max(...Array.from(layers.values()));
  const layerCount = Math.max(2, maxLayer + 1);

  // Group nodes by layer
  const nodesByLayer = new Map<number, FeynmanNode[]>();
  for (const node of nodes) {
    const layer = layers.get(node.id) ?? 0;
    if (!nodesByLayer.has(layer)) nodesByLayer.set(layer, []);
    nodesByLayer.get(layer)!.push(node);
  }

  // Calculate adaptive spacing based on graph density
  const maxNodesInLayer = Math.max(...Array.from(nodesByLayer.values()).map((n) => n.length), 1);
  const adaptiveNodeSpacing = Math.max(minNodeSpacing, availableHeight / Math.max(2, maxNodesInLayer + 2));
  const nodeVerticalSize = adaptiveNodeSpacing * 1.2;

  // Assign X coordinates by layer with adaptive spacing
  const layerToX = new Map<number, number>();
  for (let i = 0; i < layerCount; i++) {
    const x = padding + (i / Math.max(1, layerCount - 1)) * availableWidth;
    layerToX.set(i, x);
  }

  // Assign Y coordinates within each layer with improved spacing to prevent overlaps
  for (const [layer, layerNodes] of nodesByLayer) {
    const x = layerToX.get(layer) ?? width / 2;
    const nodeCount = layerNodes.length;

    if (nodeCount === 1) {
      // Center single node vertically
      const y = height / 2;
      const node = layerNodes[0];
      positions.set(node.id, { x, y });
    } else if (nodeCount === 2) {
      // Two nodes: spread vertically with good spacing
      const y1 = height / 2 - nodeVerticalSize / 2;
      const y2 = height / 2 + nodeVerticalSize / 2;
      positions.set(layerNodes[0].id, { x, y: y1 });
      positions.set(layerNodes[1].id, { x, y: y2 });
    } else {
      // Multiple nodes: distribute evenly with minimum spacing
      const requiredHeight = nodeCount * nodeVerticalSize;
      const totalHeight = Math.max(availableHeight, requiredHeight + minNodeSpacing * 2);
      const startY = (height - totalHeight) / 2 + minNodeSpacing;
      const spacing = totalHeight / (nodeCount + 1);

      for (let i = 0; i < nodeCount; i++) {
        const y = Math.max(padding + 20, Math.min(height - padding - 20, startY + spacing * (i + 1)));
        const node = layerNodes[i];
        positions.set(node.id, { x, y });
      }
    }
  }

  return positions;
}

// ---------------------------------------------------------------------------
// Main Layout Function
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Validation & Diagnostics
// ---------------------------------------------------------------------------

function validateDiagram(diag: FeynmanDiagram): DiagramValidation {
  const warnings: string[] = [];
  const errors: string[] = [];

  // Check for external nodes with direct connections between them
  const externalNodeIds = new Set<number>();
  for (const node of diag.nodes) {
    if ("ExternalIncoming" in node.kind || "ExternalOutgoing" in node.kind) {
      externalNodeIds.add(node.id);
    }
  }

  for (const [src, tgt] of diag.edges) {
    if (externalNodeIds.has(src) && externalNodeIds.has(tgt)) {
      warnings.push(`Edge between external nodes (${src}→${tgt}): may cause poor layout`);
    }
    // Check for self-loops
    if (src === tgt) {
      warnings.push(`Self-loop detected on node ${src}`);
    }
  }

  // Check for isolated nodes
  const connectedNodes = new Set<number>();
  for (const [src, tgt] of diag.edges) {
    connectedNodes.add(src);
    connectedNodes.add(tgt);
  }
  const isolatedNodes = diag.nodes.filter((n) => !connectedNodes.has(n.id));
  if (isolatedNodes.length > 0) {
    warnings.push(`Found ${isolatedNodes.length} isolated node(s) with no connections`);
  }

  // Check for high edge multiplicity
  const edgeCount = new Map<string, number>();
  for (const [src, tgt] of diag.edges) {
    const key = `${Math.min(src, tgt)}-${Math.max(src, tgt)}`;
    edgeCount.set(key, (edgeCount.get(key) ?? 0) + 1);
  }
  const maxEdges = Math.max(...Array.from(edgeCount.values()), 0);
  if (maxEdges > 6) {
    warnings.push(`High edge multiplicity detected: ${maxEdges} parallel edges (consider simplifying)`);
  }

  // Check diagram has at least one internal vertex
  const hasInternalVertex = diag.nodes.some((n) => !("ExternalIncoming" in n.kind || "ExternalOutgoing" in n.kind));
  if (!hasInternalVertex && diag.nodes.length > 0) {
    warnings.push(`No internal vertices found: diagram may be trivial`);
  }

  return { isValid: errors.length === 0, warnings, errors };
}

export function layoutDiagram(
  diag: FeynmanDiagram,
  canvasWidth: number,
  canvasHeight: number,
): DiagramLayout {
  // Validate diagram structure
  const validation = validateDiagram(diag);
  if (validation.warnings.length > 0) {
    console.warn("Diagram layout warnings:", validation.warnings);
  }
  if (validation.errors.length > 0) {
    console.error("Diagram layout errors:", validation.errors);
  }

  // Log diagram statistics for diagnostics
  const externalNodeCount = diag.nodes.filter((n) => "ExternalIncoming" in n.kind || "ExternalOutgoing" in n.kind).length;
  const internalNodeCount = diag.nodes.length - externalNodeCount;
  console.debug(`[Layout] Diagram: ${internalNodeCount} internal + ${externalNodeCount} external nodes, ${diag.edges.length} edges`);

  const config: LayoutConfig = {
    width: canvasWidth,
    height: canvasHeight,
    padding: 80,
    minLayerSpacing: 100,
    minNodeSpacing: 70,
  };

  // Phase 1: Assign layers
  const layers = assignLayers(diag.nodes, diag.edges);

  // Phase 2: Order nodes within layers
  const order = orderNodesInLayers(diag.nodes, diag.edges, layers);

  // Phase 3: Calculate positions
  const positions = calculatePositions(diag.nodes, layers, order, config);

  // Phase 4: Assign edge lanes for parallel edges
  const edgeLanes = assignEdgeLanes(diag.edges);

  // Build output layout nodes
  const layoutNodes: LayoutNode[] = diag.nodes.map((node) => {
    const pos = positions.get(node.id) ?? { x: canvasWidth / 2, y: canvasHeight / 2 };
    const layer = layers.get(node.id) ?? 0;
    const orderInLayer = order.get(node.id) ?? 0;
    let label = "";

    if ("ExternalIncoming" in node.kind) {
      label = node.kind.ExternalIncoming.field.symbol;
    } else if ("ExternalOutgoing" in node.kind) {
      label = node.kind.ExternalOutgoing.field.symbol;
    }

    return {
      id: node.id,
      x: pos.x,
      y: pos.y,
      kind: node.kind,
      label,
      layer,
      orderInLayer,
    };
  });

  // Build output layout edges
  const layoutEdges: LayoutEdge[] = diag.edges.map((edge, idx) => ({
    srcId: edge[0],
    tgtId: edge[1],
    edgeIndex: idx,
    laneOffset: edgeLanes[idx] ?? 0,
    edge: edge[2],
  }));

  return {
    nodes: layoutNodes,
    edges: layoutEdges,
    width: canvasWidth,
    height: canvasHeight,
  };
}
