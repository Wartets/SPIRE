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
    font-size: 0.75rem;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 3px;
    background: var(--surface-variant, #f0f0f0);
    color: var(--on-surface-variant, #666);
    white-space: nowrap;
    transition: opacity 0.2s ease;
  }

  .confidence-high {
    opacity: 1;
    background: var(--success-container, #d4edda);
    color: var(--on-success-container, #155724);
  }

  .confidence-medium {
    opacity: 0.8;
    background: var(--warning-container, #fff3cd);
    color: var(--on-warning-container, #856404);
  }

  .confidence-low {
    opacity: 0.6;
    background: var(--error-container, #f8d7da);
    color: var(--on-error-container, #721c24);
  }
</style>
