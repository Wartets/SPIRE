<!--
  SPIRE - References Panel

  Dynamically populated bibliography widget.  As the user performs
  calculations (constructing reactions, generating diagrams, evaluating
  amplitudes), the CitationRegistry accumulates relevant academic
  references.  This panel renders them in Physical Review style.

  The panel is a standard SPIRE widget and can be placed anywhere on
  the grid canvas like any other widget type.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { longpress } from "$lib/actions/longpress";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import {
    activeCitations,
    activeCitationCount,
    clearCitations,
    formatCitation,
    getAllCitations,
  } from "$lib/core/services/CitationRegistry";
  import type { Citation } from "$lib/core/services/CitationRegistry";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { publishWidgetInterop } from "$lib/stores/widgetInteropStore";

  let copyMsg = "";
  const totalReferenceLibrary = getAllCitations().length;

  const REF_CMD_IDS = [
    "spire.references.clear",
    "spire.references.copy_all",
  ];

  onMount(() => {
    registerCommand({
      id: "spire.references.clear",
      title: "Clear All References",
      category: "References",
      execute: () => clearCitations(),
    });
    registerCommand({
      id: "spire.references.copy_all",
      title: "Copy References to Clipboard",
      category: "References",
      execute: () => handleCopyAll(),
    });
  });

  onDestroy(() => {
    for (const id of REF_CMD_IDS) {
      unregisterCommand(id);
    }
  });

  function handleClear(): void {
    clearCitations();
  }

  async function handleCopyAll(): Promise<void> {
    const text = $activeCitations
      .map((c, i) => `[${i + 1}] ${formatCitation(c)}`)
      .join("\n\n");

    try {
      await navigator.clipboard.writeText(text);
      copyMsg = "Copied!";
      setTimeout(() => (copyMsg = ""), 2000);
    } catch {
      copyMsg = "Copy failed";
      setTimeout(() => (copyMsg = ""), 2000);
    }
  }

  function arxivUrl(c: Citation): string | null {
    if (!c.arxiv) return null;
    const id = c.arxiv.replace(/^arxiv:/i, "");
    return `https://arxiv.org/abs/${id}`;
  }

  function doiUrl(c: Citation): string | null {
    if (!c.doi) return null;
    return `https://doi.org/${c.doi}`;
  }

  function inspireUrl(c: Citation): string {
    const title = encodeURIComponent(c.title);
    return `https://inspirehep.net/literature?sort=mostrecent&size=25&page=1&q=${title}`;
  }

  function scholarUrl(c: Citation): string {
    const title = encodeURIComponent(c.title);
    return `https://scholar.google.com/scholar?q=${title}`;
  }

  function openCitationContext(citation: Citation, idx: number, x: number, y: number): void {
    const plainText = `[${idx + 1}] ${formatCitation(citation)}`;
    const arxiv = arxivUrl(citation);
    const doi = doiUrl(citation);
    showContextMenu(x, y, [
      {
        type: "action",
        id: `citation-copy-${idx}`,
        label: "Copy Citation",
        icon: "CPY",
        action: () => navigator.clipboard.writeText(plainText),
      },
      {
        type: "action",
        id: `citation-web-${idx}`,
        label: "Search on INSPIRE",
        icon: "WEB",
        action: () => window.open(inspireUrl(citation), "_blank", "noopener,noreferrer"),
      },
      {
        type: "action",
        id: `citation-scholar-${idx}`,
        label: "Search on Google Scholar",
        icon: "WEB",
        action: () => window.open(scholarUrl(citation), "_blank", "noopener,noreferrer"),
      },
      { type: "separator", id: `citation-sep-${idx}` },
      {
        type: "action",
        id: `citation-open-arxiv-${idx}`,
        label: "Open arXiv",
        icon: "arX",
        action: () => arxiv && window.open(arxiv, "_blank", "noopener,noreferrer"),
        disabled: !arxiv,
      },
      {
        type: "action",
        id: `citation-open-doi-${idx}`,
        label: "Open DOI",
        icon: "DOI",
        action: () => doi && window.open(doi, "_blank", "noopener,noreferrer"),
        disabled: !doi,
      },
    ]);
  }

  $: publishWidgetInterop("references", {
    count: $activeCitationCount,
    topCitation: $activeCitations[0]?.title ?? null,
  });
</script>

