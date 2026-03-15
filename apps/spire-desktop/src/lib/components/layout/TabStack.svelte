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
  import {
    setActiveTab,
    closeNode,
    reorderTabsInStack,
    moveTabToStack,
    splitTabToEdge,
  } from "$lib/stores/layoutStore";
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

  type EdgeDrop = "left" | "right" | "top" | "bottom";

  interface DragPayload {
    sourceStackId: string;
    sourceIndex: number;
    widgetId: string;
  }

  const MIME = "application/x-spire-tab";

  let hoverEdge: EdgeDrop | null = null;

  function makePayload(index: number): DragPayload {
    return {
      sourceStackId: node.id,
      sourceIndex: index,
      widgetId: node.children[index]?.id ?? "",
    };
  }

  function writePayload(event: DragEvent, payload: DragPayload): void {
    if (!event.dataTransfer) return;
    event.dataTransfer.effectAllowed = "move";
    const encoded = JSON.stringify(payload);
    event.dataTransfer.setData(MIME, encoded);
    event.dataTransfer.setData("text/plain", encoded);
  }

  function readPayload(event: DragEvent): DragPayload | null {
    const raw = event.dataTransfer?.getData(MIME) || event.dataTransfer?.getData("text/plain");
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw) as DragPayload;
      if (!parsed.sourceStackId || typeof parsed.sourceIndex !== "number") return null;
      return parsed;
    } catch {
      return null;
    }
  }

  function handleTabDragStart(event: DragEvent, index: number): void {
    writePayload(event, makePayload(index));
  }

  function handleTabDrop(event: DragEvent, targetIndex: number): void {
    event.preventDefault();
    event.stopPropagation();
    const payload = readPayload(event);
    if (!payload) return;

    if (payload.sourceStackId === node.id) {
      reorderTabsInStack(node.id, payload.sourceIndex, targetIndex);
      return;
    }

    moveTabToStack(payload.sourceStackId, payload.sourceIndex, node.id, targetIndex);
  }

  function handleTabBarDrop(event: DragEvent): void {
    event.preventDefault();
    const payload = readPayload(event);
    if (!payload) return;
    moveTabToStack(payload.sourceStackId, payload.sourceIndex, node.id, node.children.length);
  }

  function allowDrop(event: DragEvent): void {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "move";
    }
  }

  function handleEdgeDragOver(event: DragEvent, edge: EdgeDrop): void {
    allowDrop(event);
    hoverEdge = edge;
  }

  function clearEdgeHover(): void {
    hoverEdge = null;
  }

  function handleEdgeDrop(event: DragEvent, edge: EdgeDrop): void {
    event.preventDefault();
    event.stopPropagation();
    const payload = readPayload(event);
    if (!payload) {
      hoverEdge = null;
      return;
    }
    splitTabToEdge(payload.sourceStackId, payload.sourceIndex, node.id, edge);
    hoverEdge = null;
  }

  $: activeIndex = Math.min(node.activeIndex, node.children.length - 1);
  $: activeChild = node.children[activeIndex] ?? null;
</script>

<div class="tab-stack">
  <div
    class="tab-bar"
    on:dragover={allowDrop}
    on:drop={handleTabBarDrop}
    role="tablist"
    tabindex="-1"
  >
    {#each node.children as child, i}
      <button
        class="tab-btn"
        class:active={i === activeIndex}
        on:click={() => handleSelectTab(i)}
        draggable="true"
        on:dragstart={(e) => handleTabDragStart(e, i)}
        on:dragover={allowDrop}
        on:drop={(e) => handleTabDrop(e, i)}
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
    {/each}
  </div>
  <div class="tab-content" on:dragleave={clearEdgeHover} role="region">
    <div
      class="edge-drop edge-left"
      class:edge-active={hoverEdge === "left"}
      on:dragover={(e) => handleEdgeDragOver(e, "left")}
      on:drop={(e) => handleEdgeDrop(e, "left")}
      role="button"
      tabindex="-1"
    ></div>
    <div
      class="edge-drop edge-right"
      class:edge-active={hoverEdge === "right"}
      on:dragover={(e) => handleEdgeDragOver(e, "right")}
      on:drop={(e) => handleEdgeDrop(e, "right")}
      role="button"
      tabindex="-1"
    ></div>
    <div
      class="edge-drop edge-top"
      class:edge-active={hoverEdge === "top"}
      on:dragover={(e) => handleEdgeDragOver(e, "top")}
      on:drop={(e) => handleEdgeDrop(e, "top")}
      role="button"
      tabindex="-1"
    ></div>
    <div
      class="edge-drop edge-bottom"
      class:edge-active={hoverEdge === "bottom"}
      on:dragover={(e) => handleEdgeDragOver(e, "bottom")}
      on:drop={(e) => handleEdgeDrop(e, "bottom")}
      role="button"
      tabindex="-1"
    ></div>

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
  }

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
    position: relative;
  }

  .edge-drop {
    position: absolute;
    z-index: 6;
  }

  .edge-left,
  .edge-right {
    top: 0;
    bottom: 0;
    width: 20%;
  }

  .edge-left {
    left: 0;
  }

  .edge-right {
    right: 0;
  }

  .edge-top,
  .edge-bottom {
    left: 0;
    right: 0;
    height: 20%;
  }

  .edge-top {
    top: 0;
  }

  .edge-bottom {
    bottom: 0;
  }

  .edge-left.edge-active {
    border-left: 4px solid var(--hl-symbol);
  }

  .edge-right.edge-active {
    border-right: 4px solid var(--hl-symbol);
  }

  .edge-top.edge-active {
    border-top: 4px solid var(--hl-symbol);
  }

  .edge-bottom.edge-active {
    border-bottom: 4px solid var(--hl-symbol);
  }
</style>
