<script lang="ts">
  export let source: string = '';
  export let confidence: 'high' | 'medium' | 'low' = 'high';
</script>

<span class="provenance-badge confidence-{confidence}" title="Data source: {source}">
  {source}
</span>

<style>
  .provenance-badge {
    display: inline-block;
    font-size: var(--text-xs);
    font-weight: 600;
    padding: 0.15rem 0.45rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    white-space: nowrap;
  }

  .confidence-high {
    opacity: 1;
    border-color: var(--hl-success);
    color: var(--hl-success);
  }

  .confidence-medium {
    opacity: 1;
    border-color: var(--hl-value);
    color: var(--hl-value);
  }

  .confidence-low {
    opacity: 1;
    border-color: var(--hl-error);
    color: var(--hl-error);
  }
</style>
