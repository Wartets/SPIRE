<script lang="ts">
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";

  export let minValue = 0;
  export let maxValue = 0;
  export let centralValue: number | null = null;
  export let minusError = 0;
  export let plusError = 0;
  export let sigmaMultiplier = 1;
  export let disabled = false;
  export let onSnap: (() => void) | null = null;

  const sigmaChoices = [1, 2, 5];
</script>

<div class="asym-bounds" aria-label="Asymmetric bounds control">
  <div class="row">
    <label>
      <span>Min</span>
      <SpireNumberInput bind:value={minValue} step={0.1} min={0} ariaLabel="Minimum value" />
    </label>
    <label>
      <span>Max</span>
      <SpireNumberInput bind:value={maxValue} step={0.1} min={0} ariaLabel="Maximum value" />
    </label>
  </div>

  <div class="actions">
    <label>
      <span>σ</span>
      <select bind:value={sigmaMultiplier}>
        {#each sigmaChoices as option}
          <option value={option}>{option}σ</option>
        {/each}
      </select>
    </label>
    <button type="button" on:click={() => onSnap?.()} disabled={disabled}>PDG Bounds</button>
  </div>

  {#if centralValue != null}
    <p class="meta">central {centralValue.toPrecision(6)} · -{minusError.toPrecision(3)} / +{plusError.toPrecision(3)}</p>
  {/if}
</div>

<style>
  .asym-bounds {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.4rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  label > span {
    color: var(--fg-secondary);
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-family: var(--font-mono);
  }

  .actions {
    display: flex;
    align-items: end;
    gap: 0.45rem;
  }

  select,
  button {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    padding: 0.2rem 0.4rem;
  }

  button:hover:not(:disabled) {
    border-color: var(--border-focus);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .meta {
    margin: 0;
    color: var(--fg-secondary);
    font-size: 0.66rem;
    font-family: var(--font-mono);
  }
</style>
