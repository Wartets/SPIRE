<!--
  SPIRE - Log Console Component

  Displays a scrollable list of timestamped system messages,
  computation summaries, and error notifications. The log store
  is populated by appendLog() calls throughout the application.
-->
<script lang="ts">
  import { logs } from "$lib/stores/physicsStore";
  import { afterUpdate } from "svelte";

  let scrollContainer: HTMLDivElement;

  /** Auto-scroll to bottom when new log entries arrive. */
  afterUpdate(() => {
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  });

  function clearLogs(): void {
    logs.set([]);
  }

  $: entries = $logs;
</script>

<div class="log-console">
  <div class="log-header">
    <h3>Console</h3>
    <div class="log-controls">
      <span class="entry-count">{entries.length} entries</span>
      <button class="clear-btn" on:click={clearLogs}>Clear</button>
    </div>
  </div>

  <div class="log-scroll" bind:this={scrollContainer}>
    {#if entries.length === 0}
      <p class="hint">System messages and computation results will appear here.</p>
    {:else}
      {#each entries as entry}
        <div
          class="log-entry"
          class:error={entry.includes("ERROR")}
          class:success={entry.includes("[valid]") || entry.includes("loaded") || entry.includes("Generated") || entry.includes("Derived")}
        >
          {entry}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .log-console {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.3rem;
  }
  .log-header h3 {
    margin: 0;
    font-size: 0.85rem;
    color: var(--fg-accent);
  }
  .log-controls {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .entry-count {
    font-size: 0.68rem;
    color: var(--fg-secondary);
  }
  .clear-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.15rem 0.4rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .clear-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }
  .log-scroll {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.3rem 0.4rem;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    line-height: 1.6;
  }
  .hint {
    font-size: 0.75rem;
    color: var(--fg-secondary);
    font-style: italic;
    margin: 0;
  }
  .log-entry {
    color: var(--fg-primary);
    padding: 0.05rem 0;
    border-bottom: 1px solid var(--border);
    word-break: break-word;
  }
  .log-entry.error {
    color: var(--hl-error);
  }
  .log-entry.success {
    color: var(--hl-success);
  }
</style>
