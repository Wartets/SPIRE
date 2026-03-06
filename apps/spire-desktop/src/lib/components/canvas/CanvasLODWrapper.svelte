<!--
  SPIRE - Canvas LOD Wrapper

  Central level-of-detail wrapper for canvas widgets.  Intercepts the
  current zoom state and renders one of three tiers:

    full    (zoom ≥ 0.6)  — The standard interactive widget component.
    summary (0.3 ≤ z < 0.6) — Simplified card: title, icon, and a
                               widget-provided summary snippet.
    minimal (z < 0.3)      — Dense title card showing only the icon,
                               widget type label, and coloured accent.

  Widget components remain entirely unaware of LOD logic.  This wrapper
  handles tier switching, CSS variable injection, and the transition
  between states.

  Summary data can be provided by widgets that export a `summaryHtml`
  slot or by a fallback "widget-type + dimensions" description.
-->
<script lang="ts">
  import type { WidgetType } from "$lib/stores/notebookStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import {
    type LodLevel,
    zoomToLod,
    WIDGET_ICONS,
    WIDGET_ACCENT,
  } from "$lib/components/canvas/lodUtils";

  // ── Props ─────────────────────────────────────────────────────────────

  /** The widget type being rendered. */
  export let widgetType: WidgetType;

  /** Current canvas zoom level. */
  export let zoom: number = 1;

  // ── LOD Derivation ────────────────────────────────────────────────────

  $: lodLevel = zoomToLod(zoom) as LodLevel;
  $: label = WIDGET_LABELS[widgetType] ?? widgetType;
  $: icon = WIDGET_ICONS[widgetType] ?? '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.2"><rect x="2" y="2" width="12" height="12" rx="1.5" /></svg>';
  $: accent = WIDGET_ACCENT[widgetType] ?? "#5eb8ff";

  // Compute the font-floor CSS variable (min 9px screen-space)
  $: effectiveFontScale = Math.max(9 / 12, 1 / zoom);
</script>

<div
  class="lod-wrapper"
  class:lod-full={lodLevel === "full"}
  class:lod-summary={lodLevel === "summary"}
  class:lod-minimal={lodLevel === "minimal"}
  style="--lod-accent: {accent}; --canvas-zoom: {zoom};"
>
  {#if lodLevel === "full"}
    <!-- Full interactive widget -->
    <slot name="full" />
  {:else if lodLevel === "summary"}
    <!-- Summary card: icon + title + optional summary snippet -->
    <div class="lod-summary-card">
      <div class="lod-summary-header">
        <span class="lod-icon" style="color: {accent};">{@html icon}</span>
        <span class="lod-label">{label}</span>
      </div>
      <div class="lod-summary-body">
        <slot name="summary">
          <span class="lod-placeholder-text">{label}</span>
        </slot>
      </div>
    </div>
  {:else}
    <!-- Minimal tile: dense coloured rectangle -->
    <div class="lod-minimal-card" style="border-left-color: {accent};">
      <span class="lod-icon">{@html icon}</span>
      <span class="lod-label">{label}</span>
    </div>
  {/if}
</div>

<style>
  .lod-wrapper {
    width: 100%;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* ── Full tier ── */
  .lod-full {
    /* Standard rendering — slot content fills the wrapper */
    display: flex;
    flex-direction: column;
  }

  .lod-full :global(> *) {
    flex: 1;
    min-height: 0;
  }

  /* ── Summary tier ── */
  .lod-summary-card {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
    padding: 1.4rem;
    height: 100%;
    overflow: hidden;
  }

  .lod-summary-header {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    flex-shrink: 0;
  }

  .lod-summary-header .lod-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }

  .lod-summary-header .lod-icon :global(svg) {
    width: 36px;
    height: 36px;
  }

  .lod-summary-header .lod-label {
    font-size: 2.1rem;
    font-weight: 700;
    color: var(--fg-accent, #fff);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .lod-summary-body {
    flex: 1;
    overflow: hidden;
    font-size: 1.7rem;
    color: var(--fg-secondary, #888);
    line-height: 1.5;
  }

  .lod-placeholder-text {
    color: var(--fg-secondary, #888);
    font-style: italic;
    font-size: 1.5rem;
  }

  /* ── Minimal tier ── */
  .lod-minimal-card {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.2rem;
    height: 100%;
    background: var(--bg-inset, #0e0e0e);
    border-left: 8px solid var(--lod-accent, #5eb8ff);
    overflow: hidden;
  }

  .lod-minimal-card .lod-icon {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }

  .lod-minimal-card .lod-icon :global(svg) {
    width: 48px;
    height: 48px;
  }

  .lod-minimal-card .lod-label {
    font-size: 3.6rem;
    font-weight: 700;
    color: var(--fg-primary, #e8e8e8);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    white-space: normal;
    word-wrap: break-word;
    overflow-wrap: break-word;
    line-height: 1.15;
    overflow: hidden;
  }
</style>
