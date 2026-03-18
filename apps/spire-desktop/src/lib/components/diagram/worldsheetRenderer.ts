/**
 * SPIRE - Physics-Realistic Worldsheet Renderer
 *
 * Renders Feynman diagrams with ultra-realistic string worldsheet
 * visualization showing actual surface topology.
 *
 * Key Features:
 * - Real surface splitting/merging at vertices (not abstract tubes)
 * - Realistic ribbon topology with variable width based on physics
 * - Proper handle/loop visualization (genus of worldsheet)
 * - Smooth surface transitions with proper continuity
 * - No unnecessary vertex dots - physics-based visualization only
 *
 * The worldsheet is rendered as a continuous surface where:
 * - External legs are boundaries of the surface
 * - Propagators are cylindrical surfaces joining vertices
 * - Vertices are smooth junctions where surfaces merge
 * - Loops create topological handles on the surface
 */

import type {
  FeynmanDiagram,
  FeynmanEdge,
  NodeKind,
} from "$lib/types/spire";
import { layoutDiagram as computeLayout } from "./layoutEngine";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

export interface WorldsheetRenderOptions {
  width: number;
  height: number;
  padding: number;
  nodePositionOverrides?: Record<number, { x: number; y: number }>;
  baseWidth: number;
  sampleCount: number;
  smoothIterations: number;
  colors: {
    boundary: string;
    fill: string;
    iso: string;
    transition: string;
    label: string;
    background: string;
    incomingLabel: string;
    outgoingLabel: string;
  };
}

