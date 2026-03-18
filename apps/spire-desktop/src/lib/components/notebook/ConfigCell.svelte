<!--
  SPIRE - Config Cell

  TOML editor cell for loading theoretical models into the notebook
  session.  Parses the TOML content on the backend, builds the model,
  and stores it in the session scope for subsequent script cells.

  Shift+Enter executes the cell and advances focus.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";
  import { tooltip } from "$lib/actions/tooltip";
  import Icon from "$lib/components/ui/Icon.svelte";

  export let cell = {} as CellData;

  const dispatch = createEventDispatcher<{
    sourceChange: string;
    execute: void;
    delete: void;
    moveUp: void;
    moveDown: void;
    duplicate: void;
    insertAbove: string;
    insertBelow: string;
    advanceFocus: void;
  }>();

  let textareaEl: HTMLTextAreaElement | undefined;

  function autoSizeEditor(): void {
    if (!textareaEl) return;
    textareaEl.style.height = "0px";
    textareaEl.style.height = `${Math.max(56, textareaEl.scrollHeight)}px`;
  }

  function handleInput(e: Event): void {
    const target = e.target as HTMLTextAreaElement;
    dispatch("sourceChange", target.value);
    autoSizeEditor();
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter" && e.shiftKey) {
      e.preventDefault();
      dispatch("execute");
      dispatch("advanceFocus");
      return;
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      dispatch("execute");
      return;
    }
    if (e.key === "Enter" && e.altKey) {
      e.preventDefault();
      dispatch("execute");
      dispatch("insertBelow", "config");
      dispatch("advanceFocus");
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d") {
      e.preventDefault();
      dispatch("duplicate");
      return;
    }
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

  onMount(() => {
    autoSizeEditor();
  });

  $: if (textareaEl) {
    requestAnimationFrame(() => autoSizeEditor());
  }

  $: executionLabel =
    cell.executionCount != null ? `[${cell.executionCount}]` : "[ ]";

  $: hasError = cell.lastResult != null && !cell.lastResult.success;
</script>

<div class="config-cell" class:error={hasError} class:running={cell.running}>
  <div class="cc-toolbar">
    <span class="cc-badge">Config</span>
    <span class="cc-exec-count" use:tooltip={{ text: "Execution count" }}>{executionLabel}</span>
    <button
      class="cc-run"
      on:click={() => dispatch("execute")}
      disabled={cell.running}
      use:tooltip={{ text: "Load Model (Shift+Enter)" }}
    >
      {#if cell.running}
        ...
      {:else}
        <Icon name="play" size={14} />
      {/if}
    </button>
    <div class="cc-actions">
      <button class="cc-btn" on:click={() => dispatch("insertAbove", "config")} use:tooltip={{ text: "Insert Config Above" }}>+↑</button>
      <button class="cc-btn" on:click={() => dispatch("insertBelow", "config")} use:tooltip={{ text: "Insert Config Below" }}>+↓</button>
      <button class="cc-btn" on:click={() => dispatch("duplicate")} use:tooltip={{ text: "Duplicate Cell (Ctrl+D)" }}>⎘</button>
      <button class="cc-btn" on:click={() => dispatch("moveUp")} use:tooltip={{ text: "Move Up" }}><Icon name="up" size={13} /></button>
      <button class="cc-btn" on:click={() => dispatch("moveDown")} use:tooltip={{ text: "Move Down" }}><Icon name="down" size={13} /></button>
      <button class="cc-btn cc-delete" on:click={() => dispatch("delete")} use:tooltip={{ text: "Delete Cell" }}><Icon name="close" size={13} /></button>
    </div>
  </div>

  <textarea
    bind:this={textareaEl}
    class="cc-editor"
    value={cell.source}
    on:input={handleInput}
    on:keydown={handleKeydown}
    spellcheck="false"
    placeholder="# TOML model configuration..."
  ></textarea>

  {#if cell.lastResult}
    <div class="cc-output" class:cc-output-error={hasError}>
      {#if cell.lastResult.error}
        <pre class="cc-error-text">{cell.lastResult.error}</pre>
      {:else if cell.lastResult.output}
        <pre class="cc-success-text">{cell.lastResult.output}</pre>
      {:else}
        <span class="cc-success-text">Model loaded successfully.</span>
      {/if}
      <span class="cc-timing">{cell.lastResult.duration_ms.toFixed(1)} ms</span>
    </div>
  {/if}
</div>

<style>
  .config-cell {
    border: 1px solid var(--border, #333);
    border-left: 3px solid var(--hl-warning, #f1fa8c);
    background: var(--bg-surface, #1a1a2e);
    margin-bottom: 0.25rem;
  }

  .config-cell.running {
    border-left-color: var(--hl-info, #4a9eff);
  }

  .config-cell.error {
    border-left-color: var(--hl-error, #ff5555);
  }

  .cc-toolbar {
    display: flex;
    align-items: center;
    padding: 0.15rem 0.4rem;
    background: var(--color-bg-inset);
    border-bottom: 1px solid var(--border, #333);
    gap: 0.3rem;
  }

  .cc-badge {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--hl-warning, #f1fa8c);
    font-weight: 600;
  }

  .cc-exec-count {
    font-size: 0.6rem;
    color: var(--fg-secondary, var(--color-text-muted));
    font-family: var(--font-mono, "Fira Code", monospace);
    min-width: 2rem;
  }

  .cc-run {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--hl-warning, #f1fa8c);
    font-size: 0.65rem;
    cursor: pointer;
    padding: 0.05rem 0.35rem;
    line-height: 1.2;
  }

  .cc-run:hover:not(:disabled) {
    background: var(--color-bg-inset);
    color: var(--fg-accent, var(--color-text-primary));
  }

  .cc-run:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .cc-actions {
    margin-left: auto;
    display: flex;
    gap: 0.15rem;
  }

  .cc-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.6rem;
    cursor: pointer;
    min-width: 1.35rem;
    padding: 0.05rem 0.2rem;
    line-height: 1.2;
    text-align: center;
    border-radius: 4px;
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  .cc-btn:hover {
    color: var(--color-text-primary);
    border-color: var(--border, #333);
  }

  .cc-delete:hover {
    color: var(--hl-error, #ff5555);
  }

  .cc-editor {
    width: 100%;
    min-height: 3rem;
    padding: 0.4rem;
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    border: none;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.75rem;
    line-height: 1.5;
    resize: none;
    overflow: hidden;
    outline: none;
    box-sizing: border-box;
    tab-size: 2;
    field-sizing: content;
  }

  .cc-output {
    border-top: 1px solid var(--border, #333);
    padding: 0.3rem 0.4rem;
    background: var(--color-bg-base);
    position: relative;
  }

  .cc-output-error {
    background: rgba(255, 85, 85, 0.05);
  }

  .cc-error-text {
    color: var(--hl-error, #ff5555);
    margin: 0;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .cc-success-text {
    color: var(--hl-success, #50fa7b);
    margin: 0;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
    white-space: pre-wrap;
  }

  .cc-timing {
    position: absolute;
    bottom: 0.15rem;
    right: 0.3rem;
    font-size: 0.5rem;
    color: var(--fg-secondary, var(--color-text-muted));
  }
</style>
