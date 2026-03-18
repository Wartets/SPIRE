<script lang="ts">
  interface MoleculeTokenView {
    symbol: string;
    count: number;
    isotopeA?: number | null;
    atomicNumber?: number | null;
  }

  interface AtomNode {
    x: number;
    y: number;
    z: number;
    r: number;
    color: string;
    key: string;
  }

  interface Bond {
    from: number;
    to: number;
    key: string;
  }

  export let formula = "";
  export let tokens: MoleculeTokenView[] = [];
  export let highlightSymbol: string | null = null;
  export let subtitle = "";

  let viewportEl: HTMLDivElement | null = null;
  let rotating = false;
  let activePointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  let startYaw = -0.6;
  let startPitch = 0.26;
  let yaw = -0.6;
  let pitch = 0.26;

  const CPK_COLORS: Record<string, string> = {
    H: "#f2f5f7",
    C: "#8f9aa7",
    N: "#5e8fd7",
    O: "#d9645b",
    F: "#73c989",
    P: "#d8a24a",
    S: "#d7c64d",
    Cl: "#60b070",
    Br: "#96614d",
    I: "#7351a6",
    He: "#85d0e8",
    Ne: "#7cc7cf",
    Ar: "#7ab9d8",
  };

  function fallbackColor(z: number): string {
    const hue = 200 - Math.min(120, z);
    const sat = 26 + Math.min(34, z * 0.32);
    const light = 54 - Math.min(10, z * 0.05);
    return `hsl(${hue} ${sat}% ${light}%)`;
  }

  function colorFor(symbol: string, z: number): string {
    return CPK_COLORS[symbol] ?? fallbackColor(z);
  }

  function covalentRadius(symbol: string, z: number): number {
    const table: Record<string, number> = {
      H: 0.31,
      C: 0.76,
      N: 0.71,
      O: 0.66,
      F: 0.57,
      P: 1.07,
      S: 1.05,
      Cl: 1.02,
      Br: 1.2,
      I: 1.39,
    };
    if (table[symbol]) return table[symbol];
    if (z <= 2) return 0.35;
    if (z <= 10) return 0.72;
    if (z <= 18) return 1.02;
    if (z <= 36) return 1.18;
    if (z <= 54) return 1.32;
    return 1.48;
  }

  function expandAtoms(tokenList: MoleculeTokenView[]): MoleculeTokenView[] {
    const expanded: MoleculeTokenView[] = [];
    for (const token of tokenList) {
      const count = Math.max(1, Math.min(token.count, 18));
      for (let i = 0; i < count; i += 1) expanded.push(token);
    }
    return expanded;
  }

  function selectCenterAtom(atomList: MoleculeTokenView[]): number {
    return atomList
      .map((atom, index) => ({
        index,
        score: (atom.symbol === "H" ? -100 : 0) + (atom.atomicNumber ?? 1) * 6 + atom.count * 4,
      }))
      .sort((a, b) => b.score - a.score)[0]?.index ?? 0;
  }

  function buildScene(tokenList: MoleculeTokenView[]): { atoms: AtomNode[]; bonds: Bond[] } {
    const expanded = expandAtoms(tokenList);
    if (expanded.length === 0) return { atoms: [], bonds: [] };

    const centerIndex = selectCenterAtom(expanded);
    const ordered = [expanded[centerIndex], ...expanded.filter((_, index) => index !== centerIndex)];
    const atoms: AtomNode[] = [];

    ordered.forEach((atom, index) => {
      const z = Math.max(1, atom.atomicNumber ?? 1);
      const radius = covalentRadius(atom.symbol, z) * 8 + 2.2;

      if (index === 0) {
        atoms.push({
          x: 0,
          y: 0,
          z: 0,
          r: radius,
          color: colorFor(atom.symbol, z),
          key: `atom-${index}`,
        });
        return;
      }

      const shell = Math.floor((index - 1) / 6);
      const slot = (index - 1) % 6;
      const slotsThisShell = Math.min(6, ordered.length - shell * 6 - 1);
      const angle = (slot / Math.max(1, slotsThisShell)) * Math.PI * 2 + shell * 0.36;
      const distance = 38 + shell * 26 + radius * 0.9;

      atoms.push({
        x: Math.cos(angle) * distance,
        y: Math.sin(angle) * distance * 0.76,
        z: Math.sin(angle + 0.5) * distance * 0.42 - shell * 10,
        r: radius,
        color: colorFor(atom.symbol, z),
        key: `atom-${index}`,
      });
    });

    const bonds = atoms.slice(1).map((_, index) => ({
      from: 0,
      to: index + 1,
      key: `bond-0-${index + 1}`,
    }));

    return { atoms, bonds };
  }

  function rotatePoint(point: { x: number; y: number; z: number }): { x: number; y: number; z: number } {
    const cosY = Math.cos(yaw);
    const sinY = Math.sin(yaw);
    const cosX = Math.cos(pitch);
    const sinX = Math.sin(pitch);

    const x1 = point.x * cosY + point.z * sinY;
    const z1 = -point.x * sinY + point.z * cosY;
    const y2 = point.y * cosX - z1 * sinX;
    const z2 = point.y * sinX + z1 * cosX;

    return { x: x1, y: y2, z: z2 };
  }

  function projectPoint(point: { x: number; y: number; z: number }, perspective = 420): { x: number; y: number; depth: number; scale: number } {
    const depth = perspective / (perspective - point.z);
    return {
      x: 124 + point.x * depth,
      y: 92 + point.y * depth,
      depth: point.z,
      scale: depth,
    };
  }

  function beginRotate(event: PointerEvent): void {
    if (!viewportEl) return;
    rotating = true;
    activePointerId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    startYaw = yaw;
    startPitch = pitch;
    viewportEl.setPointerCapture(event.pointerId);
  }

  function moveRotate(event: PointerEvent): void {
    if (!rotating || event.pointerId !== activePointerId) return;
    yaw = startYaw + (event.clientX - startX) * 0.01;
    pitch = Math.max(-1.1, Math.min(1.1, startPitch + (event.clientY - startY) * 0.008));
  }

  function endRotate(event: PointerEvent): void {
    if (!rotating || event.pointerId !== activePointerId) return;
    if (viewportEl?.hasPointerCapture(event.pointerId)) viewportEl.releasePointerCapture(event.pointerId);
    rotating = false;
    activePointerId = null;
  }

  $: scene = buildScene(tokens);
  $: sceneLabel = subtitle ? `${formula || "selected molecule"} · ${subtitle}` : `${formula || "selected molecule"}`;
  $: projectedAtoms = scene.atoms
    .map((atom) => {
      const rotated = rotatePoint(atom);
      const projected = projectPoint(rotated);
      return { ...atom, ...projected };
    })
    .sort((a, b) => a.depth - b.depth);
  $: projectedBonds = scene.bonds.map((bond) => {
    const from = projectedAtoms[bond.from];
    const to = projectedAtoms[bond.to];
    return { ...bond, from, to, depth: ((from?.depth ?? 0) + (to?.depth ?? 0)) * 0.5 };
  }).sort((a, b) => a.depth - b.depth);
