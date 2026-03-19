<!--
  SPIRE - Markdown Cell

  Dual-mode cell for rich-text annotation.  In edit mode, displays
  a raw Markdown textarea.  In view mode, renders a lightweight
  HTML preview.  Click to edit; blur or Escape to preview.

  Not executable - purely documentary.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";
  import { tooltip } from "$lib/actions/tooltip";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { executeWorkerTask } from "$lib/workers/executeWorkerTask";
  import "katex/dist/katex.min.css";

  type KatexRenderResponse = {
    ok: boolean;
    html?: string;
    error?: string;
  };

  export let cell: CellData;

  const dispatch = createEventDispatcher<{
    sourceChange: string;
    delete: void;
    moveUp: void;
    moveDown: void;
    duplicate: void;
    insertAbove: string;
    insertBelow: string;
  }>();

  let editing = false;
  let textareaEl: HTMLTextAreaElement | undefined;

  function autoSizeEditor(): void {
    if (!textareaEl) return;
    textareaEl.style.height = "0px";
    textareaEl.style.height = `${Math.max(60, textareaEl.scrollHeight)}px`;
  }

  function startEditing(): void {
    editing = true;
    requestAnimationFrame(() => {
      textareaEl?.focus();
      autoSizeEditor();
    });
  }

  function stopEditing(): void {
    editing = false;
  }

  function handleInput(e: Event): void {
    const target = e.target as HTMLTextAreaElement;
    dispatch("sourceChange", target.value);
    autoSizeEditor();
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      stopEditing();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      stopEditing();
      return;
    }
    if (e.altKey && e.key === "Enter") {
      e.preventDefault();
      dispatch("insertBelow", "markdown");
      stopEditing();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d") {
      e.preventDefault();
      dispatch("duplicate");
      return;
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

  async function renderMarkdownWithMath(src: string): Promise<string> {
    const mathTokens: Array<{ token: string; expr: string; display: boolean }> = [];

    let withTokens = src.replace(/\$\$([\s\S]+?)\$\$/g, (_, expr: string) => {
      const token = `@@MATH_BLOCK_${mathTokens.length}@@`;
      mathTokens.push({ token, expr: expr.trim(), display: true });
      return token;
    });

    withTokens = withTokens.replace(/\$([^$\n]+?)\$/g, (_, expr: string) => {
      const token = `@@MATH_INLINE_${mathTokens.length}@@`;
      mathTokens.push({ token, expr: expr.trim(), display: false });
      return token;
    });

    let html = renderMarkdown(withTokens);
    if (mathTokens.length === 0) return html;

    for (const item of mathTokens) {
      let replacement = item.expr;
      try {
        const response = await executeWorkerTask<
          { expression: string; displayMode: boolean },
          KatexRenderResponse
        >(
          new URL("$lib/workers/katexRender.worker.ts", import.meta.url),
          { expression: item.expr, displayMode: item.display },
        );
        if (response.ok && response.html) {
          replacement = response.html;
        }
      } catch {
        replacement = item.expr;
      }
      html = html.replaceAll(item.token, replacement);
    }

    return html;
  }

  let renderedHtml = "";
  let renderSeq = 0;

  $: {
    const seq = ++renderSeq;
    void renderMarkdownWithMath(cell.source).then((html) => {
      if (seq === renderSeq) renderedHtml = html;
    });
  }
  $: if (editing && textareaEl) {
    requestAnimationFrame(() => autoSizeEditor());
  }
</script>

<div class="md-cell" class:editing>
  <div class="md-toolbar">
    <span class="md-badge">Markdown</span>
    <div class="md-actions">
      <button class="md-btn" on:click={() => dispatch("insertAbove", "markdown")} use:tooltip={{ text: "Insert Markdown Above" }}>+↑</button>
      <button class="md-btn" on:click={() => dispatch("insertBelow", "markdown")} use:tooltip={{ text: "Insert Markdown Below" }}>+↓</button>
      <button class="md-btn" on:click={() => dispatch("duplicate")} use:tooltip={{ text: "Duplicate Cell (Ctrl+D)" }}>⎘</button>
      <button class="md-btn" on:click={() => dispatch("moveUp")} use:tooltip={{ text: "Move Up" }}><Icon name="up" size={12} /></button>
      <button class="md-btn" on:click={() => dispatch("moveDown")} use:tooltip={{ text: "Move Down" }}><Icon name="down" size={12} /></button>
      <button class="md-btn md-delete" on:click={() => dispatch("delete")} use:tooltip={{ text: "Delete Cell" }}><Icon name="close" size={12} /></button>
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
    background: var(--color-bg-inset);
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
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.6rem;
    cursor: pointer;
    min-width: 1.35rem;
    padding: 0.05rem 0.2rem;
    line-height: 1.2;
    border-radius: 4px;
    text-align: center;
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  .md-btn:hover {
    color: var(--color-text-primary);
    border-color: var(--border, #333);
  }

  .md-delete:hover {
    color: var(--hl-error, #ff5555);
  }

  .md-editor {
    width: 100%;
    min-height: 3.25rem;
    padding: 0.4rem;
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    border: none;
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.75rem;
    resize: none;
    overflow: hidden;
    outline: none;
    box-sizing: border-box;
    field-sizing: content;
  }

  .md-preview {
    padding: 0.5rem 0.6rem;
    cursor: text;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--color-text-primary);
    min-height: 1.5rem;
  }

  .md-preview :global(h1),
  .md-preview :global(h2),
  .md-preview :global(h3),
  .md-preview :global(h4),
  .md-preview :global(h5),
  .md-preview :global(h6) {
    margin: 0.3rem 0 0.15rem;
    color: var(--fg-accent, var(--color-text-primary));
  }

  .md-preview :global(h1) { font-size: 1.2rem; }
  .md-preview :global(h2) { font-size: 1.0rem; }
  .md-preview :global(h3) { font-size: 0.9rem; }

  .md-preview :global(code) {
    background: var(--color-bg-inset);
    padding: 0.1rem 0.25rem;
    font-size: 0.72rem;
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  .md-preview :global(pre) {
    background: var(--color-bg-inset);
    padding: 0.4rem;
    overflow-x: auto;
    margin: 0.3rem 0;
  }

  .md-preview :global(strong) {
    color: var(--fg-accent, var(--color-text-primary));
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
