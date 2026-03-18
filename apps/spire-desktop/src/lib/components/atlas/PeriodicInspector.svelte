<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { ElementData } from "$lib/core/physics/nuclearDataLoader";
  import {
    formatHalfLife,
    bindingEnergyPerNucleon,
  } from "$lib/core/physics/nuclearDataLoader";
  import type { AtlasPeriodicIsotopeSelection } from "$lib/stores/atlasPeriodicStore";
  import ElementFactSheet from "$lib/components/atlas/ElementFactSheet.svelte";
  import AtomicModel3D from "$lib/components/atlas/AtomicModel3D.svelte";
  import DecayTree from "$lib/components/atlas/DecayTree.svelte";

  export let element: ElementData | null = null;
  export let isotope: AtlasPeriodicIsotopeSelection | null = null;

  const dispatch = createEventDispatcher<{
    addToReaction: {
      target: "initial" | "final";
      id: string;
      label: string;
    };
  }>();

  $: currentElement = isotope?.element ?? element;
  $: isoN = isotope ? isotope.A - isotope.Z : 0;
  $: isoBA = isotope ? bindingEnergyPerNucleon(isotope.Z, isotope.A) : 0;
  $: isoHL = isotope ? formatHalfLife(isotope.isotopeData.half_life_s) : "";
  $: decayTreeInput = isotope
    ? { Z: isotope.Z, A: isotope.A, symbol: isotope.symbol, isotope: isotope.isotopeData }
    : null;
  $: displaySymbol = isotope ? `${isotope.symbol}-${isotope.A}` : currentElement?.symbol ?? "";
  $: displayName = isotope
    ? `${isotope.name} isotope, Z=${isotope.Z}`
    : currentElement
      ? `${currentElement.name} element, Z=${currentElement.Z}`
      : "";
  $: shellLayers = currentElement
    ? new Set(
        currentElement.electron_configuration
          .split(" ")
          .map((token) => Number.parseInt(token[0] ?? "0", 10))
          .filter((value) => Number.isFinite(value) && value > 0),
      ).size
    : 0;

  function reactionAddId(): string | null {
    if (isotope) return `${isotope.symbol}-${isotope.A}`;
    if (currentElement) return currentElement.symbol;
    return null;
  }

  function reactionAddLabel(): string {
    if (isotope) return `${isotope.symbol}-${isotope.A}`;
    if (currentElement) return `${currentElement.symbol} (${currentElement.name})`;
    return "selection";
  }

  function emitAddToReaction(target: "initial" | "final"): void {
    const id = reactionAddId();
    if (!id) return;
    dispatch("addToReaction", {
      target,
      id,
      label: reactionAddLabel(),
    });
  }

  function phaseFor(selectedElement: ElementData): string {
    if (["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(selectedElement.symbol)) return "gas";
    if (["Br", "Hg"].includes(selectedElement.symbol)) return "liquid";
    return "solid";
  }

  function blockFor(selectedElement: ElementData): string {
    if (selectedElement.group == null) return "f";
    if (selectedElement.group <= 2) return "s";
    if (selectedElement.group <= 12) return "d";
    return "p";
  }

  function modeLabel(modeName: string): string {
    switch (modeName) {
      case "beta-minus":
        return "beta-";
      case "beta-plus":
        return "beta+";
      case "alpha":
        return "alpha";
      case "ec":
        return "EC";
      case "gamma":
        return "gamma";
      case "sf":
        return "SF";
      default:
        return modeName;
    }
  }

</script>

<div class="inspector">
  <header class="inspector-header">
    <div class="title-block">
      <strong>{displaySymbol}</strong>
      <span>{displayName}</span>
    </div>
    <div class="inspector-actions">
      {#if reactionAddId()}
        <div class="reaction-actions">
          <button class="reaction-add" on:click={() => emitAddToReaction("initial")}>+ initial</button>
          <button class="reaction-add" on:click={() => emitAddToReaction("final")}>+ final</button>
        </div>
      {/if}
    </div>
  </header>

  {#if currentElement}
    <ElementFactSheet
      element={currentElement}
      title={isotope ? "Atomic host element" : "Element profile"}
      layout="compact"
    />

    <section class="data-card">
      <h5>Atomic Model</h5>
      <AtomicModel3D
        element={currentElement}
        {isotope}
        subtitle={isotope
          ? `${isotope.symbol}-${isotope.A} nucleus with planar shell occupancy`
          : `Electron configuration ${currentElement.electron_configuration}`}
      />
    </section>

    {#if isotope}
      <section class="data-card">
        <h5>Selected nuclide</h5>
        <div class="preview-grid">
          <div class="preview-item">
            <span>Nuclide</span>
            <strong>{isotope.symbol}-{isotope.A}</strong>
          </div>
          <div class="preview-item">
            <span>N/Z ratio</span>
            <strong>{(isoN / Math.max(1, isotope.Z)).toFixed(3)}</strong>
          </div>
          <div class="preview-item">
            <span>Half-life</span>
            <strong>{isoHL}</strong>
          </div>
          <div class="preview-item">
            <span>Binding / A</span>
            <strong>{isoBA.toFixed(3)} MeV</strong>
          </div>
        </div>
      </section>
    {/if}

    <section class="qn-card">
      {#if isotope}
        <div><span>Nuclide</span><strong>{isotope.symbol}-{isotope.A}</strong></div>
        <div><span>Z (protons)</span><strong>{isotope.Z}</strong></div>
        <div><span>N (neutrons)</span><strong>{isoN}</strong></div>
        <div><span>A (mass number)</span><strong>{isotope.A}</strong></div>
        <div><span>Spin / Parity</span><strong>{isotope.isotopeData.spin_parity ?? "n/a"}</strong></div>
        <div><span>Half-life</span><strong>{isoHL}</strong></div>
        <div><span>Binding / A</span><strong>{isoBA.toFixed(3)} MeV</strong></div>
        <div><span>Abundance</span><strong>{isotope.isotopeData.abundance_percent || 0}%</strong></div>
        <div><span>Mass excess</span><strong>{isotope.isotopeData.mass_excess_kev.toFixed(1)} keV</strong></div>
        <div><span>Decay channels</span><strong>{isotope.isotopeData.decay_modes.length}</strong></div>
      {/if}
      <div><span>Category</span><strong>{currentElement.category.replace(/-/g, " ")}</strong></div>
      <div><span>Period / Group</span><strong>{currentElement.period} / {currentElement.group ?? "f-block"}</strong></div>
      <div><span>Electron config</span><strong class="mono">{currentElement.electron_configuration}</strong></div>
      <div><span>Atomic mass</span><strong>{currentElement.atomic_mass.toFixed(4)}</strong></div>
      <div><span>Block</span><strong>{blockFor(currentElement)}-block</strong></div>
      <div><span>Phase @ STP</span><strong>{phaseFor(currentElement)}</strong></div>
      <div><span>Shell layers</span><strong>{shellLayers}</strong></div>
    </section>

    {#if isotope}
      <section class="data-card">
        <h5>Nuclear Decay Channels</h5>
        <DecayTree isotopeDecay={decayTreeInput} />
      </section>
    {/if}

    {#if isotope && isotope.isotopeData.decay_modes.length > 0}
      <section class="data-card">
        <h5>Decay Inventory</h5>
        <div class="inventory-grid">
          {#each isotope.isotopeData.decay_modes as decayMode, index (`${decayMode.mode}-${index}`)}
            <article>
              <strong>{modeLabel(decayMode.mode)}</strong>
              <span>Branching fraction</span>
              <em>{(decayMode.fraction * 100).toFixed(2)}%</em>
              <span>Daughter</span>
              <em>Z={decayMode.daughter_Z}, A={decayMode.daughter_A}</em>
            </article>
          {/each}
        </div>
      </section>
    {/if}
  {:else}
    <div class="empty">No item selected.</div>
  {/if}
</div>

<style>
  .inspector {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.45rem;
    min-height: 0;
  }

  .inspector-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-bottom: 1px solid var(--color-border, var(--border));
    padding-bottom: 0.3rem;
  }

  .title-block {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
  }

  .title-block strong,
  .title-block span,
  .reaction-add,
  .data-card h5,
  .preview-item,
  .qn-card span,
  .qn-card strong,
  .inventory-grid article,
  .empty {
    font-family: var(--font-mono);
  }

  .title-block strong {
    font-size: 0.88rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .title-block span {
    font-size: 0.69rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .inspector-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .reaction-actions {
    display: flex;
    gap: 0.22rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .reaction-add {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-primary, var(--fg-primary));
    padding: 0.18rem 0.4rem;
    font-size: 0.62rem;
    cursor: pointer;
  }

  .reaction-add {
    border-color: var(--color-accent, var(--hl-symbol));
  }

  .data-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .data-card h5 {
    margin: 0 0 0.35rem;
    font-size: 0.72rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .preview-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.3rem;
  }

  .preview-item {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    font-size: 0.67rem;
  }

  .preview-item span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .preview-item strong {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .qn-card {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.7rem;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .qn-card > div {
    display: flex;
    justify-content: space-between;
    gap: 0.35rem;
    border-bottom: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.25);
    padding-bottom: 0.14rem;
  }

  .qn-card span,
  .qn-card strong {
    font-size: 0.67rem;
  }

  .qn-card span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .qn-card strong {
    color: var(--color-text-primary, var(--fg-primary));
    font-weight: 600;
    text-align: right;
  }

  .inventory-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.32rem;
  }

  .inventory-grid article {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    gap: 0.14rem;
  }

  .inventory-grid strong {
    color: var(--color-accent, var(--hl-symbol));
    font-size: 0.68rem;
  }

  .inventory-grid span,
  .inventory-grid em {
    font-size: 0.61rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .empty {
    border: 1px dashed var(--color-border, var(--border));
    padding: 0.45rem;
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.7rem;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 0.65rem;
    word-break: break-all;
  }

  @media (max-width: 820px) {
    .preview-grid,
    .qn-card {
      grid-template-columns: 1fr;
    }
  }
</style>
