<script lang="ts">
  import { pdgCatalogMode, pdgSearchResults, chargeFilter, spinFilter, massRange, pdgFacetStats } from '$lib/stores/pdgStore';
  import PdgSearch from './PdgSearch.svelte';
  import VirtualParticleList from './VirtualParticleList.svelte';
  import PdgDetails from './PdgDetails.svelte';
  import { selectedPdgParticle } from '$lib/stores/pdgStore';

  // Facet filter state
  let selectedCharges: number[] = [];
  let selectedSpins: string[] = [];

  function toggleChargeFilter(charge: number) {
    if (selectedCharges.includes(charge)) {
      selectedCharges = selectedCharges.filter((c) => c !== charge);
    } else {
      selectedCharges = [...selectedCharges, charge];
    }
    chargeFilter.set(selectedCharges.length > 0 ? selectedCharges : 'all');
  }

  function toggleSpinFilter(spin: string) {
    if (selectedSpins.includes(spin)) {
      selectedSpins = selectedSpins.filter((s) => s !== spin);
    } else {
      selectedSpins = [...selectedSpins, spin];
    }
    spinFilter.set(selectedSpins.length > 0 ? selectedSpins : 'all');
  }

  function resetFilters() {
    selectedCharges = [];
    selectedSpins = [];
    chargeFilter.set('all');
    spinFilter.set('all');
    massRange.set({ min: 0, max: 1000000 });
  }

  function switchMode(mode: 'core_sm' | 'full_catalog') {
    pdgCatalogMode.set(mode);
  }
</script>

<div class="pdg-catalog-layout">
  <!-- Sidebar: Filters + Search + List -->
  <aside class="pdg-sidebar">
    <div class="sidebar-section">
      <h4>Browse Mode</h4>
      <div class="mode-buttons">
        <button
          class:active={$pdgCatalogMode === 'core_sm'}
          on:click={() => switchMode('core_sm')}
        >
          Core SM (13)
        </button>
        <button
          class:active={$pdgCatalogMode === 'full_catalog'}
          on:click={() => switchMode('full_catalog')}
        >
          Full Catalog (~1100)
        </button>
      </div>
    </div>

    <!-- Facet Filters -->
    <div class="sidebar-section">
      <div class="filter-header">
        <h4>Filters</h4>
        {#if selectedCharges.length > 0 || selectedSpins.length > 0}
          <button class="reset-button" on:click={resetFilters}>Reset</button>
        {/if}
      </div>

      <!-- Charge Filter -->
      <div class="filter-group">
        <div class="filter-title">Electric Charge</div>
        {#each Object.entries($pdgFacetStats.chargeDistribution) as [charge, count]}
          <label class="filter-checkbox">
            <input
              type="checkbox"
              checked={selectedCharges.includes(Number(charge))}
              on:change={() => toggleChargeFilter(Number(charge))}
            />
            <span>
              {Number(charge) === 0 ? '0' : Number(charge) > 0 ? '+' + charge : '−' + Math.abs(Number(charge))}
              ({count})
            </span>
          </label>
        {/each}
      </div>

      <!-- Spin Filter -->
      <div class="filter-group">
        <div class="filter-title">Spin</div>
        {#each Object.entries($pdgFacetStats.spinDistribution) as [spin, count]}
          <label class="filter-checkbox">
            <input
              type="checkbox"
              checked={selectedSpins.includes(spin)}
              on:change={() => toggleSpinFilter(spin)}
            />
            <span>{spin} ({count})</span>
          </label>
        {/each}
      </div>
    </div>

    <!-- Search -->
    <div class="sidebar-section">
      <h4>Search</h4>
      <PdgSearch />
    </div>

    <!-- Particle List -->
    <div class="sidebar-section list-section">
      <h4>Particles ({$pdgSearchResults.length})</h4>
      <VirtualParticleList
        items={$pdgSearchResults}
        selectedId={$selectedPdgParticle?.pdg_id ?? null}
      />
    </div>
  </aside>

  <!-- Main: Detail Panel -->
  <main class="pdg-viewer">
    {#if $selectedPdgParticle}
      <PdgDetails particle={$selectedPdgParticle} />
    {:else}
      <div class="empty-detail">
        <p>Select a particle to view details</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .pdg-catalog-layout {
    display: grid;
    grid-template-columns: 400px 1fr;
    gap: 1rem;
    height: 100%;
    overflow: hidden;
  }

  .pdg-sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    overflow-y: auto;
    padding: 0.5rem;
    background: var(--surface, #fff);
    border-right: 1px solid var(--outline, #ddd);
  }

  .sidebar-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .sidebar-section h4 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--on-surface, #000);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .filter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .filter-header h4 {
    margin: 0;
    flex: 1;
  }

  .reset-button {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--outline, #ddd);
    background: var(--surface-variant, #f5f5f5);
    border-radius: 3px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .reset-button:hover {
    background: var(--error, #f44336);
    color: white;
    border-color: var(--error, #f44336);
  }

  .mode-buttons {
    display: flex;
    gap: 0.5rem;
  }

  .mode-buttons button {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--outline, #ccc);
    background: var(--surface, #fff);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .mode-buttons button:hover {
    border-color: var(--primary, #1976d2);
    background: var(--primary-container, #e3f2fd);
  }

  .mode-buttons button.active {
    background: var(--primary, #1976d2);
    color: white;
    border-color: var(--primary, #1976d2);
  }

  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem;
    background: var(--surface-variant, #f5f5f5);
    border-radius: 4px;
  }

  .filter-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--on-surface-variant, #666);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .filter-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    cursor: pointer;
    padding: 0.25rem;
    border-radius: 3px;
    transition: background-color 0.15s ease;
  }

  .filter-checkbox:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  .filter-checkbox input {
    cursor: pointer;
  }

  .list-section {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .pdg-viewer {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 1rem;
    background: var(--surface, #fff);
  }

  .empty-detail {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--on-surface-variant, #999);
    text-align: center;
  }

  .empty-detail p {
    margin: 0;
    font-size: 1.1rem;
  }

  @media (max-width: 1200px) {
    .pdg-catalog-layout {
      grid-template-columns: 1fr;
    }

    .pdg-sidebar {
      border-right: none;
      border-bottom: 1px solid var(--outline, #ddd);
      max-height: 40%;
    }
  }
</style>
