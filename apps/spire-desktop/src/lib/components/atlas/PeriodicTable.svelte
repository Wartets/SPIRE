<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { onMount, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import IsotopeDrawer from "$lib/components/atlas/IsotopeDrawer.svelte";
  import {
    loadElements,
    loadAllIsotopes,
    loadIsotopesForZ,
    type ElementData,
    type IsotopeData,
  } from "$lib/core/physics/nuclearDataLoader";
  import { broadcastSelection } from "$lib/stores/selectionBus";
  import { appendLog } from "$lib/stores/physicsStore";

  const logAtlas = (message: string): void => {
    appendLog(message, { category: "Atlas" });
  };

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let elements: ElementData[] = [];
  let selectedElement: ElementData | null = null;
  let selectedIsotopeData: IsotopeData | null = null;
  let isotopeMap: Record<number, IsotopeData[]> = {};
  let isotopeRegistry: Record<string, IsotopeData[]> = {};
  let isotopeStats: Record<number, {
    count: number;
    stable: number;
    naturalA: number | null;
    heavyA: number | null;
  }> = {};
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
  let hoveredElement: ElementData | null = null;
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
    const allIsotopes = await loadAllIsotopes();
    isotopeRegistry = allIsotopes;
    isotopeStats = Object.fromEntries(
      elements.map((el) => {
        const isotopes = allIsotopes[String(el.Z)] ?? [];
        const stable = isotopes.filter((iso) => iso.half_life_s === null).length;
        const natural = [...isotopes].sort((a, b) => (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0))[0] ?? null;
        const heavy = [...isotopes].sort((a, b) => b.A - a.A)[0] ?? null;
        return [el.Z, {
          count: isotopes.length,
          stable,
          naturalA: natural?.A ?? null,
          heavyA: heavy?.A ?? null,
        }];
      }),
    );
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
    logAtlas(`Periodic table: selected ${el.name} (Z=${el.Z})`);
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
    logAtlas(`Isotope selected: ${d.symbol}-${d.A} (Z=${d.Z})`);
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
    const target = event.target as HTMLElement | null;
    if (target?.closest(".cell")) return;
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
  $: focusElementData = selectedElement ?? hoveredElement ?? visibleElements[0] ?? null;
  $: visibleStableCount = visibleElements.reduce((sum, el) => sum + (isotopeStats[el.Z]?.stable ?? 0), 0);
  $: visibleIsotopeCount = visibleElements.reduce((sum, el) => sum + (isotopeStats[el.Z]?.count ?? 0), 0);
  $: visibleAverageStable = visibleElements.length > 0 ? visibleStableCount / visibleElements.length : 0;
  $: visibleBlockCounts = visibleElements.reduce(
    (acc, el) => {
      const block = blockFor(el);
      acc[block] += 1;
      return acc;
    },
    { s: 0, p: 0, d: 0, f: 0 },
  );
  $: focusIsotopes = focusElementData
    ? [...(isotopeRegistry[String(focusElementData.Z)] ?? [])]
        .sort((a, b) => (b.abundance_percent ?? 0) - (a.abundance_percent ?? 0))
        .slice(0, 4)
    : [];
  $: focusMassSpan = focusElementData
    ? (() => {
        const isotopes = isotopeRegistry[String(focusElementData.Z)] ?? [];
        if (isotopes.length === 0) return "n/a";
        const minA = isotopes.reduce((m, iso) => Math.min(m, iso.A), Number.POSITIVE_INFINITY);
        const maxA = isotopes.reduce((m, iso) => Math.max(m, iso.A), Number.NEGATIVE_INFINITY);
        return `${minA}–${maxA}`;
      })()
    : "n/a";
  $: focusShellCount = focusElementData
    ? new Set(
        focusElementData.electron_configuration
          .split(" ")
          .map((token) => token.trim())
          .filter((token) => token.length > 0)
          .map((token) => Number.parseInt(token[0] ?? "0", 10))
          .filter((value) => Number.isFinite(value) && value > 0),
      ).size
    : 0;
  $: focusValenceApprox = focusElementData
    ? (() => {
        const shell = valenceShell(focusElementData);
        const match = shell.match(/([spdf])(\d+)/i);
        if (!match) return "n/a";
        return `${match[1].toLowerCase()}${match[2]}`;
      })()
    : "n/a";
  $: focusCategoryLabel = focusElementData ? focusElementData.category.replace(/-/g, " ") : "n/a";

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
    const stats = isotopeStats[el.Z];
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
      `Known isotopes: ${stats?.count ?? 0} · stable: ${stats?.stable ?? 0}`,
      `Config: ${el.electron_configuration}`,
    ];
    return lines.join("\n");
  }

  function valenceShell(el: ElementData): string {
    const tokens = el.electron_configuration.split(" ");
    return tokens[tokens.length - 1] ?? el.electron_configuration;
  }

  function phaseFor(el: ElementData): string {
    if (["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(el.symbol)) return "gas";
    if (["Br", "Hg"].includes(el.symbol)) return "liq";
    return "sol";
  }

  function blockFor(el: ElementData): "s" | "p" | "d" | "f" {
    if (el.group == null) return "f";
    if (el.group <= 2) return "s";
    if (el.group <= 12) return "d";
    return "p";
  }
</script>

<div class="pt-outer">
  {#if focusElementData}
    <section class="pt-summary-band">
      <article class="focus-card" style={`--focus-accent:${categoryColor(focusElementData.category)}`}>
        <div class="focus-title-row">
          <div>
            <strong>{focusElementData.symbol}</strong>
            <span>{focusElementData.name}</span>
          </div>
          <span class="focus-z">Z {focusElementData.Z}</span>
        </div>
        <div class="focus-grid">
          <div><span>Atomic mass</span><strong>{focusElementData.atomic_mass.toFixed(4)}</strong></div>
          <div><span>Period / Group</span><strong>{focusElementData.period} / {focusElementData.group ?? "f"}</strong></div>
          <div><span>Block / Phase</span><strong>{blockFor(focusElementData)}-block · {phaseFor(focusElementData)}</strong></div>
          <div><span>Known isotopes</span><strong>{isotopeStats[focusElementData.Z]?.count ?? 0}</strong></div>
          <div><span>Stable isotopes</span><strong>{isotopeStats[focusElementData.Z]?.stable ?? 0}</strong></div>
          <div><span>Valence shell</span><strong class="mono">{valenceShell(focusElementData)}</strong></div>
        </div>
      </article>

      <div class="metric-cards">
        <article>
          <span>Visible elements</span>
          <strong>{visibleElements.length}</strong>
        </article>
        <article>
          <span>Stable isotopes in view</span>
          <strong>{visibleStableCount}</strong>
        </article>
        <article>
          <span>Total isotopes in view</span>
          <strong>{visibleIsotopeCount}</strong>
        </article>
        <article>
          <span>Filter state</span>
          <strong>{categoryFilter === "all" ? "full atlas" : categoryFilter}</strong>
        </article>
        <article>
          <span>Avg stable / element</span>
          <strong>{visibleAverageStable.toFixed(2)}</strong>
        </article>
        <article>
          <span>Block mix</span>
          <strong>s:{visibleBlockCounts.s} p:{visibleBlockCounts.p} d:{visibleBlockCounts.d} f:{visibleBlockCounts.f}</strong>
        </article>
      </div>
    </section>
  {/if}

  {#if focusElementData}
    <section class="element-intel">
      <article>
        <h5>{focusElementData.symbol} isotope intelligence</h5>
        <div class="intel-grid">
          <div><span>Mass span A</span><strong>{focusMassSpan}</strong></div>
          <div><span>Natural anchor</span><strong>{isotopeStats[focusElementData.Z]?.naturalA ?? "n/a"}</strong></div>
          <div><span>Heavy anchor</span><strong>{isotopeStats[focusElementData.Z]?.heavyA ?? "n/a"}</strong></div>
          <div><span>Known stable</span><strong>{isotopeStats[focusElementData.Z]?.stable ?? 0}</strong></div>
        </div>
      </article>
      <article>
        <h5>Dominant isotopes</h5>
        <div class="iso-chip-row">
          {#if focusIsotopes.length === 0}
            <span class="iso-chip">No isotope registry</span>
          {:else}
            {#each focusIsotopes as iso (`focus-iso-${focusElementData.Z}-${iso.A}`)}
              <span class="iso-chip">
                {focusElementData.symbol}-{iso.A}
                <em>{iso.abundance_percent != null ? `${iso.abundance_percent.toFixed(2)}%` : "trace"}</em>
              </span>
            {/each}
          {/if}
        </div>
      </article>

      <article>
        <h5>Electronic profile</h5>
        <div class="intel-grid">
          <div><span>Electron count</span><strong>{focusElementData.Z}</strong></div>
          <div><span>Shell layers</span><strong>{focusShellCount}</strong></div>
          <div><span>Valence estimate</span><strong class="mono">{focusValenceApprox}</strong></div>
          <div><span>Family</span><strong>{focusCategoryLabel}</strong></div>
        </div>
      </article>
    </section>
  {/if}

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
        {@const stats = isotopeStats[el.Z]}
        <button
          class="cell"
          class:cell--selected={selectedElement?.Z === el.Z}
          class:cell--flash={flashZ === el.Z}
          style="grid-column:{el.display_col};grid-row:{el.display_row};--accent:{categoryColor(el.category)}"
          use:tooltip={{ text: elementTooltip(el), maxWidth: 380 }}
          aria-label={`Element ${el.name}, Z=${el.Z}`}
          on:mouseenter={() => (hoveredElement = el)}
          on:focus={() => (hoveredElement = el)}
          on:click={() => handleCellClick(el)}
        >
          <div class="cell-topline">
            <span class="z">{el.Z}</span>
            <span class="mass">{el.atomic_mass.toFixed(2)}</span>
          </div>
          <span class="symbol">{el.symbol}</span>
          <span class="ename">{el.name}</span>
          <span class="config">{valenceShell(el)}</span>
          <div class="cell-footline">
            <span class="cell-chip">{blockFor(el)}-{phaseFor(el)}</span>
            <span class="cell-chip">iso {stats?.count ?? 0}</span>
            <span class="cell-chip">stable {stats?.stable ?? 0}</span>
          </div>
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

  .pt-summary-band {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 0.75fr);
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .element-intel {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .element-intel article {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.36rem 0.45rem;
  }

  .element-intel h5 {
    margin: 0 0 0.28rem;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .intel-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.2rem 0.45rem;
  }

  .intel-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .intel-grid span,
  .intel-grid strong,
  .iso-chip,
  .iso-chip em {
    font-family: var(--font-mono);
  }

  .intel-grid span {
    font-size: 0.56rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .intel-grid strong {
    font-size: 0.7rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .iso-chip-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .iso-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.16rem;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.28);
    padding: 0.05rem 0.22rem;
    font-size: 0.58rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .iso-chip em {
    font-size: 0.53rem;
    color: var(--color-text-muted, var(--fg-secondary));
    font-style: normal;
  }

  .focus-card,
  .metric-cards article {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
  }

  .focus-card {
    border-left: 3px solid var(--focus-accent, var(--hl-symbol));
    padding: 0.4rem 0.5rem;
  }

  .focus-title-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.35rem;
  }

  .focus-title-row strong {
    display: block;
    font-family: var(--font-mono);
    font-size: 1rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .focus-title-row span,
  .focus-z,
  .focus-grid span,
  .focus-grid strong,
  .metric-cards span,
  .metric-cards strong {
    font-family: var(--font-mono);
  }

  .focus-title-row > div > span {
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.66rem;
  }

  .focus-z {
    color: var(--focus-accent, var(--hl-symbol));
    font-size: 0.7rem;
  }

  .focus-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.28rem 0.6rem;
  }

  .focus-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .focus-grid span {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .focus-grid strong {
    font-size: 0.69rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .metric-cards {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.35rem;
  }

  .metric-cards article {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0.35rem 0.4rem;
    min-height: 4.2rem;
  }

  .metric-cards span {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .metric-cards strong {
    font-size: 0.8rem;
    color: var(--color-text-primary, var(--fg-primary));
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
    width: 70rem;
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
    min-height: 4.75rem;
    max-height: 4.75rem;
    padding: 0.18rem 0.24rem;
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    border-left: 3px solid var(--accent);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-mono);
    transition: background 0.12s;
    overflow: hidden;
    min-width: 0;
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

  .cell-topline,
  .cell-footline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.16rem;
    min-width: 0;
  }

  .cell-footline {
    flex-wrap: wrap;
    align-content: flex-start;
    overflow: hidden;
    max-height: 1.2rem;
  }

  .mass {
    font-size: 0.5rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .symbol {
    font-size: 1.02rem;
    font-weight: 700;
    color: var(--color-text-primary, var(--fg-primary));
    line-height: 1.1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ename {
    font-size: 0.52rem;
    color: var(--color-text-muted, var(--fg-secondary));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.1;
  }

  .config {
    font-size: 0.49rem;
    color: var(--color-accent, var(--hl-symbol));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .cell-chip {
    font-size: 0.46rem;
    color: var(--color-text-muted, var(--fg-secondary));
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.2);
    padding: 0.02rem 0.12rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
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

  .mono {
    font-family: var(--font-mono);
  }

  @media (max-width: 1240px) {
    .pt-summary-band {
      grid-template-columns: 1fr;
    }

    .element-intel {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric-cards {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 920px) {
    .element-intel {
      grid-template-columns: 1fr;
    }
  }
</style>
