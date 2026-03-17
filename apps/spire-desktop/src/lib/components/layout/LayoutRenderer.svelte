<!--
  SPIRE - Layout Renderer

  Recursive component that traverses the layout tree and renders
  the appropriate container or leaf component for each node.

  - RowNode / ColNode  →  SplitPane with nested LayoutRenderers
  - StackNode          →  TabStack with nested LayoutRenderers
  - WidgetLeaf         →  WidgetContainer (the actual physics widget)

  This is a `<svelte:self>` recursion pattern.

  Props:
    node - The LayoutNode to render (any variant).
-->
<script lang="ts">
  import type { LayoutNode, RowNode, ColNode, StackNode, WidgetLeaf } from "$lib/stores/layoutStore";
  import SplitPane from "./SplitPane.svelte";
  import WidgetContainer from "./WidgetContainer.svelte";

  export let node: LayoutNode;
</script>

{#if node.type === "row" || node.type === "col"}
  {@const container = node}
  <SplitPane
    nodeId={container.id}
    direction={container.type}
    sizes={container.sizes}
  >
    <svelte:fragment slot="child" let:index>
      {#if container.children[index]}
        <svelte:self node={container.children[index]} />
      {/if}
    </svelte:fragment>
  </SplitPane>
{:else if node.type === "stack"}
  {@const stack = node}
  {#if stack.children.length === 0}
    <div style="height: 100%; display: flex; align-items: center; justify-content: center; color: var(--fg-secondary); font-size: 0.75rem;">
      Empty workspace
    </div>
  {:else if stack.children.length === 1}
    <svelte:self node={stack.children[0]} />
  {:else}
    <SplitPane
      nodeId={stack.id}
      direction="row"
      sizes={stack.children.map(() => 1)}
    >
      <svelte:fragment slot="child" let:index>
        {#if stack.children[index]}
          <svelte:self node={stack.children[index]} />
        {/if}
      </svelte:fragment>
    </SplitPane>
  {/if}
{:else if node.type === "widget"}
  <WidgetContainer node={node} />
{:else}
  <div style="color: var(--hl-error); padding: 1rem;">
    Unknown layout node type
  </div>
{/if}
