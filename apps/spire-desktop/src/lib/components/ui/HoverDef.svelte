<!--
  SPIRE — HoverDef Tooltip Component

  Inline wrapper that enriches physics terms with glossary tooltips.
  On hover, queries the GlossaryService for the term's definition,
  formula, value, and PDG reference, then displays a floating tooltip.

  Usage:
    <HoverDef term="alpha_s">α_s</HoverDef>

  The component is presentation-only — it queries the central
  GlossaryService and renders whatever data is registered for the
  given key.  If the key is not found, the slot content is rendered
  without decoration.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import { lookupTerm } from "$lib/core/services/GlossaryService";
  import type { GlossaryEntry } from "$lib/core/services/GlossaryService";

  /** The glossary key to look up (e.g. "alpha_s", "m_z"). */
  export let term: string;

  let entry: GlossaryEntry | undefined = undefined;
  let visible = false;
  let tooltipEl: HTMLDivElement | undefined;
  let anchorEl: HTMLSpanElement | undefined;

  // Position state
  let tipTop = 0;
  let tipLeft = 0;

  onMount(() => {
    entry = lookupTerm(term);
  });

  async function showTooltip(): Promise<void> {
    if (!entry) return;
    visible = true;
    await tick();
    positionTooltip();
  }

  function hideTooltip(): void {
    visible = false;
  }

  function positionTooltip(): void {
    if (!anchorEl || !tooltipEl) return;

    const anchorRect = anchorEl.getBoundingClientRect();
    const tipRect = tooltipEl.getBoundingClientRect();
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;

    // Default: above the anchor, horizontally centred
    let top = anchorRect.top - tipRect.height - 8;
    let left = anchorRect.left + anchorRect.width / 2 - tipRect.width / 2;

    // If tooltip would clip above the viewport, place below
    if (top < 4) {
      top = anchorRect.bottom + 8;
    }

    // Clamp horizontal position to viewport
    if (left < 4) left = 4;
    if (left + tipRect.width > viewportW - 4) {
      left = viewportW - tipRect.width - 4;
    }

    // Clamp vertical to viewport
    if (top + tipRect.height > viewportH - 4) {
      top = viewportH - tipRect.height - 4;
    }

    tipTop = top;
    tipLeft = left;
  }
</script>

<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
<span
  class="hover-def"
  class:has-entry={!!entry}
  bind:this={anchorEl}
  on:mouseenter={showTooltip}
  on:mouseleave={hideTooltip}
  on:focusin={showTooltip}
  on:focusout={hideTooltip}
  role="term"
  tabindex="0"
>
  <slot />
</span>

{#if visible && entry}
  <div
    class="hover-def-tooltip"
    bind:this={tooltipEl}
    style="top: {tipTop}px; left: {tipLeft}px;"
    role="tooltip"
  >
    <div class="tooltip-header">
      <span class="tooltip-term">{entry.term}</span>
      {#if entry.category}
        <span class="tooltip-category">{entry.category}</span>
      {/if}
    </div>

    <p class="tooltip-definition">{entry.definition}</p>

    {#if entry.formula}
      <div class="tooltip-formula">{entry.formula}</div>
    {/if}

    {#if entry.value}
      <div class="tooltip-value">
        <span class="tooltip-label">Value:</span> {entry.value}
      </div>
    {/if}

    {#if entry.pdgRef}
      <div class="tooltip-ref">
        <span class="tooltip-label">Ref:</span> {entry.pdgRef}
      </div>
    {/if}
  </div>
{/if}

<style>
  .hover-def {
    display: inline;
    cursor: default;
  }
  .hover-def.has-entry {
    border-bottom: 1px dotted var(--hl-symbol, #5eb8ff);
    cursor: help;
  }

  .hover-def-tooltip {
    position: fixed;
    z-index: 10000;
    max-width: 380px;
    min-width: 220px;
    padding: 0.6rem 0.75rem;
    background: var(--bg-primary, #121212);
    border: 1px solid var(--border-focus, #666);
    color: var(--fg-primary, #e8e8e8);
    font-family: var(--font-mono, monospace);
    font-size: 0.72rem;
    line-height: 1.45;
    pointer-events: none;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
  }

  .tooltip-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
  }
  .tooltip-term {
    font-weight: 700;
    color: var(--fg-accent, #fff);
    font-size: 0.78rem;
  }
  .tooltip-category {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary, #888);
    padding: 0.1rem 0.3rem;
    border: 1px solid var(--border, #333);
  }

  .tooltip-definition {
    margin: 0 0 0.35rem;
    color: var(--fg-primary, #e8e8e8);
  }

  .tooltip-formula {
    font-style: italic;
    color: var(--hl-value, #d4a017);
    margin-bottom: 0.25rem;
    word-break: break-word;
  }

  .tooltip-value {
    color: var(--hl-symbol, #5eb8ff);
    margin-bottom: 0.2rem;
  }

  .tooltip-ref {
    color: var(--fg-secondary, #888);
    font-size: 0.65rem;
  }

  .tooltip-label {
    font-weight: 700;
    color: var(--fg-secondary, #888);
    text-transform: uppercase;
    font-size: 0.6rem;
    letter-spacing: 0.04em;
  }
</style>
