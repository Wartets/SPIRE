/**
 * SPIRE - String Theory Worldsheet Renderer
 *
 * Renders Feynman diagrams in a string-theoretic representation where
 * particle worldlines become worldsheets (tubes/ribbons).
 *
 * In string theory, point particles are replaced by one-dimensional
 * strings.  Feynman vertices (where worldlines meet) become smooth
 * surfaces where tubes join and split.  This renderer produces an
 * SVG visualisation of this picture:
 *
 *   • External legs → tubes narrowing to points at the boundary
 *   • Propagators   → tubes (open strings) or bands
 *   • Vertices      → smooth Y-shaped junctions where tubes merge
 *   • Loops         → handles (genus) on the worldsheet
 *
 * The topology of the worldsheet encodes the same information as
 * the Feynman graph but makes the smooth UV behaviour of string
 * amplitudes visually apparent.
 */

import type {
  FeynmanDiagram,
  FeynmanNode,
  NodeKind,
} from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface WorldsheetRenderOptions {
  width: number;
  height: number;
  padding: number;
  tubeWidth: number;
  colors: {
    tube: string;
    tubeStroke: string;
    tubeFill: string;
    label: string;
    background: string;
    highlight: string;
    incomingLabel: string;
    outgoingLabel: string;
  };
}

const DEFAULT_OPTIONS: WorldsheetRenderOptions = {
  width: 600,
  height: 400,
  padding: 70,
  tubeWidth: 22,
  colors: {
    tube: "#5eb8ff",
    tubeStroke: "#3d8ecf",
    tubeFill: "rgba(94, 184, 255, 0.08)",
    label: "#e8e8e8",
    background: "transparent",
    highlight: "#d4a017",
    incomingLabel: "#2ecc71",
    outgoingLabel: "#e74c3c",
  },
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NodePos {
  id: number;
  x: number;
  y: number;
  kind: NodeKind;
  label: string;
}

// ---------------------------------------------------------------------------
// Layout (same as Feynman but with more vertical spacing for tubes)
// ---------------------------------------------------------------------------

function layoutNodes(
  diag: FeynmanDiagram,
  width: number,
  height: number,
  padding: number,
): Map<number, NodePos> {
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

  const posMap = new Map<number, NodePos>();

  // Incoming: left
  const inSpacing = incoming.length > 1 ? innerH / (incoming.length - 1) : 0;
  const inOff = incoming.length > 1 ? 0 : innerH / 2;
  for (let i = 0; i < incoming.length; i++) {
    const n = incoming[i];
    const label = "ExternalIncoming" in n.kind ? n.kind.ExternalIncoming.field.symbol : "";
    posMap.set(n.id, { id: n.id, x: padding, y: padding + inOff + i * inSpacing, kind: n.kind, label });
  }

  // Outgoing: right
  const outSpacing = outgoing.length > 1 ? innerH / (outgoing.length - 1) : 0;
  const outOff = outgoing.length > 1 ? 0 : innerH / 2;
  for (let i = 0; i < outgoing.length; i++) {
    const n = outgoing[i];
    const label = "ExternalOutgoing" in n.kind ? n.kind.ExternalOutgoing.field.symbol : "";
    posMap.set(n.id, { id: n.id, x: padding + innerW, y: padding + outOff + i * outSpacing, kind: n.kind, label });
  }

  // Internal
  if (internal.length === 1) {
    posMap.set(internal[0].id, { id: internal[0].id, x: padding + innerW / 2, y: padding + innerH / 2, kind: internal[0].kind, label: "" });
  } else if (internal.length === 2) {
    posMap.set(internal[0].id, { id: internal[0].id, x: padding + innerW * 0.35, y: padding + innerH / 2, kind: internal[0].kind, label: "" });
    posMap.set(internal[1].id, { id: internal[1].id, x: padding + innerW * 0.65, y: padding + innerH / 2, kind: internal[1].kind, label: "" });
  } else {
    const cx = padding + innerW / 2;
    const cy = padding + innerH / 2;
    const r = Math.min(innerW, innerH) * 0.2;
    for (let i = 0; i < internal.length; i++) {
      const angle = (2 * Math.PI * i) / internal.length - Math.PI / 2;
      posMap.set(internal[i].id, {
        id: internal[i].id, x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle),
        kind: internal[i].kind, label: "",
      });
    }
  }

  return posMap;
}

