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
    --slider-track-bg: var(--bg-inset, #0e0e0e);
    --slider-border: var(--border-color, var(--border, #333333));
    --slider-accent: var(--accent, var(--hl-symbol, #5eb8ff));
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
    width: 12px;
    height: 12px;
    border-radius: 50%;
    margin-top: -6px;
    border: 1px solid var(--slider-border);
    background: var(--bg-surface, #1a1a1a);
  }

  .spire-slider::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1px solid var(--slider-border);
    background: var(--bg-surface, #1a1a1a);
  }

  .spire-slider:focus-visible {
    outline: 1px solid var(--border-focus, #666666);
    outline-offset: 2px;
  }
</style>
