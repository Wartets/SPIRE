<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { slide } from "svelte/transition";
  import type { ElementData, IsotopeData } from "$lib/core/physics/nuclearDataLoader";
  import {
    formatHalfLife,
    bindingEnergyPerNucleon,
  } from "$lib/core/physics/nuclearDataLoader";

  export let element: ElementData;
  export let isotopes: IsotopeData[] = [];
  export let selectedIsotope: IsotopeData | null = null;

  const dispatch = createEventDispatcher<{
    isotopeSelect: { Z: number; A: number; symbol: string; name: string; isotope: IsotopeData };
    close: void;
  }>();

  function handleIsotopeClick(iso: IsotopeData): void {
    selectedIsotope = iso;
    dispatch("isotopeSelect", {
      Z: element.Z,
      A: iso.A,
      symbol: element.symbol,
      name: element.name,
      isotope: iso,
    });
  }

  function modeLabel(mode: string): string {
    switch (mode) {
      case "beta-minus": return "β⁻";
      case "beta-plus":  return "β⁺";
      case "alpha":      return "α";
      case "ec":         return "EC";
      case "gamma":      return "γ";
      case "sf":         return "SF";
      default:           return mode;
    }
  }

  function isStable(iso: IsotopeData): boolean {
    return iso.half_life_s === null && iso.decay_modes.length === 0;
  }

  function bindingLabel(iso: IsotopeData): string {
    const be = bindingEnergyPerNucleon(element.Z, iso.A);
    return be > 0 ? `${be.toFixed(2)} MeV/A` : "—";
  }
</script>

