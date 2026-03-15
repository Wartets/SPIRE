<script lang="ts">
  import type { ColoredValenceQuark } from "$lib/core/physics/composites";
  import {
    computeQuarkNodeLayout,
    buildConfinementHullPath,
    generateGluonPath,
    type SvgPoint,
  } from "$lib/core/physics/svg_schematics";

  export let valence: ColoredValenceQuark[] = [];
  export let label = "Composite state";

  const W = 320;
  const H = 250;
  const CX = 160;
  const CY = 120;

  interface LayoutNode {
    q: ColoredValenceQuark;
    point: SvgPoint;
  }

  interface EdgePair {
    a: number;
    b: number;
    path: string;
  }

  function buildPairs(nodes: LayoutNode[]): EdgePair[] {
    const n = nodes.length;
    if (n < 2) return [];

    const edges = new Set<string>();
    const pairs: [number, number][] = [];

    const push = (a: number, b: number) => {
      const i = Math.min(a, b);
      const j = Math.max(a, b);
      const key = `${i}_${j}`;
      if (i === j || edges.has(key)) return;
      edges.add(key);
      pairs.push([i, j]);
    };

    for (let i = 0; i < n; i += 1) {
      push(i, (i + 1) % n);
    }

    if (n <= 4) {
      for (let i = 0; i < n; i += 1) {
        for (let j = i + 1; j < n; j += 1) {
          push(i, j);
        }
      }
    }

    return pairs.map(([a, b]) => {
      const p1 = nodes[a].point;
      const p2 = nodes[b].point;
      return {
        a,
        b,
        path: generateGluonPath(p1.x, p1.y, p2.x, p2.y),
      };
    });
  }

  function colorClass(color: ColoredValenceQuark["color"]): string {
    switch (color) {
      case "red":
      case "anti-red":
        return "q-red";
      case "green":
      case "anti-green":
        return "q-green";
      default:
        return "q-blue";
    }
  }

  $: nodeRadius = valence.length <= 2 ? 56 : valence.length === 3 ? 64 : 72;
  $: points = computeQuarkNodeLayout(valence.length, CX, CY, nodeRadius);
  $: nodes = valence.map((q, i) => ({ q, point: points[i] })) as LayoutNode[];
  $: confinementPath = buildConfinementHullPath(points, 28);
  $: gluonEdges = buildPairs(nodes);
</script>

<div class="composition-wrap">
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={label}>
    {#if confinementPath}
      <path class="confinement" d={confinementPath} />
    {/if}

    {#each gluonEdges as edge (`${edge.a}-${edge.b}`)}
      <path class="gluon" d={edge.path} />
    {/each}

    {#each nodes as node, idx (`${node.q.flavor}-${idx}`)}
      <g transform={`translate(${node.point.x}, ${node.point.y})`}>
        <circle class={`quark ${colorClass(node.q.color)}`} r="18" />
        <text class="symbol" x="0" y="0">{node.q.symbol}</text>
      </g>
    {/each}

    <text class="caption" x="160" y="232">{label}</text>
  </svg>
</div>

<style>
  .composition-wrap {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    min-height: 250px;
  }

  svg {
    width: 100%;
    height: 250px;
    display: block;
  }

  .confinement {
    fill: rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.05);
    stroke: var(--color-border, var(--border));
    stroke-width: 1.3;
    stroke-dasharray: 4 4;
  }

  .gluon {
    fill: none;
    stroke: rgba(var(--color-accent-rgb, 94, 184, 255), 0.9);
    stroke-width: 1.2;
    stroke-linecap: round;
  }

  .quark {
    stroke: var(--color-border, var(--border));
    stroke-width: 1.1;
  }

  .q-red {
    fill: color-mix(in srgb, #ff5f56 72%, var(--color-bg-surface, var(--bg-surface)));
  }

  .q-green {
    fill: color-mix(in srgb, #34c759 70%, var(--color-bg-surface, var(--bg-surface)));
  }

  .q-blue {
    fill: color-mix(in srgb, #0a84ff 72%, var(--color-bg-surface, var(--bg-surface)));
  }

  .symbol {
    font-family: var(--font-mono);
    fill: var(--color-text-primary, var(--fg-primary));
    font-size: 12px;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: central;
    user-select: none;
  }

  .caption {
    font-family: var(--font-mono);
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 11px;
    text-anchor: middle;
  }
</style>
