<!--
  SPIRE - Interactive Diagram Editor

  SVG-based Feynman diagram editor with:
  - Force-directed automatic layout (Coulomb repulsion + Hooke springs)
  - Draggable nodes with pointer event handling
  - Edge rendering: solid (fermion), wavy (boson), curly (gluon), dashed (scalar)
  - Reads from the generatedDiagrams physics store
  - Diagram selector for browsing generated topologies
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { generatedDiagrams } from "$lib/stores/physicsStore";
  import { renderFeynmanDiagram } from "$lib/components/diagram/feynmanRenderer";
  import { generateTikZ, generateTikZDocument } from "$lib/components/diagram/tikzExport";
  import type {
    FeynmanDiagram,
    FeynmanNode,
    FeynmanEdge,
    TopologySet,
    NodeKind,
  } from "$lib/types/spire";

  // ---------------------------------------------------------------------------
  // Layout Types
  // ---------------------------------------------------------------------------

  interface LayoutNode {
    id: number;
    x: number;
    y: number;
    vx: number;
    vy: number;
    kind: NodeKind;
    label: string;
    pinned: boolean;
  }

  interface LayoutEdge {
    source: number;
    target: number;
    label: string;
    particleName: string;
    edgeType: "fermion" | "boson" | "gluon" | "scalar";
    isExternal: boolean;
  }

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let diagrams: FeynmanDiagram[] = [];
  let selectedIdx: number = 0;
  let nodes: LayoutNode[] = [];
  let edges: LayoutEdge[] = [];
  let selectedNodeId: number | null = null;
  let showGrid = true;
  let snapToGrid = false;
  let lockExternals = false;
  let showEdgeLabels = true;
  let showTikzExport = false;
  let tikzStandalone = false;
  let viewMode: "editable" | "rendered" | "text" = "editable";

  // SVG viewport
  const SVG_W = 600;
  const SVG_H = 400;
  const NODE_RADIUS = 20;
  const EXTERNAL_NODE_HALF_W = 28;
  const EXTERNAL_NODE_HALF_H = 14;
  const GRID_STEP = 16;

  // Force simulation
  let simRunning = false;
  let simFrameId = 0;
  const COULOMB_K = 5000;
  const SPRING_K = 0.04;
  const SPRING_REST = 100;
  const DAMPING = 0.92;
  const SIM_TICKS = 300;
  let simTick = 0;

  // Drag state
  let draggingId: number | null = null;
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  // ---------------------------------------------------------------------------
  // Store subscription
  // ---------------------------------------------------------------------------

  let unsubDiagrams: (() => void) | null = null;

  function onDiagramsChanged(ts: TopologySet | null): void {
    if (ts && ts.diagrams.length > 0) {
      diagrams = ts.diagrams;
      selectedIdx = 0;
      loadDiagram(diagrams[0]);
    } else {
      diagrams = [];
      nodes = [];
      edges = [];
    }
  }

  // ---------------------------------------------------------------------------
  // Diagram Loading
  // ---------------------------------------------------------------------------

  function classifyEdge(particleName: string): "fermion" | "boson" | "gluon" | "scalar" {
    const lower = particleName.toLowerCase();
    if (lower === "g" || lower === "gluon" || lower.startsWith("gl")) return "gluon";
    if (lower === "gamma" || lower === "z" || lower === "w+" || lower === "w-"
        || lower === "w" || lower === "photon" || lower.startsWith("z0")) return "boson";
    if (lower === "h" || lower === "h0" || lower === "higgs" || lower.startsWith("phi")
        || lower.startsWith("scalar")) return "scalar";
    return "fermion";
  }

  function nodeLabel(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return kind.ExternalIncoming.field.name;
    if ("ExternalOutgoing" in kind) return kind.ExternalOutgoing.field.name;
    if ("InternalVertex" in kind) return "⊗";
    return "?";
  }

  function loadDiagram(d: FeynmanDiagram): void {
    stopSimulation();
    selectedNodeId = null;

    // Build layout nodes with initial random positions.
    nodes = d.nodes.map((n) => ({
      id: n.id,
      x: n.position ? n.position[0] * 50 + SVG_W / 2 : 80 + Math.random() * (SVG_W - 160),
      y: n.position ? n.position[1] * 50 + SVG_H / 2 : 80 + Math.random() * (SVG_H - 160),
      vx: 0,
      vy: 0,
      kind: n.kind,
      label: nodeLabel(n.kind),
      pinned: false,
    }));

    // Build layout edges.
    edges = d.edges.map(([src, tgt, e]) => ({
      source: src,
      target: tgt,
      label: e.momentum_label,
      particleName: e.field.name,
      edgeType: classifyEdge(e.field.name),
      isExternal: e.is_external,
    }));

    centerGraph();

    startSimulation();
  }

  function centerGraph(): void {
    if (nodes.length === 0) return;

    let minX = Number.POSITIVE_INFINITY;
    let maxX = Number.NEGATIVE_INFINITY;
    let minY = Number.POSITIVE_INFINITY;
    let maxY = Number.NEGATIVE_INFINITY;

    for (const node of nodes) {
      const halfW = isVertex(node.kind) ? 8 : EXTERNAL_NODE_HALF_W;
      const halfH = isVertex(node.kind) ? 8 : EXTERNAL_NODE_HALF_H;
      minX = Math.min(minX, node.x - halfW);
      maxX = Math.max(maxX, node.x + halfW);
      minY = Math.min(minY, node.y - halfH);
      maxY = Math.max(maxY, node.y + halfH);
    }

    const dx = SVG_W / 2 - (minX + maxX) / 2;
    const dy = SVG_H / 2 - (minY + maxY) / 2;

    nodes = nodes.map((node) => ({
      ...node,
      x: Math.max(NODE_RADIUS, Math.min(SVG_W - NODE_RADIUS, node.x + dx)),
      y: Math.max(NODE_RADIUS, Math.min(SVG_H - NODE_RADIUS, node.y + dy)),
    }));
  }

  function nudgeSimulation(): void {
    for (const node of nodes) {
      if (node.pinned) continue;
      node.vx += (Math.random() - 0.5) * 1.5;
      node.vy += (Math.random() - 0.5) * 1.5;
    }
    nodes = nodes;
    if (!simRunning) startSimulation();
  }

  function togglePinSelection(): void {
    if (selectedNodeId === null) return;
    const node = nodes.find((n) => n.id === selectedNodeId);
    if (!node) return;
    node.pinned = !node.pinned;
    nodes = nodes;
  }

  function pinExternals(shouldPin: boolean): void {
    for (const node of nodes) {
      if (!isVertex(node.kind)) node.pinned = shouldPin;
    }
    nodes = nodes;
  }

  function applyGridSnap(node: LayoutNode): void {
    if (!snapToGrid) return;
    node.x = Math.round(node.x / GRID_STEP) * GRID_STEP;
    node.y = Math.round(node.y / GRID_STEP) * GRID_STEP;
  }

  // ---------------------------------------------------------------------------
  // Force-Directed Simulation
  // ---------------------------------------------------------------------------

  function startSimulation(): void {
    simTick = 0;
    simRunning = true;
    simFrameId = requestAnimationFrame(tickSimulation);
  }

  function stopSimulation(): void {
    simRunning = false;
    cancelAnimationFrame(simFrameId);
  }

  function tickSimulation(): void {
    if (!simRunning || simTick >= SIM_TICKS) {
      simRunning = false;
      return;
    }

    // Coulomb repulsion between all pairs
    for (let i = 0; i < nodes.length; i++) {
      if (nodes[i].pinned) continue;
      for (let j = i + 1; j < nodes.length; j++) {
        let dx = nodes[i].x - nodes[j].x;
        let dy = nodes[i].y - nodes[j].y;
        let dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 1) dist = 1;
        const force = COULOMB_K / (dist * dist);
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        nodes[i].vx += fx;
        nodes[i].vy += fy;
        if (!nodes[j].pinned) {
          nodes[j].vx -= fx;
          nodes[j].vy -= fy;
        }
      }
    }

    // Hooke spring attraction along edges
    for (const edge of edges) {
      const a = nodes.find((n) => n.id === edge.source);
      const b = nodes.find((n) => n.id === edge.target);
      if (!a || !b) continue;
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      let dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 1) dist = 1;
      const displacement = dist - SPRING_REST;
      const fx = (dx / dist) * SPRING_K * displacement;
      const fy = (dy / dist) * SPRING_K * displacement;
      if (!a.pinned) { a.vx += fx; a.vy += fy; }
      if (!b.pinned) { b.vx -= fx; b.vy -= fy; }
    }

    // Apply damping and update positions
    for (const node of nodes) {
      if (node.pinned) continue;
      node.vx *= DAMPING;
      node.vy *= DAMPING;
      node.x += node.vx;
      node.y += node.vy;
      // Keep within bounds
      node.x = Math.max(NODE_RADIUS, Math.min(SVG_W - NODE_RADIUS, node.x));
      node.y = Math.max(NODE_RADIUS, Math.min(SVG_H - NODE_RADIUS, node.y));
    }

    simTick++;
    centerGraph();
    nodes = nodes; // trigger Svelte reactivity

    simFrameId = requestAnimationFrame(tickSimulation);
  }

  // ---------------------------------------------------------------------------
  // Diagram Selection
  // ---------------------------------------------------------------------------

  function selectDiagram(idx: number): void {
    if (idx < 0 || idx >= diagrams.length) return;
    selectedIdx = idx;
    loadDiagram(diagrams[idx]);
  }

  // ---------------------------------------------------------------------------
  // Drag Interaction
  // ---------------------------------------------------------------------------

  function onPointerDown(e: PointerEvent, nodeId: number): void {
    e.preventDefault();
    const node = nodes.find((n) => n.id === nodeId);
    if (!node) return;
    selectedNodeId = nodeId;
    if (lockExternals && !isVertex(node.kind)) {
      node.pinned = true;
      nodes = nodes;
      return;
    }
    draggingId = nodeId;
    node.pinned = true;
    const svg = (e.target as SVGElement).closest("svg");
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    dragOffsetX = e.clientX - rect.left - node.x;
    dragOffsetY = e.clientY - rect.top - node.y;
    (e.target as SVGElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent): void {
    if (draggingId === null) return;
    const node = nodes.find((n) => n.id === draggingId);
    if (!node) return;
    const svg = (e.target as SVGElement).closest("svg");
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    node.x = Math.max(NODE_RADIUS, Math.min(SVG_W - NODE_RADIUS, e.clientX - rect.left - dragOffsetX));
    node.y = Math.max(NODE_RADIUS, Math.min(SVG_H - NODE_RADIUS, e.clientY - rect.top - dragOffsetY));
    nodes = nodes; // reactivity
  }

  function onPointerUp(e: PointerEvent): void {
    if (draggingId !== null) {
      const node = nodes.find((n) => n.id === draggingId);
      if (node) {
        applyGridSnap(node);
        if (!(lockExternals && !isVertex(node.kind))) {
          node.pinned = false;
        }
      }
      draggingId = null;
      nodes = nodes;
    }
  }

  function nodeAnchorPoint(
    node: LayoutNode,
    towardX: number,
    towardY: number,
  ): { x: number; y: number } {
    const dx = towardX - node.x;
    const dy = towardY - node.y;
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;

    if (isVertex(node.kind)) {
      const r = 8;
      return { x: node.x + ux * r, y: node.y + uy * r };
    }

    const tx = Math.abs(ux) > 1e-6 ? EXTERNAL_NODE_HALF_W / Math.abs(ux) : Number.POSITIVE_INFINITY;
    const ty = Math.abs(uy) > 1e-6 ? EXTERNAL_NODE_HALF_H / Math.abs(uy) : Number.POSITIVE_INFINITY;
    const t = Math.min(tx, ty);
    return { x: node.x + ux * t, y: node.y + uy * t };
  }

  function edgeEndpoints(edge: LayoutEdge): { x1: number; y1: number; x2: number; y2: number } {
    const a = nodes.find((n) => n.id === edge.source);
    const b = nodes.find((n) => n.id === edge.target);
    if (!a || !b) return { x1: 0, y1: 0, x2: 0, y2: 0 };

    const start = nodeAnchorPoint(a, b.x, b.y);
    const end = nodeAnchorPoint(b, a.x, a.y);
    return { x1: start.x, y1: start.y, x2: end.x, y2: end.y };
  }

  // ---------------------------------------------------------------------------
  // SVG Edge Path Generators
  // ---------------------------------------------------------------------------

  function straightPath(x1: number, y1: number, x2: number, y2: number): string {
    return `M${x1},${y1} L${x2},${y2}`;
  }

  function wavyPath(x1: number, y1: number, x2: number, y2: number): string {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 1) return straightPath(x1, y1, x2, y2);
    const waves = Math.max(3, Math.round(len / 20));
    const nx = -dy / len;
    const ny = dx / len;
    const amp = 6;
    let d = `M${x1},${y1}`;
    for (let i = 1; i <= waves; i++) {
      const t = i / waves;
      const mx = x1 + dx * (t - 0.5 / waves);
      const my = y1 + dy * (t - 0.5 / waves);
      const sign = i % 2 === 0 ? 1 : -1;
      const cx = mx + nx * amp * sign;
      const cy = my + ny * amp * sign;
      const ex = x1 + dx * t;
      const ey = y1 + dy * t;
      d += ` Q${cx},${cy} ${ex},${ey}`;
    }
    return d;
  }

  function curlyPath(x1: number, y1: number, x2: number, y2: number): string {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const len = Math.sqrt(dx * dx + dy * dy);
    if (len < 1) return straightPath(x1, y1, x2, y2);
    const loops = Math.max(3, Math.round(len / 15));
    const nx = -dy / len;
    const ny = dx / len;
    const amp = 8;
    let d = `M${x1},${y1}`;
    for (let i = 0; i < loops; i++) {
      const t0 = i / loops;
      const t1 = (i + 0.5) / loops;
      const t2 = (i + 1) / loops;
      const sign = i % 2 === 0 ? 1 : -1;
      const cp1x = x1 + dx * t0 + nx * amp * sign;
      const cp1y = y1 + dy * t0 + ny * amp * sign;
      const cp2x = x1 + dx * t1 + nx * amp * sign;
      const cp2y = y1 + dy * t1 + ny * amp * sign;
      const ex = x1 + dx * t2;
      const ey = y1 + dy * t2;
      d += ` C${cp1x},${cp1y} ${cp2x},${cp2y} ${ex},${ey}`;
    }
    return d;
  }

  function edgePath(edge: LayoutEdge): string {
    const { x1, y1, x2, y2 } = edgeEndpoints(edge);
    if (!(x1 || y1 || x2 || y2)) return "";
    switch (edge.edgeType) {
      case "boson": return wavyPath(x1, y1, x2, y2);
      case "gluon": return curlyPath(x1, y1, x2, y2);
      default: return straightPath(x1, y1, x2, y2);
    }
  }

  function edgeMidpoint(edge: LayoutEdge): { x: number; y: number } {
    const { x1, y1, x2, y2 } = edgeEndpoints(edge);
    return { x: (x1 + x2) / 2, y: (y1 + y2) / 2 };
  }

  // ---------------------------------------------------------------------------
  // Node Styling
  // ---------------------------------------------------------------------------

  function nodeColor(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return "var(--color-accent)"; // blue
    if ("ExternalOutgoing" in kind) return "var(--color-success)"; // green
    return "#ff9800"; // orange (vertex)
  }

  function isVertex(kind: NodeKind): boolean {
    return "InternalVertex" in kind;
  }

  function selectedNode(): LayoutNode | null {
    if (selectedNodeId === null) return null;
    return nodes.find((n) => n.id === selectedNodeId) ?? null;
  }

  function edgeStrokeColor(edgeType: string): string {
    switch (edgeType) {
      case "boson": return "var(--color-accent)";
      case "gluon": return "var(--color-error)";
      case "scalar": return "#ab47bc";
      default: return "var(--color-text-primary)";
    }
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  onMount(() => {
    unsubDiagrams = generatedDiagrams.subscribe(onDiagramsChanged);
  });

  onDestroy(() => {
    stopSimulation();
    if (unsubDiagrams) unsubDiagrams();
  });

  $: selectedDiagram = diagrams[selectedIdx] ?? null;
  $: renderedSvg = selectedDiagram
    ? renderFeynmanDiagram(selectedDiagram, {
        width: 540,
        height: 340,
      })
    : "";

  $: diagramText = selectedDiagram
    ? [
        `Diagram #${selectedDiagram.id}`,
        `Loop: ${typeof selectedDiagram.loop_order === "string" ? selectedDiagram.loop_order : JSON.stringify(selectedDiagram.loop_order)}`,
        `Channels: ${selectedDiagram.channels.join(", ") || "-"}`,
        `Nodes: ${selectedDiagram.nodes.length}`,
        `Edges: ${selectedDiagram.edges.length}`,
      ].join("\n")
    : "";

  $: tikzCode = selectedDiagram
    ? (tikzStandalone ? generateTikZDocument(selectedDiagram) : generateTikZ(selectedDiagram))
    : "";

  function copyTikZ(): void {
    if (!tikzCode) return;
    navigator.clipboard.writeText(tikzCode).catch(() => {});
  }

  function exportRenderedSvg(): void {
    if (!renderedSvg || !selectedDiagram) return;
    const blob = new Blob([renderedSvg], { type: "image/svg+xml" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `diagram-editor-${selectedDiagram.id}.svg`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<!-- ======================================================================= -->
<!-- Template                                                                 -->
<!-- ======================================================================= -->

<div class="diagram-editor">
  {#if diagrams.length === 0}
    <div class="empty-state">
      <p>No diagrams available.</p>
      <p class="hint">Generate Feynman diagrams in the <strong>Diagram Visualizer</strong> widget first.</p>
    </div>
  {:else}
    <!-- Diagram Selector -->
    <div class="selector-bar">
      <span class="mode-chip">Unified editor mode</span>
      <span class="selector-label">Diagram:</span>
      <div class="selector-buttons">
        {#each diagrams as d, idx}
          <button
            class="sel-btn"
            class:active={idx === selectedIdx}
            on:click={() => selectDiagram(idx)}
          >
            #{d.id}
          </button>
        {/each}
      </div>
      <button class="relayout-btn" on:click={() => loadDiagram(diagrams[selectedIdx])}>
        ⟳ Relayout
      </button>
      <button class="relayout-btn" on:click={centerGraph} use:tooltip={{ text: "Center graph in viewport" }}>
        ◎ Center
      </button>
      <button class="relayout-btn" on:click={nudgeSimulation} use:tooltip={{ text: "Perturb layout to escape local minima" }}>
        ✦ Shake
      </button>
      {#if simRunning}
        <span class="sim-badge">Simulating…</span>
      {/if}
    </div>

    <div class="mode-tabs">
      <button class="mode-btn" class:active={viewMode === "editable"} on:click={() => (viewMode = "editable")}>Edit</button>
      <button class="mode-btn" class:active={viewMode === "rendered"} on:click={() => (viewMode = "rendered")}>Rendered</button>
      <button class="mode-btn" class:active={viewMode === "text"} on:click={() => (viewMode = "text")}>Text</button>
      <div class="mode-spacer"></div>
      {#if viewMode === "rendered"}
        <button class="mode-btn" on:click={() => (showTikzExport = !showTikzExport)}>TikZ</button>
        <button class="mode-btn" on:click={exportRenderedSvg}>SVG ↓</button>
      {/if}
    </div>

    <div class="summary-bar">
      <span>{diagrams.length} diagram{diagrams.length !== 1 ? "s" : ""}</span>
      {#if selectedDiagram}
        <span class="sep">·</span>
        <span>nodes: {selectedDiagram.nodes.length}</span>
        <span class="sep">·</span>
        <span>edges: {selectedDiagram.edges.length}</span>
        <span class="sep">·</span>
        <span>S = {selectedDiagram.symmetry_factor}</span>
        <span class="sep">·</span>
        <span>channels: {selectedDiagram.channels.join("+") || "—"}</span>
      {/if}
    </div>

    {#if showTikzExport && selectedDiagram}
      <div class="tikz-overlay">
        <div class="tikz-header">
          <span class="tikz-title">LaTeX TikZ Export</span>
          <label class="tikz-toggle">
            <input type="checkbox" bind:checked={tikzStandalone} />
            Standalone
          </label>
          <button class="copy-btn" on:click={copyTikZ}>Copy</button>
          <button class="close-btn" on:click={() => (showTikzExport = false)}>✕</button>
        </div>
        <pre class="tikz-code">{tikzCode}</pre>
      </div>
    {/if}

    <p class="editor-hint">Diagram Editor and Diagram Visualizer now share summary/export affordances; use Edit mode for manual node work, Rendered mode for publication view.</p>

    {#if viewMode === "editable"}
      <div class="edit-toolbar">
        <button class="edit-btn" on:click={() => (showGrid = !showGrid)} class:active={showGrid}>Grid</button>
        <button class="edit-btn" on:click={() => (snapToGrid = !snapToGrid)} class:active={snapToGrid}>Snap</button>
        <button class="edit-btn" on:click={() => (lockExternals = !lockExternals)} class:active={lockExternals}>Lock ext.</button>
        <button class="edit-btn" on:click={() => (showEdgeLabels = !showEdgeLabels)} class:active={showEdgeLabels}>Labels</button>
        <button class="edit-btn" on:click={togglePinSelection} disabled={selectedNodeId === null}>
          {selectedNode()?.pinned ? "Unpin selected" : "Pin selected"}
        </button>
        <button class="edit-btn" on:click={() => pinExternals(true)}>Pin externals</button>
        <button class="edit-btn" on:click={() => pinExternals(false)}>Unpin externals</button>
      </div>
    {/if}

    {#if viewMode === "editable"}
      <!-- SVG Canvas -->
      <svg
        class="diagram-svg"
        viewBox="0 0 {SVG_W} {SVG_H}"
        on:pointermove={onPointerMove}
        on:pointerup={onPointerUp}
        role="application"
        aria-label="Feynman diagram editor canvas"
      >
      {#if showGrid}
        <g class="grid-layer" aria-hidden="true">
          {#each Array(Math.floor(SVG_W / GRID_STEP) + 1) as _, i}
            <line x1={i * GRID_STEP} y1="0" x2={i * GRID_STEP} y2={SVG_H} class="grid-line" />
          {/each}
          {#each Array(Math.floor(SVG_H / GRID_STEP) + 1) as _, j}
            <line x1="0" y1={j * GRID_STEP} x2={SVG_W} y2={j * GRID_STEP} class="grid-line" />
          {/each}
        </g>
      {/if}
      <!-- Arrowhead markers -->
      <defs>
        <marker id="arrow-fermion" viewBox="0 0 10 10" refX="10" refY="5"
          markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--color-text-primary)" />
        </marker>
        <marker id="arrow-boson" viewBox="0 0 10 10" refX="10" refY="5"
          markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--color-accent)" />
        </marker>
      </defs>

      <!-- Edges -->
      {#each edges as edge}
        <path
          d={edgePath(edge)}
          fill="none"
          stroke={edgeStrokeColor(edge.edgeType)}
          stroke-width={edge.edgeType === "gluon" ? 2.5 : 1.8}
          stroke-dasharray={edge.edgeType === "scalar" ? "6,4" : "none"}
          marker-end={edge.edgeType === "fermion" ? "url(#arrow-fermion)" : ""}
          class="edge-path"
        />
        <!-- Edge label (particle name) -->
        {#if showEdgeLabels && edge.particleName}
          {@const m = edgeMidpoint(edge)}
          {#if m}
            <text
              x={m.x}
              y={m.y - 8}
              class="edge-label"
              fill={edgeStrokeColor(edge.edgeType)}
            >{edge.particleName}</text>
          {/if}
        {/if}
      {/each}

      <!-- Nodes -->
      {#each nodes as node}
        {#if isVertex(node.kind)}
          <!-- Vertex: small filled circle -->
          <circle
            cx={node.x}
            cy={node.y}
            r={8}
            fill={nodeColor(node.kind)}
            stroke="#222"
            stroke-width={selectedNodeId === node.id ? "2.5" : "1.5"}
            class="node-shape"
            on:pointerdown={(e) => onPointerDown(e, node.id)}
            role="button"
            tabindex="-1"
            aria-label="Vertex {node.id}"
          />
        {:else}
          <!-- External particle: rounded rect with label -->
          <g on:pointerdown={(e) => onPointerDown(e, node.id)} class="node-group" role="button" tabindex="-1" aria-label="Particle {node.label}">
            <rect
              x={node.x - 28}
              y={node.y - 14}
              width={56}
              height={28}
              rx={6}
              ry={6}
              fill="rgba(30, 30, 40, 0.85)"
              stroke={nodeColor(node.kind)}
              stroke-width={selectedNodeId === node.id ? "2.5" : "1.5"}
              class="node-shape"
            />
            <text x={node.x} y={node.y + 4} class="node-label" fill={nodeColor(node.kind)}>
              {node.label}
            </text>
          </g>
        {/if}
      {/each}
      </svg>
    {:else if viewMode === "rendered"}
      <div class="rendered-view" aria-label="Rendered diagram preview">
        <div class="rendered-svg">{@html renderedSvg}</div>
      </div>
    {:else}
      <pre class="diagram-text">{diagramText}</pre>
    {/if}

    <!-- Legend -->
    <div class="legend">
      <span class="legend-item" style="color: var(--color-accent);">〜 Boson</span>
      <span class="legend-item" style="color: var(--color-text-primary);">→ Fermion</span>
      <span class="legend-item" style="color: var(--color-error);">⌇ Gluon</span>
      <span class="legend-item" style="color: #ab47bc;">┅ Scalar</span>
      <span class="sep">|</span>
      <span class="legend-item" style="color: var(--color-accent);">■ Incoming</span>
      <span class="legend-item" style="color: var(--color-success);">■ Outgoing</span>
      <span class="legend-item" style="color: #ff9800;">● Vertex</span>
    </div>
  {/if}
</div>

<!-- ======================================================================= -->
<!-- Styles                                                                   -->
<!-- ======================================================================= -->

<style>
  .diagram-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow: hidden;
    font-size: 0.85rem;
  }

  .mode-tabs {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .mode-btn {
    padding: 0.15rem 0.45rem;
    font-size: 0.68rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .mode-btn.active,
  .mode-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  .mode-spacer {
    flex: 1;
  }

  .summary-bar {
    font-size: 0.7rem;
    color: var(--fg-secondary);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.2rem;
    padding: 0.1rem 0;
  }

  .summary-bar .sep {
    opacity: 0.45;
  }

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

  .copy-btn,
  .close-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.1rem 0.35rem;
    font-size: 0.65rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .edit-toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    align-items: center;
  }

  .edit-btn {
    padding: 0.15rem 0.45rem;
    font-size: 0.66rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    cursor: pointer;
    font-family: var(--font-mono);
  }

  .edit-btn.active,
  .edit-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  .edit-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-muted);
    text-align: center;
    gap: 0.3rem;
  }
  .empty-state .hint { font-size: 0.78rem; color: var(--color-text-muted); }

  /* --- Selector Bar --- */
  .selector-bar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding: 0.4rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    flex-shrink: 0;
  }
  .mode-chip {
    padding: 0.1rem 0.3rem;
    font-size: 0.62rem;
    color: var(--fg-accent);
    border: 1px solid var(--border);
    background: var(--bg-inset);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .selector-label {
    color: var(--color-accent);
    font-weight: 600;
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .selector-buttons {
    display: flex;
    gap: 0.2rem;
    flex-wrap: wrap;
  }
  .sel-btn {
    padding: 0.15rem 0.45rem;
    font-size: 0.72rem;
    border: 1px solid rgba(138, 180, 248, 0.25);
    border-radius: 4px;
    background: rgba(138, 180, 248, 0.06);
    color: var(--color-text-primary);
    cursor: pointer;
    transition: background 0.12s;
    font-family: "JetBrains Mono", monospace;
  }
  .sel-btn:hover { background: rgba(138, 180, 248, 0.18); color: var(--color-text-primary); }
  .sel-btn.active {
    background: rgba(138, 180, 248, 0.25);
    color: var(--color-accent);
    border-color: var(--color-accent);
  }
  .relayout-btn {
    padding: 0.2rem 0.5rem;
    font-size: 0.72rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.05);
    color: var(--color-text-muted);
    cursor: pointer;
    margin-left: auto;
  }
  .relayout-btn:hover { background: rgba(255, 255, 255, 0.12); color: var(--color-text-primary); }
  .sim-badge {
    font-size: 0.68rem;
    color: var(--hl-value);
    font-style: italic;
  }

  .editor-hint {
    margin: -0.2rem 0 0;
    font-size: 0.66rem;
    color: var(--fg-secondary);
    font-style: italic;
  }

  /* --- SVG Canvas --- */
  .diagram-svg {
    flex: 1;
    min-height: 200px;
    background: color-mix(in srgb, var(--color-bg-base) 75%, transparent);
    border: 1px solid var(--border);
    cursor: default;
    user-select: none;
  }

  .grid-line {
    stroke: rgba(255, 255, 255, 0.045);
    stroke-width: 1;
    pointer-events: none;
  }

  .edge-path {
    pointer-events: none;
  }
  .edge-label {
    font-size: 10px;
    font-family: "JetBrains Mono", monospace;
    text-anchor: middle;
    pointer-events: none;
  }

  .node-shape {
    cursor: grab;
  }
  .node-shape:active { cursor: grabbing; }
  .node-group { cursor: grab; }
  .node-group:active { cursor: grabbing; }

  .rendered-view {
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--color-bg-base) 75%, transparent);
    display: flex;
    flex-direction: column;
    overflow: auto;
    padding: 0.4rem;
    gap: 0.35rem;
  }

  .rendered-svg {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 220px;
  }

  .rendered-svg :global(svg) {
    max-width: 100%;
    height: auto;
  }

  .diagram-text {
    flex: 1;
    min-height: 220px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--color-bg-base) 75%, transparent);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    line-height: 1.45;
    padding: 0.45rem;
    margin: 0;
    overflow: auto;
  }

  .node-label {
    font-size: 11px;
    font-family: "JetBrains Mono", monospace;
    text-anchor: middle;
    dominant-baseline: middle;
    pointer-events: none;
  }

  /* --- Legend --- */
  .legend {
    display: flex;
    gap: 0.6rem;
    justify-content: center;
    padding: 0.25rem 0;
    font-size: 0.7rem;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .legend-item {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    opacity: 0.8;
  }
  .sep { color: #444; }
</style>
