<!--
  SPIRE — Particle Table

  Visual Standard Model periodic-table-style particle selector.
  Renders a dense grid of particle cells grouped by category
  (Quarks, Leptons, Gauge Bosons, Scalars). Each cell shows the
  particle symbol, name, and mass. Click to select.

  Can be toggled between this "Table View" and a traditional
  list/dropdown via the `mode` prop.

  Props:
    particles  — Array of selectable particle entries.
    selected   — Currently selected particle ID (bound).
    mode       — "table" | "list" display mode.

  Events:
    select     — Dispatched when a particle is clicked, with the particle ID.
-->
<script context="module" lang="ts">
  /** A single entry in the particle table. */
  export interface ParticleEntry {
    id: string;
    symbol: string;
    name: string;
    mass: string;
    charge: string;
    category: "quark" | "lepton" | "gauge_boson" | "scalar" | "other";
  }
</script>

<script lang="ts">
  import { createEventDispatcher } from "svelte";

  // ── Default Standard Model Particle Catalog ──

  const DEFAULT_SM_PARTICLES: ParticleEntry[] = [
    // Quarks
    { id: "u",  symbol: "u",  name: "Up",      mass: "2.2 MeV",   charge: "+2/3", category: "quark" },
    { id: "d",  symbol: "d",  name: "Down",    mass: "4.7 MeV",   charge: "-1/3", category: "quark" },
    { id: "c",  symbol: "c",  name: "Charm",   mass: "1.27 GeV",  charge: "+2/3", category: "quark" },
    { id: "s",  symbol: "s",  name: "Strange", mass: "96 MeV",    charge: "-1/3", category: "quark" },
    { id: "t",  symbol: "t",  name: "Top",     mass: "172.7 GeV", charge: "+2/3", category: "quark" },
    { id: "b",  symbol: "b",  name: "Bottom",  mass: "4.18 GeV",  charge: "-1/3", category: "quark" },
    // Leptons
    { id: "e",    symbol: "e",   name: "Electron",       mass: "0.511 MeV", charge: "-1", category: "lepton" },
    { id: "mu",   symbol: "μ",   name: "Muon",           mass: "105.7 MeV", charge: "-1", category: "lepton" },
    { id: "tau",  symbol: "τ",   name: "Tau",            mass: "1.777 GeV", charge: "-1", category: "lepton" },
    { id: "nu_e", symbol: "νₑ",  name: "Electron Neutrino", mass: "< 1 eV", charge: "0", category: "lepton" },
    { id: "nu_mu",  symbol: "ν_μ", name: "Muon Neutrino",     mass: "< 1 eV", charge: "0", category: "lepton" },
    { id: "nu_tau", symbol: "ν_τ", name: "Tau Neutrino",      mass: "< 1 eV", charge: "0", category: "lepton" },
    // Gauge Bosons
    { id: "gamma", symbol: "γ",  name: "Photon",   mass: "0",         charge: "0",  category: "gauge_boson" },
    { id: "g",     symbol: "g",  name: "Gluon",    mass: "0",         charge: "0",  category: "gauge_boson" },
    { id: "W",     symbol: "W±", name: "W Boson",  mass: "80.38 GeV", charge: "±1", category: "gauge_boson" },
    { id: "Z",     symbol: "Z⁰", name: "Z Boson",  mass: "91.19 GeV", charge: "0",  category: "gauge_boson" },
    // Scalars
    { id: "H",     symbol: "H",  name: "Higgs",    mass: "125.1 GeV", charge: "0",  category: "scalar" },
  ];

  // ── Props ──

  export let particles: ParticleEntry[] = DEFAULT_SM_PARTICLES;
  export let selected: string = "";
  export let mode: "table" | "list" = "table";

  const dispatch = createEventDispatcher<{ select: string }>();

  function handleSelect(id: string): void {
    selected = id;
    dispatch("select", id);
  }

  // Initialise particles with default if empty
  $: if (particles.length === 0) {
    particles = DEFAULT_SM_PARTICLES;
  }

  // Group particles by category
  $: quarks = particles.filter(p => p.category === "quark");
  $: leptons = particles.filter(p => p.category === "lepton");
  $: bosons = particles.filter(p => p.category === "gauge_boson");
  $: scalars = particles.filter(p => p.category === "scalar");
  $: others = particles.filter(p => p.category === "other");

  // Category colour mapping
  function categoryColor(cat: string): string {
    switch (cat) {
      case "quark":       return "rgba(94, 184, 255, 0.15)";
      case "lepton":      return "rgba(46, 204, 113, 0.15)";
      case "gauge_boson": return "rgba(212, 160, 23, 0.15)";
      case "scalar":      return "rgba(231, 76, 60, 0.15)";
      default:            return "rgba(136, 136, 136, 0.1)";
    }
  }

  function categoryBorder(cat: string): string {
    switch (cat) {
      case "quark":       return "rgba(94, 184, 255, 0.4)";
      case "lepton":      return "rgba(46, 204, 113, 0.4)";
      case "gauge_boson": return "rgba(212, 160, 23, 0.4)";
      case "scalar":      return "rgba(231, 76, 60, 0.4)";
      default:            return "var(--border)";
    }
  }
