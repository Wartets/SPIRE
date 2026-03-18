/**
 * SPIRE - Feynman Diagram SVG Renderer
 *
 * Renders proper Feynman diagrams following physics conventions:
 *
 *   • Fermions:  solid lines with arrows (→)
 *   • Photons:   wavy/sinusoidal lines  (∿)
 *   • Gluons:    curly/loopy lines      (⌇)
 *   • Scalars:   dashed lines           (- -)
 *   • W/Z/massive vectors: wavy lines
 *   • Gravitons: double wavy lines
 *   • Vertices:  filled dots
 *   • External legs: labelled with particle symbol + momentum
 *
 *   - New hierarchical layout engine with left-to-right orientation
 *   - Planar edge routing to minimize crossings
 *   - Proper handling of multi-edge connections
 *   - Fixed connection point calculations
 *   - No zoom/pan on mouse wheel
 *
 * The renderer produces a pure SVG string from a FeynmanDiagram object.
 * No external dependencies required.
 */

import type {
  FeynmanDiagram,
  FeynmanNode,
  FeynmanEdge,
  NodeKind,
  PropagatorForm,
} from "$lib/types/spire";
import { layoutDiagram as computeLayout, type DiagramLayout } from "./layoutEngine";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export interface FeynmanRenderOptions {
  width: number;
  height: number;
  padding: number;
  /** Optional per-node position override for manual editing in UI. */
  nodePositionOverrides?: Record<number, { x: number; y: number }>;
  /** Scale of the propagator wave/curl patterns. */
  patternScale: number;
  /** Colors */
  colors: {
    fermion: string;
    photon: string;
    gluon: string;
    scalar: string;
    higgs: string;
    vertex: string;
    label: string;
    background: string;
    momentum: string;
    externalIn: string;
    externalOut: string;
  };
}

const DEFAULT_OPTIONS: FeynmanRenderOptions = {
  width: 600,
  height: 400,
  padding: 60,
  patternScale: 1.0,
  colors: {
    fermion: "#e8e8e8",
    photon: "#5eb8ff",
    gluon: "#2ecc71",
    scalar: "#d4a017",
    higgs: "#d4a017",
    vertex: "#ffffff",
    label: "#e8e8e8",
    background: "transparent",
    momentum: "#888888",
    externalIn: "#2ecc71",
    externalOut: "#e74c3c",
  },
};

// ---------------------------------------------------------------------------
// Line Style Classification
// ---------------------------------------------------------------------------

type LineStyle = "fermion" | "antifermion" | "photon" | "gluon" | "scalar" | "higgs" | "graviton" | "massive_vector";

function isLikelyAntiParticleField(edge: FeynmanEdge): boolean {
  const id = edge.field.id?.toLowerCase() ?? "";
  const name = edge.field.name?.toLowerCase() ?? "";
  const symbol = edge.field.symbol ?? "";
  return (
    id.includes("_bar") ||
    id.includes("bar") ||
    id.endsWith("+") ||
    name.includes("anti") ||
    symbol.includes("+") ||
    symbol.includes("̄")
  );
}

function classifyLineStyle(
  edge: FeynmanEdge,
  srcKind?: NodeKind,
  tgtKind?: NodeKind,
): LineStyle {
  const form = edge.propagator?.form;
  const spin = edge.field.quantum_numbers?.spin ?? edge.propagator?.spin ?? 0;
  const mass = edge.field.mass ?? edge.propagator?.mass ?? 0;
  const anti = isLikelyAntiParticleField(edge);

  const externalIncoming = Boolean(srcKind && "ExternalIncoming" in srcKind);
  const externalOutgoing = Boolean(tgtKind && "ExternalOutgoing" in tgtKind);
  const inferFermionStyle = (): LineStyle => {
    if (externalIncoming || externalOutgoing) {
      return anti ? "antifermion" : "fermion";
    }
    return anti ? "antifermion" : "fermion";
  };

  // Propagator form takes priority
  if (form) {
    switch (form) {
      case "DiracFermion": return inferFermionStyle();
      case "MasslessVector": return "photon";
      case "MassiveVector": return "massive_vector";
      case "Scalar": return mass > 100 ? "higgs" : "scalar";
      case "RaritaSchwinger": return inferFermionStyle();
      case "MasslessSpin2":
      case "MassiveSpin2": return "graviton";
      default: break;
    }
  }

  // Fall back to spin
  if (spin === 1) return inferFermionStyle(); // spin-½ (twice_spin = 1)
  if (spin === 2 && mass === 0) return "photon"; // spin-1 massless
  if (spin === 2 && mass > 0) return "massive_vector"; // spin-1 massive
  if (spin === 0) return "scalar";
  if (spin === 4) return "graviton"; // spin-2

  // Check field name heuristics
  const name = edge.field.name?.toLowerCase() ?? "";
  const id = edge.field.id?.toLowerCase() ?? "";
  if (name.includes("gluon") || id === "g") return "gluon";
  if (name.includes("photon") || id === "gamma" || id === "a") return "photon";
  if (name.includes("higgs") || id === "h") return "higgs";
  if (name.includes("w") || name.includes("z")) return "massive_vector";

  return "fermion"; // default
}

