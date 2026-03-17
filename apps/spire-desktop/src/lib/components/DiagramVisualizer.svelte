<!--
  SPIRE - Diagram Visualizer Component (v2)

  Multi-mode renderer for Feynman diagrams with four view modes:

    1. **Feynman** (default) — Physics-standard SVG rendering with proper
       propagator line styles (wavy photons, curly gluons, dashed scalars,
       arrowed fermions) and conventional vertex placement.

    2. **Worldsheet** — String-theory representation where particle
       worldlines become tubes/ribbons that join and split at vertices,
       visualising the smooth UV behaviour of string amplitudes.

    3. **Mermaid** — Flowchart-based rendering via Mermaid.js (legacy).

    4. **Text** — Plain-text ASCII fallback with full node/edge listing.

  Additional features:
    • Export to LaTeX TikZ (tikz-feynman package)
    • Export SVG as image
    • Diagram metadata panel (channels, loop order, symmetry factor)
    • Zoom and pan within the diagram viewport
-->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { generatedDiagrams } from "$lib/stores/physicsStore";
  import type {
    FeynmanDiagram,
    FeynmanNode,
    FeynmanEdge,
    NodeKind,
    LoopOrder,
    OneLoopTopologyKind,
  } from "$lib/types/spire";
  import { renderFeynmanDiagram } from "./diagram/feynmanRenderer";
  import { renderWorldsheet } from "./diagram/worldsheetRenderer";
  import { generateTikZ, generateTikZDocument } from "./diagram/tikzExport";
  import { publishWidgetInterop } from "$lib/stores/widgetInteropStore";

  // ── View Mode ──────────────────────────────────────────────────────────

  type ViewMode = "feynman" | "worldsheet" | "mermaid" | "text";
  let viewMode: ViewMode = "feynman";

  /** Currently selected diagram index. */
  let selectedIdx: number = 0;

  /** Container for rendered diagram (SVG or Mermaid). */
  let renderContainer: HTMLDivElement;

  /** Container specifically for Mermaid rendering (separate to avoid conflicts). */
  let mermaidContainer: HTMLDivElement;

  /** Whether Mermaid has been initialized. */
  let mermaidReady = false;

  /** Whether the TikZ export overlay is shown. */
  let showTikzExport = false;

  /** Whether to show the full standalone document. */
  let tikzStandalone = false;

  /** Zoom level for SVG renderers. */
  let diagramZoom = 1;

  /** Track last rendered diagram ID + viewMode to avoid unnecessary re-renders. */
  let lastRenderedKey = "";

  // ── Reactive Data ──────────────────────────────────────────────────────

  $: diagrams = $generatedDiagrams?.diagrams ?? [];
  $: selectedDiagram = diagrams[selectedIdx] ?? null;
  $: {
    // Reset index when diagram list changes
    if (selectedIdx >= diagrams.length && diagrams.length > 0) {
      selectedIdx = 0;
    }
  }

  // ── Render Orchestration ───────────────────────────────────────────────
  // Use a keyed reactive block to render only when diagram or mode changes.

  $: renderKey = selectedDiagram
    ? `${selectedDiagram.id}-${viewMode}-${diagramZoom}`
    : "";

  $: if (renderKey && renderKey !== lastRenderedKey) {
    // Schedule render after DOM update
    scheduleRender();
  }

  async function scheduleRender(): Promise<void> {
    await tick();
    if (!selectedDiagram) return;

    const key = `${selectedDiagram.id}-${viewMode}-${diagramZoom}`;
    if (key === lastRenderedKey) return;
    lastRenderedKey = key;

    switch (viewMode) {
      case "feynman":
        renderFeynman(selectedDiagram);
        break;
      case "worldsheet":
        renderWorldsheetView(selectedDiagram);
        break;
      case "mermaid":
        await renderMermaidView(selectedDiagram);
        break;
      case "text":
        // Text is handled directly in the template
        break;
    }
  }

  // ── Feynman Renderer ──────────────────────────────────────────────────

  function renderFeynman(diag: FeynmanDiagram): void {
    if (!renderContainer) return;
    const svgW = Math.round(520 * diagramZoom);
    const svgH = Math.round(340 * diagramZoom);
    const svg = renderFeynmanDiagram(diag, { width: svgW, height: svgH });
    renderContainer.innerHTML = svg;
  }

  // ── Worldsheet Renderer ──────────────────────────────────────────────

  function renderWorldsheetView(diag: FeynmanDiagram): void {
    if (!renderContainer) return;
    const svgW = Math.round(520 * diagramZoom);
    const svgH = Math.round(340 * diagramZoom);
    const svg = renderWorldsheet(diag, { width: svgW, height: svgH });
    renderContainer.innerHTML = svg;
  }

  // ── Mermaid Renderer (legacy) ─────────────────────────────────────────

  onMount(async () => {
    try {
      const mermaid = (await import("mermaid")).default;
      mermaid.initialize({
        startOnLoad: false,
        theme: "dark",
        themeVariables: {
          primaryColor: "#162447",
          primaryBorderColor: "#1a5276",
          primaryTextColor: "#e8e8e8",
          lineColor: "#5dade2",
          secondaryColor: "#0f3460",
          tertiaryColor: "#0d1b2a",
          fontFamily: '"Fira Code", "Cascadia Code", monospace',
          fontSize: "12px",
        },
        flowchart: {
          htmlLabels: true,
          curve: "basis",
          padding: 12,
          nodeSpacing: 50,
          rankSpacing: 60,
        },
      });
      mermaidReady = true;
    } catch (e) {
      console.warn("Mermaid init failed:", e);
    }
  });

  function graphToMermaid(diag: FeynmanDiagram): string {
    const lines: string[] = ["graph LR"];
    const esc = (s: string): string =>
      s.replace(/"/g, "#quot;").replace(/\\/g, "#92;").replace(/[<>{}]/g, "");

    for (const node of diag.nodes) {
      const id = `N${node.id}`;
      const kind = node.kind;
      if ("ExternalIncoming" in kind) {
        lines.push(`    ${id}(["${esc(kind.ExternalIncoming.field.symbol)} →"]):::incoming`);
      } else if ("ExternalOutgoing" in kind) {
        lines.push(`    ${id}(["→ ${esc(kind.ExternalOutgoing.field.symbol)}"]):::outgoing`);
      } else if ("InternalVertex" in kind) {
        lines.push(`    ${id}(("V")):::vertex`);
      }
    }

    for (const [src, tgt, edge] of diag.edges) {
      const particle = esc(edge.field.symbol);
      const mom = esc(edge.momentum_label);
      const label = mom ? `${particle} [${mom}]` : particle;
      if (edge.is_external) {
        lines.push(`    N${src} -. "${label}" .-> N${tgt}`);
      } else {
        lines.push(`    N${src} -- "${label}" --> N${tgt}`);
      }
    }

    lines.push("");
    lines.push("    classDef incoming fill:#0a3d2e,stroke:#2ecc71,stroke-width:2px,color:#2ecc71");
    lines.push("    classDef outgoing fill:#3d0a0a,stroke:#e74c3c,stroke-width:2px,color:#e74c3c");
    lines.push("    classDef vertex fill:#162447,stroke:#5dade2,stroke-width:2px,color:#f39c12");
    return lines.join("\n");
  }

  async function renderMermaidView(diag: FeynmanDiagram): Promise<void> {
    if (!mermaidContainer || !mermaidReady) return;
    try {
      const mermaid = (await import("mermaid")).default;
      const definition = graphToMermaid(diag);
      const uniqueId = `spire-diagram-${diag.id}-${Date.now()}`;
      mermaidContainer.innerHTML = "";
      const { svg } = await mermaid.render(uniqueId, definition);
      mermaidContainer.innerHTML = svg;
    } catch (e) {
      console.warn("Mermaid render error:", e);
      if (mermaidContainer) {
        mermaidContainer.innerHTML = `<pre style="color:#e74c3c;font-size:0.75rem;">Mermaid render error</pre>`;
      }
    }
  }

  // ── Text Renderer ─────────────────────────────────────────────────────

  function nodeLabel(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return `→ ${kind.ExternalIncoming.field.symbol} (in)`;
    if ("ExternalOutgoing" in kind) return `${kind.ExternalOutgoing.field.symbol} → (out)`;
    if ("InternalVertex" in kind) return `vertex [${kind.InternalVertex.field_ids.join(", ")}]`;
    return "?";
  }

  function loopLabel(lo: LoopOrder): string {
    if (lo === "Tree") return "Tree";
    if (lo === "OneLoop") return "1-Loop";
    if (lo === "TwoLoop") return "2-Loop";
    if (typeof lo === "object" && "NLoop" in lo) return `${lo.NLoop}-Loop`;
    return String(lo);
  }

  function topologyLabel(kind: OneLoopTopologyKind): string {
    if (kind === "Tadpole") return "Tadpole (A₀)";
    if (kind === "Bubble") return "Bubble (B₀)";
    if (kind === "Triangle") return "Triangle (C₀)";
    if (kind === "Box") return "Box (D₀)";
    if (typeof kind === "object" && "NPoint" in kind) return `${kind.NPoint}-Point`;
    return String(kind);
  }

  function renderDiagramText(diag: FeynmanDiagram): string {
    const lines: string[] = [];
    lines.push(`Diagram #${diag.id}  |  ${loopLabel(diag.loop_order)}  |  S = ${diag.symmetry_factor}`);
    lines.push(`Channels: ${diag.channels.join(", ") || "-"}`);
    lines.push(`Connected: ${diag.is_connected ? "yes" : "no"}`);
    lines.push("─".repeat(40));
    lines.push("Nodes:");
    for (const node of diag.nodes) {
      lines.push(`  [${node.id}] ${nodeLabel(node.kind)}`);
    }
    lines.push("");
    lines.push("Edges:");
    for (const [src, tgt, edge] of diag.edges) {
      lines.push(`  ${src} --${edge.field.symbol}(${edge.momentum_label})--> ${tgt}${edge.is_external ? " (ext)" : ""}`);
    }
    if (diag.loop_topology_kind) {
      lines.push(`\nLoop Topology: ${topologyLabel(diag.loop_topology_kind)}`);
    }
    if (diag.is_one_particle_irreducible) lines.push("1PI: yes");
    if (diag.momentum_routing) lines.push(`Loop Momenta: ${diag.momentum_routing.loop_momenta.join(", ")}`);
    return lines.join("\n");
  }

  $: diagramText = selectedDiagram ? renderDiagramText(selectedDiagram) : "";
  $: tikzCode = selectedDiagram
    ? (tikzStandalone ? generateTikZDocument(selectedDiagram) : generateTikZ(selectedDiagram))
    : "";

  // ── Export Actions ────────────────────────────────────────────────────

  function copyTikZ(): void {
    if (!tikzCode) return;
    navigator.clipboard.writeText(tikzCode).catch(() => {});
  }

  function exportSvg(): void {
    if (!selectedDiagram || !renderContainer) return;
    const svg = renderContainer.innerHTML;
    const blob = new Blob([svg], { type: "image/svg+xml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `feynman-diagram-${selectedDiagram.id}.svg`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function zoomIn(): void { diagramZoom = Math.min(3, diagramZoom + 0.2); }
  function zoomOut(): void { diagramZoom = Math.max(0.4, diagramZoom - 0.2); }
  function zoomReset(): void { diagramZoom = 1; }

  /** Handle wheel events on the SVG viewport for smooth zoom. */
  function handleSvgWheel(event: WheelEvent): void {
    event.preventDefault();
    event.stopPropagation();
    const step = event.deltaY > 0 ? -0.1 : 0.1;
    diagramZoom = Math.min(3, Math.max(0.4, diagramZoom + step));
  }

  $: if (selectedDiagram) {
    publishWidgetInterop("diagram", {
      selectedDiagramId: selectedDiagram.id,
      loopOrder: loopLabel(selectedDiagram.loop_order),
      channelSummary: selectedDiagram.channels.join("+") || "-",
      edgeCount: selectedDiagram.edges.length,
      nodeCount: selectedDiagram.nodes.length,
      viewMode,
    });
  }
</script>

<div class="diagram-viz" data-tour-id="diagram-visualizer">
  <!-- Header -->
  <div class="dv-header">
    <h3>Feynman Diagrams</h3>
    {#if diagrams.length > 0}
      <div class="dv-zoom">
        <button class="icon-btn" on:click={zoomOut} use:tooltip={{ text: "Zoom Out" }}>−</button>
        <button class="icon-btn zoom-label" on:click={zoomReset} use:tooltip={{ text: "Reset Zoom" }}>{Math.round(diagramZoom * 100)}%</button>
        <button class="icon-btn" on:click={zoomIn} use:tooltip={{ text: "Zoom In" }}>+</button>
      </div>
    {/if}
  </div>

  {#if diagrams.length === 0}
    <p class="hint">No diagrams generated yet. Run the pipeline to generate topologies.</p>
  {:else}
    <!-- View Mode Tabs -->
    <div class="mode-bar">
      <div class="mode-tabs">
        <button class="mode-btn" class:active={viewMode === "feynman"} on:click={() => { viewMode = "feynman"; lastRenderedKey = ""; }}>
          <span class="mode-icon">⟨ ⟩</span> Feynman
        </button>
        <button class="mode-btn" class:active={viewMode === "worldsheet"} on:click={() => { viewMode = "worldsheet"; lastRenderedKey = ""; }}>
          <span class="mode-icon">≋</span> String
        </button>
        <button class="mode-btn" class:active={viewMode === "mermaid"} on:click={() => { viewMode = "mermaid"; lastRenderedKey = ""; }}>
          <span class="mode-icon">⊞</span> Graph
        </button>
        <button class="mode-btn" class:active={viewMode === "text"} on:click={() => { viewMode = "text"; lastRenderedKey = ""; }}>
          <span class="mode-icon">≡</span> Text
        </button>
      </div>
      <div class="export-btns">
        <button class="export-btn" on:click={() => { showTikzExport = !showTikzExport; }} use:tooltip={{ text: "LaTeX TikZ Export" }}>
          TikZ
        </button>
        {#if viewMode === "feynman" || viewMode === "worldsheet"}
          <button class="export-btn" on:click={exportSvg} use:tooltip={{ text: "Download SVG" }}>
            SVG ↓
          </button>
        {/if}
      </div>
    </div>

    <!-- Diagram Selector (tabs for each diagram) -->
    {#if diagrams.length > 1}
      <div class="diagram-tabs">
        {#each diagrams as diag, idx}
          <button
            class="tab-btn"
            class:active={idx === selectedIdx}
            on:click={() => { selectedIdx = idx; lastRenderedKey = ""; }}
          >
            #{diag.id}
            <span class="tab-channel">{diag.channels[0] ?? ""}</span>
          </button>
        {/each}
      </div>
    {/if}

    <!-- Summary Bar -->
    <div class="summary-bar">
      <span>{diagrams.length} diagram{diagrams.length !== 1 ? "s" : ""}</span>
      {#if $generatedDiagrams}
        <span class="sep">·</span>
        <span>max: {loopLabel($generatedDiagrams.max_loop_order)}</span>
      {/if}
      {#if selectedDiagram}
        <span class="sep">·</span>
        <span>{loopLabel(selectedDiagram.loop_order)}</span>
        <span class="sep">·</span>
        <span>S = {selectedDiagram.symmetry_factor}</span>
        <span class="sep">·</span>
        <span>Ch: {selectedDiagram.channels.join("+") || "—"}</span>
        {#if selectedDiagram.loop_topology_kind}
          <span class="sep">·</span>
          <span>{topologyLabel(selectedDiagram.loop_topology_kind)}</span>
        {/if}
        {#if selectedDiagram.is_one_particle_irreducible}
          <span class="tag-1pi">1PI</span>
        {/if}
      {/if}
    </div>

    <!-- TikZ Export Overlay -->
    {#if showTikzExport && selectedDiagram}
      <div class="tikz-overlay">
        <div class="tikz-header">
          <span class="tikz-title">LaTeX TikZ Export</span>
          <label class="tikz-toggle">
            <input type="checkbox" bind:checked={tikzStandalone} />
            Standalone document
          </label>
          <button class="copy-btn" on:click={copyTikZ}>Copy</button>
          <button class="close-btn" on:click={() => (showTikzExport = false)}>✕</button>
        </div>
        <pre class="tikz-code">{tikzCode}</pre>
      </div>
    {/if}

    <!-- Diagram Canvas -->
    {#if selectedDiagram}
      <div class="diagram-canvas">
        {#if viewMode === "text"}
          <pre class="diagram-text">{diagramText}</pre>
        {:else if viewMode === "mermaid"}
          <div class="mermaid-container" bind:this={mermaidContainer}>
            {#if !mermaidReady}
              <p class="hint">Initialising Mermaid renderer…</p>
            {/if}
          </div>
        {:else}
          <!-- Feynman / Worldsheet SVG -->
          <div class="svg-container" bind:this={renderContainer} on:wheel={handleSvgWheel} data-wheel-capture></div>
        {/if}
      </div>

      <!-- Edge Table -->
      <details class="edge-details">
        <summary>Propagators ({selectedDiagram.edges.length})</summary>
        <div class="edge-table">
          <div class="edge-header">
            <span>Src</span><span>→</span><span>Tgt</span><span>Particle</span><span>Momentum</span><span>Type</span>
          </div>
          {#each selectedDiagram.edges as [src, tgt, edge]}
            <div class="edge-row">
              <span>{src}</span>
              <span>→</span>
              <span>{tgt}</span>
              <span class="particle-name">{edge.field.symbol}</span>
              <span class="momentum">{edge.momentum_label}</span>
              <span class="edge-type">{edge.is_external ? "ext" : "int"}</span>
            </div>
          {/each}
        </div>
      </details>
    {/if}
  {/if}
</div>

<style>
  .diagram-viz {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100%;
    overflow: hidden;
  }

  /* ── Header ── */
  .dv-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.25rem;
  }

  .dv-header h3 {
    margin: 0;
    font-size: 0.85rem;
    color: var(--fg-accent);
  }

  .dv-zoom {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .icon-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    width: 22px;
    height: 22px;
    font-size: 0.75rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    font-family: var(--font-mono);
  }
  .icon-btn:hover { border-color: var(--border-focus); }
  .zoom-label {
    width: auto;
    padding: 0 0.3rem;
    font-size: 0.65rem;
    color: var(--fg-secondary);
  }

  /* ── Hint ── */
  .hint {
    font-size: 0.8rem;
    color: var(--fg-secondary);
    font-style: italic;
    margin: 0;
  }

  /* ── Mode Bar ── */
  .mode-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem;
  }

  .mode-tabs {
    display: flex;
    gap: 2px;
  }

  .mode-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.15rem 0.45rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
    display: flex;
    align-items: center;
    gap: 0.2rem;
    transition: border-color 0.1s, color 0.1s;
  }

  .mode-btn:hover {
    color: var(--fg-primary);
    border-color: var(--border-focus);
  }

  .mode-btn.active {
    background: var(--bg-surface);
    border-color: var(--hl-symbol);
    color: var(--hl-symbol);
  }

  .mode-icon {
    font-size: 0.7rem;
    opacity: 0.7;
  }

  .export-btns {
    display: flex;
    gap: 2px;
  }

  .export-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.15rem 0.45rem;
    font-size: 0.65rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.1s, color 0.1s;
  }

  .export-btn:hover {
    color: var(--hl-value);
    border-color: var(--hl-value);
  }

  /* ── Diagram Tabs ── */
  .diagram-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }

  .tab-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.15rem 0.4rem;
    font-size: 0.7rem;
    cursor: pointer;
    font-family: var(--font-mono);
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .tab-btn.active {
    background: var(--bg-surface);
    border-color: var(--fg-accent);
    color: var(--fg-accent);
  }
  .tab-channel {
    font-size: 0.6rem;
    color: var(--hl-symbol);
    opacity: 0.7;
  }

  /* ── Summary Bar ── */
  .summary-bar {
    font-size: 0.7rem;
    color: var(--fg-secondary);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.2rem;
    padding: 0.1rem 0;
  }
  .sep { opacity: 0.4; }
  .tag-1pi {
    background: var(--bg-surface);
    border: 1px solid var(--hl-symbol);
    color: var(--hl-symbol);
    font-size: 0.6rem;
    padding: 0 0.25rem;
    margin-left: 0.2rem;
  }

  /* ── TikZ Export Overlay ── */
  .tikz-overlay {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    max-height: 200px;
    overflow: hidden;
  }

  .tikz-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.4rem;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
  }

  .tikz-title {
    font-size: 0.72rem;
    font-weight: bold;
    color: var(--fg-accent);
    flex: 1;
  }

  .tikz-toggle {
    font-size: 0.65rem;
    color: var(--fg-secondary);
    display: flex;
    align-items: center;
    gap: 0.2rem;
    cursor: pointer;
  }

  .tikz-toggle input {
    accent-color: var(--hl-symbol);
  }

  .copy-btn, .close-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.1rem 0.35rem;
    font-size: 0.65rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .copy-btn:hover { color: var(--hl-success); border-color: var(--hl-success); }
  .close-btn:hover { color: var(--hl-error); border-color: var(--hl-error); }

  .tikz-code {
    font-size: 0.68rem;
    line-height: 1.4;
    color: var(--fg-primary);
    overflow: auto;
    padding: 0.4rem;
    margin: 0;
    white-space: pre;
    flex: 1;
    min-height: 0;
  }

  /* ── Diagram Canvas ── */
  .diagram-canvas {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: auto;
    min-height: 120px;
  }

  .svg-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    overflow: auto;
    min-height: 160px;
    padding: 0.5rem;
  }

  .svg-container :global(svg) {
    max-width: 100%;
    height: auto;
  }

  .diagram-text {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.5rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: var(--fg-primary);
    overflow: auto;
    margin: 0;
    white-space: pre;
    flex: 1;
  }

  .mermaid-container {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.75rem;
    flex: 1;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 160px;
  }
  .mermaid-container :global(svg) { max-width: 100%; height: auto; }
  .mermaid-container :global(.edgeLabel) {
    background-color: var(--bg-inset) !important;
    color: var(--hl-value) !important;
    font-family: var(--font-mono) !important;
    font-size: 0.72rem !important;
  }
  .mermaid-container :global(.edgePaths path) {
    stroke: var(--hl-symbol) !important;
    stroke-width: 1.5px !important;
  }

  /* ── Edge Table ── */
  .edge-details {
    border: 1px solid var(--border);
    font-size: 0.7rem;
  }

  .edge-details > summary {
    padding: 0.25rem 0.4rem;
    background: var(--bg-surface);
    color: var(--fg-secondary);
    cursor: pointer;
    font-size: 0.68rem;
    font-family: var(--font-mono);
    user-select: none;
  }

  .edge-details > summary:hover {
    color: var(--fg-primary);
  }

  .edge-table {
    max-height: 120px;
    overflow-y: auto;
  }

  .edge-header,
  .edge-row {
    display: grid;
    grid-template-columns: 2rem 1.2rem 2rem 1fr 1fr 2.5rem;
    gap: 0.2rem;
    padding: 0.15rem 0.4rem;
    align-items: center;
  }

  .edge-header {
    background: var(--bg-surface);
    color: var(--fg-secondary);
    text-transform: uppercase;
    font-size: 0.6rem;
    letter-spacing: 0.05em;
    position: sticky;
    top: 0;
  }

  .edge-row {
    border-bottom: 1px solid var(--border);
    color: var(--fg-primary);
  }
  .edge-row:last-child { border-bottom: none; }
  .particle-name { color: var(--hl-value); }
  .momentum { color: var(--hl-symbol); }
  .edge-type { color: var(--fg-secondary); font-size: 0.6rem; }
</style>
