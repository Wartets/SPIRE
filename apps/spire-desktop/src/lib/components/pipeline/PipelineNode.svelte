<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { PipelineNode, PipelinePort } from "$lib/stores/pipelineGraphStore";

  export let node: PipelineNode;
  export let selected = false;

  const dispatch = createEventDispatcher<{
    select: { nodeId: string };
    remove: { nodeId: string };
    startdrag: { nodeId: string; clientX: number; clientY: number };
    portdragstart: {
      nodeId: string;
      portId: string;
      direction: "input" | "output";
      clientX: number;
      clientY: number;
    };
  }>();

  function portTop(index: number, count: number): number {
    const safeCount = Math.max(1, count);
    const topPadding = 38;
    const usableHeight = Math.max(42, node.size.height - 52);
    return topPadding + (usableHeight * (index + 0.5)) / safeCount;
  }

  function handleNodeMouseDown(event: MouseEvent): void {
    if (event.button !== 0) return;
    dispatch("select", { nodeId: node.id });
    dispatch("startdrag", {
      nodeId: node.id,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  }

  function handlePortMouseDown(event: MouseEvent, port: PipelinePort): void {
    if (event.button !== 0) return;
    event.stopPropagation();
    dispatch("select", { nodeId: node.id });
    dispatch("portdragstart", {
      nodeId: node.id,
      portId: port.id,
      direction: port.direction,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  }
</script>

<div
  class="pipeline-node"
  class:selected
  style="left: {node.position.x}px; top: {node.position.y}px; width: {node.size.width}px; height: {node.size.height}px;"
  on:mousedown={handleNodeMouseDown}
  role="button"
  tabindex="0"
  aria-label={node.label}
>
  <header class="node-header">
    <div class="node-title">{node.label}</div>
    <button
      class="node-remove"
      type="button"
      aria-label="Remove node"
      on:click|stopPropagation={() => dispatch("remove", { nodeId: node.id })}
    >
      ×
    </button>
  </header>

  <div class="node-body">
    {#each node.inputs as input, index (input.id)}
      <button
        type="button"
        class="port port-input"
        data-port-id={input.id}
        data-port-direction="input"
        style="top: {portTop(index, node.inputs.length)}px;"
        on:mousedown={(event) => handlePortMouseDown(event, input)}
        aria-label={`Input ${input.label}`}
      >
        <span class="port-dot"></span>
        <span class="port-label">{input.label}</span>
      </button>
    {/each}

    {#each node.outputs as output, index (output.id)}
      <button
        type="button"
        class="port port-output"
        data-port-id={output.id}
        data-port-direction="output"
        style="top: {portTop(index, node.outputs.length)}px;"
        on:mousedown={(event) => handlePortMouseDown(event, output)}
        aria-label={`Output ${output.label}`}
      >
        <span class="port-label">{output.label}</span>
        <span class="port-dot"></span>
      </button>
    {/each}
  </div>
</div>

<style>
  .pipeline-node {
    position: absolute;
    border: 1px solid var(--color-border);
    border-radius: 10px;
    background: linear-gradient(180deg, color-mix(in oklab, var(--color-bg-elevated) 92%, #fff 8%), var(--color-bg-surface));
    box-shadow: 0 8px 22px rgba(0, 0, 0, 0.3);
    user-select: none;
    cursor: grab;
  }

  .pipeline-node:active {
    cursor: grabbing;
  }

  .pipeline-node.selected {
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px rgba(var(--color-accent-rgb), 0.45), 0 10px 26px rgba(0, 0, 0, 0.35);
  }

  .node-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.38rem 0.55rem;
    border-bottom: 1px solid color-mix(in oklab, var(--color-border) 85%, transparent);
    background: color-mix(in oklab, var(--color-bg-inset) 70%, transparent);
    border-radius: 10px 10px 0 0;
  }

  .node-title {
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-weight: 700;
  }

  .node-remove {
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    width: 1.2rem;
    height: 1.2rem;
    line-height: 1;
    padding: 0;
  }

  .node-remove:hover {
    color: var(--color-error);
    border-color: color-mix(in oklab, var(--color-error) 55%, transparent);
  }

  .node-body {
    position: relative;
    width: 100%;
    height: calc(100% - 2rem);
  }

  .port {
    position: absolute;
    transform: translateY(-50%);
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    padding: 0;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    line-height: 1;
    max-width: 48%;
  }

  .port-input {
    left: 0.28rem;
  }

  .port-output {
    right: 0.28rem;
    justify-content: flex-end;
    text-align: right;
  }

  .port-dot {
    width: 0.52rem;
    height: 0.52rem;
    border-radius: 50%;
    border: 1px solid var(--color-border);
    background: var(--color-bg-inset);
    flex-shrink: 0;
  }

  .port:hover {
    color: var(--color-text-primary);
  }

  .port:hover .port-dot {
    border-color: var(--color-accent);
    background: color-mix(in oklab, var(--color-accent) 36%, var(--color-bg-inset));
    box-shadow: 0 0 0 4px rgba(var(--color-accent-rgb), 0.12);
  }

  .port-label {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