function isExternalIncomingKind(kind: NodeKind | undefined): boolean {
  return Boolean(kind && "ExternalIncoming" in kind);
}

function isExternalOutgoingKind(kind: NodeKind | undefined): boolean {
  return Boolean(kind && "ExternalOutgoing" in kind);
}

function shouldReverseFermionArrow(
  style: LineStyle,
  srcKind: NodeKind | undefined,
  tgtKind: NodeKind | undefined,
): boolean {
  const anti = style === "antifermion";

  const srcIncoming = isExternalIncomingKind(srcKind);
  const tgtIncoming = isExternalIncomingKind(tgtKind);
  const srcOutgoing = isExternalOutgoingKind(srcKind);
  const tgtOutgoing = isExternalOutgoingKind(tgtKind);

  // Incoming particle line should point toward interaction vertex.
  if (srcIncoming && !tgtIncoming) return anti;
  if (tgtIncoming && !srcIncoming) return !anti;

  // Outgoing particle line should point away from interaction vertex.
  if (tgtOutgoing && !srcOutgoing) return anti;
  if (srcOutgoing && !tgtOutgoing) return !anti;

  // Internal lines: keep anti-particles reversed by convention.
  return anti;
}

// ---------------------------------------------------------------------------
// Layout: Position nodes in a left-to-right Feynman diagram layout
// ---------------------------------------------------------------------------

interface PositionedNode {
  id: number;
  x: number;
  y: number;
  kind: NodeKind;
  label: string;
}

interface PositionedEdge {
  srcId: number;
  tgtId: number;
  srcX: number;
  srcY: number;
  tgtX: number;
  tgtY: number;
  edge: FeynmanEdge;
  style: LineStyle;
  laneOffset?: number;
}
interface EndpointPortSlot {
  slot: number;
  total: number;
}

function endpointKey(edgeIndex: number, isSource: boolean): string {
  return `${edgeIndex}:${isSource ? "s" : "t"}`;
}

function computeEndpointPortSlots(edges: PositionedEdge[]): Map<string, EndpointPortSlot> {
  const incidence = new Map<number, Array<{ edgeIndex: number; isSource: boolean; angle: number }>>();

  for (let edgeIndex = 0; edgeIndex < edges.length; edgeIndex++) {
    const e = edges[edgeIndex];
    const dx = e.tgtX - e.srcX;
    const dy = e.tgtY - e.srcY;
    const srcAngle = Math.atan2(dy, dx);
    const tgtAngle = Math.atan2(-dy, -dx);

    const srcList = incidence.get(e.srcId) ?? [];
    srcList.push({ edgeIndex, isSource: true, angle: srcAngle });
    incidence.set(e.srcId, srcList);

    const tgtList = incidence.get(e.tgtId) ?? [];
    tgtList.push({ edgeIndex, isSource: false, angle: tgtAngle });
    incidence.set(e.tgtId, tgtList);
  }

  const slots = new Map<string, EndpointPortSlot>();

  for (const entries of incidence.values()) {
    entries.sort((a, b) => {
      if (a.angle !== b.angle) return a.angle - b.angle;
      if (a.edgeIndex !== b.edgeIndex) return a.edgeIndex - b.edgeIndex;
      return Number(a.isSource) - Number(b.isSource);
    });

    const total = entries.length;
    for (let i = 0; i < total; i++) {
      const entry = entries[i];
      slots.set(endpointKey(entry.edgeIndex, entry.isSource), {
        slot: i,
        total,
      });
    }
  }

  return slots;
}

function endpointSignedRank(slot: number, total: number): number {
  return slot - (total - 1) / 2;
}

function endpointSpreadAngle(slot: number, total: number): number {
  if (total <= 1) return 0;
  const maxHalfSpread = Math.min(Math.PI * 0.42, ((total - 1) * 0.2) / 2);
  const step = (2 * maxHalfSpread) / (total - 1);
  return -maxHalfSpread + slot * step;
}

