<script lang="ts">
  import { onDestroy } from "svelte";

  export let value = 0;
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step = 1;
  export let disabled = false;
  export let placeholder = "";
  export let inputId: string | undefined = undefined;
  export let ariaLabel: string | undefined = undefined;

  let inputText = "";
  let holdDelayTimer: ReturnType<typeof setTimeout> | null = null;
  let holdIntervalTimer: ReturnType<typeof setInterval> | null = null;

  function formatValue(next: number): string {
    if (Number.isInteger(step)) {
      return String(Math.round(next));
    }
    return String(next);
  }

  $: if (document?.activeElement?.id !== inputId) {
    inputText = formatValue(value);
  }

  function clamp(next: number): number {
    let output = next;
    if (typeof min === "number") output = Math.max(min, output);
    if (typeof max === "number") output = Math.min(max, output);
    return output;
  }

  function parseInputText(): number | null {
    const trimmed = inputText.trim();
    if (trimmed.length === 0) return null;
    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed)) return null;
    return parsed;
  }

  function commitInput(): void {
    const parsed = parseInputText();
    if (parsed === null) {
      inputText = formatValue(value);
      return;
    }
    value = clamp(parsed);
    inputText = formatValue(value);
  }

  function stepBy(direction: 1 | -1, shiftKey: boolean): void {
    if (disabled) return;
    const multiplier = shiftKey ? 10 : 1;
    const delta = direction * step * multiplier;
    value = clamp(value + delta);
    inputText = formatValue(value);
  }

  function clearHoldTimers(): void {
    if (holdDelayTimer !== null) {
      clearTimeout(holdDelayTimer);
      holdDelayTimer = null;
    }
    if (holdIntervalTimer !== null) {
      clearInterval(holdIntervalTimer);
      holdIntervalTimer = null;
    }
  }

  function startHold(direction: 1 | -1, event: MouseEvent): void {
    if (disabled) return;
    stepBy(direction, event.shiftKey);
    clearHoldTimers();
    holdDelayTimer = setTimeout(() => {
      holdIntervalTimer = setInterval(() => {
        stepBy(direction, event.shiftKey);
      }, 50);
    }, 300);
  }

  function stopHold(): void {
    clearHoldTimers();
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      commitInput();
    } else if (event.key === "Escape") {
      event.preventDefault();
      inputText = formatValue(value);
    }
  }

  onDestroy(() => {
    clearHoldTimers();
  });
</script>

<div class="spire-number-input" class:disabled>
  <input
    id={inputId}
    class="field"
    type="text"
    bind:value={inputText}
    {disabled}
    {placeholder}
    aria-label={ariaLabel}
    inputmode="decimal"
    on:blur={commitInput}
    on:keydown={onKeyDown}
  />

  <div class="controls">
    <button
      type="button"
      class="chevron"
      disabled={disabled}
      aria-label="Increment value"
      on:mousedown={(e) => startHold(1, e)}
      on:mouseup={stopHold}
      on:mouseleave={stopHold}
    >
      <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
        <path d="M1 5.5L4 2.5L7 5.5" fill="none" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
    <button
      type="button"
      class="chevron"
      disabled={disabled}
      aria-label="Decrement value"
      on:mousedown={(e) => startHold(-1, e)}
      on:mouseup={stopHold}
      on:mouseleave={stopHold}
    >
      <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
        <path d="M1 2.5L4 5.5L7 2.5" fill="none" stroke="currentColor" stroke-width="1.2" />
      </svg>
    </button>
  </div>
</div>

<style>
  .spire-number-input {
    display: grid;
    grid-template-columns: 1fr 1rem;
    width: 100%;
    border: 1px solid var(--border-color, var(--border, #333333));
    background: var(--bg-input, var(--bg-inset, #0e0e0e));
    color: var(--fg-input, var(--fg-primary, #e8e8e8));
  }

  .spire-number-input.disabled {
    opacity: 0.55;
  }

  .field {
    border: none;
    outline: none;
    background: transparent;
    color: inherit;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.78rem;
    padding: 0.25rem 0.4rem;
    min-width: 0;
  }

  .controls {
    display: grid;
    grid-template-rows: 1fr 1fr;
    border-left: 1px solid var(--border-color, var(--border, #333333));
  }

  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    margin: 0;
    border: none;
    border-bottom: 1px solid var(--border-color, var(--border, #333333));
    background: var(--bg-surface, #1a1a1a);
    color: var(--fg-muted, var(--fg-secondary, #888888));
    cursor: pointer;
    min-height: 0.7rem;
  }

  .chevron:last-child {
    border-bottom: none;
  }

  .chevron:hover:not(:disabled) {
    color: var(--accent, var(--hl-symbol, #5eb8ff));
  }

  .chevron:disabled {
    cursor: not-allowed;
  }

  .spire-number-input:focus-within {
    border-color: var(--border-focus, #666666);
  }
</style>
