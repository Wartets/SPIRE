<script lang="ts">
  export let value = 0;
  export let min = 0;
  export let max = 100;
  export let step = 1;
  export let disabled = false;
  export let inputId: string | undefined = undefined;
  export let ariaLabel: string | undefined = undefined;

  function toNumber(input: string): number {
    const parsed = Number(input);
    return Number.isFinite(parsed) ? parsed : value;
  }

  $: range = Math.max(max - min, 1e-9);
  $: clampedValue = Math.min(max, Math.max(min, value));
  $: percent = ((clampedValue - min) / range) * 100;

  function handleInput(event: Event): void {
    const next = toNumber((event.currentTarget as HTMLInputElement).value);
    value = Math.min(max, Math.max(min, next));
  }
</script>

<input
  class="spire-slider"
  id={inputId}
  type="range"
  bind:value
  {min}
  {max}
  {step}
  {disabled}
  aria-label={ariaLabel}
  style={`--percent:${percent}%;`}
  on:input={handleInput}
/>

<style>
  .spire-slider {
    width: 100%;
    height: 1rem;
    margin: 0;
    background: transparent;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    --slider-track-bg: var(--color-bg-inset);
    --slider-border: var(--color-border);
    --slider-accent: var(--color-accent);
  }

  .spire-slider:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spire-slider::-webkit-slider-runnable-track {
    height: 2px;
    border: 1px solid var(--slider-border);
    background: linear-gradient(
      to right,
      var(--slider-accent) 0%,
      var(--slider-accent) calc(var(--percent) * 1%),
      var(--slider-track-bg) calc(var(--percent) * 1%),
      var(--slider-track-bg) 100%
    );
  }

  .spire-slider::-moz-range-track {
    height: 2px;
    border: 1px solid var(--slider-border);
    background: linear-gradient(
      to right,
      var(--slider-accent) 0%,
      var(--slider-accent) calc(var(--percent) * 1%),
      var(--slider-track-bg) calc(var(--percent) * 1%),
      var(--slider-track-bg) 100%
    );
  }

  .spire-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 50%;
    margin-top: -0.375rem;
    border: 1px solid var(--slider-border);
    background: var(--color-bg-surface);
  }

  .spire-slider::-moz-range-thumb {
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 50%;
    border: 1px solid var(--slider-border);
    background: var(--color-bg-surface);
  }

  .spire-slider:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
</style>
