<script lang="ts">
  import { onMount } from "svelte";
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

  onMount(async () => {
    elements = await loadElements();
  });

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  async function handleElementClick(el: ElementData): Promise<void> {
    if (selectedElement?.Z === el.Z) {
      selectedElement = null;
      selectedIsotopeData = null;
      return;
    }

    selectedElement = el;
    selectedIsotopeData = null;

    if (!isotopeMap[el.Z]) {
      isotopeMap[el.Z] = await loadIsotopesForZ(el.Z);
    }

    broadcastSelection({ type: "ELEMENT_SELECTED", data: el });
    appendLog(`Periodic table: selected ${el.name} (Z=${el.Z})`);
    triggerFlash(el.Z);
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
    const lines = [
      `${el.name} (${el.symbol})`,
      `Z = ${el.Z}  ·  A\u2248${el.atomic_mass.toFixed(3)}`,
      `Period ${el.period}${el.group != null ? ", Group " + el.group : ", f-block"}`,
      `Category: ${el.category}`,
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

  <!-- Main 18-column grid -->
  <div class="periodic-wrap">
    {#each elements as el (el.Z)}
      <button
        class="cell"
        class:cell--selected={selectedElement?.Z === el.Z}
        class:cell--flash={flashZ === el.Z}
        style="grid-column:{el.display_col};grid-row:{el.display_row};--accent:{categoryColor(el.category)}"
        use:tooltip={{ text: elementTooltip(el), maxWidth: 380 }}
        aria-label={`Element ${el.name}, Z=${el.Z}`}
        on:click={() => handleElementClick(el)}
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

  <!-- Isotope drawer (slides open when an element is selected) -->
  {#if selectedElement}
    <IsotopeDrawer
      element={selectedElement}
      isotopes={isotopeMap[selectedElement.Z] ?? []}
      bind:selectedIsotope={selectedIsotopeData}
      on:isotopeSelect={handleIsotopeSelect}
      on:close={() => { selectedElement = null; selectedIsotopeData = null; }}
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

  .periodic-wrap {
    display: grid;
    grid-template-columns: repeat(18, minmax(0, 1fr));
    grid-template-rows: repeat(9, auto);
    gap: 0.15rem;
    overflow: auto;
    padding: 0.15rem;
    flex: 1 1 0;
    min-height: 0;
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