// ---------------------------------------------------------------------------
// Tube Path Generation
// ---------------------------------------------------------------------------

/**
 * Generate an SVG path for a tube (worldsheet segment) between two points.
 * The tube has parallel edges that follow a smooth cubic Bézier curve.
 */
function tubePath(
  x1: number, y1: number, x2: number, y2: number,
  w: number,
  taperStart: boolean = false,
  taperEnd: boolean = false,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return "";

  // Normal
  const nx = -dy / dist;
  const ny = dx / dist;

  const w1 = taperStart ? w * 0.15 : w;
  const w2 = taperEnd ? w * 0.15 : w;

  // Four corners of the tube
  const topLeft = { x: x1 + nx * w1 / 2, y: y1 + ny * w1 / 2 };
  const botLeft = { x: x1 - nx * w1 / 2, y: y1 - ny * w1 / 2 };
  const topRight = { x: x2 + nx * w2 / 2, y: y2 + ny * w2 / 2 };
  const botRight = { x: x2 - nx * w2 / 2, y: y2 - ny * w2 / 2 };

  // Control points for smooth curvature
  const tension = dist * 0.35;
  const ux = dx / dist;
  const uy = dy / dist;

  return [
    `M ${topLeft.x} ${topLeft.y}`,
    `C ${topLeft.x + ux * tension} ${topLeft.y + uy * tension}, ${topRight.x - ux * tension} ${topRight.y - uy * tension}, ${topRight.x} ${topRight.y}`,
    `L ${botRight.x} ${botRight.y}`,
    `C ${botRight.x - ux * tension} ${botRight.y - uy * tension}, ${botLeft.x + ux * tension} ${botLeft.y + uy * tension}, ${botLeft.x} ${botLeft.y}`,
    `Z`,
  ].join(" ");
}

/**
 * Generate tube edge lines (the visible outline of the worldsheet).
 * Returns two separate path strings for the top and bottom edges.
 */
function tubeEdgeLines(
  x1: number, y1: number, x2: number, y2: number,
  w: number,
  taperStart: boolean = false,
  taperEnd: boolean = false,
): [string, string] {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return ["", ""];

  const nx = -dy / dist;
  const ny = dx / dist;
  const ux = dx / dist;
  const uy = dy / dist;

  const w1 = taperStart ? w * 0.15 : w;
  const w2 = taperEnd ? w * 0.15 : w;
  const tension = dist * 0.35;

  const topLeft = { x: x1 + nx * w1 / 2, y: y1 + ny * w1 / 2 };
  const botLeft = { x: x1 - nx * w1 / 2, y: y1 - ny * w1 / 2 };
  const topRight = { x: x2 + nx * w2 / 2, y: y2 + ny * w2 / 2 };
  const botRight = { x: x2 - nx * w2 / 2, y: y2 - ny * w2 / 2 };

  const top = `M ${topLeft.x} ${topLeft.y} C ${topLeft.x + ux * tension} ${topLeft.y + uy * tension}, ${topRight.x - ux * tension} ${topRight.y - uy * tension}, ${topRight.x} ${topRight.y}`;
  const bot = `M ${botLeft.x} ${botLeft.y} C ${botLeft.x + ux * tension} ${botLeft.y + uy * tension}, ${botRight.x - ux * tension} ${botRight.y - uy * tension}, ${botRight.x} ${botRight.y}`;

  return [top, bot];
}

// ---------------------------------------------------------------------------
// Vertex Junction (smooth merging of tubes)
// ---------------------------------------------------------------------------

function vertexJunction(x: number, y: number, r: number, color: string): string {
  // Draw a smooth gradient circle at the junction point
  return `<circle cx="${x}" cy="${y}" r="${r}" fill="${color}" opacity="0.35" />
    <circle cx="${x}" cy="${y}" r="${r * 0.5}" fill="${color}" opacity="0.2" />`;
}

// ---------------------------------------------------------------------------
// Main Render Function
// ---------------------------------------------------------------------------

