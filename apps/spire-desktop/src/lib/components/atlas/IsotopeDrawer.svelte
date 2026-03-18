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
  export let compact = false;
  export let showHeader = true;

  let customA = 1;
  let customSeedZ = -1;

  $: if (element && customSeedZ !== element.Z) {
    customSeedZ = element.Z;
    customA = Math.max(element.Z, Math.round(element.atomic_mass));
  }

  const dispatch = createEventDispatcher<{
    isotopeSelect: { Z: number; A: number; symbol: string; name: string; isotope: IsotopeData };
    close: void;
  }>();

  function handleIsotopeClick(isotope: IsotopeData): void {
    selectedIsotope = isotope;
    dispatch("isotopeSelect", {
      Z: element.Z,
      A: isotope.A,
      symbol: element.symbol,
      name: element.name,
      isotope,
    });
  }

  function modeLabel(mode: string): string {
    switch (mode) {
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
        return mode;
    }
  }

  function isStable(isotope: IsotopeData): boolean {
    return isotope.half_life_s === null && isotope.decay_modes.length === 0;
  }

  function bindingLabel(isotope: IsotopeData): string {
    const binding = bindingEnergyPerNucleon(element.Z, isotope.A);
    return binding > 0 ? `${binding.toFixed(2)} MeV/A` : "-";
  }

  function forgeCustomIsotope(): void {
    const massNumber = Math.max(element.Z, Math.round(customA || element.atomic_mass));
    const isotope: IsotopeData = {
      A: massNumber,
      mass_excess_kev: 0,
      half_life_s: null,
      spin_parity: "custom",
      abundance_percent: 0,
      decay_modes: [],
    };

    handleIsotopeClick(isotope);
  }
</script>

