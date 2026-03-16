<!--
  SPIRE - Script Cell

  Editable Rhai code cell with execution support.  Shows the source
  editor, a run button, execution count, and output/error display.
  Shift+Enter executes the cell and advances focus to the next cell.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";
  import { tooltip } from "$lib/actions/tooltip";

  export let cell: CellData;

  const dispatch = createEventDispatcher<{
    sourceChange: string;
    execute: void;
    delete: void;
    moveUp: void;
    moveDown: void;
    advanceFocus: void;
  }>();

  let textareaEl: HTMLTextAreaElement | undefined;

  function handleInput(e: Event): void {
    const target = e.target as HTMLTextAreaElement;
    dispatch("sourceChange", target.value);
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter" && e.shiftKey) {
      e.preventDefault();
      dispatch("execute");
      dispatch("advanceFocus");
    }
    // Tab inserts two spaces
    if (e.key === "Tab") {
      e.preventDefault();
      const ta = e.target as HTMLTextAreaElement;
      const start = ta.selectionStart;
      const end = ta.selectionEnd;
      const value = ta.value;
      const newValue = value.substring(0, start) + "  " + value.substring(end);
      dispatch("sourceChange", newValue);
      requestAnimationFrame(() => {
        ta.selectionStart = ta.selectionEnd = start + 2;
      });
    }
  }

  /** Focus this cell's textarea (called by parent). */
  export function focus(): void {
    textareaEl?.focus();
  }

  $: executionLabel =
    cell.executionCount != null ? `[${cell.executionCount}]` : "[ ]";

  $: hasError = cell.lastResult != null && !cell.lastResult.success;
  $: hasOutput =
    cell.lastResult != null &&
    (cell.lastResult.output.length > 0 ||
      cell.lastResult.return_value != null);
</script>

<div class="script-cell" class:error={hasError} class:running={cell.running}>
  <div class="sc-toolbar">
    <span class="sc-badge">Script</span>
    <span class="sc-exec-count" use:tooltip={{ text: "Execution count" }}>{executionLabel}</span>
    <button
      class="sc-run"
      on:click={() => dispatch("execute")}
      disabled={cell.running}
      use:tooltip={{ text: "Run Cell (Shift+Enter)" }}
    >
      {#if cell.running}
        ...
      {:else}
        ▶
      {/if}
    </button>
    <div class="sc-actions">
      <button class="sc-btn" on:click={() => dispatch("moveUp")} use:tooltip={{ text: "Move Up" }}>↑</button>
      <button class="sc-btn" on:click={() => dispatch("moveDown")} use:tooltip={{ text: "Move Down" }}>↓</button>
      <button class="sc-btn sc-delete" on:click={() => dispatch("delete")} use:tooltip={{ text: "Delete Cell" }}>✕</button>
    </div>
  </div>

  <textarea
    bind:this={textareaEl}
    class="sc-editor"
    value={cell.source}
    on:input={handleInput}
    on:keydown={handleKeydown}
    spellcheck="false"
    placeholder="// Rhai script..."
  ></textarea>

  {#if cell.lastResult}
    <div class="sc-output" class:sc-output-error={hasError}>
      {#if cell.lastResult.error}
        <pre class="sc-error-text">{cell.lastResult.error}</pre>
      {/if}
      {#if cell.lastResult.output}
        <pre class="sc-stdout">{cell.lastResult.output}</pre>
      {/if}
      {#if cell.lastResult.return_value != null}
        <pre class="sc-return">{JSON.stringify(cell.lastResult.return_value, null, 2)}</pre>
      {/if}
      <span class="sc-timing">{cell.lastResult.duration_ms.toFixed(1)} ms</span>
    </div>
  {/if}
</div>

<style>
  .script-cell {
    border: 1px solid var(--border, #333);
    border-left: 3px solid var(--hl-success, #50fa7b);
    background: var(--bg-surface, #1a1a2e);
    margin-bottom: 0.25rem;
  }

  .script-cell.running {
    border-left-color: var(--hl-warning, #f1fa8c);
  }

  .script-cell.error {
    border-left-color: var(--hl-error, #ff5555);
  }

  .sc-toolbar {
    display: flex;
    align-items: center;
    padding: 0.15rem 0.4rem;
    background: var(--color-bg-inset);
    border-bottom: 1px solid var(--border, #333);
    gap: 0.3rem;
  }

  .sc-badge {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--hl-success, #50fa7b);
    font-weight: 600;
  }

  .sc-exec-count {
    font-size: 0.6rem;
    color: var(--fg-secondary, var(--color-text-muted));
    font-family: var(--font-mono, "Fira Code", monospace);
    min-width: 2rem;
  }

  .sc-run {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--hl-success, #50fa7b);
    font-size: 0.65rem;
    cursor: pointer;
    padding: 0.05rem 0.35rem;
    line-height: 1.2;
  }

  .sc-run:hover:not(:disabled) {
    background: var(--color-bg-inset);
    color: var(--fg-accent, var(--color-text-primary));
  }

  .sc-run:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sc-actions {
    margin-left: auto;
    display: flex;
    gap: 0.15rem;
  }

  .sc-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.6rem;
    cursor: pointer;
    padding: 0 0.2rem;
    line-height: 1.2;
  }

  .sc-btn:hover {
    color: var(--color-text-primary);
    border-color: var(--border, #333);
  }

  .sc-delete:hover {
    color: var(--hl-error, #ff5555);
  }

  .sc-editor {
    width: 100%;
    min-height: 4rem;
    padding: 0.4rem;
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    border: none;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.75rem;
    line-height: 1.5;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
    tab-size: 2;
  }

  .sc-output {
    border-top: 1px solid var(--border, #333);
    padding: 0.3rem 0.4rem;
    background: var(--color-bg-base);
    position: relative;
  }

  .sc-output-error {
    background: rgba(255, 85, 85, 0.05);
  }

  .sc-error-text {
    color: var(--hl-error, #ff5555);
    margin: 0;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .sc-stdout {
    color: var(--color-text-primary);
    margin: 0;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
    white-space: pre-wrap;
  }

  .sc-return {
    color: var(--hl-info, #4a9eff);
    margin: 0.15rem 0 0;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
    white-space: pre-wrap;
  }

  .sc-timing {
    position: absolute;
    bottom: 0.15rem;
    right: 0.3rem;
    font-size: 0.5rem;
    color: var(--fg-secondary, var(--color-text-muted));
  }
</style>
