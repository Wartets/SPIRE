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
  import { setActiveTab, closeNode } from "$lib/stores/layoutStore";
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

  $: activeIndex = Math.min(node.activeIndex, node.children.length - 1);
  $: activeChild = node.children[activeIndex] ?? null;
</script>

<div class="tab-stack">
  <div class="tab-bar">
    {#each node.children as child, i}
      <button
        class="tab-btn"
        class:active={i === activeIndex}
        on:click={() => handleSelectTab(i)}
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
  }
</style>
