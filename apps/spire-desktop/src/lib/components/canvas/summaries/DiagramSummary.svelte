<!--
  SPIRE - Diagram Visualizer Summary Card

  Lightweight summary view rendered when the canvas zoom is between
  0.3× and 0.6×.  Shows diagram count, loop order, channels, and
  selected diagram metadata without mounting the full interactive
  renderer or Mermaid/Three.js dependencies.
-->
<script lang="ts">
  import { generatedDiagrams } from "$lib/stores/physicsStore";
  import type { LoopOrder } from "$lib/types/spire";

  function loopLabel(lo: LoopOrder): string {
    if (lo === "Tree") return "Tree";
    if (lo === "OneLoop") return "1-Loop";
    if (lo === "TwoLoop") return "2-Loop";
    if (typeof lo === "object" && "NLoop" in lo) return `${lo.NLoop}-Loop`;
    return String(lo);
  }

  $: diagrams = $generatedDiagrams?.diagrams ?? [];
  $: maxLoop = $generatedDiagrams?.max_loop_order;
  $: first = diagrams[0] ?? null;
</script>

<div class="diag-summary">
  {#if diagrams.length === 0}
    <span class="empty">No diagrams</span>
  {:else}
    <div class="stat-row">
      <span class="stat-value">{diagrams.length}</span>
      <span class="stat-label">diagram{diagrams.length !== 1 ? "s" : ""}</span>
    </div>
    {#if maxLoop}
      <div class="stat-row">
        <span class="stat-label">max</span>
        <span class="stat-value">{loopLabel(maxLoop)}</span>
      </div>
    {/if}
    {#if first}
      <div class="stat-row">
        <span class="stat-label">S =</span>
        <span class="stat-value">{first.symmetry_factor}</span>
        <span class="stat-label sep">·</span>
        <span class="stat-value">{first.channels.join("+") || "—"}</span>
      </div>
      {#if first.is_one_particle_irreducible}
        <span class="tag">1PI</span>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .diag-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .stat-row {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .stat-value {
    color: var(--hl-value, #d4a017);
    font-size: 1.8rem;
    font-weight: 700;
  }
  .stat-label {
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 1.4rem;
  }
  .sep { opacity: 0.4; }
  .tag {
    display: inline-block;
    font-size: 1.1rem;
    padding: 0.05rem 0.3rem;
    border: 1px solid var(--color-accent);
    color: var(--color-accent);
    width: fit-content;
  }
  .empty {
    color: var(--fg-secondary, var(--color-text-muted));
    font-style: italic;
    font-size: 1.4rem;
  }
</style>
