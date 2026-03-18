<!--
  SPIRE - Log Console Component

  Displays a scrollable list of timestamped system messages,
  computation summaries, and error notifications. The log store
  is populated by appendLog() calls throughout the application.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { logs } from "$lib/stores/physicsStore";
  import { afterUpdate } from "svelte";

  let scrollContainer: HTMLDivElement;
  let compactWrap = false;
  let resizeObserver: ResizeObserver | null = null;

  type LogLevel = "info" | "warn" | "error" | "success" | "debug";

  interface ParsedLogEntry {
    raw: string;
    timestamp: string;
    message: string;
    level: LogLevel;
    category: string;
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
    if (/(^|\b)(error|failed|failure|exception|fatal|panic|invalid|erreur|échec)(\b|:)/i.test(m)) return "error";
    if (/(^|\b)(warn|warning|deprecated|avertissement)(\b|:)/i.test(m)) return "warn";
    if (/(^|\b)(debug|trace|verbose)(\b|:)/i.test(m)) return "debug";
    if (
      /(^|\b)(success|loaded|generated|derived|complete|completed|saved|exported|imported|restored|done)(\b|:)/i.test(
        m,
      )
    ) {
      return "success";
    }
    return "info";
  }

  function inferCategory(message: string): string {
    const explicitTag = message.match(/^\[([^\]]{2,32})\]/)?.[1];
    if (explicitTag) return explicitTag.trim();

    const m = message.toLowerCase();
    if (/\b(storage|localstorage|migration|persist|restore)\b/i.test(m)) return "Storage";
    if (/\b(import|export|ufo|slha|json|csv|clipboard|download)\b/i.test(m)) return "ImportExport";
    if (/\bworkspace\b/i.test(m)) return "Workspace";
    if (/\bmodel|framework|vertex|field\b/i.test(m)) return "Model";
    if (/\breaction|final state|initial state|reconstruct\b/i.test(m)) return "Reaction";
    if (/\bdiagram|feynman|topolog\w*\b/i.test(m)) return "Diagram";
    if (/\bamplitude\b/i.test(m)) return "Amplitude";
    if (/\bkinematic|mandelstam|threshold\b/i.test(m)) return "Kinematics";
    if (/\bdalitz\b/i.test(m)) return "Dalitz";
    if (/\banalysis|run card|cross section\b/i.test(m)) return "Analysis";
    if (/\blagrangian|gauge|counterterm|rge\b/i.test(m)) return "Lagrangian";
    if (/\bexternal model|ufo import|slha import\b/i.test(m)) return "ExternalModels";
    if (/\bevent display|detector|event(s)? buffered\b/i.test(m)) return "EventDisplay";
    if (/\bscan|scanner|benchmark scan\b/i.test(m)) return "Scanner";
    if (/\bdecay|branching ratio|br\b/i.test(m)) return "Decay";
    if (/\bcompute grid|distributed|worker\b/i.test(m)) return "ComputeGrid";
    if (/\bnotebook|cell\b/i.test(m)) return "Notebook";
    if (/\btelemetry|metrics|perf\w*\b/i.test(m)) return "Telemetry";
    if (/\bcosmolog|lcdm|relic\b/i.test(m)) return "Cosmology";
    if (/\bflavor|b-mixing|r_k\b/i.test(m)) return "Flavor";
    if (/\bplugin\b/i.test(m)) return "Plugin";
    if (/\bglobal fit|mcmc|fit session\b/i.test(m)) return "GlobalFit";
    if (/\bpipeline|widget bus|synced settings\b/i.test(m)) return "Pipeline";
    return "General";
  }

  function parseEntry(raw: string): ParsedLogEntry {
    const match = raw.match(
      /^\[(.*?)\]\s*(?:\[(INFO|WARN|ERROR|SUCCESS|DEBUG)\]\s*)?(?:\[([^\]]+)\]\s*)?(.*)$/i,
    );
    const timestamp = match?.[1] ?? "--:--:--";
    const explicitLevel = (match?.[2]?.toLowerCase() as LogLevel | undefined) ?? undefined;
    const explicitCategory = match?.[3]?.trim();
    const message = match?.[4] ?? raw;
    return {
      raw,
      timestamp,
      message,
      level: explicitLevel ?? inferLevel(message),
      category: explicitCategory || inferCategory(message),
    };
  }

  let showInfo = true;
  let showWarn = true;
  let showError = true;
  let showSuccess = true;
  let showDebug = false;
  let categoryVisibility: Record<string, boolean> = {};

  $: entries = $logs.map(parseEntry);
  $: categories = Array.from(new Set(entries.map((entry) => entry.category))).sort((a, b) =>
    a.localeCompare(b),
  );

  $: {
    const next: Record<string, boolean> = { ...categoryVisibility };
    let changed = false;
    for (const category of categories) {
      if (!(category in next)) {
        next[category] = true;
        changed = true;
      }
    }
    if (changed) categoryVisibility = next;
  }

  function categoryEnabled(category: string): boolean {
    return categoryVisibility[category] ?? true;
  }

  function toggleCategory(category: string): void {
    categoryVisibility = {
      ...categoryVisibility,
      [category]: !(categoryVisibility[category] ?? true),
    };
  }

  function enableAllCategories(): void {
    const allOn: Record<string, boolean> = {};
    for (const category of categories) allOn[category] = true;
    categoryVisibility = allOn;
  }

  function disableAllCategories(): void {
    const allOff: Record<string, boolean> = {};
    for (const category of categories) allOff[category] = false;
    categoryVisibility = allOff;
  }

  $: filteredEntries = entries.filter((entry) => {
    const levelVisible =
      (entry.level === "info" && showInfo) ||
      (entry.level === "warn" && showWarn) ||
      (entry.level === "error" && showError) ||
      (entry.level === "success" && showSuccess) ||
      (entry.level === "debug" && showDebug);
    const categoryVisible = categoryVisibility[entry.category] ?? true;
    return levelVisible && categoryVisible;
  });

  onMount(() => {
    if (!scrollContainer) return;
    resizeObserver = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? scrollContainer.clientWidth;
      compactWrap = width < 560;
    });
    resizeObserver.observe(scrollContainer);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
  });