function attachPointOnNodeBoundary(
  nodeX: number,
  nodeY: number,
  towardX: number,
  towardY: number,
  radius: number,
  slot: number,
  total: number,
): { x: number; y: number } {
  const dx = towardX - nodeX;
  const dy = towardY - nodeY;
  const dist = Math.sqrt(dx * dx + dy * dy);
  
  // If no direction, use a default
  if (dist < 0.001) {
    return { x: nodeX + radius, y: nodeY };
  }
  
  // Base angle toward target
  const baseAngle = Math.atan2(dy, dx);
  
  // Spread angle for multi-edges
  let spreadAngle = 0;
  if (total > 1) {
    const maxSpread = Math.min(Math.PI * 0.35, (total - 1) * 0.15);
    const step = (2 * maxSpread) / (total - 1);
    spreadAngle = -maxSpread + slot * step;
  }
  
  const finalAngle = baseAngle + spreadAngle;
  return {
    x: nodeX + Math.cos(finalAngle) * radius,
    y: nodeY + Math.sin(finalAngle) * radius,
  };
}

function isExternalNode(kind: NodeKind): boolean {
  return "ExternalIncoming" in kind || "ExternalOutgoing" in kind;
}

function nodeRenderRadius(kind: NodeKind): number {
  return isExternalNode(kind) ? 4 : 5;
}

function nodeAttachRadius(kind: NodeKind): number {
  return nodeRenderRadius(kind) + 2;
}

function centerPositionedNodes(
  nodes: PositionedNode[],
  width: number,
  height: number,
  padding: number,
): PositionedNode[] {
  if (nodes.length === 0) return nodes;

  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of nodes) {
    const r = nodeAttachRadius(node.kind);
    minX = Math.min(minX, node.x - r);
    maxX = Math.max(maxX, node.x + r);
    minY = Math.min(minY, node.y - r);
    maxY = Math.max(maxY, node.y + r);
  }

  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  let dx = width / 2 - centerX;
  let dy = height / 2 - centerY;

  const projectedMinX = minX + dx;
  const projectedMaxX = maxX + dx;
  const projectedMinY = minY + dy;
  const projectedMaxY = maxY + dy;

  if (projectedMinX < padding) dx += padding - projectedMinX;
  if (projectedMaxX > width - padding) dx -= projectedMaxX - (width - padding);
  if (projectedMinY < padding) dy += padding - projectedMinY;
  if (projectedMaxY > height - padding) dy -= projectedMaxY - (height - padding);

  return nodes.map((node) => ({
    ...node,
    x: node.x + dx,
    y: node.y + dy,
  }));
}

function layoutDiagram(
  diag: FeynmanDiagram,
  width: number,
  height: number,
  padding: number,
): { nodes: PositionedNode[]; edges: PositionedEdge[] } {
  // Use the new hierarchical layout engine
  const layout = computeLayout(diag, width, height);
  
  const nodeKindById = new Map<number, NodeKind>(diag.nodes.map((n) => [n.id, n.kind]));
  const posMap = new Map<number, { x: number; y: number }>(
    layout.nodes.map((n) => [n.id, { x: n.x, y: n.y }])
  );

  // Build PositionedNode array from layout nodes
  const pNodes: PositionedNode[] = layout.nodes.map((lnode) => ({
    id: lnode.id,
    x: lnode.x,
    y: lnode.y,
    kind: lnode.kind,
    label: lnode.label,
  }));

  // Build PositionedEdge array from layout edges
  const pEdges: PositionedEdge[] = layout.edges.map((ledge) => {
    const srcPos = posMap.get(ledge.srcId) ?? { x: 0, y: 0 };
    const tgtPos = posMap.get(ledge.tgtId) ?? { x: 0, y: 0 };
    const srcKind = nodeKindById.get(ledge.srcId);
    const tgtKind = nodeKindById.get(ledge.tgtId);

    return {
      srcId: ledge.srcId,
      tgtId: ledge.tgtId,
      srcX: srcPos.x,
      srcY: srcPos.y,
      tgtX: tgtPos.x,
      tgtY: tgtPos.y,
      edge: ledge.edge,
      style: classifyLineStyle(ledge.edge, srcKind, tgtKind),
      laneOffset: ledge.laneOffset,
    };
  });

  return { nodes: pNodes, edges: pEdges };
}

