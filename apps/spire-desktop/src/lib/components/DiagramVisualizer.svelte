<!--
  SPIRE — Diagram Visualizer Component

  Renders Feynman diagrams graphically using Mermaid.js with a
  text-based ASCII fallback toggle. Each diagram's graph structure,
  channel classification, loop order, and symmetry factor are displayed.

  Mermaid renders a Left-to-Right flowchart where:
    • ExternalIncoming nodes appear on the left  (stadium shape)
    • Vertex nodes appear in the centre           (circle)
    • ExternalOutgoing nodes appear on the right  (stadium shape)
    • Edges are arrows labelled with particle symbol + momentum
-->
<script lang="ts">
  import { onMount, afterUpdate, tick } from "svelte";
  import { generatedDiagrams } from "$lib/stores/physicsStore";
  import type {
    FeynmanDiagram,
    FeynmanNode,
    FeynmanEdge,
    NodeKind,
    LoopOrder,
    OneLoopTopologyKind,
  } from "$lib/types/spire";

  /** Currently selected diagram index. */
  let selectedIdx: number = 0;

  /** Toggle between Mermaid graphical view and text fallback. */
  let showText: boolean = false;

  /** Container element for Mermaid SVG output. */
  let mermaidContainer: HTMLDivElement;

  /** Flag to track if mermaid has been initialized. */
  let mermaidReady: boolean = false;

  // ── Mermaid initialisation (runs once on mount) ──────────────────────

  onMount(async () => {
    try {
      const mermaid = (await import("mermaid")).default;
      mermaid.initialize({
        startOnLoad: false,
        theme: "dark",
        themeVariables: {
          primaryColor: "#162447",
          primaryBorderColor: "#1a5276",
          primaryTextColor: "#e0e0e0",
          lineColor: "#5dade2",
          secondaryColor: "#0f3460",
          tertiaryColor: "#0d1b2a",
          fontFamily: '"Fira Code", "Cascadia Code", monospace',
          fontSize: "12px",
          nodeBorder: "#2980b9",
          mainBkg: "#162447",
          clusterBkg: "#0a0f1a",
          edgeLabelBackground: "#0d1b2a",
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
      console.warn("Mermaid init failed, falling back to text mode:", e);
      showText = true;
    }
  });

  // ── Helper: NodeKind → readable label ────────────────────────────────

  function nodeLabel(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) {
      return `→ ${kind.ExternalIncoming.field.symbol} (in)`;
    }
    if ("ExternalOutgoing" in kind) {
      return `${kind.ExternalOutgoing.field.symbol} → (out)`;
    }
    if ("Vertex" in kind) {
      return `vertex [${kind.Vertex.field_ids.join(", ")}]`;
    }
    return "?";
  }

  /** Short label for Mermaid node text. */
  function nodeShort(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return kind.ExternalIncoming.field.symbol;
    if ("ExternalOutgoing" in kind) return kind.ExternalOutgoing.field.symbol;
    if ("Vertex" in kind) return "V";
    return "?";
  }

  /** Format LoopOrder into a string. */
  function loopLabel(lo: LoopOrder): string {
    if (lo === "Tree") return "Tree";
    if (lo === "OneLoop") return "1-Loop";
    if (lo === "TwoLoop") return "2-Loop";
    if (typeof lo === "object" && "NLoop" in lo) return `${lo.NLoop}-Loop`;
    return String(lo);
  }

  /** Format OneLoopTopologyKind into a human-readable label. */
  function topologyLabel(kind: OneLoopTopologyKind): string {
    if (kind === "Tadpole") return "Tadpole (A₀)";
    if (kind === "Bubble") return "Bubble (B₀)";
    if (kind === "Triangle") return "Triangle (C₀)";
    if (kind === "Box") return "Box (D₀)";
    if (typeof kind === "object" && "NPoint" in kind) return `${kind.NPoint}-Point`;
    return String(kind);
  }

  // ── Mermaid graph generation ─────────────────────────────────────────

  /**
   * Convert a FeynmanDiagram into a Mermaid flowchart definition.
   *
   * Layout: graph LR (left-to-right)
   *  - ExternalIncoming → stadium shape ([...])
   *  - ExternalOutgoing → stadium shape ([...])
   *  - Vertex           → circle ((...))
   *  - Edges            → arrows with particle label
   */
  function graphToMermaid(diag: FeynmanDiagram): string {
    const lines: string[] = ["graph LR"];

    // Sanitise a string for Mermaid (escape quotes and special chars)
    const esc = (s: string): string =>
      s.replace(/"/g, "#quot;")
       .replace(/\\/g, "#92;")
       .replace(/[<>{}]/g, "");

    // ── Node definitions ──
    for (const node of diag.nodes) {
      const id = `N${node.id}`;
      const kind = node.kind;

      if ("ExternalIncoming" in kind) {
        const label = esc(kind.ExternalIncoming.field.symbol);
        // Stadium shape for external incoming
        lines.push(`    ${id}(["${label} →"]):::incoming`);
      } else if ("ExternalOutgoing" in kind) {
        const label = esc(kind.ExternalOutgoing.field.symbol);
        // Stadium shape for external outgoing
        lines.push(`    ${id}(["→ ${label}"]):::outgoing`);
      } else if ("Vertex" in kind) {
        const vf = kind.Vertex;
        const label = esc(vf.field_ids.join(","));
        // Circle for vertex
        lines.push(`    ${id}(("${label}")):::vertex`);
      }
    }

    // ── Edge definitions ──
    for (const [src, tgt, edge] of diag.edges) {
      const particle = esc(edge.particle.field.symbol);
      const mom = esc(edge.momentum_label);
      const label = mom ? `${particle} [${mom}]` : particle;

      if (edge.is_external) {
        // Dashed arrow for external legs
        lines.push(`    N${src} -. "${label}" .-> N${tgt}`);
      } else {
        // Solid arrow for internal propagators
        lines.push(`    N${src} -- "${label}" --> N${tgt}`);
      }
    }

    // ── Class definitions for styling ──
    lines.push("");
    lines.push("    classDef incoming fill:#0a3d2e,stroke:#2ecc71,stroke-width:2px,color:#2ecc71");
    lines.push("    classDef outgoing fill:#3d0a0a,stroke:#e74c3c,stroke-width:2px,color:#e74c3c");
    lines.push("    classDef vertex fill:#162447,stroke:#5dade2,stroke-width:2px,color:#f39c12");

    return lines.join("\n");
  }

  // ── Reactive rendering ───────────────────────────────────────────────

  async function renderMermaid(diag: FeynmanDiagram | null): Promise<void> {
    if (!diag || !mermaidContainer || !mermaidReady || showText) return;

    try {
      const mermaid = (await import("mermaid")).default;
      const definition = graphToMermaid(diag);
      const uniqueId = `spire-diagram-${diag.id}-${Date.now()}`;

      // Clear previous render
      mermaidContainer.innerHTML = "";

      const { svg } = await mermaid.render(uniqueId, definition);
      mermaidContainer.innerHTML = svg;
    } catch (e) {
      console.warn("Mermaid render error:", e);
      // On failure, show the raw definition for debugging
      if (mermaidContainer) {
        mermaidContainer.innerHTML = `<pre style="color:#e74c3c;font-size:0.75rem;">Render error — showing raw definition:\n\n${graphToMermaid(diag)}</pre>`;
      }
    }
  }

  // Trigger re-render when diagram selection or view mode changes
  $: selectedDiagram = ($generatedDiagrams?.diagrams ?? [])[selectedIdx] ?? null;
  $: diagrams = $generatedDiagrams?.diagrams ?? [];

  // Use afterUpdate to re-render after the DOM is ready
  afterUpdate(async () => {
    if (!showText && mermaidReady && selectedDiagram) {
      await tick();
      renderMermaid(selectedDiagram);
    }
  });

  // ── Text-art fallback ────────────────────────────────────────────────

  function renderDiagramText(diag: FeynmanDiagram): string {
    const lines: string[] = [];
    lines.push(`Diagram #${diag.id}  |  ${loopLabel(diag.loop_order)}  |  S = ${diag.symmetry_factor}`);
    lines.push(`Channels: ${diag.channels.join(", ") || "—"}`);
    lines.push(`Connected: ${diag.is_connected ? "yes" : "no"}`);
    lines.push("─".repeat(40));
    lines.push("Nodes:");
    for (const node of diag.nodes) {
      lines.push(`  [${node.id}] ${nodeLabel(node.kind)}`);
    }
    lines.push("");
    lines.push("Edges:");
    for (const [src, tgt, edge] of diag.edges) {
      const pName = edge.particle.field.symbol;
      const mom = edge.momentum_label;
      const ext = edge.is_external ? " (ext)" : "";
      lines.push(`  ${src} --${pName}(${mom})--> ${tgt}${ext}`);
    }
    // Phase 15: show loop topology info if present
    if (diag.loop_topology_kind) {
      lines.push("");
      lines.push(`Loop Topology: ${topologyLabel(diag.loop_topology_kind)}`);
    }
    if (diag.is_one_particle_irreducible) {
      lines.push("1PI: yes");
    }
    if (diag.momentum_routing) {
      lines.push(`Loop Momenta: ${diag.momentum_routing.loop_momenta.join(", ")}`);
    }
    return lines.join("\n");
  }

  $: diagramText = selectedDiagram ? renderDiagramText(selectedDiagram) : "";
</script>

<div class="diagram-viz">
  <h3>Feynman Diagrams</h3>

  {#if diagrams.length === 0}
    <p class="hint">No diagrams generated yet. Run the pipeline to generate topologies.</p>
  {:else}
    <!-- Toolbar: tabs + view toggle -->
    <div class="toolbar">
      <div class="diagram-tabs">
        {#each diagrams as diag, idx}
          <button
            class="tab-btn"
            class:active={idx === selectedIdx}
            on:click={() => (selectedIdx = idx)}
          >
            #{diag.id}
          </button>
        {/each}
      </div>
      <button class="view-toggle" on:click={() => (showText = !showText)}>
        {showText ? "Graph View" : "Text View"}
      </button>
    </div>

    <!-- Summary Bar -->
    <div class="summary-bar">
      {diagrams.length} diagram{diagrams.length !== 1 ? "s" : ""} generated
      {#if $generatedDiagrams}
        — max order: {loopLabel($generatedDiagrams.max_loop_order)}
      {/if}
      {#if selectedDiagram}
        | {loopLabel(selectedDiagram.loop_order)} | S = {selectedDiagram.symmetry_factor}
        | Ch: {selectedDiagram.channels.join(", ") || "—"}
        {#if selectedDiagram.loop_topology_kind}
          | Topology: {topologyLabel(selectedDiagram.loop_topology_kind)}
        {/if}
        {#if selectedDiagram.is_one_particle_irreducible}
          | 1PI
        {/if}
      {/if}
    </div>

    <!-- Diagram Detail -->
    {#if selectedDiagram}
      <div class="diagram-canvas">
        {#if showText}
          <!-- Text-art fallback -->
          <pre class="diagram-text">{diagramText}</pre>
        {:else}
          <!-- Mermaid graphical rendering -->
          <div class="mermaid-container" bind:this={mermaidContainer}>
            {#if !mermaidReady}
              <p class="hint">Initialising renderer…</p>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Edge Table (always shown) -->
      <div class="edge-table">
        <div class="edge-header">
          <span>Src</span><span>→</span><span>Tgt</span><span>Particle</span><span>Momentum</span>
        </div>
        {#each selectedDiagram.edges as [src, tgt, edge]}
          <div class="edge-row">
            <span>{src}</span>
            <span>→</span>
            <span>{tgt}</span>
            <span class="particle-name">{edge.particle.field.symbol}</span>
            <span class="momentum">{edge.momentum_label}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .diagram-viz {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--fg-secondary);
    font-style: italic;
    margin: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }
  .diagram-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }
  .tab-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .tab-btn.active {
    background: var(--bg-surface);
    border-color: var(--fg-accent);
    color: var(--fg-accent);
  }
  .view-toggle {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.2rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-mono);
  }
  .view-toggle:hover {
    border-color: var(--border-focus);
  }
  .summary-bar {
    font-size: 0.75rem;
    color: var(--fg-secondary);
    padding: 0.2rem 0;
  }
  .diagram-canvas {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow: auto;
    min-height: 120px;
  }
  .diagram-text {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.5rem;
    font-size: 0.72rem;
    line-height: 1.5;
    color: var(--fg-primary);
    overflow-x: auto;
    margin: 0;
    white-space: pre;
  }

  /* ── Mermaid container ── */
  .mermaid-container {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.75rem;
    flex: 1;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 180px;
  }
  .mermaid-container :global(svg) {
    max-width: 100%;
    height: auto;
  }
  .mermaid-container :global(.edgeLabel) {
    background-color: var(--bg-inset) !important;
    color: var(--hl-value) !important;
    font-family: var(--font-mono) !important;
    font-size: 0.72rem !important;
    padding: 0.1rem 0.3rem !important;
  }
  .mermaid-container :global(.edgePaths path) {
    stroke: var(--hl-symbol) !important;
    stroke-width: 1.5px !important;
  }
  .mermaid-container :global(.marker) {
    fill: var(--hl-symbol) !important;
    stroke: var(--hl-symbol) !important;
  }
  .mermaid-container :global(text) {
    fill: var(--fg-primary) !important;
  }

  .edge-table {
    font-size: 0.72rem;
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .edge-header,
  .edge-row {
    display: grid;
    grid-template-columns: 2rem 1.2rem 2rem 1fr 1fr;
    gap: 0.2rem;
    padding: 0.2rem 0.4rem;
    align-items: center;
  }
  .edge-header {
    background: var(--bg-surface);
    color: var(--fg-secondary);
    text-transform: uppercase;
    font-size: 0.65rem;
    letter-spacing: 0.05em;
  }
  .edge-row {
    border-bottom: 1px solid var(--border);
    color: var(--fg-primary);
  }
  .edge-row:last-child {
    border-bottom: none;
  }
  .particle-name {
    color: var(--hl-value);
  }
  .momentum {
    color: var(--hl-symbol);
  }
</style>
