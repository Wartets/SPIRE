<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { DecayChannelPreset } from "$lib/core/physics/composites";
  import type { IsotopeData } from "$lib/core/physics/nuclearDataLoader";
  import { formatHalfLife } from "$lib/core/physics/nuclearDataLoader";
  import MathRenderer from "$lib/components/math/MathRenderer.svelte";

  interface DecaySelectEvent {
    parentId: string;
    finalStateIds: string[];
    label: string;
  }

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  /** Parent particle ID (for hadronic / EW decay presets). */
  export let parentId = "";
  /** Hadronic / EW decay channels. */
  export let channels: DecayChannelPreset[] = [];

  /**
   * Isotope decay configuration.  When provided, the tree renders nuclear
   * decay modes instead of particle decay channels.
   */
  export let isotopeDecay: {
    Z: number;
    A: number;
    symbol: string;
    isotope: IsotopeData;
  } | null = null;

  // ---------------------------------------------------------------------------
  // Events
  // ---------------------------------------------------------------------------

  const dispatch = createEventDispatcher<{ select: DecaySelectEvent }>();

  // ---------------------------------------------------------------------------
  // Geometry constants – adapt to label width dynamically
  // ---------------------------------------------------------------------------

  const rootX = 38;
  const splitX = 148;
  /** Right-hand column position; grows with the number of final-state IDs. */
  $: branchX = 280;
  $: finalX = branchX + 160;
  $: width = finalX + 80;

  // ---------------------------------------------------------------------------
  // Row computation – hadronic mode
  // ---------------------------------------------------------------------------

  interface HadronRow {
    ch: DecayChannelPreset;
    y: number;
    finalLabel: string;
  }

  function interactionClass(interaction: DecayChannelPreset["interaction"]): string {
    if (interaction === "strong") return "strong";
    if (interaction === "em") return "em";
    return "weak";
  }

  function emitHadronSelect(ch: DecayChannelPreset): void {
    dispatch("select", {
      parentId,
      finalStateIds: ch.finalStateIds,
      label: `${parentId} → ${ch.finalStateIds.join(" + ")}`,
    });
  }

  function onHadronKeydown(e: KeyboardEvent, ch: DecayChannelPreset): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      emitHadronSelect(ch);
    }
  }

  const spacing = 56;
  const top = 30;

  $: sortedChannels = [...channels].sort((a, b) => b.branchingRatio - a.branchingRatio);
  $: hadronRows = sortedChannels.map((ch, i) => ({
    ch,
    y: top + i * spacing,
    finalLabel: ch.finalStateIds.join(" + "),
  })) as HadronRow[];
  $: hadronHeight = Math.max(110, top * 2 + Math.max(0, hadronRows.length - 1) * spacing);
  $: hadronRootY = hadronRows.length > 0 ? 0.5 * (hadronRows[0].y + hadronRows[hadronRows.length - 1].y) : 52;

  // ---------------------------------------------------------------------------
  // Row computation – isotope mode
  // ---------------------------------------------------------------------------

  interface IsotopeRow {
    mode: string;
    fraction: number;
    daughterZ: number;
    daughterA: number;
    modeLabel: string;
    brLabel: string;
    daughterLabel: string;
    y: number;
  }

  function decayModeLabel(mode: string): string {
    switch (mode) {
      case "beta-minus": return "β⁻";
      case "beta-plus":  return "β⁺";
      case "alpha":      return "α";
      case "ec":         return "EC";
      case "gamma":      return "γ";
      case "sf":         return "SF";
      default:           return mode;
    }
  }

  function decayModeClass(mode: string): string {
    switch (mode) {
      case "beta-minus": return "mode-bm";
      case "beta-plus":  return "mode-bp";
      case "alpha":      return "mode-alpha";
      case "ec":         return "mode-ec";
      default:           return "";
    }
  }

  $: isotopeRows = isotopeDecay
    ? isotopeDecay.isotope.decay_modes.map((dm, i) => ({
        mode: dm.mode,
        fraction: dm.fraction,
        daughterZ: dm.daughter_Z,
        daughterA: dm.daughter_A,
        modeLabel: decayModeLabel(dm.mode),
        brLabel: `${(dm.fraction * 100).toFixed(1)}%`,
        daughterLabel: `Z=${dm.daughter_Z}, A=${dm.daughter_A}`,
        y: top + i * spacing,
      })) as IsotopeRow[]
    : [];
  $: isotopeHeight = Math.max(110, top * 2 + Math.max(0, isotopeRows.length - 1) * spacing);
  $: isotopeRootY = isotopeRows.length > 0 ? 0.5 * (isotopeRows[0].y + isotopeRows[isotopeRows.length - 1].y) : 52;
  $: isotopeParentLabel = isotopeDecay ? `${isotopeDecay.symbol}-${isotopeDecay.A}` : "";
  $: isotopeHalfLife = isotopeDecay ? formatHalfLife(isotopeDecay.isotope.half_life_s) : "";

  const latexAliases: Record<string, string> = {
    gamma: "\\gamma",
    g: "g",
    "pi+": "\\pi^{+}",
    "pi-": "\\pi^{-}",
    pi0: "\\pi^{0}",
    "k+": "K^{+}",
    "k-": "K^{-}",
    k0: "K^{0}",
    lambda0: "\\Lambda^{0}",
    "sigma+": "\\Sigma^{+}",
    sigma0: "\\Sigma^{0}",
    "sigma-": "\\Sigma^{-}",
    xi0: "\\Xi^{0}",
    "xi-": "\\Xi^{-}",
    "omega-": "\\Omega^{-}",
    rho0: "\\rho^{0}",
    phi_meson: "\\phi",
    j_psi: "J/\\psi",
    upsilon: "\\Upsilon",
    "e-": "e^{-}",
    "e+": "e^{+}",
    "mu-": "\\mu^{-}",
    "mu+": "\\mu^{+}",
    "tau-": "\\tau^{-}",
    "tau+": "\\tau^{+}",
    nu_e: "\\nu_{e}",
    nu_mu: "\\nu_{\\mu}",
    nu_tau: "\\nu_{\\tau}",
  };

  function particleLatex(id: string): string {
    const trimmed = id.trim();
    if (!trimmed) return "\\varnothing";
    if (latexAliases[trimmed]) return latexAliases[trimmed];
    if (trimmed.endsWith("_bar")) return `\\bar{${particleLatex(trimmed.slice(0, -4))}}`;
    return `\\mathrm{${trimmed.replace(/_/g, "\\,")}}`;
  }

  function interactionLatex(interaction: DecayChannelPreset["interaction"]): string {
    switch (interaction) {
      case "strong":
        return "\\text{strong}";
      case "em":
        return "\\text{EM}";
      default:
        return "\\text{weak}";
    }
  }

  function hadronChannelLatex(ch: DecayChannelPreset): string {
    return [
      `${particleLatex(parentId)} \\to ${ch.finalStateIds.map((id) => particleLatex(id)).join(" + ")}`,
      `\\mathrm{BR} = ${(ch.branchingRatio * 100).toFixed(2)}\\%`,
      interactionLatex(ch.interaction),
    ].join("\\qquad");
  }

  function decayModeLatex(mode: string): string {
    switch (mode) {
      case "beta-minus":
        return "\\beta^{-}";
      case "beta-plus":
        return "\\beta^{+}";
      case "alpha":
        return "\\alpha";
      case "ec":
        return "\\mathrm{EC}";
      case "gamma":
        return "\\gamma";
      case "sf":
        return "\\mathrm{SF}";
      default:
        return `\\mathrm{${mode}}`;
    }
  }

  function isotopeChannelLatex(row: IsotopeRow): string {
    if (!isotopeDecay) return "";
    return [
      `{}^{${isotopeDecay.A}}_{${isotopeDecay.Z}}\\mathrm{${isotopeDecay.symbol}} \\xrightarrow{${decayModeLatex(row.mode)}} \\left(A=${row.daughterA},\\, Z=${row.daughterZ}\\right)`,
      `\\mathrm{BR} = ${row.brLabel.replace("%", "\\%")}`,
    ].join("\\qquad");
  }
