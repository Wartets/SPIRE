<!--
  SPIRE - Analysis Widget Summary Card

  Lightweight summary for the Analysis Widget at medium zoom.
  Shows cross-section readout, event counts, and observable name
  without mounting Chart.js or WebGL dependencies.
-->
<script lang="ts">
  /**
   * These props are injected by the LOD wrapper from the widget's
   * local state.  Because AnalysisWidget keeps all state locally
   * (no global store), the summary uses reasonable defaults and
   * the LOD card relies on the label from the widget registry.
   *
   * In future iterations, a shared analysis store could pipe live
   * cross-section values here.
   */
  export let crossSection: string = "";
  export let eventsPassed: number = 0;
  export let eventsGenerated: number = 0;
  export let observableName: string = "";
</script>

<div class="analysis-summary">
  {#if crossSection}
    <div class="stat-row primary">
      <span class="stat-label">σ =</span>
      <span class="stat-value">{crossSection}</span>
      <span class="stat-unit">pb</span>
    </div>
  {/if}
  {#if eventsGenerated > 0}
    <div class="stat-row">
      <span class="stat-value">{eventsPassed.toLocaleString()}</span>
      <span class="stat-label">/ {eventsGenerated.toLocaleString()} events</span>
    </div>
  {/if}
  {#if observableName}
    <div class="observable">{observableName}</div>
  {/if}
  {#if !crossSection && eventsGenerated === 0}
    <span class="empty">No results yet</span>
  {/if}
</div>

<style>
  .analysis-summary {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .stat-row {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
  }
  .primary .stat-value {
    font-size: 2rem;
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
  .stat-unit {
    color: var(--fg-secondary, #888);
    font-size: 1.3rem;
  }
  .observable {
    color: var(--hl-symbol, #5eb8ff);
    font-size: 1.3rem;
    font-style: italic;
  }
  .empty {
    color: var(--fg-secondary, #888);
    font-style: italic;
    font-size: 1.4rem;
  }
</style>
