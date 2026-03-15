<script lang="ts">
  import type { ColoredValenceQuark } from "$lib/core/physics/composites";
  import {
    computeQuarkNodeLayout,
    buildConfinementHullPath,
    generateGluonPath,
    type SvgPoint,
  } from "$lib/core/physics/svg_schematics";

  /** Rendering mode: quark-bag diagram or nuclear liquid-drop model. */
  export let mode: "quark" | "nucleus" = "quark";

  /** Valence quarks for quark-bag mode. */
  export let valence: ColoredValenceQuark[] = [];
  export let label = "Composite state";

  /** Number of protons for nucleus mode. */
  export let protons = 0;
  /** Number of neutrons for nucleus mode. */
  export let neutrons = 0;

  const W = 320;
  const H = 250;
  const CX = 160;
  const CY = 115;

  // ---------------------------------------------------------------------------
  // Quark-bag geometry
  // ---------------------------------------------------------------------------

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

  // ---------------------------------------------------------------------------
  // Nuclear droplet geometry
  // ---------------------------------------------------------------------------

  interface NucleonDot {
    cx: number;
    cy: number;
    isProton: boolean;
  }

  /** Deterministic jitter for nucleon positions (no random, reproducible). */
  function hashSeed(seed: number): number {
    let h = seed ^ 0x9e3779b9;
    h = ((h >> 16) ^ h) * 0x45d9f3b;
    h = ((h >> 16) ^ h) * 0x45d9f3b;
    return ((h >> 16) ^ h) >>> 0;
  }

  function buildNucleonLayout(Z: number, N: number): NucleonDot[] {
    const total = Z + N;
    if (total === 0) return [];

    const RX = 76;
    const RY = 64;
    const dotR = total <= 4 ? 14 : total <= 16 ? 10 : total <= 40 ? 7 : 5;
    const spacingFactor = dotR * 2.2;

    const dots: NucleonDot[] = [];

    // Interleave proton/neutron ordering for visual balance
    const labels = Array.from({ length: Z }, () => true).concat(
      Array.from({ length: N }, () => false),
    );
    // shuffle deterministically
    for (let i = labels.length - 1; i > 0; i--) {
      const j = hashSeed(i + total * 31) % (i + 1);
      [labels[i], labels[j]] = [labels[j], labels[i]];
    }

    // Pack into concentric rings
    const rings: number[] = total <= 4 ? [total] : total <= 20 ? [2, total - 2] : [4, 8, total - 12 > 0 ? Math.min(12, total - 12) : 0, Math.max(0, total - 24)].filter((r) => r > 0);

    let idx = 0;
    rings.forEach((count, ringIdx) => {
      const r = (ringIdx + 1) * spacingFactor;
      const rScale = Math.min(1.0, RX / r);
      const rx = r * rScale;
      const ry = r * (RY / RX) * rScale;
      for (let k = 0; k < count; k++) {
        const angle = (k / count) * 2 * Math.PI + (ringIdx * 0.4);
        const jitterH = hashSeed(idx * 7 + k * 13);
        const jx = ((jitterH % 100) / 100 - 0.5) * dotR * 0.6;
        const jy = ((hashSeed(idx * 11 + k * 5) % 100) / 100 - 0.5) * dotR * 0.6;
        dots.push({
          cx: CX + rx * Math.cos(angle) + jx,
          cy: CY + ry * Math.sin(angle) + jy,
          isProton: labels[idx] ?? false,
        });
        idx++;
        if (idx >= labels.length) break;
      }
    });

    return dots;
  }

  $: nucleons = mode === "nucleus" ? buildNucleonLayout(protons, neutrons) : [];
  $: nuclearLabel = mode === "nucleus" ? `${protons}p  ${neutrons}n  (A=${protons + neutrons})` : label;
</script>

<div class="composition-wrap">
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={mode === "nucleus" ? nuclearLabel : label}>

    {#if mode === "quark"}
      <!-- Quark-bag confinement hull -->
      {#if confinementPath}
        <path class="confinement" d={confinementPath} />
      {/if}

      <!-- Gluon lines -->
      {#each gluonEdges as edge (`${edge.a}-${edge.b}`)}
        <path class="gluon" d={edge.path} />
      {/each}

      <!-- Quark nodes -->
      {#each nodes as node, idx (`${node.q.flavor}-${idx}`)}
        <g transform={`translate(${node.point.x}, ${node.point.y})`}>
          <circle class={`quark ${colorClass(node.q.color)}`} r="18" />
          <text class="symbol" x="0" y="0">{node.q.symbol}</text>
        </g>
      {/each}

      <text class="caption" x="160" y="236">{label}</text>

    {:else}
      <!-- Nuclear liquid-drop ellipse -->
      <ellipse
        class="nucleus-shell"
        cx={CX} cy={CY} rx="90" ry="76"
      />

      <!-- Nucleon dots -->
      {#each nucleons as n, i (i)}
        <circle
          class={n.isProton ? "nucleon proton" : "nucleon neutron"}
          cx={n.cx} cy={n.cy}
          r={nucleons.length <= 4 ? 14 : nucleons.length <= 16 ? 10 : nucleons.length <= 40 ? 7 : 5}
        />
      {/each}

      <!-- Shell model labels -->
      <text class="caption" x="160" y="210">{nuclearLabel}</text>
      <text class="nucleus-hint" x="16" y="232">
        B/A ≈ {(nucleons.length > 0 ? (15.75 - 17.8 * Math.pow(protons + neutrons, -1/3) - 0.711 * protons * (protons - 1) / Math.pow(protons + neutrons, 1/3) / (protons + neutrons)).toFixed(2) : "—")} MeV
      </text>
    {/if}

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

  /* --- Quark-bag styles --- */

  .confinement {
    fill: rgba(136, 136, 136, 0.05);
    stroke: var(--color-border, var(--border));
    stroke-width: 1.3;
    stroke-dasharray: 4 4;
  }

  .gluon {
    fill: none;
    stroke: rgba(94, 184, 255, 0.9);
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

  /* --- Nuclear droplet styles --- */

  .nucleus-shell {
    fill: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 6%, var(--color-bg-inset, var(--bg-inset)));
    stroke: var(--color-border, var(--border));
    stroke-width: 1.5;
    stroke-dasharray: 6 3;
  }

  .nucleon {
    stroke: var(--color-border, var(--border));
    stroke-width: 0.8;
  }

  .proton {
    fill: color-mix(in srgb, #ff5f56 70%, var(--color-bg-inset, var(--bg-inset)));
  }

  .neutron {
    fill: color-mix(in srgb, #8e8e93 70%, var(--color-bg-inset, var(--bg-inset)));
  }

  .nucleus-hint {
    font-family: var(--font-mono);
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 10px;
  }

  .caption {
    font-family: var(--font-mono);
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 11px;
    text-anchor: middle;
  }
</style>