</script>

<div class="decay-tree-wrap">
  {#if isotopeDecay}
    <!-- ── Isotope decay tree ───────────────────────────── -->
    {#if isotopeRows.length === 0}
      <p class="empty">
        {isotopeParentLabel} is stable (t½ = {isotopeHalfLife}).
      </p>
    {:else}
      <svg viewBox={`0 0 ${width} ${isotopeHeight}`} role="img" aria-label={`Decay tree for ${isotopeParentLabel}`}>
        <defs>
          <marker id="decay-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <path d="M0,0 L8,3 L0,6 z" fill="currentColor"></path>
          </marker>
        </defs>
        <!-- Half-life label below parent -->
        <text class="root" x={rootX - 4} y={isotopeRootY - 12}>{isotopeParentLabel}</text>
        <text class="hl-label" x={rootX - 4} y={isotopeRootY + 2}>t½={isotopeHalfLife}</text>
        <line class="trunk" x1={rootX} y1={isotopeRootY} x2={splitX} y2={isotopeRootY} marker-end="url(#decay-arrow)" />

        {#each isotopeRows as row, idx (`${row.mode}-${idx}`)}
          <line class="branch" x1={splitX} y1={isotopeRootY} x2={branchX} y2={row.y} marker-end="url(#decay-arrow)" />
          <line class="leaf" x1={branchX} y1={row.y} x2={finalX} y2={row.y} marker-end="url(#decay-arrow)" />

          <text class={`mode-tag ${decayModeClass(row.mode)}`} x={branchX + 6} y={row.y - 5}>
            {row.modeLabel}
          </text>
          <text class="final" x={finalX + 8} y={row.y}>{row.daughterLabel}</text>
          <text class="label" x={(splitX + branchX) * 0.5} y={(isotopeRootY + row.y) * 0.5 - 4}>
            {row.brLabel}
          </text>
        {/each}
      </svg>
      <div class="channel-math-list">
        {#each isotopeRows as row, idx (`math-iso-${row.mode}-${idx}`)}
          <div class="channel-math-item">
            <MathRenderer latex={isotopeChannelLatex(row)} block={true} />
          </div>
        {/each}
      </div>
    {/if}

  {:else}
    <!-- ── Hadronic / EW decay tree ──────────────────────── -->
    {#if hadronRows.length === 0}
      <p class="empty">No decay presets available for this state.</p>
    {:else}
      <svg viewBox={`0 0 ${width} ${hadronHeight}`} role="img" aria-label={`Decay tree for ${parentId}`}>
        <defs>
          <marker id="decay-arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
            <path d="M0,0 L8,3 L0,6 z" fill="currentColor"></path>
          </marker>
        </defs>
        <line class="trunk" x1={rootX} y1={hadronRootY} x2={splitX} y2={hadronRootY} marker-end="url(#decay-arrow)" />
        <text class="root" x={rootX - 4} y={hadronRootY}>{parentId}</text>

        {#each hadronRows as row, idx (`${row.finalLabel}-${idx}`)}
          <line class="branch" x1={splitX} y1={hadronRootY} x2={branchX} y2={row.y} marker-end="url(#decay-arrow)" />
          <line class="leaf" x1={branchX} y1={row.y} x2={finalX} y2={row.y} marker-end="url(#decay-arrow)" />

          <text class="final" x={finalX + 8} y={row.y}>{row.finalLabel}</text>
          <text
            class={`label ${interactionClass(row.ch.interaction)}`}
            x={(splitX + branchX) * 0.5}
            y={(hadronRootY + row.y) * 0.5 - 4}
          >
            {(row.ch.branchingRatio * 100).toFixed(2)}% · {row.ch.interaction}
          </text>

          <rect
            class="hit"
            x={splitX - 6}
            y={Math.min(hadronRootY, row.y) - 8}
            width={finalX - splitX + 26}
            height={Math.abs(row.y - hadronRootY) + 16}
            role="button"
            tabindex="0"
            aria-label={`Load decay ${parentId} to ${row.finalLabel}`}
            on:click={() => emitHadronSelect(row.ch)}
            on:keydown={(e) => onHadronKeydown(e, row.ch)}
          />
        {/each}
      </svg>
      <div class="channel-math-list">
        {#each sortedChannels as ch, idx (`math-hadron-${idx}`)}
          <div class="channel-math-item">
            <MathRenderer latex={hadronChannelLatex(ch)} block={true} />
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .decay-tree-wrap {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    min-height: 120px;
    overflow-x: auto;
  }

  svg {
    width: 100%;
    height: auto;
    display: block;
    overflow: visible;
  }

  .trunk,
  .branch,
  .leaf {
    stroke: var(--color-text-primary, var(--fg-primary));
    stroke-width: 1.1;
    fill: none;
  }

  .root,
  .final,
  .label {
    font-family: var(--font-mono);
    font-size: 11px;
    fill: var(--color-text-primary, var(--fg-primary));
    dominant-baseline: middle;
  }

  .root {
    text-anchor: end;
    fill: var(--color-accent, var(--hl-symbol));
    font-weight: 700;
  }

  .hl-label {
    font-family: var(--font-mono);
    font-size: 9px;
    fill: var(--color-text-muted, var(--fg-secondary));
    text-anchor: end;
    dominant-baseline: middle;
  }

  .label {
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 10px;
  }

  .label.strong { fill: var(--color-success, var(--hl-success)); }
  .label.weak   { fill: var(--hl-value); }
  .label.em     { fill: var(--color-accent, var(--hl-symbol)); }

  .mode-tag {
    font-family: var(--font-mono);
    font-size: 11px;
    dominant-baseline: middle;
    font-weight: 700;
  }

  .mode-tag.mode-bm     { fill: var(--hl-error); }
  .mode-tag.mode-bp     { fill: var(--hl-value); }
  .mode-tag.mode-alpha  { fill: var(--color-accent, var(--hl-symbol)); }
  .mode-tag.mode-ec     { fill: var(--color-success, var(--hl-success)); }

  .hit {
    fill: transparent;
    cursor: pointer;
  }

  .hit:focus { outline: none; }

  .hit:focus-visible {
    fill: rgba(94, 184, 255, 0.1);
    stroke: var(--color-accent, var(--hl-symbol));
    stroke-width: 1;
  }

  .empty {
    margin: 0;
    padding: 0.5rem;
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.75rem;
    font-style: italic;
    font-family: var(--font-mono);
  }

  .channel-math-list {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
    padding: 0.35rem 0.45rem 0.45rem;
    border-top: 1px solid color-mix(in srgb, var(--color-border, var(--border)) 80%, transparent);
    background: rgba(var(--color-accent-rgb), 0.03);
  }

  .channel-math-item {
    border: 1px solid color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 14%, var(--color-border, var(--border)));
    background: rgba(var(--color-surface-rgb, 15, 23, 42), 0.22);
    padding: 0.18rem 0.28rem;
  }
</style>