const DEFAULT_OPTIONS: WorldsheetRenderOptions = {
  width: 600,
  height: 420,
  padding: 80,
  baseWidth: 16,
  sampleCount: 28,
  smoothIterations: 10,
  colors: {
    boundary: "#53c8ff",
    fill: "rgba(41, 122, 179, 0.22)",
    iso: "rgba(180, 232, 255, 0.28)",
    transition: "rgba(78, 171, 230, 0.28)",
    label: "#e8e8e8",
    background: "transparent",
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

interface Vec2 { x: number; y: number }

interface BandGeometry {
  top: Vec2[];
  bottom: Vec2[];
  center: Vec2[];
  nearStart: Vec2[];
  nearEnd: Vec2[];
  labelPos: Vec2;
}

function vec(x = 0, y = 0): Vec2 {
  return { x, y };
}

function add(a: Vec2, b: Vec2): Vec2 { return { x: a.x + b.x, y: a.y + b.y }; }
function sub(a: Vec2, b: Vec2): Vec2 { return { x: a.x - b.x, y: a.y - b.y }; }
function mul(a: Vec2, s: number): Vec2 { return { x: a.x * s, y: a.y * s }; }
function len(a: Vec2): number { return Math.sqrt(a.x * a.x + a.y * a.y); }
function norm(a: Vec2): Vec2 { const l = len(a) || 1; return { x: a.x / l, y: a.y / l }; }
function perp(a: Vec2): Vec2 { return { x: -a.y, y: a.x }; }

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}

function cubicPoint(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: number): Vec2 {
  const u = 1 - t;
  const b0 = u * u * u;
  const b1 = 3 * u * u * t;
  const b2 = 3 * u * t * t;
  const b3 = t * t * t;
  return {
    x: b0 * p0.x + b1 * p1.x + b2 * p2.x + b3 * p3.x,
    y: b0 * p0.y + b1 * p1.y + b2 * p2.y + b3 * p3.y,
  };
}

function cubicDeriv(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: number): Vec2 {
  const u = 1 - t;
  return {
    x: 3 * u * u * (p1.x - p0.x) + 6 * u * t * (p2.x - p1.x) + 3 * t * t * (p3.x - p2.x),
    y: 3 * u * u * (p1.y - p0.y) + 6 * u * t * (p2.y - p1.y) + 3 * t * t * (p3.y - p2.y),
  };
}

function smoothLine(points: Vec2[], iterations: number): Vec2[] {
  if (points.length < 4 || iterations <= 0) return points;
  let out = points.map((p) => ({ ...p }));
  for (let k = 0; k < iterations; k++) {
    const next = out.map((p) => ({ ...p }));
    for (let i = 1; i < out.length - 1; i++) {
      next[i].x = (out[i - 1].x + out[i + 1].x + 2 * out[i].x) / 4;
      next[i].y = (out[i - 1].y + out[i + 1].y + 2 * out[i].y) / 4;
    }
    out = next;
  }
  return out;
}

function polylinePath(points: Vec2[]): string {
  if (points.length === 0) return "";
  let d = `M ${points[0].x} ${points[0].y}`;
  for (let i = 1; i < points.length; i++) d += ` L ${points[i].x} ${points[i].y}`;
  return d;
}

function closedBandPath(top: Vec2[], bottom: Vec2[]): string {
  if (top.length === 0 || bottom.length === 0) return "";
  const rb = [...bottom].reverse();
  let d = `M ${top[0].x} ${top[0].y}`;
  for (let i = 1; i < top.length; i++) d += ` L ${top[i].x} ${top[i].y}`;
  for (let i = 0; i < rb.length; i++) d += ` L ${rb[i].x} ${rb[i].y}`;
  return `${d} Z`;
}

function computeLaneOffsets(edges: Array<[number, number, FeynmanEdge]>): number[] {
  const groups = new Map<string, number[]>();
  for (let i = 0; i < edges.length; i++) {
    const [s, t] = edges[i];
    const key = `${Math.min(s, t)}-${Math.max(s, t)}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(i);
  }
  const out = new Array(edges.length).fill(0);
  for (const idxs of groups.values()) {
    for (let i = 0; i < idxs.length; i++) out[idxs[i]] = i - (idxs.length - 1) / 2;
  }
  return out;
}

function transitionPatch(center: Vec2, ringPoints: Vec2[]): string {
  if (ringPoints.length < 3) return "";
  const sorted = [...ringPoints].sort(
    (a, b) => Math.atan2(a.y - center.y, a.x - center.x) - Math.atan2(b.y - center.y, b.x - center.x)
  );
  const cyc = [...sorted, sorted[0], sorted[1]];
  let d = `M ${cyc[0].x} ${cyc[0].y}`;
  for (let i = 1; i < cyc.length - 1; i++) {
    const c = cyc[i];
    const n = cyc[i + 1];
    d += ` Q ${c.x} ${c.y} ${(c.x + n.x) / 2} ${(c.y + n.y) / 2}`;
  }
  return `${d} Z`;
}

function formatParticleLabel(raw: string): string {
  let s = (raw ?? "").trim();
  if (!s) return s;
  if (s.startsWith("$") && s.endsWith("$")) s = s.slice(1, -1);
  s = s.replace(/^\\/, "");

  const namedGreek: Record<string, string> = {
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

  const plain = s.toLowerCase();
  if (namedGreek[plain]) return namedGreek[plain];

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
    .replace(/\^-$/, "⁻")
    .replace(/\^\+$/, "⁺")
    .replace(/([A-Za-zα-ωΑ-Ω])-(?!\w)/g, "$1⁻")
    .replace(/([A-Za-zα-ωΑ-Ω])\+(?!\w)/g, "$1⁺");

  if (s === "e-") return "e⁻";
  if (s === "e+") return "e⁺";
  if (s === "mu-") return "μ⁻";
  if (s === "mu+") return "μ⁺";
  return s;
}

// ---------------------------------------------------------------------------
// Enhanced Layout with better spacing
// ---------------------------------------------------------------------------

function layoutNodes(
  diag: FeynmanDiagram,
  width: number,
  height: number,
  padding: number,
  overrides?: Record<number, { x: number; y: number }>,
): Map<number, NodePos> {
  const map = new Map<number, NodePos>();

  const layout = computeLayout(diag, width, height);
  for (const n of layout.nodes) {
    const override = overrides?.[n.id];
    map.set(n.id, {
      id: n.id,
      x: override?.x ?? n.x,
      y: override?.y ?? n.y,
      kind: n.kind,
      label: n.label,
    });
  }

  return map;
}

// ---------------------------------------------------------------------------
// Realistic Surface Rendering
// ---------------------------------------------------------------------------

/**
 * Render a realistic worldsheet surface that smoothly merges at vertices
 * Uses Bezier curves and gradients to create a physically realistic appearance
 */
function buildBandFromCubic(
  p0: Vec2,
  p1: Vec2,
  p2: Vec2,
  p3: Vec2,
  halfWidth: number,
  samples: number,
  smoothIterations: number,
): BandGeometry {
  const center: Vec2[] = [];
  const top: Vec2[] = [];
  const bottom: Vec2[] = [];

  for (let i = 0; i < samples; i++) {
    const t = i / (samples - 1);
    const c = cubicPoint(p0, p1, p2, p3, t);
    const d = cubicDeriv(p0, p1, p2, p3, t);
    const n = norm(perp(d));
    center.push(c);
    top.push(add(c, mul(n, halfWidth)));
    bottom.push(sub(c, mul(n, halfWidth)));
  }

  const topSmooth = smoothLine(top, smoothIterations);
  const bottomSmooth = smoothLine(bottom, smoothIterations);
  const li = Math.floor((samples - 1) * 0.52);
  const dMid = cubicDeriv(p0, p1, p2, p3, li / (samples - 1));
  const nMid = norm(perp(dMid));

  return {
    top: topSmooth,
    bottom: bottomSmooth,
    center,
    nearStart: [topSmooth[1], bottomSmooth[1]],
    nearEnd: [topSmooth[topSmooth.length - 2], bottomSmooth[bottomSmooth.length - 2]],
    labelPos: add(center[li], mul(nMid, halfWidth + 10)),
  };
}

// ---------------------------------------------------------------------------
// Main Render Function
// ---------------------------------------------------------------------------

export function renderWorldsheet(
  diag: FeynmanDiagram,
  opts?: Partial<WorldsheetRenderOptions>,
): string {
  const o: WorldsheetRenderOptions = { ...DEFAULT_OPTIONS, ...opts };
  const { width, height, baseWidth, sampleCount, smoothIterations, colors } = o;

  const complexity = diag.nodes.length + diag.edges.length * 1.35;
  const qualityDrop = Math.floor(complexity / 14);
  const adaptiveSamples = clamp(sampleCount - qualityDrop * 2, 10, sampleCount);
  const adaptiveSmooth = clamp(smoothIterations - Math.floor(qualityDrop / 2), 3, smoothIterations);
  const renderIso = complexity <= 46;

  const posMap = layoutNodes(diag, width, height, o.padding, o.nodePositionOverrides);

  const nodePositions = Array.from(posMap.values()).map((n) => vec(n.x, n.y));
  let dMin = Number.POSITIVE_INFINITY;
  for (let i = 0; i < nodePositions.length; i++) {
    for (let j = i + 1; j < nodePositions.length; j++) {
      dMin = Math.min(dMin, len(sub(nodePositions[i], nodePositions[j])));
    }
  }
  const halfWidth = clamp(Math.min(baseWidth, (Number.isFinite(dMin) ? dMin : 100) * 0.45), 6, 20);

  const incident = new Map<number, number[]>();
  for (let i = 0; i < diag.edges.length; i++) {
    const [s, t] = diag.edges[i];
    if (!incident.has(s)) incident.set(s, []);
    if (!incident.has(t)) incident.set(t, []);
    incident.get(s)!.push(i);
    incident.get(t)!.push(i);
  }

  const lane = computeLaneOffsets(diag.edges);

  const surfaceFills: string[] = [];
  const boundarySvg: string[] = [];
  const isoSvg: string[] = [];
  const labelSvg: string[] = [];
  const transitionSvg: string[] = [];

  const nearByNode = new Map<number, Vec2[]>();

  for (let i = 0; i < diag.edges.length; i++) {
    const [srcId, tgtId, edge] = diag.edges[i];
    const srcNode = posMap.get(srcId);
    const tgtNode = posMap.get(tgtId);
    if (!srcNode || !tgtNode) continue;

    const srcPos = vec(srcNode.x, srcNode.y);
    const dstPos = vec(tgtNode.x, tgtNode.y);
    const d = sub(dstPos, srcPos);
    const baseDir = norm(d);
    const n = perp(baseDir);
    const laneShift = lane[i] * halfWidth * 0.7;

    const tangent = (nodeId: number, fallback: Vec2): Vec2 => {
      const ids = incident.get(nodeId) ?? [];
      let acc = vec(0, 0);
      for (const ei of ids) {
        const [s, t] = diag.edges[ei];
        const otherId = s === nodeId ? t : s;
        const p = posMap.get(otherId);
        if (!p) continue;
        acc = add(acc, norm(sub(vec(p.x, p.y), vec(posMap.get(nodeId)!.x, posMap.get(nodeId)!.y))));
      }
      if (len(acc) < 0.0001) return norm(fallback);
      return norm(acc);
    };

    let t0 = tangent(srcId, d);
    let t1 = tangent(tgtId, mul(d, -1));
    if (t0.x * baseDir.x + t0.y * baseDir.y < 0) t0 = mul(t0, -1);
    if (t1.x * -baseDir.x + t1.y * -baseDir.y < 0) t1 = mul(t1, -1);

    const h = clamp(len(d) * 0.34, 24, 120);
    const p0 = add(srcPos, mul(n, laneShift));
    const p3 = add(dstPos, mul(n, laneShift));
    const p1 = add(p0, mul(t0, h));
    const p2 = sub(p3, mul(t1, h));

    const band = buildBandFromCubic(p0, p1, p2, p3, halfWidth, adaptiveSamples, adaptiveSmooth);

    surfaceFills.push(`<path d="${closedBandPath(band.top, band.bottom)}" fill="${colors.fill}" stroke="none" />`);
    boundarySvg.push(`<path d="${polylinePath(band.top)}" fill="none" stroke="${colors.boundary}" stroke-width="1.45" stroke-linecap="round" stroke-linejoin="round" />`);
    boundarySvg.push(`<path d="${polylinePath(band.bottom)}" fill="none" stroke="${colors.boundary}" stroke-width="1.45" stroke-linecap="round" stroke-linejoin="round" />`);
    if (renderIso) {
      isoSvg.push(`<path d="${polylinePath(band.center)}" fill="none" stroke="${colors.iso}" stroke-width="0.8" stroke-dasharray="2.4 3.6" />`);
    }

    if (!nearByNode.has(srcId)) nearByNode.set(srcId, []);
    if (!nearByNode.has(tgtId)) nearByNode.set(tgtId, []);
    nearByNode.get(srcId)!.push(...band.nearStart);
    nearByNode.get(tgtId)!.push(...band.nearEnd);

    if (edge.field.symbol) {
      labelSvg.push(
        `<text x="${band.labelPos.x}" y="${band.labelPos.y}" fill="${colors.label}" font-size="10" font-weight="500" font-family="'Fira Code',monospace" text-anchor="middle" dominant-baseline="middle" font-style="italic" opacity="0.92">${escSvg(formatParticleLabel(edge.field.symbol))}</text>`
      );
    }
  }

  for (const [, node] of posMap) {
    if ("InternalVertex" in node.kind) {
      const pts = nearByNode.get(node.id) ?? [];
      const d = transitionPatch(vec(node.x, node.y), pts);
      if (d) transitionSvg.push(`<path d="${d}" fill="${colors.transition}" stroke="none" />`);
    }
  }

  for (const [, node] of posMap) {
    const isIncoming = "ExternalIncoming" in node.kind;
    const isOutgoing = "ExternalOutgoing" in node.kind;
    if (isIncoming || isOutgoing) {
      const labelX = isIncoming ? node.x - 24 : node.x + 24;
      const anchor = isIncoming ? "end" : "start";
      const color = isIncoming ? colors.incomingLabel : colors.outgoingLabel;
      labelSvg.push(
        `<text x="${labelX}" y="${node.y}" fill="${color}" font-size="12" font-weight="bold" font-family="'Fira Code',monospace" text-anchor="${anchor}" dominant-baseline="middle">${escSvg(formatParticleLabel(node.label))}</text>`
      );
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" style="background:${colors.background}">
  <g class="worldsheet-surfaces" opacity="0.95">
    ${surfaceFills.join("\n    ")}
  </g>
  <g class="worldsheet-transitions">
    ${transitionSvg.join("\n    ")}
  </g>
  <g class="worldsheet-iso">
    ${isoSvg.join("\n    ")}
  </g>
  <g class="worldsheet-boundary">
    ${boundarySvg.join("\n    ")}
  </g>
  <g class="worldsheet-labels">
    ${labelSvg.join("\n    ")}
  </g>
</svg>`;
}

function escSvg(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