export function renderWorldsheet(
  diag: FeynmanDiagram,
  opts?: Partial<WorldsheetRenderOptions>,
): string {
  const o: WorldsheetRenderOptions = { ...DEFAULT_OPTIONS, ...opts };
  const { width, height, padding, tubeWidth, colors } = o;

  const posMap = layoutNodes(diag, width, height, padding);

  const tubeFills: string[] = [];
  const tubeStrokes: string[] = [];
  const labelSvg: string[] = [];

  // Render edges as tubes
  for (const [src, tgt, edge] of diag.edges) {
    const srcNode = posMap.get(src);
    const tgtNode = posMap.get(tgt);
    if (!srcNode || !tgtNode) continue;

    const isExtSrc = "ExternalIncoming" in srcNode.kind || "ExternalOutgoing" in srcNode.kind;
    const isExtTgt = "ExternalIncoming" in tgtNode.kind || "ExternalOutgoing" in tgtNode.kind;

    // Draw tube
    const fillPath = tubePath(srcNode.x, srcNode.y, tgtNode.x, tgtNode.y, tubeWidth, isExtSrc, isExtTgt);
    tubeFills.push(`<path d="${fillPath}" fill="${colors.tubeFill}" stroke="none" />`);

    const [topLine, botLine] = tubeEdgeLines(srcNode.x, srcNode.y, tgtNode.x, tgtNode.y, tubeWidth, isExtSrc, isExtTgt);
    tubeStrokes.push(`<path d="${topLine}" fill="none" stroke="${colors.tubeStroke}" stroke-width="1.5" opacity="0.7" />`);
    tubeStrokes.push(`<path d="${botLine}" fill="none" stroke="${colors.tubeStroke}" stroke-width="1.5" opacity="0.7" />`);

    // Particle label at midpoint
    if (edge.field.symbol) {
      const midX = (srcNode.x + tgtNode.x) / 2;
      const midY = (srcNode.y + tgtNode.y) / 2;
      const dx = tgtNode.x - srcNode.x;
      const dy = tgtNode.y - srcNode.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const nx = -dy / dist;
      const ny = dx / dist;
      const lx = midX + nx * (tubeWidth / 2 + 12);
      const ly = midY + ny * (tubeWidth / 2 + 12);
      labelSvg.push(
        `<text x="${lx}" y="${ly}" fill="${colors.highlight}" font-size="10" font-family="'Fira Code',monospace" text-anchor="middle" dominant-baseline="middle" font-style="italic">${escSvg(edge.field.symbol)}</text>`
      );
    }
  }

  // Render vertex junctions
  const junctionSvg: string[] = [];
  for (const [, node] of posMap) {
    if ("InternalVertex" in node.kind) {
      junctionSvg.push(vertexJunction(node.x, node.y, tubeWidth * 0.8, colors.tube));
    }
  }

  // External particle labels
  const extLabelSvg: string[] = [];
  for (const [, node] of posMap) {
    const isIncoming = "ExternalIncoming" in node.kind;
    const isOutgoing = "ExternalOutgoing" in node.kind;
    if (isIncoming || isOutgoing) {
      const labelX = isIncoming ? node.x - 20 : node.x + 20;
      const anchor = isIncoming ? "end" : "start";
      const color = isIncoming ? colors.incomingLabel : colors.outgoingLabel;
      extLabelSvg.push(
        `<text x="${labelX}" y="${node.y}" fill="${color}" font-size="13" font-weight="bold" font-family="'Fira Code',monospace" text-anchor="${anchor}" dominant-baseline="middle">${escSvg(node.label)}</text>`
      );
    }
  }

  // Title annotation
  const titleSvg = `<text x="${width - 8}" y="16" fill="${colors.label}" font-size="10" font-family="'Fira Code',monospace" text-anchor="end" opacity="0.5">String worldsheet</text>`;

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" style="background:${colors.background}">
  <defs>
    <filter id="ws-glow">
      <feGaussianBlur stdDeviation="3" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
  </defs>
  <g class="worldsheet-tubes" filter="url(#ws-glow)">
    ${tubeFills.join("\n    ")}
  </g>
  <g class="worldsheet-edges">
    ${tubeStrokes.join("\n    ")}
  </g>
  <g class="worldsheet-junctions">
    ${junctionSvg.join("\n    ")}
  </g>
  <g class="worldsheet-labels">
    ${labelSvg.join("\n    ")}
    ${extLabelSvg.join("\n    ")}
  </g>
  ${titleSvg}
</svg>`;
}

function escSvg(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