</script>

<section class="molecule-preview-card">
  <div
    class="molecule-viewport-shell"
    bind:this={viewportEl}
    role="application"
    aria-label={sceneLabel}
    on:pointerdown={beginRotate}
    on:pointermove={moveRotate}
    on:pointerup={endRotate}
    on:pointercancel={endRotate}
  >
    {#if projectedAtoms.length === 0}
      <div class="empty"></div>
    {:else}
      <svg class="molecule-viewport" viewBox="0 0 248 188" role="img" aria-label={sceneLabel}>
        <rect class="viewport" x="10" y="10" width="228" height="168" rx="10" />

        {#each projectedBonds as bond (bond.key)}
          {#if bond.from && bond.to}
            <line class="bond" x1={bond.from.x} y1={bond.from.y} x2={bond.to.x} y2={bond.to.y} />
          {/if}
        {/each}

        {#each projectedAtoms as atom (atom.key)}
          <circle
            class="atom"
            class:atom--highlight={highlightSymbol !== null && atom.key === projectedAtoms[0]?.key}
            cx={atom.x}
            cy={atom.y}
            r={Math.max(3.2, atom.r * atom.scale)}
            style={`--atom-color:${atom.color}`}
          />
        {/each}
      </svg>
    {/if}
  </div>
</section>

<style>
  .molecule-preview-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.35rem;
  }

  .molecule-viewport-shell {
    cursor: grab;
    touch-action: none;
  }

  .molecule-viewport-shell:active {
    cursor: grabbing;
  }

  .molecule-viewport,
  .empty {
    width: 100%;
    display: block;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.14);
    background: #09121b;
    min-height: 15rem;
  }

  .empty {
    min-height: 12rem;
  }

  .viewport {
    fill: rgba(8, 17, 26, 0.96);
    stroke: rgba(141, 161, 181, 0.12);
  }

  .bond {
    stroke: rgba(202, 214, 228, 0.54);
    stroke-width: 2.1;
    stroke-linecap: round;
  }

  .atom {
    fill: var(--atom-color);
    stroke: color-mix(in srgb, var(--atom-color) 74%, black);
    stroke-width: 0.8;
  }

  .atom--highlight {
    stroke: rgba(240, 244, 248, 0.9);
    stroke-width: 1.1;
  }
</style>
