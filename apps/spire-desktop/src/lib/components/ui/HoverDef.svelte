<!--
  SPIRE - HoverDef Tooltip Component

  Inline wrapper that enriches physics terms with glossary tooltips.
  On hover, queries the GlossaryService for the term's definition,
  formula, value, and PDG reference, then displays a floating tooltip.

  Usage:
    <HoverDef term="alpha_s">α_s</HoverDef>

  The component is presentation-only - it queries the central
  GlossaryService and renders whatever data is registered for the
  given key.  If the key is not found, the slot content is rendered
  without decoration.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { lookupTerm } from "$lib/core/services/GlossaryService";
  import type { GlossaryEntry } from "$lib/core/services/GlossaryService";
  import { tooltip } from "$lib/actions/tooltip";

  /** The glossary key to look up (e.g. "alpha_s", "m_z"). */
  export let term: string;

  let entry: GlossaryEntry | undefined = undefined;
  let tooltipText = "";

  onMount(() => {
    entry = lookupTerm(term);
    tooltipText = formatTooltip(entry);
  });

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
  class="hover-def"
  class:has-entry={!!entry}
  use:tooltip={{ text: tooltipText, delay: 400 }}
  role="term"
  tabindex="0"
>
  <slot />
</span>

<style>
  .hover-def {
    display: inline;
    cursor: default;
  }
  .hover-def.has-entry {
    border-bottom: 1px dotted var(--color-accent);
    cursor: help;
  }

</style>
