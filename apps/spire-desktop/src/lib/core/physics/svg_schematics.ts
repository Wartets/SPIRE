export interface SvgPoint {
  x: number;
  y: number;
}

export interface GluonPathOptions {
  turns?: number;
  amplitude?: number;
  segmentsPerTurn?: number;
}

export function computeQuarkNodeLayout(
  count: number,
  cx: number,
  cy: number,
  radius: number,
): SvgPoint[] {
  if (count <= 0) return [];
  if (count === 1) return [{ x: cx, y: cy }];

  const nodes: SvgPoint[] = [];
  const phase = -Math.PI / 2;
  for (let i = 0; i < count; i += 1) {
    const t = phase + (2 * Math.PI * i) / count;
    nodes.push({
      x: cx + radius * Math.cos(t),
      y: cy + radius * Math.sin(t),
    });
  }
  return nodes;
}

function centroid(points: SvgPoint[]): SvgPoint {
  if (points.length === 0) return { x: 0, y: 0 };
  const sum = points.reduce(
    (acc, p) => ({ x: acc.x + p.x, y: acc.y + p.y }),
    { x: 0, y: 0 },
  );
  return {
    x: sum.x / points.length,
    y: sum.y / points.length,
  };
}

function normalize(dx: number, dy: number): SvgPoint {
  const m = Math.hypot(dx, dy) || 1;
  return { x: dx / m, y: dy / m };
}

export function buildConfinementHullPath(nodes: SvgPoint[], padding = 24): string {
  if (nodes.length === 0) return "";

  if (nodes.length === 1) {
    const p = nodes[0];
    const r = Math.max(12, padding);
    return [
      `M ${p.x + r} ${p.y}`,
      `A ${r} ${r} 0 1 0 ${p.x - r} ${p.y}`,
      `A ${r} ${r} 0 1 0 ${p.x + r} ${p.y}`,
      "Z",
    ].join(" ");
  }

  const c = centroid(nodes);
  const inflated = nodes.map((p) => {
    const n = normalize(p.x - c.x, p.y - c.y);
    return { x: p.x + n.x * padding, y: p.y + n.y * padding };
  });

  const mids = inflated.map((p, i) => {
    const next = inflated[(i + 1) % inflated.length];
    return { x: 0.5 * (p.x + next.x), y: 0.5 * (p.y + next.y) };
  });

  const start = mids[0];
  const commands = [`M ${start.x} ${start.y}`];
  for (let i = 0; i < inflated.length; i += 1) {
    const ctrl = inflated[(i + 1) % inflated.length];
    const end = mids[(i + 1) % mids.length];
    commands.push(`Q ${ctrl.x} ${ctrl.y} ${end.x} ${end.y}`);
  }
  commands.push("Z");
  return commands.join(" ");
}

export function generateGluonPath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  opts: GluonPathOptions = {},
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const distance = Math.hypot(dx, dy);
  if (distance < 1e-6) {
    return `M ${x1} ${y1}`;
  }

  const turns = opts.turns ?? Math.max(5, Math.round(distance / 22));
  const amplitude = opts.amplitude ?? Math.min(9, Math.max(4, distance / 24));
  const segmentsPerTurn = opts.segmentsPerTurn ?? 10;
  const samples = Math.max(12, turns * segmentsPerTurn);

  const tx = dx / distance;
  const ty = dy / distance;
  const nx = -ty;
  const ny = tx;

  const commands: string[] = [];
  for (let i = 0; i <= samples; i += 1) {
    const t = i / samples;
    const baseX = x1 + dx * t;
    const baseY = y1 + dy * t;
    const envelope = Math.pow(Math.sin(Math.PI * t), 0.85);
    const offset = amplitude * envelope * Math.sin(2 * Math.PI * turns * t);
    const px = baseX + nx * offset;
    const py = baseY + ny * offset;
    if (i === 0) {
      commands.push(`M ${px} ${py}`);
    } else {
      commands.push(`L ${px} ${py}`);
    }
  }
  return commands.join(" ");
}
