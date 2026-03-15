<!--
  SPIRE - Cell Renderer (Pluggable Dispatch)

  Generic component that dispatches to the appropriate cell type
  component based on `cell.type`.  This enables extreme modularity:
  adding a new cell type requires only a new component and a single
  branch in this renderer.

  All cell events are forwarded to the parent NotebookWidget.
  Right-click shows a context-sensitive menu via the global store.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { menuAction, menuSeparator, menuSubmenu } from "$lib/types/menu";
  import type { ContextMenuItem } from "$lib/types/menu";
  import { longpress } from "$lib/actions/longpress";

  import MarkdownCell from "./MarkdownCell.svelte";
  import ScriptCell from "./ScriptCell.svelte";
  import ConfigCell from "./ConfigCell.svelte";

  export let cell: CellData;
  /** Zero-based index of this cell in the notebook (for Run Above / Below). */
  export let index: number = 0;
  /** Total cell count (for boundary checks). */
  export let totalCells: number = 1;

  const dispatch = createEventDispatcher<{
    sourceChange: { id: string; source: string };
    execute: { id: string };
    delete: { id: string };
    moveUp: { id: string };
    moveDown: { id: string };
    advanceFocus: { id: string };
    runAllAbove: { index: number };
    runAllBelow: { index: number };
    clearOutput: { id: string };
    insertBelow: { index: number; type: string };
  }>();

  let innerRef: ScriptCell | ConfigCell | undefined;

  function onSourceChange(e: CustomEvent<string>): void {
    dispatch("sourceChange", { id: cell.id, source: e.detail });
  }

  function onExecute(): void {
    dispatch("execute", { id: cell.id });
  }

  function onDelete(): void {
    dispatch("delete", { id: cell.id });
  }

  function onMoveUp(): void {
    dispatch("moveUp", { id: cell.id });
  }

  function onMoveDown(): void {
    dispatch("moveDown", { id: cell.id });
  }

  function onAdvanceFocus(): void {
    dispatch("advanceFocus", { id: cell.id });
  }

  /** Focus the inner editor (used by parent for Shift+Enter advance). */
  export function focus(): void {
    if (innerRef && "focus" in innerRef) {
      innerRef.focus();
    }
  }

  // ── Context Menu ──

  function buildContextItems(): ContextMenuItem[] {
    const items: ContextMenuItem[] = [];

    // Execution commands (only for executable cells)
    if (cell.type !== "markdown") {
      items.push(
        menuAction("ctx-run-cell", "▶  Run Cell", () => dispatch("execute", { id: cell.id }), { shortcut: "Shift+Enter" }),
      );
      if (index > 0) {
        items.push(
          menuAction("ctx-run-above", "▲  Run All Above", () => dispatch("runAllAbove", { index })),
        );
      }
      if (index < totalCells - 1) {
        items.push(
          menuAction("ctx-run-below", "▼  Run All Below", () => dispatch("runAllBelow", { index })),
        );
      }
      items.push(menuSeparator());
    }

    // Output management
    if (cell.type !== "markdown" && cell.lastResult) {
      items.push(
        menuAction("ctx-clear-output", "⌧  Clear Output", () => dispatch("clearOutput", { id: cell.id })),
        menuSeparator(),
      );
    }

    // Insert submenu
    items.push(
      menuSubmenu("ctx-insert-below", "Insert Cell Below", [
        menuAction("ctx-ins-script", "▶  Script", () => dispatch("insertBelow", { index, type: "script" })),
        menuAction("ctx-ins-markdown", "¶  Markdown", () => dispatch("insertBelow", { index, type: "markdown" })),
        menuAction("ctx-ins-config", "⚙  Config", () => dispatch("insertBelow", { index, type: "config" })),
      ]),
    );

    items.push(menuSeparator());

    // Movement
    if (index > 0) {
      items.push(menuAction("ctx-move-up", "↑  Move Up", () => dispatch("moveUp", { id: cell.id })));
    }
    if (index < totalCells - 1) {
      items.push(menuAction("ctx-move-down", "↓  Move Down", () => dispatch("moveDown", { id: cell.id })));
    }

    items.push(menuSeparator());

    // Destructive
    items.push(
      menuAction("ctx-delete-cell", "✕  Delete Cell", () => dispatch("delete", { id: cell.id })),
    );

    return items;
  }

  function openContextAt(x: number, y: number): void {
    showContextMenu(x, y, buildContextItems());
  }

  function handleContextMenu(event: MouseEvent): void {
    if (event.shiftKey) return;        // Shift bypass → native menu
    event.preventDefault();
    event.stopPropagation();

    openContextAt(event.clientX, event.clientY);
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="cell-renderer-wrapper"
  on:contextmenu={handleContextMenu}
  use:longpress={{
    duration: 480,
    moveTolerance: 12,
    onLongPress: (detail) => openContextAt(detail.x, detail.y),
  }}
>
{#if cell.type === "markdown"}
  <MarkdownCell
    {cell}
    on:sourceChange={onSourceChange}
    on:delete={onDelete}
    on:moveUp={onMoveUp}
    on:moveDown={onMoveDown}
  />
{:else if cell.type === "script"}
  <ScriptCell
    bind:this={innerRef}
    {cell}
    on:sourceChange={onSourceChange}
    on:execute={onExecute}
    on:delete={onDelete}
    on:moveUp={onMoveUp}
    on:moveDown={onMoveDown}
    on:advanceFocus={onAdvanceFocus}
  />
{:else if cell.type === "config"}
  <ConfigCell
    bind:this={innerRef}
    {cell}
    on:sourceChange={onSourceChange}
    on:execute={onExecute}
    on:delete={onDelete}
    on:moveUp={onMoveUp}
    on:moveDown={onMoveDown}
    on:advanceFocus={onAdvanceFocus}
  />
{:else}
  <p style="color: var(--hl-error, #ff5555); padding: 0.5rem;">
    Unknown cell type: {cell.type}
  </p>
{/if}
</div>

