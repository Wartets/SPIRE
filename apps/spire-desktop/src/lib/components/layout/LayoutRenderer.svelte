<!--
  SPIRE — Layout Renderer

  Recursive component that traverses the layout tree and renders
  the appropriate container or leaf component for each node.

  - RowNode / ColNode  →  SplitPane with nested LayoutRenderers
  - StackNode          →  TabStack with nested LayoutRenderers
  - WidgetLeaf         →  WidgetContainer (the actual physics widget)

  This is a `<svelte:self>` recursion pattern.

  Props:
    node — The LayoutNode to render (any variant).
-->
<script lang="ts">
  import type { LayoutNode, RowNode, ColNode, StackNode, WidgetLeaf } from "$lib/stores/layoutStore";
  import SplitPane from "./SplitPane.svelte";
  import TabStack from "./TabStack.svelte";
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
  <TabStack {node}>
    <svelte:fragment slot="child" let:node={childNode}>
      <svelte:self node={childNode} />
    </svelte:fragment>
  </TabStack>
{:else if node.type === "widget"}
  <WidgetContainer node={node} />
{:else}
  <div style="color: var(--hl-error); padding: 1rem;">
    Unknown layout node type
  </div>
{/if}
