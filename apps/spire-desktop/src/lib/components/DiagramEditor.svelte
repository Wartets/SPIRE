<!--
  SPIRE — Interactive Diagram Editor

  SVG-based Feynman diagram editor with:
  - Force-directed automatic layout (Coulomb repulsion + Hooke springs)
  - Draggable nodes with pointer event handling
  - Edge rendering: solid (fermion), wavy (boson), curly (gluon), dashed (scalar)
  - Reads from the generatedDiagrams physics store
  - Diagram selector for browsing generated topologies
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { generatedDiagrams } from "$lib/stores/physicsStore";
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

  // SVG viewport
  const SVG_W = 600;
  const SVG_H = 400;
  const NODE_RADIUS = 20;

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
    if ("Vertex" in kind) return "⊗";
    return "?";
  }

  function loadDiagram(d: FeynmanDiagram): void {
    stopSimulation();

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
      particleName: e.particle.field.name,
      edgeType: classifyEdge(e.particle.field.name),
      isExternal: e.is_external,
    }));

    startSimulation();
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
      if (node) node.pinned = false;
      draggingId = null;
    }
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
    const a = nodes.find((n) => n.id === edge.source);
    const b = nodes.find((n) => n.id === edge.target);
    if (!a || !b) return "";
    switch (edge.edgeType) {
      case "boson": return wavyPath(a.x, a.y, b.x, b.y);
      case "gluon": return curlyPath(a.x, a.y, b.x, b.y);
      default: return straightPath(a.x, a.y, b.x, b.y);
    }
  }

  // ---------------------------------------------------------------------------
  // Node Styling
  // ---------------------------------------------------------------------------

  function nodeColor(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return "#42a5f5"; // blue
    if ("ExternalOutgoing" in kind) return "#66bb6a"; // green
    return "#ff9800"; // orange (vertex)
  }

  function isVertex(kind: NodeKind): boolean {
    return "Vertex" in kind;
  }

  function edgeStrokeColor(edgeType: string): string {
    switch (edgeType) {
      case "boson": return "#42a5f5";
      case "gluon": return "#ef5350";
      case "scalar": return "#ab47bc";
      default: return "#e0e0e0";
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
      {#if simRunning}
        <span class="sim-badge">Simulating…</span>
      {/if}
    </div>

    <!-- SVG Canvas -->
    <svg
      class="diagram-svg"
      viewBox="0 0 {SVG_W} {SVG_H}"
      on:pointermove={onPointerMove}
      on:pointerup={onPointerUp}
      role="application"
      aria-label="Feynman diagram editor canvas"
    >
      <!-- Arrowhead markers -->
      <defs>
        <marker id="arrow-fermion" viewBox="0 0 10 10" refX="10" refY="5"
          markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#e0e0e0" />
        </marker>
        <marker id="arrow-boson" viewBox="0 0 10 10" refX="10" refY="5"
          markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" fill="#42a5f5" />
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
        {#if edge.particleName}
          {@const a = nodes.find(n => n.id === edge.source)}
          {@const b = nodes.find(n => n.id === edge.target)}
          {#if a && b}
            <text
              x={(a.x + b.x) / 2}
              y={(a.y + b.y) / 2 - 8}
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
            stroke-width="1.5"
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
              stroke-width="1.5"
              class="node-shape"
            />
            <text x={node.x} y={node.y + 4} class="node-label" fill={nodeColor(node.kind)}>
              {node.label}
            </text>
          </g>
        {/if}
      {/each}
    </svg>

    <!-- Legend -->
    <div class="legend">
      <span class="legend-item" style="color: #42a5f5;">〜 Boson</span>
      <span class="legend-item" style="color: #e0e0e0;">→ Fermion</span>
      <span class="legend-item" style="color: #ef5350;">⌇ Gluon</span>
      <span class="legend-item" style="color: #ab47bc;">┅ Scalar</span>
      <span class="sep">|</span>
      <span class="legend-item" style="color: #42a5f5;">■ Incoming</span>
      <span class="legend-item" style="color: #66bb6a;">■ Outgoing</span>
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

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    text-align: center;
    gap: 0.3rem;
  }
  .empty-state .hint { font-size: 0.78rem; color: #555; }

  /* --- Selector Bar --- */
  .selector-bar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding: 0.4rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    flex-shrink: 0;
  }
  .selector-label {
    color: #8ab4f8;
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
    color: #ccc;
    cursor: pointer;
    transition: background 0.12s;
    font-family: "JetBrains Mono", monospace;
  }
  .sel-btn:hover { background: rgba(138, 180, 248, 0.18); color: #fff; }
  .sel-btn.active {
    background: rgba(138, 180, 248, 0.25);
    color: #8ab4f8;
    border-color: #8ab4f8;
  }
  .relayout-btn {
    padding: 0.2rem 0.5rem;
    font-size: 0.72rem;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.05);
    color: #aaa;
    cursor: pointer;
    margin-left: auto;
  }
  .relayout-btn:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
  .sim-badge {
    font-size: 0.68rem;
    color: #ff9800;
    font-style: italic;
  }

  /* --- SVG Canvas --- */
  .diagram-svg {
    flex: 1;
    min-height: 200px;
    background: #111118;
    border-radius: 6px;
    cursor: default;
    user-select: none;
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
