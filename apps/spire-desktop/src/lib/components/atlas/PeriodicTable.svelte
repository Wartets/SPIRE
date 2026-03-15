<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { onMount, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import IsotopeDrawer from "$lib/components/atlas/IsotopeDrawer.svelte";
  import {
    loadElements,
    loadIsotopesForZ,
    type ElementData,
    type IsotopeData,
  } from "$lib/core/physics/nuclearDataLoader";
  import { broadcastSelection } from "$lib/stores/selectionBus";
  import { appendLog } from "$lib/stores/physicsStore";

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let elements: ElementData[] = [];
  let selectedElement: ElementData | null = null;
  let selectedIsotopeData: IsotopeData | null = null;
  let isotopeMap: Record<number, IsotopeData[]> = {};
  let flashZ: number | null = null;
  let flashTimer: ReturnType<typeof setTimeout> | null = null;
  let elementQuery = "";
  let categoryFilter: "all" | ElementData["category"] = "all";
  let periodicViewportEl: HTMLDivElement | null = null;
  let periodicGridEl: HTMLDivElement | null = null;
  let panX = 0;
  let panY = 0;
  let zoom = 1;
  let isDragging = false;
  let activePointerId: number | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let panStartX = 0;
  let panStartY = 0;
  let suppressNextClick = false;

  const ZOOM_MIN = 0.65;
  const ZOOM_MAX = 2.25;

  const dispatch = createEventDispatcher<{
    clearInfo: void;
    elementSelect: ElementData;
  }>();

  $: visibleElements = elements.filter((el) => {
    const q = elementQuery.trim().toLowerCase();
    const categoryMatch = categoryFilter === "all" || el.category === categoryFilter;
    if (!categoryMatch) return false;
    if (!q) return true;
    return (
      el.name.toLowerCase().includes(q) ||
      el.symbol.toLowerCase().includes(q) ||
      String(el.Z).includes(q)
    );
  });

  $: if (elementQuery.trim().length >= 2 && visibleElements.length === 1) {
    focusElement(visibleElements[0]);
  }

  onMount(async () => {
    elements = await loadElements();
    await tick();
    centerPeriodicGrid();
  });

  $: if (selectedElement) {
    focusElement(selectedElement);
  }

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  async function handleElementClick(el: ElementData): Promise<void> {
    if (selectedElement?.Z === el.Z) {
      selectedElement = null;
      selectedIsotopeData = null;
      dispatch("clearInfo");
      return;
    }

    selectedElement = el;
    selectedIsotopeData = null;
    dispatch("elementSelect", el);

    if (!isotopeMap[el.Z]) {
      isotopeMap[el.Z] = await loadIsotopesForZ(el.Z);
    }

    broadcastSelection({ type: "ELEMENT_SELECTED", data: el });
    appendLog(`Periodic table: selected ${el.name} (Z=${el.Z})`);
    triggerFlash(el.Z);
  }

  function handleCellClick(el: ElementData): void {
    if (suppressNextClick) {
      suppressNextClick = false;
      return;
    }
    void handleElementClick(el);
  }

  function handleIsotopeSelect(
    event: CustomEvent<{ Z: number; A: number; symbol: string; name: string; isotope: IsotopeData }>,
  ): void {
    const d = event.detail;
    selectedIsotopeData = d.isotope;
    broadcastSelection({ type: "ISOTOPE_SELECTED", data: d });
    appendLog(`Isotope selected: ${d.symbol}-${d.A} (Z=${d.Z})`);
    triggerFlash(d.Z);
  }

  function triggerFlash(Z: number): void {
    if (flashTimer !== null) clearTimeout(flashTimer);
    flashZ = Z;
    flashTimer = setTimeout(() => {
      flashZ = null;
      flashTimer = null;
    }, 800);
  }

  function clamp(v: number, lo: number, hi: number): number {
    return Math.max(lo, Math.min(hi, v));
  }

  function centerPeriodicGrid(): void {
    if (!periodicViewportEl || !periodicGridEl) return;
    const viewport = periodicViewportEl.getBoundingClientRect();
    const grid = periodicGridEl.getBoundingClientRect();
    panX = Math.max(0, (viewport.width - grid.width * zoom) * 0.5);
    panY = Math.max(0, (viewport.height - grid.height * zoom) * 0.5);
  }

  function focusElement(el: ElementData): void {
    if (!periodicViewportEl || !periodicGridEl) return;
    const viewport = periodicViewportEl.getBoundingClientRect();
    const cellW = gridCellWidth();
    const cellH = gridCellHeight();
    const targetX = (el.display_col - 0.5) * cellW * zoom;
    const targetY = (el.display_row - 0.5) * cellH * zoom;
    panX = viewport.width * 0.5 - targetX;
    panY = viewport.height * 0.4 - targetY;
  }

  function gridCellWidth(): number {
    if (!periodicGridEl) return 56;
    const style = getComputedStyle(periodicGridEl);
    const columns = style.gridTemplateColumns.split(" ").length || 18;
    return periodicGridEl.scrollWidth / columns;
  }

  function gridCellHeight(): number {
    if (!periodicGridEl) return 52;
    return periodicGridEl.scrollHeight / 9;
  }

  function resetView(): void {
    zoom = 1;
    centerPeriodicGrid();
  }

  function zoomBy(delta: number): void {
    const next = clamp(zoom + delta, ZOOM_MIN, ZOOM_MAX);
    if (Math.abs(next - zoom) < 1e-6) return;
    zoom = next;
  }

  function beginDrag(event: PointerEvent): void {
    if (!periodicViewportEl) return;
    isDragging = true;
    activePointerId = event.pointerId;
    periodicViewportEl.setPointerCapture(event.pointerId);
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    panStartX = panX;
    panStartY = panY;
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!isDragging || event.pointerId !== activePointerId) return;
    const dx = event.clientX - dragStartX;
    const dy = event.clientY - dragStartY;
    panX = panStartX + dx;
    panY = panStartY + dy;
    if (Math.abs(dx) + Math.abs(dy) > 4) suppressNextClick = true;
  }

  function endDrag(event: PointerEvent): void {
    if (!isDragging || event.pointerId !== activePointerId) return;
    if (periodicViewportEl?.hasPointerCapture(event.pointerId)) {
      periodicViewportEl.releasePointerCapture(event.pointerId);
    }
    isDragging = false;
    activePointerId = null;
  }

  function handleWheel(event: WheelEvent): void {
    event.preventDefault();
    zoomBy(event.deltaY < 0 ? 0.08 : -0.08);
  }

  function handleViewportKeydown(event: KeyboardEvent): void {
    const step = 28;
    if (event.key === "ArrowLeft") {
      panX += step;
      event.preventDefault();
    } else if (event.key === "ArrowRight") {
      panX -= step;
      event.preventDefault();
    } else if (event.key === "ArrowUp") {
      panY += step;
      event.preventDefault();
    } else if (event.key === "ArrowDown") {
      panY -= step;
      event.preventDefault();
    } else if (event.key === "+" || event.key === "=") {
      zoomBy(0.08);
      event.preventDefault();
    } else if (event.key === "-") {
      zoomBy(-0.08);
      event.preventDefault();
    } else if (event.key.toLowerCase() === "r") {
      resetView();
      event.preventDefault();
    }
  }

  $: zoomPercent = `${Math.round(zoom * 100)}%`;

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  function categoryColor(category: string): string {
    switch (category) {
      case "alkali-metal":          return "var(--hl-error)";
      case "alkaline-earth-metal":  return "var(--hl-value)";
      case "transition-metal":      return "var(--hl-symbol)";
      case "metalloid":             return "#9b59b6";
      case "nonmetal":              return "var(--hl-success)";
      case "halogen":               return "#e67e22";
      case "noble-gas":             return "#1abc9c";
      case "post-transition-metal": return "#5d9cec";
      case "lanthanide":            return "#b8860b";
      case "actinide":              return "#c0392b";
      default:                      return "var(--fg-secondary)";
    }
  }

  function elementTooltip(el: ElementData): string {
    const block = el.group == null
      ? "f"
      : el.group <= 2
        ? "s"
        : el.group <= 12
          ? "d"
          : "p";
    const phaseHint =
      ["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(el.symbol)
        ? "gas"
        : ["Br", "Hg"].includes(el.symbol)
          ? "liquid"
          : "solid";

    const lines = [
      `${el.name} (${el.symbol})`,
      `Z = ${el.Z}  ·  A\u2248${el.atomic_mass.toFixed(3)}`,
      `Period ${el.period}${el.group != null ? ", Group " + el.group : ", f-block"}`,
      `Category: ${el.category}`,
      `Block: ${block}-block · phase@STP: ${phaseHint}`,
      `Config: ${el.electron_configuration}`,
    ];
    return lines.join("\n");
  }
</script>

<div class="pt-outer">
  <!-- Legend row -->
  <div class="pt-legend">
    {#each [["alkali-metal","Alkali"],["alkaline-earth-metal","Alk. Earth"],["transition-metal","Transition"],["metalloid","Metalloid"],["nonmetal","Nonmetal"],["halogen","Halogen"],["noble-gas","Noble Gas"],["post-transition-metal","Post-Trans."],["lanthanide","Lanthanide"],["actinide","Actinide"]] as [cat, lbl]}
      <span class="legend-item" style="--lc:{categoryColor(cat)}">{lbl}</span>
    {/each}
  </div>

  <div class="pt-controls">
    <input
      class="pt-search"
      type="search"
      placeholder="Search by name, symbol, or Z…"
      bind:value={elementQuery}
    />
    <select class="pt-filter" bind:value={categoryFilter}>
      <option value="all">All categories</option>
      <option value="alkali-metal">Alkali metals</option>
      <option value="alkaline-earth-metal">Alkaline earth</option>
      <option value="transition-metal">Transition</option>
      <option value="post-transition-metal">Post-transition</option>
      <option value="metalloid">Metalloid</option>
      <option value="nonmetal">Nonmetal</option>
      <option value="halogen">Halogen</option>
      <option value="noble-gas">Noble gas</option>
      <option value="lanthanide">Lanthanide</option>
      <option value="actinide">Actinide</option>
    </select>

    <div class="nav-controls">
      <button type="button" on:click={() => zoomBy(-0.1)} aria-label="Zoom out">−</button>
      <span class="zoom-indicator">{zoomPercent}</span>
      <button type="button" on:click={() => zoomBy(0.1)} aria-label="Zoom in">+</button>
      <button type="button" on:click={resetView} aria-label="Reset periodic table view">Reset</button>
      {#if selectedElement}
        <button type="button" on:click={() => focusElement(selectedElement!)} aria-label="Focus selected element">Focus selection</button>
      {/if}
    </div>
  </div>

  <!-- Main 18-column grid -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
  <div
    class="periodic-viewport"
    bind:this={periodicViewportEl}
    data-wheel-capture
    role="application"
    tabindex="0"
    aria-label="Interactive periodic table viewport"
    on:pointerdown={beginDrag}
    on:pointermove={handlePointerMove}
    on:pointerup={endDrag}
    on:pointercancel={endDrag}
    on:wheel|stopPropagation={handleWheel}
    on:keydown={handleViewportKeydown}
  >
    <div
      class="periodic-wrap"
      bind:this={periodicGridEl}
      style={`transform: translate(${panX}px, ${panY}px) scale(${zoom});`}
    >
      {#each visibleElements as el (el.Z)}
        <button
          class="cell"
          class:cell--selected={selectedElement?.Z === el.Z}
          class:cell--flash={flashZ === el.Z}
          style="grid-column:{el.display_col};grid-row:{el.display_row};--accent:{categoryColor(el.category)}"
          use:tooltip={{ text: elementTooltip(el), maxWidth: 380 }}
          aria-label={`Element ${el.name}, Z=${el.Z}`}
          on:click={() => handleCellClick(el)}
        >
          <span class="z">{el.Z}</span>
          <span class="symbol">{el.symbol}</span>
          <span class="ename">{el.name}</span>
        </button>
      {/each}

      <!-- f-block separator labels -->
      <div class="fblock-label" style="grid-column:1/4;grid-row:8">Lanthanides</div>
      <div class="fblock-label" style="grid-column:1/4;grid-row:9">Actinides</div>
    </div>
  </div>

  <!-- Isotope drawer (slides open when an element is selected) -->
  {#if selectedElement}
    <IsotopeDrawer
      element={selectedElement}
      isotopes={isotopeMap[selectedElement.Z] ?? []}
      bind:selectedIsotope={selectedIsotopeData}
      on:isotopeSelect={handleIsotopeSelect}
      on:close={() => {
        selectedElement = null;
        selectedIsotopeData = null;
        dispatch("clearInfo");
      }}
    />
  {/if}
</div>

<style>
  .pt-outer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 0.3rem;
    overflow: hidden;
  }

  .pt-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.4rem;
    padding: 0.2rem 0.1rem;
    border-bottom: 1px solid var(--color-border, var(--border));
    flex-shrink: 0;
  }

  .legend-item {
    font-family: var(--font-mono);
    font-size: 0.58rem;
    color: var(--lc);
    border: 1px solid var(--lc);
    padding: 0.05rem 0.25rem;
    opacity: 0.85;
  }

  .periodic-viewport {
    position: relative;
    overflow: hidden;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    flex: 1 1 0;
    min-height: 0;
    touch-action: none;
  }

  .periodic-wrap {
    display: grid;
    grid-template-columns: repeat(18, minmax(2.4rem, 1fr));
    grid-template-rows: repeat(9, auto);
    gap: 0.15rem;
    padding: 0.15rem;
    width: 56rem;
    transform-origin: 0 0;
    will-change: transform;
  }

  .pt-controls {
    display: flex;
    gap: 0.3rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .nav-controls {
    margin-left: auto;
    display: inline-flex;
    gap: 0.22rem;
    align-items: center;
  }

  .nav-controls button {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.62rem;
    padding: 0.14rem 0.33rem;
    cursor: pointer;
  }

  .zoom-indicator {
    font-family: var(--font-mono);
    font-size: 0.61rem;
    color: var(--color-text-muted, var(--fg-secondary));
    min-width: 2.8rem;
    text-align: center;
  }

  .pt-search,
  .pt-filter {
    background: var(--color-bg-inset, var(--bg-inset));
    border: 1px solid var(--color-border, var(--border));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.65rem;
    padding: 0.15rem 0.28rem;
  }

  .pt-search {
    flex: 1;
    min-width: 14rem;
  }

  .cell {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    min-height: 2.8rem;
    padding: 0.15rem 0.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.02rem;
    border-left: 3px solid var(--accent);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-mono);
    transition: background 0.12s;
  }

  .cell:hover {
    background: color-mix(in srgb, var(--accent) 14%, var(--color-bg-inset, var(--bg-inset)));
  }

  .cell--selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, var(--color-bg-inset, var(--bg-inset)));
    outline: 1px solid var(--accent);
  }

  .cell--flash {
    animation: flash-pulse 0.8s ease-out;
  }

  @keyframes flash-pulse {
    0%   { outline: 2px solid var(--color-success, var(--hl-success)); box-shadow: 0 0 8px var(--color-success, var(--hl-success)); }
    100% { outline: 2px solid transparent; box-shadow: none; }
  }

  .z {
    font-size: 0.56rem;
    color: var(--color-text-muted, var(--fg-secondary));
    line-height: 1;
  }

  .symbol {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--color-text-primary, var(--fg-primary));
    line-height: 1.1;
  }

  .ename {
    font-size: 0.48rem;
    color: var(--color-text-muted, var(--fg-secondary));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.1;
  }

  .fblock-label {
    font-family: var(--font-mono);
    font-size: 0.58rem;
    color: var(--color-text-muted, var(--fg-secondary));
    display: flex;
    align-items: center;
    padding-left: 0.2rem;
    border-right: 1px dashed var(--color-border, var(--border));
    white-space: nowrap;
  }
</style>