function computeEdgeLaneOffsets(edges: PositionedEdge[]): number[] {
  const groups = new Map<string, number[]>();
  for (let i = 0; i < edges.length; i++) {
    const e = edges[i];
    const sx = Math.min(e.srcX, e.tgtX).toFixed(1);
    const tx = Math.max(e.srcX, e.tgtX).toFixed(1);
    const key = `${sx}:${tx}`;
    const list = groups.get(key) ?? [];
    list.push(i);
    groups.set(key, list);
  }

  const offsets = new Array<number>(edges.length).fill(0);
  for (const idxs of groups.values()) {
    idxs.sort((a, b) => {
      const ea = edges[a];
      const eb = edges[b];
      const ma = (ea.srcY + ea.tgtY) * 0.5;
      const mb = (eb.srcY + eb.tgtY) * 0.5;
      return ma - mb;
    });
    const n = idxs.length;
    for (let i = 0; i < n; i++) {
      offsets[idxs[i]] = i - (n - 1) / 2;
    }
  }
  return offsets;
}

function curvedQuadraticPath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  laneOffset: number,
): string {
  const mx = (x1 + x2) * 0.5;
  const my = (y1 + y2) * 0.5;
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.sqrt(dx * dx + dy * dy) || 1;
  const nx = -dy / len;
  const ny = dx / len;
  const cx = mx + nx * laneOffset;
  const cy = my + ny * laneOffset;
  return `M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`;
}

// ---------------------------------------------------------------------------
// SVG Path Generators for Different Propagator Types
// ---------------------------------------------------------------------------

/** Straight line from (x1,y1) to (x2,y2). */
function straightPath(x1: number, y1: number, x2: number, y2: number): string {
  return `M ${x1} ${y1} L ${x2} ${y2}`;
}

/**
 * Sinusoidal (wavy) path for photons.
 * Creates a sine wave along the line between two points.
 */
function wavyPath(
  x1: number, y1: number, x2: number, y2: number,
  amplitude: number = 6, wavelength: number = 16,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return `M ${x1} ${y1}`;

  const numWaves = Math.max(2, Math.round(dist / wavelength));
  const segments = numWaves * 4; // 4 segments per wave
  const stepX = dx / segments;
  const stepY = dy / segments;
  // Normal direction
  const nx = -dy / dist;
  const ny = dx / dist;

  let d = `M ${x1} ${y1}`;
  for (let i = 1; i <= segments; i++) {
    const t = i / segments;
    const baseX = x1 + dx * t;
    const baseY = y1 + dy * t;
    const wave = Math.sin(t * numWaves * 2 * Math.PI) * amplitude;
    d += ` L ${baseX + nx * wave} ${baseY + ny * wave}`;
  }
  return d;
}

/**
 * Curly (loopy) path for gluons.
 * Creates loops along the line between two points.
 */
