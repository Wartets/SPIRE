<!--
  SPIRE - Amplitude Panel Summary Card

  Lightweight summary for the Amplitude Panel at medium zoom.
  Shows amplitude count, source diagram count, and the active
  expression snippet.
-->
<script lang="ts">
  import {
    amplitudeResults,
    activeAmplitude,
    generatedDiagrams,
  } from "$lib/stores/physicsStore";

  $: results = $amplitudeResults ?? [];
  $: expression = $activeAmplitude ?? "";
  $: diagramCount = $generatedDiagrams?.diagrams?.length ?? 0;
  $: snippet = expression.length > 60
    ? expression.slice(0, 57) + "…"
    : expression;
</script>

<div class="amp-summary">
  {#if results.length === 0}
    <span class="empty">No amplitudes</span>
  {:else}
    <div class="stat-row">
      <span class="stat-value">{results.length}</span>
      <span class="stat-label">amplitude{results.length !== 1 ? "s" : ""}</span>
    </div>
    {#if diagramCount > 0}
      <div class="stat-row">
        <span class="stat-label">from</span>
        <span class="stat-value">{diagramCount}</span>
        <span class="stat-label">diagram{diagramCount !== 1 ? "s" : ""}</span>
      </div>
    {/if}
    {#if snippet}
      <div class="expression">{snippet}</div>
    {/if}
  {/if}
</div>

<style>
  .amp-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .stat-row {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
  }
  .stat-value {
    color: var(--hl-value, #d4a017);
    font-size: 1.8rem;
    font-weight: 700;
  }
  .stat-label {
    color: var(--fg-secondary, #888);
    font-size: 1.4rem;
  }
  .expression {
    color: var(--hl-symbol, #5eb8ff);
    font-size: 1.3rem;
    font-style: italic;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .empty {
    color: var(--fg-secondary, #888);
    font-style: italic;
    font-size: 1.4rem;
  }
</style>
