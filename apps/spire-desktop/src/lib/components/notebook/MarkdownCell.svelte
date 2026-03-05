<!--
  SPIRE — Markdown Cell

  Dual-mode cell for rich-text annotation.  In edit mode, displays
  a raw Markdown textarea.  In view mode, renders a lightweight
  HTML preview.  Click to edit; blur or Escape to preview.

  Not executable — purely documentary.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";

  export let cell: CellData;

  const dispatch = createEventDispatcher<{
    sourceChange: string;
    delete: void;
    moveUp: void;
    moveDown: void;
  }>();

  let editing = false;
  let textareaEl: HTMLTextAreaElement | undefined;

  function startEditing(): void {
    editing = true;
    requestAnimationFrame(() => textareaEl?.focus());
  }

  function stopEditing(): void {
    editing = false;
  }

  function handleInput(e: Event): void {
    const target = e.target as HTMLTextAreaElement;
    dispatch("sourceChange", target.value);
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      stopEditing();
    }
  }

  // ── Lightweight Markdown → HTML ──

  /**
   * Minimal Markdown renderer.  Handles headings, bold, italic,
   * inline code, code blocks, and line breaks.  No external
   * dependencies required.
   */
  function renderMarkdown(src: string): string {
    let html = src
      // Escape raw HTML
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      // Fenced code blocks
      .replace(/```([\s\S]*?)```/g, "<pre><code>$1</code></pre>")
      // Headings (### before ## before #)
      .replace(/^######\s+(.*)$/gm, "<h6>$1</h6>")
      .replace(/^#####\s+(.*)$/gm, "<h5>$1</h5>")
      .replace(/^####\s+(.*)$/gm, "<h4>$1</h4>")
      .replace(/^###\s+(.*)$/gm, "<h3>$1</h3>")
      .replace(/^##\s+(.*)$/gm, "<h2>$1</h2>")
      .replace(/^#\s+(.*)$/gm, "<h1>$1</h1>")
      // Bold and italic
      .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      .replace(/\*(.+?)\*/g, "<em>$1</em>")
      // Inline code
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      // Unordered lists
      .replace(/^\s*[-*]\s+(.*)$/gm, "<li>$1</li>")
      // Horizontal rule
      .replace(/^---$/gm, "<hr/>")
      // Line breaks → paragraphs
      .replace(/\n\n+/g, "</p><p>")
      .replace(/\n/g, "<br/>");

    // Wrap list items
    html = html.replace(/(<li>.*?<\/li>)+/gs, "<ul>$&</ul>");

    return `<p>${html}</p>`;
  }

  $: renderedHtml = renderMarkdown(cell.source);
</script>

<div class="md-cell" class:editing>
  <div class="md-toolbar">
    <span class="md-badge">Markdown</span>
    <div class="md-actions">
      <button class="md-btn" on:click={() => dispatch("moveUp")} title="Move Up">↑</button>
      <button class="md-btn" on:click={() => dispatch("moveDown")} title="Move Down">↓</button>
      <button class="md-btn md-delete" on:click={() => dispatch("delete")} title="Delete Cell">✕</button>
    </div>
  </div>

  {#if editing}
    <textarea
      bind:this={textareaEl}
      class="md-editor"
      value={cell.source}
      on:input={handleInput}
      on:blur={stopEditing}
      on:keydown={handleKeydown}
      spellcheck="false"
    ></textarea>
  {:else}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="md-preview"
      on:click={startEditing}
    >
      {@html renderedHtml}
    </div>
  {/if}
</div>

<style>
  .md-cell {
    border: 1px solid var(--border, #333);
    border-left: 3px solid var(--hl-info, #4a9eff);
    background: var(--bg-surface, #1a1a2e);
    margin-bottom: 0.25rem;
  }

  .md-toolbar {
    display: flex;
    align-items: center;
    padding: 0.15rem 0.4rem;
    background: var(--bg-inset, #0e0e0e);
    border-bottom: 1px solid var(--border, #333);
    gap: 0.3rem;
  }

  .md-badge {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--hl-info, #4a9eff);
    font-weight: 600;
  }

  .md-actions {
    margin-left: auto;
    display: flex;
    gap: 0.15rem;
  }

  .md-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary, #888);
    font-size: 0.6rem;
    cursor: pointer;
    padding: 0 0.2rem;
    line-height: 1.2;
  }

  .md-btn:hover {
    color: var(--fg-primary, #e8e8e8);
    border-color: var(--border, #333);
  }

  .md-delete:hover {
    color: var(--hl-error, #ff5555);
  }

  .md-editor {
    width: 100%;
    min-height: 5rem;
    padding: 0.4rem;
    background: var(--bg-primary, #121212);
    color: var(--fg-primary, #e8e8e8);
    border: none;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.75rem;
    resize: vertical;
    outline: none;
    box-sizing: border-box;
  }

  .md-preview {
    padding: 0.5rem 0.6rem;
    cursor: text;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--fg-primary, #e8e8e8);
    min-height: 1.5rem;
  }

  .md-preview :global(h1),
  .md-preview :global(h2),
  .md-preview :global(h3),
  .md-preview :global(h4),
  .md-preview :global(h5),
  .md-preview :global(h6) {
    margin: 0.3rem 0 0.15rem;
    color: var(--fg-accent, #fff);
  }

  .md-preview :global(h1) { font-size: 1.2rem; }
  .md-preview :global(h2) { font-size: 1.0rem; }
  .md-preview :global(h3) { font-size: 0.9rem; }

  .md-preview :global(code) {
    background: var(--bg-inset, #0e0e0e);
    padding: 0.1rem 0.25rem;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  .md-preview :global(pre) {
    background: var(--bg-inset, #0e0e0e);
    padding: 0.4rem;
    overflow-x: auto;
    margin: 0.3rem 0;
  }

  .md-preview :global(strong) {
    color: var(--fg-accent, #fff);
  }

  .md-preview :global(hr) {
    border: none;
    border-top: 1px solid var(--border, #333);
    margin: 0.4rem 0;
  }

  .md-preview :global(ul) {
    padding-left: 1.2rem;
    margin: 0.2rem 0;
  }

  .md-preview :global(p) {
    margin: 0.2rem 0;
  }
</style>
