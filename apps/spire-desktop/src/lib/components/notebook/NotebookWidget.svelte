<!--
  SPIRE - Notebook Widget

  Top-level container for the cell-based execution engine.
  Renders an ordered list of cells via CellRenderer, provides
  the Add Cell menu, toolbar actions (Run All, Clear Outputs,
  Reset Session), and manages focus advancement on Shift+Enter.

  State is isolated from the workspace layout store.  The notebook
  creates its own backend Rhai session on first execution.
-->
<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { CellType } from "$lib/core/domain/notebook";
  import {
    getWidgetUiSnapshot,
    setWidgetUiSnapshot,
  } from "$lib/stores/workspaceStore";
  import {
    notebookDocument,
    cellCount,
    isExecuting,
    insertCell,
    appendCell,
    removeCell,
    updateCellSource,
    moveCell,
    duplicateCell,
    executeCell,
    executeAllCells,
    clearAllOutputs,
    clearCellOutput,
    resetSession,
    destroySession,
  } from "$lib/stores/notebookDocumentStore";

  import CellRenderer from "./CellRenderer.svelte";

  // ── Cell References (for focus management) ──

  let cellRefs: Record<string, CellRenderer> = {};
  let rootEl: HTMLDivElement | null = null;

  function bindCellRef(id: string) {
    return (node: CellRenderer) => {
      cellRefs[id] = node;
    };
  }

  // ── Event Handlers ──

  function handleSourceChange(e: CustomEvent<{ id: string; source: string }>): void {
    updateCellSource(e.detail.id, e.detail.source);
  }

  function handleExecute(e: CustomEvent<{ id: string }>): void {
    executeCell(e.detail.id);
  }

  function handleDelete(e: CustomEvent<{ id: string }>): void {
    removeCell(e.detail.id);
  }

  function handleMoveUp(e: CustomEvent<{ id: string }>): void {
    const cells = $notebookDocument.cells;
    const idx = cells.findIndex((c) => c.id === e.detail.id);
    if (idx > 0) moveCell(idx, idx - 1);
  }

  function handleMoveDown(e: CustomEvent<{ id: string }>): void {
    const cells = $notebookDocument.cells;
    const idx = cells.findIndex((c) => c.id === e.detail.id);
    if (idx >= 0 && idx < cells.length - 1) moveCell(idx, idx + 1);
  }

  function handleAdvanceFocus(e: CustomEvent<{ id: string }>): void {
    const cells = $notebookDocument.cells;
    const idx = cells.findIndex((c) => c.id === e.detail.id);
    if (idx >= 0 && idx < cells.length - 1) {
      const nextId = cells[idx + 1].id;
      requestAnimationFrame(() => {
        cellRefs[nextId]?.focus();
      });
    }
  }

  async function handleRunAllAbove(e: CustomEvent<{ index: number }>): Promise<void> {
    const cells = $notebookDocument.cells;
    for (let i = 0; i < e.detail.index; i++) {
      const c = cells[i];
      if (c.type === "markdown") continue;
      const result = await executeCell(c.id);
      if (result && !result.success) break;
    }
  }

  async function handleRunAllBelow(e: CustomEvent<{ index: number }>): Promise<void> {
    const cells = $notebookDocument.cells;
    for (let i = e.detail.index; i < cells.length; i++) {
      const c = cells[i];
      if (c.type === "markdown") continue;
      const result = await executeCell(c.id);
      if (result && !result.success) break;
    }
  }

  function handleClearOutput(e: CustomEvent<{ id: string }>): void {
    clearCellOutput(e.detail.id);
  }

  function handleInsertBelow(e: CustomEvent<{ index: number; type: string }>): void {
    insertCell(e.detail.type as CellType, e.detail.index);
  }

  function handleInsertAbove(e: CustomEvent<{ index: number; type: string }>): void {
    insertCell(e.detail.type as CellType, e.detail.index - 1);
  }

  function handleDuplicate(e: CustomEvent<{ id: string }>): void {
    duplicateCell(e.detail.id);
  }

  let dragFromIndex: number | null = null;
  let dragOverIndex: number | null = null;

  function handleCellDragStart(index: number, event: DragEvent): void {
    dragFromIndex = index;
    event.dataTransfer?.setData("text/plain", String(index));
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function handleCellDragOver(index: number, event: DragEvent): void {
    event.preventDefault();
    dragOverIndex = index;
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  function handleCellDrop(index: number, event: DragEvent): void {
    event.preventDefault();
    const from = dragFromIndex;
    dragFromIndex = null;
    dragOverIndex = null;
    if (from == null || from === index) return;
    moveCell(from, index);
  }

  function clearDragState(): void {
    dragFromIndex = null;
    dragOverIndex = null;
  }

  // ── UI Snapshot Persistence ──

  let notebookUiKey = "notebook";
  let cellsScroller: HTMLDivElement | null = null;

  function persistNotebookUi(patch: Record<string, unknown>): void {
    setWidgetUiSnapshot(notebookUiKey, patch);
  }

  let scrollPersistRaf: number | null = null;

  function handleCellsScroll(): void {
    if (!cellsScroller) return;
    if (scrollPersistRaf !== null) cancelAnimationFrame(scrollPersistRaf);
    scrollPersistRaf = requestAnimationFrame(() => {
      scrollPersistRaf = null;
      persistNotebookUi({ scrollTop: cellsScroller?.scrollTop ?? 0 });
    });
  }

  // ── Add Cell ──

  let showAddMenu = false;

  function addCellOfType(type: CellType): void {
    appendCell(type);
    showAddMenu = false;
  }

  $: persistNotebookUi({ showAddMenu });

  onMount(async () => {
    const canvasWidgetId = rootEl?.closest("[data-canvas-item-id]")?.getAttribute("data-canvas-item-id");
    if (canvasWidgetId) {
      notebookUiKey = `notebook:${canvasWidgetId}`;
    }

    const snapshot = getWidgetUiSnapshot<{ scrollTop?: number; showAddMenu?: boolean }>(
      notebookUiKey,
    );
    if (!snapshot) return;

    if (typeof snapshot.showAddMenu === "boolean") {
      showAddMenu = snapshot.showAddMenu;
    }

    await tick();
    if (cellsScroller && typeof snapshot.scrollTop === "number") {
      cellsScroller.scrollTop = snapshot.scrollTop;
    }
  });

  // ── Cleanup ──

  onDestroy(() => {
    if (scrollPersistRaf !== null) {
      cancelAnimationFrame(scrollPersistRaf);
      scrollPersistRaf = null;
    }
    destroySession();
  });
</script>

<div class="notebook-widget" bind:this={rootEl}>
  <!-- Toolbar -->
  <header class="nb-toolbar">
    <span class="nb-title">{$notebookDocument.title}</span>

    <div class="nb-toolbar-actions">
      <button
        class="nb-action"
        on:click={() => executeAllCells()}
        disabled={$isExecuting}
        use:tooltip={{ text: "Run All Cells" }}
      ><Icon name="play" size={13} /> <span>Run All</span></button>
      <button
        class="nb-action"
        on:click={() => clearAllOutputs()}
        use:tooltip={{ text: "Clear All Outputs" }}
      ><Icon name="close" size={13} /> <span>Clear</span></button>
      <button
        class="nb-action"
        on:click={() => resetSession()}
        use:tooltip={{ text: "Reset Session (clears scope and model)" }}
      ><Icon name="reset" size={13} /> <span>Reset</span></button>
    </div>

    <span class="nb-cell-count">{$cellCount} cells</span>
  </header>

  <!-- Cell List -->
  <div class="nb-cells" role="list" bind:this={cellsScroller} on:scroll={handleCellsScroll}>
    {#each $notebookDocument.cells as cell, idx (cell.id)}
      <div
        class="nb-cell-shell"
        class:drag-over={dragOverIndex === idx}
        role="listitem"
        draggable="true"
        on:dragstart={(event) => handleCellDragStart(idx, event)}
        on:dragover={(event) => handleCellDragOver(idx, event)}
        on:dragleave={() => (dragOverIndex = null)}
        on:drop={(event) => handleCellDrop(idx, event)}
        on:dragend={clearDragState}
      >
        <CellRenderer
          {cell}
          index={idx}
          totalCells={$notebookDocument.cells.length}
          bind:this={cellRefs[cell.id]}
          on:sourceChange={handleSourceChange}
          on:execute={handleExecute}
          on:delete={handleDelete}
          on:moveUp={handleMoveUp}
          on:moveDown={handleMoveDown}
          on:advanceFocus={handleAdvanceFocus}
          on:runAllAbove={handleRunAllAbove}
          on:runAllBelow={handleRunAllBelow}
          on:clearOutput={handleClearOutput}
          on:duplicate={handleDuplicate}
          on:insertAbove={handleInsertAbove}
          on:insertBelow={handleInsertBelow}
        />
      </div>
    {/each}

    {#if $notebookDocument.cells.length === 0}
      <p class="nb-empty-hint">
        No cells yet. Click <strong>+ Add Cell</strong> below to begin.
      </p>
    {/if}
  </div>

  <!-- Add Cell Bar -->
  <div class="nb-add-bar">
    <div class="nb-add-wrapper">
      <button
        class="nb-add-btn"
        on:click={() => (showAddMenu = !showAddMenu)}
      >
        + Add Cell
      </button>

      {#if showAddMenu}
        <div class="nb-add-menu">
          <button class="nb-menu-item" on:click={() => addCellOfType("markdown")}>
            <span class="nb-menu-icon" style="color: var(--hl-info, #4a9eff)"><Icon name="text" size={14} /></span>
            Markdown
          </button>
          <button class="nb-menu-item" on:click={() => addCellOfType("script")}>
            <span class="nb-menu-icon" style="color: var(--hl-success, #50fa7b)"><Icon name="play" size={14} /></span>
            Script
          </button>
          <button class="nb-menu-item" on:click={() => addCellOfType("config")}>
            <span class="nb-menu-icon" style="color: var(--hl-warning, #f1fa8c)"><Icon name="settings" size={14} /></span>
            Config
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .notebook-widget {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    font-family: var(--font-mono, "Fira Code", monospace);
  }

  /* ── Toolbar ── */

  .nb-toolbar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--color-bg-inset);
    border-bottom: 1px solid var(--border, #333);
    flex-shrink: 0;
  }

  .nb-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--fg-accent, var(--color-text-primary));
    letter-spacing: 0.04em;
  }

  .nb-toolbar-actions {
    display: flex;
    gap: 0.3rem;
    margin-left: 0.5rem;
  }

  .nb-action {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.6rem;
    font-family: inherit;
    cursor: pointer;
    padding: 0.1rem 0.4rem;
  }

  .nb-action:hover:not(:disabled) {
    color: var(--color-text-primary);
    border-color: var(--fg-secondary, var(--color-text-muted));
  }

  .nb-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .nb-cell-count {
    margin-left: auto;
    font-size: 0.55rem;
    color: var(--fg-secondary, var(--color-text-muted));
  }

  /* ── Cell List ── */

  .nb-cells {
    flex: 1;
    overflow-y: auto;
    padding: 0.4rem;
    min-height: 0;
  }

  .nb-empty-hint {
    text-align: center;
    color: var(--fg-secondary, var(--color-text-muted));
    font-style: italic;
    font-size: 0.75rem;
    margin-top: 2rem;
  }

  .nb-cell-shell {
    border-radius: 6px;
  }

  .nb-cell-shell.drag-over {
    outline: 1px dashed var(--hl-symbol, #4a9eff);
    outline-offset: 2px;
  }

  /* ── Add Cell Bar ── */

  .nb-add-bar {
    flex-shrink: 0;
    padding: 0.25rem 0.4rem;
    border-top: 1px solid var(--border, #333);
    background: var(--color-bg-inset);
  }

  .nb-add-wrapper {
    position: relative;
    display: inline-block;
  }

  .nb-add-btn {
    background: none;
    border: 1px dashed var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.65rem;
    font-family: inherit;
    cursor: pointer;
    padding: 0.15rem 0.6rem;
  }

  .nb-add-btn:hover {
    color: var(--color-text-primary);
    border-color: var(--fg-secondary, var(--color-text-muted));
  }

  .nb-add-menu {
    position: absolute;
    bottom: 100%;
    left: 0;
    margin-bottom: 0.2rem;
    background: var(--bg-surface, #1a1a2e);
    border: 1px solid var(--border, #333);
    display: flex;
    flex-direction: column;
    min-width: 8rem;
    z-index: 10;
  }

  .nb-menu-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.5rem;
    background: none;
    border: none;
    color: var(--color-text-primary);
    font-size: 0.68rem;
    font-family: inherit;
    cursor: pointer;
    text-align: left;
  }

  .nb-menu-item:hover {
    background: var(--color-bg-inset);
  }

  .nb-menu-icon {
    font-size: 0.8rem;
    width: 1rem;
    text-align: center;
  }
</style>
