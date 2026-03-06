<!--
  SPIRE — Split Pane

  Generic resizable container that renders its children separated by
  draggable resize handles.  Supports both horizontal (row) and
  vertical (col) orientations.

  The children's relative sizes are controlled by the `sizes` array
  from the layout node.  Dragging a handle updates the sizes via the
  `resizePanes` reducer.

  Props:
    nodeId      — The ID of the container node (for resize updates).
    direction   — "row" (horizontal) or "col" (vertical).
    sizes       — Flex-grow proportions for each child slot.

  Slots:
    Each child should be passed via `<slot />`.  This component uses
    an indexed slot approach: the parent `LayoutRenderer` renders
    children inline.
-->
<script lang="ts">
  import { resizePanes } from "$lib/stores/layoutStore";

  export let nodeId: string;
  export let direction: "row" | "col";
  export let sizes: number[];

  let containerEl: HTMLDivElement;
  let dragging = false;
  let dragIndex = -1;
  let startPos = 0;
  let startSizes: number[] = [];

  function totalSize(): number {
    return sizes.reduce((a, b) => a + b, 0);
  }

  function handleMouseDown(event: MouseEvent, index: number): void {
    event.preventDefault();
    dragging = true;
    dragIndex = index;
    startPos = direction === "row" ? event.clientX : event.clientY;
    startSizes = [...sizes];

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  function handleMouseMove(event: MouseEvent): void {
    if (!dragging || !containerEl) return;

    const rect = containerEl.getBoundingClientRect();
    const totalPx = direction === "row" ? rect.width : rect.height;
    const currentPos = direction === "row" ? event.clientX : event.clientY;
    const deltaPx = currentPos - startPos;
    const total = totalSize();
    const deltaRatio = (deltaPx / totalPx) * total;

    const newSizes = [...startSizes];
    const minSize = 0.05 * total; // 5% minimum

    // Redistribute between adjacent panes
    newSizes[dragIndex] = Math.max(minSize, startSizes[dragIndex] + deltaRatio);
    newSizes[dragIndex + 1] = Math.max(minSize, startSizes[dragIndex + 1] - deltaRatio);

    // Clamp: ensure neither goes below minimum
    if (newSizes[dragIndex] < minSize) {
      newSizes[dragIndex] = minSize;
      newSizes[dragIndex + 1] = startSizes[dragIndex] + startSizes[dragIndex + 1] - minSize;
    }
    if (newSizes[dragIndex + 1] < minSize) {
      newSizes[dragIndex + 1] = minSize;
      newSizes[dragIndex] = startSizes[dragIndex] + startSizes[dragIndex + 1] - minSize;
    }

    sizes = newSizes;
  }

  function handleMouseUp(): void {
    if (dragging) {
      resizePanes(nodeId, sizes);
    }
    dragging = false;
    dragIndex = -1;
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", handleMouseUp);
  }
</script>

<div
  class="split-pane"
  class:split-row={direction === "row"}
  class:split-col={direction === "col"}
  class:is-dragging={dragging}
  bind:this={containerEl}
>
  {#each sizes as size, i}
    <div class="split-child" style="flex-grow: {size};">
      <slot name="child" index={i} />
    </div>
    {#if i < sizes.length - 1}
      <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
      <div
        class="split-handle"
        class:handle-h={direction === "row"}
        class:handle-v={direction === "col"}
        on:mousedown={(e) => handleMouseDown(e, i)}
        role="separator"
        aria-orientation={direction === "row" ? "vertical" : "horizontal"}
        tabindex="-1"
      ></div>
    {/if}
  {/each}
</div>

<style>
  .split-pane {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .split-row {
    flex-direction: row;
  }

  .split-col {
    flex-direction: column;
  }

  .split-child {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    flex-shrink: 1;
    flex-basis: 0;
  }

  .split-handle {
    flex-shrink: 0;
    background: var(--border);
    transition: background 0.15s ease;
    z-index: 5;
  }

  .split-handle:hover,
  .is-dragging .split-handle {
    background: var(--hl-symbol);
  }

  .handle-h {
    width: 6px;
    cursor: col-resize;
    margin: 0 -1px;
    position: relative;
  }

  .handle-h::after {
    content: '';
    position: absolute;
    inset: 0 -4px;
  }

  .handle-v {
    height: 6px;
    cursor: row-resize;
    margin: -1px 0;
    position: relative;
  }

  .handle-v::after {
    content: '';
    position: absolute;
    inset: -4px 0;
  }
</style>