</script>

<div class="log-console">
  <div class="log-header">
    <h3>Console</h3>
    <div class="log-controls">
      <span class="entry-count">{filteredEntries.length}/{entries.length}</span>
      <button class="clear-btn" on:click={clearLogs}>Clear</button>
    </div>
  </div>

  <div class="filters-toolbar">
    <div class="level-filters" aria-label="Level filters">
      <label class="lf-chip"><input type="checkbox" bind:checked={showInfo} />Info</label>
      <label class="lf-chip"><input type="checkbox" bind:checked={showWarn} />Warn</label>
      <label class="lf-chip"><input type="checkbox" bind:checked={showError} />Error</label>
      <label class="lf-chip"><input type="checkbox" bind:checked={showSuccess} />Success</label>
      <label class="lf-chip"><input type="checkbox" bind:checked={showDebug} />Debug</label>
    </div>

    {#if categories.length > 0}
      <div class="category-filters" aria-label="Category filters">
        <button class="cf-btn" type="button" on:click={enableAllCategories}>All</button>
        <button class="cf-btn" type="button" on:click={disableAllCategories}>None</button>
        {#each categories as category}
          <button
            class="cf-btn"
            class:active={categoryEnabled(category)}
            type="button"
            on:click={() => toggleCategory(category)}
            aria-pressed={categoryEnabled(category)}
          >{category}</button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="log-scroll" bind:this={scrollContainer}>
    {#if filteredEntries.length === 0}
      <p class="hint">System messages and computation results will appear here.</p>
    {:else}
      {#each filteredEntries as entry}
        <div
          class="log-entry"
          class:compact-wrap={compactWrap}
          class:error={entry.level === "error"}
          class:warn={entry.level === "warn"}
          class:success={entry.level === "success"}
          class:debug={entry.level === "debug"}
        >
          <span class="log-ts">[{entry.timestamp}]</span>
          <span class="log-cat">[{entry.category}]</span>
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
    grid-template-columns: 5.4rem 8.2rem minmax(0, 1fr);
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
    overflow-wrap: anywhere;
    text-indent: 0;
  }

  .log-cat {
    color: color-mix(in srgb, var(--hl-symbol) 72%, var(--fg-secondary));
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .log-entry:not(.compact-wrap) .log-msg {
    padding-left: 0;
  }

  .log-entry:not(.compact-wrap) .log-msg :global(br) {
    line-height: inherit;
  }

  .log-entry.compact-wrap {
    display: block;
  }

  .log-entry.compact-wrap .log-ts {
    display: inline;
    margin-right: 0.25rem;
  }

  .log-entry.compact-wrap .log-cat {
    display: inline;
    margin-right: 0.25rem;
  }

  .log-entry.compact-wrap .log-msg {
    display: inline;
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

  .filters-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.55rem 0.8rem;
    flex-wrap: wrap;
    margin-bottom: 0.35rem;
  }

  .level-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0;
  }

  .category-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.28rem;
    margin-bottom: 0;
    justify-content: flex-end;
  }

  .cf-btn {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    font-size: 0.64rem;
    padding: 0.06rem 0.35rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .cf-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  .cf-btn.active {
    border-color: color-mix(in srgb, var(--hl-symbol) 55%, var(--border));
    color: var(--fg-primary);
    background: color-mix(in srgb, var(--bg-surface) 78%, var(--hl-symbol) 22%);
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
    width: 0.72rem;
    height: 0.72rem;
    accent-color: var(--hl-symbol);
    cursor: pointer;
  }

  .lf-chip:has(input:checked) {
    border-color: color-mix(in srgb, var(--hl-symbol) 55%, var(--border));
    color: var(--fg-primary);
    background: color-mix(in srgb, var(--bg-surface) 78%, var(--hl-symbol) 22%);
  }

  @media (max-width: 560px) {
    .log-entry {
      display: block;
    }

    .filters-toolbar {
      gap: 0.35rem;
    }

    .category-filters {
      justify-content: flex-start;
    }
  }

  @media (min-width: 980px) {
    .filters-toolbar {
      flex-wrap: nowrap;
      align-items: center;
    }

    .level-filters {
      flex: 0 0 auto;
    }

    .category-filters {
      flex: 1 1 auto;
      justify-content: flex-end;
    }
  }
</style>
