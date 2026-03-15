<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { DecayChannelPreset } from "$lib/core/physics/composites";

  interface DecaySelectEvent {
    parentId: string;
    finalStateIds: string[];
    label: string;
  }

  export let parentId = "";
  export let channels: DecayChannelPreset[] = [];

  const dispatch = createEventDispatcher<{ select: DecaySelectEvent }>();

  interface Row {
    ch: DecayChannelPreset;
    y: number;
    finalLabel: string;
  }

  const width = 640;
  const rootX = 38;
  const splitX = 160;
  const branchX = 320;
  const finalX = 510;

  function interactionClass(interaction: DecayChannelPreset["interaction"]): string {
    if (interaction === "strong") return "strong";
    if (interaction === "em") return "em";
    return "weak";
  }

  function emitSelect(ch: DecayChannelPreset): void {
    dispatch("select", {
      parentId,
      finalStateIds: ch.finalStateIds,
      label: `${parentId} -> ${ch.finalStateIds.join(" + ")}`,
    });
  }

  function onBranchKeydown(e: KeyboardEvent, ch: DecayChannelPreset): void {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      emitSelect(ch);
    }
  }

  $: sorted = [...channels].sort((a, b) => b.branchingRatio - a.branchingRatio);
  $: spacing = 56;
  $: top = 30;
  $: rows = sorted.map((ch, i) => ({
    ch,
    y: top + i * spacing,
    finalLabel: ch.finalStateIds.join(" + "),
  })) as Row[];
  $: height = Math.max(110, top * 2 + Math.max(0, rows.length - 1) * spacing);
  $: rootY = rows.length > 0 ? 0.5 * (rows[0].y + rows[rows.length - 1].y) : 52;
</script>

<div class="decay-tree-wrap">
  {#if rows.length === 0}
    <p class="empty">No decay presets available for this state.</p>
  {:else}
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-label={`Decay tree for ${parentId}`}>
      <line class="trunk" x1={rootX} y1={rootY} x2={splitX} y2={rootY} />
      <text class="root" x={rootX - 4} y={rootY}>{parentId}</text>

      {#each rows as row, idx (`${row.finalLabel}-${idx}`)}
        <line class="branch" x1={splitX} y1={rootY} x2={branchX} y2={row.y} />
        <line class="leaf" x1={branchX} y1={row.y} x2={finalX} y2={row.y} />

        <text class="final" x={finalX + 8} y={row.y}>{row.finalLabel}</text>
        <text
          class={`label ${interactionClass(row.ch.interaction)}`}
          x={(splitX + branchX) * 0.5}
          y={(rootY + row.y) * 0.5 - 4}
        >
          {(row.ch.branchingRatio * 100).toFixed(2)}% · {row.ch.interaction}
        </text>

        <rect
          class="hit"
          x={splitX - 6}
          y={Math.min(rootY, row.y) - 8}
          width={finalX - splitX + 26}
          height={Math.abs(row.y - rootY) + 16}
          role="button"
          tabindex="0"
          aria-label={`Load decay ${parentId} to ${row.finalLabel}`}
          on:click={() => emitSelect(row.ch)}
          on:keydown={(e) => onBranchKeydown(e, row.ch)}
        />
      {/each}
    </svg>
  {/if}
</div>

<style>
  .decay-tree-wrap {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    min-height: 120px;
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

  .label {
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 10px;
  }

  .label.strong {
    fill: var(--color-success, var(--hl-success));
  }

  .label.weak {
    fill: var(--hl-value);
  }

  .label.em {
    fill: var(--color-accent, var(--hl-symbol));
  }

  .hit {
    fill: transparent;
    cursor: pointer;
  }

  .hit:focus {
    outline: none;
  }

  .hit:focus-visible {
    fill: rgba(var(--color-accent-rgb, 94, 184, 255), 0.1);
    stroke: var(--color-accent, var(--hl-symbol));
    stroke-width: 1;
  }

  .empty {
    margin: 0;
    padding: 0.5rem;
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.75rem;
    font-style: italic;
  }
</style>
