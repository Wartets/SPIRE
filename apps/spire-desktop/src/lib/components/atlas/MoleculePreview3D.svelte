<script lang="ts">
  interface MoleculeTokenView {
    symbol: string;
    count: number;
  }

  export let formula = "";
  export let tokens: MoleculeTokenView[] = [];
  export let highlightSymbol: string | null = null;

  interface Sphere {
    x: number;
    y: number;
    r: number;
    depth: number;
    symbol: string;
    key: string;
  }

  function hashSymbol(symbol: string): number {
    let h = 0;
    for (let i = 0; i < symbol.length; i += 1) {
      h = (h * 31 + symbol.charCodeAt(i)) >>> 0;
    }
    return h;
  }

  function colorFor(symbol: string): string {
    const hash = hashSymbol(symbol);
    const hue = hash % 360;
    const sat = 52 + (hash % 18);
    const light = symbol === highlightSymbol ? 64 : 54;
    return `hsl(${hue} ${sat}% ${light}%)`;
  }

  function buildSpheres(tokenList: MoleculeTokenView[]): Sphere[] {
    const expanded: string[] = [];
    for (const token of tokenList) {
      const count = Math.max(1, Math.min(token.count, 12));
      for (let i = 0; i < count; i += 1) {
        expanded.push(token.symbol);
      }
    }

    if (expanded.length === 0) {
      return [];
    }

    return expanded
      .map((symbol, index) => {
        const ring = Math.floor(index / 6);
        const pos = index % 6;
        const angle = (pos / 6) * Math.PI * 2 + ring * 0.36;
        const spread = 18 + ring * 24;
        const depth = Math.sin(angle) * 24 + ring * 3;
        const x = 104 + Math.cos(angle) * spread + ring * 2;
        const y = 84 + Math.sin(angle) * (spread * 0.58) - depth * 0.14;
        const r = Math.max(11, 17 - ring * 1.2);
        return {
          x,
          y,
          r,
          depth,
          symbol,
          key: `${symbol}-${index}`,
        } satisfies Sphere;
      })
      .sort((a, b) => a.depth - b.depth);
  }

  $: spheres = buildSpheres(tokens);
  $: atomCount = tokens.reduce((sum, token) => sum + token.count, 0);
</script>

<section class="molecule-preview-card">
  <div class="header">
    <div>
      <h5>3D Molecule Preview</h5>
      <p>{formula || "No formula selected"}</p>
    </div>
    <span class="count">{atomCount} atoms</span>
  </div>

  {#if spheres.length === 0}
    <div class="empty">Run molecule synthesis to generate the retained molecular preview.</div>
  {:else}
    <svg viewBox="0 0 220 170" role="img" aria-label={`3D-style molecule preview for ${formula}`}>
      <defs>
        <radialGradient id="sphere-shine" cx="32%" cy="28%">
          <stop offset="0%" stop-color="rgba(255,255,255,0.95)" />
          <stop offset="45%" stop-color="rgba(255,255,255,0.28)" />
          <stop offset="100%" stop-color="rgba(255,255,255,0)" />
        </radialGradient>
      </defs>

      {#each spheres as sphere (sphere.key)}
        <g transform={`translate(${sphere.x}, ${sphere.y})`}>
          <ellipse class="shadow" cx="2" cy={sphere.r * 0.92} rx={sphere.r * 0.82} ry={sphere.r * 0.34} />
          <circle class="atom" r={sphere.r} style={`--atom-color:${colorFor(sphere.symbol)}`} />
          <circle class="shine" r={sphere.r * 0.92} />
          <text class="label" x="0" y="0">{sphere.symbol}</text>
        </g>
      {/each}
    </svg>

    <div class="legend">
      {#each tokens as token (`${token.symbol}-${token.count}`)}
        <span class:highlight={token.symbol === highlightSymbol} style={`--atom-color:${colorFor(token.symbol)}`}>
          <i></i>{token.symbol}<sub>{token.count}</sub>
        </span>
      {/each}
    </div>
  {/if}
</section>

<style>
  .molecule-preview-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    align-items: baseline;
    margin-bottom: 0.35rem;
  }

  .header h5 {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .header p,
  .count,
  .empty,
  .legend span {
    font-family: var(--font-mono);
  }

  .header p {
    margin: 0.12rem 0 0;
    font-size: 0.66rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .count {
    font-size: 0.62rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .empty {
    font-size: 0.68rem;
    color: var(--color-text-muted, var(--fg-secondary));
    border: 1px dashed var(--color-border, var(--border));
    padding: 0.5rem;
  }

  svg {
    width: 100%;
    height: auto;
    display: block;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.16);
    background:
      radial-gradient(circle at top, rgba(255,255,255,0.08), transparent 42%),
      linear-gradient(180deg, rgba(255,255,255,0.03), transparent 60%);
  }

  .shadow {
    fill: rgba(0, 0, 0, 0.18);
  }

  .atom {
    fill: var(--atom-color);
    stroke: color-mix(in srgb, var(--atom-color) 74%, black);
    stroke-width: 1;
  }

  .shine {
    fill: url(#sphere-shine);
  }

  .label {
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    fill: rgba(10, 10, 16, 0.88);
    text-anchor: middle;
    dominant-baseline: central;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.45rem;
    margin-top: 0.35rem;
  }

  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 0.24rem;
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .legend span.highlight {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .legend i {
    width: 0.66rem;
    height: 0.66rem;
    border-radius: 50%;
    display: inline-block;
    background: var(--atom-color);
    box-shadow: inset 0 0 0 1px rgba(0,0,0,0.25);
  }
</style>
