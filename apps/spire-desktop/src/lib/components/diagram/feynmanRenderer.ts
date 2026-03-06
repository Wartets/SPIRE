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

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export interface FeynmanRenderOptions {
  width: number;
  height: number;
  padding: number;
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

function classifyLineStyle(edge: FeynmanEdge): LineStyle {
  const form = edge.propagator?.form;
  const spin = edge.field.quantum_numbers?.spin ?? edge.propagator?.spin ?? 0;
  const mass = edge.field.mass ?? edge.propagator?.mass ?? 0;

  // Propagator form takes priority
  if (form) {
    switch (form) {
      case "DiracFermion": return "fermion";
      case "MasslessVector": return "photon";
      case "MassiveVector": return "massive_vector";
      case "Scalar": return mass > 100 ? "higgs" : "scalar";
      case "RaritaSchwinger": return "fermion";
      case "MasslessSpin2":
      case "MassiveSpin2": return "graviton";
      default: break;
    }
  }

  // Fall back to spin
  if (spin === 1) return "fermion"; // spin-½ (twice_spin = 1)
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
}

function layoutDiagram(
  diag: FeynmanDiagram,
  width: number,
  height: number,
  padding: number,
): { nodes: PositionedNode[]; edges: PositionedEdge[] } {
  const incoming: FeynmanNode[] = [];
  const outgoing: FeynmanNode[] = [];
  const internal: FeynmanNode[] = [];

  for (const node of diag.nodes) {
    if ("ExternalIncoming" in node.kind) incoming.push(node);
    else if ("ExternalOutgoing" in node.kind) outgoing.push(node);
    else internal.push(node);
  }

  const innerW = width - 2 * padding;
  const innerH = height - 2 * padding;

  // Position map
  const posMap = new Map<number, { x: number; y: number }>();

  // Incoming: left column
  const inSpacing = incoming.length > 1 ? innerH / (incoming.length - 1) : 0;
  const inOffsetY = incoming.length > 1 ? 0 : innerH / 2;
  for (let i = 0; i < incoming.length; i++) {
    posMap.set(incoming[i].id, {
      x: padding,
      y: padding + inOffsetY + i * inSpacing,
    });
  }

  // Outgoing: right column
  const outSpacing = outgoing.length > 1 ? innerH / (outgoing.length - 1) : 0;
  const outOffsetY = outgoing.length > 1 ? 0 : innerH / 2;
  for (let i = 0; i < outgoing.length; i++) {
    posMap.set(outgoing[i].id, {
      x: padding + innerW,
      y: padding + outOffsetY + i * outSpacing,
    });
  }

  // Internal vertices: spread horizontally between incoming and outgoing
  if (internal.length === 1) {
    posMap.set(internal[0].id, {
      x: padding + innerW / 2,
      y: padding + innerH / 2,
    });
  } else if (internal.length === 2) {
    // Classic s-channel: two vertices at 1/3 and 2/3
    posMap.set(internal[0].id, {
      x: padding + innerW * 0.33,
      y: padding + innerH / 2,
    });
    posMap.set(internal[1].id, {
      x: padding + innerW * 0.67,
      y: padding + innerH / 2,
    });
  } else {
    // General case: arrange in a circle in the center
    const cx = padding + innerW / 2;
    const cy = padding + innerH / 2;
    const r = Math.min(innerW, innerH) * 0.2;
    for (let i = 0; i < internal.length; i++) {
      const angle = (2 * Math.PI * i) / internal.length - Math.PI / 2;
      posMap.set(internal[i].id, {
        x: cx + r * Math.cos(angle),
        y: cy + r * Math.sin(angle),
      });
    }
  }

  // Use explicit positions if available
  for (const node of diag.nodes) {
    if (node.position) {
      // Positions are in abstract coordinates — map to SVG space
      // Only use if ALL nodes have positions
      // (skip for now — automatic layout is more reliable)
    }
  }

  // Build positioned nodes
  const pNodes: PositionedNode[] = diag.nodes.map((n) => {
    const pos = posMap.get(n.id) ?? { x: width / 2, y: height / 2 };
    let label = "";
    if ("ExternalIncoming" in n.kind) label = n.kind.ExternalIncoming.field.symbol;
    else if ("ExternalOutgoing" in n.kind) label = n.kind.ExternalOutgoing.field.symbol;
    return { id: n.id, x: pos.x, y: pos.y, kind: n.kind, label };
  });

  // Build positioned edges
  const pEdges: PositionedEdge[] = diag.edges.map(([src, tgt, edge]) => {
    const srcPos = posMap.get(src) ?? { x: 0, y: 0 };
    const tgtPos = posMap.get(tgt) ?? { x: 0, y: 0 };
    return {
      srcId: src,
      tgtId: tgt,
      srcX: srcPos.x,
      srcY: srcPos.y,
      tgtX: tgtPos.x,
      tgtY: tgtPos.y,
      edge,
      style: classifyLineStyle(edge),
    };
  });

  return { nodes: pNodes, edges: pEdges };
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

// ---------------------------------------------------------------------------
// Main Render Function
// ---------------------------------------------------------------------------

export function renderFeynmanDiagram(
  diag: FeynmanDiagram,
  opts?: Partial<FeynmanRenderOptions>,
): string {
  const o: FeynmanRenderOptions = { ...DEFAULT_OPTIONS, ...opts };
  const { width, height, padding, colors } = o;

  const { nodes, edges } = layoutDiagram(diag, width, height, padding);

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

  for (const pe of edges) {
    const { srcX, srcY, tgtX, tgtY, edge, style } = pe;

    // Shorten lines so they don't overlap vertex dots
    const dx = tgtX - srcX;
    const dy = tgtY - srcY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const shrink = 8; // pixels to pull back from each end
    const ratio = dist > 2 * shrink ? shrink / dist : 0;
    const ax = srcX + dx * ratio;
    const ay = srcY + dy * ratio;
    const bx = tgtX - dx * ratio;
    const by = tgtY - dy * ratio;

    let path: string;
    let strokeColor: string;
    let strokeWidth = 1.8;
    let dashArray = "";
    let markerEnd = "";

    switch (style) {
      case "fermion":
        path = straightPath(ax, ay, bx, by);
        strokeColor = colors.fermion;
        markerEnd = `url(#${ensureMarker("fermion")})`;
        break;
      case "antifermion":
        path = straightPath(bx, by, ax, ay); // reversed
        strokeColor = colors.fermion;
        markerEnd = `url(#${ensureMarker("antifermion")})`;
        break;
      case "photon":
      case "massive_vector":
        path = wavyPath(ax, ay, bx, by, 6 * o.patternScale, 16 * o.patternScale);
        strokeColor = colors.photon;
        break;
      case "gluon":
        path = curlyPath(ax, ay, bx, by, 7 * o.patternScale);
        strokeColor = colors.gluon;
        strokeWidth = 1.6;
        break;
      case "scalar":
        path = dashedPath(ax, ay, bx, by);
        strokeColor = colors.scalar;
        dashArray = "6 4";
        break;
      case "higgs":
        path = dashedPath(ax, ay, bx, by);
        strokeColor = colors.higgs;
        dashArray = "6 4";
        strokeWidth = 2.2;
        break;
      case "graviton":
        path = doubleWavyPath(ax, ay, bx, by, 4 * o.patternScale, 12 * o.patternScale);
        strokeColor = colors.photon;
        strokeWidth = 1.4;
        break;
      default:
        path = straightPath(ax, ay, bx, by);
        strokeColor = colors.fermion;
    }

    edgeSvg.push(
      `<path d="${path}" fill="none" stroke="${strokeColor}" stroke-width="${strokeWidth}"` +
      (dashArray ? ` stroke-dasharray="${dashArray}"` : "") +
      (markerEnd ? ` marker-end="${markerEnd}"` : "") +
      ` stroke-linecap="round" stroke-linejoin="round" />`
    );

    // Momentum label at midpoint
    if (edge.momentum_label) {
      const midX = (srcX + tgtX) / 2;
      const midY = (srcY + tgtY) / 2;
      const ndx = -(tgtY - srcY);
      const ndy = tgtX - srcX;
      const nd = Math.sqrt(ndx * ndx + ndy * ndy) || 1;
      const labelOffset = 14;
      const lx = midX + (ndx / nd) * labelOffset;
      const ly = midY + (ndy / nd) * labelOffset;
      labelSvg.push(
        `<text x="${lx}" y="${ly}" fill="${colors.momentum}" font-size="10" font-family="'Fira Code',monospace" text-anchor="middle" dominant-baseline="middle" font-style="italic">${escSvg(edge.momentum_label)}</text>`
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
        `<circle cx="${pn.x}" cy="${pn.y}" r="4" fill="${labelColor}" stroke="none" />`
      );
      // Label: to the left for incoming, to the right for outgoing
      const labelX = isIncoming ? pn.x - 18 : pn.x + 18;
      const anchor = isIncoming ? "end" : "start";
      nodeSvg.push(
        `<text x="${labelX}" y="${pn.y}" fill="${labelColor}" font-size="13" font-weight="bold" font-family="'Fira Code',monospace" text-anchor="${anchor}" dominant-baseline="middle">${escSvg(pn.label)}</text>`
      );
    } else {
      // Internal vertex: filled dot
      nodeSvg.push(
        `<circle cx="${pn.x}" cy="${pn.y}" r="5" fill="${colors.vertex}" stroke="${colors.vertex}" stroke-width="1.5" />`
      );
    }
  }

  // Channel label
  const channelLabel = diag.channels.length > 0
    ? diag.channels.join("+") + "-channel"
    : "";
  const channelSvg = channelLabel
    ? `<text x="${width - 8}" y="16" fill="${colors.momentum}" font-size="10" font-family="'Fira Code',monospace" text-anchor="end" opacity="0.7">${channelLabel} | ${loopOrderLabel(diag.loop_order)}</text>`
    : "";

  // Assemble SVG
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" style="background:${colors.background}">
  <defs>
    ${markers.join("\n    ")}
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
