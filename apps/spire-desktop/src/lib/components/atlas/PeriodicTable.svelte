<script lang="ts">
  import { onMount, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import type { ElementData } from "$lib/core/physics/nuclearDataLoader";
  import {
    atlasPeriodicState,
    atlasPeriodicVisibleElements,
    initializeAtlasPeriodicStore,
    selectAtlasPeriodicElement,
    setAtlasPeriodicCategoryFilter,
    setAtlasPeriodicQuery,
    type AtlasPeriodicCategoryFilter,
  } from "$lib/stores/atlasPeriodicStore";

  let periodicViewportEl: HTMLDivElement | null = null;
  let periodicGridEl: HTMLDivElement | null = null;
  let panX = 0;
  let panY = 0;
  let zoom = 0.5;
  let isDragging = false;
  let hoveredElement: ElementData | null = null;
  let activePointerId: number | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let panStartX = 0;
  let panStartY = 0;
  let suppressNextClick = false;
  let viewportResizeObserver: ResizeObserver | null = null;
  let initialCenterDone = false;
  let panAnimationFrame: number | null = null;
  let zoomSettleTimer: ReturnType<typeof setTimeout> | null = null;
  let isZooming = false;

  const ZOOM_MIN = 0.3;
  const ZOOM_MAX = 4.75;
  const PAN_OVERSCROLL_RATIO_X = 0.85;
  const PAN_OVERSCROLL_MAX_X = 4200;
  const PAN_OVERSCROLL_RATIO_Y = 0.55;
  const PAN_OVERSCROLL_MAX_Y = 2400;

  onMount(() => {
    void (async () => {
      await initializeAtlasPeriodicStore();
      await tick();
      centerPeriodicGrid();
      requestAnimationFrame(() => centerPeriodicGrid());
    })();

    if (periodicViewportEl) {
      viewportResizeObserver = new ResizeObserver(() => {
        const [nextX, nextY] = constrainPan(panX, panY);
        panX = nextX;
        panY = nextY;
      });
      viewportResizeObserver.observe(periodicViewportEl);
    }

    return () => {
      viewportResizeObserver?.disconnect();
      viewportResizeObserver = null;
      if (panAnimationFrame !== null) cancelAnimationFrame(panAnimationFrame);
      if (zoomSettleTimer) clearTimeout(zoomSettleTimer);
    };
  });

  $: if ($atlasPeriodicState.query.trim().length >= 2 && $atlasPeriodicVisibleElements.length === 1) {
    focusElement($atlasPeriodicVisibleElements[0]);
  }

  $: if ($atlasPeriodicState.ready && periodicViewportEl && periodicGridEl && !initialCenterDone) {
    initialCenterDone = true;
    requestAnimationFrame(() => centerPeriodicGrid());
  }

  $: selectedLabel = $atlasPeriodicState.selectedIsotope
    ? `${$atlasPeriodicState.selectedIsotope.symbol}-${$atlasPeriodicState.selectedIsotope.A}`
    : $atlasPeriodicState.selectedElement
      ? `${$atlasPeriodicState.selectedElement.symbol} (${$atlasPeriodicState.selectedElement.name})`
      : "none";

  $: visibleStableCount = $atlasPeriodicVisibleElements.reduce(
    (sum, element) => sum + ($atlasPeriodicState.isotopeStats[element.Z]?.stable ?? 0),
    0,
  );

  $: visibleIsotopeCount = $atlasPeriodicVisibleElements.reduce(
    (sum, element) => sum + ($atlasPeriodicState.isotopeStats[element.Z]?.count ?? 0),
    0,
  );

  function categoryColor(category: string): string {
    switch (category) {
      case "alkali-metal":
        return "var(--hl-error)";
      case "alkaline-earth-metal":
        return "var(--hl-value)";
      case "transition-metal":
        return "var(--hl-symbol)";
      case "metalloid":
        return "#8f8fb8";
      case "nonmetal":
        return "var(--hl-success)";
      case "halogen":
        return "#d37f2c";
      case "noble-gas":
        return "#4db7be";
      case "post-transition-metal":
        return "#5d9cec";
      case "lanthanide":
        return "#b8860b";
      case "actinide":
        return "#c65a4b";
      default:
        return "var(--fg-secondary)";
    }
  }

  function blockFor(element: ElementData): "s" | "p" | "d" | "f" {
    if (element.group == null) return "f";
    if (element.group <= 2) return "s";
    if (element.group <= 12) return "d";
    return "p";
  }

  function phaseFor(element: ElementData): string {
    if (["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(element.symbol)) return "gas";
    if (["Br", "Hg"].includes(element.symbol)) return "liq";
    return "sol";
  }

  function valenceShell(element: ElementData): string {
    const tokens = element.electron_configuration.split(" ");
    return tokens[tokens.length - 1] ?? element.electron_configuration;
  }

  function compactValenceShell(element: ElementData): string {
    const shell = valenceShell(element);
    return shell.replace(/\s+/g, "").replace(/\^/g, "");
  }

  function compactPeriodGroupLabel(element: ElementData): string {
    return element.group != null ? `P${element.period} G${element.group}` : `P${element.period} f`;
  }

  function elementTooltip(element: ElementData): string {
    const stats = $atlasPeriodicState.isotopeStats[element.Z];
    const lines = [
      `${element.name} (${element.symbol})`,
      `Z = ${element.Z} | A~${element.atomic_mass.toFixed(3)}`,
      `Period ${element.period}${element.group != null ? `, Group ${element.group}` : ", f-block"}`,
      `Category: ${element.category.replace(/-/g, " ")}`,
      `Block: ${blockFor(element)}-block | phase@STP: ${phaseFor(element)}`,
      `Known isotopes: ${stats?.count ?? 0} | stable: ${stats?.stable ?? 0}`,
      `Config: ${element.electron_configuration}`,
    ];
    return lines.join("\n");
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, value));
  }

  function gridCellWidth(): number {
    if (!periodicGridEl) return 44;
    const style = getComputedStyle(periodicGridEl);
    const columns = style.gridTemplateColumns.split(" ").length || 18;
    return periodicGridEl.scrollWidth / columns;
  }

  function gridCellHeight(): number {
    if (!periodicGridEl) return 38;
    return periodicGridEl.scrollHeight / 9;
  }

  function centerPeriodicGrid(): void {
    if (!periodicViewportEl || !periodicGridEl) return;
    const viewport = periodicViewportEl.getBoundingClientRect();
    const gridWidth = periodicGridEl.offsetWidth;
    const gridHeight = periodicGridEl.offsetHeight;
    const targetX = (viewport.width - gridWidth * zoom) * 0.5;
    const targetY = (viewport.height - gridHeight * zoom) * 0.5;
    setPan(targetX, targetY, false);
  }

  function focusElement(element: ElementData): void {
    if (!periodicViewportEl || !periodicGridEl) return;
    const viewport = periodicViewportEl.getBoundingClientRect();
    const targetX = (element.display_col - 0.5) * gridCellWidth() * zoom;
    const targetY = (element.display_row - 0.5) * gridCellHeight() * zoom;
    setPan(viewport.width * 0.5 - targetX, viewport.height * 0.46 - targetY, true);
  }

  function resetView(): void {
    zoom = 0.92;
    centerPeriodicGrid();
  }

  function zoomBy(delta: number): void {
    if (!periodicViewportEl) return;
    const nextZoom = clamp(zoom + delta, ZOOM_MIN, ZOOM_MAX);
    if (Math.abs(nextZoom - zoom) < 1e-6) return;

    const viewport = periodicViewportEl.getBoundingClientRect();
    const anchorX = viewport.width * 0.5;
    const anchorY = viewport.height * 0.5;
    const worldX = (anchorX - panX) / zoom;
    const worldY = (anchorY - panY) / zoom;

    zoom = nextZoom;
    markZooming();
    const targetPanX = anchorX - worldX * zoom;
    const targetPanY = anchorY - worldY * zoom;
    const [nextX, nextY] = constrainPan(targetPanX, targetPanY);
    panX = nextX;
    panY = nextY;
  }

  function markZooming(): void {
    isZooming = true;
    if (zoomSettleTimer) clearTimeout(zoomSettleTimer);
    zoomSettleTimer = setTimeout(() => {
      isZooming = false;
      zoomSettleTimer = null;
    }, 120);
  }

  function setPan(targetX: number, targetY: number, animated: boolean): void {
    const [boundedX, boundedY] = constrainPan(targetX, targetY);
    if (!animated) {
      panX = boundedX;
      panY = boundedY;
      return;
    }

    if (panAnimationFrame !== null) cancelAnimationFrame(panAnimationFrame);
    const startX = panX;
    const startY = panY;
    const startTime = performance.now();
    const duration = 170;

    const step = (time: number): void => {
      const t = Math.min(1, (time - startTime) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      panX = startX + (boundedX - startX) * eased;
      panY = startY + (boundedY - startY) * eased;
      if (t < 1) {
        panAnimationFrame = requestAnimationFrame(step);
      } else {
        panAnimationFrame = null;
      }
    };

    panAnimationFrame = requestAnimationFrame(step);
  }

  function constrainPan(targetX: number, targetY: number): [number, number] {
    if (!periodicViewportEl || !periodicGridEl) return [targetX, targetY];

    const viewport = periodicViewportEl.getBoundingClientRect();
    const scaledWidth = periodicGridEl.offsetWidth * zoom;
    const scaledHeight = periodicGridEl.offsetHeight * zoom;
    const overscrollX = Math.min(viewport.width * PAN_OVERSCROLL_RATIO_X, PAN_OVERSCROLL_MAX_X);
    const overscrollY = Math.min(viewport.height * PAN_OVERSCROLL_RATIO_Y, PAN_OVERSCROLL_MAX_Y);

    const minX = scaledWidth <= viewport.width
      ? (viewport.width - scaledWidth) * 0.5 - overscrollX
      : viewport.width - scaledWidth - overscrollX;
    const maxX = scaledWidth <= viewport.width
      ? (viewport.width - scaledWidth) * 0.5 + overscrollX
      : overscrollX;

    const minY = scaledHeight <= viewport.height
      ? (viewport.height - scaledHeight) * 0.5 - overscrollY
      : viewport.height - scaledHeight - overscrollY;
    const maxY = scaledHeight <= viewport.height
      ? (viewport.height - scaledHeight) * 0.5 + overscrollY
      : overscrollY;

    return [
      clamp(targetX, Math.min(minX, maxX), Math.max(minX, maxX)),
      clamp(targetY, Math.min(minY, maxY), Math.max(minY, maxY)),
    ];
  }

  function beginDrag(event: PointerEvent): void {
    if (!periodicViewportEl) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest(".cell")) return;
    event.preventDefault();
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
    event.preventDefault();
    const dx = event.clientX - dragStartX;
    const dy = event.clientY - dragStartY;
    const [nextX, nextY] = constrainPan(panStartX + dx, panStartY + dy);
    panX = nextX;
    panY = nextY;
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
    if (!periodicViewportEl) return;

    const viewport = periodicViewportEl.getBoundingClientRect();
    const anchorX = event.clientX - viewport.left;
    const anchorY = event.clientY - viewport.top;
    const worldX = (anchorX - panX) / zoom;
    const worldY = (anchorY - panY) / zoom;
    const delta = clamp(event.deltaY, -140, 140);
    const zoomFactor = Math.exp(-delta * 0.0018);
    const nextZoom = clamp(zoom * zoomFactor, ZOOM_MIN, ZOOM_MAX);
    if (Math.abs(nextZoom - zoom) < 1e-6) return;

    zoom = nextZoom;
    markZooming();
    const targetPanX = anchorX - worldX * zoom;
    const targetPanY = anchorY - worldY * zoom;
    const [nextX, nextY] = constrainPan(targetPanX, targetPanY);
    panX = nextX;
    panY = nextY;
  }

  function handleViewportKeydown(event: KeyboardEvent): void {
    const step = 28;
    if (event.key === "ArrowLeft") {
      const [nextX, nextY] = constrainPan(panX + step, panY);
      panX = nextX;
      panY = nextY;
      event.preventDefault();
    } else if (event.key === "ArrowRight") {
      const [nextX, nextY] = constrainPan(panX - step, panY);
      panX = nextX;
      panY = nextY;
      event.preventDefault();
    } else if (event.key === "ArrowUp") {
      const [nextX, nextY] = constrainPan(panX, panY + step);
      panX = nextX;
      panY = nextY;
      event.preventDefault();
    } else if (event.key === "ArrowDown") {
      const [nextX, nextY] = constrainPan(panX, panY - step);
      panX = nextX;
      panY = nextY;
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

  function handleCellClick(element: ElementData): void {
    if (suppressNextClick) {
      suppressNextClick = false;
      return;
    }

    if ($atlasPeriodicState.selectedElement?.Z === element.Z && !$atlasPeriodicState.selectedIsotope) {
      return;
    }

    selectAtlasPeriodicElement(element);
    requestAnimationFrame(() => focusElement(element));
  }

  function focusSelectedElement(): void {
    if ($atlasPeriodicState.selectedElement) {
      focusElement($atlasPeriodicState.selectedElement);
    }
  }
</script>

<div class="pt-outer">
  <section class="pt-toolbar">
    <div class="toolbar-main">
      <input
        class="pt-search"
        type="search"
        placeholder="Search by name, symbol, or Z..."
        value={$atlasPeriodicState.query}
        on:input={(event) => setAtlasPeriodicQuery((event.currentTarget as HTMLInputElement).value)}
      />

      <label class="control-field">
        <span>Category</span>
        <select
          class="pt-filter"
          value={$atlasPeriodicState.categoryFilter}
          on:change={(event) =>
            setAtlasPeriodicCategoryFilter((event.currentTarget as HTMLSelectElement).value as AtlasPeriodicCategoryFilter)}
        >
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
      </label>

      <div class="nav-controls">
        <button type="button" on:click={() => zoomBy(-0.1)} aria-label="Zoom out">-</button>
        <span class="zoom-indicator">{Math.round(zoom * 100)}%</span>
        <button type="button" on:click={() => zoomBy(0.1)} aria-label="Zoom in">+</button>
        <button type="button" on:click={resetView} aria-label="Reset periodic table view">Reset</button>
        {#if $atlasPeriodicState.selectedElement}
          <button type="button" on:click={focusSelectedElement} aria-label="Focus selected element">Focus</button>
        {/if}
      </div>
    </div>

    <div class="toolbar-metrics">
      <article>
        <span>Selection</span>
        <strong>{selectedLabel}</strong>
      </article>
      <article>
        <span>Visible elements</span>
        <strong>{$atlasPeriodicVisibleElements.length}</strong>
      </article>
      <article>
        <span>Visible isotopes</span>
        <strong>{visibleIsotopeCount}</strong>
      </article>
      <article>
        <span>Stable isotopes</span>
        <strong>{visibleStableCount}</strong>
      </article>
    </div>
  </section>

  <div class="pt-legend">
    {#each [
      ["alkali-metal", "Alkali"],
      ["alkaline-earth-metal", "Alk. Earth"],
      ["transition-metal", "Transition"],
      ["metalloid", "Metalloid"],
      ["nonmetal", "Nonmetal"],
      ["halogen", "Halogen"],
      ["noble-gas", "Noble Gas"],
      ["post-transition-metal", "Post-Trans."],
      ["lanthanide", "Lanthanide"],
      ["actinide", "Actinide"],
    ] as [category, label]}
      <span class="legend-item" style={`--lc:${categoryColor(category)}`}>{label}</span>
    {/each}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
  <div
    class="periodic-viewport"
    bind:this={periodicViewportEl}
    data-wheel-capture
    data-dragging={isDragging ? "true" : "false"}
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
      class:periodic-wrap--zooming={isZooming}
      bind:this={periodicGridEl}
      style={`transform: translate(${panX}px, ${panY}px) scale(${zoom});`}
    >
      {#each $atlasPeriodicVisibleElements as element (element.Z)}
        {@const stats = $atlasPeriodicState.isotopeStats[element.Z]}
        <button
          class="cell"
          class:cell--selected={$atlasPeriodicState.selectedElement?.Z === element.Z}
          style={`grid-column:${element.display_col};grid-row:${element.display_row};--accent:${categoryColor(element.category)}`}
          use:tooltip={{ text: elementTooltip(element), maxWidth: 380 }}
          aria-label={`Element ${element.name}, Z=${element.Z}`}
          on:mouseenter={() => (hoveredElement = element)}
          on:focus={() => (hoveredElement = element)}
          on:click={() => handleCellClick(element)}
        >
          <span class="cell-corner cell-corner--tl">Z{element.Z}</span>
          <span class="cell-corner cell-corner--tr">{element.atomic_mass.toFixed(1)}</span>
          <span class="symbol">{element.symbol}</span>
          <span class="name" title={element.name}>{element.name}</span>
          <span class="cell-corner cell-corner--bl">{compactPeriodGroupLabel(element)}</span>
          <span class="cell-corner cell-corner--br">{stats?.count ?? 0}i · {compactValenceShell(element)}</span>
        </button>
      {/each}

      <div class="fblock-label" style="grid-column:1/4;grid-row:8">Lanthanides</div>
      <div class="fblock-label" style="grid-column:1/4;grid-row:9">Actinides</div>
    </div>

    <button
      type="button"
      class="zoom-overlay"
      on:click={resetView}
      aria-label="Reset zoom and recenter periodic table"
      title="Reset zoom"
    >
      <span class="zoom-overlay__label">Zoom</span>
      <strong>{Math.round(zoom * 100)}%</strong>
    </button>
  </div>
</div>

<style>
  .pt-outer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    gap: 0.35rem;
    overflow: hidden;
  }

  .pt-toolbar {
    display: flex;
    flex-direction: column;
    gap: 0.22rem;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    padding: 0.26rem;
    flex-shrink: 0;
  }

  .toolbar-main {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(11rem, 13rem) auto;
    gap: 0.2rem;
    align-items: end;
  }

  .control-field {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .control-field span,
  .toolbar-metrics span,
  .toolbar-metrics strong,
  .legend-item,
  .pt-search,
  .pt-filter,
  .nav-controls button,
  .zoom-indicator,
  .cell,
  .fblock-label {
    font-family: var(--font-mono);
  }

  .control-field span {
    font-size: 0.5rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .pt-search,
  .pt-filter,
  .nav-controls button {
    background: var(--color-bg-inset, var(--bg-inset));
    border: 1px solid var(--color-border, var(--border));
    color: var(--color-text-primary, var(--fg-primary));
    font-size: 0.54rem;
    padding: 0.14rem 0.24rem;
    transition: border-color 90ms ease-out, background-color 90ms ease-out, color 90ms ease-out;
  }

  .nav-controls {
    display: inline-flex;
    gap: 0.22rem;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .zoom-indicator {
    font-size: 0.52rem;
    color: var(--color-text-muted, var(--fg-secondary));
    min-width: 2.8rem;
    text-align: center;
  }

  .toolbar-metrics {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.2rem;
  }

  .toolbar-metrics article {
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.16);
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.16rem 0.22rem;
    min-width: 0;
  }

  .toolbar-metrics span {
    display: block;
    font-size: 0.48rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--color-text-muted, var(--fg-secondary));
    margin-bottom: 0.12rem;
  }

  .toolbar-metrics strong {
    display: block;
    font-size: 0.58rem;
    color: var(--color-text-primary, var(--fg-primary));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pt-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.14rem 0.24rem;
    padding: 0.12rem 0.06rem;
    border-bottom: 1px solid var(--color-border, var(--border));
    flex-shrink: 0;
  }

  .legend-item {
    font-size: 0.46rem;
    color: var(--lc);
    border: 1px solid var(--lc);
    padding: 0.03rem 0.16rem;
    opacity: 0.88;
  }

  .periodic-viewport {
    position: relative;
    overflow: hidden;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    flex: 1 1 0;
    min-height: 0;
    touch-action: none;
    cursor: grab;
    user-select: none;
    -webkit-user-select: none;
  }

  .zoom-overlay {
    position: absolute;
    right: 0.5rem;
    bottom: 0.48rem;
    z-index: 3;
    display: inline-flex;
    align-items: center;
    gap: 0.28rem;
    border: 1px solid var(--color-border, var(--border));
    background: color-mix(in srgb, var(--color-bg-surface, var(--bg-surface)) 88%, transparent);
    color: var(--color-text-primary, var(--fg-primary));
    padding: 0.2rem 0.34rem;
    font-family: var(--font-mono);
    font-size: 0.52rem;
    cursor: pointer;
    backdrop-filter: blur(2px);
    transition: border-color 90ms ease-out, background-color 90ms ease-out, transform 90ms ease-out;
  }

  .zoom-overlay:hover {
    border-color: var(--color-accent, var(--hl-symbol));
    background: color-mix(in srgb, var(--color-bg-surface, var(--bg-surface)) 78%, transparent);
    transform: translateY(-1px);
  }

  .zoom-overlay__label {
    color: var(--color-text-muted, var(--fg-secondary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .zoom-overlay strong {
    font-size: 0.62rem;
    color: var(--color-accent, var(--hl-symbol));
    min-width: 2.4rem;
    text-align: center;
  }

  .periodic-viewport:active,
  .periodic-viewport[data-dragging="true"] {
    cursor: grabbing;
  }

  .periodic-wrap {
    display: grid;
    grid-template-columns: repeat(18, minmax(3.8rem, 1fr));
    grid-template-rows: repeat(9, auto);
    gap: 0.38rem;
    padding: 0.45rem;
    width: 122rem;
    transform-origin: 0 0;
    will-change: transform;
    transition: transform 90ms cubic-bezier(0.2, 0.65, 0.2, 1);
    user-select: none;
    -webkit-user-select: none;
  }

  .periodic-viewport[data-dragging="true"] .periodic-wrap {
    transition: none;
  }

  .periodic-wrap.periodic-wrap--zooming {
    transition-duration: 64ms;
  }

  .cell {
    position: relative;
    border: 1px solid var(--color-border, var(--border));
    border-left: 3px solid var(--accent);
    background: var(--color-bg-surface, var(--bg-surface));
    min-height: 8rem;
    max-height: 8rem;
    padding: 0.4rem;
    display: grid;
    place-items: center;
    cursor: pointer;
    text-align: center;
    overflow: hidden;
    min-width: 0;
    border-radius: 0.08rem;
    user-select: none;
    -webkit-user-select: none;
    transition:
      background-color 95ms ease-out,
      border-color 95ms ease-out,
      outline-color 95ms ease-out,
      transform 95ms ease-out;
  }

  .cell:hover {
    background: color-mix(in srgb, var(--accent) 10%, var(--color-bg-surface, var(--bg-surface)));
    transform: translateY(-1px);
  }

  .cell--selected {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, var(--color-bg-surface, var(--bg-surface)));
    outline: 2px solid color-mix(in srgb, var(--accent) 70%, transparent);
  }

  .cell-corner {
    position: absolute;
    font-size: 0.5rem;
    line-height: 1;
    color: var(--color-text-muted, var(--fg-secondary));
    letter-spacing: 0.01em;
    white-space: nowrap;
    text-transform: uppercase;
    pointer-events: none;
    opacity: 0.9;
  }

  .cell-corner--tl {
    left: 0.26rem;
    top: 0.26rem;
  }

  .cell-corner--tr {
    right: 0.26rem;
    top: 0.26rem;
    text-align: right;
  }

  .cell-corner--bl {
    left: 0.26rem;
    bottom: 0.26rem;
    max-width: 44%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cell-corner--br {
    right: 0.26rem;
    bottom: 0.26rem;
    text-align: right;
    max-width: 44%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .symbol {
    font-size: 2.3rem;
    font-weight: 800;
    color: var(--color-text-primary, var(--fg-primary));
    line-height: 1;
    letter-spacing: 0.02em;
    align-self: center;
    justify-self: center;
    margin-top: -0.35rem;
  }

  .name {
    position: absolute;
    left: 50%;
    top: 1.76rem;
    transform: translateX(-50%);
    font-size: 0.62rem;
    line-height: 1.04;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    color: var(--color-text-muted, var(--fg-secondary));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 85%;
    margin-top: 0;
  }

  .fblock-label {
    display: flex;
    align-items: center;
    padding: 0;
    font-size: 1.7rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--color-text-muted, var(--fg-secondary));
    border-right: 1px dashed var(--color-border, var(--border));
  }

  @media (max-width: 1240px) {
    .toolbar-main {
      grid-template-columns: 1fr;
    }

    .toolbar-metrics {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 840px) {
    .toolbar-metrics {
      grid-template-columns: 1fr;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .periodic-wrap,
    .cell,
    .zoom-overlay,
    .pt-search,
    .pt-filter,
    .nav-controls button {
      transition: none;
    }
  }
</style>
