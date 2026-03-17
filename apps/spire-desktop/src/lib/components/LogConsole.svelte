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

  type LogLevel = "info" | "warn" | "error" | "success" | "debug";

  interface ParsedLogEntry {
    raw: string;
    timestamp: string;
    message: string;
    level: LogLevel;
  }

  /** Auto-scroll to bottom when new log entries arrive. */
  afterUpdate(() => {
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
    }
  });

  function clearLogs(): void {
    logs.set([]);
  }

  function inferLevel(message: string): LogLevel {
    const m = message.toLowerCase();
    if (m.includes("error") || m.includes("failed") || m.includes("exception")) return "error";
    if (m.includes("warn") || m.includes("warning")) return "warn";
    if (m.includes("debug") || m.includes("trace")) return "debug";
    if (
      m.includes("success") ||
      m.includes("loaded") ||
      m.includes("generated") ||
      m.includes("derived") ||
      m.includes("complete")
    ) {
      return "success";
    }
    return "info";
  }

  function parseEntry(raw: string): ParsedLogEntry {
    const match = raw.match(/^\[(.*?)\]\s*(.*)$/);
    const timestamp = match?.[1] ?? "--:--:--";
    const message = match?.[2] ?? raw;
    return {
      raw,
      timestamp,
      message,
      level: inferLevel(message),
    };
  }

  let showInfo = true;
  let showWarn = true;
  let showError = true;
  let showSuccess = true;
  let showDebug = false;

  function levelEnabled(level: LogLevel): boolean {
    if (level === "info") return showInfo;
    if (level === "warn") return showWarn;
    if (level === "error") return showError;
    if (level === "success") return showSuccess;
    return showDebug;
  }

  $: entries = $logs.map(parseEntry);
  $: filteredEntries = entries.filter((entry) => levelEnabled(entry.level));
</script>

<div class="log-console">
  <div class="log-header">
    <h3>Console</h3>
    <div class="log-controls">
      <span class="entry-count">{filteredEntries.length}/{entries.length}</span>
      <button class="clear-btn" on:click={clearLogs}>Clear</button>
    </div>
  </div>

  <div class="level-filters">
    <label class="lf-chip"><input type="checkbox" bind:checked={showInfo} />Info</label>
    <label class="lf-chip"><input type="checkbox" bind:checked={showWarn} />Warn</label>
    <label class="lf-chip"><input type="checkbox" bind:checked={showError} />Error</label>
    <label class="lf-chip"><input type="checkbox" bind:checked={showSuccess} />Success</label>
    <label class="lf-chip"><input type="checkbox" bind:checked={showDebug} />Debug</label>
  </div>

  <div class="log-scroll" bind:this={scrollContainer}>
    {#if filteredEntries.length === 0}
      <p class="hint">System messages and computation results will appear here.</p>
    {:else}
      {#each filteredEntries as entry}
        <div
          class="log-entry"
          class:error={entry.level === "error"}
          class:warn={entry.level === "warn"}
          class:success={entry.level === "success"}
          class:debug={entry.level === "debug"}
        >
          <span class="log-ts">[{entry.timestamp}]</span>
          <span class="log-msg">{entry.message}</span>
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
    display: grid;
    grid-template-columns: 5.4rem minmax(0, 1fr);
    align-items: start;
    gap: 0.35rem;
    color: var(--fg-primary);
    padding: 0.08rem 0;
    border-bottom: 1px solid var(--border);
    word-break: break-word;
  }
  .log-ts {
    color: var(--fg-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .log-msg {
    white-space: pre-wrap;
  }
  .log-entry.error {
    color: var(--hl-error);
  }
  .log-entry.warn {
    color: var(--hl-value);
  }
  .log-entry.success {
    color: var(--hl-success);
  }
  .log-entry.debug {
    color: color-mix(in srgb, var(--fg-secondary) 85%, var(--fg-primary));
  }

  .level-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.3rem;
  }

  .lf-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.65rem;
    padding: 0.05rem 0.35rem;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    user-select: none;
  }

  .lf-chip input {
    width: 0.7rem;
    height: 0.7rem;
  }

  @media (max-width: 560px) {
    .log-entry {
      display: block;
    }
    .log-ts {
      display: inline;
      margin-right: 0.25rem;
    }
    .log-msg {
      display: inline;
    }
  }
</style>