function curlyPath(
  x1: number, y1: number, x2: number, y2: number,
  loopRadius: number = 7, numLoops: number | null = null,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return `M ${x1} ${y1}`;

  const loops = numLoops ?? Math.max(3, Math.round(dist / 20));
  // Unit vectors
  const ux = dx / dist;
  const uy = dy / dist;
  const nx = -uy;
  const ny = ux;

  let d = `M ${x1} ${y1}`;
  const loopStep = dist / loops;

  for (let i = 0; i < loops; i++) {
    const startT = i / loops;
    const endT = (i + 1) / loops;
    const midT = (startT + endT) / 2;

    const sx = x1 + dx * startT;
    const sy = y1 + dy * startT;
    const ex = x1 + dx * endT;
    const ey = y1 + dy * endT;
    const mx = x1 + dx * midT;
    const my = y1 + dy * midT;

    // Control points for a semicircular arc
    const side = (i % 2 === 0) ? 1 : -1;
    const cp1x = sx + nx * loopRadius * 1.8 * side + ux * loopStep * 0.25;
    const cp1y = sy + ny * loopRadius * 1.8 * side + uy * loopStep * 0.25;
    const cp2x = ex + nx * loopRadius * 1.8 * side - ux * loopStep * 0.25;
    const cp2y = ey + ny * loopRadius * 1.8 * side - uy * loopStep * 0.25;

    d += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${ex} ${ey}`;
  }

  return d;
}

/** Dashed line (for scalars) - returns a straight path with dash-array styling. */
function dashedPath(x1: number, y1: number, x2: number, y2: number): string {
  return straightPath(x1, y1, x2, y2);
}

/** Double wavy path for gravitons. */
function doubleWavyPath(
  x1: number, y1: number, x2: number, y2: number,
  amplitude: number = 5, wavelength: number = 14, separation: number = 3,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return `M ${x1} ${y1}`;
  const nx = -dy / dist;
  const ny = dx / dist;

  const path1 = wavyPath(
    x1 + nx * separation, y1 + ny * separation,
    x2 + nx * separation, y2 + ny * separation,
    amplitude, wavelength,
  );
  const path2 = wavyPath(
    x1 - nx * separation, y1 - ny * separation,
    x2 - nx * separation, y2 - ny * separation,
    amplitude, wavelength,
  );
  return path1 + " " + path2;
}

// ---------------------------------------------------------------------------
// Arrow marker for fermion lines
// ---------------------------------------------------------------------------

function arrowMarkerDef(id: string, color: string): string {
  return `<marker id="${id}" viewBox="0 0 10 10" refX="8" refY="5"
    markerWidth="6" markerHeight="6" orient="auto-start-reverse">
    <path d="M 0 0 L 10 5 L 0 10 z" fill="${color}" />
  </marker>`;
}

function formatParticleLabel(raw: string): string {
  let s = (raw ?? "").trim();
  if (!s) return s;
  if (s.startsWith("$") && s.endsWith("$")) s = s.slice(1, -1);
  s = s.replace(/^\\/, "");

  const greek: Record<string, string> = {
    mu: "μ",
    nu: "ν",
    tau: "τ",
    gamma: "γ",
    alpha: "α",
    beta: "β",
    rho: "ρ",
    phi: "ϕ",
    pi: "π",
    ell: "ℓ",
  };

  if (greek[s.toLowerCase()]) return greek[s.toLowerCase()];

  s = s
    .replace(/\\mu/gi, "μ")
    .replace(/\\nu/gi, "ν")
    .replace(/\\tau/gi, "τ")
    .replace(/\\gamma/gi, "γ")
    .replace(/\\ell/gi, "ℓ")
    .replace(/\^\{-\}/g, "⁻")
    .replace(/\^\{\+\}/g, "⁺")
    .replace(/\^-/g, "⁻")
    .replace(/\^\+/g, "⁺")
    .replace(/([A-Za-zα-ωΑ-Ω])-(?!\w)/g, "$1⁻")
    .replace(/([A-Za-zα-ωΑ-Ω])\+(?!\w)/g, "$1⁺");

  if (s === "e-") return "e⁻";
  if (s === "e+") return "e⁺";
  if (s === "mu-") return "μ⁻";
  if (s === "mu+") return "μ⁺";
  return s;
}

// ---------------------------------------------------------------------------
// Main Render Function
// ---------------------------------------------------------------------------

export function renderFeynmanDiagram(
  diag: FeynmanDiagram,
  opts?: Partial<FeynmanRenderOptions>,
): string {
  const o: FeynmanRenderOptions = { ...DEFAULT_OPTIONS, ...opts };
  const { width, height, padding, colors } = o;

  let { nodes, edges } = layoutDiagram(diag, width, height, padding);

  if (o.nodePositionOverrides && Object.keys(o.nodePositionOverrides).length > 0) {
    nodes = nodes.map((node) => {
      const override = o.nodePositionOverrides?.[node.id];
      return override
        ? { ...node, x: override.x, y: override.y }
        : node;
    });

    const nodePos = new Map<number, { x: number; y: number }>(
      nodes.map((n) => [n.id, { x: n.x, y: n.y }])
    );

    edges = edges.map((edge) => {
      const src = nodePos.get(edge.srcId);
      const tgt = nodePos.get(edge.tgtId);
      return {
        ...edge,
        srcX: src?.x ?? edge.srcX,
        srcY: src?.y ?? edge.srcY,
        tgtX: tgt?.x ?? edge.tgtX,
        tgtY: tgt?.y ?? edge.tgtY,
      };
    });
  }

  const nodeById = new Map<number, PositionedNode>(nodes.map((node) => [node.id, node]));

  // Collect unique marker IDs
  const markers: string[] = [];
  const markerIds = new Set<string>();

  function ensureMarker(style: LineStyle): string {
    const color =
      style === "fermion" || style === "antifermion"
        ? colors.fermion
        : style === "photon" || style === "massive_vector"
        ? colors.photon
        : style === "gluon"
        ? colors.gluon
        : colors.scalar;
    const id = `arrow-${style}`;
    if (!markerIds.has(id)) {
      markerIds.add(id);
      markers.push(arrowMarkerDef(id, color));
    }
    return id;
  }

  // Build SVG elements
  const edgeSvg: string[] = [];
  const labelSvg: string[] = [];
  const occupiedLabelBoxes: Array<{ x1: number; y1: number; x2: number; y2: number }> = [];

  const edgeCount = edges.length;
  const nodeCount = nodes.length;
  const complexity = edgeCount * nodeCount;
  const denseDiagram = edgeCount >= 80 || nodeCount >= 60 || complexity >= 2400;
  const veryDenseDiagram = edgeCount >= 140 || nodeCount >= 95 || complexity >= 5200;
  const useCollisionAwareMomentumLabels = !denseDiagram;
  const momentumLabelStride = veryDenseDiagram
    ? Math.max(2, Math.ceil(edgeCount / 56))
    : denseDiagram
    ? Math.max(1, Math.ceil(edgeCount / 120))
    : 1;

  const edgePairGroups = new Map<string, number[]>();
  for (let i = 0; i < edges.length; i++) {
    const e = edges[i];
    const key = `${Math.min(e.srcId, e.tgtId)}-${Math.max(e.srcId, e.tgtId)}`;
    if (!edgePairGroups.has(key)) edgePairGroups.set(key, []);
    edgePairGroups.get(key)!.push(i);
  }

  const slotIndexByEdge = new Map<number, { slotIndex: number; totalSlots: number }>();
  for (const idxs of edgePairGroups.values()) {
    const totalSlots = idxs.length;
    for (let s = 0; s < idxs.length; s++) {
      slotIndexByEdge.set(idxs[s], { slotIndex: s, totalSlots });
    }
  }

  for (let edgeIndex = 0; edgeIndex < edges.length; edgeIndex++) {
    const pe = edges[edgeIndex];
    const { srcId, tgtId, srcX, srcY, tgtX, tgtY, edge, style } = pe;

    const srcNode = nodeById.get(srcId);
    const tgtNode = nodeById.get(tgtId);
    
    // Skip edges between two external nodes (invalid in Feynman diagrams)
    const srcExternal = srcNode && ("ExternalIncoming" in srcNode.kind || "ExternalOutgoing" in srcNode.kind);
    const tgtExternal = tgtNode && ("ExternalIncoming" in tgtNode.kind || "ExternalOutgoing" in tgtNode.kind);
    if (srcExternal && tgtExternal) {
      console.warn(`[Feynman] Skipping edge ${srcId}→${tgtId}: both nodes are external particles`);
      continue;
    }
    
    const srcInset = srcNode ? nodeAttachRadius(srcNode.kind) : 6;
    const tgtInset = tgtNode ? nodeAttachRadius(tgtNode.kind) : 6;

    // Precomputed slot assignment for parallel edges
    const slotInfo = slotIndexByEdge.get(edgeIndex) ?? { slotIndex: 0, totalSlots: 1 };
    const { slotIndex, totalSlots } = slotInfo;

    const srcAttach = attachPointOnNodeBoundary(srcX, srcY, tgtX, tgtY, srcInset, slotIndex, totalSlots);
    const tgtAttach = attachPointOnNodeBoundary(tgtX, tgtY, srcX, srcY, tgtInset, slotIndex, totalSlots);

    const ax = srcAttach.x;
    const ay = srcAttach.y;
    const bx = tgtAttach.x;
    const by = tgtAttach.y;

    const laneOffset = (pe.laneOffset ?? 0) * 11;

    let path: string;
    let strokeColor: string;
    let strokeWidth = 1.8;
    let dashArray = "";
    let markerEnd = "";

    switch (style) {
      case "fermion":
        path = shouldReverseFermionArrow(style, srcNode?.kind, tgtNode?.kind)
          ? curvedQuadraticPath(bx, by, ax, ay, -laneOffset)
          : curvedQuadraticPath(ax, ay, bx, by, laneOffset);
        strokeColor = colors.fermion;
        markerEnd = `url(#${ensureMarker("fermion")})`;
        break;
      case "antifermion":
        path = shouldReverseFermionArrow(style, srcNode?.kind, tgtNode?.kind)
          ? curvedQuadraticPath(bx, by, ax, ay, -laneOffset)
          : curvedQuadraticPath(ax, ay, bx, by, laneOffset);
        strokeColor = colors.fermion;
        markerEnd = `url(#${ensureMarker("antifermion")})`;
        break;
      case "photon":
      case "massive_vector":
        path = veryDenseDiagram
          ? curvedQuadraticPath(ax, ay, bx, by, laneOffset * 0.6)
          : wavyPath(
              ax,
              ay,
              bx,
              by,
              (denseDiagram ? 4.8 : 6) * o.patternScale,
              (denseDiagram ? 20 : 16) * o.patternScale,
            );
        strokeColor = colors.photon;
        break;
      case "gluon":
        path = veryDenseDiagram
          ? curvedQuadraticPath(ax, ay, bx, by, laneOffset * 0.55)
          : curlyPath(ax, ay, bx, by, (denseDiagram ? 5.5 : 7) * o.patternScale);
        strokeColor = colors.gluon;
        strokeWidth = 1.6;
        break;
      case "scalar":
        path = curvedQuadraticPath(ax, ay, bx, by, laneOffset);
        strokeColor = colors.scalar;
        dashArray = "6 4";
        break;
      case "higgs":
        path = curvedQuadraticPath(ax, ay, bx, by, laneOffset);
        strokeColor = colors.higgs;
        dashArray = "6 4";
        strokeWidth = 2.2;
        break;
      case "graviton":
        path = veryDenseDiagram
          ? curvedQuadraticPath(ax, ay, bx, by, laneOffset * 0.6)
          : doubleWavyPath(
              ax,
              ay,
              bx,
              by,
              (denseDiagram ? 3.2 : 4) * o.patternScale,
              (denseDiagram ? 15 : 12) * o.patternScale,
            );
        strokeColor = colors.photon;
        strokeWidth = 1.4;
        break;
      default:
        path = curvedQuadraticPath(ax, ay, bx, by, laneOffset);
        strokeColor = colors.fermion;
    }

    // Subtle underlay for contrast on dense diagrams.
    edgeSvg.push(
      `<path d="${path}" fill="none" stroke="rgba(0,0,0,0.38)" stroke-width="${strokeWidth + 2.1}"` +
      ` stroke-linecap="round" stroke-linejoin="round" />`
    );

    edgeSvg.push(
      `<path d="${path}" fill="none" stroke="${strokeColor}" stroke-width="${strokeWidth}"` +
      (dashArray ? ` stroke-dasharray="${dashArray}"` : "") +
      (markerEnd ? ` marker-end="${markerEnd}"` : "") +
      ` stroke-linecap="round" stroke-linejoin="round" />`
    );

    // Momentum label with collision-aware placement
    if (edge.momentum_label && edgeIndex % momentumLabelStride === 0) {
      const midX = (ax + bx) / 2;
      const midY = (ay + by) / 2;
      const ndx = -(by - ay);
      const ndy = bx - ax;
      const nd = Math.sqrt(ndx * ndx + ndy * ndy) || 1;
      const laneDistance = Math.max(2, Math.abs(laneOffset || 0));
      const preferredSide = totalSlots > 1 ? (laneOffset >= 0 ? 1 : -1) : (edgeIndex % 2 === 0 ? 1 : -1);

      const label = escSvg(edge.momentum_label);

      const approxTextWidth = Math.max(20, edge.momentum_label.length * 6.2);
      const approxTextHeight = 12;

      const collidesWithPlacedLabels = (x: number, y: number): boolean => {
        if (!useCollisionAwareMomentumLabels) return false;
        const box = {
          x1: x - approxTextWidth / 2 - 4,
          y1: y - approxTextHeight / 2 - 3,
          x2: x + approxTextWidth / 2 + 4,
          y2: y + approxTextHeight / 2 + 3,
        };

        for (const taken of occupiedLabelBoxes) {
          const overlap = !(box.x2 < taken.x1 || box.x1 > taken.x2 || box.y2 < taken.y1 || box.y1 > taken.y2);
          if (overlap) return true;
        }

        for (const node of nodes) {
          const dx = x - node.x;
          const dy = y - node.y;
          const minDist = ("ExternalIncoming" in node.kind || "ExternalOutgoing" in node.kind) ? 16 : 14;
          if (dx * dx + dy * dy < minDist * minDist) {
            return true;
          }
        }

        return false;
      };

      let chosenX = midX;
      let chosenY = midY;
      let placed = false;
      const candidates = useCollisionAwareMomentumLabels ? [10, 14, 18, 24, 30] : [12, 18];
      const sides = [preferredSide, -preferredSide];

      for (const offsetBase of candidates) {
        const dynamicOffset = offsetBase + Math.min(16, laneDistance * 1.8);
        for (const side of sides) {
          const lx = midX + (ndx / nd) * dynamicOffset * side;
          const ly = midY + (ndy / nd) * dynamicOffset * side;
          if (!collidesWithPlacedLabels(lx, ly)) {
            chosenX = lx;
            chosenY = ly;
            placed = true;
            break;
          }
        }
        if (placed) break;
      }

      if (!placed) {
        const fallbackOffset = 34 + Math.min(10, laneDistance);
        chosenX = midX + (ndx / nd) * fallbackOffset * preferredSide;
        chosenY = midY + (ndy / nd) * fallbackOffset * preferredSide;
      }

      if (useCollisionAwareMomentumLabels) {
        occupiedLabelBoxes.push({
          x1: chosenX - approxTextWidth / 2 - 4,
          y1: chosenY - approxTextHeight / 2 - 3,
          x2: chosenX + approxTextWidth / 2 + 4,
          y2: chosenY + approxTextHeight / 2 + 3,
        });
      }

      labelSvg.push(
        `<text x="${chosenX}" y="${chosenY}" fill="${colors.momentum}" font-size="10" font-family="'Fira Code',monospace" text-anchor="middle" dominant-baseline="middle" font-style="italic" opacity="0.95">${label}</text>`
      );
    }
  }

  // Node rendering
  const nodeSvg: string[] = [];
  for (const pn of nodes) {
    const isExternal =
      "ExternalIncoming" in pn.kind || "ExternalOutgoing" in pn.kind;
    const isIncoming = "ExternalIncoming" in pn.kind;

    if (isExternal) {
      // External particle: small circle + label
      const labelColor = isIncoming ? colors.externalIn : colors.externalOut;
      nodeSvg.push(
        `<circle class="node-anchor" data-node-id="${pn.id}" cx="${pn.x}" cy="${pn.y}" r="4" fill="${labelColor}" stroke="none" />`
      );
      // Label: to the left for incoming, to the right for outgoing
      const labelX = isIncoming ? pn.x - 18 : pn.x + 18;
      const anchor = isIncoming ? "end" : "start";
      nodeSvg.push(
        `<text data-node-id="${pn.id}" x="${labelX}" y="${pn.y}" fill="${labelColor}" font-size="13" font-weight="bold" font-family="'Fira Code',monospace" text-anchor="${anchor}" dominant-baseline="middle">${escSvg(formatParticleLabel(pn.label))}</text>`
      );
    } else {
      // Internal vertex: subtle marker (de-emphasized)
      nodeSvg.push(
        `<circle class="node-anchor" data-node-id="${pn.id}" cx="${pn.x}" cy="${pn.y}" r="1.8" fill="${colors.vertex}" opacity="0.6" stroke="none" />`
      );
    }
  }

  // Channel label - improved visibility with background and smart positioning
  const channelLabel = diag.channels.length > 0
    ? diag.channels.join("+") + "-channel"
    : "";
  const loopLabel = loopOrderLabel(diag.loop_order);
  const fullLabel = channelLabel ? `${channelLabel} | ${loopLabel}` : loopLabel;
  
  // Smart positioning: top-right normally, top-left if would overflow
  const labelBoxWidth = Math.min(170, width - 2 * padding - 20);
  const labelBoxMaxX = width - padding - 8;
  const labelBoxDefaultX = width - labelBoxWidth - padding + 4;
  const labelBoxX = labelBoxDefaultX + labelBoxWidth > labelBoxMaxX 
    ? padding - 4  // Use top-left if would overflow right
    : labelBoxDefaultX;
  const textAnchor = labelBoxX < padding + 40 ? "start" : "start"; // Always left-align text
  const textStartX = labelBoxX + 8;
  
  const channelSvg = fullLabel
    ? `<g class="feynman-metadata">
        <rect x="${labelBoxX}" y="6" width="${labelBoxWidth}" height="24" fill="rgba(22, 36, 71, 0.92)" stroke="${colors.momentum}" stroke-width="1.5" rx="3" opacity="0.97" />
        <text x="${textStartX}" y="22" fill="${colors.momentum}" font-size="12" font-weight="600" font-family="'Fira Code',monospace" text-anchor="${textAnchor}" dominant-baseline="middle" letter-spacing="0.4px">${escSvg(fullLabel)}</text>
      </g>`
    : "";

  // Assemble SVG with improved rendering order and styling
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" style="background:${colors.background}" role="img" aria-label="Feynman diagram">
  <defs>
    ${markers.join("\n    ")}
    <style>
      .feynman-edges { mix-blend-mode: darken; }
      .feynman-labels { mix-blend-mode: normal; }
      .feynman-nodes { mix-blend-mode: lighten; }
      .feynman-metadata { mix-blend-mode: normal; filter: drop-shadow(0 1px 2px rgba(0,0,0,0.3)); }
    </style>
  </defs>
  <g class="feynman-edges">
    ${edgeSvg.join("\n    ")}
  </g>
  <g class="feynman-labels">
    ${labelSvg.join("\n    ")}
  </g>
  <g class="feynman-nodes">
    ${nodeSvg.join("\n    ")}
  </g>
  ${channelSvg}
</svg>`;
}

function loopOrderLabel(lo: FeynmanDiagram["loop_order"]): string {
  if (lo === "Tree") return "Tree";
  if (lo === "OneLoop") return "1-Loop";
  if (lo === "TwoLoop") return "2-Loop";
  if (typeof lo === "object" && "NLoop" in lo) return `${lo.NLoop}-Loop`;
  return String(lo);
}

function escSvg(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
