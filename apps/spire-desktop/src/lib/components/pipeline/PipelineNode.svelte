<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { PipelineNode, PipelinePort } from "$lib/stores/pipelineGraphStore";
  import type { PipelineNodeExecutionStatus } from "$lib/core/pipeline/PipelineExecutor";

  export let node: PipelineNode;
  export let selected = false;
  export let status: PipelineNodeExecutionStatus = "idle";
  export let errorMessage: string | null = null;

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
  class:running={status === "running"}
  class:completed={status === "completed"}
  class:error={status === "error"}
  class:stale={status === "stale"}
  style="left: {node.position.x}px; top: {node.position.y}px; width: {node.size.width}px; height: {node.size.height}px;"
  on:mousedown={handleNodeMouseDown}
  role="button"
  tabindex="0"
  aria-label={node.label}
>
  <header class="node-header">
    <div class="node-title-wrap">
      <div class="node-title">{node.label}</div>
      {#if status === "running"}
        <span class="node-spinner" aria-label="Running"></span>
      {:else if status === "stale"}
        <span class="node-chip stale-chip">Stale</span>
      {:else if status === "completed"}
        <span class="node-chip complete-chip">Cached</span>
      {/if}
    </div>
    <button
      class="node-remove"
      type="button"
      aria-label="Remove node"
      on:click|stopPropagation={() => dispatch("remove", { nodeId: node.id })}
    >
      ×
    </button>
  </header>

  {#if status === "error"}
    <div class="node-error" title={errorMessage ?? "Execution failed"}>⚠ {errorMessage ?? "Execution failed"}</div>
  {/if}

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

  .pipeline-node.running {
    border-color: var(--color-accent);
  }

  .pipeline-node.completed {
    border-color: var(--color-success);
  }

  .pipeline-node.error {
    border-color: var(--color-error);
  }

  .pipeline-node.stale {
    border-color: color-mix(in oklab, var(--color-warning) 70%, var(--color-border));
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

  .node-title-wrap {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
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

  .node-spinner {
    width: 0.68rem;
    height: 0.68rem;
    border: 2px solid rgba(var(--color-accent-rgb), 0.3);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: node-spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  @keyframes node-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .node-chip {
    border: 1px solid var(--color-border);
    border-radius: 4px;
    padding: 0.06rem 0.28rem;
    font-size: 0.56rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .complete-chip {
    border-color: color-mix(in oklab, var(--color-success) 55%, var(--color-border));
    color: var(--color-success);
  }

  .stale-chip {
    border-color: color-mix(in oklab, var(--color-warning) 55%, var(--color-border));
    color: var(--color-warning);
  }

  .node-error {
    margin: 0.35rem 0.45rem 0;
    padding: 0.18rem 0.28rem;
    border: 1px solid color-mix(in oklab, var(--color-error) 50%, var(--color-border));
    border-radius: 6px;
    background: rgba(var(--color-error-rgb), 0.08);
    color: var(--color-error);
    font-size: 0.58rem;
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
