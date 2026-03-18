<script lang="ts">
  import type { ElementData } from "$lib/core/physics/nuclearDataLoader";

  export let element: ElementData;
  export let isotopeCount: number | null = null;
  export let stableCount: number | null = null;
  export let title = "Element profile";
  export let layout: "compact" | "full" = "full";

  function phaseFor(el: ElementData): string {
    if (["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(el.symbol)) return "gas";
    if (["Br", "Hg"].includes(el.symbol)) return "liquid";
    return "solid";
  }

  function blockFor(el: ElementData): "s" | "p" | "d" | "f" {
    if (el.group == null) return "f";
    if (el.group <= 2) return "s";
    if (el.group <= 12) return "d";
    return "p";
  }

  function valenceShell(el: ElementData): string {
    const tokens = el.electron_configuration.split(" ");
    return tokens[tokens.length - 1] ?? el.electron_configuration;
  }
</script>

<section class:compact={layout === "compact"} class="fact-sheet">
  <div class="fact-header">
    <div>
      <h5>{title}</h5>
      <p>{element.symbol} · {element.name}</p>
    </div>
    <span class="z-tag">Z {element.Z}</span>
  </div>

  <div class="fact-grid">
    <div><span>Atomic mass</span><strong>{element.atomic_mass.toFixed(4)}</strong></div>
    <div><span>Period / Group</span><strong>{element.period} / {element.group ?? "f"}</strong></div>
    <div><span>Block / Phase</span><strong>{blockFor(element)}-block · {phaseFor(element)}</strong></div>
    <div><span>Category</span><strong>{element.category.replace(/-/g, " ")}</strong></div>
    <div><span>Valence shell</span><strong class="mono">{valenceShell(element)}</strong></div>
    <div><span>Electron config</span><strong class="mono">{element.electron_configuration}</strong></div>
    {#if isotopeCount !== null}
      <div><span>Known isotopes</span><strong>{isotopeCount}</strong></div>
    {/if}
    {#if stableCount !== null}
      <div><span>Stable isotopes</span><strong>{stableCount}</strong></div>
    {/if}
  </div>
</section>

<style>
  .fact-sheet {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.42rem 0.46rem;
  }

  .fact-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.45rem;
    margin-bottom: 0.34rem;
  }

  .fact-header h5,
  .fact-header p,
  .fact-grid span,
  .fact-grid strong,
  .z-tag {
    font-family: var(--font-mono);
  }

  .fact-header h5 {
    margin: 0;
    font-size: 0.68rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .fact-header p {
    margin: 0.1rem 0 0;
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .z-tag {
    border: 1px solid color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 40%, var(--color-border, var(--border)));
    color: var(--color-accent, var(--hl-symbol));
    font-size: 0.58rem;
    padding: 0.08rem 0.24rem;
    background: rgba(var(--color-accent-rgb), 0.05);
  }

  .fact-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.28rem 0.62rem;
  }

  .fact-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    min-width: 0;
  }

  .fact-grid span {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .fact-grid strong {
    font-size: 0.67rem;
    color: var(--color-text-primary, var(--fg-primary));
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .fact-sheet.compact .fact-grid {
    gap: 0.24rem 0.42rem;
  }

  .fact-sheet.compact .fact-grid span {
    font-size: 0.52rem;
  }

  .fact-sheet.compact .fact-grid strong {
    font-size: 0.63rem;
  }
</style>
