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

  /** Track last rendered diagram ID + viewMode to avoid unnecessary re-renders. */
  let lastRenderedKey = "";

  // ── Canvas Pan/Zoom State ──────────────────────────────────────────────
  let svgElement: SVGElement | null = null;
  let panX = 0;
  let panY = 0;
  let isPanning = false;
  let panStartX = 0;
  let panStartY = 0;
  let manualMoveEnabled = false;
  let draggedNodeId: number | null = null;
  let dragNodeOffsetX = 0;
  let dragNodeOffsetY = 0;
  let nodePositionOverrides: Record<number, { x: number; y: number }> = {};
  let detachCanvasHandlers: (() => void) | null = null;
  let autoReviewEnabled = true;
  const autoReviewedKeys = new Set<string>();
  let reviewRunCount = 0;
  const positionCache = new Map<number, Record<number, { x: number; y: number }>>();
  const renderCache = new Map<string, string>();
  const MAX_RENDER_CACHE_ENTRIES = 24;
  let pendingDragUpdate: { id: number; x: number; y: number } | null = null;
  let dragRafId: number | null = null;

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
    ? `${selectedDiagram.id}-${viewMode}`
    : "";

  $: if (renderKey && renderKey !== lastRenderedKey) {
    // Schedule render after DOM update
    scheduleRender();
  }

  // Ensure initial render on mount if a diagram is already selected
  onMount(() => {
    if (selectedDiagram && !lastRenderedKey) {
      scheduleRender();
    }

    return () => {
      if (detachCanvasHandlers) {
        detachCanvasHandlers();
        detachCanvasHandlers = null;
      }
    };
  });

  function bindCanvasInteraction(container: HTMLDivElement | null): void {
    if (detachCanvasHandlers) {
      detachCanvasHandlers();
      detachCanvasHandlers = null;
    }
    if (!container) return;

    const onDown = (e: MouseEvent) => handleCanvasMouseDown(e);
    const onMove = (e: MouseEvent) => handleCanvasMouseMove(e);
    const onUp = (e: MouseEvent) => handleCanvasMouseUp(e);
    const onLeave = (e: MouseEvent) => handleCanvasMouseLeave(e);
    const onWheel = (e: WheelEvent) => handleCanvasWheel(e);

    container.addEventListener("mousedown", onDown);
    container.addEventListener("mousemove", onMove);
    container.addEventListener("mouseup", onUp);
    container.addEventListener("mouseleave", onLeave);
    container.addEventListener("wheel", onWheel, { passive: false });

    detachCanvasHandlers = () => {
      container.removeEventListener("mousedown", onDown);
      container.removeEventListener("mousemove", onMove);
      container.removeEventListener("mouseup", onUp);
      container.removeEventListener("mouseleave", onLeave);
      container.removeEventListener("wheel", onWheel);
    };
  }

  function getNodePosition(nodeId: number): { x: number; y: number } | null {
    const override = nodePositionOverrides[nodeId];
    if (override) return override;
    const nodeEl = renderContainer?.querySelector(`circle.node-anchor[data-node-id=\"${nodeId}\"]`) as SVGCircleElement | null;
    if (!nodeEl) return null;
    const x = Number(nodeEl.getAttribute("cx") ?? "0");
    const y = Number(nodeEl.getAttribute("cy") ?? "0");
    return Number.isFinite(x) && Number.isFinite(y) ? { x, y } : null;
  }

  function applySvgTransform(): void {
    if (!svgElement) return;
    svgElement.style.transform = `translate(${panX}px, ${panY}px)`;
    svgElement.style.transformOrigin = "center center";
  }

  function overrideSignature(): string {
    const entries = Object.entries(nodePositionOverrides);
    if (entries.length === 0) return "no-pos";
    entries.sort(([a], [b]) => Number(a) - Number(b));
    let out = "";
    for (const [id, p] of entries) {
      out += `${id}:${p.x.toFixed(1)},${p.y.toFixed(1)};`;
    }
    return out;
  }

  function renderCacheKey(mode: ViewMode, diagId: number): string {
    return `${mode}:${diagId}:${overrideSignature()}`;
  }

  function canReuseRenderCache(): boolean {
    return Object.keys(nodePositionOverrides).length === 0;
  }

  function getCachedRender(key: string): string | undefined {
    const cached = renderCache.get(key);
    if (cached !== undefined) {
      renderCache.delete(key);
      renderCache.set(key, cached);
    }
    return cached;
  }

  function setCachedRender(key: string, value: string): void {
    renderCache.set(key, value);
    while (renderCache.size > MAX_RENDER_CACHE_ENTRIES) {
      const oldestKey = renderCache.keys().next().value;
      if (!oldestKey) break;
      renderCache.delete(oldestKey);
    }
  }

  function shouldAutoReview(diag: FeynmanDiagram): boolean {
    return diag.loop_order === "Tree" && diag.nodes.length <= 24 && diag.edges.length <= 32;
  }

  function savePositionCache(diagId: number): void {
    positionCache.set(diagId, { ...nodePositionOverrides });
  }

  function restorePositionCache(diagId: number): void {
    nodePositionOverrides = positionCache.get(diagId) ? { ...positionCache.get(diagId)! } : {};
  }

  function clampNodePosition(x: number, y: number): { x: number; y: number } {
    return {
      x: Math.max(36, Math.min(564, x)),
      y: Math.max(24, Math.min(396, y)),
    };
  }

  function formatParticleLabel(raw: string): string {
    let s = (raw ?? "").trim();
    if (!s) return s;
    if (s.startsWith("$") && s.endsWith("$")) s = s.slice(1, -1);
    s = s.replace(/^\\/, "");

    const greek: Record<string, string> = {
      mu: "μ",
      nu: "ν",
      tau: "τ",
      gamma: "γ",
      alpha: "α",
      beta: "β",
      rho: "ρ",
      phi: "ϕ",
      pi: "π",
      ell: "ℓ",
    };
    if (greek[s.toLowerCase()]) return greek[s.toLowerCase()];

    s = s
      .replace(/\\mu/gi, "μ")
      .replace(/\\nu/gi, "ν")
      .replace(/\\tau/gi, "τ")
      .replace(/\\gamma/gi, "γ")
      .replace(/\\ell/gi, "ℓ")
      .replace(/\^\{-\}/g, "⁻")
      .replace(/\^\{\+\}/g, "⁺")
      .replace(/\^-/g, "⁻")
      .replace(/\^\+/g, "⁺")
      .replace(/([A-Za-zα-ωΑ-Ω])-(?!\w)/g, "$1⁻")
      .replace(/([A-Za-zα-ωΑ-Ω])\+(?!\w)/g, "$1⁺");

    if (s === "e-") return "e⁻";
    if (s === "e+") return "e⁺";
    if (s === "mu-") return "μ⁻";
    if (s === "mu+") return "μ⁺";
    return s;
  }

  function computeLayerTargets(diag: FeynmanDiagram): Map<number, number> {
    const targets = new Map<number, number>();
    const outgoing = new Map<number, number[]>();
    for (const n of diag.nodes) outgoing.set(n.id, []);
    for (const [s, t] of diag.edges) outgoing.get(s)?.push(t);

    const queue: number[] = [];
    const queued = new Set<number>();
    for (const n of diag.nodes) {
      if ("ExternalIncoming" in n.kind) {
        targets.set(n.id, 0);
        queue.push(n.id);
        queued.add(n.id);
      }
    }

    let qIndex = 0;
    let safetySteps = 0;
    const maxSafetySteps = Math.max(64, diag.nodes.length * Math.max(1, diag.edges.length) * 6);

    while (qIndex < queue.length && safetySteps < maxSafetySteps) {
      const id = queue[qIndex++]!;
      queued.delete(id);
      safetySteps += 1;
      const l = targets.get(id) ?? 0;
      for (const t of outgoing.get(id) ?? []) {
        const cand = l + 1;
        const prev = targets.get(t);
        // Use shortest-path relaxation so cyclic graphs converge.
        if (prev === undefined || cand < prev) {
          targets.set(t, cand);
          if (!queued.has(t)) {
            queue.push(t);
            queued.add(t);
          }
        }
      }
    }

    let maxLayer = 0;
    for (const v of targets.values()) maxLayer = Math.max(maxLayer, v);
    if (maxLayer <= 0) maxLayer = 1;

    for (const n of diag.nodes) {
      if ("ExternalIncoming" in n.kind) {
        targets.set(n.id, 0);
      } else if ("ExternalOutgoing" in n.kind) {
        targets.set(n.id, 1);
      } else {
        const l = targets.get(n.id);
        targets.set(n.id, l === undefined ? 0.5 : l / maxLayer);
      }
    }
    return targets;
  }

  function applySmartGrid(positions: Map<number, { x: number; y: number }>, diag: FeynmanDiagram): Map<number, { x: number; y: number }> {
    const left = 74;
    const right = 526;
    const top = 36;
    const bottom = 388;
    const gx = 52;
    const gy = 44;

    const grid: Array<{ x: number; y: number; used: boolean; idx: number }> = [];
    let idx = 0;
    for (let y = top; y <= bottom; y += gy) {
      for (let x = left; x <= right; x += gx) {
        grid.push({ x, y, used: false, idx: idx++ });
      }
    }

    const layerTargets = computeLayerTargets(diag);
    const nodesSorted = [...diag.nodes].sort((a, b) => {
      const ar = "ExternalIncoming" in a.kind ? 0 : "InternalVertex" in a.kind ? 1 : 2;
      const br = "ExternalIncoming" in b.kind ? 0 : "InternalVertex" in b.kind ? 1 : 2;
      if (ar !== br) return ar - br;
      return (layerTargets.get(a.id) ?? 0.5) - (layerTargets.get(b.id) ?? 0.5);
    });

    const placed = new Map<number, { x: number; y: number }>();
    for (const node of nodesSorted) {
      const p = positions.get(node.id) ?? { x: 300, y: 210 };
      let best: { i: number; score: number } | null = null;
      for (let i = 0; i < grid.length; i++) {
        if (grid[i].used) continue;
        const dx = grid[i].x - p.x;
        const dy = grid[i].y - p.y;
        let score = dx * dx + dy * dy;

        const targetLayer = layerTargets.get(node.id) ?? 0.5;
        const desiredX = left + targetLayer * (right - left);
        score += Math.abs(grid[i].x - desiredX) * 9;

        if ("ExternalIncoming" in node.kind) score += Math.max(0, grid[i].x - 180) * 2.2;
        if ("ExternalOutgoing" in node.kind) score += Math.max(0, 420 - grid[i].x) * 2.2;

        if (!best || score < best.score) best = { i, score };
      }

      if (best) {
        grid[best.i].used = true;
        placed.set(node.id, { x: grid[best.i].x, y: grid[best.i].y });
      } else {
        placed.set(node.id, clampNodePosition(p.x, p.y));
      }
    }
    return placed;
  }

  function reviewNodeLayout(passes = 1): void {
    if (!selectedDiagram || (viewMode !== "feynman" && viewMode !== "worldsheet")) return;

    const ids = selectedDiagram.nodes.map((n) => n.id);
    const edgeList = selectedDiagram.edges;
    const nodeCount = ids.length;
    const edgeCount = edgeList.length;
    const positions = new Map<number, { x: number; y: number }>();
    for (const id of ids) {
      const p = getNodePosition(id);
      if (p) positions.set(id, { ...p });
    }

    if (positions.size === 0) return;

    const role = new Map<number, "incoming" | "outgoing" | "internal">();
    for (const n of selectedDiagram.nodes) {
      if ("ExternalIncoming" in n.kind) role.set(n.id, "incoming");
      else if ("ExternalOutgoing" in n.kind) role.set(n.id, "outgoing");
      else role.set(n.id, "internal");
    }

    const layerTargets = computeLayerTargets(selectedDiagram);

    const runPass = () => {
      const minSeparation = 56;
      const targetEdgeLength = 108;
      const left = 78;
      const right = 522;
      const denseScore = nodeCount * nodeCount + edgeCount * 12;
      const iterCount = denseScore >= 14000 ? 10 : denseScore >= 8000 ? 16 : 36;
      const pairStride = nodeCount >= 72 ? 4 : nodeCount >= 54 ? 3 : nodeCount >= 38 ? 2 : 1;
      const edgeStride = edgeCount >= 210 ? 4 : edgeCount >= 140 ? 3 : edgeCount >= 90 ? 2 : 1;

      for (let iter = 0; iter < iterCount; iter++) {
        for (let i = 0; i < ids.length; i++) {
          const jStart = i + 1 + ((i + iter) % pairStride);
          for (let j = jStart; j < ids.length; j += pairStride) {
            const a = positions.get(ids[i]);
            const b = positions.get(ids[j]);
            if (!a || !b) continue;
            const dx = b.x - a.x;
            const dy = b.y - a.y;
            const d = Math.sqrt(dx * dx + dy * dy) || 0.0001;
            const ri = role.get(ids[i]) === "internal" ? minSeparation : minSeparation + 8;
            const rj = role.get(ids[j]) === "internal" ? minSeparation : minSeparation + 8;
            const minD = (ri + rj) * 0.5;
            if (d >= minD) continue;
            const push = (minD - d) * 0.26;
            const nx = dx / d;
            const ny = dy / d;
            a.x -= nx * push;
            a.y -= ny * push;
            b.x += nx * push;
            b.y += ny * push;
          }
        }

        const edgeStart = iter % edgeStride;
        for (let edgeIdx = edgeStart; edgeIdx < edgeList.length; edgeIdx += edgeStride) {
          const [srcId, tgtId] = edgeList[edgeIdx];
          const a = positions.get(srcId);
          const b = positions.get(tgtId);
          if (!a || !b) continue;
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const d = Math.sqrt(dx * dx + dy * dy) || 0.0001;
          const delta = (d - targetEdgeLength) * 0.05;
          const nx = dx / d;
          const ny = dy / d;
          a.x += nx * delta;
          a.y += ny * delta;
          b.x -= nx * delta;
          b.y -= ny * delta;
        }

        for (const id of ids) {
          const p = positions.get(id);
          if (!p) continue;
          const r = role.get(id);
          const targetLayer = layerTargets.get(id) ?? 0.5;
          const desiredX = left + targetLayer * (right - left);
          const wx = r === "incoming" || r === "outgoing" ? 0.22 : 0.1;
          p.x += (desiredX - p.x) * wx;

          if (r === "incoming") p.x = Math.min(p.x, 172);
          if (r === "outgoing") p.x = Math.max(p.x, 428);

          const c = clampNodePosition(p.x, p.y);
          p.x = c.x;
          p.y = c.y;
        }
      }
    };

    const denseDiagram = nodeCount >= 42 || edgeCount >= 86;
    const maxLoops = denseDiagram ? (nodeCount >= 70 || edgeCount >= 150 ? 2 : 3) : 8;
    const loops = Math.max(1, Math.min(passes, maxLoops));
    for (let i = 0; i < loops; i++) {
      runPass();
    }

    const snapped = applySmartGrid(positions, selectedDiagram);
    nodePositionOverrides = Object.fromEntries(Array.from(snapped.entries()));
    savePositionCache(selectedDiagram.id);

    if (viewMode === "feynman") renderFeynman(selectedDiagram);
    if (viewMode === "worldsheet") renderWorldsheetView(selectedDiagram);
  }

  function handleCanvasMouseDown(e: MouseEvent) {
    // Only pan with left mouse button
    if (e.button !== 0) return;
    
    // If clicking on interactive elements (buttons, links), don't pan
    if ((e.target as HTMLElement).closest("button, a, input")) return;
    
    const target = e.target as HTMLElement;
    if (manualMoveEnabled && viewMode === "feynman") {
      const draggable = target.closest("[data-node-id]") as HTMLElement | null;
      if (draggable) {
        const nodeId = Number(draggable.getAttribute("data-node-id") ?? "");
        if (Number.isFinite(nodeId)) {
          const p = getNodePosition(nodeId);
          if (p) {
            draggedNodeId = nodeId;
            dragNodeOffsetX = e.clientX - p.x;
            dragNodeOffsetY = e.clientY - p.y;
            e.preventDefault();
            return;
          }
        }
      }
    }

    isPanning = true;
    panStartX = e.clientX - panX;
    panStartY = e.clientY - panY;
    e.preventDefault();
  }

  function handleCanvasMouseMove(e: MouseEvent) {
    if (draggedNodeId !== null && selectedDiagram && viewMode === "feynman") {
      const x = e.clientX - dragNodeOffsetX;
      const y = e.clientY - dragNodeOffsetY;
      const c = clampNodePosition(x, y);
      pendingDragUpdate = { id: draggedNodeId, x: c.x, y: c.y };
      if (dragRafId === null) {
        dragRafId = requestAnimationFrame(() => {
          if (!pendingDragUpdate || !selectedDiagram) {
            dragRafId = null;
            return;
          }
          const upd = pendingDragUpdate;
          pendingDragUpdate = null;
          nodePositionOverrides = {
            ...nodePositionOverrides,
            [upd.id]: { x: upd.x, y: upd.y },
          };
          savePositionCache(selectedDiagram.id);
          renderFeynman(selectedDiagram);
          dragRafId = null;
        });
      }
      e.preventDefault();
      return;
    }

    if (!isPanning) return;

    panX = e.clientX - panStartX;
    panY = e.clientY - panStartY;
    applySvgTransform();
    e.preventDefault();
  }

  function handleCanvasMouseUp(e: MouseEvent) {
    draggedNodeId = null;
    isPanning = false;
    pendingDragUpdate = null;
    if (dragRafId !== null) {
      cancelAnimationFrame(dragRafId);
      dragRafId = null;
    }
    e.preventDefault();
  }

  function handleCanvasMouseLeave(e: MouseEvent) {
    draggedNodeId = null;
    isPanning = false;
    pendingDragUpdate = null;
    if (dragRafId !== null) {
      cancelAnimationFrame(dragRafId);
      dragRafId = null;
    }
  }

  function handleCanvasWheel(e: WheelEvent) {
    // Disable default wheel behavior (scrolling)
    e.preventDefault();
  }

  function resetCanvasPan() {
    panX = 0;
    panY = 0;
    applySvgTransform();
  }

  async function scheduleRender(): Promise<void> {
    await tick();
    if (!selectedDiagram) return;

    const key = `${selectedDiagram.id}-${viewMode}`;
    if (key === lastRenderedKey) return;
    lastRenderedKey = key;

    // Reset pan when switching diagrams or modes
    resetCanvasPan();
    restorePositionCache(selectedDiagram.id);
    manualMoveEnabled = false;
    reviewRunCount = 0;

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
        svgElement = null;
        if (detachCanvasHandlers) {
          detachCanvasHandlers();
          detachCanvasHandlers = null;
        }
        break;
    }

    if (
      autoReviewEnabled &&
      (viewMode === "feynman" || viewMode === "worldsheet") &&
      shouldAutoReview(selectedDiagram) &&
      !autoReviewedKeys.has(key)
    ) {
      reviewNodeLayout(1);
      autoReviewedKeys.add(key);
    }
  }

  // ── Feynman Renderer ──────────────────────────────────────────────────

  function renderFeynman(diag: FeynmanDiagram): void {
    if (!renderContainer) return;
    bindCanvasInteraction(renderContainer);
    const cacheKey = renderCacheKey("feynman", diag.id);
    const useCache = canReuseRenderCache();
    const cached = useCache ? getCachedRender(cacheKey) : undefined;
    const svg = cached ?? renderFeynmanDiagram(diag, {
      width: 600,
      height: 420,
      nodePositionOverrides,
    });
    if (useCache && cached === undefined) setCachedRender(cacheKey, svg);
    renderContainer.innerHTML = svg;
    svgElement = renderContainer.querySelector("svg");
    if (svgElement) {
      svgElement.style.cursor = "grab";
      svgElement.addEventListener("mousedown", () => {
        svgElement!.style.cursor = "grabbing";
      });
      svgElement.addEventListener("mouseup", () => {
        svgElement!.style.cursor = "grab";
      });
      svgElement.addEventListener("mouseleave", () => {
        svgElement!.style.cursor = "grab";
      });
      applySvgTransform();
    }
  }

  // ── Worldsheet Renderer ──────────────────────────────────────────────

  function renderWorldsheetView(diag: FeynmanDiagram): void {
    if (!renderContainer) return;
    bindCanvasInteraction(renderContainer);
    const cacheKey = renderCacheKey("worldsheet", diag.id);
    const useCache = canReuseRenderCache();
    const cached = useCache ? getCachedRender(cacheKey) : undefined;
    const svg = cached ?? renderWorldsheet(diag, { width: 600, height: 420, nodePositionOverrides });
    if (useCache && cached === undefined) setCachedRender(cacheKey, svg);
    renderContainer.innerHTML = svg;
    svgElement = renderContainer.querySelector("svg");
    if (svgElement) {
      svgElement.style.cursor = "grab";
      applySvgTransform();
    }
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
        lines.push(`    ${id}(["${esc(formatParticleLabel(kind.ExternalIncoming.field.symbol))} →"]):::incoming`);
      } else if ("ExternalOutgoing" in kind) {
        lines.push(`    ${id}(["→ ${esc(formatParticleLabel(kind.ExternalOutgoing.field.symbol))}"]):::outgoing`);
      } else if ("InternalVertex" in kind) {
        lines.push(`    ${id}(("V")):::vertex`);
      }
    }

    for (const [src, tgt, edge] of diag.edges) {
      const particle = esc(formatParticleLabel(edge.field.symbol));
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
      bindCanvasInteraction(mermaidContainer);
      const cacheKey = renderCacheKey("mermaid", diag.id);
      const useCache = canReuseRenderCache();
      const cached = useCache ? getCachedRender(cacheKey) : undefined;
      if (cached !== undefined) {
        mermaidContainer.innerHTML = cached;
        svgElement = mermaidContainer.querySelector("svg");
        if (svgElement) {
          svgElement.style.cursor = "grab";
          applySvgTransform();
        }
        return;
      }
      const mermaid = (await import("mermaid")).default;
      const definition = graphToMermaid(diag);
      const uniqueId = `spire-diagram-${diag.id}-${Date.now()}`;
      mermaidContainer.innerHTML = "";
      const { svg } = await mermaid.render(uniqueId, definition);
      mermaidContainer.innerHTML = svg;
      if (useCache) setCachedRender(cacheKey, svg);
      
      // Setup panning for mermaid SVG
      svgElement = mermaidContainer.querySelector("svg");
      if (svgElement) {
        svgElement.style.cursor = "grab";
        svgElement.addEventListener("mousedown", () => {
          svgElement!.style.cursor = "grabbing";
        });
        svgElement.addEventListener("mouseup", () => {
          svgElement!.style.cursor = "grab";
        });
        svgElement.addEventListener("mouseleave", () => {
          svgElement!.style.cursor = "grab";
        });
        applySvgTransform();
      }
    } catch (e) {
      console.warn("Mermaid render error:", e);
      if (mermaidContainer) {
        mermaidContainer.innerHTML = `<pre style="color:#e74c3c;font-size:0.75rem;">Mermaid render error</pre>`;
      }
    }
  }

  // ── Text Renderer ─────────────────────────────────────────────────────

  function nodeLabel(kind: NodeKind): string {
    if ("ExternalIncoming" in kind) return `→ ${formatParticleLabel(kind.ExternalIncoming.field.symbol)} (in)`;
    if ("ExternalOutgoing" in kind) return `${formatParticleLabel(kind.ExternalOutgoing.field.symbol)} → (out)`;
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
  $: tikzCode = showTikzExport && selectedDiagram
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
        {#if viewMode === "feynman"}
          <button
            class="export-btn"
            class:active-toggle={manualMoveEnabled}
            on:click={() => {
              manualMoveEnabled = !manualMoveEnabled;
            }}
            use:tooltip={{ text: "Enable manual node movement" }}
          >
            Move Nodes {manualMoveEnabled ? "ON" : "OFF"}
          </button>
        {/if}
        {#if viewMode === "feynman" || viewMode === "worldsheet"}
          <button
            class="export-btn"
            on:click={() => {
              reviewRunCount += 1;
              const loopPasses = Math.min(8, 2 + reviewRunCount);
              reviewNodeLayout(loopPasses);
            }}
            use:tooltip={{ text: "Weighted smart-grid spacing (re-click to iterate more)" }}
          >
            Review Positions ×{Math.min(8, 2 + reviewRunCount)}
          </button>
        {/if}
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
          <div class="svg-container" bind:this={renderContainer}></div>
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

  .export-btn.active-toggle {
    color: var(--hl-success);
    border-color: var(--hl-success);
    background: color-mix(in srgb, var(--bg-surface) 75%, var(--hl-success) 25%);
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
    overflow: hidden;
    min-height: 120px;
    max-height: 500px;
    position: relative;
  }

  .svg-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    overflow: hidden;
    min-height: 200px;
    max-height: 500px;
    position: relative;
    cursor: grab;
  }

  .svg-container :global(svg) {
    width: 600px;
    height: 420px;
    flex-shrink: 0;
    transition: transform 0.05s linear;
    user-select: none;
    pointer-events: auto;
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
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 160px;
  }
  .mermaid-container :global(svg) {
    width: 600px;
    height: 420px;
    flex-shrink: 0;
    transition: transform 0.05s linear;
    user-select: none;
    pointer-events: auto;
  }
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