<div class="isotope-drawer" class:compact transition:slide={{ duration: 220 }}>
  {#if showHeader}
    <header class="drawer-header">
      <div class="element-info">
        <span class="el-symbol">{element.symbol}</span>
        <span class="el-name">{element.name}</span>
        <span class="el-z">Z = {element.Z}</span>
      </div>
      <button class="close-btn" on:click={() => dispatch("close")} aria-label="Close isotope drawer">x</button>
    </header>
  {/if}

  {#if isotopes.length === 0}
    <p class="no-data">No isotope data available for this element. You can still forge a custom isotope below.</p>
  {:else}
    <div class="isotope-list">
      {#each isotopes as isotope (isotope.A)}
        <button
          class="iso-row"
          class:stable={isStable(isotope)}
          class:selected={selectedIsotope?.A === isotope.A}
          on:click={() => handleIsotopeClick(isotope)}
          aria-label={`Select ${element.symbol}-${isotope.A}`}
        >
          <span class="iso-mass">{element.symbol}-{isotope.A}</span>
          <span class="iso-spin">{isotope.spin_parity}</span>
          <span class="iso-hl">{formatHalfLife(isotope.half_life_s)}</span>
          <span class="iso-be">{bindingLabel(isotope)}</span>
          <span class="iso-abund">
            {isotope.abundance_percent > 0 ? `${isotope.abundance_percent.toFixed(4)}%` : "-"}
          </span>
          <span class="iso-decay">
            {#if isotope.decay_modes.length === 0}
              <span class="stable-tag">stable</span>
            {:else}
              {#each isotope.decay_modes as decayMode}
                <span class="decay-tag mode-{decayMode.mode}">
                  {modeLabel(decayMode.mode)}
                  {#if decayMode.daughter_Z != null}
                    -> <em>Z={decayMode.daughter_Z}, A={decayMode.daughter_A}</em>
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
        <h5>{element.symbol}-{selectedIsotope.A} | {selectedIsotope.spin_parity}</h5>
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
            {#each selectedIsotope.decay_modes as decayMode}
              <span class="decay-tag mode-{decayMode.mode}">
                {modeLabel(decayMode.mode)} | {(decayMode.fraction * 100).toFixed(1)}%
                -> {decayMode.daughter_Z ? `(Z=${decayMode.daughter_Z}, A=${decayMode.daughter_A})` : ""}
              </span>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/if}

  <section class="forge-card">
    <h5>Custom isotope forge</h5>
    <div class="forge-controls">
      <label>
        <span>Mass number A</span>
        <input
          type="number"
          min={element.Z}
          step="1"
          bind:value={customA}
          aria-label={`Custom isotope mass number for ${element.symbol}`}
        />
      </label>
      <button type="button" on:click={forgeCustomIsotope}>
        Select {element.symbol}-A
      </button>
    </div>
    <p>
      Creates a user-defined isotope profile ({element.symbol}-{Math.max(element.Z, Math.round(customA || element.atomic_mass))})
      even when measured registry data is incomplete.
    </p>
  </section>
</div>

<style>
  .isotope-drawer {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    overflow: hidden;
  }

  .isotope-drawer.compact {
    border: none;
    background: transparent;
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

  .el-symbol,
  .el-name,
  .el-z,
  .close-btn,
  .no-data,
  .iso-row,
  .detail-card h5,
  .detail-grid,
  .decay-list {
    font-family: var(--font-mono);
  }

  .el-symbol {
    font-size: 1rem;
    font-weight: 700;
    color: var(--color-accent, var(--hl-symbol));
  }

  .el-name {
    font-size: 0.75rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .el-z {
    font-size: 0.68rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .close-btn {
    border: 1px solid var(--color-border, var(--border));
    background: transparent;
    color: var(--color-text-muted, var(--fg-secondary));
    padding: 0.1rem 0.35rem;
    font-size: 0.7rem;
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .no-data {
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

  .isotope-drawer.compact .isotope-list {
    max-height: 16rem;
  }

  .iso-row {
    display: grid;
    grid-template-columns: 5.2rem 3.3rem 5.8rem 4.8rem 4.2rem 1fr;
    align-items: center;
    gap: 0.25rem;
    padding: 0.22rem 0.42rem;
    border: none;
    border-bottom: 1px solid var(--color-border, var(--border));
    background: transparent;
    cursor: pointer;
    text-align: left;
    font-size: 0.64rem;
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

  .iso-spin,
  .iso-hl,
  .iso-be,
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
    font-size: 0.62rem;
  }

  .decay-tag {
    font-size: 0.61rem;
    padding: 0.04rem 0.22rem;
    border: 1px solid var(--color-border, var(--border));
    white-space: nowrap;
  }

  .decay-tag.mode-beta-minus {
    border-color: var(--hl-error);
    color: var(--hl-error);
  }

  .decay-tag.mode-beta-plus {
    border-color: var(--hl-value);
    color: var(--hl-value);
  }

  .decay-tag.mode-alpha {
    border-color: var(--hl-symbol);
    color: var(--hl-symbol);
  }

  .decay-tag.mode-ec {
    border-color: var(--hl-success);
    color: var(--hl-success);
  }

  .decay-tag.mode-gamma {
    border-color: var(--color-text-muted, var(--fg-secondary));
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .detail-card {
    padding: 0.4rem 0.5rem;
    border-top: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
  }

  .forge-card {
    border-top: 1px solid var(--color-border, var(--border));
    padding: 0.38rem 0.5rem 0.45rem;
    background: color-mix(in srgb, var(--color-bg-inset, var(--bg-inset)) 82%, transparent);
    font-family: var(--font-mono);
  }

  .forge-card h5 {
    margin: 0 0 0.26rem;
    font-size: 0.66rem;
    text-transform: uppercase;
    color: var(--color-text-primary, var(--fg-primary));
    letter-spacing: 0.04em;
  }

  .forge-controls {
    display: flex;
    align-items: flex-end;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .forge-controls label {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .forge-controls span {
    font-size: 0.52rem;
    text-transform: uppercase;
    color: var(--color-text-muted, var(--fg-secondary));
    letter-spacing: 0.05em;
  }

  .forge-controls input,
  .forge-controls button {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.6rem;
    padding: 0.12rem 0.2rem;
  }

  .forge-controls input {
    width: 5.2rem;
    appearance: textfield;
    -moz-appearance: textfield;
  }

  .forge-controls input::-webkit-outer-spin-button,
  .forge-controls input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .forge-controls button {
    cursor: pointer;
  }

  .forge-controls button:hover {
    border-color: var(--color-accent, var(--hl-symbol));
  }

  .forge-card p {
    margin: 0.24rem 0 0;
    font-size: 0.57rem;
    color: var(--color-text-muted, var(--fg-secondary));
    line-height: 1.35;
  }

  .isotope-drawer.compact .detail-card {
    padding: 0.35rem 0.4rem;
  }

  .detail-card h5 {
    margin: 0 0 0.3rem;
    font-size: 0.7rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, auto 1fr);
    gap: 0.15rem 0.6rem;
    font-size: 0.64rem;
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
    font-size: 0.64rem;
  }

  .decay-head {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  @media (max-width: 900px) {
    .iso-row {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
