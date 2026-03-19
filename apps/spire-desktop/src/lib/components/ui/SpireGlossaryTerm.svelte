<!--
  SPIRE - SpireGlossaryTerm

  Reusable glossary-aware inline term wrapper.
  Looks up `term` in GlossaryService and shows a tooltip with definition,
  formula, value, and reference when available.
-->
<script lang="ts">
  import { tooltip } from "$lib/actions/tooltip";
  import { lookupTerm, type GlossaryEntry } from "$lib/core/services/GlossaryService";

  /** Glossary key (e.g. "cross_section", "freeze_out"). */
  export let term: string;

  /** Tooltip delay in milliseconds. */
  export let delay = 400;

  $: entry = lookupTerm(term);
  $: tooltipText = formatTooltip(entry);

  function formatTooltip(item: GlossaryEntry | undefined): string {
    if (!item) return "";
    const lines: string[] = [];
    lines.push(item.term);
    if (item.category) lines.push(`Category: ${item.category}`);
    lines.push(item.definition);
    if (item.formula) lines.push(`Formula: ${item.formula}`);
    if (item.value) lines.push(`Value: ${item.value}`);
    if (item.pdgRef) lines.push(`Ref: ${item.pdgRef}`);
    return lines.join("\n");
  }
</script>

<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
<span
  class="spire-glossary-term"
  class:has-entry={!!entry}
  use:tooltip={{ text: tooltipText, delay }}
  role="term"
  tabindex="0"
>
  <slot />
</span>

<style>
  .spire-glossary-term {
    display: inline;
    cursor: default;
  }

  .spire-glossary-term.has-entry {
    border-bottom: 1px dotted var(--color-accent);
    cursor: help;
  }
</style>
