<script lang="ts">
  import { pdgSearchQuery } from '$lib/stores/pdgStore';

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    pdgSearchQuery.set(target.value);
  }

  function clearSearch() {
    pdgSearchQuery.set('');
  }
</script>

<div class="pdg-search">
  <input
    type="text"
    placeholder="Search particles by name, symbol, or PDG ID..."
    value={$pdgSearchQuery}
    on:input={handleInput}
    class="search-input"
  />
  {#if $pdgSearchQuery.trim()}
    <button on:click={clearSearch} class="clear-button" title="Clear search">
      ✕
    </button>
  {/if}
</div>

<style>
  .pdg-search {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin: 0.5rem 0;
  }

  .search-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--outline, #ccc);
    border-radius: 4px;
    font-size: 0.9rem;
    background: var(--surface, #fff);
    color: var(--on-surface, #000);
    transition: border-color 0.2s ease;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--primary, #1976d2);
    box-shadow: 0 0 0 2px rgba(25, 118, 210, 0.1);
  }

  .clear-button {
    padding: 0.4rem 0.6rem;
    border: none;
    border-radius: 4px;
    background: var(--surface-variant, #f5f5f5);
    color: var(--on-surface-variant, #666);
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s ease;
  }

  .clear-button:hover {
    background: var(--error, #f44336);
    color: white;
  }
</style>
