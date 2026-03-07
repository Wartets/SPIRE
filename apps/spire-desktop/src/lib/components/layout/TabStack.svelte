<!--
  SPIRE - Tab Stack

  Renders a tabbed group of layout nodes.  Only the active tab's
  content is displayed; the tab bar shows all children with their
  widget type labels.

  Props:
    node - The StackNode from the layout tree.
-->
<script lang="ts">
  import type { StackNode, LayoutNode } from "$lib/stores/layoutStore";
  import { setActiveTab, closeNode, moveNode } from "$lib/stores/layoutStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";

  export let node: StackNode;

  function getTabLabel(child: LayoutNode): string {
    if (child.type === "widget") {
      return WIDGET_LABELS[child.widgetType] ?? child.widgetType;
    }
    return child.type.toUpperCase();
  }

  function handleSelectTab(index: number): void {
    setActiveTab(node.id, index);
  }

  function handleCloseTab(index: number, event: MouseEvent | KeyboardEvent): void {
    event.stopPropagation();
    const child = node.children[index];
    closeNode(child.id);
  }

  // ── Drag-and-Drop State ──
  let dragTabIndex: number | null = null;
  let dropTabIndex: number | null = null;
  let dropEdge: 'left' | 'right' | 'top' | 'bottom' | null = null;

  function handleDragStart(event: DragEvent, index: number): void {
    dragTabIndex = index;
    event.dataTransfer?.setData('application/tab', JSON.stringify({
      widgetId: node.children[index].id,
      sourceNodeId: node.id,
    }));
    event.dataTransfer!.effectAllowed = 'move';
  }

  function handleDragOver(event: DragEvent, index: number): void {
    event.preventDefault();
    dropTabIndex = index;
    dropEdge = null;
  }

  function handleDrop(event: DragEvent, index: number): void {
    event.preventDefault();
    const payload = event.dataTransfer?.getData('application/tab');
    if (!payload) return;
    const { widgetId } = JSON.parse(payload);
    // Reorder within stack
    if (widgetId && dragTabIndex !== null && dropTabIndex !== null && widgetId === node.children[dragTabIndex].id) {
      if (dragTabIndex !== dropTabIndex) {
        // True tab reordering: call moveNode with 'center'
        moveNode(widgetId, node.children[dropTabIndex].id, 'center');
        setActiveTab(node.id, dropTabIndex);
      }
    }
    dragTabIndex = null;
    dropTabIndex = null;
  }

  // ── Edge Drop Zones for Splitting ──
  function handleEdgeDragOver(event: DragEvent, edge: 'left' | 'right' | 'top' | 'bottom'): void {
    event.preventDefault();
    dropEdge = edge;
  }
  function handleEdgeDrop(event: DragEvent, edge: 'left' | 'right' | 'top' | 'bottom', index: number): void {
    event.preventDefault();
    const payload = event.dataTransfer?.getData('application/tab');
    if (!payload) return;
    const { widgetId } = JSON.parse(payload);
    if (widgetId) {
      // True pane splitting: call moveNode with edge
      moveNode(widgetId, node.children[index].id, edge);
      setActiveTab(node.id, index);
    }
    dropEdge = null;
    dragTabIndex = null;
    dropTabIndex = null;
  }

  $: activeIndex = Math.min(node.activeIndex, node.children.length - 1);
  $: activeChild = node.children[activeIndex] ?? null;
</script>

<div class="tab-stack">
  <div class="tab-bar">
    {#each node.children as child, i}
      <div class="tab-drag-wrapper">
        <button
          class="tab-btn"
          class:active={i === activeIndex}
          draggable="true"
          on:dragstart={(e) => handleDragStart(e, i)}
          on:dragover={(e) => handleDragOver(e, i)}
          on:drop={(e) => handleDrop(e, i)}
          title={getTabLabel(child)}
        >
          <span class="tab-label">{getTabLabel(child)}</span>
          <span
            class="tab-close"
            on:click={(e) => handleCloseTab(i, e)}
            on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleCloseTab(i, e); }}
            role="button"
            tabindex="-1"
            aria-label="Close tab"
          >&times;</span>
        </button>
        <!-- Edge drop zones -->
        <div class="edge-drop-zone left" on:dragover={(e) => handleEdgeDragOver(e, 'left')} on:drop={(e) => handleEdgeDrop(e, 'left', i)} style="border-left: {dropEdge === 'left' ? '4px solid var(--hl-symbol)' : 'none'}"></div>
        <div class="edge-drop-zone right" on:dragover={(e) => handleEdgeDragOver(e, 'right')} on:drop={(e) => handleEdgeDrop(e, 'right', i)} style="border-right: {dropEdge === 'right' ? '4px solid var(--hl-symbol)' : 'none'}"></div>
        <div class="edge-drop-zone top" on:dragover={(e) => handleEdgeDragOver(e, 'top')} on:drop={(e) => handleEdgeDrop(e, 'top', i)} style="border-top: {dropEdge === 'top' ? '4px solid var(--hl-symbol)' : 'none'}"></div>
        <div class="edge-drop-zone bottom" on:dragover={(e) => handleEdgeDragOver(e, 'bottom')} on:drop={(e) => handleEdgeDrop(e, 'bottom', i)} style="border-bottom: {dropEdge === 'bottom' ? '4px solid var(--hl-symbol)' : 'none'}"></div>
      </div>
    {/each}
  </div>
  <div class="tab-content">
    {#if activeChild}
      <slot name="child" node={activeChild} />
    {/if}
  </div>
</div>

<style>
  .tab-stack {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
    min-height: 1.4rem;
    position: relative;
  }

  .tab-drag-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .edge-drop-zone {
    position: absolute;
    width: 20%;
    height: 100%;
    z-index: 2;
    pointer-events: auto;
    opacity: 0;
  }
  .edge-drop-zone.left { left: 0; top: 0; }
  .edge-drop-zone.right { right: 0; top: 0; }
  .edge-drop-zone.top { left: 0; top: 0; height: 20%; width: 100%; }
  .edge-drop-zone.bottom { left: 0; bottom: 0; height: 20%; width: 100%; }
  .edge-drop-zone[style*="solid"] { opacity: 1; }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem 0.5rem;
    background: transparent;
    border: none;
    border-right: 1px solid var(--border);
    color: var(--fg-secondary);
    font-size: 0.62rem;
    font-family: var(--font-mono);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    white-space: nowrap;
  }

  .tab-btn:hover {
    background: var(--bg-surface);
    color: var(--fg-primary);
  }

  .tab-btn.active {
    background: var(--bg-surface);
    color: var(--fg-accent);
    border-bottom: 2px solid var(--hl-symbol);
  }

  .tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100px;
  }

  .tab-close {
    font-size: 0.72rem;
    color: var(--fg-secondary);
    cursor: pointer;
    line-height: 1;
  }

  .tab-close:hover {
    color: var(--hl-error);
  }

  .tab-content {
    flex: 1;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
</style>