</script>

<!-- Mode Toggle -->
<div class="pt-mode-toggle">
  <button
    class="pt-mode-btn"
    class:active={mode === "list"}
    on:click={() => mode = "list"}
  >List</button>
  <button
    class="pt-mode-btn"
    class:active={mode === "table"}
    on:click={() => mode = "table"}
  >Table</button>
</div>

{#if mode === "table"}
  <!-- Periodic Table Grid View -->
  <div class="pt-grid">
    {#if quarks.length > 0}
      <div class="pt-group-label">Quarks</div>
      <div class="pt-row">
        {#each quarks as p (p.id)}
          <button
            class="pt-cell"
            class:selected={selected === p.id}
            style="background: {categoryColor(p.category)}; border-color: {selected === p.id ? 'var(--fg-accent)' : categoryBorder(p.category)};"
            on:click={() => handleSelect(p.id)}
            title="{p.name} — {p.mass}, Q = {p.charge}"
          >
            <span class="pt-symbol">{p.symbol}</span>
            <span class="pt-mass">{p.mass}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if leptons.length > 0}
      <div class="pt-group-label">Leptons</div>
      <div class="pt-row">
        {#each leptons as p (p.id)}
          <button
            class="pt-cell"
            class:selected={selected === p.id}
            style="background: {categoryColor(p.category)}; border-color: {selected === p.id ? 'var(--fg-accent)' : categoryBorder(p.category)};"
            on:click={() => handleSelect(p.id)}
            title="{p.name} — {p.mass}, Q = {p.charge}"
          >
            <span class="pt-symbol">{p.symbol}</span>
            <span class="pt-mass">{p.mass}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if bosons.length > 0}
      <div class="pt-group-label">Gauge Bosons</div>
      <div class="pt-row">
        {#each bosons as p (p.id)}
          <button
            class="pt-cell"
            class:selected={selected === p.id}
            style="background: {categoryColor(p.category)}; border-color: {selected === p.id ? 'var(--fg-accent)' : categoryBorder(p.category)};"
            on:click={() => handleSelect(p.id)}
            title="{p.name} — {p.mass}, Q = {p.charge}"
          >
            <span class="pt-symbol">{p.symbol}</span>
            <span class="pt-mass">{p.mass}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if scalars.length > 0}
      <div class="pt-group-label">Scalars</div>
      <div class="pt-row">
        {#each scalars as p (p.id)}
          <button
            class="pt-cell"
            class:selected={selected === p.id}
            style="background: {categoryColor(p.category)}; border-color: {selected === p.id ? 'var(--fg-accent)' : categoryBorder(p.category)};"
            on:click={() => handleSelect(p.id)}
            title="{p.name} — {p.mass}, Q = {p.charge}"
          >
            <span class="pt-symbol">{p.symbol}</span>
            <span class="pt-mass">{p.mass}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if others.length > 0}
      <div class="pt-group-label">Other</div>
      <div class="pt-row">
        {#each others as p (p.id)}
          <button
            class="pt-cell"
            class:selected={selected === p.id}
            style="background: {categoryColor(p.category)}; border-color: {selected === p.id ? 'var(--fg-accent)' : categoryBorder(p.category)};"
            on:click={() => handleSelect(p.id)}
            title="{p.name} — {p.mass}, Q = {p.charge}"
          >
            <span class="pt-symbol">{p.symbol}</span>
            <span class="pt-mass">{p.mass}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

{:else}
  <!-- List View -->
  <div class="pt-list">
    {#each particles as p (p.id)}
      <button
        class="pt-list-item"
        class:selected={selected === p.id}
        on:click={() => handleSelect(p.id)}
      >
        <span class="pt-list-symbol" style="color: {categoryBorder(p.category)};">{p.symbol}</span>
        <span class="pt-list-name">{p.name}</span>
        <span class="pt-list-mass">{p.mass}</span>
        <span class="pt-list-charge">Q = {p.charge}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  /* ── Mode Toggle ── */
  .pt-mode-toggle {
    display: flex;
    gap: 0.2rem;
    margin-bottom: 0.4rem;
  }

  .pt-mode-btn {
    flex: 1;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    font-weight: 600;
    font-family: var(--font-mono);
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    cursor: pointer;
    transition: all 0.12s;
  }

  .pt-mode-btn.active {
    background: rgba(94, 184, 255, 0.12);
    color: var(--hl-symbol);
    border-color: rgba(94, 184, 255, 0.4);
  }

  .pt-mode-btn:hover:not(.active) {
    background: var(--bg-surface);
    color: var(--fg-primary);
  }

  /* ── Table Grid View ── */
  .pt-grid {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .pt-group-label {
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary);
    padding: 0.1rem 0;
  }

  .pt-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .pt-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 3.6rem;
    height: 3rem;
    border: 1px solid var(--border);
    cursor: pointer;
    transition: border-color 0.12s, box-shadow 0.12s;
    padding: 0.15rem;
    font-family: var(--font-mono);
  }

  .pt-cell:hover {
    border-color: var(--fg-accent) !important;
    box-shadow: 0 0 6px rgba(255, 255, 255, 0.1);
  }

  .pt-cell.selected {
    border-width: 2px;
    box-shadow: 0 0 8px rgba(94, 184, 255, 0.3);
  }

  .pt-symbol {
    font-size: 0.9rem;
    font-weight: 700;
    color: var(--fg-accent);
    line-height: 1.1;
  }

  .pt-mass {
    font-size: 0.52rem;
    color: var(--fg-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    text-align: center;
  }

  /* ── List View ── */
  .pt-list {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    max-height: 300px;
    overflow-y: auto;
  }

  .pt-list-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.4rem;
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--fg-primary);
    text-align: left;
    transition: background 0.1s, border-color 0.1s;
  }

  .pt-list-item:hover {
    background: var(--bg-surface);
    border-color: var(--border);
  }

  .pt-list-item.selected {
    background: rgba(94, 184, 255, 0.1);
    border-color: rgba(94, 184, 255, 0.4);
    color: var(--fg-accent);
  }

  .pt-list-symbol {
    font-weight: 700;
    font-size: 0.82rem;
    min-width: 1.8rem;
    text-align: center;
  }

  .pt-list-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pt-list-mass {
    color: var(--fg-secondary);
    font-size: 0.65rem;
    white-space: nowrap;
  }

  .pt-list-charge {
    color: var(--fg-secondary);
    font-size: 0.62rem;
    white-space: nowrap;
  }
</style>
