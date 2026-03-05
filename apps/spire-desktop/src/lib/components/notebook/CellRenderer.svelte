<!--
  SPIRE — Cell Renderer (Pluggable Dispatch)

  Generic component that dispatches to the appropriate cell type
  component based on `cell.type`.  This enables extreme modularity:
  adding a new cell type requires only a new component and a single
  branch in this renderer.

  All cell events are forwarded to the parent NotebookWidget.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CellData } from "$lib/core/domain/notebook";

  import MarkdownCell from "./MarkdownCell.svelte";
  import ScriptCell from "./ScriptCell.svelte";
  import ConfigCell from "./ConfigCell.svelte";

  export let cell: CellData;

  const dispatch = createEventDispatcher<{
    sourceChange: { id: string; source: string };
    execute: { id: string };
    delete: { id: string };
    moveUp: { id: string };
    moveDown: { id: string };
    advanceFocus: { id: string };
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
</script>

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
