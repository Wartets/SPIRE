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
    margin: 0;
  }

  .search-input {
    flex: 1;
    padding: var(--spacing-sm);
    border: 1px solid var(--border);
    font-size: var(--text-sm);
    background: var(--bg-inset);
    color: var(--fg-primary);
    font-family: var(--font-mono);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--border-focus);
  }

  .clear-button {
    padding: 0.35rem 0.6rem;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    cursor: pointer;
    font-size: var(--text-sm);
    font-family: var(--font-mono);
  }

  .clear-button:hover {
    border-color: var(--hl-error);
    color: var(--hl-error);
  }
</style>
