<script lang="ts">
  interface MoleculeTokenView {
    symbol: string;
    count: number;
    isotopeA?: number | null;
  }

  export let formula = "";
  export let tokens: MoleculeTokenView[] = [];
  export let highlightSymbol: string | null = null;
  export let subtitle = "";

  let renderMode: "ball-stick" | "spacefill" | "wireframe" = "ball-stick";
  let cameraRotation = 18;
  let cameraTilt = 0.62;

  interface AtomNode {
    x: number;
    y: number;
    r: number;
    depth: number;
    symbol: string;
    isotopeA: number | null;
    key: string;
  }

  interface Bond {
    from: number;
    to: number;
    depth: number;
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

  function expandAtoms(tokenList: MoleculeTokenView[]): MoleculeTokenView[] {
    const expanded: string[] = [];
    const atoms: MoleculeTokenView[] = [];
    for (const token of tokenList) {
      const count = Math.max(1, Math.min(token.count, 12));
      for (let i = 0; i < count; i += 1) {
        expanded.push(token.symbol);
        atoms.push({ symbol: token.symbol, count: 1, isotopeA: token.isotopeA ?? null });
      }
    }

    if (expanded.length === 0) {
      return [];
    }

    return atoms.sort((a, b) => {
      if (a.symbol === "H" && b.symbol !== "H") return 1;
      if (a.symbol !== "H" && b.symbol === "H") return -1;
      return hashSymbol(b.symbol) - hashSymbol(a.symbol);
    });
  }

  function buildAtoms(tokenList: MoleculeTokenView[]): AtomNode[] {
    const expanded = expandAtoms(tokenList);

    if (expanded.length === 0) {
      return [];
    }

    return expanded
      .map((atom, index) => {
        if (index === 0) {
          return {
            x: 112,
            y: 84,
            r: 18,
            depth: 6,
            symbol: atom.symbol,
            isotopeA: atom.isotopeA ?? null,
            key: `${atom.symbol}-${index}`,
          } satisfies AtomNode;
        }

        const ring = Math.floor((index - 1) / 6);
        const pos = (index - 1) % 6;
        const angle = (pos / 6) * Math.PI * 2 + ring * 0.22 + 0.35 + (cameraRotation * Math.PI) / 180;
        const spread = 34 + ring * 24;
        const depth = Math.sin(angle) * 24 + ring * 5;
        const x = 112 + Math.cos(angle) * spread + ring * 3;
        const y = 84 + Math.sin(angle) * (spread * cameraTilt) - depth * 0.18;
        const r = Math.max(9.5, 15.5 - ring * 1.1);
        return {
          x,
          y,
          r,
          depth,
          symbol: atom.symbol,
          isotopeA: atom.isotopeA ?? null,
          key: `${atom.symbol}-${index}`,
        } satisfies AtomNode;
      })
      .sort((a, b) => a.depth - b.depth);
  }

  function buildBonds(atomList: AtomNode[]): Bond[] {
    if (atomList.length <= 1) return [];
    const centerIndex = atomList.findIndex((node) => node.x === 112 && node.y === 84);
    const hub = centerIndex >= 0 ? centerIndex : 0;
    const bonds: Bond[] = [];

    for (let i = 0; i < atomList.length; i += 1) {
      if (i === hub) continue;
      const from = hub;
      const to = i;
      bonds.push({
        from,
        to,
        depth: (atomList[from].depth + atomList[to].depth) / 2,
        key: `bond-${from}-${to}`,
      });
      if (i > 1 && i % 2 === 0) {
        bonds.push({
          from: i - 1,
          to: i,
          depth: (atomList[i - 1].depth + atomList[i].depth) / 2,
          key: `bond-${i - 1}-${i}`,
        });
      }
    }

    return bonds.sort((a, b) => a.depth - b.depth);
  }

  $: atoms = buildAtoms(tokens);
  $: bonds = buildBonds(atoms);
  $: atomCount = tokens.reduce((sum, token) => sum + token.count, 0);

  function renderedRadius(r: number): number {
    if (renderMode === "spacefill") return r * 1.26;
    if (renderMode === "wireframe") return Math.max(4, r * 0.42);
    return r;
  }
</script>

<section class="molecule-preview-card">
  <div class="header">
    <div>
      <h5>3D Molecule Preview</h5>
      <p>{formula || "No formula selected"}</p>
      {#if subtitle}
        <p class="subtitle">{subtitle}</p>
      {/if}
    </div>
    <div class="header-controls">
      <span class="count">{atomCount} atoms</span>
      <select bind:value={renderMode} aria-label="Molecule render mode">
        <option value="ball-stick">Ball-stick</option>
        <option value="spacefill">Space-fill</option>
        <option value="wireframe">Wireframe</option>
      </select>
    </div>
  </div>

  <div class="camera-controls">
    <label>
      Camera φ
      <input type="range" min="-180" max="180" step="1" bind:value={cameraRotation} />
      <span>{cameraRotation}°</span>
    </label>
    <label>
      Tilt
      <input type="range" min="0.38" max="0.86" step="0.01" bind:value={cameraTilt} />
      <span>{cameraTilt.toFixed(2)}</span>
    </label>
  </div>

  {#if atoms.length === 0}
    <div class="empty">Run molecule synthesis to generate the retained molecular preview.</div>
  {:else}
    <svg class={`mode-${renderMode}`} viewBox="0 0 240 184" role="img" aria-label={`3D-style molecule preview for ${formula}`}>
      <defs>
        <radialGradient id="sphere-shine" cx="32%" cy="28%">
          <stop offset="0%" stop-color="rgba(255,255,255,0.95)" />
          <stop offset="45%" stop-color="rgba(255,255,255,0.28)" />
          <stop offset="100%" stop-color="rgba(255,255,255,0)" />
        </radialGradient>
        <linearGradient id="bond-shine" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="rgba(255,255,255,0.5)" />
          <stop offset="100%" stop-color="rgba(255,255,255,0.06)" />
        </linearGradient>
      </defs>

      <rect class="bg-grid" x="8" y="8" width="224" height="168" rx="10" />

      {#each bonds as bond (bond.key)}
        {@const a = atoms[bond.from]}
        {@const b = atoms[bond.to]}
        <g>
          <line class="bond bond-shadow" x1={a.x + 2} y1={a.y + 2} x2={b.x + 2} y2={b.y + 2} />
          <line class="bond" x1={a.x} y1={a.y} x2={b.x} y2={b.y} />
        </g>
      {/each}

      {#each atoms as atom (atom.key)}
        <g transform={`translate(${atom.x}, ${atom.y})`}>
          <ellipse class="shadow" cx="2" cy={atom.r * 0.96} rx={atom.r * 0.84} ry={atom.r * 0.34} />
          <circle class="halo" r={renderedRadius(atom.r) * 1.14} style={`--atom-color:${colorFor(atom.symbol)}`} />
          <circle class="atom" r={renderedRadius(atom.r)} style={`--atom-color:${colorFor(atom.symbol)}`} />
          <circle class="shine" r={renderedRadius(atom.r) * 0.92} />
          {#if atom.isotopeA}
            <text class="mass-label" x={-renderedRadius(atom.r) * 0.54} y={-renderedRadius(atom.r) * 0.34}>{atom.isotopeA}</text>
          {/if}
          <text class="label" x="0" y="1">{atom.symbol}</text>
        </g>
      {/each}
    </svg>

    <div class="legend">
      {#each tokens as token (`${token.symbol}-${token.count}`)}
        <span class:highlight={token.symbol === highlightSymbol} style={`--atom-color:${colorFor(token.symbol)}`}>
          <i></i>{#if token.isotopeA}<sup>{token.isotopeA}</sup>{/if}{token.symbol}<sub>{token.count}</sub>
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

  .header-controls {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.15rem;
  }

  .header-controls select,
  .camera-controls input {
    accent-color: var(--color-accent, var(--hl-symbol));
  }

  .header-controls select {
    background: var(--color-bg-surface, var(--bg-surface));
    border: 1px solid var(--color-border, var(--border));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.58rem;
    padding: 0.08rem 0.18rem;
  }

  .camera-controls {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.45rem;
    margin-bottom: 0.32rem;
  }

  .camera-controls label {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.22rem;
    font-family: var(--font-mono);
    font-size: 0.55rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .camera-controls span {
    min-width: 2.3rem;
    text-align: right;
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

  .subtitle {
    color: var(--color-accent, var(--hl-symbol));
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

  .bg-grid {
    fill: rgba(255, 255, 255, 0.015);
    stroke: rgba(255, 255, 255, 0.06);
  }

  .bond {
    stroke: url(#bond-shine);
    stroke-width: 4.4;
    stroke-linecap: round;
  }

  svg.mode-spacefill .bond,
  svg.mode-spacefill .bond-shadow {
    stroke-width: 2.2;
    opacity: 0.55;
  }

  svg.mode-wireframe .bond {
    stroke: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 70%, white);
    stroke-width: 2;
  }

  svg.mode-wireframe .bond-shadow {
    stroke-width: 0;
  }

  .bond-shadow {
    stroke: rgba(0, 0, 0, 0.22);
    stroke-width: 5.8;
    stroke-linecap: round;
  }

  .shadow {
    fill: rgba(0, 0, 0, 0.18);
  }

  .halo {
    fill: color-mix(in srgb, var(--atom-color) 18%, transparent);
  }

  svg.mode-wireframe .halo,
  svg.mode-wireframe .shine {
    opacity: 0.2;
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

  .mass-label {
    font-family: var(--font-mono);
    font-size: 6px;
    font-weight: 700;
    fill: rgba(255,255,255,0.88);
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