<div class="references-panel">
  <div class="ref-header">
    <h3>References ({$activeCitationCount})</h3>
    <div class="ref-actions">
      {#if copyMsg}
        <span class="copy-msg">{copyMsg}</span>
      {/if}
      <button
        class="ref-btn"
        on:click={handleCopyAll}
        disabled={$activeCitationCount === 0}
        use:tooltip={{ text: "Copy all references to clipboard" }}
      >
        Copy
      </button>
      <button
        class="ref-btn ref-btn-clear"
        on:click={handleClear}
        disabled={$activeCitationCount === 0}
        use:tooltip={{ text: "Clear active references" }}
      >
        Clear
      </button>
    </div>
  </div>

  <p class="ref-library-note">
    Active bibliography sourced from a curated library of <strong>{totalReferenceLibrary}</strong>
    particle-theory and collider-phenomenology references.
  </p>

  {#if $activeCitationCount === 0}
    <div class="ref-empty">
      <p>
        No active references yet. This panel auto-collects citations as you run model
        import, Feynman-diagram generation, amplitude workflows, RGE studies, and event analysis.
      </p>
      <ul class="ref-empty-tips">
        <li>Run at least one workflow stage (model load, reaction, amplitude, scanner, or RGE) to populate citations.</li>
        <li>Use <kbd>Ctrl</kbd>+<kbd>K</kbd> then “Copy References” for one-click export into notes or manuscripts.</li>
        <li>Right-click any citation for quick arXiv/DOI/INSPIRE actions.</li>
        <li>Use citation context notes below each entry to justify method choices in reports.</li>
      </ul>
    </div>
  {:else}
    <p class="ref-hint">Tip: right-click (or long-press) entries for citation actions and direct paper links.</p>
    <ol class="ref-list">
      {#each $activeCitations as citation, idx}
        <li
          class="ref-item"
          on:contextmenu={(event) => {
            event.preventDefault();
            event.stopPropagation();
            openCitationContext(citation, idx, event.clientX, event.clientY);
          }}
          use:longpress={{
            duration: 460,
            moveTolerance: 12,
            onLongPress: (detail) => openCitationContext(citation, idx, detail.x, detail.y),
          }}
        >
          <div class="ref-citation">
            <span class="ref-idx">[{idx + 1}]</span>
            <span class="ref-authors">{citation.authors.length <= 2 ? citation.authors.join(" and ") : `${citation.authors[0]} et al.`}</span>
            <span class="ref-title">"{citation.title}"</span>
            {#if citation.journal}
              <span class="ref-journal">{citation.journal},</span>
            {/if}
            {#if citation.volume}
              <span class="ref-volume">{citation.volume},</span>
            {/if}
            {#if citation.pages}
              <span class="ref-pages">{citation.pages},</span>
            {/if}
            <span class="ref-year">({citation.year})</span>
          </div>
          <div class="ref-links">
            {#if citation.arxiv}
              <a class="ref-link" href={arxivUrl(citation)} target="_blank" rel="noopener" use:tooltip={{ text: "Open arXiv" }}>
                arXiv:{citation.arxiv}
              </a>
            {/if}
            {#if citation.doi}
              <a class="ref-link" href={doiUrl(citation)} target="_blank" rel="noopener" use:tooltip={{ text: "Open DOI" }}>
                DOI
              </a>
            {/if}
            <a class="ref-link" href={inspireUrl(citation)} target="_blank" rel="noopener" use:tooltip={{ text: "Search on INSPIRE" }}>
              INSPIRE
            </a>
          </div>
          {#if citation.context}
            <p class="ref-context">{citation.context}</p>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .references-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100%;
    overflow: hidden;
  }

  .ref-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .ref-header h3 {
    margin: 0;
    font-size: 0.88rem;
    color: var(--fg-accent);
  }

  .ref-library-note {
    margin: 0;
    font-size: 0.68rem;
    color: var(--fg-secondary);
    line-height: 1.35;
  }
  .ref-actions {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .ref-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.15rem 0.4rem;
    font-size: 0.65rem;
    cursor: pointer;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .ref-btn:hover:not(:disabled) {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }
  .ref-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .ref-btn-clear:hover:not(:disabled) {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }
  .copy-msg {
    font-size: 0.62rem;
    color: var(--hl-success);
  }

  .ref-empty {
    color: var(--fg-secondary);
    font-size: 0.75rem;
    margin: 1rem 0;
    line-height: 1.5;
  }

  .ref-empty p {
    margin: 0 0 0.45rem;
    font-style: italic;
  }

  .ref-empty-tips {
    margin: 0;
    padding-left: 1rem;
    display: grid;
    gap: 0.2rem;
    font-size: 0.7rem;
  }

  .ref-empty-tips kbd {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.05rem 0.25rem;
  }

  .ref-hint {
    margin: 0;
    font-size: 0.66rem;
    color: var(--fg-secondary);
    font-style: italic;
  }

  .ref-list {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .ref-item {
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border);
  }
  .ref-item:last-child {
    border-bottom: none;
  }

  .ref-citation {
    font-size: 0.73rem;
    line-height: 1.5;
    color: var(--fg-primary);
  }
  .ref-idx {
    font-weight: 700;
    color: var(--hl-symbol);
    margin-right: 0.35rem;
  }
  .ref-authors {
    font-weight: 600;
  }
  .ref-title {
    font-style: italic;
    color: var(--fg-accent);
  }
  .ref-journal {
    color: var(--fg-secondary);
  }
  .ref-volume {
    font-weight: 700;
  }
  .ref-pages,
  .ref-year {
    color: var(--fg-secondary);
  }

  .ref-links {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.2rem;
  }
  .ref-link {
    font-size: 0.65rem;
    color: var(--hl-symbol);
    text-decoration: none;
    border-bottom: 1px dotted var(--hl-symbol);
  }
  .ref-link:hover {
    color: var(--fg-accent);
    border-bottom-color: var(--fg-accent);
  }

  .ref-context {
    margin: 0.2rem 0 0;
    font-size: 0.67rem;
    color: var(--fg-secondary);
    font-style: normal;
    line-height: 1.4;
    padding-left: 0.1rem;
    border-left: 2px solid color-mix(in srgb, var(--hl-symbol) 40%, transparent);
  }
</style>
