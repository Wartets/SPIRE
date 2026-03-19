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
  import SpireGlossaryTerm from "$lib/components/ui/SpireGlossaryTerm.svelte";

  /** The glossary key to look up (e.g. "alpha_s", "m_z"). */
  export let term: string;
</script>

<SpireGlossaryTerm {term}>
  <slot />
</SpireGlossaryTerm>
