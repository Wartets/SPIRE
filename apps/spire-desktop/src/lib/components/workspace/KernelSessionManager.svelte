<script lang="ts">
  import { workspaces, activeWorkspaceId } from "$lib/stores/layoutStore";
  import { pipelineGraph, pipelineRunState } from "$lib/stores/pipelineGraphStore";
  import { theoreticalModel } from "$lib/stores/physicsStore";

  export let open = false;
  export let onClose: () => void;

  $: nodesCount = Object.keys($pipelineGraph.nodes).length;
  $: edgesCount = Object.keys($pipelineGraph.edges).length;
  $: runningNodeCount = Object.values($pipelineRunState.nodeStates).filter((s) => s.status === "running").length;
  $: staleNodeCount = Object.values($pipelineRunState.nodeStates).filter((s) => s.status === "stale").length;

  $: memoryUsedMb = (() => {
    const perfWithMemory = performance as Performance & {
      memory?: { usedJSHeapSize: number };
    };
    const bytes = perfWithMemory.memory?.usedJSHeapSize;
    if (typeof bytes !== "number") return null;
    return Math.round((bytes / 1024 / 1024) * 10) / 10;
  })();
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="ksm-overlay" on:click={onClose}>
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="ksm-panel"
      role="dialog"
      aria-modal="true"
      aria-label="Kernel and session manager"
      tabindex="-1"
      on:click|stopPropagation
    >
      <header class="ksm-header">
        <h2>Kernel / Session Manager</h2>
        <button class="ksm-close" on:click={onClose}>×</button>
      </header>

      <section class="ksm-grid">
        <div class="ksm-card">
          <span class="label">Active model</span>
          <strong>{$theoreticalModel?.name ?? "None loaded"}</strong>
        </div>
        <div class="ksm-card">
          <span class="label">Pipeline nodes</span>
          <strong>{nodesCount}</strong>
        </div>
        <div class="ksm-card">
          <span class="label">Pipeline edges</span>
          <strong>{edgesCount}</strong>
        </div>
        <div class="ksm-card">
          <span class="label">Memory (JS heap)</span>
          <strong>{memoryUsedMb === null ? "N/A" : `${memoryUsedMb} MB`}</strong>
        </div>
      </section>

      <section class="ksm-runtime">
        <h3>Execution status</h3>
        <div class="status-row">
          <span>Run active</span>
          <strong>{$pipelineRunState.isRunning ? "Yes" : "No"}</strong>
        </div>
        <div class="status-row">
          <span>Running nodes</span>
          <strong>{runningNodeCount}</strong>
        </div>
        <div class="status-row">
          <span>Stale nodes</span>
          <strong>{staleNodeCount}</strong>
        </div>
      </section>

      <section class="ksm-workspaces">
        <h3>Workspaces</h3>
        {#each $workspaces as ws}
          <div class="workspace-row" class:is-active={ws.id === $activeWorkspaceId}>
            <span class="dot" style={`background:${ws.color}`}></span>
            <div class="meta">
              <strong>{ws.name}</strong>
              <small>{ws.description}</small>
            </div>
            <span class="mode">{ws.mode}</span>
          </div>
        {/each}
      </section>
    </div>
  </div>
{/if}

<style>
  .ksm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.48);
    z-index: 12600;
    display: flex;
    justify-content: flex-end;
  }

  .ksm-panel {
    width: min(520px, 88vw);
    height: 100%;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 0.7rem;
    overflow: auto;
  }

  .ksm-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.4rem;
  }

  .ksm-header h2 {
    margin: 0;
    font-size: 0.9rem;
    color: var(--fg-primary);
    letter-spacing: 0.04em;
  }

  .ksm-close {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    width: 1.6rem;
    height: 1.6rem;
    cursor: pointer;
  }

  .ksm-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem;
  }

  .ksm-card {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .ksm-card .label {
    font-size: 0.64rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .ksm-card strong {
    font-size: 0.8rem;
    color: var(--fg-primary);
  }

  .ksm-runtime,
  .ksm-workspaces {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .ksm-runtime h3,
  .ksm-workspaces h3 {
    margin: 0 0 0.2rem;
    font-size: 0.72rem;
    color: var(--fg-accent);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .status-row,
  .workspace-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.68rem;
    color: var(--fg-secondary);
  }

  .workspace-row {
    border: 1px solid var(--border);
    padding: 0.28rem 0.35rem;
    justify-content: flex-start;
  }

  .workspace-row.is-active {
    border-color: var(--hl-symbol);
  }

  .dot {
    width: 0.55rem;
    height: 0.55rem;
    flex-shrink: 0;
  }

  .meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .meta strong {
    font-size: 0.7rem;
    color: var(--fg-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta small {
    font-size: 0.62rem;
    color: var(--fg-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mode {
    font-size: 0.62rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
  }
</style>