<div class="isotope-drawer" transition:slide={{ duration: 220 }}>
  <header class="drawer-header">
    <div class="element-info">
      <span class="el-symbol">{element.symbol}</span>
      <span class="el-name">{element.name}</span>
      <span class="el-z">Z = {element.Z}</span>
    </div>
    <button class="close-btn" on:click={() => dispatch("close")} aria-label="Close isotope drawer">
      ✕
    </button>
  </header>

  {#if isotopes.length === 0}
    <p class="no-data">No isotope data available for this element.</p>
  {:else}
    <div class="isotope-list">
      {#each isotopes as iso (iso.A)}
        <button
          class="iso-row"
          class:stable={isStable(iso)}
          class:selected={selectedIsotope?.A === iso.A}
          on:click={() => handleIsotopeClick(iso)}
          aria-label={`Select ${element.symbol}-${iso.A}`}
        >
          <span class="iso-mass">{element.symbol}-{iso.A}</span>
          <span class="iso-spin">{iso.spin_parity}</span>
          <span class="iso-hl">{formatHalfLife(iso.half_life_s)}</span>
          <span class="iso-be">{bindingLabel(iso)}</span>
          <span class="iso-abund">
            {iso.abundance_percent > 0 ? `${iso.abundance_percent.toFixed(4)}%` : "—"}
          </span>
          <span class="iso-decay">
            {#if iso.decay_modes.length === 0}
              <span class="stable-tag">stable</span>
            {:else}
              {#each iso.decay_modes as dm}
                <span class="decay-tag mode-{dm.mode}">
                  {modeLabel(dm.mode)}
                  {#if dm.daughter_Z != null}
                    →<em>Z={dm.daughter_Z},A={dm.daughter_A}</em>
                  {/if}
                </span>
              {/each}
            {/if}
          </span>
        </button>
      {/each}
    </div>

    {#if selectedIsotope}
      <section class="detail-card" transition:slide={{ duration: 160 }}>
        <h5>{element.symbol}-{selectedIsotope.A} &nbsp;·&nbsp; {selectedIsotope.spin_parity}</h5>
        <div class="detail-grid">
          <span>Mass excess</span>
          <strong>{selectedIsotope.mass_excess_kev.toFixed(2)} keV</strong>
          <span>Half-life</span>
          <strong>{formatHalfLife(selectedIsotope.half_life_s)}</strong>
          <span>Binding / A</span>
          <strong>{bindingLabel(selectedIsotope)}</strong>
          <span>Abundance</span>
          <strong>
            {selectedIsotope.abundance_percent > 0
              ? `${selectedIsotope.abundance_percent.toFixed(4)} %`
              : "trace / radioactive"}
          </strong>
        </div>
        {#if selectedIsotope.decay_modes.length > 0}
          <div class="decay-list">
            <span class="decay-head">Decay modes:</span>
            {#each selectedIsotope.decay_modes as dm}
              <span class="decay-tag mode-{dm.mode}">
                {modeLabel(dm.mode)} · {(dm.fraction * 100).toFixed(1)}%
                → {element.symbol.slice(0, 2)}{dm.daughter_Z ? `(Z=${dm.daughter_Z}, A=${dm.daughter_A})` : ""}
              </span>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .isotope-drawer {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    overflow: hidden;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
  }

  .element-info {
    display: flex;
    align-items: baseline;
    gap: 0.45rem;
  }

  .el-symbol {
    font-family: var(--font-mono);
    font-size: 1.0rem;
    font-weight: 700;
    color: var(--color-accent, var(--hl-symbol));
  }

  .el-name {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .el-z {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .close-btn {
    border: 1px solid var(--color-border, var(--border));
    background: transparent;
    color: var(--color-text-muted, var(--fg-secondary));
    padding: 0.1rem 0.35rem;
    font-size: 0.7rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .no-data {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-text-muted, var(--fg-secondary));
    font-style: italic;
    padding: 0.5rem;
    margin: 0;
  }

  .isotope-list {
    display: flex;
    flex-direction: column;
    max-height: 220px;
    overflow-y: auto;
  }

  .iso-row {
    display: grid;
    grid-template-columns: 6rem 3.5rem 6rem 5.5rem 5rem 1fr;
    align-items: center;
    gap: 0.3rem;
    padding: 0.25rem 0.5rem;
    border: none;
    border-bottom: 1px solid var(--color-border, var(--border));
    background: transparent;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .iso-row:hover {
    background: var(--color-bg-inset, var(--bg-inset));
  }

  .iso-row.selected {
    background: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 12%, transparent);
    border-left: 2px solid var(--color-accent, var(--hl-symbol));
  }

  .iso-mass {
    font-weight: 600;
    color: var(--color-accent, var(--hl-symbol));
  }

  .iso-row.stable .iso-mass {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .iso-spin {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .iso-hl {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .iso-be {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .iso-abund {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .iso-decay {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .stable-tag {
    color: var(--color-success, var(--hl-success));
    font-size: 0.65rem;
  }

  .decay-tag {
    font-size: 0.63rem;
    padding: 0.05rem 0.25rem;
    border: 1px solid var(--color-border, var(--border));
    white-space: nowrap;
  }

  .decay-tag.mode-beta-minus  { border-color: var(--hl-error); color: var(--hl-error); }
  .decay-tag.mode-beta-plus   { border-color: var(--hl-value); color: var(--hl-value); }
  .decay-tag.mode-alpha        { border-color: var(--hl-symbol); color: var(--hl-symbol); }
  .decay-tag.mode-ec           { border-color: var(--hl-success); color: var(--hl-success); }
  .decay-tag.mode-gamma        { border-color: var(--color-text-muted, var(--fg-secondary)); color: var(--color-text-muted, var(--fg-secondary)); }

  .detail-card {
    padding: 0.4rem 0.5rem;
    border-top: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
  }

  .detail-card h5 {
    margin: 0 0 0.3rem;
    font-family: var(--font-mono);
    font-size: 0.76rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, auto 1fr);
    gap: 0.15rem 0.6rem;
    font-family: var(--font-mono);
    font-size: 0.67rem;
  }

  .detail-grid span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .detail-grid strong {
    color: var(--color-text-primary, var(--fg-primary));
    font-weight: 500;
  }

  .decay-list {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
    margin-top: 0.3rem;
    font-family: var(--font-mono);
    font-size: 0.66rem;
  }

  .decay-head {
    color: var(--color-text-muted, var(--fg-secondary));
  }
</style>
