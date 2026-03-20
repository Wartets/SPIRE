<script lang="ts">
  import type { PdgValue } from '$lib/types/spire';

  export let value: PdgValue;
  export let showSource: boolean = false;

  $: displayValue = formatPdgValue(value);

  function formatPdgValue(val: PdgValue): string {
    if (val.kind === 'exact') {
      return val.value.toFixed(6).replace(/0+$/, '').replace(/\.$/, '');
    }
    if (val.kind === 'symmetric') {
      const central = val.value.toFixed(6).replace(/0+$/, '').replace(/\.$/, '');
      const error = val.error.toExponential(2);
      return `${central} ± ${error}`;
    }
    const central = val.value.toFixed(6).replace(/0+$/, '').replace(/\.$/, '');
    return `${central} (+${val.error.plus.toExponential(2)})(-${val.error.minus.toExponential(2)})`;
  }
</script>

<span class="measured-value">
  {displayValue}
  {#if showSource}
    <span class="source-badge">{value.kind}</span>
  {/if}
</span>

<style>
  .measured-value {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
  }

  .source-badge {
    font-size: 0.75rem;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--surface-variant, #f0f0f0);
    color: var(--on-surface-variant, #666);
    white-space: nowrap;
  }
</style>
